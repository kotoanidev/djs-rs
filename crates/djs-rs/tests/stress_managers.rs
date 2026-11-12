//! Extreme-load tests of the managers (D-041): cache-hit fetches on one and on every core,
//! fetches and sends through a mock REST API (against the bare `Rest` calls, to isolate the
//! conversion and caching), and the memory REST results take in the cache.
//!
//! Ignored by default; run them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rs --release --test stress_managers -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    convert::Infallible,
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_rs::{
    Shards,
    cache::{CacheConfig, Limit},
    gateway::Compression,
    model::{
        guild::Member,
        id::{ChannelId, GuildId, UserId},
    },
    prelude::*,
    rest::Rest,
};
use futures_util::{SinkExt, StreamExt, stream};
use http_body_util::Full;
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle, time::timeout};
use tokio_websockets::{Message, ServerBuilder};

const STEP: Duration = Duration::from_secs(60);
const GUILD: GuildId = GuildId::new(1_400_000_000_000_000_002);
const CHANNEL: ChannelId = ChannelId::new(20);
/// Requests in flight in the concurrent REST floods.
const IN_FLIGHT: usize = 64;

fn member(id: u64) -> Value {
    json!({"user": {"id": id.to_string(), "username": format!("user-{id}"),
                    "discriminator": "0", "global_name": null, "avatar": null},
           "nick": null, "roles": ["11"], "joined_at": "2026-01-01T00:00:00.000000+00:00",
           "deaf": false, "mute": false, "flags": 0, "pending": false})
}

fn message(id: u64, content: &Value) -> Value {
    json!({"id": id.to_string(), "channel_id": CHANNEL.to_string(),
           "guild_id": GUILD.to_string(),
           "author": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
                      "avatar": null, "bot": true},
           "content": content, "timestamp": "2026-11-12T10:00:00.000000+00:00",
           "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
           "type": 0})
}

/// A mock REST API answering Get Guild Member and Create Message, as fast as it can.
async fn mock_rest() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let next_message = Arc::new(std::sync::atomic::AtomicU64::new(1_000));
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let next_message = Arc::clone(&next_message);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let next_message = Arc::clone(&next_message);
                    async move {
                        let path = request.uri().path().to_owned();
                        let segments: Vec<&str> = path.split('/').collect();
                        let body = match segments.as_slice() {
                            [.., "members", id] => member(id.parse().unwrap()),
                            [.., "messages"] => {
                                use http_body_util::BodyExt;
                                let body = request.into_body().collect().await.unwrap();
                                let body: Value = serde_json::from_slice(&body.to_bytes()).unwrap();
                                let id =
                                    next_message.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                message(id, &body["content"])
                            }
                            _ => json!({"message": "404: Not Found", "code": 0}),
                        };
                        Ok::<_, Infallible>(
                            Response::builder()
                                .header("content-type", "application/json")
                                .body(Full::new(Bytes::from(body.to_string())))
                                .unwrap(),
                        )
                    }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    base
}

/// A gateway that identifies the client, sends an empty `READY` and waits for the close.
async fn mock_gateway() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
        let text = |payload: Value| Message::text(payload.to_string());
        ws.send(text(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
            .await
            .unwrap();
        while let Some(Ok(frame)) = ws.next().await {
            let identify = frame
                .as_text()
                .is_some_and(|text| serde_json::from_str::<Value>(text).unwrap()["op"] == 2);
            if identify {
                break;
            }
        }
        ws.send(text(json!({"t": "READY", "s": 1, "op": 0, "d": {
            "v": 10,
            "user": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
                     "avatar": null, "bot": true},
            "guilds": [], "session_id": "s", "resume_gateway_url": "ws://127.0.0.1:9",
            "application": {"id": "1"}
        }})))
        .await
        .unwrap();
        while let Some(Ok(frame)) = ws.next().await {
            if frame.is_close() {
                return;
            }
        }
    });
    (url, task)
}

struct Session {
    client: Client,
    ctx: Context,
    running: JoinHandle<Result<(), djs_rs::Error>>,
    gateway: JoinHandle<()>,
}

