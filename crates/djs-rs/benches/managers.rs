//! Benchmarks of the managers' cache path (D-041). Run with `cargo bench -p djs-rs`.
//!
//! The global `AllocProfiler` reports allocations per operation next to timings: a `fetch`
//! the cache answers must allocate nothing. The futures are polled once with a no-op waker,
//! so no runtime is measured; a client connected to a local mock gateway provides the
//! `Context`.

use std::{
    future::{Future, IntoFuture},
    pin::pin,
    sync::LazyLock,
    task::{Context as TaskContext, Poll, Waker},
};

use divan::{Bencher, black_box};
use djs_rs::{
    Shards,
    cache::{CacheConfig, Limit},
    gateway::Compression,
    model::{
        guild::{Member, Role},
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
        message::Message,
    },
    prelude::*,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{net::TcpListener, runtime::Runtime, sync::mpsc};
use tokio_websockets::{Message as WsMessage, ServerBuilder};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

const GUILD: GuildId = GuildId::new(1_400_000_000_000_000_002);
const CHANNEL: ChannelId = ChannelId::new(20);
const MEMBERS: u64 = 1_000;

/// A logged-in client's context, with 1000 members, 200 messages and a role cached.
static CTX: LazyLock<(Runtime, Context)> = LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let ctx = runtime.block_on(connect());
    let cache = ctx.cache().unwrap();
    for id in 1..=MEMBERS {
        let member: Member = serde_json::from_value(json!({
            "user": {"id": id.to_string(), "username": "nelly", "discriminator": "0",
                     "global_name": null, "avatar": null},
            "nick": null, "roles": [], "joined_at": "2026-01-01T00:00:00.000000+00:00",
            "deaf": false, "mute": false, "flags": 0, "pending": false
        }))
        .unwrap();
        cache.store_member(GUILD, &member);
    }
    for id in 1..=200_u64 {
        let message: Message = serde_json::from_value(json!({
            "id": id.to_string(), "channel_id": CHANNEL.to_string(),
            "author": {"id": "1", "username": "bot", "discriminator": "0",
                       "global_name": null, "avatar": null},
            "content": "hello", "timestamp": "2026-11-12T10:00:00.000000+00:00",
            "edited_timestamp": null, "tts": false, "mention_everyone": false,
            "pinned": false, "type": 0
        }))
        .unwrap();
        cache.store_message(&message);
    }
    let role: Role = serde_json::from_value(json!({
        "id": "5", "name": "mods", "color": 0, "hoist": false, "position": 1,
        "permissions": "0", "managed": false, "mentionable": false, "flags": 0,
        "colors": {"primary_color": 0, "secondary_color": null, "tertiary_color": null}
    }))
    .unwrap();
    cache.store_role(GUILD, &role);
    (runtime, ctx)
});

/// Connects a client with a cache to a mock gateway that only sends `READY`.
async fn connect() -> Context {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
        let text = |payload: Value| WsMessage::text(payload.to_string());
        ws.send(text(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
            .await
            .unwrap();
        while let Some(Ok(frame)) = ws.next().await {
            if frame
                .as_text()
                .is_some_and(|text| serde_json::from_str::<Value>(text).unwrap()["op"] == 2)
            {
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
        while ws.next().await.is_some() {}
    });
    let client = Client::builder("bench-token")
        .intents(Intents::GUILDS)
        .gateway_url(url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .cache(
            CacheConfig::builder()
                .members(Limit::Unlimited)
                .users(Limit::Unlimited)
                .messages(Limit::Max(200))
                .build(),
        )
        .build();
    let (tx, mut rx) = mpsc::unbounded_channel();
    client.on(Events::ClientReady, move |ctx, _| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(ctx);
            Ok(())
        }
    });
    tokio::spawn(async move { client.login().await });
    rx.recv().await.unwrap()
}

/// Polls a fetch once: a cache hit is ready at once.
fn now<F: IntoFuture>(fetch: F) -> F::Output {
    let mut future = pin!(fetch.into_future());
    match future
        .as_mut()
        .poll(&mut TaskContext::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("a cache hit is ready at once"),
    }
}

/// The floor: reading the cache view directly.
#[divan::bench]
fn member_view_get(bencher: Bencher<'_, '_>) {
    let ctx = &CTX.1;
    let mut next = 0;
    bencher.bench_local(|| {
        next = next % MEMBERS + 1;
        ctx.cache()
            .unwrap()
            .members(GUILD)
            .get(black_box(UserId::new(next)))
    });
}

/// `ctx.members(guild).fetch(user).await` answered by the cache.
#[divan::bench]
fn member_fetch_hit(bencher: Bencher<'_, '_>) {
    let ctx = &CTX.1;
    let mut next = 0;
    bencher.bench_local(|| {
        next = next % MEMBERS + 1;
        now(ctx.members(GUILD).fetch(black_box(UserId::new(next)))).unwrap()
    });
}

/// `ctx.messages(channel).fetch(id).await` answered by the cache.
#[divan::bench]
fn message_fetch_hit(bencher: Bencher<'_, '_>) {
    let ctx = &CTX.1;
    let mut next = 0;
    bencher.bench_local(|| {
        next = next % 200 + 1;
        now(ctx.messages(CHANNEL).fetch(black_box(MessageId::new(next)))).unwrap()
    });
}

/// `ctx.roles(guild).fetch(id).await` answered by the cache.
#[divan::bench]
fn role_fetch_hit(bencher: Bencher<'_, '_>) {
    let ctx = &CTX.1;
    bencher.bench_local(|| now(ctx.roles(GUILD).fetch(black_box(RoleId::new(5)))).unwrap());
}

/// A forced fetch turned into its future but not sent: the cost of boxing the request.
#[divan::bench]
fn member_fetch_forced_future(bencher: Bencher<'_, '_>) {
    let ctx = &CTX.1;
    bencher.bench_local(|| {
        black_box(
            ctx.members(GUILD)
                .fetch(black_box(UserId::new(1)))
                .force()
                .into_future(),
        )
    });
}
