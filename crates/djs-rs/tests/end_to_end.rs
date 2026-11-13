//! End-to-end test of the vertical slice: `Client::login` fetches the gateway URL from a mock
//! REST API, connects to a mock gateway, receives `!ping` and replies through REST.

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::{
    convert::Infallible,
    hash::Hash,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bytes::Bytes;
use djs_rs::{
    Error, HandlerMode, Shards,
    cache::{Backend, CacheConfig, DashMapStore, Limit, Resource, Sweep},
    event::Dispatch,
    gateway::Compression,
    model::{
        id::{ChannelId, GuildId, UserId},
        permissions::Permissions,
    },
    prelude::*,
    rest::Rest,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder};

const STEP: Duration = Duration::from_secs(5);

/// Requests received by the mock REST API: (method, path, JSON body).
type Seen = Arc<Mutex<Vec<(String, String, Value)>>>;

/// A Get Gateway Bot response.
fn gateway_bot(url: &str, shards: u32, remaining: u32, max_concurrency: u32) -> Value {
    json!({"url": url, "shards": shards, "session_start_limit": {
        "total": 1000, "remaining": remaining, "reset_after": 3_600_000,
        "max_concurrency": max_concurrency
    }})
}

/// Starts a mock REST API that answers `GET /gateway/bot` with `gateway_bot` and echoes
/// created messages.
async fn mock_rest(gateway_bot: Value) -> (String, Seen, mpsc::UnboundedReceiver<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    let seen: Seen = Arc::default();
    let (created_tx, created_rx) = mpsc::unbounded_channel();

    let state = Arc::clone(&seen);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let (state, gateway_bot, created_tx) =
                (Arc::clone(&state), gateway_bot.clone(), created_tx.clone());
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let (state, gateway_bot, created_tx) =
                        (Arc::clone(&state), gateway_bot.clone(), created_tx.clone());
                    async move {
                        let method = request.method().to_string();
                        let path = request.uri().path().to_owned();
                        let body = request.into_body().collect().await.unwrap().to_bytes();
                        let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                        state
                            .lock()
                            .unwrap()
                            .push((method.clone(), path.clone(), body.clone()));
                        let response = if path.ends_with("/gateway/bot") {
                            gateway_bot
                        } else {
                            created_tx.send(body.clone()).unwrap();
                            json!({
                                "id": "99", "channel_id": "20",
                                "author": {"id": "1", "username": "test-bot", "discriminator": "0",
                                           "global_name": null, "avatar": null, "bot": true},
                                "content": body["content"], "timestamp": "2026-11-10T19:42:08.000000+00:00",
                                "edited_timestamp": null, "tts": false, "mention_everyone": false,
                                "pinned": false, "type": 19
                            })
                        };
                        Ok::<_, Infallible>(Response::new(Full::new(Bytes::from(
                            response.to_string(),
                        ))))
                    }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    (base, seen, created_rx)
}

fn message_create(sequence: u64, id: &str, content: &str) -> Value {
    json!({"t": "MESSAGE_CREATE", "s": sequence, "op": 0, "d": {
        "id": id, "channel_id": "20", "guild_id": "2",
        "author": {"id": "3", "username": "nelly", "discriminator": "0", "global_name": null,
                   "avatar": null},
        "content": content, "timestamp": "2026-11-10T19:42:07.123000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
        "type": 0
    }})
}

/// Accepts one connection and runs [`mock_session`] on it.
async fn mock_gateway(listener: TcpListener, dispatches: Vec<Value>) -> (Value, Option<u16>) {
    let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
    mock_session(stream, dispatches).await
}

/// Runs a mock gateway session: Hello → (identify) → READY → the given dispatches, then waits for
/// the client to close the connection. Returns the identify payload and the close code.
async fn mock_session(stream: TcpStream, dispatches: Vec<Value>) -> (Value, Option<u16>) {
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    let send = |payload: Value| Message::text(payload.to_string());

    ws.send(send(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
        .await
        .unwrap();
    let identify = loop {
        let message = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
        if let Some(text) = message.as_text() {
            let payload: Value = serde_json::from_str(text).unwrap();
            if payload["op"] == 2 {
                break payload;
            }
        }
    };
    ws.send(send(json!({"t": "READY", "s": 1, "op": 0, "d": {
        "v": 10,
        "user": {"id": "1", "username": "test-bot", "discriminator": "0", "global_name": null,
                 "avatar": null, "bot": true},
        "guilds": [], "session_id": "s", "resume_gateway_url": "ws://127.0.0.1:9",
        "application": {"id": "1"}
    }})))
    .await
    .unwrap();
    for dispatch in dispatches {
        ws.send(send(dispatch)).await.unwrap();
    }

    let code = loop {
        match timeout(STEP, ws.next()).await.expect("client never closed") {
            Some(Ok(message)) if message.is_close() => {
                break message.as_close().map(|(code, _)| u16::from(code));
            }
            Some(Ok(_)) => {}
            Some(Err(_)) | None => break None,
        }
    };
    (identify, code)
}

#[tokio::test]
async fn ping_pong() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let (rest_base, seen, mut created) = mock_rest(gateway_bot(&gateway_url, 1, 999, 1)).await;
    let gateway = tokio::spawn(mock_gateway(
        gateway,
        vec![
            message_create(2, "10", "hello"),
            message_create(3, "11", "!ping"),
        ],
    ));

    let client = Client::builder("test-token")
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
        .rest(Rest::builder().token("test-token").api(rest_base).build())
        .compression(Compression::None)
        .build();

    let (ready_tx, mut ready_rx) = mpsc::unbounded_channel();
    client.on(Events::ClientReady, move |_ctx, ready| {
        let ready_tx = ready_tx.clone();
        async move {
            ready_tx.send(ready.user.username.to_string())?;
            Ok(())
        }
    });
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!ping" {
            msg.reply(&ctx, "Pong!").await?;
        }
        Ok(())
    });
    let mut events = client.events();

    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };

    assert_eq!(
        timeout(STEP, ready_rx.recv()).await.unwrap().unwrap(),
        "test-bot"
    );
    let reply = timeout(STEP, created.recv()).await.unwrap().unwrap();
    assert_eq!(
        reply,
        json!({"content": "Pong!", "message_reference": {"type": 0, "message_id": "11"}})
    );

    // Logging in twice is rejected while running.
    assert!(matches!(client.login().await, Err(Error::AlreadyRunning)));

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    let (identify, close_code) = gateway.await.unwrap();
    assert_eq!(identify["d"]["token"], "test-token");
    assert_eq!(identify["d"]["intents"], 33281);
    assert_eq!(
        close_code,
        Some(1000),
        "shutdown must invalidate the session"
    );

    // Exactly one reply was sent, to the right channel, after fetching the gateway URL.
    let requests: Vec<(String, String)> = seen
        .lock()
        .unwrap()
        .iter()
        .map(|(method, path, _)| (method.clone(), path.clone()))
        .collect();
    assert_eq!(
        requests,
        [
            ("GET".to_owned(), "/api/v10/gateway/bot".to_owned()),
            (
                "POST".to_owned(),
                "/api/v10/channels/20/messages".to_owned()
            ),
        ]
    );

    // The event stream saw every dispatch.
    let mut names = Vec::new();
    while let Ok(Some(event)) = timeout(Duration::from_millis(100), events.next()).await {
        names.push(event.event.name().to_owned());
    }
    assert_eq!(
        names,
        [
            "shardReady",
            "clientReady",
            "messageCreate",
            "messageCreate"
        ]
    );
}

