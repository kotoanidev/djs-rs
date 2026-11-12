//! Tests `Shard` against a scripted mock of the Discord gateway.

use std::time::Duration;

use djs_gateway::{Compression, Config, ReceiveErrorKind, Shard, ShardState};
use djs_model::{
    gateway::{
        CloseCode, DispatchEvent, GatewayEvent, Intents, ShardId,
        payload::{Heartbeat, RequestGuildMembers},
    },
    id::GuildId,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    net::{TcpListener, TcpStream},
    time::{Instant, timeout},
};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

type Server = WebSocketStream<TcpStream>;

const TOKEN: &str = "test-token";
const INTENTS: Intents = Intents::GUILDS.union(Intents::GUILD_MESSAGES);
const STEP: Duration = Duration::from_secs(5);

async fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, url)
}

/// Accepts one WebSocket connection, returning it with the request's query string.
async fn accept(listener: &TcpListener) -> (Server, String) {
    let (stream, _) = timeout(STEP, listener.accept())
        .await
        .expect("no connection")
        .unwrap();
    let (request, server) = ServerBuilder::new().accept(stream).await.unwrap();
    (server, request.uri().query().unwrap_or_default().to_owned())
}

async fn send(server: &mut Server, payload: Value) {
    server
        .send(Message::text(payload.to_string()))
        .await
        .unwrap();
}

/// Receives the next text payload sent by the client.
async fn recv(server: &mut Server) -> Value {
    loop {
        let message = timeout(STEP, server.next())
            .await
            .expect("client sent nothing")
            .expect("connection ended")
            .unwrap();
        if let Some(text) = message.as_text() {
            return serde_json::from_str(text).unwrap();
        }
    }
}

/// Receives client payloads until one has the given opcode, answering heartbeats on the way.
async fn recv_op(server: &mut Server, op: u64) -> Value {
    loop {
        let payload = recv(server).await;
        if payload["op"] == op {
            return payload;
        }
        if payload["op"] == 1 {
            send(server, json!({"op": 11})).await;
        }
    }
}

fn hello(interval_ms: u64) -> Value {
    json!({"t": null, "s": null, "op": 10, "d": {"heartbeat_interval": interval_ms}})
}

fn ready(sequence: u64, resume_url: &str) -> Value {
    json!({"t": "READY", "s": sequence, "op": 0, "d": {
        "v": 10,
        "user": {"id": "1", "username": "test-bot", "discriminator": "0", "global_name": null,
                 "avatar": null, "bot": true},
        "guilds": [{"id": "2", "unavailable": true}],
        "session_id": "session-1",
        "resume_gateway_url": resume_url,
        "shard": [0, 1],
        "application": {"id": "1", "flags": 0}
    }})
}

fn message_create(sequence: u64) -> Value {
    json!({"t": "MESSAGE_CREATE", "s": sequence, "op": 0, "d": {
        "id": "10", "channel_id": "20", "guild_id": "2",
        "author": {"id": "3", "username": "nelly", "discriminator": "0", "global_name": null,
                   "avatar": null},
        "content": "!ping", "timestamp": "2026-11-10T19:42:07.123000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
        "type": 0
    }})
}

fn shard(url: &str) -> Shard {
    let config = Config::builder(TOKEN, INTENTS)
        .gateway_url(url)
        .compression(Compression::None)
        .build();
    Shard::with_config(ShardId::ONE, config)
}

/// Polls the shard until `stop` returns true for an event, collecting all events.
async fn collect_until(
    shard: &mut Shard,
    mut stop: impl FnMut(&GatewayEvent) -> bool,
) -> Vec<GatewayEvent> {
    let mut events = Vec::new();
    loop {
        let event = timeout(STEP, shard.next_event())
            .await
            .expect("shard produced no event in time")
            .expect("shard ended")
            .expect("shard returned an error");
        let done = stop(&event);
        events.push(event);
        if done {
            return events;
        }
    }
}

