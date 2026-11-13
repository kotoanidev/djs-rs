//! Extreme-load tests of interactions (D-041): floods of `INTERACTION_CREATE` through an
//! event handler and through the router (D-251), and floods of replies through a mock REST
//! API.
//!
//! Ignored by default; run them in release mode, one at a time with pauses (they load every
//! core), and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-rs --release --test stress_interactions -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_rs::{HandlerMode, Shards, gateway::Compression, prelude::*, rest::Rest};
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Notify, Semaphore},
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder};

const STEP: Duration = Duration::from_secs(120);

const HELLO: &str = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":600000}}"#;
const READY: &str = r#"{"t":"READY","s":1,"op":0,"d":{"v":10,"user":{"id":"1","username":"stress-bot","discriminator":"0","global_name":null,"avatar":null,"bot":true},"guilds":[],"session_id":"s","resume_gateway_url":"ws://127.0.0.1:9","shard":[0,1],"application":{"id":"1","flags":0}}}"#;

/// A realistic `/mod config set` interaction (≈ 1.9 KB): the guild and channel objects, the
/// invoking member, five options and their resolved user, member and role.
fn interaction_dispatch(sequence: u64) -> String {
    format!(
        r#"{{"t":"INTERACTION_CREATE","s":{sequence},"op":0,"d":{{"id":"{}","application_id":"1","type":2,"token":"not-a-real-token-only-for-benchmarking-0123456789-aWQ6MTMwMDAwMDAwMDAwMDAwMDAwMQ","version":1,"app_permissions":"2248473465835073","locale":"pl","guild_locale":"en-US","guild_id":"1400000000000000002","guild":{{"id":"1400000000000000002","locale":"en-US","features":["COMMUNITY","NEWS"]}},"channel_id":"1200000000000000020","channel":{{"id":"1200000000000000020","type":0,"name":"general","guild_id":"1400000000000000002","nsfw":false,"flags":0,"permissions":"2248473465835073","position":3,"topic":"Talk here","last_message_id":"1500000000000000099","rate_limit_per_user":0,"parent_id":"1200000000000000001"}},"member":{{"user":{{"id":"1300000000000000003","username":"nelly","discriminator":"0","global_name":"Nelly","avatar":"a_1269e74af4df7417b13759eae50c83dc","public_flags":0,"clan":null,"avatar_decoration_data":null}},"roles":["1100000000000000001","1100000000000000002"],"joined_at":"2024-05-01T12:00:00.000000+00:00","premium_since":null,"deaf":false,"mute":false,"flags":0,"pending":false,"nick":null,"avatar":null,"banner":null,"communication_disabled_until":null,"permissions":"2248473465835073"}},"entitlements":[],"authorizing_integration_owners":{{"0":"1400000000000000002"}},"context":0,"attachment_size_limit":10485760,"data":{{"id":"1600000000000000001","name":"mod","type":1,"guild_id":"1400000000000000002","options":[{{"name":"config","type":2,"options":[{{"name":"set","type":1,"options":[{{"name":"reason","type":3,"value":"spam in general, number {sequence}"}},{{"name":"days","type":4,"value":7}},{{"name":"silent","type":5,"value":true}},{{"name":"target","type":6,"value":"1300000000000000011"}},{{"name":"role","type":8,"value":"1100000000000000001"}}]}}]}}],"resolved":{{"users":{{"1300000000000000011":{{"id":"1300000000000000011","username":"spammer","discriminator":"0","global_name":null,"avatar":null,"public_flags":0}}}},"members":{{"1300000000000000011":{{"roles":[],"joined_at":"2026-11-01T12:00:00.000000+00:00","premium_since":null,"flags":0,"pending":false,"nick":null,"avatar":null,"banner":null,"communication_disabled_until":null,"permissions":"104324673"}}}},"roles":{{"1100000000000000001":{{"id":"1100000000000000001","name":"Muted","color":0,"colors":{{"primary_color":0,"secondary_color":null,"tertiary_color":null}},"hoist":false,"icon":null,"unicode_emoji":null,"position":1,"permissions":"0","managed":false,"mentionable":false,"flags":0}}}}}}}}}}}}"#,
        1_700_000_000_000_000_000_u64 + sequence
    )
}

