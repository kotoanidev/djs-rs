//! A larger bot's setup: automatic sharding, a presence that changes, an `events()` stream
//! next to the handlers, sequential handlers, a `WebhookClient`, and mimalloc as the global
//! allocator.
//!
//! | Feature | Shows |
//! |---|---|
//! | `Shards::Auto` | as many shards as Discord recommends, `ShardReady` for each |
//! | presence | `ClientBuilder::presence` at login, `Client::set_presence` every minute |
//! | `client.events()` | one stream of every event, counted by name |
//! | `HandlerMode::Sequential` | handlers in event order; slow work moved to tasks |
//! | `!stats` | what the event stream counted |
//! | `!webhook` | a temporary webhook used through `WebhookClient`: send, edit, delete |
//! | mimalloc | the global allocator recommended for busy bots (README, "Performance") |
//!
//! With `DISCORD_WEBHOOK_URL` set, the example also announces its start through that webhook
//! before logging in: a `WebhookClient` needs no bot token and no gateway connection.
//!
//! # Running
//!
//! 1. Enable the **Message Content** privileged intent in the
//!    [Developer Portal](https://discord.com/developers/applications); `!webhook` needs the
//!    Manage Webhooks permission in the channel.
//! 2. Put `DISCORD_TOKEN` in `.env` (see `.env.example`).
//! 3. Run `cargo run --example advanced` (`--release` for the allocator to matter), then send
//!    `!stats` or `!webhook`.
//!
//! discord.js counterpart: the `shards`, `presence` and `WebhookClient` options and classes;
//! discord.js has no ordered handler mode or event stream.

use std::{
    collections::HashMap,
    future::Future,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use djs_rs::{
    HandlerMode, Shards, WebhookClient,
    model::{
        gateway::{Activity, Status, payload::UpdatePresence},
        id::ChannelId,
        rest::{CreateWebhook, EditWebhookMessage, ExecuteWebhook},
    },
    prelude::*,
};

#[path = "common/mod.rs"]
mod common;

// Faster than the system allocator when events are decoded on one thread and dropped on
// others (`docs/PERFORMANCE.md`); any bot can declare it.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// What the event stream counted.
struct Stats {
    started: Instant,
    total: AtomicU64,
    by_name: Mutex<HashMap<String, u64>>,
}

impl Stats {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            total: AtomicU64::new(0),
            by_name: Mutex::new(HashMap::new()),
        }
    }

    fn record(&self, name: &str) {
        self.total.fetch_add(1, Ordering::Relaxed);
        let mut by_name = self.by_name.lock().unwrap_or_else(PoisonError::into_inner);
        match by_name.get_mut(name) {
            Some(count) => *count += 1,
            None => {
                by_name.insert(name.to_owned(), 1);
            }
        }
    }

    fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    /// The `n` most frequent events, most frequent first.
    fn top(&self, n: usize) -> Vec<(String, u64)> {
        let by_name = self.by_name.lock().unwrap_or_else(PoisonError::into_inner);
        let mut top: Vec<_> = by_name
            .iter()
            .map(|(name, count)| (name.clone(), *count))
            .collect();
        top.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top.truncate(n);
        top
    }
}

/// Runs `work` in a task of its own. In `Sequential` mode a handler holds back the later
/// events of its shard until it returns, so anything slow — a REST call — is moved out of it.
fn spawn_logged(work: impl Future<Output = Result<(), HandlerError>> + Send + 'static) {
    tokio::spawn(async move {
        if let Err(error) = work.await {
            eprintln!("A task failed: {error}");
        }
    });
}

