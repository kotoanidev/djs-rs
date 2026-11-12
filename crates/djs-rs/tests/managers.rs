//! The managers and the structure methods (D-229–D-233) against a mock REST API: the cache is
//! read first and filled with what Discord returns, `force` skips it, lists keep Discord's
//! order, and every action sends the request Discord documents.

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use djs_rs::{
    Shards,
    cache::{CacheConfig, Limit},
    gateway::Compression,
    model::{
        gateway::payload::GuildMemberAdd,
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
        message::Message,
        rest::{CreateGuildBan, GetChannelMessages, GetGuildBans, ListGuildMembers},
        util::Timestamp,
    },
    prelude::*,
    rest::Rest,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle, time::timeout};
use tokio_websockets::{Message as WsMessage, ServerBuilder};

const STEP: Duration = Duration::from_secs(5);

const GUILD: GuildId = GuildId::new(payloads::GUILD);
const CHANNEL: ChannelId = ChannelId::new(20);
/// A user the mock API does not know (404 Unknown Member / Unknown User).
const UNKNOWN: u64 = 404;

/// One request the mock API received.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    /// The path after `/api/v10`.
    path: String,
    body: Value,
    reason: Option<String>,
}

type Log = Arc<Mutex<Vec<Seen>>>;

fn user(id: &str) -> Value {
    json!({"id": id, "username": format!("user-{id}"), "discriminator": "0",
           "global_name": null, "avatar": null})
}

fn member(id: &str, nick: &Value) -> Value {
    json!({"user": user(id), "nick": nick, "roles": [],
           "joined_at": "2026-01-01T00:00:00.000000+00:00",
           "deaf": false, "mute": false, "flags": 0, "pending": false})
}

fn role(id: &str) -> Value {
    json!({"id": id, "name": format!("role-{id}"), "color": 0, "hoist": false, "position": 1,
           "permissions": "0", "managed": false, "mentionable": false, "flags": 0,
           "colors": {"primary_color": 0, "secondary_color": null, "tertiary_color": null}})
}

fn message(channel: &str, id: &str, content: &Value) -> Value {
    json!({"id": id, "channel_id": channel, "author": user("3"), "content": content,
           "timestamp": "2026-11-10T19:42:07.123000+00:00", "edited_timestamp": null,
           "tts": false, "mention_everyone": false, "pinned": false, "type": 0})
}

fn channel(id: &str) -> Value {
    json!({"id": id, "type": 0, "guild_id": GUILD.to_string(), "name": "general",
           "position": 0, "permission_overwrites": [], "nsfw": false})
}

fn guild() -> Value {
    let create: Value = serde_json::from_str(&payloads::guild_create(1, 3, 0, 1, 2)).unwrap();
    let mut guild = create["d"].clone();
    guild["approximate_member_count"] = json!(42);
    guild
}

