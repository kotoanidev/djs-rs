//! Extreme-load tests of the REST client and its rate limiter (D-041).
//!
//! These tests are ignored by default because they take seconds and saturate the machine. Run
//! them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rest --release --test stress -- --ignored --nocapture --test-threads 1
//! ```
//!
//! The mock server enforces Discord-like rate limits and counts every request that would have
//! been rejected by Discord, so the tests check correctness under load, not only speed.

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_model::{id::ChannelId, rest::CreateMessage};
use djs_rest::Rest;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, sync::Semaphore, task::JoinSet};

/// How the mock server limits requests.
#[derive(Clone, Copy)]
enum Mode {
    /// No rate-limit headers at all.
    Unlimited,
    /// Per-channel buckets: `limit` requests per `window`, reported through headers.
    Buckets { limit: u32, window: Duration },
    /// The first request of every channel is answered with a (non-global) 429.
    FirstRequestLimited { retry_after: Duration },
}

#[derive(Default)]
struct Window {
    started: Option<Instant>,
    used: u32,
    limited_once: bool,
}

struct Server {
    mode: Mode,
    windows: Mutex<HashMap<u64, Window>>,
    requests: AtomicU64,
    rejected: AtomicU64,
    arrivals: Mutex<Vec<Instant>>,
}

const MESSAGE: &str = r#"{"id":"99","channel_id":"20","author":{"id":"1","username":"bot","discriminator":"0","global_name":null,"avatar":null,"bot":true},"content":"x","timestamp":"2026-11-10T19:42:08.000000+00:00","edited_timestamp":null,"tts":false,"mention_everyone":false,"pinned":false,"type":0}"#;