impl Session {
    async fn start(cache: Option<CacheConfig>) -> Self {
        let api = mock_rest().await;
        let (url, gateway) = mock_gateway().await;
        let rest = Rest::builder()
            .token("stress-token")
            .api(api)
            .global_requests_per_second(0)
            .build();
        let mut builder = Client::builder("stress-token")
            .intents(Intents::GUILDS)
            .gateway_url(url)
            .shards(Shards::Count(1))
            .compression(Compression::None)
            .rest(rest);
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
        let running = {
            let client = client.clone();
            tokio::spawn(async move { client.login().await })
        };
        let ctx = timeout(STEP, rx.recv()).await.unwrap().unwrap();
        Self {
            client,
            ctx,
            running,
            gateway,
        }
    }

    async fn finish(self) {
        self.client.shutdown();
        timeout(STEP, self.running).await.unwrap().unwrap().unwrap();
        self.gateway.await.unwrap();
    }
}

fn members_cache() -> CacheConfig {
    CacheConfig::builder()
        .members(Limit::Unlimited)
        .users(Limit::Unlimited)
        .build()
}

/// Fills the cache with `count` members (IDs 1..=count) through the public write API.
fn preload(ctx: &Context, count: u64) {
    let cache = ctx.cache().unwrap();
    for id in 1..=count {
        let member: Member = serde_json::from_value(member(id)).unwrap();
        cache.store_member(GUILD, &member);
    }
}

fn report(name: &str, operations: u64, elapsed: Duration) {
    println!(
        "{name}: {operations} in {elapsed:.2?} = {:.0} ns/op, {:.2} M ops/s",
        elapsed.as_nanos() as f64 / operations as f64,
        operations as f64 / elapsed.as_secs_f64() / 1e6,
    );
}

/// `(VmRSS, VmHWM)` of this process in KiB (Linux only).
fn memory() -> Option<(u64, u64)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| -> Option<u64> {
        status
            .lines()
            .find(|line| line.starts_with(name))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

const MEMBERS: u64 = 10_000;
const HITS: u64 = 1_000_000;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "extreme-load test; run in release mode with --ignored --nocapture"]
async fn fetch_1m_cache_hits() {
    let session = Session::start(Some(members_cache())).await;
    let ctx = &session.ctx;
    preload(ctx, MEMBERS);

    for round in 0..3 {
        // The cache view directly: the floor.
        let start = Instant::now();
        for index in 0..HITS {
            let id = UserId::new(index % MEMBERS + 1);
            black_box(ctx.cache().unwrap().members(GUILD).get(id).unwrap());
        }
        report(&format!("[{round}] cache view get"), HITS, start.elapsed());

        // Through the manager: the `Fetch` builder, `IntoFuture` and the cached state.
        let start = Instant::now();
        for index in 0..HITS {
            let id = UserId::new(index % MEMBERS + 1);
            black_box(ctx.members(GUILD).fetch(id).await.unwrap());
        }
        report(
            &format!("[{round}] manager fetch (hit)"),
            HITS,
            start.elapsed(),
        );
    }
    session.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "extreme-load test; run in release mode with --ignored --nocapture"]
async fn fetch_cache_hits_on_every_core() {
    let session = Session::start(Some(members_cache())).await;
    preload(&session.ctx, MEMBERS);
    let cores = std::thread::available_parallelism().map_or(4, usize::from);

    for tasks in [1, cores / 2, cores, cores * 4] {
        let tasks = tasks.max(1);
        let start = Instant::now();
        let mut running = Vec::new();
        for task in 0..tasks as u64 {
            let ctx = session.ctx.clone();
            running.push(tokio::spawn(async move {
                for index in 0..HITS {
                    let id = UserId::new((index + task * 997) % MEMBERS + 1);
                    black_box(ctx.members(GUILD).fetch(id).await.unwrap());
                }
            }));
        }
        for task in running {
            task.await.unwrap();
        }
        report(
            &format!("{tasks} tasks × 1M manager fetch (hit)"),
            HITS * tasks as u64,
            start.elapsed(),
        );
    }
    session.finish().await;
}