/// The mock API: what Discord would answer, by method and path.
fn respond(method: &str, path: &str, body: &Value) -> (u16, Value) {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let unknown = UNKNOWN.to_string();
    let not_found = (404, json!({"message": "Unknown", "code": 10_000}));
    match (method, segments.as_slice()) {
        ("GET", ["guilds", _, "members", id] | ["users", id]) if *id == unknown => not_found,
        ("GET", ["guilds", _, "members", id]) => (200, member(id, &json!("nick"))),
        ("GET", ["guilds", _, "members"]) => (
            200,
            json!([member("8", &Value::Null), member("7", &Value::Null)]),
        ),
        ("PATCH", ["guilds", _, "members", id]) => {
            let nick = body.get("nick").cloned().unwrap_or(Value::Null);
            (200, member(id, &nick))
        }
        ("GET", ["guilds", _, "bans"]) => (
            200,
            json!([{"reason": "spam", "user": user("7")}, {"reason": null, "user": user("8")}]),
        ),
        ("GET", ["guilds", _, "bans", id]) => (200, json!({"reason": "spam", "user": user(id)})),
        ("GET", ["guilds", _, "roles"]) => (200, json!([role("5"), role("3"), role("9")])),
        ("GET", ["guilds", _, "roles", id]) => (200, role(id)),
        ("GET", ["guilds", _]) => (200, guild()),
        ("GET", ["channels", id]) => (200, channel(id)),
        ("GET", ["channels", channel, "messages"]) => (
            200,
            json!([
                message(channel, "52", &json!("newest")),
                message(channel, "51", &json!("older"))
            ]),
        ),
        ("GET", ["channels", channel, "messages", id]) => {
            (200, message(channel, id, &json!("fetched")))
        }
        ("POST", ["channels", channel, "messages"]) => {
            (200, message(channel, "99", &body["content"]))
        }
        ("PATCH", ["channels", channel, "messages", id]) => {
            (200, message(channel, id, &body["content"]))
        }
        ("POST", ["channels", channel, "messages", id, "crosspost"]) => {
            (200, message(channel, id, &json!("published")))
        }
        ("POST", ["channels", _, "messages", _, "threads"]) => (
            200,
            json!({"id": "31", "type": 11, "guild_id": GUILD.to_string(), "parent_id": "20",
                   "name": body["name"], "owner_id": "1", "message_count": 0, "member_count": 1,
                   "thread_metadata": {"archived": false, "auto_archive_duration": 1440,
                                       "archive_timestamp": "2026-11-12T10:00:00.000000+00:00",
                                       "locked": false}}),
        ),
        ("POST", ["users", "@me", "channels"]) => (
            200,
            json!({"id": "40", "type": 1, "recipients": [user("7")], "last_message_id": null}),
        ),
        ("GET", ["users", id]) => (200, user(id)),
        (
            "PUT" | "DELETE",
            ["guilds", _, "members", _, "roles", _]
            | ["guilds", _, "bans", _]
            | ["channels", _, "messages", "pins", _],
        )
        | (
            "DELETE",
            ["guilds", _, "members", _]
            | ["users", "@me", "guilds", _]
            | ["channels", _, "messages", _],
        )
        | ("PUT", ["channels", _, "messages", _, "reactions", _, "@me"])
        | ("POST", ["channels", _, "typing"] | ["channels", _, "messages", "bulk-delete"]) => {
            (204, Value::Null)
        }
        _ => (404, json!({"message": "404: Not Found", "code": 0})),
    }
}

/// Starts the mock API, logging every request.
async fn mock_rest() -> (String, Log) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api", listener.local_addr().unwrap());
    let log: Log = Arc::default();
    let state = Arc::clone(&log);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let state = Arc::clone(&state);
                    async move {
                        let method = request.method().to_string();
                        let path = request
                            .uri()
                            .path()
                            .trim_start_matches("/api/v10")
                            .to_owned();
                        let reason = request
                            .headers()
                            .get("x-audit-log-reason")
                            .map(|value| value.to_str().unwrap().to_owned());
                        let body = request.into_body().collect().await.unwrap().to_bytes();
                        let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                        let (status, response) = respond(&method, &path, &body);
                        state.lock().unwrap().push(Seen {
                            method,
                            path,
                            body,
                            reason,
                        });
                        let body = if status == 204 {
                            Bytes::new()
                        } else {
                            Bytes::from(response.to_string())
                        };
                        let response = Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(Full::new(body))
                            .unwrap();
                        Ok::<_, Infallible>(response)
                    }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    (base, log)
}

/// A gateway that identifies the client, sends an empty `READY` (these sessions never resume)
/// and waits for the close.
async fn mock_gateway() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
        let text = |payload: Value| WsMessage::text(payload.to_string());
        ws.send(text(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
            .await
            .unwrap();
        loop {
            let frame = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
            let identify = frame
                .as_text()
                .is_some_and(|text| serde_json::from_str::<Value>(text).unwrap()["op"] == 2);
            if identify {
                break;
            }
        }
        ws.send(text(json!({"t": "READY", "s": 1, "op": 0, "d": {
            "v": 10,
            "user": {"id": "1", "username": "test-bot", "discriminator": "0",
                     "global_name": null, "avatar": null, "bot": true},
            "guilds": [], "session_id": "s", "resume_gateway_url": "ws://127.0.0.1:9",
            "application": {"id": "1"}
        }})))
        .await
        .unwrap();
        while let Ok(Some(Ok(frame))) = timeout(STEP, ws.next()).await {
            if frame.is_close() {
                return;
            }
        }
    });
    (url, task)
}

