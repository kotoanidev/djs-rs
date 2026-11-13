//! A mock Create Message endpoint that fails on purpose (D-279, reliability), served over
//! plain HTTP (djs-rs) and HTTPS (serenity, D-282) by the same process and state.
//!
//! | `--scenario` | The fault | A correct client |
//! |---|---|---|
//! | `route429` | the first request of each channel gets a 429 for its route, `retry_after` 0.5 s | waits, then retries: every message is sent, no request inside the window |
//! | `global429` | the first request gets a global 429, `retry_after` 0.5 s | pauses every route, then retries |
//! | `server5xx` | the first attempt at every 4th message of each channel gets a 502 Bad Gateway (an HTML page, as from a proxy) | retries it: every message is sent |
//!
//! A request that arrives while its route's (or the global) window is open is answered with
//! another 429: within 25 ms of the window's opening it was already on its way when the 429
//! was sent (`in_flight`); later, the client ignored the 429 (`early`). For each 429 the mock
//! also records how long the client waited: from the 429 to the first request after it that
//! was not in flight, on the same channel (or any, for the global one). The counts are written
//! to `--report` after every request.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use djs_vs_node::{Options, payloads::sent_message};
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, body::Incoming, header::CONTENT_TYPE};
use serde_json::json;
use tokio::net::TcpListener;

use crate::tls;

/// How long a 429 asks the client to wait.
const RETRY_AFTER: Duration = Duration::from_millis(500);

/// How long after a 429 a request still counts as sent before the client could know of it.
const IN_FLIGHT: Duration = Duration::from_millis(25);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Route429,
    Global429,
    Server5xx,
}

#[derive(Debug, Default)]
struct Counts {
    received: u64,
    ok: u64,
    limited: u64,
    in_flight: u64,
    early: u64,
    server_errors: u64,
    limited_channels: HashSet<String>,
    global_done: bool,
    /// How many messages each channel has accepted.
    delivered: HashMap<String, u64>,
    /// The (channel, message) attempts already answered with a 502.
    failed_once: HashSet<(String, u64)>,
    /// The 429 windows: by channel, and `"global"`.
    windows: HashMap<String, Window>,
    /// How long the client waited after each 429, in milliseconds.
    waited_ms: Vec<f64>,
}

/// A 429's window: when it opened and closes, and whether the client's wait was recorded.
#[derive(Clone, Copy, Debug)]
struct Window {
    opened: Instant,
    closes: Instant,
    waited: bool,
}

impl Window {
    fn open(now: Instant) -> Self {
        Self {
            opened: now,
            closes: now + RETRY_AFTER,
            waited: false,
        }
    }
}

/// Records the client's wait after the windows of `keys`, at its first request not in flight.
fn record_waits(counts: &mut Counts, keys: [&str; 2], now: Instant) {
    let mut waits = Vec::new();
    for key in keys {
        if let Some(window) = counts.windows.get_mut(key)
            && !window.waited
            && now - window.opened >= IN_FLIGHT
        {
            window.waited = true;
            waits.push((now - window.opened).as_secs_f64() * 1000.0);
        }
    }
    counts.waited_ms.extend(waits);
}

#[derive(Debug)]
struct Faults {
    scenario: Scenario,
    report: PathBuf,
    counts: Mutex<Counts>,
}

impl Faults {
    fn write_report(&self, counts: &Counts) {
        let report = json!({
            "scenario": format!("{:?}", self.scenario).to_lowercase(),
            "received": counts.received,
            "ok": counts.ok,
            "limited": counts.limited,
            "in_flight": counts.in_flight,
            "early": counts.early,
            "server_errors": counts.server_errors,
            "waited_ms": counts.waited_ms,
        });
        let _ = fs::write(&self.report, report.to_string());
    }
}

fn response(status: u16, body: String, headers: &[(&str, String)]) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json");
    for (name, value) in headers {
        builder = builder.header(*name, value);
    }
    builder
        .body(Full::new(Bytes::from(body)))
        .expect("a valid response")
}

/// A 429 for `wait`, for the route or global.
fn rate_limited(wait: Duration, global: bool) -> Response<Full<Bytes>> {
    let seconds = wait.as_secs_f64();
    let mut headers = vec![
        ("retry-after", format!("{}", seconds.ceil().max(1.0))),
        (
            "x-ratelimit-scope",
            (if global { "global" } else { "user" }).to_owned(),
        ),
    ];
    if global {
        headers.push(("x-ratelimit-global", "true".to_owned()));
    } else {
        headers.extend([
            ("x-ratelimit-limit", "1".to_owned()),
            ("x-ratelimit-remaining", "0".to_owned()),
            ("x-ratelimit-reset-after", format!("{seconds:.3}")),
            (
                "x-ratelimit-bucket",
                "fedcba9876543210fedcba9876543210".to_owned(),
            ),
        ]);
    }
    let body = json!({
        "message": "You are being rate limited.",
        "retry_after": seconds,
        "global": global,
    });
    response(429, body.to_string(), &headers)
}

