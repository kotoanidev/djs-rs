//! Extreme-load tests of the collectors (D-041, D-264): what the index costs the event path
//! with none, many unrelated and one busy collector, a storm of clicks on one collector,
//! registering and dropping collectors, and a 100 000-member `fetch_all`.
//!
//! Ignored by default; run them in release mode, one at a time with pauses (they load every
//! core), and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rs --release --test stress_collectors -- --ignored --nocapture --test-threads 1
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

use djs_rs::{
    HandlerMode, Shards,
    cache::{CacheConfig, Limit},
    gateway::Compression,
    model::id::{ChannelId, GuildId, MessageId},
    prelude::*,
    rest::Rest,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::{
    net::TcpListener,
    sync::{Notify, mpsc},
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder};

const STEP: Duration = Duration::from_secs(120);

const HELLO: &str = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":600000}}"#;
const READY: &str = r#"{"t":"READY","s":1,"op":0,"d":{"v":10,"user":{"id":"1","username":"stress-bot","discriminator":"0","global_name":null,"avatar":null,"bot":true},"guilds":[],"session_id":"s","resume_gateway_url":"ws://127.0.0.1:9","shard":[0,1],"application":{"id":"1","flags":0}}}"#;

/// The flood's guild, channel and the message its clicks are on.
const GUILD: u64 = 1_400_000_000_000_000_002;
const CHANNEL: u64 = 1_200_000_000_000_000_020;
const MENU: u64 = 1_500_000_000_000_000_001;

/// A realistic message dispatch (≈ 700 bytes) in the flood's channel.
fn message_dispatch(sequence: u64) -> String {
    format!(
        r#"{{"t":"MESSAGE_CREATE","s":{sequence},"op":0,"d":{{"type":0,"tts":false,"timestamp":"2026-11-10T19:42:07.123000+00:00","pinned":false,"nonce":"{sequence}","mentions":[],"mention_roles":[],"mention_everyone":false,"member":{{"roles":["1100000000000000001","1100000000000000002"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"banner":null,"avatar":null}},"id":"{}","flags":0,"embeds":[],"edited_timestamp":null,"content":"stress message number {sequence}","components":[],"channel_id":"{CHANNEL}","author":{{"username":"nelly","public_flags":0,"id":"1300000000000000003","global_name":"Nelly","discriminator":"0","clan":null,"avatar_decoration_data":null,"avatar":"a_1269e74af4df7417b13759eae50c83dc"}},"attachments":[],"guild_id":"{GUILD}"}}}}"#,
        2_000_000_000_000_000_000_u64 + sequence
    )
}

/// A realistic button click (≈ 1.6 KB) on the menu message, by one of 1 000 users.
fn click_dispatch(sequence: u64) -> String {
    let user = 1_300_000_000_000_000_000_u64 + sequence % 1000;
    format!(
        r#"{{"t":"INTERACTION_CREATE","s":{sequence},"op":0,"d":{{"version":1,"type":3,"token":"aW50ZXJhY3Rpb246MTIzNDU2Nzg5MDEyMzQ1Njc4OTpub3QtYS1yZWFsLXRva2Vu{sequence}","id":"{}","application_id":"1","guild_id":"{GUILD}","channel_id":"{CHANNEL}","app_permissions":"2248473465835073","locale":"en-US","guild_locale":"en-US","entitlements":[],"authorizing_integration_owners":{{"0":"{GUILD}"}},"context":0,"attachment_size_limit":10485760,"guild":{{"id":"{GUILD}","locale":"en-US","features":["COMMUNITY"]}},"channel":{{"id":"{CHANNEL}","type":0,"guild_id":"{GUILD}","name":"general","position":0,"permission_overwrites":[],"nsfw":false,"flags":0,"permissions":"2248473465835073"}},"member":{{"user":{{"id":"{user}","username":"clicker","discriminator":"0","global_name":"Clicker","avatar":null,"public_flags":0}},"roles":["1100000000000000001"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"avatar":null,"permissions":"2248473465835073"}},"message":{{"type":0,"tts":false,"timestamp":"2026-11-10T19:42:07.123000+00:00","pinned":false,"mentions":[],"mention_roles":[],"mention_everyone":false,"id":"{MENU}","flags":0,"embeds":[],"edited_timestamp":null,"content":"Pick one","components":[{{"type":1,"id":1,"components":[{{"type":2,"id":2,"style":1,"label":"Yes","custom_id":"yes"}},{{"type":2,"id":3,"style":4,"label":"No","custom_id":"no"}}]}}],"channel_id":"{CHANNEL}","author":{{"username":"stress-bot","id":"1","discriminator":"0","global_name":null,"avatar":null,"bot":true}},"attachments":[]}},"data":{{"custom_id":"yes","component_type":2}}}}}}"#,
        3_000_000_000_000_000_000_u64 + sequence
    )
}

/// A `GUILD_MEMBERS_CHUNK` of `members` members starting at `first`.
fn chunk_dispatch(
    sequence: u64,
    nonce: &str,
    index: u32,
    count: u32,
    first: u64,
    members: u64,
) -> String {
    let members: Vec<String> = (first..first + members)
        .map(|id| {
            format!(
                r#"{{"user":{{"id":"{}","username":"member{id}","discriminator":"0","global_name":null,"avatar":null}},"roles":["1100000000000000001"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"avatar":null}}"#,
                1_600_000_000_000_000_000_u64 + id
            )
        })
        .collect();
    format!(
        r#"{{"t":"GUILD_MEMBERS_CHUNK","s":{sequence},"op":0,"d":{{"guild_id":"{GUILD}","members":[{}],"chunk_index":{index},"chunk_count":{count},"nonce":"{nonce}"}}}}"#,
        members.join(",")
    )
}

/// What the mock gateway sends after `READY`, once the test says `go`.
#[derive(Clone, Copy)]
enum Script {
    /// `events` dispatches made by `make`.
    Flood {
        events: u64,
        make: fn(u64) -> String,
    },
    /// Answers a member request with `members` members in chunks of `per_chunk`.
    Members { members: u64, per_chunk: u64 },
}

async fn serve(listener: TcpListener, script: Script, go: Arc<Notify>) {
    let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
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
    match script {
        Script::Flood { events, make } => {
            go.notified().await;
            for sequence in 2..events + 2 {
                ws.feed(Message::text(make(sequence))).await.unwrap();
                if sequence % 64 == 0 {
                    ws.flush().await.unwrap();
                }
            }
            ws.flush().await.unwrap();
        }
        Script::Members { members, per_chunk } => {
            let request = loop {
                let frame = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
                let Some(text) = frame.as_text() else {
                    continue;
                };
                let command: Value = serde_json::from_str(text).unwrap();
                if command["op"] == 8 {
                    break command;
                }
            };
            let nonce = request["d"]["nonce"].as_str().unwrap().to_owned();
            let count = u32::try_from(members.div_ceil(per_chunk)).unwrap();
            for index in 0..count {
                let first = u64::from(index) * per_chunk;
                let size = per_chunk.min(members - first);
                let chunk = chunk_dispatch(u64::from(index) + 2, &nonce, index, count, first, size);
                ws.feed(Message::text(chunk)).await.unwrap();
            }
            ws.flush().await.unwrap();
        }
    }
    while let Ok(Some(Ok(_))) = timeout(STEP, ws.next()).await {}
}

/// Counts and wakes the test once `target` is reached.
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

fn memory_since(before: Option<(u64, u64)>) -> String {
    match (before, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {} → {} MiB, peak {} MiB",
            before / 1024,
            after / 1024,
            peak / 1024
        ),
        _ => "memory n/a".to_owned(),
    }
}