#[tokio::test]
async fn sequential_mode_preserves_order() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let dispatches = (0..5)
        .map(|i| message_create(2 + i, &(100 + i).to_string(), &i.to_string()))
        .collect();
    let gateway = tokio::spawn(mock_gateway(gateway, dispatches));

    let client = Client::builder("test-token")
        .intents(Intents::GUILD_MESSAGES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .handler_mode(HandlerMode::Sequential)
        .build();

    let (order_tx, mut order_rx) = mpsc::unbounded_channel();
    client.on(Events::MessageCreate, move |_ctx, msg| {
        let order_tx = order_tx.clone();
        async move {
            // Earlier messages sleep longer; sequential mode must still keep the order.
            let index: u64 = msg.content.parse()?;
            tokio::time::sleep(Duration::from_millis(50 - index * 10)).await;
            order_tx.send(index)?;
            Ok(())
        }
    });

    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let mut order = Vec::new();
    for _ in 0..5 {
        order.push(timeout(STEP, order_rx.recv()).await.unwrap().unwrap());
    }
    assert_eq!(order, [0, 1, 2, 3, 4]);

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
}

#[tokio::test]
async fn handler_errors_do_not_stop_the_client() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let gateway = tokio::spawn(mock_gateway(
        gateway,
        vec![
            message_create(2, "10", "fail"),
            message_create(3, "11", "ok"),
        ],
    ));

    let client = Client::builder("test-token")
        .intents(Intents::GUILD_MESSAGES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .build();
    let (ok_tx, mut ok_rx) = mpsc::unbounded_channel();
    client.on(Events::MessageCreate, move |ctx, msg| {
        let ok_tx = ok_tx.clone();
        async move {
            if msg.content == "fail" {
                return Err("handler failed on purpose".into());
            }
            ok_tx.send((msg.id, ctx.cache().is_none()))?;
            Ok(())
        }
    });

    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let (id, no_cache) = timeout(STEP, ok_rx.recv()).await.unwrap().unwrap();
    assert_eq!(id.get(), 11);
    assert!(no_cache, "a client has no cache unless configured (D-170)");
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
    // The stream event type is public API.
    let _: fn(Arc<djs_rs::event::payload::ShardResume>) -> Dispatch = Dispatch::ShardResume;
}