/// Whether this attempt is the first at a 4th, 8th… message of the channel.
fn server_error(counts: &mut Counts, channel: &str) -> bool {
    let message = counts.delivered.get(channel).copied().unwrap_or(0) + 1;
    message.is_multiple_of(4) && counts.failed_once.insert((channel.to_owned(), message))
}

async fn respond(request: Request<Incoming>, faults: &Faults) -> Response<Full<Bytes>> {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let _ = request.into_body().collect().await;
    let channel = path
        .strip_prefix("/api/v10/channels/")
        .and_then(|rest| rest.strip_suffix("/messages"))
        .map(str::to_owned);
    let (Method::POST, Some(channel)) = (method, channel) else {
        return response(
            404,
            r#"{"message": "404: Not Found", "code": 0}"#.to_owned(),
            &[],
        );
    };

    let now = Instant::now();
    let mut counts = faults.counts.lock().unwrap_or_else(PoisonError::into_inner);
    counts.received += 1;
    let keys = ["global", channel.as_str()];
    record_waits(&mut counts, keys, now);
    let open = keys
        .iter()
        .filter_map(|key| counts.windows.get(*key).copied())
        .filter(|window| window.closes > now)
        .max_by_key(|window| window.closes);
    let answer = if let Some(Window { opened, closes, .. }) = open {
        if now - opened < IN_FLIGHT {
            counts.in_flight += 1;
        } else {
            counts.early += 1;
        }
        counts.limited += 1;
        let global = counts
            .windows
            .get("global")
            .is_some_and(|window| window.closes > now);
        rate_limited(closes - now, global)
    } else {
        match faults.scenario {
            Scenario::Route429 if counts.limited_channels.insert(channel.clone()) => {
                counts.limited += 1;
                counts.windows.insert(channel.clone(), Window::open(now));
                rate_limited(RETRY_AFTER, false)
            }
            Scenario::Global429 if !counts.global_done => {
                counts.global_done = true;
                counts.limited += 1;
                counts
                    .windows
                    .insert("global".to_owned(), Window::open(now));
                rate_limited(RETRY_AFTER, true)
            }
            Scenario::Server5xx if server_error(&mut counts, &channel) => {
                counts.server_errors += 1;
                Response::builder()
                    .status(502)
                    .header(CONTENT_TYPE, "text/html")
                    .body(Full::new(Bytes::from_static(
                        b"<html><body>502 Bad Gateway</body></html>",
                    )))
                    .expect("a valid response")
            }
            _ => {
                counts.ok += 1;
                *counts.delivered.entry(channel.clone()).or_default() += 1;
                let reset = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("the clock is after 1970")
                    .as_secs_f64()
                    + 1.0;
                response(
                    200,
                    sent_message(&channel, counts.received),
                    &[
                        ("x-ratelimit-limit", "5".to_owned()),
                        ("x-ratelimit-remaining", "4".to_owned()),
                        ("x-ratelimit-reset", format!("{reset:.3}")),
                        ("x-ratelimit-reset-after", "1.000".to_owned()),
                        (
                            "x-ratelimit-bucket",
                            "0123456789abcdef0123456789abcdef".to_owned(),
                        ),
                    ],
                )
            }
        }
    };
    faults.write_report(&counts);
    answer
}

/// `fault-rest --scenario S --dir D --report FILE`: serves the failing API over plain HTTP and
/// HTTPS after printing `{"api": "http://127.0.0.1:<port>/api", "tls_api": "127.0.0.1:<port>",
/// "ca": "<path>"}`.
pub(crate) async fn run(options: &Options) {
    let scenario = match options.required("scenario") {
        "route429" => Scenario::Route429,
        "global429" => Scenario::Global429,
        "server5xx" => Scenario::Server5xx,
        other => panic!("unknown --scenario {other} (route429, global429, server5xx)"),
    };
    let faults = Arc::new(Faults {
        scenario,
        report: PathBuf::from(options.required("report")),
        counts: Mutex::default(),
    });
    faults.write_report(&Counts::default());
    let (acceptor, ca) = tls::acceptor(Path::new(options.required("dir")));
    let plain = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let secure = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    println!(
        "{}",
        json!({
            "api": format!("http://{}/api", plain.local_addr().expect("bound")),
            "tls_api": secure.local_addr().expect("bound").to_string(),
            "ca": ca.display().to_string(),
        })
    );
    let handler = move |faults: Arc<Faults>| {
        move |request| {
            let faults = Arc::clone(&faults);
            async move { respond(request, &faults).await }
        }
    };
    tokio::spawn(tls::serve_with(plain, None, handler(Arc::clone(&faults))));
    tls::serve_with(secure, Some(acceptor), handler(faults)).await;
}
