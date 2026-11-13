//! Collectors (D-263–D-269) against a scripted mock gateway: each test registers a
//! collector, then sends the events it should (and should not) collect, and checks what
//! comes out and why the collector ended.

use std::time::Duration;

use djs_rs::{
    Shards,
    collector::{EndReason, ReactionEvent},
    gateway::Compression,
    model::{
        component::ComponentType,
        id::{ChannelId, GuildId, MessageId, UserId},
        interaction::Interaction,
        message::Message,
    },
    prelude::*,
    rest::Rest,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    sync::mpsc,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_websockets::{Message as WsMessage, ServerBuilder};

const STEP: Duration = Duration::from_secs(5);
const GUILD: u64 = 10;
const CHANNEL: u64 = 20;
/// The message the collectors watch.
const MESSAGE: u64 = 30;

fn user(id: u64) -> Value {
    json!({"id": id.to_string(), "username": format!("user-{id}"), "discriminator": "0",
           "global_name": null, "avatar": null})
}

fn member(id: u64) -> Value {
    json!({"user": user(id), "roles": [], "joined_at": "2026-01-01T00:00:00.000000+00:00",
           "deaf": false, "mute": false, "flags": 0, "pending": false})
}

fn message_json(channel: u64, id: u64, author: u64, content: &str) -> Value {
    json!({"id": id.to_string(), "channel_id": channel.to_string(), "author": user(author),
           "content": content, "timestamp": "2026-11-10T19:42:07.123000+00:00",
           "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
           "type": 0, "guild_id": GUILD.to_string()})
}

fn message_create(channel: u64, id: u64, author: u64, content: &str) -> (&'static str, Value) {
    ("MESSAGE_CREATE", message_json(channel, id, author, content))
}

/// A button (`component_type` 2) or select menu (3) click by the user `by` on `message`, which
/// optionally answers the interaction `answers`.
fn click(
    id: u64,
    by: u64,
    message: u64,
    custom_id: &str,
    component_type: u8,
    answers: Option<u64>,
) -> (&'static str, Value) {
    let mut on = message_json(CHANNEL, message, 1, "menu");
    if let Some(original) = answers {
        on["interaction_metadata"] = json!({"id": original.to_string(), "type": 2,
            "user": user(7), "authorizing_integration_owners": {}});
    }
    (
        "INTERACTION_CREATE",
        json!({
            "id": id.to_string(), "application_id": "1", "type": 3, "token": format!("token-{id}"),
            "version": 1, "guild_id": GUILD.to_string(), "channel_id": CHANNEL.to_string(),
            "app_permissions": "0", "member": member(by), "message": on,
            "data": {"custom_id": custom_id, "component_type": component_type}
        }),
    )
}

/// A slash command run by `user`, as an [`Interaction`] a handler would hold.
fn command(id: u64, user: u64) -> Interaction {
    serde_json::from_value(json!({
        "id": id.to_string(), "application_id": "1", "type": 2, "token": format!("token-{id}"),
        "version": 1, "guild_id": GUILD.to_string(), "channel_id": CHANNEL.to_string(),
        "app_permissions": "0", "member": member(user),
        "data": {"id": "91", "name": "ping", "type": 1}
    }))
    .unwrap()
}

fn modal_submit(id: u64, user: u64, custom_id: &str) -> (&'static str, Value) {
    (
        "INTERACTION_CREATE",
        json!({
            "id": id.to_string(), "application_id": "1", "type": 5, "token": format!("token-{id}"),
            "version": 1, "guild_id": GUILD.to_string(), "channel_id": CHANNEL.to_string(),
            "app_permissions": "0", "member": member(user),
            "data": {"custom_id": custom_id, "components": []}
        }),
    )
}

fn reaction(event: &'static str, user: u64, emoji: &str) -> (&'static str, Value) {
    (
        event,
        json!({
            "user_id": user.to_string(), "channel_id": CHANNEL.to_string(),
            "message_id": MESSAGE.to_string(), "guild_id": GUILD.to_string(),
            "emoji": {"id": null, "name": emoji}, "burst": false, "type": 0
        }),
    )
}

fn the_message() -> Message {
    serde_json::from_value(message_json(CHANNEL, MESSAGE, 1, "watched")).unwrap()
}

type Script = mpsc::UnboundedSender<(&'static str, Value)>;

/// A gateway that identifies the client, sends `READY`, then forwards what the test sends;
/// the client's member and soundboard requests (opcodes 8 and 31) go to `requests`.
async fn scripted_gateway() -> (
    String,
    Script,
    mpsc::UnboundedReceiver<Value>,
    JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (tx, mut rx) = mpsc::unbounded_channel::<(&'static str, Value)>();
    let (requests_tx, requests) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
        let text = |payload: Value| WsMessage::text(payload.to_string());
        ws.send(text(json!({"op": 10, "d": {"heartbeat_interval": 45_000}})))
            .await
            .unwrap();
        loop {
            let frame = timeout(STEP, ws.next()).await.unwrap().unwrap().unwrap();
            if frame
                .as_text()
                .is_some_and(|text| serde_json::from_str::<Value>(text).unwrap()["op"] == 2)
            {
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
        let mut sequence = 2;
        loop {
            tokio::select! {
                dispatch = rx.recv() => {
                    let Some((event, data)) = dispatch else { return };
                    ws.send(text(json!({"t": event, "s": sequence, "op": 0, "d": data})))
                        .await
                        .unwrap();
                    sequence += 1;
                }
                frame = ws.next() => match frame {
                    Some(Ok(frame)) if !frame.is_close() => {
                        let command = frame
                            .as_text()
                            .and_then(|text| serde_json::from_str::<Value>(text).ok());
                        if let Some(command) = command
                            && (command["op"] == 8 || command["op"] == 31)
                        {
                            let _ = requests_tx.send(command["d"].clone());
                        }
                    }
                    _ => return,
                },
            }
        }
    });
    (url, tx, requests, task)
}

/// A logged-in client, its handler context and the gateway's script.
struct Session {
    client: Client,
    ctx: Context,
    gateway: Script,
    requests: mpsc::UnboundedReceiver<Value>,
    running: JoinHandle<Result<(), djs_rs::Error>>,
}

impl Session {
    async fn start() -> Self {
        Self::start_with(|client| client).await
    }

    async fn start_with(configure: impl FnOnce(Client) -> Client) -> Self {
        let (url, gateway, requests, _task) = scripted_gateway().await;
        let client = Client::builder("test-token")
            .intents(Intents::GUILDS)
            .gateway_url(url)
            .shards(Shards::Count(1))
            .compression(Compression::None)
            // No request reaches it: collectors only read the gateway.
            .rest(Rest::builder().token("test-token").api("http://127.0.0.1:9/api").build())
            .build();
        let client = configure(client);
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
            gateway,
            requests,
            running,
        }
    }

    /// The next member or soundboard request the client sent.
    async fn request(&mut self) -> Value {
        timeout(STEP, self.requests.recv()).await.unwrap().unwrap()
    }

    fn send(&self, dispatch: (&'static str, Value)) {
        self.gateway.send(dispatch).unwrap();
    }

    async fn finish(self) {
        self.client.shutdown();
        timeout(STEP, self.running).await.unwrap().unwrap().unwrap();
    }
}

async fn next<S: futures_util::Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    timeout(STEP, stream.next())
        .await
        .expect("the collector stalled")
}

#[tokio::test]
async fn a_message_collector_filters_and_stops_at_its_limit() {
    let session = Session::start().await;
    let ctx = &session.ctx;
    let mut messages = ctx
        .message_collector(ChannelId::new(CHANNEL))
        .filter(|message| message.author.id == UserId::new(5))
        .max(2)
        .build();

    session.send(message_create(CHANNEL, 100, 6, "not from 5"));
    session.send(message_create(99, 101, 5, "another channel"));
    session.send(message_create(CHANNEL, 102, 5, "first"));
    session.send(message_create(CHANNEL, 103, 5, "second"));
    session.send(message_create(CHANNEL, 104, 5, "after the limit"));

    let first = next(&mut messages).await.unwrap();
    let second = next(&mut messages).await.unwrap();
    assert_eq!(
        (first.content.as_str(), second.content.as_str()),
        ("first", "second")
    );
    assert_eq!(next(&mut messages).await.map(|m| m.id), None);
    assert_eq!(messages.end_reason(), Some(EndReason::Limit));
    // The other channel's message never reached it.
    assert_eq!((messages.collected(), messages.processed()), (2, 3));
    session.finish().await;
}

#[tokio::test]
async fn await_messages_returns_what_arrived_in_time() {
    let session = Session::start().await;
    let ctx = session.ctx.clone();
    let waiting = tokio::spawn(async move {
        ctx.await_messages(ChannelId::new(CHANNEL))
            .time(Duration::from_millis(400))
            .await
    });
    sleep(Duration::from_millis(50)).await;
    session.send(message_create(CHANNEL, 200, 5, "a"));
    session.send(message_create(CHANNEL, 201, 6, "b"));
    let collected = timeout(STEP, waiting).await.unwrap().unwrap();
    assert_eq!(collected.reason(), EndReason::Time);
    assert_eq!(
        collected.keys().copied().collect::<Vec<_>>(),
        [MessageId::new(200), MessageId::new(201)]
    );

    // Nothing arriving is not an error.
    let empty = session
        .ctx
        .await_messages(ChannelId::new(CHANNEL))
        .time(Duration::from_millis(50))
        .await;
    assert!(empty.is_empty());
    assert_eq!(empty.reason(), EndReason::Time);
    session.finish().await;
}

#[tokio::test]
async fn collected_messages_still_reach_the_handlers() {
    let (tx, mut seen) = mpsc::unbounded_channel();
    let session = Session::start_with(move |client| {
        client.on(Events::MessageCreate, move |_ctx, message| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(message.id);
                Ok(())
            }
        });
        client
    })
    .await;
    let mut messages = session
        .ctx
        .message_collector(ChannelId::new(CHANNEL))
        .build();
    session.send(message_create(CHANNEL, 300, 5, "both"));
    assert_eq!(next(&mut messages).await.unwrap().id, MessageId::new(300));
    let handled = timeout(STEP, seen.recv()).await.unwrap().unwrap();
    assert_eq!(handled, MessageId::new(300));
    session.finish().await;
}