/// A logged-in client, its handler context and the mock API's log.
struct Session {
    client: Client,
    ctx: Context,
    log: Log,
    running: JoinHandle<Result<(), djs_rs::Error>>,
    gateway: JoinHandle<()>,
}

impl Session {
    async fn start(cache: Option<CacheConfig>) -> Self {
        let (api, log) = mock_rest().await;
        let (url, gateway) = mock_gateway().await;
        let mut builder = Client::builder("test-token")
            .intents(Intents::GUILDS)
            .gateway_url(url)
            .shards(Shards::Count(1))
            .compression(Compression::None)
            .rest(Rest::builder().token("test-token").api(api).build());
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
        log.lock().unwrap().clear();
        Self {
            client,
            ctx,
            log,
            running,
            gateway,
        }
    }

    /// The requests made since the last call.
    fn take(&self) -> Vec<Seen> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }

    /// The single request made since the last call.
    fn one(&self) -> Seen {
        let mut seen = self.take();
        assert_eq!(seen.len(), 1, "{seen:?}");
        seen.pop().unwrap()
    }

    async fn finish(self) {
        self.client.shutdown();
        timeout(STEP, self.running).await.unwrap().unwrap().unwrap();
        self.gateway.await.unwrap();
    }
}

fn full_cache() -> CacheConfig {
    CacheConfig::builder()
        .members(Limit::Unlimited)
        .users(Limit::Unlimited)
        .messages(Limit::Max(100))
        .threads(Limit::Unlimited)
        .build()
}

#[tokio::test]
async fn fetch_reads_the_cache_first_and_force_asks_discord() {
    let session = Session::start(Some(full_cache())).await;
    let ctx = &session.ctx;
    let user = UserId::new(7);

    // Not cached: Discord is asked, and the answer is cached.
    assert!(!ctx.members(GUILD).fetch(user).is_cached());
    let member = ctx.members(GUILD).fetch(user).await.unwrap();
    assert_eq!(member.nick.as_deref(), Some("nick"));
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("GET", "/guilds/1400000000000000002/members/7")
    );
    let cached = ctx.members(GUILD).cache().unwrap().get(user).unwrap();
    assert!(Arc::ptr_eq(&cached, &member));

    // Cached: no request, the same `Arc`.
    let fetch = ctx.members(GUILD).fetch(user);
    assert!(fetch.is_cached());
    assert!(Arc::ptr_eq(&fetch.await.unwrap(), &member));
    assert!(session.take().is_empty());

    // Forced: Discord is asked again.
    ctx.members(GUILD).fetch(user).force().await.unwrap();
    assert_eq!(session.one().method, "GET");

    // The other managers work the same way.
    let role = ctx.roles(GUILD).fetch(RoleId::new(5)).await.unwrap();
    assert_eq!(role.name, "role-5");
    assert!(ctx.roles(GUILD).fetch(RoleId::new(5)).is_cached());
    let channel = ctx.channels().fetch(ChannelId::new(30)).await.unwrap();
    assert!(Arc::ptr_eq(
        &ctx.channels().fetch(ChannelId::new(30)).await.unwrap(),
        &channel
    ));
    let fetched = ctx.users().fetch(UserId::new(9)).await.unwrap();
    assert_eq!(fetched.username, "user-9");
    assert!(ctx.users().fetch(UserId::new(9)).is_cached());
    let message = ctx
        .messages(CHANNEL)
        .fetch(MessageId::new(50))
        .await
        .unwrap();
    assert_eq!(message.content, "fetched");
    assert!(ctx.messages(CHANNEL).fetch(MessageId::new(50)).is_cached());
    let guild = ctx.guilds().fetch(GUILD).await.unwrap();
    assert_eq!(guild.member_count, 42);
    assert!(ctx.guilds().fetch(GUILD).is_cached());
    let paths: Vec<String> = session.take().into_iter().map(|seen| seen.path).collect();
    assert_eq!(
        paths,
        [
            "/guilds/1400000000000000002/roles/5",
            "/channels/30",
            "/users/9",
            "/channels/20/messages/50",
            "/guilds/1400000000000000002",
        ]
    );
    session.finish().await;
}