/// Starts the mock server and returns a client for it.
async fn start(
    mode: Mode,
    configure: impl FnOnce(djs_rest::RestBuilder) -> Rest,
) -> (Rest, Arc<Server>) {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let server = Arc::new(Server {
        mode,
        windows: Mutex::default(),
        requests: AtomicU64::new(0),
        rejected: AtomicU64::new(0),
        arrivals: Mutex::default(),
    });
    let state = Arc::clone(&server);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            stream.set_nodelay(true).unwrap();
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let state = Arc::clone(&state);
                    async move { Ok::<_, Infallible>(handle(&state, request).await) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    let rest = configure(
        Rest::builder()
            .token("stress-token")
            .api(format!("http://{address}/api")),
    );
    (rest, server)
}

async fn handle(server: &Server, request: Request<Incoming>) -> Response<Full<Bytes>> {
    let now = Instant::now();
    server.requests.fetch_add(1, Ordering::Relaxed);
    let channel: u64 = request
        .uri()
        .path()
        .split('/')
        .nth(4)
        .and_then(|id| id.parse().ok())
        .unwrap_or(0);
    let _ = request.into_body().collect().await;
    server.arrivals.lock().unwrap().push(now);

    let mut response = Response::builder().header("content-type", "application/json");
    let mut status = StatusCode::OK;
    match server.mode {
        Mode::Unlimited => {}
        Mode::Buckets { limit, window } => {
            let mut windows = server.windows.lock().unwrap();
            let state = windows.entry(channel).or_default();
            let started = match state.started {
                Some(started) if now < started + window => started,
                _ => {
                    state.used = 0;
                    *state.started.insert(now)
                }
            };
            state.used += 1;
            let reset_after = (started + window).saturating_duration_since(now);
            // Round up to whole milliseconds, like Discord's three-decimal header.
            let reset_after = reset_after.as_millis() as f64 / 1000.0 + 0.001;
            if state.used > limit {
                status = StatusCode::TOO_MANY_REQUESTS;
                server.rejected.fetch_add(1, Ordering::Relaxed);
            }
            response = response
                .header("x-ratelimit-bucket", "stress-bucket")
                .header("x-ratelimit-limit", limit.to_string())
                .header(
                    "x-ratelimit-remaining",
                    limit.saturating_sub(state.used).to_string(),
                )
                .header("x-ratelimit-reset-after", format!("{reset_after:.3}"));
            if status == StatusCode::TOO_MANY_REQUESTS {
                response = response.header("retry-after", format!("{reset_after:.3}"));
                let body = format!(
                    r#"{{"message":"You are being rate limited.","retry_after":{reset_after:.3},"global":false}}"#
                );
                return response.status(status).body(Full::from(body)).unwrap();
            }
        }
        Mode::FirstRequestLimited { retry_after } => {
            let mut windows = server.windows.lock().unwrap();
            let state = windows.entry(channel).or_default();
            if !state.limited_once {
                state.limited_once = true;
                server.rejected.fetch_add(1, Ordering::Relaxed);
                let seconds = retry_after.as_secs_f64();
                let body = format!(
                    r#"{{"message":"You are being rate limited.","retry_after":{seconds:.3},"global":false}}"#
                );
                return response
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("x-ratelimit-bucket", "stress-bucket")
                    .header("x-ratelimit-remaining", "0")
                    .header("x-ratelimit-reset-after", format!("{seconds:.3}"))
                    .header("x-ratelimit-scope", "shared")
                    .header("retry-after", format!("{seconds:.3}"))
                    .body(Full::from(body))
                    .unwrap();
            }
        }
    }
    response
        .status(status)
        .body(Full::from(Bytes::from_static(MESSAGE.as_bytes())))
        .unwrap()
}

/// Sends `per_channel` messages to each of `channels` channels with at most `concurrency`
/// requests in flight, returning the elapsed time.
async fn flood(rest: &Rest, channels: u64, per_channel: u64, concurrency: usize) -> Duration {
    let permits = Arc::new(Semaphore::new(concurrency));
    let body = Arc::new(CreateMessage::from("stress"));
    let started = Instant::now();
    let mut tasks = JoinSet::new();
    for round in 0..per_channel {
        for channel in 1..=channels {
            let (rest, body, permits) = (rest.clone(), Arc::clone(&body), Arc::clone(&permits));
            tasks.spawn(async move {
                let _permit = permits.acquire_owned().await.unwrap();
                rest.create_message(ChannelId::new(channel), &body)
                    .await
                    .unwrap_or_else(|error| panic!("round {round}, channel {channel}: {error:?}"))
            });
        }
    }
    while let Some(result) = tasks.join_next().await {
        result.unwrap();
    }
    started.elapsed()
}

/// `(VmRSS, VmHWM)` of this process in KiB, from `/proc/self/status` (Linux only).
fn memory_kib() -> Option<(u64, u64)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

/// Resets `VmHWM` to the current RSS so the next reading shows the peak of the measured part.
fn reset_peak_memory() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn report(name: &str, requests: u64, elapsed: Duration, server: &Server) {
    let memory = memory_kib().map_or_else(
        || "n/a".to_owned(),
        |(rss, peak)| format!("rss {} MiB, peak {} MiB", rss / 1024, peak / 1024),
    );
    println!(
        "[stress] {name}: {requests} requests in {elapsed:.2?} = {:.0} req/s; \
         server saw {}, rejected {}; {memory}",
        requests as f64 / elapsed.as_secs_f64(),
        server.requests.load(Ordering::Relaxed),
        server.rejected.load(Ordering::Relaxed),
    );
}

/// Raw throughput: 200 000 requests over 10 000 channels (10 000 buckets, so idle-bucket
/// sweeping runs) with 256 in flight, and no rate limits on the server.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn throughput_200k_requests_10k_buckets() {
    let (rest, server) = start(Mode::Unlimited, |builder| {
        builder.global_requests_per_second(0).build()
    })
    .await;
    // Warm up the connection pool.
    flood(&rest, 256, 1, 256).await;
    server.requests.store(0, Ordering::Relaxed);
    reset_peak_memory();

    let elapsed = flood(&rest, 10_000, 20, 256).await;
    report("throughput", 200_000, elapsed, &server);
    assert_eq!(server.requests.load(Ordering::Relaxed), 200_000);
}

