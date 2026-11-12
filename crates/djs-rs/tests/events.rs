//! The client's events (D-221–D-228) against a scripted mock gateway: guild availability with
//! and without a cache, previous states, emoji and sticker diffs, `Raw`, and the shard
//! lifecycle (reconnect, resume, invalidated session, errors, fatal close).

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use djs_rs::{
    EventStream, HandlerMode, Shards,
    cache::{CacheConfig, Limit},
    event::Dispatch,
    gateway::Compression,
    model::{
        gateway::CloseCode,
        id::{EmojiId, GuildId},
    },
    prelude::*,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

const STEP: Duration = Duration::from_secs(5);
/// Longer than the 1–5 s Discord asks to wait after an invalidated session.
const INVALIDATED_STEP: Duration = Duration::from_secs(10);

const GUILD: u64 = payloads::GUILD;
const OTHER_GUILD: u64 = 1_400_000_000_000_000_999;

/// One step of a scripted gateway connection.
enum Step {
    /// Send a payload.
    Send(Value),
    /// Send a text frame that is not JSON.
    SendInvalid,
    /// Wait for a client payload with this opcode.
    Expect(u64),
    /// Close the connection with this code.
    Close(u16),
    /// Wait until the client closes the connection.
    Hold,
}

fn send(payload: Value) -> Step {
    Step::Send(payload)
}

/// A `READY` listing `guilds` as unavailable, resuming at `resume_url`.
fn ready(guilds: &[u64], resume_url: &str) -> Step {
    let guilds: Vec<Value> = guilds
        .iter()
        .map(|id| json!({"id": id.to_string(), "unavailable": true}))
        .collect();
    send(json!({"t": "READY", "s": 1, "op": 0, "d": {
        "v": 10,
        "user": {"id": "1", "username": "test-bot", "discriminator": "0", "global_name": null,
                 "avatar": null, "bot": true},
        "guilds": guilds, "session_id": "s", "resume_gateway_url": resume_url,
        "application": {"id": "1"}
    }}))
}

fn dispatch(name: &str, sequence: u64, d: Value) -> Step {
    let mut payload = json!({"t": name, "s": sequence, "op": 0});
    payload["d"] = d;
    send(payload)
}

fn message(id: &str, content: &str) -> Value {
    json!({
        "id": id, "channel_id": "20", "guild_id": "2",
        "author": {"id": "3", "username": "nelly", "discriminator": "0", "global_name": null,
                   "avatar": null},
        "content": content, "timestamp": "2026-11-10T19:42:07.123000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
        "type": 0
    })
}

/// The `GUILD_CREATE` of the shared payloads (50 emojis, no stickers), for `id`.
fn guild_create(sequence: u64, id: u64) -> Step {
    let json =
        payloads::guild_create(sequence, 3, 0, 2, 2).replace(&GUILD.to_string(), &id.to_string());
    send(serde_json::from_str(&json).unwrap())
}

/// Serves one scripted connection per element of `connections`, in order.
fn serve(listener: TcpListener, connections: Vec<Vec<Step>>) -> JoinHandle<()> {
    tokio::spawn(async move {
        for script in connections {
            let (stream, _) = timeout(INVALIDATED_STEP, listener.accept())
                .await
                .expect("the client did not connect")
                .unwrap();
            run(stream, script).await;
        }
    })
}

async fn run(stream: TcpStream, script: Vec<Step>) {
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    let text = |payload: Value| Message::text(payload.to_string());
    ws.send(text(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
        .await
        .unwrap();
    for step in script {
        match step {
            Step::Send(payload) => ws.send(text(payload)).await.unwrap(),
            Step::SendInvalid => ws.send(Message::text("{not json")).await.unwrap(),
            Step::Expect(op) => expect(&mut ws, op).await,
            Step::Close(code) => {
                let code = tokio_websockets::CloseCode::try_from(code).unwrap();
                let _ = ws.send(Message::close(Some(code), "closing")).await;
                return;
            }
            Step::Hold => hold(&mut ws).await,
        }
    }
}

async fn expect(ws: &mut WebSocketStream<TcpStream>, op: u64) {
    loop {
        let message = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
        if let Some(text) = message.as_text() {
            let payload: Value = serde_json::from_str(text).unwrap();
            if payload["op"] == op {
                return;
            }
        }
    }
}

async fn hold(ws: &mut WebSocketStream<TcpStream>) {
    while let Ok(Some(Ok(message))) = timeout(INVALIDATED_STEP, ws.next()).await {
        if message.is_close() {
            return;
        }
    }
}

/// A client for the mock gateway at `url`.
fn new_client(url: &str, cache: Option<CacheConfig>) -> Client {
    let mut builder = Client::builder("test-token")
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::GUILD_EXPRESSIONS)
        .gateway_url(url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .handler_mode(HandlerMode::Sequential);
    if let Some(cache) = cache {
        builder = builder.cache(cache);
    }
    builder.build()
}

async fn bind() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, url)
}

/// Receives events until `last` (by name), returning every name on the way.
async fn names_until(events: &mut EventStream, last: &str, step: Duration) -> Vec<String> {
    let mut names = Vec::new();
    loop {
        let event = timeout(step, events.next())
            .await
            .unwrap_or_else(|_| panic!("no {last} after {names:?}"))
            .unwrap();
        names.push(event.event.name().to_owned());
        if names.last().is_some_and(|name| name == last) {
            return names;
        }
    }
}

/// Logs `client` in, waits for `last`, shuts down and returns the event names.
async fn session(client: &Client, gateway: JoinHandle<()>, last: &str) -> Vec<String> {
    let mut events = client.events();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let names = names_until(&mut events, last, INVALIDATED_STEP).await;
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
    names
}

#[tokio::test]
async fn guild_availability_is_tracked_with_and_without_a_cache() {
    for cache in [None, Some(CacheConfig::default())] {
        let (listener, url) = bind().await;
        let gateway = serve(
            listener,
            vec![vec![
                Step::Expect(2),
                ready(&[GUILD], &url),
                // From READY: available, not joined.
                guild_create(2, GUILD),
                // Not in READY: joined.
                guild_create(3, OTHER_GUILD),
                dispatch(
                    "GUILD_DELETE",
                    4,
                    json!({"id": GUILD.to_string(), "unavailable": true}),
                ),
                guild_create(5, GUILD),
                dispatch("GUILD_DELETE", 6, json!({"id": OTHER_GUILD.to_string()})),
                Step::Hold,
            ]],
        );
        let client = new_client(&url, cache.clone());
        let olds = Arc::new(Mutex::new(Vec::new()));
        for event in ["unavailable", "delete"] {
            let olds = Arc::clone(&olds);
            if event == "unavailable" {
                client.on(Events::GuildUnavailable, move |_ctx, gone| {
                    olds.lock().unwrap().push((gone.id, gone.old.is_some()));
                    async { Ok(()) }
                });
            } else {
                client.on(Events::GuildDelete, move |_ctx, gone| {
                    olds.lock().unwrap().push((gone.id, gone.old.is_some()));
                    async { Ok(()) }
                });
            }
        }

        let names = session(&client, gateway, "guildDelete").await;
        assert_eq!(
            names,
            [
                "shardReady",
                "clientReady",
                "guildAvailable",
                "guildCreate",
                "guildUnavailable",
                "guildAvailable",
                "guildDelete",
            ],
            "cache: {cache:?}"
        );
        let cached = cache.is_some();
        assert_eq!(
            *olds.lock().unwrap(),
            [
                (GuildId::new(GUILD), cached),
                (GuildId::new(OTHER_GUILD), cached)
            ]
        );
    }
}

#[tokio::test]
async fn handlers_get_the_previous_state() {
    let (listener, url) = bind().await;
    let mut edited = message("11", "edited");
    edited["edited_timestamp"] = json!("2026-11-10T19:43:00.000000+00:00");
    let gateway = serve(
        listener,
        vec![vec![
            Step::Expect(2),
            ready(&[], &url),
            dispatch("MESSAGE_CREATE", 2, message("11", "hello")),
            dispatch("MESSAGE_UPDATE", 3, edited),
            dispatch("MESSAGE_CREATE", 4, message("12", "second")),
            dispatch(
                "MESSAGE_DELETE",
                5,
                json!({"id": "11", "channel_id": "20", "guild_id": "2"}),
            ),
            dispatch(
                "MESSAGE_DELETE_BULK",
                6,
                json!({"ids": ["12", "13"], "channel_id": "20", "guild_id": "2"}),
            ),
            Step::Hold,
        ]],
    );
    let client = new_client(
        &url,
        Some(CacheConfig::builder().messages(Limit::Max(10)).build()),
    );
    let seen = Arc::new(Mutex::new(Vec::new()));
    {
        let seen = Arc::clone(&seen);
        client.on(Events::MessageUpdate, move |_ctx, update| {
            let old = update.old.as_ref().map(|old| old.content.to_string());
            // The payload dereferences to the new message (D-228).
            seen.lock()
                .unwrap()
                .push(format!("update {old:?} -> {}", update.content));
            async { Ok(()) }
        });
    }
    {
        let seen = Arc::clone(&seen);
        client.on(Events::MessageDelete, move |_ctx, delete| {
            let old = delete.old.as_ref().map(|old| old.content.to_string());
            seen.lock()
                .unwrap()
                .push(format!("delete {} {old:?}", delete.id));
            async { Ok(()) }
        });
    }
    {
        let seen = Arc::clone(&seen);
        client.on(Events::MessageBulkDelete, move |_ctx, bulk| {
            let old: Vec<String> = bulk.old.iter().map(|old| old.content.to_string()).collect();
            seen.lock()
                .unwrap()
                .push(format!("bulk {} {old:?}", bulk.ids.len()));
            async { Ok(()) }
        });
    }

    session(&client, gateway, "messageDeleteBulk").await;
    assert_eq!(
        *seen.lock().unwrap(),
        [
            r#"update Some("hello") -> edited"#,
            r#"delete 11 Some("edited")"#,
            r#"bulk 2 ["second"]"#,
        ]
    );
}

/// The shared `GUILD_CREATE`'s emoji `i`, with `name`.
fn emoji(i: u64, name: &str) -> Value {
    json!({"id": (1_250_000_000_000_000_000 + i).to_string(), "name": name, "roles": [],
           "require_colons": true, "managed": false, "animated": false, "available": true})
}

#[tokio::test]
async fn emoji_and_sticker_lists_are_diffed_with_a_cache() {
    // Emoji 0 renamed, emoji 49 removed, emoji 1000 added.
    let mut emojis: Vec<Value> = (1..49).map(|i| emoji(i, &format!("emoji_{i}"))).collect();
    emojis.insert(0, emoji(0, "renamed"));
    emojis.push(emoji(1000, "new"));
    let sticker = json!({"id": "1300000000000000001", "name": "wave", "description": null,
                         "tags": "wave", "type": 2, "format_type": 1, "available": true,
                         "guild_id": GUILD.to_string()});
    let script = |url: &str| {
        vec![
            Step::Expect(2),
            ready(&[], url),
            guild_create(2, GUILD),
            dispatch(
                "GUILD_EMOJIS_UPDATE",
                3,
                json!({"guild_id": GUILD.to_string(), "emojis": emojis}),
            ),
            dispatch(
                "GUILD_STICKERS_UPDATE",
                4,
                json!({"guild_id": GUILD.to_string(), "stickers": [sticker]}),
            ),
            Step::Hold,
        ]
    };

    // With emojis and stickers in the cache: one event per change, in discord.js' order.
    let (listener, url) = bind().await;
    let gateway = serve(listener, vec![script(&url)]);
    let cache = CacheConfig::builder()
        .emojis(Limit::Unlimited)
        .stickers(Limit::Unlimited)
        .build();
    let client = new_client(&url, Some(cache));
    let changes = Arc::new(Mutex::new(Vec::new()));
    {
        let changes = Arc::clone(&changes);
        client.on(Events::GuildEmojiUpdate, move |_ctx, update| {
            changes
                .lock()
                .unwrap()
                .push(format!("{} -> {}", update.old.name, update.name));
            async { Ok(()) }
        });
    }
    {
        let changes = Arc::clone(&changes);
        client.on(Events::GuildEmojiDelete, move |_ctx, delete| {
            assert_eq!(delete.guild_id, GuildId::new(GUILD));
            changes
                .lock()
                .unwrap()
                .push(format!("deleted {}", delete.name));
            async { Ok(()) }
        });
    }
    let names = session(&client, gateway, "stickerCreate").await;
    assert_eq!(
        names,
        [
            "shardReady",
            "clientReady",
            "guildCreate",
            "guildEmojisUpdate",
            "emojiUpdate",
            "emojiCreate",
            "emojiDelete",
            "guildStickersUpdate",
            "stickerCreate",
        ]
    );
    assert_eq!(
        *changes.lock().unwrap(),
        ["emoji_0 -> renamed", "deleted emoji_49"]
    );
    let cache = client.cache().unwrap();
    assert!(
        cache
            .emojis(GuildId::new(GUILD))
            .get(EmojiId::new(1_250_000_000_000_000_000 + 1000))
            .is_some()
    );

    // Without them, only the lists arrive (D-223).
    let (listener, url) = bind().await;
    let gateway = serve(listener, vec![script(&url)]);
    let client = new_client(&url, Some(CacheConfig::default()));
    let names = session(&client, gateway, "guildStickersUpdate").await;
    assert_eq!(
        names,
        [
            "shardReady",
            "clientReady",
            "guildCreate",
            "guildEmojisUpdate",
            "guildStickersUpdate",
        ]
    );
}

#[tokio::test]
async fn raw_sees_every_dispatch_first() {
    let (listener, url) = bind().await;
    let gateway = serve(
        listener,
        vec![vec![
            Step::Expect(2),
            ready(&[], &url),
            dispatch("SOMETHING_NEW", 2, json!({"x": 1})),
            dispatch("MESSAGE_CREATE", 3, message("11", "hello")),
            Step::Hold,
        ]],
    );
    let client = new_client(&url, None);
    let order = Arc::new(Mutex::new(Vec::new()));
    {
        let order = Arc::clone(&order);
        client.on(Events::Raw, move |_ctx, raw| {
            order.lock().unwrap().push(format!("raw {}", raw.name()));
            async { Ok(()) }
        });
    }
    {
        let order = Arc::clone(&order);
        client.on(Events::MessageCreate, move |_ctx, _message| {
            order.lock().unwrap().push("messageCreate".to_owned());
            async { Ok(()) }
        });
    }
    // The stream never receives `Raw`, and reports the unknown dispatch by its name.
    let names = session(&client, gateway, "messageCreate").await;
    assert_eq!(
        names,
        [
            "shardReady",
            "clientReady",
            "SOMETHING_NEW",
            "messageCreate"
        ]
    );
    assert_eq!(
        *order.lock().unwrap(),
        [
            "raw READY",
            "raw SOMETHING_NEW",
            "raw MESSAGE_CREATE",
            "messageCreate"
        ]
    );
}

#[tokio::test]
async fn reconnects_resumes_and_invalidated_sessions_are_events() {
    let (listener, url) = bind().await;
    let gateway = serve(
        listener,
        vec![
            // Discord asks for a reconnect.
            vec![
                Step::Expect(2),
                ready(&[], &url),
                dispatch("MESSAGE_CREATE", 2, message("11", "one")),
                send(json!({"op": 7, "d": null})),
                Step::Hold,
            ],
            // The client resumes; two missed events are replayed; then the session is lost.
            vec![
                Step::Expect(6),
                dispatch("MESSAGE_CREATE", 3, message("12", "two")),
                dispatch("MESSAGE_CREATE", 4, message("13", "three")),
                dispatch("RESUMED", 5, Value::Null),
                send(json!({"op": 9, "d": false})),
                Step::Hold,
            ],
            // The client identifies again.
            vec![Step::Expect(2), ready(&[], &url), Step::Hold],
        ],
    );
    let client = new_client(&url, None);
    let replayed = Arc::new(Mutex::new(None));
    {
        let replayed = Arc::clone(&replayed);
        client.on(Events::ShardResume, move |_ctx, resume| {
            *replayed.lock().unwrap() = Some(resume.replayed);
            async { Ok(()) }
        });
    }
    let mut events = client.events();
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    let first = names_until(&mut events, "shardResume", STEP).await;
    assert_eq!(
        first,
        [
            "shardReady",
            "clientReady",
            "messageCreate",
            "shardReconnecting",
            "messageCreate",
            "messageCreate",
            "shardResume",
        ]
    );
    // A second READY is a `ShardReady`, but `ClientReady` came once (D-224).
    let second = names_until(&mut events, "shardReady", INVALIDATED_STEP).await;
    assert_eq!(second, ["invalidated", "shardReconnecting", "shardReady"]);
    assert_eq!(*replayed.lock().unwrap(), Some(2));

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    gateway.await.unwrap();
}

#[tokio::test]
async fn errors_and_fatal_closes_are_events() {
    let (listener, url) = bind().await;
    let gateway = serve(
        listener,
        vec![vec![
            Step::Expect(2),
            ready(&[], &url),
            Step::SendInvalid,
            dispatch("MESSAGE_CREATE", 2, message("11", "still here")),
            Step::Close(4004),
        ]],
    );
    let client = new_client(&url, None);
    let closed = Arc::new(Mutex::new(None));
    {
        let closed = Arc::clone(&closed);
        client.on(Events::ShardDisconnect, move |_ctx, code| {
            *closed.lock().unwrap() = Some(*code);
            async { Ok(()) }
        });
    }
    let mut events = client.events();
    let result = timeout(STEP, client.login()).await.unwrap();
    assert!(result.is_err(), "a fatal close stops the client");
    gateway.await.unwrap();

    let mut names = Vec::new();
    while let Ok(Some(event)) = timeout(Duration::from_millis(100), events.next()).await {
        if let Dispatch::ShardError(error) = &event.event {
            assert!(!error.is_fatal());
        }
        names.push(event.event.name().to_owned());
    }
    assert_eq!(
        names,
        [
            "shardReady",
            "clientReady",
            "shardError",
            "messageCreate",
            "shardDisconnect"
        ]
    );
    assert_eq!(
        *closed.lock().unwrap(),
        Some(CloseCode::AuthenticationFailed)
    );
}