#[tokio::test]
async fn identify_dispatch_reconnect_and_resume() {
    let (listener, url) = listener().await;
    let resume_url = url.clone();

    let server = tokio::spawn(async move {
        // First connection: identify, receive two dispatches, then a reconnect request.
        let (mut first, query) = accept(&listener).await;
        assert_eq!(query, "v=10&encoding=json");
        send(&mut first, hello(45_000)).await;
        let identify = recv_op(&mut first, 2).await;
        assert_eq!(identify["d"]["token"], TOKEN);
        assert_eq!(identify["d"]["intents"], INTENTS.bits());
        assert_eq!(identify["d"]["shard"], json!([0, 1]));
        assert_eq!(identify["d"]["properties"]["browser"], "djs-rs");
        send(&mut first, ready(1, &resume_url)).await;
        send(&mut first, message_create(2)).await;
        send(
            &mut first,
            json!({"t": null, "s": null, "op": 7, "d": null}),
        )
        .await;

        // Second connection: the shard must resume with the last sequence number.
        let (mut second, _) = accept(&listener).await;
        send(&mut second, hello(45_000)).await;
        let resume = recv_op(&mut second, 6).await;
        assert_eq!(resume["d"]["token"], TOKEN);
        assert_eq!(resume["d"]["session_id"], "session-1");
        assert_eq!(resume["d"]["seq"], 2);
        send(
            &mut second,
            json!({"t": "RESUMED", "s": 3, "op": 0, "d": {}}),
        )
        .await;
        // Keep the connection open until the client is done.
        let _ = timeout(STEP, second.next()).await;
    });

    let mut shard = shard(&url);
    let events = collect_until(&mut shard, |event| {
        matches!(event, GatewayEvent::Dispatch(_, DispatchEvent::Resumed))
    })
    .await;

    let summary: Vec<String> = events
        .iter()
        .map(|event| match event {
            GatewayEvent::Dispatch(sequence, event) => format!("{}#{sequence}", event.name()),
            other => format!("{other:?}").split('(').next().unwrap().to_owned(),
        })
        .collect();
    assert_eq!(
        summary,
        [
            "Hello",
            "READY#1",
            "MESSAGE_CREATE#2",
            "Reconnect",
            "Hello",
            "RESUMED#3"
        ]
    );
    let GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message)) = &events[2] else {
        unreachable!()
    };
    assert_eq!(message.content, "!ping");
    assert_eq!(shard.state(), ShardState::Active);
    assert_eq!(shard.session_id(), Some("session-1"));
    assert_eq!(shard.sequence(), Some(3));

    drop(shard);
    server.await.unwrap();
}

#[tokio::test]
async fn heartbeats_are_sent_and_acknowledged() {
    let (listener, url) = listener().await;
    let server = tokio::spawn(async move {
        let (mut connection, _) = accept(&listener).await;
        send(&mut connection, hello(100)).await;
        let mut heartbeats = 0;
        while heartbeats < 3 {
            let payload = recv(&mut connection).await;
            if payload["op"] == 1 {
                heartbeats += 1;
                send(&mut connection, json!({"op": 11})).await;
            }
        }
        heartbeats
    });

    let mut shard = shard(&url);
    let mut acks = 0;
    collect_until(&mut shard, |event| {
        if matches!(event, GatewayEvent::HeartbeatAck) {
            acks += 1;
        }
        acks == 3
    })
    .await;
    assert!(shard.latency().is_some());
    assert_eq!(server.await.unwrap(), 3);
}

#[tokio::test]
async fn zombied_connection_is_replaced_respecting_the_identify_limit() {
    let (listener, url) = listener().await;
    let server = tokio::spawn(async move {
        let (mut first, _) = accept(&listener).await;
        send(&mut first, hello(100)).await;
        // Identify, then never acknowledge heartbeats: the shard must give up on this
        // connection once a heartbeat goes unanswered.
        let identify = loop {
            let payload = recv(&mut first).await;
            if payload["op"] == 2 {
                break Instant::now();
            }
        };
        let (mut second, _) = accept(&listener).await;
        send(&mut second, hello(45_000)).await;
        // Without a session the shard identifies again — but not sooner than 5 s after the
        // previous identify.
        let again = loop {
            let payload = timeout(Duration::from_secs(10), recv(&mut second))
                .await
                .unwrap();
            if payload["op"] == 2 {
                break Instant::now();
            }
        };
        again - identify
    });

    let mut shard = shard(&url);
    let gap = timeout(Duration::from_secs(15), async {
        tokio::pin!(server);
        loop {
            tokio::select! {
                result = &mut server => break result.unwrap(),
                _ = shard.next_event() => {}
            }
        }
    })
    .await
    .expect("shard did not reconnect and identify");
    assert!(
        gap >= Duration::from_secs(5),
        "identified again after only {gap:?}"
    );
}