/// Serves `sessions` sessions that each send `events` interactions after `READY`.
async fn serve(listener: TcpListener, sessions: u32, events: u64) {
    let mut tasks = Vec::new();
    for _ in 0..sessions {
        let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
        tasks.push(tokio::spawn(serve_session(stream, events)));
    }
    for task in tasks {
        task.await.unwrap();
    }
}

async fn serve_session(stream: TcpStream, events: u64) {
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
    for sequence in 2..events + 2 {
        ws.feed(Message::text(interaction_dispatch(sequence)))
            .await
            .unwrap();
        if sequence % 64 == 0 {
            ws.flush().await.unwrap();
        }
    }
    ws.flush().await.unwrap();
    while let Ok(Some(Ok(_))) = timeout(STEP, ws.next()).await {}
}

/// A mock REST API: Get Gateway Bot recommends `shards` shards (all identifying at once), and
/// interaction callbacks are answered `204` and counted in `callbacks`.
async fn mock_rest(gateway_url: String, shards: u32, callbacks: Arc<AtomicU64>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    let gateway = Bytes::from(format!(
        r#"{{"url":"{gateway_url}","shards":{shards},"session_start_limit":{{"total":1000,"remaining":1000,"reset_after":0,"max_concurrency":{shards}}}}}"#
    ));
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            stream.set_nodelay(true).unwrap();
            let gateway = gateway.clone();
            let callbacks = Arc::clone(&callbacks);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let gateway = gateway.clone();
                    let callbacks = Arc::clone(&callbacks);
                    async move {
                        let response = if request.uri().path().ends_with("/callback") {
                            callbacks.fetch_add(1, Ordering::Relaxed);
                            Response::builder()
                                .status(204)
                                .body(Full::new(Bytes::new()))
                                .unwrap()
                        } else {
                            Response::new(Full::new(gateway))
                        };
                        Ok::<_, Infallible>(response)
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

/// Counts handler calls and wakes the test once `target` is reached.
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

/// What handles the flood.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle {
    /// An `InteractionCreate` handler matching the command's name, as with discord.js.
    Event,
    /// A route among 20 (D-251).
    Route,
    /// Both: the event handler and the route.
    Both,
    /// A route that replies, with at most this many replies in flight (0: no limit, the
    /// handler mode decides).
    Reply(usize),
}

/// Reads what a handler would: the subcommand path and three options.
fn read(data: &djs_rs::model::interaction::CommandData) -> bool {
    std::hint::black_box((
        data.subcommand(),
        data.get_string("reason"),
        data.get_integer("days"),
        data.get_user("target").map(|user| user.id),
    ));
    true
}

/// Floods a client running `shards` shards with `events` interactions in total. Returns the
/// time from `login` to the last handler call and the callbacks the mock API received.
async fn run(handle: Handle, shards: u32, events: u64, mode: HandlerMode) -> (Duration, u64) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(serve(listener, shards, events / u64::from(shards)));
    let callbacks = Arc::new(AtomicU64::new(0));
    let api = mock_rest(url.clone(), shards, Arc::clone(&callbacks)).await;
    let rest = Rest::builder()
        .token("stress-token")
        .api(api)
        .global_requests_per_second(0)
        .build();
    let builder = Client::builder("stress-token")
        .intents(Intents::GUILDS)
        .compression(Compression::None)
        .handler_mode(mode)
        .rest(rest);
    let client = if shards == 1 {
        builder.gateway_url(url).shards(Shards::Count(1)).build()
    } else {
        builder.build()
    };

    let handlers = if handle == Handle::Both { 2 } else { 1 };
    let counter = Counter::new(events * handlers);
    if matches!(handle, Handle::Event | Handle::Both) {
        let counter = counter.clone();
        client.on(Events::InteractionCreate, move |_, interaction| {
            let counter = counter.clone();
            async move {
                if let Some(command) = interaction.command()
                    && command.name == "mod"
                    && read(command)
                {
                    counter.hit();
                }
                Ok(())
            }
        });
    }
    if handle != Handle::Event {
        for route in 0..19 {
            client.command(&format!("command{route}"), |_, _| async { Ok(()) });
        }
        let counter = counter.clone();
        match handle {
            Handle::Reply(limit) => {
                let limit = (limit > 0).then(|| Arc::new(Semaphore::new(limit)));
                client.command("mod config set", move |ctx, cmd| {
                    let counter = counter.clone();
                    let limit = limit.clone();
                    async move {
                        read(&cmd);
                        let _permit = match &limit {
                            Some(limit) => Some(limit.acquire().await?),
                            None => None,
                        };
                        cmd.reply(&ctx, "Done").await?;
                        counter.hit();
                        Ok(())
                    }
                });
            }
            // The same work as the event handler, for an A/B of the routing.
            _ => {
                client.command("mod config set", move |_, cmd| {
                    let counter = counter.clone();
                    async move {
                        if read(&cmd) {
                            counter.hit();
                        }
                        Ok(())
                    }
                });
            }
        }
    }

    let started = Instant::now();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    timeout(STEP, counter.done.notified())
        .await
        .expect("handlers did not finish in time");
    let elapsed = started.elapsed();
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    server.await.unwrap();
    (elapsed, callbacks.load(Ordering::Relaxed))
}