#[tokio::test]
async fn without_a_cache_every_fetch_asks_discord() {
    let session = Session::start(None).await;
    let ctx = &session.ctx;
    assert!(ctx.members(GUILD).cache().is_none());
    for _ in 0..2 {
        let fetch = ctx.members(GUILD).fetch(UserId::new(7));
        assert!(!fetch.is_cached());
        assert_eq!(fetch.await.unwrap().user.username, "user-7");
    }
    assert_eq!(session.take().len(), 2);
    let sent = ctx.messages(CHANNEL).send("hello").await.unwrap();
    assert_eq!(sent.content, "hello");
    session.finish().await;
}

#[tokio::test]
async fn failures_are_returned_and_nothing_is_cached() {
    let session = Session::start(Some(full_cache())).await;
    let ctx = &session.ctx;
    let error = ctx
        .members(GUILD)
        .fetch(UserId::new(UNKNOWN))
        .await
        .unwrap_err();
    assert_eq!(error.status().map(|status| status.as_u16()), Some(404));
    assert!(ctx.members(GUILD).cache().unwrap().is_empty());
    assert!(ctx.users().fetch(UserId::new(UNKNOWN)).await.is_err());
    session.finish().await;
}

#[tokio::test]
async fn lists_keep_discords_order_and_are_cached() {
    let session = Session::start(Some(full_cache())).await;
    let ctx = &session.ctx;

    let roles = ctx.roles(GUILD).list().await.unwrap();
    let ids: Vec<u64> = roles.keys().map(|id| id.get()).collect();
    assert_eq!(ids, [5, 3, 9]);
    assert!(Arc::ptr_eq(
        &ctx.roles(GUILD)
            .cache()
            .unwrap()
            .get(RoleId::new(3))
            .unwrap(),
        roles.get(&RoleId::new(3)).unwrap()
    ));

    let mut query = ListGuildMembers::default();
    query.limit = Some(2);
    let members = ctx.members(GUILD).list(&query).await.unwrap();
    let ids: Vec<u64> = members.keys().map(|id| id.get()).collect();
    assert_eq!(ids, [8, 7]);
    assert_eq!(ctx.members(GUILD).cache().unwrap().len(), 2);

    let messages = ctx
        .messages(CHANNEL)
        .list(&GetChannelMessages::default())
        .await
        .unwrap();
    let contents: Vec<&str> = messages
        .values()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(contents, ["newest", "older"]);
    assert!(ctx.messages(CHANNEL).fetch(MessageId::new(51)).is_cached());

    // Bans are not cached: the model type.
    let bans = ctx
        .bans(GUILD)
        .list(&GetGuildBans::default())
        .await
        .unwrap();
    assert_eq!(
        bans.get(&UserId::new(7)).unwrap().reason.as_deref(),
        Some("spam")
    );
    assert!(bans.get(&UserId::new(8)).unwrap().reason.is_none());
    let ban = ctx.bans(GUILD).fetch(UserId::new(7));
    assert!(!ban.is_cached());
    assert_eq!(ban.await.unwrap().user.id, UserId::new(7));

    let seen: Vec<String> = session
        .take()
        .into_iter()
        .map(|seen| format!("{} {}", seen.method, seen.path))
        .collect();
    assert_eq!(
        seen,
        [
            "GET /guilds/1400000000000000002/roles",
            "GET /guilds/1400000000000000002/members",
            "GET /channels/20/messages",
            "GET /guilds/1400000000000000002/bans",
            "GET /guilds/1400000000000000002/bans/7",
        ]
    );
    session.finish().await;
}