/// `!webhook`: a webhook created for the demonstration, used through a `WebhookClient` and
/// deleted again. A real `WebhookClient` would be built from a webhook URL kept in its
/// configuration (`WebhookClient::from_url`).
async fn webhook_demo(ctx: Context, channel_id: ChannelId) -> Result<(), HandlerError> {
    const REASON: Option<&str> = Some("djs-rs advanced example");
    let webhook = ctx
        .webhooks()
        .create(channel_id, &CreateWebhook::new("djs-rs example"), REASON)
        .await?;
    let result = async {
        let token = webhook
            .token
            .as_deref()
            .ok_or("the new webhook has no token")?;
        let client = WebhookClient::new(webhook.id, token);
        let mut body = ExecuteWebhook::from("Sent through a `WebhookClient`: no bot token needed.");
        body.username = Some("Webhook demo".into());
        let message = client.send(body).await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        let edit = EditWebhookMessage::from("Edited through the same webhook; gone in 5 seconds.");
        client.edit_message(message.id, &edit).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        client.delete_message(message.id, None).await?;
        Ok::<_, HandlerError>(())
    }
    .await;
    // Delete the webhook whatever happened above.
    ctx.webhooks().delete(webhook.id, REASON).await?;
    result
}

/// Changes the presence every minute to the number of events seen.
fn rotate_presence(client: &Client, stats: Arc<Stats>) {
    let client = client.clone();
    tokio::spawn(async move {
        let mut every_minute = tokio::time::interval(Duration::from_secs(60));
        every_minute.tick().await; // The first tick is immediate.
        loop {
            every_minute.tick().await;
            let activity = Activity::watching(format!("{} events", stats.total()));
            // Sent on every shard, paced by the gateway's command limit.
            if let Err(error) =
                client.set_presence(&UpdatePresence::new(Status::Online, [activity]))
            {
                eprintln!("Could not update the presence: {error}");
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    common::init_tracing();

    if let Some(url) = common::config_value("DISCORD_WEBHOOK_URL") {
        WebhookClient::from_url(&url)?
            .send("The djs-rs advanced example is starting.")
            .await?;
    }

    let client = Client::builder(common::token()?)
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
        // The default, spelled out: the shard count comes from Get Gateway Bot.
        .shards(Shards::Auto)
        .presence(UpdatePresence::new(
            Status::Idle,
            [Activity::playing("starting up")],
        ))
        // Handlers run one after another, in the order of their shard's events.
        .handler_mode(HandlerMode::Sequential)
        .build();

    // Every event, in order, whatever handlers are registered. The stream is unbounded, so
    // it is read in a task of its own.
    let stats = Arc::new(Stats::new());
    let mut events = client.events();
    let counter = Arc::clone(&stats);
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            counter.record(event.event.name());
        }
    });
    rotate_presence(&client, Arc::clone(&stats));

    client.on(Events::ShardReady, |ctx, ready| async move {
        let shard = ctx.shard_id();
        println!(
            "Shard {} of {} is ready with {} guilds.",
            shard.number(),
            shard.total(),
            ready.guilds.len()
        );
        Ok(())
    });

    client.on(Events::ClientReady, |_ctx, ready| async move {
        println!(
            "Logged in as {}: every shard is ready. Send `!stats` or `!webhook`.",
            ready.user.tag()
        );
        Ok(())
    });

    client.on(Events::MessageCreate, move |ctx, msg| {
        let stats = Arc::clone(&stats);
        async move {
            match msg.content.trim() {
                "!stats" => {
                    let top: Vec<_> = stats
                        .top(5)
                        .into_iter()
                        .map(|(name, count)| format!("`{name}` {count}"))
                        .collect();
                    let text = format!(
                        "{} events in {:.0?} on shard {}: {}",
                        stats.total(),
                        stats.started.elapsed(),
                        ctx.shard_id().number(),
                        top.join(" · ")
                    );
                    spawn_logged(async move {
                        msg.reply(&ctx, text).await?;
                        Ok(())
                    });
                }
                "!webhook" => {
                    let channel_id = msg.channel_id;
                    spawn_logged(webhook_demo(ctx, channel_id));
                }
                _ => {}
            }
            Ok(())
        }
    });

    println!(
        "The handlers need the intents {:?}.",
        client.required_intents()
    );
    common::shutdown_on_ctrl_c(&client);
    client.login().await?;
    Ok(())
}
