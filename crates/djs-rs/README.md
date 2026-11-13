# djs-rs

A high-performance, discord.js-style framework for building Discord bots in Rust.

> **Status:** pre-alpha. A working end-to-end bot: client with automatic sharding, every
> discord.js event (with the previous state from the cache), managers for guilds, channels,
> users, members, messages, reactions, roles, bans, threads, emojis, stickers, events,
> stages, soundboard sounds, invites, webhooks, application commands and auto moderation
> with methods on the structures, interaction responses, typed option getters and a
> command router, collectors, a `WebhookClient`, presence and an optional cache. The
> full discord.js surface arrives in later phases — see the
> [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `discord.js`

## Example

```rust,no_run
use djs_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), djs_rs::Error> {
    let client = Client::builder(std::env::var("DISCORD_TOKEN").unwrap())
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
        .build();

    client.on(Events::ClientReady, |_ctx, ready| async move {
        println!("Logged in as {}", ready.user.tag());
        Ok(())
    });

    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!ping" {
            msg.reply(&ctx, "Pong!").await?;
        }
        Ok(())
    });

    client.login().await
}
```

A runnable version lives in [`examples/ping.rs`](https://github.com/kotoanidev/djs-rs/blob/main/crates/djs-rs/examples/ping.rs).

### More examples

Each example is one file in
[`examples/`](https://github.com/kotoanidev/djs-rs/tree/main/crates/djs-rs/examples), run
with `cargo run --example <name>`, its setup explained at the top. They read
`DISCORD_TOKEN` (and their other settings) from the environment or a git-ignored `.env`
(see `.env.example`).

| Example | What it shows |
|---|---|
| `ping` | the smallest bot: `!ping` → `Pong!` |
| `slash_commands` | guild command registration, the router, options and subcommands, autocomplete, a modal awaited with `await_modal_submit`, ephemeral and deferred replies |
| `components` | a paginated button menu with a component collector, a quiz with `await_messages`, a reaction poll with `await_reactions` |
| `moderation` | join, leave, ban, edit and deletion logs with the cached previous state, `/timeout`, `/ban`, `/purge` and `/members` (`fetch_all`) limited to moderators |
| `advanced` | `Shards::Auto`, a changing presence, the `events()` stream, `HandlerMode::Sequential`, `WebhookClient`, mimalloc |

The examples that register commands remove them again with `-- --cleanup`.

## How it fits together

`djs-rs` wires the lower-level crates into a discord.js-like client:

| Layer | Crate | Re-exported as |
|---|---|---|
| Discord API types | `djs-model` | `djs_rs::model` |
| REST client | `djs-rest` | `djs_rs::rest` |
| Gateway | `djs-gateway` | `djs_rs::gateway` |
| Cache | `djs-cache` | `djs_rs::cache` |
| Builders (embeds, commands, components, modals) | `djs-builders` | `djs_rs::builders` |
| Formatters (mentions, timestamps, markdown, escaping) | `djs-formatters` | `djs_rs::formatters` |
| `Collection` (insertion-ordered map with discord.js' helpers) | `djs-collection` | `djs_rs::collection` |

## Events

`Events` has discord.js' events by their names (`Events::GuildMemberAdd`,
`Events::MessageUpdate`, …), the gateway dispatches discord.js has no event for, the shard
lifecycle (`ShardReady`, `ShardReconnecting`, `ShardResume`, `Invalidated`, `ShardError`,
`ShardDisconnect`) and `Raw`, which hands every dispatch as it was decoded.

- Most handlers receive the `djs-model` payload unchanged (behind an `Arc`).
- Where discord.js hands an old or a removed object, the payload has `old` — what the cache
  held before the event, `None` without a cache — and `new`, and reads like `new`:
  `update.old` and `update.content` in a `MessageUpdate` handler.
- `GuildCreate` (joined) and `GuildAvailable` (back from `READY` or an outage) are told apart
  with or without a cache; the per-emoji and per-sticker events need those in the cache.
- `ClientReady` fires once, at the `READY` of the last shard, without waiting for the guilds
  (unlike discord.js, the cache may still be filling).

```rust,no_run
use djs_rs::prelude::*;

fn log_edits(client: &Client) {
    client.on(Events::MessageUpdate, |_ctx, update| async move {
        if let Some(old) = &update.old {
            println!("edited: {:?} -> {:?}", old.content, update.content);
        }
        Ok(())
    });
}
```

Handlers run concurrently in their own tokio tasks by default (like discord.js); switch to
`HandlerMode::Sequential` for strictly ordered processing. If you prefer an explicit loop,
`client.events()` returns a stream of every event (all but `Raw`).

### Intents and partials

Discord only sends an event if the client requests one of its intents (`Event::INTENTS`;
`MessageCreate` comes with `GUILD_MESSAGES` for guilds and `DIRECT_MESSAGES` for DMs). At
login the client warns about handlers, routes and collectors whose events none of its intents
covers (D-271), and `client.required_intents()` shows what they need (D-272); the intents
themselves stay explicit. Reading guild messages' content also needs the privileged
`MESSAGE_CONTENT` intent.

There is no `partials` option (D-273): every event arrives even when the cache does not hold
its objects — `old` is `None` and payloads carry IDs — and the managers' `fetch` completes
them. The documentation of the `djs_rs::event` module maps each discord.js partial to what
djs-rs hands instead.

## Managers and structure methods

The operations of a resource live on a manager reached from the handler `Context`:
`ctx.guilds()`, `ctx.channels()`, `ctx.guild_channels(guild)`, `ctx.users()`,
`ctx.members(guild)`, `ctx.messages(channel)`, `ctx.roles(guild)`, `ctx.bans(guild)`,
`ctx.emojis(guild)`, `ctx.stickers(guild)`, `ctx.scheduled_events(guild)`,
`ctx.stage_instances(guild)`, `ctx.soundboard_sounds(guild)`, `ctx.threads(channel)`,
`ctx.thread_members(thread)`, `ctx.auto_moderation_rules(guild)`, `ctx.commands()`,
`ctx.guild_commands(guild)`, `ctx.application_emojis()`, `ctx.invites()` and
`ctx.webhooks()` (discord.js' `client.guilds`, `guild.channels`, `guild.members`,
`channel.threads`, `client.application.commands`, …).

- `fetch(id)` returns the cached value without a request, and asks Discord otherwise; the
  answer is cached. `.force()` always asks Discord; `.is_cached()` tells beforehand.
- Lists (`ctx.roles(guild).list()`, `ctx.messages(channel).list(&query)`, …) are
  `Collection`s in Discord's order.
- Managers return the type the cache keeps, behind an `Arc` (`Arc<CachedMember>`,
  `Arc<CachedMessage>`, `Arc<Channel>`, …), whether the value came from the cache or from
  Discord, and with or without a cache. Bans, which are never cached, are `djs-model` types.
- Deleting through a manager only sends the request: the cache changes when the gateway
  event arrives, so the delete event still carries the previous state.
- Some answers come through the gateway: `ctx.members(guild).fetch_all()`,
  `.fetch_by_query("ab")` and `.fetch_many(ids)` gather the member chunks of their request
  (D-270), and `ctx.guilds().fetch_soundboard_sounds(ids, time)` the sounds of several guilds.

Structures stay plain data; behaviour such as `msg.reply(&ctx, …)` comes from extension traits
in the prelude (`MessageExt`, `ChannelExt`, `UserExt`, `MemberExt`, `GuildExt`) that take the
`Context` explicitly — no client reference is stored in every structure. They work on the
model types, the cached types and the event payloads that dereference to them.

```rust,no_run
use djs_rs::{model::util::Timestamp, prelude::*};

fn moderate(client: &Client) {
    client.on(Events::MessageCreate, |ctx, msg| async move {
        let Some(guild_id) = msg.guild_id else { return Ok(()) };
        if msg.content.contains("discord.gg/") {
            msg.delete(&ctx).await?;
            let member = ctx.members(guild_id).fetch(msg.author.id).await?;
            let in_ten_minutes = Timestamp::from_unix_secs(Timestamp::now().as_unix_secs() + 600);
            member.timeout(&ctx, in_ten_minutes, Some("invite link")).await?;
            msg.author.send(&ctx, "Invite links are not allowed here.").await?;
        }
        Ok(())
    });
}
```

## Interactions

Slash commands, context menus, buttons, select menus, modals and autocomplete all arrive as
`Events::InteractionCreate` (as in discord.js). `interaction.command()`, `component()`,
`modal()` and `autocomplete()` return the typed data of that kind, whose getters read the
options and fields with their resolved users, members, roles and channels —
`get_string("reason")`, `get_user("target")`, `subcommand()`, `text_value("feedback")`,
`selected_roles("roles")`, … (discord.js' `CommandInteractionOptionResolver` and
`ModalSubmitFields`). `InteractionExt` in the prelude answers: `reply`, `defer_reply`,
`edit_reply`, `follow_up`, `update`, `defer_update`, `show_modal`, `respond`, … — one
method per Discord callback; the order Discord requires is documented on the trait.

```rust,no_run
use djs_rs::{model::message::MessageFlags, prelude::*};

fn interactions(client: &Client) {
    client.on(Events::InteractionCreate, |ctx, interaction| async move {
        if let Some(command) = interaction.command() {
            match command.name.as_str() {
                "greet" => {
                    let who = command.get_user("user").map_or("stranger", |user| &user.username);
                    interaction.reply(&ctx, format!("Hello, {who}!")).await?;
                }
                "report" => {
                    interaction.defer_reply(&ctx, MessageFlags::EPHEMERAL).await?;
                    // … slow work, up to 15 minutes …
                    interaction.edit_reply(&ctx, "Report sent").await?;
                }
                _ => {}
            }
        } else if let Some(modal) = interaction.modal() {
            let feedback = modal.text_value("feedback").unwrap_or_default();
            interaction.reply(&ctx, format!("Thanks for: {feedback}")).await?;
        }
        Ok(())
    });
}
```

### The router

Instead of one `match` in `InteractionCreate`, handlers can be routed by name (D-251–D-255):
`client.command("mod ban", …)` for a subcommand (a route for `"mod"` catches the
subcommands without their own), `client.autocomplete(…)`, and `client.component("vote", …)` /
`client.modal(…)` by the `custom_id` part before `:` — `vote:42` reaches `"vote"` with
`args()` = `"42"`. Each handler gets a typed wrapper that dereferences to the data and
answers like the interaction. The router does not register commands with Discord (use
`ctx.commands().set(…)`), and `InteractionCreate` still fires for every interaction.

```rust,no_run
use djs_rs::prelude::*;

fn routes(client: &Client) {
    client.command("ping", |ctx, cmd| async move {
        cmd.reply(&ctx, "Pong!").await?;
        Ok(())
    });
    client.command("mod ban", |ctx, cmd| async move {
        let reason = cmd.get_string("reason").unwrap_or("no reason");
        cmd.reply(&ctx, format!("Banned: {reason}")).await?;
        Ok(())
    });
    client.component("vote", |ctx, button| async move {
        button.update(&ctx, format!("Voted for {}", button.args())).await?;
        Ok(())
    });
}
```

## Collectors

A collector yields what arrives where it watches — the messages of a channel, the reactions
or components of a message, the components of an interaction's reply, the modals a user
submits — as a stream, until its `time`, `idle` or `max` limit, its `stop()`, or the deletion
of what it watches (D-263–D-269). The `await_*` shortcuts wait for one click or modal
(`Result`, with the reason on failure) or collect until the end (a `Collection` with
`reason()`). Events reach collectors through an index by ID, so a thousand open menus cost
an event one lookup, and they still reach the handlers and the router.

```rust,no_run
use std::time::Duration;

use djs_rs::prelude::*;

fn quiz(client: &Client) {
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content != "!quiz" {
            return Ok(());
        }
        msg.reply(&ctx, "What is 6 × 7?").await?;
        let author = msg.author.id;
        let answers = ctx
            .await_messages(msg.channel_id)
            .filter(move |answer| answer.author.id == author)
            .max(1)
            .time(Duration::from_secs(30))
            .await;
        let verdict = match answers.first() {
            Some(answer) if answer.content.trim() == "42" => "Correct!",
            Some(_) => "No, it is 42.",
            None => "Time is up.",
        };
        msg.reply(&ctx, verdict).await?;
        Ok(())
    });
}
```

With the `Sequential` handler mode a handler waiting for a collector blocks its shard, so
the collector receives nothing from it; keep the default `Concurrent` mode (D-267).

## Webhooks without a bot

`WebhookClient` sends through a webhook URL — from a script, a CI job or a server — with no
bot token and no gateway connection (discord.js' `WebhookClient`):

```rust,no_run
use djs_rs::WebhookClient;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let webhook = WebhookClient::from_url(&std::env::var("WEBHOOK_URL")?)?;
webhook.send("Build passed").await?;
# Ok(())
# }
```

## The cache

Unlike discord.js, the client keeps a cache only when asked to: a bot that does not read it
pays nothing for it. `ClientBuilder::cache` takes a `CacheConfig` choosing what to keep
(by default guilds, channels and roles; `CacheConfig::discord_js()` keeps what discord.js
keeps). Every event is applied to the cache before its handlers run, and the client runs the
configured periodic sweeps while it is logged in.

```rust,no_run
use djs_rs::{
    cache::{CacheConfig, Limit},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<(), djs_rs::Error> {
    let client = Client::builder(std::env::var("DISCORD_TOKEN").unwrap())
        .intents(Intents::GUILDS | Intents::GUILD_MEMBERS | Intents::GUILD_MESSAGES)
        .cache(CacheConfig::builder().members(Limit::Unlimited).build())
        .build();

    client.on(Events::MessageCreate, |ctx, msg| async move {
        let cache = ctx.cache().expect("the client has a cache");
        if let Some(member) = msg.guild_id.and_then(|id| cache.members(id).get(msg.author.id)) {
            println!("{} wrote a message", member.display_name());
        }
        Ok(())
    });

    client.login().await
}
```

The cache's storage is pluggable: `ClientBuilder::cache_backend` takes any
`djs_rs::cache::Backend`, and the client's type follows it (`Client<MyBackend>`); code that
does not name a backend uses the default `DashMapBackend`.

## Performance: the global allocator

With the default concurrent handlers, events are decoded on the shard's task and dropped on
tokio worker threads. The default allocator on Linux (glibc) handles this producer/consumer
pattern poorly. In the stress tests (`tests/stress_mimalloc.rs`, see `docs/PERFORMANCE.md`),
using [mimalloc](https://docs.rs/mimalloc) as the bot's global allocator raised the
throughput by about 80 % with 10 handlers per event, by 30–50 % with an `events()` stream and
by about 15 % with one handler; with `HandlerMode::Sequential` it made no difference. These are
rates of hundreds of thousands of events per second per shard, so only very large bots need
to care. To switch, add `mimalloc = "0.1"` to your bot's dependencies and declare:

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
# fn main() {}
```

## Cargo features

| Feature | Default | Description |
|---|---|---|
| `zstd-stream` | ✅ | `zstd-stream` gateway compression (needs a C compiler) |
| `zlib-stream` | | `zlib-stream` gateway compression in pure Rust (no C compiler) |
| `native-roots` | | Also trust the operating system's root certificates (e.g. a corporate proxy's CA), in addition to the bundled Mozilla roots, for REST and gateway connections |

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