#[tokio::test]
async fn component_collectors_by_message_channel_and_interaction() {
    let session = Session::start().await;
    let ctx = &session.ctx;
    let message = the_message();
    let mut on_message = message
        .component_collector(ctx)
        .component_type(ComponentType::Button)
        .max_users(2)
        .build();
    let mut in_channel = ctx.component_collector(ChannelId::new(CHANNEL)).build();
    let mut on_reply = command(90, 7).component_collector(ctx).build();

    // A select menu: not a button, so ignored by `on_message`.
    session.send(click(400, 5, MESSAGE, "pick", 3, None));
    session.send(click(401, 5, MESSAGE, "yes", 2, None));
    session.send(click(402, 5, MESSAGE, "again", 2, None));
    session.send(click(403, 6, MESSAGE, "no", 2, None));
    // On the reply to the command 90: another message.
    session.send(click(404, 6, 31, "reply-button", 2, Some(90)));

    let ids: Vec<String> = [
        next(&mut on_message).await.unwrap(),
        next(&mut on_message).await.unwrap(),
        next(&mut on_message).await.unwrap(),
    ]
    .iter()
    .map(|click| click.custom_id.to_string())
    .collect();
    assert_eq!(ids, ["yes", "again", "no"]);
    assert!(next(&mut on_message).await.is_none());
    assert_eq!(on_message.end_reason(), Some(EndReason::UserLimit));

    let mut channel_ids = Vec::new();
    for _ in 0..5 {
        channel_ids.push(next(&mut in_channel).await.unwrap().custom_id.to_string());
    }
    assert_eq!(channel_ids, ["pick", "yes", "again", "no", "reply-button"]);

    let reply_click = next(&mut on_reply).await.unwrap();
    assert_eq!(reply_click.custom_id, "reply-button");
    on_reply.stop();
    assert!(next(&mut on_reply).await.is_none());
    assert_eq!(on_reply.end_reason(), Some(EndReason::Stopped));

    // A click's own collector watches the clicked message (its response edits it).
    let mut after_click = reply_click.component_collector(ctx).max(1).build();
    session.send(click(405, 6, 31, "second-press", 2, Some(90)));
    assert_eq!(
        next(&mut after_click).await.unwrap().custom_id,
        "second-press"
    );
    session.finish().await;
}

