//! The clients of the reliability scenarios (D-279), the same for both libraries: a bot that
//! counts the numbered messages of [`fault_gateway`](crate::fault_gateway), and a sender of
//! messages through [`fault_rest`](crate::fault_rest).
//!
//! | Subcommand | What it does |
//! |---|---|
//! | `reliability --library L --scenario S --messages N [--timeout-s T]` | Receives the messages; reports the distinct, duplicated and missing ones, the longest pause between two, and whether the client stopped by itself. |
//! | `rest-faults --library L --scenario S --requests N --concurrency C` | Sends the messages; reports how many were sent and the errors. |
//!
//! djs-rs takes `--gateway URL` / `--api URL`; serenity takes `--tls ADDRESS --ca FILE`
//! (D-282) and its default configuration.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, OnceLock, PoisonError},
    time::{Duration, Instant},
};

use djs_rs::{
    Events, Shards,
    gateway::Compression,
    model::{gateway::Intents, id::ChannelId as DjsChannelId, rest::CreateMessage},
    rest::Rest,
};
use djs_vs_node::{Options, TOKEN, payloads::channel_id};
use serde_json::{Value, json};
use serenity::all::{
    ChannelId, ClientBuilder, Context, EventHandler, GatewayIntents, HttpBuilder, Message,
    async_trait,
};
use tokio::sync::Notify;

use crate::tls;

/// What the bot received.
#[derive(Debug, Default)]
struct Received {
    seen: Vec<bool>,
    distinct: u64,
    duplicates: u64,
    unnumbered: u64,
    last: Option<Instant>,
    longest_pause: Duration,
}

/// Counts the numbered messages a bot receives.
#[derive(Debug)]
struct Tracker {
    expected: u64,
    started: Instant,
    received: Mutex<Received>,
    complete: Notify,
    stopped: OnceLock<(f64, String)>,
    stop: Notify,
}

impl Tracker {
    fn new(expected: u64) -> Arc<Self> {
        Arc::new(Self {
            expected,
            started: Instant::now(),
            received: Mutex::new(Received {
                seen: vec![false; usize::try_from(expected + 1).expect("a small count")],
                ..Received::default()
            }),
            complete: Notify::new(),
            stopped: OnceLock::new(),
            stop: Notify::new(),
        })
    }

    /// Records a message by the number in its text (`message number k: …`).
    fn record(&self, content: &str) {
        let index = content
            .strip_prefix("message number ")
            .and_then(|rest| rest.split(':').next())
            .and_then(|number| number.parse::<usize>().ok());
        let now = Instant::now();
        let mut received = self.received.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(last) = received.last {
            received.longest_pause = received.longest_pause.max(now - last);
        }
        received.last = Some(now);
        match index.filter(|&index| index > 0 && index < received.seen.len()) {
            Some(index) if received.seen[index] => received.duplicates += 1,
            Some(index) => {
                received.seen[index] = true;
                received.distinct += 1;
                if received.distinct == self.expected {
                    self.complete.notify_one();
                }
            }
            None => received.unnumbered += 1,
        }
    }

    /// Records that the client stopped by itself (its login or start returned).
    fn stopped(&self, outcome: String) {
        let ms = self.started.elapsed().as_secs_f64() * 1000.0;
        let _ = self.stopped.set((ms, outcome));
        self.stop.notify_one();
    }

    /// Waits for every message, the client's stop or the timeout.
    async fn wait(&self, timeout: Duration) {
        tokio::select! {
            () = self.complete.notified() => {}
            () = self.stop.notified() => {}
            () = tokio::time::sleep(timeout) => {}
        }
    }

    fn result(&self, library: &str, scenario: &str) -> Value {
        let received = self.received.lock().unwrap_or_else(PoisonError::into_inner);
        let stopped = self.stopped.get();
        json!({
            "scenario": "reliability",
            "fault": scenario,
            "library": library,
            "expected": self.expected,
            "distinct": received.distinct,
            "missing": self.expected - received.distinct,
            "duplicates": received.duplicates,
            "unnumbered": received.unnumbered,
            "longest_pause_ms": received.longest_pause.as_secs_f64() * 1000.0,
            "stopped_ms": stopped.map(|(ms, _)| *ms),
            "stop_outcome": stopped.map(|(_, outcome)| outcome.clone()),
            "seconds": self.started.elapsed().as_secs_f64(),
        })
    }
}

/// serenity's handler: counts `message` calls.
struct Counter(Arc<Tracker>);

#[async_trait]
impl EventHandler for Counter {
    async fn message(&self, _ctx: Context, message: Message) {
        self.0.record(&message.content);
    }
}