#[tokio::test]
async fn a_handler_shuts_the_client_down() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let gateway = tokio::spawn(mock_gateway(
        gateway,
        vec![message_create(2, "10", "!stop")],
    ));

    let client = Client::builder("test-token")
        .intents(Intents::GUILD_MESSAGES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .build();
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!stop" {
            // D-293: no captured `Client` needed.
            ctx.shutdown();
        }
        Ok(())
    });

    // `login` returns by itself: nothing outside the handler stops the client.
    timeout(STEP, client.login()).await.unwrap().unwrap();
    gateway.await.unwrap();
    // Once the client has stopped, a shutdown has no effect.
    client.shutdown();
}

#[tokio::test]
async fn auto_sharding_runs_every_recommended_shard() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    // Discord recommends two shards and lets both identify at once.
    let (rest_base, _seen, _created) = mock_rest(gateway_bot(&gateway_url, 2, 999, 2)).await;
    let sessions = tokio::spawn(async move {
        let mut sessions = Vec::new();
        for _ in 0..2 {
            let (stream, _) = timeout(STEP, gateway.accept()).await.unwrap().unwrap();
            let dispatches = vec![message_create(2, "10", "hello")];
            sessions.push(tokio::spawn(mock_session(stream, dispatches)));
        }
        let mut results = Vec::new();
        for session in sessions {
            results.push(session.await.unwrap());
        }
        results
    });

    let client = Client::builder("test-token")
        .intents(Intents::GUILD_MESSAGES)
        .rest(Rest::builder().token("test-token").api(rest_base).build())
        .compression(Compression::None)
        .build();
    let mut events = client.events();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };

    // Each shard delivers its READY and its message, tagged with its shard ID; `ClientReady`
    // comes once, from the last shard to become ready (D-224).
    let mut received = Vec::new();
    for _ in 0..5 {
        let event = timeout(STEP, events.next()).await.unwrap().unwrap();
        received.push((event.shard.to_string(), event.event.name().to_owned()));
    }
    let ready: Vec<_> = received
        .iter()
        .filter(|(_, name)| name == "clientReady")
        .collect();
    assert_eq!(ready.len(), 1, "{received:?}");
    received.retain(|(_, name)| name != "clientReady");
    received.sort();
    let expected = [
        ("[0, 2]", "messageCreate"),
        ("[0, 2]", "shardReady"),
        ("[1, 2]", "messageCreate"),
        ("[1, 2]", "shardReady"),
    ]
    .map(|(shard, name)| (shard.to_owned(), name.to_owned()));
    assert_eq!(received, expected);
    assert!(
        timeout(Duration::from_millis(100), events.next())
            .await
            .is_err(),
        "no second clientReady"
    );

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    let sessions = sessions.await.unwrap();
    let mut identified: Vec<String> = sessions
        .iter()
        .map(|(identify, _)| identify["d"]["shard"].to_string())
        .collect();
    identified.sort();
    assert_eq!(identified, ["[0,2]", "[1,2]"]);
    assert!(sessions.iter().all(|&(_, code)| code == Some(1000)));
}