#[tokio::test]
async fn fatal_close_code_stops_the_shard() {
    let (listener, url) = listener().await;
    let server = tokio::spawn(async move {
        let (mut connection, _) = accept(&listener).await;
        send(&mut connection, hello(45_000)).await;
        recv_op(&mut connection, 2).await;
        let code = tokio_websockets::CloseCode::try_from(4004).unwrap();
        connection
            .send(Message::close(Some(code), "Authentication failed."))
            .await
            .unwrap();
        let _ = timeout(STEP, connection.next()).await;
    });

    let mut shard = shard(&url);
    let error = loop {
        match timeout(STEP, shard.next_event()).await.unwrap() {
            Some(Ok(_)) => {}
            Some(Err(error)) => break error,
            None => panic!("shard ended without reporting the fatal error"),
        }
    };
    assert!(error.is_fatal());
    assert!(matches!(
        error.kind(),
        ReceiveErrorKind::FatallyClosed {
            code: CloseCode::AuthenticationFailed
        }
    ));
    assert!(shard.next_event().await.is_none());
    assert_eq!(
        shard.state(),
        ShardState::FatallyClosed(CloseCode::AuthenticationFailed)
    );
    server.await.unwrap();
}

#[tokio::test]
async fn commands_wait_for_an_active_session() {
    let (listener, url) = listener().await;
    let resume_url = url.clone();
    let server = tokio::spawn(async move {
        let (mut connection, _) = accept(&listener).await;
        send(&mut connection, hello(45_000)).await;
        // The queued command must not be sent before identifying.
        let first = recv(&mut connection).await;
        assert_eq!(first["op"], 2, "expected identify first, got {first}");
        send(&mut connection, ready(1, &resume_url)).await;
        let command = recv(&mut connection).await;
        assert_eq!(command, json!({"op": 1, "d": 42}));
    });

    let mut shard = shard(&url);
    shard.sender().command(&Heartbeat(Some(42))).unwrap();
    collect_until(&mut shard, |event| {
        matches!(event, GatewayEvent::Dispatch(_, DispatchEvent::Ready(_)))
    })
    .await;
    // Keep driving the shard so it flushes the queued command.
    let _ = timeout(Duration::from_millis(500), shard.next_event()).await;
    server.await.unwrap();
}

/// Discord's limit minus the room the shard keeps for heartbeats, identify and resume.
const COMMANDS_PER_WINDOW: usize = 115;
/// How long the server waits to be sure no more commands are coming.
const QUIET: Duration = Duration::from_secs(1);

/// Queues `count` member requests, numbered by their `limit` field from 1.
fn queue_member_requests(shard: &Shard, count: u32) {
    let sender = shard.sender();
    for n in 1..=count {
        sender
            .command(&RequestGuildMembers::query(GuildId::new(2), "", n))
            .unwrap();
    }
}

/// Receives member requests (opcode 8) until nothing arrives for [`QUIET`], answering
/// heartbeats on the way. Returns the requests' numbers.
async fn recv_member_requests(server: &mut Server) -> Vec<u64> {
    let mut requests = Vec::new();
    while let Ok(message) = timeout(QUIET, server.next()).await {
        let message = message.expect("connection ended").unwrap();
        let Some(text) = message.as_text() else {
            continue;
        };
        let payload: Value = serde_json::from_str(text).unwrap();
        match payload["op"].as_u64() {
            Some(8) => requests.push(payload["d"]["limit"].as_u64().unwrap()),
            Some(1) => send(server, json!({"op": 11})).await,
            _ => panic!("unexpected payload {payload}"),
        }
    }
    requests
}

/// Drives the shard until the server task finishes.
async fn drive_until<T>(shard: &mut Shard, server: tokio::task::JoinHandle<T>) -> T {
    let driven = async {
        tokio::pin!(server);
        loop {
            tokio::select! {
                result = &mut server => break result.unwrap(),
                _ = shard.next_event() => {}
            }
        }
    };
    timeout(Duration::from_secs(30), driven)
        .await
        .expect("the mock server did not finish")
}

