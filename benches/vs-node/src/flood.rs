//! Scenario 2: an end-to-end `MESSAGE_CREATE` flood through the mock gateway.
//!
//! The measured window runs from the first counted event to the last one, so connecting,
//! `READY` and the `GUILD_CREATE` before the flood are not part of it.

use std::{
    hint::black_box,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use djs_gateway::{Compression, Config, Shard};
use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, ShardId};
use djs_rs::{Client, Events, HandlerMode};
use serde_json::{Value, json};
use tokio::sync::Notify;

use crate::{
    Options, TOKEN,
    metrics::{cpu_ms, status_kib},
};

const INTENTS: Intents = Intents::GUILDS
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::MESSAGE_CONTENT);

/// Counts events and records when the window opened and closed.
struct Window {
    count: AtomicU64,
    target: u64,
    opened: OnceLock<(Instant, f64)>,
    done: Notify,
}

impl Window {
    fn new(target: u64) -> Self {
        assert!(target >= 2, "--events must be at least 2");
        Self {
            count: AtomicU64::new(0),
            target,
            opened: OnceLock::new(),
            done: Notify::new(),
        }
    }

    /// Counts one event; returns true for the last one.
    fn hit(&self) -> bool {
        let count = self.count.fetch_add(1, Ordering::Relaxed) + 1;
        if count == 1 {
            self.opened.get_or_init(|| (Instant::now(), cpu_ms()));
        }
        let last = count == self.target;
        if last {
            self.done.notify_one();
        }
        last
    }

    /// The result line for the closed window.
    fn result(&self, library: &str, variant: &str, allocator: &str) -> Value {
        let (opened, cpu_before) = *self.opened.get().expect("the window opened");
        let seconds = opened.elapsed().as_secs_f64();
        let cpu = cpu_ms() - cpu_before;
        #[expect(
            clippy::cast_precision_loss,
            reason = "event counts are far below 2^52"
        )]
        let events = (self.target - 1) as f64;
        json!({
            "scenario": "flood",
            "library": library,
            "variant": variant,
            "allocator": allocator,
            "events": self.target,
            "seconds": seconds,
            "events_per_sec": events / seconds,
            "cpu_ms": cpu,
            "cpu_us_per_event": cpu * 1000.0 / events,
            "peak_rss_kib": status_kib("VmHWM"),
        })
    }
}

/// A djs-rs [`Client`] with one `MESSAGE_CREATE` handler, as a bot would register it.
pub(crate) async fn client(options: &Options, allocator: &str) {
    let window = Arc::new(Window::new(options.get("events", 1_000_000)));
    let sequential = options.get("mode", String::from("concurrent")) == "sequential";
    let current_thread = options.get("runtime", String::from("multi")) == "current";
    let client = Client::builder(TOKEN)
        .intents(INTENTS)
        .gateway_url(options.required("gateway"))
        .compression(Compression::None)
        .handler_mode(if sequential {
            HandlerMode::Sequential
        } else {
            HandlerMode::Concurrent
        })
        .build();
    let counter = Arc::clone(&window);
    client.on(Events::MessageCreate, move |_ctx, message| {
        let counter = Arc::clone(&counter);
        async move {
            black_box(&message.content);
            counter.hit();
            Ok(())
        }
    });
    let running = client.clone();
    tokio::spawn(async move { running.login().await });
    window.done.notified().await;
    let variant = match (sequential, current_thread) {
        (false, false) => "Client + messageCreate handler",
        (true, false) => "Client + messageCreate handler, HandlerMode::Sequential",
        (false, true) => "Client + messageCreate handler, current-thread runtime",
        (true, true) => "Client + messageCreate handler, Sequential, current-thread runtime",
    };
    println!("{}", window.result("djs-rs", variant, allocator));
    // Exit without the clean disconnect; the mock server does not care.
    std::process::exit(0);
}

/// A bare djs-gateway [`Shard`], the layer the `Client` is built on (the counterpart of
/// `@discordjs/ws`).
pub(crate) async fn shard(options: &Options, allocator: &str) {
    let window = Window::new(options.get("events", 1_000_000));
    let config = Config::builder(TOKEN, INTENTS)
        .gateway_url(options.required("gateway"))
        .compression(Compression::None)
        .build();
    let mut shard = Shard::with_config(ShardId::ONE, config);
    while let Some(event) = shard.next_event().await {
        if let Ok(GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message))) = event {
            black_box(&message.content);
            if window.hit() {
                break;
            }
        }
    }
    println!(
        "{}",
        window.result("djs-rs", "djs-gateway Shard (typed events)", allocator)
    );
    std::process::exit(0);
}