/// `reliability`: receives the numbered messages of a running `fault-gateway`.
pub(crate) async fn gateway(options: &Options) {
    let library = options.required("library").to_owned();
    let scenario = options.required("scenario").to_owned();
    let tracker = Tracker::new(options.get("messages", 3000));
    let timeout = Duration::from_secs(options.get("timeout-s", 20));
    match library.as_str() {
        "djs-rs" => {
            let url = options.required("gateway");
            assert!(
                url.starts_with("ws://127.0.0.1:"),
                "only a local mock gateway"
            );
            let client = djs_rs::Client::builder(TOKEN)
                .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
                .gateway_url(url)
                .shards(Shards::Count(1))
                .compression(Compression::None)
                .build();
            let counter = Arc::clone(&tracker);
            client.on(Events::MessageCreate, move |_ctx, message| {
                let counter = Arc::clone(&counter);
                async move {
                    counter.record(&message.content);
                    Ok(())
                }
            });
            let stopper = Arc::clone(&tracker);
            tokio::spawn(async move {
                let outcome = match client.login().await {
                    Ok(()) => "returned Ok".to_owned(),
                    Err(error) => format!("returned an error: {error}"),
                };
                stopper.stopped(outcome);
            });
        }
        "serenity" => {
            let http = HttpBuilder::new(TOKEN).client(tls::client(options)).build();
            let gateway = http
                .get_gateway()
                .await
                .expect("the mock answers Get Gateway");
            assert!(
                gateway.url.starts_with("ws://127.0.0.1:"),
                "only a local mock gateway is allowed, got {}",
                gateway.url
            );
            let intents = GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;
            let mut client = ClientBuilder::new_with_http(http, intents)
                .event_handler(Counter(Arc::clone(&tracker)))
                .await
                .expect("the client is built");
            let stopper = Arc::clone(&tracker);
            tokio::spawn(async move {
                let outcome = match client.start().await {
                    Ok(()) => "returned Ok".to_owned(),
                    Err(error) => format!("returned an error: {error}"),
                };
                stopper.stopped(outcome);
            });
        }
        other => panic!("unknown --library {other} (djs-rs, serenity)"),
    }
    tracker.wait(timeout).await;
    println!("{}", tracker.result(&library, &scenario));
    std::process::exit(0);
}

/// `rest-faults`: sends `--requests` messages from `--concurrency` workers, each to its own
/// channel, through a running `fault-rest`, with each library's default configuration.
pub(crate) async fn rest(options: &Options) {
    let library = options.required("library").to_owned();
    let scenario = options.required("scenario").to_owned();
    let requests: u64 = options.get("requests", 100);
    let concurrency: u64 = options.get("concurrency", 4);
    let per_worker = requests / concurrency;
    let started = Instant::now();
    let mut workers = Vec::new();
    match library.as_str() {
        "djs-rs" => {
            let api = options.required("api");
            assert!(
                api.starts_with("http://127.0.0.1:"),
                "only a local mock API"
            );
            let rest = Rest::builder().token(TOKEN).api(api).build();
            for worker in 0..concurrency {
                let rest = rest.clone();
                workers.push(tokio::spawn(async move {
                    let channel = DjsChannelId::new(channel_id(0, worker));
                    let body = CreateMessage::from("reliability");
                    let mut outcomes = Vec::new();
                    for _ in 0..per_worker {
                        outcomes.push(
                            rest.create_message(channel, &body)
                                .await
                                .map(drop)
                                .map_err(|error| error.to_string()),
                        );
                    }
                    outcomes
                }));
            }
        }
        "serenity" => {
            let http = Arc::new(HttpBuilder::new(TOKEN).client(tls::client(options)).build());
            for worker in 0..concurrency {
                let http = Arc::clone(&http);
                workers.push(tokio::spawn(async move {
                    let channel = ChannelId::new(channel_id(0, worker));
                    let body = json!({ "content": "reliability" });
                    let mut outcomes = Vec::new();
                    for _ in 0..per_worker {
                        outcomes.push(
                            http.send_message(channel, Vec::new(), &body)
                                .await
                                .map(drop)
                                .map_err(|error| error.to_string()),
                        );
                    }
                    outcomes
                }));
            }
        }
        other => panic!("unknown --library {other} (djs-rs, serenity)"),
    }
    let (mut sent, mut failed) = (0_u64, 0_u64);
    let mut errors = BTreeSet::new();
    for worker in workers {
        for outcome in worker.await.expect("a worker finished") {
            match outcome {
                Ok(()) => sent += 1,
                Err(error) => {
                    failed += 1;
                    errors.insert(error.chars().take(160).collect::<String>());
                }
            }
        }
    }
    println!(
        "{}",
        json!({
            "scenario": "rest-faults",
            "fault": scenario,
            "library": library,
            "requests": per_worker * concurrency,
            "sent": sent,
            "failed": failed,
            "errors": errors,
            "seconds": started.elapsed().as_secs_f64(),
        })
    );
}