#[tokio::test]
async fn await_component_returns_the_click_or_the_reason() {
    let session = Session::start().await;
    let ctx = session.ctx.clone();
    let waiting = tokio::spawn(async move {
        the_message()
            .await_component(&ctx)
            .filter(|click| click.custom_id == "confirm")
            .time(Duration::from_secs(3))
            .await
    });
    sleep(Duration::from_millis(50)).await;
    session.send(click(500, 5, MESSAGE, "cancel", 2, None));
    session.send(click(501, 5, MESSAGE, "confirm", 2, None));
    let confirmed = timeout(STEP, waiting).await.unwrap().unwrap().unwrap();
    assert_eq!(confirmed.interaction().id.get(), 501);

    let error = the_message()
        .await_component(&session.ctx)
        .time(Duration::from_millis(50))
        .await
        .unwrap_err();
    assert_eq!(error.reason(), EndReason::Time);
    assert_eq!(error.to_string(), "nothing was collected: the time elapsed");
    session.finish().await;
}

#[tokio::test]
async fn await_modal_submit_waits_for_the_users_modal() {
    let session = Session::start().await;
    let ctx = session.ctx.clone();
    let waiting = tokio::spawn(async move {
        command(90, 7)
            .await_modal_submit(&ctx, Duration::from_secs(3))
            .filter(|modal| modal.custom_id == "feedback")
            .await
    });
    sleep(Duration::from_millis(50)).await;
    // Another user's modal, then another modal of the user, then the one awaited.
    session.send(modal_submit(600, 8, "feedback"));
    session.send(modal_submit(601, 7, "survey"));
    session.send(modal_submit(602, 7, "feedback"));
    let submitted = timeout(STEP, waiting).await.unwrap().unwrap().unwrap();
    assert_eq!(submitted.interaction().id.get(), 602);

    let error = command(91, 7)
        .await_modal_submit(&session.ctx, Duration::from_millis(50))
        .await
        .unwrap_err();
    assert_eq!(error.reason(), EndReason::Time);
    session.finish().await;
}

