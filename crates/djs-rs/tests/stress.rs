//! Extreme-load tests of the high-level `Client`: event floods through handlers (D-041).
//!
//! These tests are ignored by default because they take seconds and saturate the machine. Run
//! them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rs --release --test stress -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use std::convert::Infallible;

use bytes::Bytes;
use djs_rs::{
    HandlerMode, Shards, cache::CacheConfig, gateway::Compression, prelude::*, rest::Rest,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Notify,
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder};

const STEP: Duration = Duration::from_secs(60);

const HELLO: &str = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":600000}}"#;
const READY: &str = r#"{"t":"READY","s":1,"op":0,"d":{"v":10,"user":{"id":"1","username":"stress-bot","discriminator":"0","global_name":null,"avatar":null,"bot":true},"guilds":[],"session_id":"s","resume_gateway_url":"ws://127.0.0.1:9","shard":[0,1],"application":{"id":"1","flags":0}}}"#;

/// A realistic message dispatch (≈ 700 bytes) named `name` (`MESSAGE_UPDATE` edits one of
/// 100 messages, so the cache holds a previous version once they were all seen).
fn message_dispatch(name: &str, sequence: u64) -> String {
    let id = if name == "MESSAGE_UPDATE" {
        sequence % 100
    } else {
        sequence
    };
    format!(
        r#"{{"t":"{name}","s":{sequence},"op":0,"d":{{"type":0,"tts":false,"timestamp":"2026-11-10T19:42:07.123000+00:00","pinned":false,"nonce":"{sequence}","mentions":[],"mention_roles":[],"mention_everyone":false,"member":{{"roles":["1100000000000000001","1100000000000000002"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"banner":null,"avatar":null}},"id":"{}","flags":0,"embeds":[],"edited_timestamp":null,"content":"stress message number {sequence}","components":[],"channel_id":"1200000000000000020","author":{{"username":"nelly","public_flags":0,"id":"1300000000000000003","global_name":"Nelly","discriminator":"0","clan":null,"avatar_decoration_data":null,"avatar":"a_1269e74af4df7417b13759eae50c83dc"}},"attachments":[],"guild_id":"1400000000000000002"}}}}"#,
        1_500_000_000_000_000_000_u64 + id
    )
}

/// Serves `sessions` sessions that each send `events` `flood` message dispatches after READY,
/// then one `MESSAGE_UPDATE` if `tail`.
async fn serve(listener: TcpListener, sessions: u32, events: u64, flood: &'static str, tail: bool) {
    let mut tasks = Vec::new();
    for _ in 0..sessions {
        let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
        tasks.push(tokio::spawn(serve_session(stream, events, flood, tail)));
    }
    for task in tasks {
        task.await.unwrap();
    }
}