#[tokio::test]
async fn too_few_session_starts_is_an_error() {
    // Four shards recommended, three session starts left: nothing may connect.
    let (rest_base, seen, _created) = mock_rest(gateway_bot("ws://127.0.0.1:9", 4, 3, 1)).await;
    let client = Client::builder("test-token")
        .rest(Rest::builder().token("test-token").api(rest_base).build())
        .build();

    let error = timeout(STEP, client.login()).await.unwrap().unwrap_err();
    assert!(
        matches!(
            error,
            Error::SessionStartLimit { required: 4, remaining: 3, reset_after }
                if reset_after == Duration::from_secs(3_600)
        ),
        "{error:?}"
    );
    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "only Get Gateway Bot was called"
    );
}

#[tokio::test]
async fn a_shard_range_runs_only_those_shards() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    // Three session starts left are enough for the one shard of this process.
    let (rest_base, _seen, _created) = mock_rest(gateway_bot(&gateway_url, 4, 3, 1)).await;
    let gateway = tokio::spawn(mock_gateway(gateway, Vec::new()));

    let client = Client::builder("test-token")
        .rest(Rest::builder().token("test-token").api(rest_base).build())
        .shards(Shards::Range {
            range: 3..4,
            total: 4,
        })
        .compression(Compression::None)
        .build();
    let mut events = client.events();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let ready = timeout(STEP, events.next()).await.unwrap().unwrap();
    assert_eq!(ready.shard.to_string(), "[3, 4]");

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    let (identify, _) = gateway.await.unwrap();
    assert_eq!(identify["d"]["shard"], json!([3, 4]));
}

#[test]
#[should_panic(expected = "must not be zero")]
fn zero_shards_are_rejected() {
    let _ = Client::builder("test-token").shards(Shards::Count(0));
}

#[test]
#[should_panic(expected = "within the total")]
fn a_range_past_the_total_is_rejected() {
    let _ = Client::builder("test-token").shards(Shards::Range {
        range: 2..5,
        total: 4,
    });
}