#[tokio::test]
async fn reaction_collectors_count_removals_with_dispose() {
    let session = Session::start().await;
    let ctx = &session.ctx;
    let mut reactions = the_message()
        .reaction_collector(ctx)
        .dispose(true)
        .max(2)
        .build();

    session.send(reaction("MESSAGE_REACTION_ADD", 5, "👍"));
    // Not collected before, so its removal is not yielded.
    session.send(reaction("MESSAGE_REACTION_REMOVE", 6, "👍"));
    session.send(reaction("MESSAGE_REACTION_REMOVE", 5, "👍"));
    session.send(reaction("MESSAGE_REACTION_ADD", 6, "🔥"));
    session.send(reaction("MESSAGE_REACTION_ADD", 7, "🔥"));

    let mut events = Vec::new();
    while let Some(event) = next(&mut reactions).await {
        events.push(match event {
            ReactionEvent::Added(added) => format!("+{} {:?}", added.user_id, added.emoji.name()),
            ReactionEvent::Removed(removed) => {
                format!("-{} {:?}", removed.user_id, removed.emoji.name())
            }
            _ => "cleared".to_owned(),
        });
    }
    // The removal gave its place back: the limit is two reactions at once.
    assert_eq!(
        events,
        [
            "+5 Some(\"👍\")",
            "-5 Some(\"👍\")",
            "+6 Some(\"🔥\")",
            "+7 Some(\"🔥\")"
        ]
    );
    assert_eq!(reactions.end_reason(), Some(EndReason::Limit));
    session.finish().await;
}

#[tokio::test]
async fn await_reactions_gathers_by_emoji_until_the_message_is_deleted() {
    let session = Session::start().await;
    let ctx = session.ctx.clone();
    let waiting = tokio::spawn(async move { the_message().await_reactions(&ctx).await });
    sleep(Duration::from_millis(50)).await;
    session.send(reaction("MESSAGE_REACTION_ADD", 5, "👍"));
    session.send(reaction("MESSAGE_REACTION_ADD", 6, "👍"));
    session.send(reaction("MESSAGE_REACTION_ADD", 6, "🔥"));
    session.send((
        "MESSAGE_DELETE",
        json!({"id": MESSAGE.to_string(),
        "channel_id": CHANNEL.to_string(), "guild_id": GUILD.to_string()}),
    ));
    let votes = timeout(STEP, waiting).await.unwrap().unwrap();
    assert_eq!(votes.reason(), EndReason::MessageDelete);
    assert_eq!(votes["👍"].users, [UserId::new(5), UserId::new(6)]);
    assert_eq!(votes["🔥"].count(), 1);
    session.finish().await;
}