#[tokio::test]
async fn member_actions_send_the_documented_requests() {
    let session = Session::start(Some(full_cache())).await;
    let ctx = &session.ctx;
    let members = ctx.members(GUILD);
    let user = UserId::new(7);
    let base = "/guilds/1400000000000000002";

    members.kick(user, Some("spam")).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (
            seen.method.as_str(),
            seen.path.as_str(),
            seen.reason.as_deref()
        ),
        ("DELETE", &*format!("{base}/members/7"), Some("spam"))
    );

    let mut ban = CreateGuildBan::default();
    ban.delete_message_seconds = Some(3600);
    members.ban(user, &ban, None).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("PUT", &*format!("{base}/bans/7"))
    );
    assert_eq!(seen.body, json!({"delete_message_seconds": 3600}));
    assert_eq!(seen.reason, None);

    members.unban(user, Some("appeal")).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.reason.as_deref()),
        ("DELETE", Some("appeal"))
    );

    let until = Timestamp::parse("2027-01-15T09:00:00.000000+00:00").unwrap();
    members.timeout(user, Some(until), None).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("PATCH", &*format!("{base}/members/7"))
    );
    assert_eq!(
        seen.body,
        json!({"communication_disabled_until": "2027-01-15T09:00:00.000000+00:00"})
    );
    members.timeout(user, None, None).await.unwrap();
    assert_eq!(
        session.one().body,
        json!({"communication_disabled_until": null})
    );

    let renamed = members
        .set_nickname(user, Some("Ferris"), None)
        .await
        .unwrap();
    assert_eq!(renamed.nick.as_deref(), Some("Ferris"));
    assert_eq!(session.one().body, json!({"nick": "Ferris"}));
    // The edited member replaced the cached one.
    assert_eq!(
        members.cache().unwrap().get(user).unwrap().nick.as_deref(),
        Some("Ferris")
    );
    let cleared = members.set_nickname(user, None, None).await.unwrap();
    assert_eq!(cleared.nick, None);
    assert_eq!(session.one().body, json!({"nick": null}));

    members
        .add_role(user, RoleId::new(5), Some("verified"))
        .await
        .unwrap();
    let seen = session.one();
    assert_eq!(
        (
            seen.method.as_str(),
            seen.path.as_str(),
            seen.reason.as_deref()
        ),
        (
            "PUT",
            &*format!("{base}/members/7/roles/5"),
            Some("verified")
        )
    );
    members
        .remove_role(user, RoleId::new(5), None)
        .await
        .unwrap();
    assert_eq!(session.one().method, "DELETE");

    // The same actions on structures: a cached member and a join payload.
    let member = members.fetch(user).await.unwrap();
    member.kick(ctx, Some("again")).await.unwrap();
    let seen = session.one();
    assert_eq!(seen.path, format!("{base}/members/7"));
    let join: GuildMemberAdd = serde_json::from_value({
        let mut join = member_json(8);
        join["guild_id"] = json!(GUILD.to_string());
        join
    })
    .unwrap();
    join.add_role(ctx, RoleId::new(6), None).await.unwrap();
    assert_eq!(session.one().path, format!("{base}/members/8/roles/6"));
    session.finish().await;
}

fn member_json(id: u64) -> Value {
    member(&id.to_string(), &Value::Null)
}