/// Starts a client on a mock gateway following `script`, with `cache`, and returns it with
/// its handler context once `READY` was handled.
async fn start(
    script: Script,
    mode: HandlerMode,
    cache: Option<CacheConfig>,
) -> (Client, Context, Arc<Notify>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let go = Arc::new(Notify::new());
    let server = tokio::spawn(serve(listener, script, Arc::clone(&go)));
    let mut builder = Client::builder("stress-token")
        .intents(Intents::GUILD_MESSAGES)
        .compression(Compression::None)
        .handler_mode(mode)
        .gateway_url(url)
        .shards(Shards::Count(1))
        // No request reaches it.
        .rest(Rest::builder().token("stress-token").api("http://127.0.0.1:9/api").build());
    if let Some(cache) = cache {
        builder = builder.cache(cache);
    }
    let client = builder.build();
    let (tx, mut rx) = mpsc::unbounded_channel();
    client.on(Events::ClientReady, move |ctx, _| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(ctx);
            Ok(())
        }
    });
    {
        let client = client.clone();
        tokio::spawn(async move { client.login().await });
    }
    let ctx = timeout(STEP, rx.recv()).await.unwrap().unwrap();
    (client, ctx, go, server)
}

/// What the message floods register besides their counting handler.
#[derive(Clone, Copy, Debug)]
enum Collectors {
    None,
    /// Message collectors on other channels (each event misses them in the index).
    Unrelated(u64),
    /// Component collectors on other messages (a kind the messages do not need).
    OtherKind(u64),
    /// One message collector on the flood's channel, read by another task.
    Busy,
}