/// 10 000 requests queued at once on 100 channels whose buckets allow 10 requests per 100 ms.
/// The limiter must not trigger a single 429.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn bucket_limits_hold_under_10k_queued_requests() {
    let (limit, window) = (10, Duration::from_millis(100));
    let (rest, server) = start(Mode::Buckets { limit, window }, |builder| {
        builder
            .global_requests_per_second(0)
            .offset(Duration::from_millis(5))
            .build()
    })
    .await;
    reset_peak_memory();
    let elapsed = flood(&rest, 100, 100, 10_000).await;
    report("bucket limits", 10_000, elapsed, &server);
    assert_eq!(
        server.rejected.load(Ordering::Relaxed),
        0,
        "the limiter let a 429 through"
    );
    // 100 requests per channel at 10 per window need at least 9 full windows.
    assert!(elapsed >= window * 9, "finished too fast: {elapsed:?}");
}

/// The first request of each of 1 000 channels is rate limited; every request must still
/// succeed and each channel must be rejected exactly once (no retry storms).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn recovers_from_a_429_storm() {
    let (rest, server) = start(
        Mode::FirstRequestLimited {
            retry_after: Duration::from_millis(50),
        },
        |builder| builder.global_requests_per_second(0).build(),
    )
    .await;
    reset_peak_memory();
    let elapsed = flood(&rest, 1_000, 5, 1_000).await;
    report("429 storm", 5_000, elapsed, &server);
    assert_eq!(server.rejected.load(Ordering::Relaxed), 1_000);
    assert_eq!(server.requests.load(Ordering::Relaxed), 6_000);
}

/// 250 requests to 250 different channels, all queued at once, with the default global limit
/// of 50 requests per second. Reports the largest number of requests the server saw in any
/// sliding one-second window.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn global_limit_holds() {
    let (rest, server) = start(Mode::Unlimited, djs_rest::RestBuilder::build).await;
    let elapsed = flood(&rest, 250, 1, 250).await;
    report("global limit", 250, elapsed, &server);

    let mut arrivals = server.arrivals.lock().unwrap().clone();
    arrivals.sort_unstable();
    let mut max_in_window = 0;
    let mut start = 0;
    for end in 0..arrivals.len() {
        while arrivals[end] - arrivals[start] >= Duration::from_secs(1) {
            start += 1;
        }
        max_in_window = max_in_window.max(end - start + 1);
    }
    println!("[stress] global limit: max {max_in_window} requests in any sliding 1 s window");
    // 250 requests at 50/s need at least four full windows after the first burst.
    assert!(
        elapsed >= Duration::from_secs(4),
        "finished too fast: {elapsed:?}"
    );
    // The limiter uses fixed one-second windows (like discord.js), so a sliding window that
    // spans a window boundary can contain up to two windows' worth of requests. D-043 replaces it
    // with a strict GCRA limiter in Phase 3; this assertion then becomes `<= 50`.
    assert!(
        max_in_window <= 100,
        "{max_in_window} requests in one second"
    );
}

/// Request latency without contention: 20 000 requests one after another on one channel.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn sequential_latency() {
    let (rest, server) = start(Mode::Unlimited, |builder| {
        builder.global_requests_per_second(0).build()
    })
    .await;
    flood(&rest, 1, 100, 1).await;
    server.requests.store(0, Ordering::Relaxed);
    let elapsed = flood(&rest, 1, 20_000, 1).await;
    report("sequential", 20_000, elapsed, &server);
    println!(
        "[stress] sequential: {:.1} µs per request round trip",
        elapsed.as_secs_f64() * 1e6 / 20_000.0
    );
}

/// Prints the size of the futures returned by the client; every queued request holds one.
#[test]
#[ignore = "stress test; run with --release --ignored"]
fn future_sizes() {
    let rest = Rest::new("token");
    let body = CreateMessage::from("x");
    let create = rest.create_message(ChannelId::new(1), &body);
    let gateway = rest.get_gateway_bot();
    println!(
        "[stress] future sizes: create_message {} B, get_gateway_bot {} B",
        size_of_val(&create),
        size_of_val(&gateway)
    );
}