#[tokio::test]
async fn deletions_end_the_collectors_living_there() {
    let session = Session::start().await;
    let ctx = &session.ctx;
    let mut messages = ctx.message_collector(ChannelId::new(CHANNEL)).build();
    let mut components = the_message().component_collector(ctx).build();
    let mut elsewhere = ctx.message_collector(ChannelId::new(99)).build();

    session.send((
        "CHANNEL_DELETE",
        json!({"id": CHANNEL.to_string(), "type": 0,
        "guild_id": GUILD.to_string(), "name": "general", "position": 0,
        "permission_overwrites": [], "nsfw": false}),
    ));
    assert!(next(&mut messages).await.is_none());
    assert_eq!(messages.end_reason(), Some(EndReason::ChannelDelete));
    assert!(next(&mut components).await.is_none());
    assert_eq!(components.end_reason(), Some(EndReason::ChannelDelete));

    let mut in_guild = the_message().reaction_collector(ctx).build();
    session.send(("GUILD_DELETE", json!({"id": GUILD.to_string()})));
    assert!(next(&mut in_guild).await.is_none());
    assert_eq!(in_guild.end_reason(), Some(EndReason::GuildDelete));
    assert_eq!(elsewhere.end_reason(), None);

    // The client's shutdown ends the rest.
    let Session {
        client, running, ..
    } = session;
    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    assert!(next(&mut elsewhere).await.is_none());
    assert_eq!(elsewhere.end_reason(), Some(EndReason::Shutdown));
}

#[tokio::test]
async fn idle_time_and_reset_timer() {
    let session = Session::start().await;
    let mut messages = session
        .ctx
        .message_collector(ChannelId::new(CHANNEL))
        .idle(Duration::from_millis(300))
        .build();
    session.send(message_create(CHANNEL, 700, 5, "keeps it alive"));
    assert_eq!(next(&mut messages).await.unwrap().id, MessageId::new(700));
    sleep(Duration::from_millis(150)).await;
    messages.reset_timer(None, Some(Duration::from_millis(100)));
    assert!(next(&mut messages).await.is_none());
    assert_eq!(messages.end_reason(), Some(EndReason::Idle));
    session.finish().await;
}

#[tokio::test]
async fn dropping_a_collector_unregisters_it() {
    let session = Session::start().await;
    let ctx = &session.ctx;
    let first = ctx.message_collector(ChannelId::new(CHANNEL)).build();
    drop(first);
    // A second collector on the same channel works on its own.
    let mut second = ctx
        .message_collector(ChannelId::new(CHANNEL))
        .max(1)
        .build();
    session.send(message_create(CHANNEL, 800, 5, "only one listener"));
    assert_eq!(next(&mut second).await.unwrap().id, MessageId::new(800));
    assert_eq!(next(&mut second).await.map(|m| m.id), None);
    session.finish().await;
}

fn chunk(nonce: &Value, index: u32, count: u32, members: &[u64]) -> (&'static str, Value) {
    let members: Vec<Value> = members.iter().map(|&id| member(id)).collect();
    (
        "GUILD_MEMBERS_CHUNK",
        json!({
            "guild_id": GUILD.to_string(), "members": members, "chunk_index": index,
            "chunk_count": count, "nonce": nonce
        }),
    )
}

#[tokio::test]
async fn fetch_all_gathers_every_chunk_of_its_nonce() {
    let mut session = Session::start().await;
    let ctx = session.ctx.clone();
    let fetching = tokio::spawn(async move {
        ctx.members(GuildId::new(GUILD))
            .fetch_all()
            .presences(true)
            .await
    });
    let request = session.request().await;
    assert_eq!(
        (
            &request["guild_id"],
            &request["query"],
            &request["limit"],
            &request["presences"]
        ),
        (
            &json!(GUILD.to_string()),
            &json!(""),
            &json!(0),
            &json!(true)
        )
    );
    let nonce = request["nonce"].clone();
    assert!(nonce.as_str().unwrap().starts_with("djs-rs:"), "{nonce}");
    // Another request's chunk is not part of the answer.
    session.send(chunk(&json!("someone-else"), 0, 1, &[99]));
    session.send(chunk(&nonce, 0, 2, &[5, 6]));
    session.send(chunk(&nonce, 1, 2, &[7]));
    let members = timeout(STEP, fetching).await.unwrap().unwrap().unwrap();
    assert_eq!(
        members.keys().copied().collect::<Vec<_>>(),
        [UserId::new(5), UserId::new(6), UserId::new(7)]
    );
    assert_eq!(members[&UserId::new(6)].user.username, "user-6");
    session.finish().await;
}