/// A mock REST API whose Get Gateway Bot recommends `shards` shards, all allowed to identify
/// at once. Returns its base URL.
async fn mock_rest(gateway_url: String, shards: u32) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    let body = Bytes::from(format!(
        r#"{{"url":"{gateway_url}","shards":{shards},"session_start_limit":{{"total":1000,"remaining":1000,"reset_after":0,"max_concurrency":{shards}}}}}"#
    ));
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let body = body.clone();
            tokio::spawn(async move {
                let service = service_fn(move |_: Request<Incoming>| {
                    let body = body.clone();
                    async move { Ok::<_, Infallible>(Response::new(Full::new(body))) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    base
}

async fn serve_session(stream: TcpStream, events: u64, flood: &'static str, tail: bool) {
    stream.set_nodelay(true).unwrap();
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    ws.send(Message::text(HELLO.to_owned())).await.unwrap();
    loop {
        let message = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
        if message
            .as_text()
            .is_some_and(|text| text.contains(r#""op":2"#))
        {
            break;
        }
    }
    ws.send(Message::text(READY.to_owned())).await.unwrap();
    if flood == "MESSAGE_UPDATE" {
        // The messages the updates edit, so that the cache holds a previous version.
        for id in 0..100 {
            ws.feed(Message::text(message_dispatch("MESSAGE_CREATE", id)))
                .await
                .unwrap();
        }
    }
    for sequence in 2..events + 2 {
        ws.feed(Message::text(message_dispatch(flood, sequence)))
            .await
            .unwrap();
        if sequence % 64 == 0 {
            ws.flush().await.unwrap();
        }
    }
    if tail {
        ws.feed(Message::text(message_dispatch(
            "MESSAGE_UPDATE",
            events + 2,
        )))
        .await
        .unwrap();
    }
    ws.flush().await.unwrap();
    while let Ok(Some(Ok(_))) = timeout(STEP, ws.next()).await {}
}

/// Counts handler invocations and wakes the test once `target` is reached.
#[derive(Clone)]
struct Counter {
    count: Arc<AtomicU64>,
    target: u64,
    done: Arc<Notify>,
}

impl Counter {
    fn new(target: u64) -> Self {
        Self {
            count: Arc::default(),
            target,
            done: Arc::default(),
        }
    }

    fn hit(&self) {
        if self.count.fetch_add(1, Ordering::Relaxed) + 1 == self.target {
            self.done.notify_one();
        }
    }
}

/// How many `MessageUpdate`s had a previous message.
static WITH_OLD: AtomicU64 = AtomicU64::new(0);

/// What the counting handlers of a flood listen to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Listen {
    /// `MESSAGE_CREATE`s, as `MessageCreate`.
    Create,
    /// `MESSAGE_UPDATE`s, as `MessageUpdate` with the previous message (D-221).
    Update,
    /// `MESSAGE_CREATE`s, as `Raw` (a clone of every dispatch).
    Raw,
    /// Nothing of the flood: one handler waits for a final `MESSAGE_UPDATE`, so the
    /// `MESSAGE_CREATE`s are only decoded (and cached), without building a payload.
    Nothing,
}

/// Floods a client running `shards` shards with `events` messages in total, registering
/// `handlers` counting handlers, and optionally consuming the event stream as well. Returns the
/// elapsed time from `login` to the last handler call.
///
/// One shard connects straight to the mock gateway; more shards go through Get Gateway Bot
/// (`Shards::Auto`, as a bot does by default), which recommends that many.
#[expect(
    clippy::too_many_lines,
    reason = "a test harness with one branch per listener"
)]
async fn run(
    listen: Listen,
    shards: u32,
    events: u64,
    handlers: u64,
    mode: HandlerMode,
    stream: bool,
    cache: bool,
) -> Duration {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let per_shard = events / u64::from(shards);
    let flood = if listen == Listen::Update {
        "MESSAGE_UPDATE"
    } else {
        "MESSAGE_CREATE"
    };
    let server = tokio::spawn(serve(
        listener,
        shards,
        per_shard,
        flood,
        listen == Listen::Nothing,
    ));

    let mut builder = Client::builder("stress-token")
        .intents(Intents::GUILD_MESSAGES)
        .compression(Compression::None)
        .handler_mode(mode);
    if cache {
        // What discord.js caches by default: every message is cached (200 per channel, so
        // the flood's channel evicts its oldest), with its author and member.
        builder = builder.cache(CacheConfig::discord_js());
    }
    let client = if shards == 1 {
        builder.gateway_url(url).shards(Shards::Count(1)).build()
    } else {
        let rest = mock_rest(url, shards).await;
        builder
            .rest(Rest::builder().token("stress-token").api(rest).build())
            .build()
    };
    let target = if listen == Listen::Nothing {
        u64::from(shards) * handlers
    } else {
        events * handlers
    };
    let counter = Counter::new(target);
    for _ in 0..handlers {
        let counter = counter.clone();
        match listen {
            Listen::Create => {
                client.on(Events::MessageCreate, move |_ctx, msg| {
                    let counter = counter.clone();
                    async move {
                        std::hint::black_box(&msg.content);
                        counter.hit();
                        Ok(())
                    }
                });
            }
            Listen::Update | Listen::Nothing => {
                client.on(Events::MessageUpdate, move |_ctx, update| {
                    let counter = counter.clone();
                    async move {
                        if update.old.is_some() {
                            WITH_OLD.fetch_add(1, Ordering::Relaxed);
                        }
                        std::hint::black_box((&update.content, &update.old));
                        counter.hit();
                        Ok(())
                    }
                });
            }
            Listen::Raw => {
                client.on(Events::Raw, move |_ctx, raw| {
                    let counter = counter.clone();
                    async move {
                        if let djs_rs::model::gateway::DispatchEvent::MessageCreate(msg) = &*raw {
                            std::hint::black_box(&msg.content);
                            counter.hit();
                        }
                        Ok(())
                    }
                });
            }
        }
    }
    let streamed = Counter::new(events);
    if stream {
        let mut events = client.events();
        let streamed = streamed.clone();
        tokio::spawn(async move {
            while let Some(event) = events.next().await {
                if event.event.name() == "messageCreate" {
                    streamed.hit();
                }
            }
        });
    }

    let started = Instant::now();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    timeout(STEP, counter.done.notified())
        .await
        .expect("handlers did not finish in time");
    if stream {
        timeout(STEP, streamed.done.notified())
            .await
            .expect("stream did not finish in time");
    }
    let elapsed = started.elapsed();
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    server.await.unwrap();
    elapsed
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

async fn measure(
    name: &str,
    shards: u32,
    events: u64,
    handlers: u64,
    mode: HandlerMode,
    stream: bool,
    cache: bool,
) {
    measure_listening(
        name,
        Listen::Create,
        events,
        mode,
        cache,
        (shards, handlers, stream),
    )
    .await;
}

/// [`measure`] for a flood whose handlers listen to `listen`; `rest` is (shards, handlers,
/// stream).
async fn measure_listening(
    name: &str,
    listen: Listen,
    events: u64,
    mode: HandlerMode,
    cache: bool,
    (shards, handlers, stream): (u32, u64, bool),
) {
    let before = memory_kib();
    reset_peak_memory();
    WITH_OLD.store(0, Ordering::Relaxed);
    let elapsed = run(listen, shards, events, handlers, mode, stream, cache).await;
    let memory = match (before, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {} → {} MiB, peak {} MiB",
            before / 1024,
            after / 1024,
            peak / 1024
        ),
        _ => "memory n/a".to_owned(),
    };
    let seconds = elapsed.as_secs_f64();
    println!(
        "[stress] {name}: {events} events × {handlers} handler(s) in {elapsed:.2?} = \
         {:.0} events/s, {:.0} handler calls/s; {memory}",
        events as f64 / seconds,
        (events * handlers) as f64 / seconds,
    );
    if listen == Listen::Update {
        println!(
            "[stress] {name}: {} of the updates had the previous message",
            WITH_OLD.load(Ordering::Relaxed)
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_concurrent_handlers() {
    measure(
        "concurrent",
        1,
        1_000_000,
        1,
        HandlerMode::Concurrent,
        false,
        false,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_sequential_handlers() {
    measure(
        "sequential",
        1,
        1_000_000,
        1,
        HandlerMode::Sequential,
        false,
        false,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_200k_fan_out_to_10_handlers() {
    measure(
        "fan-out",
        1,
        200_000,
        10,
        HandlerMode::Concurrent,
        false,
        false,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_handlers_and_event_stream() {
    measure(
        "handler + stream",
        1,
        1_000_000,
        1,
        HandlerMode::Concurrent,
        true,
        false,
    )
    .await;
}

/// Automatic sharding (D-146): 16 shards, each in its own task, 1M events in total.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_over_16_shards_concurrent() {
    measure(
        "16 shards, concurrent",
        16,
        1_000_000,
        1,
        HandlerMode::Concurrent,
        false,
        false,
    )
    .await;
}

/// The same with sequential handlers: each shard runs its handlers in its own task.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_over_16_shards_sequential() {
    measure(
        "16 shards, sequential",
        16,
        1_000_000,
        1,
        HandlerMode::Sequential,
        false,
        false,
    )
    .await;
}

/// The sequential flood with discord.js' default cache: every event is applied to the cache
/// before its handler runs (D-171).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_sequential_with_cache() {
    measure(
        "sequential + cache",
        1,
        1_000_000,
        1,
        HandlerMode::Sequential,
        false,
        true,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_concurrent_with_cache() {
    measure(
        "concurrent + cache",
        1,
        1_000_000,
        1,
        HandlerMode::Concurrent,
        false,
        true,
    )
    .await;
}

/// Phase 8 (D-221): every `MESSAGE_UPDATE` hands the previous message from the cache.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_message_updates_with_the_previous_state() {
    for (name, cache) in [
        ("updates, no cache", false),
        ("updates + cache (old)", true),
    ] {
        measure_listening(
            name,
            Listen::Update,
            1_000_000,
            HandlerMode::Sequential,
            cache,
            (1, 1, false),
        )
        .await;
    }
}

/// Phase 8 (C-091): a `Raw` handler costs a clone of every dispatch.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_raw_handler() {
    measure_listening(
        "raw",
        Listen::Raw,
        1_000_000,
        HandlerMode::Sequential,
        false,
        (1, 1, false),
    )
    .await;
}

/// Phase 8 (C-091): events nobody listens to are decoded (and cached) but no payload is built.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn flood_1m_unwatched_events() {
    for (name, cache) in [("unwatched", false), ("unwatched + cache", true)] {
        measure_listening(
            name,
            Listen::Nothing,
            1_000_000,
            HandlerMode::Sequential,
            cache,
            (1, 1, false),
        )
        .await;
    }
}