#[tokio::test]
async fn message_actions_send_the_documented_requests() {
    let session = Session::start(Some(full_cache())).await;
    let ctx = &session.ctx;
    let messages = ctx.messages(CHANNEL);

    // A reply from a cached message references it and is cached itself.
    let original = messages.fetch(MessageId::new(50)).await.unwrap();
    session.take();
    let reply = original.reply(ctx, "pong").await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("POST", "/channels/20/messages")
    );
    assert_eq!(seen.body["content"], "pong");
    assert_eq!(seen.body["message_reference"]["message_id"], "50");
    assert!(Arc::ptr_eq(
        &messages.cache().unwrap().get(reply.id).unwrap(),
        &reply
    ));
    // …and from a model message.
    let model: Message = serde_json::from_value(message("20", "60", &json!("hi"))).unwrap();
    model.reply(ctx, "hello").await.unwrap();
    assert_eq!(session.one().body["message_reference"]["message_id"], "60");

    reply.react(ctx, "🔥").await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("PUT", "/channels/20/messages/99/reactions/%F0%9F%94%A5/@me")
    );

    reply.pin(ctx, Some("important")).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (
            seen.method.as_str(),
            seen.path.as_str(),
            seen.reason.as_deref()
        ),
        ("PUT", "/channels/20/messages/pins/99", Some("important"))
    );
    reply.unpin(ctx, None).await.unwrap();
    assert_eq!(session.one().method, "DELETE");

    let thread = reply
        .start_thread(
            ctx,
            &djs_rs::model::rest::StartThreadFromMessage::new("talk"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(thread.name(), Some("talk"));
    assert_eq!(session.one().path, "/channels/20/messages/99/threads");
    assert!(
        ctx.cache()
            .unwrap()
            .threads(GUILD)
            .get(ChannelId::new(31))
            .is_some()
    );

    let published = reply.crosspost(ctx).await.unwrap();
    assert_eq!(published.content, "published");
    assert_eq!(session.one().path, "/channels/20/messages/99/crosspost");

    reply.delete(ctx).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("DELETE", "/channels/20/messages/99")
    );

    messages
        .bulk_delete(
            vec![MessageId::new(51), MessageId::new(52)],
            Some("cleanup"),
        )
        .await
        .unwrap();
    let seen = session.one();
    assert_eq!(seen.path, "/channels/20/messages/bulk-delete");
    assert_eq!(seen.body, json!({"messages": ["51", "52"]}));
    assert_eq!(seen.reason.as_deref(), Some("cleanup"));

    // Channels.
    let channel = ctx.channels().fetch(CHANNEL).await.unwrap();
    session.take();
    channel.send_typing(ctx).await.unwrap();
    let seen = session.one();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("POST", "/channels/20/typing")
    );
    let sent = channel.send(ctx, "to the channel").await.unwrap();
    assert_eq!(sent.content, "to the channel");
    assert!(session.one().body.get("message_reference").is_none());
    session.finish().await;
}

#[tokio::test]
async fn users_and_guilds() {
    let session = Session::start(Some(full_cache())).await;
    let ctx = &session.ctx;

    // A DM: the channel is opened, then the message sent to it.
    let user = ctx.users().fetch(UserId::new(7)).await.unwrap();
    session.take();
    let sent = user.send(ctx, "psst").await.unwrap();
    assert_eq!(sent.channel_id, ChannelId::new(40));
    let seen: Vec<(String, String, Value)> = session
        .take()
        .into_iter()
        .map(|seen| (seen.method, seen.path, seen.body))
        .collect();
    assert_eq!(
        seen,
        [
            (
                "POST".into(),
                "/users/@me/channels".into(),
                json!({"recipient_id": "7"})
            ),
            (
                "POST".into(),
                "/channels/40/messages".into(),
                json!({"content": "psst"})
            ),
        ]
    );

    let guild = ctx.guilds().fetch(GUILD).await.unwrap();
    session.take();
    let roles = guild.roles(ctx).list().await.unwrap();
    assert_eq!(roles.len(), 3);
    assert_eq!(guild.members(ctx).guild_id(), GUILD);
    assert_eq!(guild.bans(ctx).guild_id(), GUILD);
    guild.leave(ctx).await.unwrap();
    let seen = session.take().pop().unwrap();
    assert_eq!(
        (seen.method.as_str(), seen.path.as_str()),
        ("DELETE", "/users/@me/guilds/1400000000000000002")
    );
    session.finish().await;
}