// The window itself (115 commands per 61 s, refilling one by one) is unit-tested in
// `src/send_limit.rs`; waiting a whole minute here would slow the suite down.
#[tokio::test]
async fn commands_over_the_send_limit_are_held() {
    let (listener, url) = listener().await;
    let resume_url = url.clone();
    let server = tokio::spawn(async move {
        let (mut connection, _) = accept(&listener).await;
        send(&mut connection, hello(45_000)).await;
        recv_op(&mut connection, 2).await;
        send(&mut connection, ready(1, &resume_url)).await;
        recv_member_requests(&mut connection).await
    });

    let mut shard = shard(&url);
    queue_member_requests(&shard, 200);
    let requests = drive_until(&mut shard, server).await;

    // The first window's worth goes out at once, in order; the rest is held.
    let expected: Vec<u64> = (1..=115).collect();
    assert_eq!(requests, expected);
}

#[tokio::test]
async fn a_new_connection_starts_with_a_fresh_send_limit() {
    let (listener, url) = listener().await;
    let resume_url = url.clone();
    let server = tokio::spawn(async move {
        let (mut first, _) = accept(&listener).await;
        send(&mut first, hello(45_000)).await;
        recv_op(&mut first, 2).await;
        send(&mut first, ready(1, &resume_url)).await;
        let before = recv_member_requests(&mut first).await;
        // The window is full; ask the shard to reconnect.
        send(&mut first, json!({"op": 7, "d": null})).await;
        let _ = timeout(STEP, first.next()).await;

        let (mut second, _) = accept(&listener).await;
        send(&mut second, hello(45_000)).await;
        recv_op(&mut second, 6).await;
        send(
            &mut second,
            json!({"t": "RESUMED", "s": 2, "op": 0, "d": {}}),
        )
        .await;
        let after = recv_member_requests(&mut second).await;
        (before, after)
    });

    let mut shard = shard(&url);
    queue_member_requests(&shard, 120);
    let (before, after) = drive_until(&mut shard, server).await;

    assert_eq!(before.len(), COMMANDS_PER_WINDOW);
    // The held commands go out right after resuming instead of a window later.
    assert_eq!(after, [116, 117, 118, 119, 120]);
}

#[tokio::test]
async fn sender_reports_dropped_shard() {
    let shard = shard("ws://127.0.0.1:9");
    let sender = shard.sender();
    assert!(!sender.is_closed());
    drop(shard);
    assert!(sender.is_closed());
    assert!(sender.command(&Heartbeat(None)).is_err());
}

#[cfg(feature = "zstd-stream")]
#[tokio::test]
async fn zstd_stream_payloads_are_decompressed() {
    use zstd::stream::raw::{Encoder, InBuffer, Operation, OutBuffer};

    /// Compresses like Discord: one stream per connection, flushed after every message.
    fn compress(encoder: &mut Encoder<'static>, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut input = InBuffer::around(data);
        while input.pos() < data.len() {
            out.reserve(4096);
            let position = out.len();
            encoder
                .run(&mut input, &mut OutBuffer::around_pos(&mut out, position))
                .unwrap();
        }
        loop {
            out.reserve(4096);
            let position = out.len();
            if encoder
                .flush(&mut OutBuffer::around_pos(&mut out, position))
                .unwrap()
                == 0
            {
                return out;
            }
        }
    }

    let (listener, url) = listener().await;
    let resume_url = url.clone();
    let server = tokio::spawn(async move {
        let (mut connection, query) = accept(&listener).await;
        assert_eq!(query, "v=10&encoding=json&compress=zstd-stream");
        let mut encoder = Encoder::new(3).unwrap();
        for payload in [hello(45_000), ready(1, &resume_url), message_create(2)] {
            let compressed = compress(&mut encoder, payload.to_string().as_bytes());
            connection.send(Message::binary(compressed)).await.unwrap();
            if payload["op"] == 10 {
                recv_op(&mut connection, 2).await;
            }
        }
        let _ = timeout(STEP, connection.next()).await;
    });

    let config = Config::builder(TOKEN, INTENTS)
        .gateway_url(&url)
        .compression(Compression::ZstdStream)
        .build();
    let mut shard = Shard::with_config(ShardId::ONE, config);
    let events = collect_until(&mut shard, |event| {
        matches!(
            event,
            GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(_))
        )
    })
    .await;
    assert_eq!(events.len(), 3);
    drop(shard);
    server.await.unwrap();
}