#[tokio::test]
async fn fetch_many_asks_in_batches_of_100() {
    let mut session = Session::start().await;
    let ctx = session.ctx.clone();
    let fetching = tokio::spawn(async move {
        ctx.members(GuildId::new(GUILD))
            .fetch_many((1..=150).map(UserId::new))
            .await
    });
    for (expected, found) in [(100, [3, 4]), (50, [120, 150])] {
        let request = session.request().await;
        assert_eq!(request["user_ids"].as_array().unwrap().len(), expected);
        assert_eq!(request.get("query"), None);
        session.send(chunk(&request["nonce"], 0, 1, &found));
    }
    let members = timeout(STEP, fetching).await.unwrap().unwrap().unwrap();
    assert_eq!(members.len(), 4);
    session.finish().await;
}

#[tokio::test]
async fn member_requests_time_out_or_report_the_rate_limit() {
    use djs_rs::manager::GatewayRequestError;

    let mut session = Session::start().await;
    let ctx = session.ctx.clone();
    let error = ctx
        .members(GuildId::new(GUILD))
        .fetch_by_query("ab")
        .limit(5)
        .time(Duration::from_millis(100))
        .await
        .unwrap_err();
    assert!(matches!(error, GatewayRequestError::Timeout), "{error:?}");
    let request = session.request().await;
    assert_eq!(
        (&request["query"], &request["limit"]),
        (&json!("ab"), &json!(5))
    );

    let limited =
        tokio::spawn(async move { ctx.members(GuildId::new(GUILD)).fetch_by_query("cd").await });
    let request = session.request().await;
    session.send((
        "RATE_LIMITED",
        json!({"opcode": 8, "retry_after": 2.5,
        "meta": {"guild_id": GUILD.to_string(), "nonce": request["nonce"]}}),
    ));
    let error = timeout(STEP, limited).await.unwrap().unwrap().unwrap_err();
    assert!(
        matches!(error, GatewayRequestError::RateLimited { retry_after } if retry_after == Duration::from_millis(2500)),
        "{error:?}"
    );
    session.finish().await;
}

#[tokio::test]
async fn soundboard_sounds_of_several_guilds() {
    let mut session = Session::start().await;
    let ctx = session.ctx.clone();
    let fetching = tokio::spawn(async move {
        ctx.guilds()
            .fetch_soundboard_sounds([GuildId::new(GUILD), GuildId::new(11)], STEP)
            .await
    });
    let request = session.request().await;
    assert_eq!(request["guild_ids"], json!([GUILD.to_string(), "11"]));
    let sound = |id: u64, guild: u64| {
        json!({"name": format!("sound-{id}"),
        "sound_id": id.to_string(), "volume": 1.0, "emoji_id": null, "emoji_name": null,
        "guild_id": guild.to_string(), "available": true})
    };
    session.send((
        "SOUNDBOARD_SOUNDS",
        json!({"guild_id": "11", "soundboard_sounds": []}),
    ));
    session.send((
        "SOUNDBOARD_SOUNDS",
        json!({"guild_id": GUILD.to_string(),
        "soundboard_sounds": [sound(40, GUILD), sound(41, GUILD)]}),
    ));
    let sounds = timeout(STEP, fetching).await.unwrap().unwrap().unwrap();
    assert_eq!(sounds.len(), 2);
    assert!(sounds[&GuildId::new(11)].is_empty());
    assert_eq!(sounds[&GuildId::new(GUILD)].len(), 2);
    session.finish().await;
}

#[tokio::test]
async fn required_intents_cover_handlers_routes_and_collectors() {
    let reactions = Intents::GUILD_MESSAGE_REACTIONS | Intents::DIRECT_MESSAGE_REACTIONS;
    let session = Session::start_with(|client| {
        client.on(Events::MessageReactionAdd, |_ctx, _reaction| async move {
            Ok(())
        });
        // Interactions need no intent.
        client.command("ping", |_ctx, _command| async move { Ok(()) });
        client
    })
    .await;
    assert_eq!(session.client.required_intents(), reactions);

    let messages = session
        .ctx
        .message_collector(ChannelId::new(CHANNEL))
        .build();
    let with_collector = session.client.required_intents();
    assert!(
        with_collector.contains(reactions | Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES),
        "{with_collector:?}"
    );
    drop(messages);
    assert_eq!(session.client.required_intents(), reactions);
    session.finish().await;
}