/// Floods 1M (`events`) `MESSAGE_CREATE`s through a sequential counting handler with
/// `collectors` registered; prints the throughput.
async fn message_flood(name: &str, events: u64, collectors: Collectors) {
    let before = memory_kib();
    let (client, ctx, go, server) = start(
        Script::Flood {
            events,
            make: message_dispatch,
        },
        HandlerMode::Sequential,
        None,
    )
    .await;
    let handled = Counter::new(events);
    {
        let handled = handled.clone();
        client.on(Events::MessageCreate, move |_ctx, msg| {
            let handled = handled.clone();
            async move {
                std::hint::black_box(&msg.content);
                handled.hit();
                Ok(())
            }
        });
    }
    let mut kept = Vec::new();
    let mut kept_components = Vec::new();
    let collected = Counter::new(events);
    match collectors {
        Collectors::None => {}
        Collectors::Unrelated(count) => {
            for channel in 1..=count {
                kept.push(ctx.message_collector(ChannelId::new(channel)).build());
            }
        }
        Collectors::OtherKind(count) => {
            for channel in 1..=count {
                kept_components.push(ctx.component_collector(ChannelId::new(channel)).build());
            }
        }
        Collectors::Busy => {
            let mut busy = ctx.message_collector(ChannelId::new(CHANNEL)).build();
            let collected = collected.clone();
            tokio::spawn(async move {
                while let Some(message) = busy.next().await {
                    std::hint::black_box(&message.content);
                    collected.hit();
                }
            });
        }
    }
    let started = Instant::now();
    go.notify_one();
    timeout(STEP, handled.done.notified())
        .await
        .expect("the handler did not finish in time");
    if matches!(collectors, Collectors::Busy) {
        timeout(STEP, collected.done.notified())
            .await
            .expect("the collector did not finish in time");
    }
    let elapsed = started.elapsed();
    println!(
        "[stress] {name}: {events} messages in {elapsed:.2?} = {:.0}/s ({:.2} µs each); {}",
        events as f64 / elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 1e6 / events as f64,
        memory_since(before),
    );
    drop((kept, kept_components));
    client.shutdown();
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn messages_1m_without_collectors() {
    message_flood("no collector", 1_000_000, Collectors::None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn messages_1m_with_10k_unrelated_collectors() {
    message_flood(
        "10 000 message collectors elsewhere",
        1_000_000,
        Collectors::Unrelated(10_000),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn messages_1m_with_10k_component_collectors() {
    message_flood(
        "10 000 component collectors",
        1_000_000,
        Collectors::OtherKind(10_000),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn messages_1m_into_one_collector() {
    message_flood(
        "one collector of every message",
        1_000_000,
        Collectors::Busy,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn clicks_1m_into_one_component_collector() {
    let events = 1_000_000;
    let before = memory_kib();
    let (client, ctx, go, server) = start(
        Script::Flood {
            events,
            make: click_dispatch,
        },
        HandlerMode::Concurrent,
        None,
    )
    .await;
    let menu: djs_rs::model::message::Message = serde_json::from_str(&format!(
        r#"{{"id":"{MENU}","channel_id":"{CHANNEL}","guild_id":"{GUILD}","author":{{"id":"1","username":"stress-bot","discriminator":"0","global_name":null,"avatar":null}},"content":"Pick one","timestamp":"2026-11-10T19:42:07.123000+00:00","edited_timestamp":null,"tts":false,"mention_everyone":false,"pinned":false,"type":0}}"#
    ))
    .unwrap();
    // Tracks the 1 000 clicking users without reaching the limit.
    let mut clicks = menu.component_collector(&ctx).max_users(1001).build();
    let started = Instant::now();
    go.notify_one();
    let mut collected = 0_u64;
    timeout(STEP, async {
        while collected < events {
            let click = clicks.next().await.expect("the collector ended early");
            std::hint::black_box(&click.custom_id);
            collected += 1;
        }
    })
    .await
    .expect("the collector did not finish in time");
    let elapsed = started.elapsed();
    println!(
        "[stress] 1M clicks into one component collector (max_users): {collected} in \
         {elapsed:.2?} = {:.0}/s; {}",
        collected as f64 / elapsed.as_secs_f64(),
        memory_since(before),
    );
    assert_eq!(clicks.end_reason(), None);
    client.shutdown();
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn collectors_1m_registered_and_dropped() {
    let (client, ctx, go, server) = start(
        Script::Flood {
            events: 0,
            make: message_dispatch,
        },
        HandlerMode::Concurrent,
        None,
    )
    .await;
    go.notify_one();
    let rounds = 1_000_000_u64;
    let started = Instant::now();
    for message in 0..rounds {
        let collector = ctx
            .component_collector(ChannelId::new(message % 1000 + 1))
            .build();
        std::hint::black_box(&collector);
    }
    let single = started.elapsed();
    // With 10 000 collectors alive, registering and dropping still costs one map entry.
    let messages: Vec<_> = (0..10_000).map(menu_at).collect();
    let alive: Vec<_> = messages
        .iter()
        .map(|message| message.reaction_collector(&ctx).build())
        .collect();
    let started = Instant::now();
    for (message, _) in messages.iter().cycle().zip(0..rounds) {
        let collector = message.reaction_collector(&ctx).build();
        std::hint::black_box(&collector);
    }
    let crowded = started.elapsed();
    println!(
        "[stress] 1M collectors built and dropped: {:.0} ns each alone, {:.0} ns each next to \
         10 000 alive",
        single.as_secs_f64() * 1e9 / rounds as f64,
        crowded.as_secs_f64() * 1e9 / rounds as f64,
    );
    drop(alive);
    client.shutdown();
    server.await.unwrap();
}

/// A message of the flood's channel with the ID `MENU + offset`.
fn menu_at(offset: u64) -> djs_rs::model::message::Message {
    let mut message: djs_rs::model::message::Message = serde_json::from_str(&format!(
        r#"{{"id":"{MENU}","channel_id":"{CHANNEL}","guild_id":"{GUILD}","author":{{"id":"1","username":"stress-bot","discriminator":"0","global_name":null,"avatar":null}},"content":"Pick one","timestamp":"2026-11-10T19:42:07.123000+00:00","edited_timestamp":null,"tts":false,"mention_everyone":false,"pinned":false,"type":0}}"#
    ))
    .unwrap();
    message.id = MessageId::new(MENU + offset);
    message
}

async fn fetch_all(name: &str, members: u64, cache: Option<CacheConfig>) {
    let before = memory_kib();
    let (client, ctx, _go, server) = start(
        Script::Members {
            members,
            per_chunk: 1000,
        },
        HandlerMode::Concurrent,
        cache,
    )
    .await;
    let started = Instant::now();
    let fetched = timeout(STEP, ctx.members(GuildId::new(GUILD)).fetch_all())
        .await
        .expect("the request did not finish in time")
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(fetched.len() as u64, members);
    println!(
        "[stress] fetch_all of {members} members in 1 000-member chunks, {name}: {elapsed:.2?} \
         = {:.0} members/s; {}",
        members as f64 / elapsed.as_secs_f64(),
        memory_since(before),
    );
    drop(fetched);
    client.shutdown();
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn fetch_all_100k_members_cached() {
    let cache = CacheConfig::builder()
        .members(Limit::Unlimited)
        .users(Limit::Unlimited)
        .build();
    fetch_all("into the cache", 100_000, Some(cache)).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn fetch_all_100k_members_without_a_cache() {
    fetch_all("without a cache", 100_000, None).await;
}