/// `(VmRSS, VmHWM)` of this process in KiB (Linux only).
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

async fn measure(name: &str, handle: Handle, shards: u32, events: u64, mode: HandlerMode) {
    let before = memory_kib();
    // Resets the peak to the current RSS.
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    let (elapsed, callbacks) = run(handle, shards, events, mode).await;
    let memory = match (before, memory_kib()) {
        (Some((before, _)), Some((after, peak))) => format!(
            "rss {} → {} MiB, peak {} MiB",
            before / 1024,
            after / 1024,
            peak / 1024
        ),
        _ => "memory n/a".to_owned(),
    };
    println!(
        "[stress] {name}: {events} interactions in {elapsed:.2?} = {:.0}/s ({:.2} µs each), \
         {callbacks} callbacks; {memory}",
        events as f64 / elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 1e6 / events as f64,
    );
    if matches!(handle, Handle::Reply(_)) {
        assert_eq!(callbacks, events, "every interaction was answered once");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn interactions_1m_event_handler() {
    measure(
        "event handler",
        Handle::Event,
        1,
        1_000_000,
        HandlerMode::Concurrent,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn interactions_1m_router() {
    measure(
        "router (20 routes)",
        Handle::Route,
        1,
        1_000_000,
        HandlerMode::Concurrent,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn interactions_1m_router_and_event_handler() {
    measure(
        "router + event handler",
        Handle::Both,
        1,
        1_000_000,
        HandlerMode::Concurrent,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn interactions_1m_router_sequential() {
    measure(
        "router, sequential",
        Handle::Route,
        1,
        1_000_000,
        HandlerMode::Sequential,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn interactions_1m_router_over_16_shards() {
    measure(
        "router, 16 shards",
        Handle::Route,
        16,
        1_000_000,
        HandlerMode::Concurrent,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn replies_100k_sequential() {
    measure(
        "replies, sequential",
        Handle::Reply(0),
        1,
        100_000,
        HandlerMode::Sequential,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn replies_200k_64_in_flight() {
    measure(
        "replies, 64 in flight",
        Handle::Reply(64),
        1,
        200_000,
        HandlerMode::Concurrent,
    )
    .await;
}

/// The 16 shards decode faster than the replies are answered, so the interactions wait in
/// memory: kept at 50 000 (a peak below 0.5 GiB, D-258) since this flood at 200 000 peaked at
/// 2.2 GiB.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "stress test; run with --release --ignored"]
async fn replies_50k_64_in_flight_over_16_shards() {
    measure(
        "replies, 64 in flight, 16 shards",
        Handle::Reply(64),
        16,
        50_000,
        HandlerMode::Concurrent,
    )
    .await;
}