#[tokio::test]
async fn presence_is_sent_when_identifying_and_on_request() {
    use djs_rs::model::gateway::{Activity, Status, payload::UpdatePresence};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", listener.local_addr().unwrap());
    let gateway = tokio::spawn(async move {
        let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
        let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
        let send = |payload: Value| Message::text(payload.to_string());
        ws.send(send(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
            .await
            .unwrap();
        let identify = next_op(&mut ws, 2).await;
        ws.send(send(json!({"t": "READY", "s": 1, "op": 0, "d": {
            "v": 10,
            "user": {"id": "1", "username": "test-bot", "discriminator": "0",
                     "global_name": null, "avatar": null, "bot": true},
            "guilds": [], "session_id": "s", "resume_gateway_url": "ws://127.0.0.1:9",
            "application": {"id": "1"}
        }})))
        .await
        .unwrap();
        let update = next_op(&mut ws, 3).await;
        (identify, update)
    });

    let client = Client::builder("test-token")
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .presence(UpdatePresence::new(
            Status::Online,
            [Activity::playing("with Rust")],
        ))
        .build();
    let busy = UpdatePresence::new(Status::DoNotDisturb, [Activity::custom("Busy")]);
    // Not running yet: nothing to update.
    client.set_presence(&busy).unwrap();

    let mut events = client.events();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let ready = timeout(STEP, events.next()).await.unwrap().unwrap();
    assert_eq!(ready.event.name(), "shardReady");
    client.set_presence(&busy).unwrap();

    let (identify, update) = timeout(STEP, gateway).await.unwrap().unwrap();
    assert_eq!(
        identify["d"]["presence"],
        json!({"since": null, "activities": [{"name": "with Rust", "type": 0}],
               "status": "online", "afk": false})
    );
    assert_eq!(
        update["d"],
        json!({"since": null,
               "activities": [{"name": "Custom Status", "type": 4, "state": "Busy"}],
               "status": "dnd", "afk": false})
    );
    client.shutdown();
    let _ = timeout(STEP, running).await;
}

/// Receives client payloads until one has the given opcode.
async fn next_op(ws: &mut tokio_websockets::WebSocketStream<TcpStream>, op: u64) -> Value {
    loop {
        let message = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
        if let Some(text) = message.as_text() {
            let payload: Value = serde_json::from_str(text).unwrap();
            if payload["op"] == op {
                return payload;
            }
        }
    }
}

/// The mock `GUILD_CREATE` of [`payloads::GUILD`]: 3 members, 2 channels and 2 roles.
fn guild_create(sequence: u64) -> Value {
    serde_json::from_str(&payloads::guild_create(sequence, 3, 0, 2, 2)).unwrap()
}

const GUILD: GuildId = GuildId::new(payloads::GUILD);
/// The guild's owner and another member, and a channel, in [`guild_create`].
const OWNER: UserId = UserId::new(1_000_000_000_000_000_000);
const OTHER: UserId = UserId::new(1_000_000_000_000_000_001);
const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);

#[tokio::test]
async fn the_cache_is_filled_before_handlers_run() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let gateway = tokio::spawn(mock_gateway(
        gateway,
        vec![guild_create(2), message_create(3, "11", "hello")],
    ));

    let config = CacheConfig::builder()
        .members(Limit::Unlimited)
        .messages(Limit::Max(10))
        .build();
    let client = Client::builder("test-token")
        .intents(Intents::GUILDS | Intents::GUILD_MEMBERS | Intents::GUILD_MESSAGES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .cache(config)
        .build();
    assert_eq!(client.cache().unwrap().guilds().len(), 0);

    let (seen_tx, mut seen_rx) = mpsc::unbounded_channel();
    client.on(Events::MessageCreate, move |ctx, msg| {
        let seen_tx = seen_tx.clone();
        async move {
            let cache = ctx.cache().ok_or("no cache")?;
            // D-292: the owner has every permission; another member's roles need the
            // `@everyone` role, which this guild's payload does not carry, so `None`.
            let owner = cache.members(GUILD).get(OWNER).ok_or("no owner")?;
            let other = cache.members(GUILD).get(OTHER).ok_or("no member")?;
            let permissions = (
                owner.permissions(&ctx),
                owner.permissions_in(&ctx, CHANNEL),
                other.permissions(&ctx),
                owner.permissions_in(&ctx, ChannelId::new(77)),
            );
            seen_tx.send((
                permissions,
                cache.current_user().map(|user| user.id),
                cache
                    .guilds()
                    .get(GUILD)
                    .map(|guild| guild.name.to_string()),
                cache.members(GUILD).len(),
                cache
                    .messages(ChannelId::new(20))
                    .get(msg.id)
                    .map(|message| message.content.to_string()),
            ))?;
            Ok(())
        }
    });

    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let seen = timeout(STEP, seen_rx.recv()).await.unwrap().unwrap();
    assert_eq!(
        seen,
        (
            (
                Some(Permissions::all()),
                Some(Permissions::all()),
                None,
                None
            ),
            Some(UserId::new(1)),
            Some("Big Guild".to_owned()),
            3,
            Some("hello".to_owned()),
        )
    );

    // The client and every context share one cache.
    let cache = client.cache().unwrap();
    assert_eq!(cache.channels().in_guild(GUILD).len(), 2);
    assert_eq!(cache.roles(GUILD).len(), 2);

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
    // The cache outlives the session.
    assert_eq!(client.cache().unwrap().guilds().len(), 1);
}

/// The default stores, counting how many the cache creates.
#[derive(Debug, Default)]
struct CountingBackend {
    stores: AtomicUsize,
}

impl Backend for CountingBackend {
    type Store<K, V>
        = DashMapStore<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static;

    fn store<K, V>(&self, _resource: Resource, _limit: Limit) -> DashMapStore<K, V>
    where
        K: Eq + Hash + Copy + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        self.stores.fetch_add(1, Ordering::Relaxed);
        DashMapStore::new()
    }
}