const REST_REQUESTS: u64 = 100_000;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "extreme-load test; run in release mode with --ignored --nocapture"]
async fn fetch_100k_through_rest() {
    let session = Session::start(Some(members_cache())).await;
    let ctx = &session.ctx;

    for round in 0..2 {
        // The bare REST call: request, response, decoding.
        let start = Instant::now();
        stream::iter(0..REST_REQUESTS)
            .map(|index| async move {
                let user = UserId::new(index % 50_000 + 1);
                black_box(ctx.rest().get_guild_member(GUILD, user).await.unwrap());
            })
            .buffer_unordered(IN_FLIGHT)
            .collect::<()>()
            .await;
        report(
            &format!("[{round}] Rest::get_guild_member"),
            REST_REQUESTS,
            start.elapsed(),
        );

        // The manager: the same, then converted and cached.
        let start = Instant::now();
        stream::iter(0..REST_REQUESTS)
            .map(|index| async move {
                let user = UserId::new(index % 50_000 + 1);
                black_box(ctx.members(GUILD).fetch(user).force().await.unwrap());
            })
            .buffer_unordered(IN_FLIGHT)
            .collect::<()>()
            .await;
        report(
            &format!("[{round}] manager fetch (forced, cached)"),
            REST_REQUESTS,
            start.elapsed(),
        );
    }
    assert_eq!(ctx.members(GUILD).cache().unwrap().len(), 50_000);
    session.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "extreme-load test; run in release mode with --ignored --nocapture"]
async fn send_100k_messages_through_rest() {
    for cache in [None, Some(CacheConfig::discord_js())] {
        let cached = cache.is_some();
        let session = Session::start(cache).await;
        let ctx = &session.ctx;
        let start = Instant::now();
        stream::iter(0..REST_REQUESTS)
            .map(|_| async move {
                let body = djs_rs::model::rest::CreateMessage::from("pong");
                black_box(ctx.rest().create_message(CHANNEL, &body).await.unwrap());
            })
            .buffer_unordered(IN_FLIGHT)
            .collect::<()>()
            .await;
        report(
            &format!("Rest::create_message (cache: {cached})"),
            REST_REQUESTS,
            start.elapsed(),
        );
        let start = Instant::now();
        stream::iter(0..REST_REQUESTS)
            .map(|_| async move {
                black_box(ctx.messages(CHANNEL).send("pong").await.unwrap());
            })
            .buffer_unordered(IN_FLIGHT)
            .collect::<()>()
            .await;
        report(
            &format!("manager send (cache: {cached})"),
            REST_REQUESTS,
            start.elapsed(),
        );
        if cached {
            // discord.js' default keeps 200 messages per channel.
            assert_eq!(ctx.messages(CHANNEL).cache().unwrap().len(), 200);
        }
        session.finish().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "extreme-load test; run in release mode with --ignored --nocapture"]
async fn rest_results_fill_the_cache_200k_members() {
    const COUNT: u64 = 200_000;
    let session = Session::start(Some(members_cache())).await;
    let ctx = &session.ctx;
    // Warm the connections and allocator up, then measure.
    stream::iter(0..1_000)
        .map(|index| async move {
            ctx.rest()
                .get_guild_member(GUILD, UserId::new(index + 1_000_000))
                .await
                .unwrap();
        })
        .buffer_unordered(IN_FLIGHT)
        .collect::<()>()
        .await;
    let before = memory();
    let start = Instant::now();
    stream::iter(1..=COUNT)
        .map(|id| async move {
            black_box(ctx.members(GUILD).fetch(UserId::new(id)).await.unwrap());
        })
        .buffer_unordered(IN_FLIGHT)
        .collect::<()>()
        .await;
    report("manager fetch (miss, cached)", COUNT, start.elapsed());
    let after = memory();
    assert_eq!(
        ctx.members(GUILD).cache().unwrap().len(),
        usize::try_from(COUNT).unwrap()
    );
    assert_eq!(
        ctx.cache().unwrap().users().len(),
        usize::try_from(COUNT).unwrap()
    );
    if let (Some((rss_before, _)), Some((rss_after, peak))) = (before, after) {
        println!(
            "RSS {rss_before} KiB -> {rss_after} KiB (peak {peak} KiB): {:.0} B per member \
             (member + user)",
            (rss_after.saturating_sub(rss_before) * 1024) as f64 / COUNT as f64
        );
    }
    // Every further fetch is a hit.
    let start = Instant::now();
    for id in 1..=COUNT {
        black_box(ctx.members(GUILD).fetch(UserId::new(id)).await.unwrap());
    }
    report(
        "manager fetch (hit) over 200k members",
        COUNT,
        start.elapsed(),
    );
    session.finish().await;
}