#[tokio::test]
async fn a_custom_backend_works_through_the_client() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let gateway = tokio::spawn(mock_gateway(
        gateway,
        vec![guild_create(2), message_create(3, "11", "hello")],
    ));

    // Choosing a backend enables the cache, with the default configuration.
    let client: Client<CountingBackend> = Client::builder("test-token")
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .cache_backend(CountingBackend::default())
        .build();
    let cache = client.cache().unwrap();
    assert_eq!(cache.config(), &CacheConfig::default());
    // Guilds, channels and roles.
    assert_eq!(cache.backend().stores.load(Ordering::Relaxed), 3);

    let (seen_tx, mut seen_rx) = mpsc::unbounded_channel();
    client.on(
        Events::MessageCreate,
        move |ctx: Context<CountingBackend>, msg| {
            let seen_tx = seen_tx.clone();
            async move {
                let cache = ctx.cache().ok_or("no cache")?;
                seen_tx.send((
                    cache.guilds().contains(GUILD),
                    cache.messages(msg.channel_id).len(),
                ))?;
                Ok(())
            }
        },
    );
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    // The guild is cached; messages are not by default.
    assert_eq!(
        timeout(STEP, seen_rx.recv()).await.unwrap().unwrap(),
        (true, 0)
    );
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
}

#[tokio::test]
async fn the_client_runs_the_configured_sweeps() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let mut old = message_create(2, "11", "old");
    old["d"]["timestamp"] = json!("2015-05-15T00:00:00.000000+00:00");
    let gateway = tokio::spawn(mock_gateway(gateway, vec![old]));

    let config = CacheConfig::builder()
        .messages(Limit::Unlimited)
        .sweep_messages(Some(Sweep::new(
            Duration::from_millis(20),
            Duration::from_secs(3_600),
        )))
        .build();
    let client = Client::builder("test-token")
        .intents(Intents::GUILD_MESSAGES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .cache(config)
        .build();
    let mut events = client.events();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };

    // Subscribers get an event after the cache has it; the sweep then removes the message.
    for name in ["shardReady", "clientReady", "messageCreate"] {
        let event = timeout(STEP, events.next()).await.unwrap().unwrap();
        assert_eq!(event.event.name(), name);
    }
    let messages = || client.cache().unwrap().messages(ChannelId::new(20)).len();
    timeout(STEP, async {
        while messages() != 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the message sweep never ran");

    // Shutting down stops the sweeps along with the shards.
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
}

#[test]
fn a_client_has_no_cache_unless_configured() {
    assert!(Client::builder("test-token").build().cache().is_none());
    let client = Client::builder("test-token")
        .cache(CacheConfig::discord_js())
        .build();
    assert_eq!(
        client.cache().unwrap().config().limit(Resource::Message),
        Limit::Max(200)
    );
}
