# Coming from discord.js

djs-rs follows discord.js v14's concepts and names, so most of what a discord.js developer
knows carries over: a `Client` with intents, events by name, managers with a `cache` and
`fetch`, builders for embeds, components and commands. This page maps one to the other and
lists what differs, and why.

## The same bot

In discord.js:

```js
const { Client, Events, GatewayIntentBits } = require("discord.js");

const client = new Client({
  intents: [GatewayIntentBits.Guilds, GatewayIntentBits.GuildMessages, GatewayIntentBits.MessageContent],
});
client.once(Events.ClientReady, (ready) => console.log(`Logged in as ${ready.user.tag}`));
client.on(Events.MessageCreate, async (message) => {
  if (message.content === "!ping") await message.reply("Pong!");
});
client.login(process.env.DISCORD_TOKEN);
```

In djs-rs (the [first chapter](getting-started/first-bot.md)'s program):

```rust,no_run
{{#include ../code/examples/first_bot.rs:all}}
```

The shape is the same. What Rust adds: handlers are `async` closures returning a `Result`
(an `Err` is logged with the event's name), they receive a [`Context`] (`ctx`) through which
they reach the REST client, the cache and the managers, and `msg.reply` needs it.

## Names

| discord.js | djs-rs |
|---|---|
| `new Client({ intents })` | `Client::builder(token).intents(…).build()` |
| `GatewayIntentBits.GuildMessages` | `Intents::GUILD_MESSAGES` |
| `client.on(Events.MessageCreate, fn)` | `client.on(Events::MessageCreate, fn)` |
| `client.login(token)` | `client.login().await` (the token is given to the builder) |
| `client.destroy()` | `client.shutdown()` |
| `client.guilds.cache.get(id)` | `ctx.cache()?.guilds().get(id)` |
| `client.guilds.fetch(id)` | `ctx.guilds().fetch(id).await` |
| `guild.members.fetch(id)` | `ctx.members(guild_id).fetch(id).await`, or `guild.members(&ctx)` |
| `channel.send("hi")` | `channel.send(&ctx, "hi").await` |
| `message.reply`, `.edit`, `.delete`, `.react` | the same, with `&ctx` |
| `interaction.reply`, `deferReply`, `editReply`, `followUp`, `showModal` | `reply`, `defer_reply`, `edit_reply`, `follow_up`, `show_modal` |
| `EmbedBuilder`, `ButtonBuilder`, `ActionRowBuilder`, `SlashCommandBuilder` | the same names, in `djs_rs::builders` |
| `PermissionFlagsBits.BanMembers` | `Permissions::BAN_MEMBERS` |
| `@discordjs/rest`'s `REST` | [`Rest`](going-further/rest.md) (`djs-rest`) |
| `@discordjs/voice` | [`client.voice()`](going-further/voice.md) (`djs-voice`) |
| `ShardingManager`, `broadcastEval` | [`ClusterManager`, cluster commands](going-further/sharding.md) |
| `time()`, `userMention()`, `bold()` (formatters) | `time`, `user_mention`, `bold`, in `djs_rs::formatters` |
| `Collection` | `djs_rs::collection::Collection` |

Methods are `snake_case`, types `UpperCamelCase`, constants `SCREAMING_CASE`: `fetchAll`
is `fetch_all`, `ChannelType.GuildText` is `ChannelType::GuildText`. The API reference
names the discord.js counterpart of every item ("discord.js counterpart: …"), so its
search finds a djs-rs item by its discord.js name.

## What differs, and why

- **The cache is opt-in.** A client has none until `.cache(…)` is called, and then keeps
  only what its configuration lists (guilds, channels and roles by default);
  `CacheConfig::discord_js()` keeps what discord.js does. A bot that does not read the
  cache pays nothing for it. See [The cache](basics/the-cache.md).
- **No partials.** discord.js drops events about uncached objects unless *partials* are
  enabled. djs-rs always sends every event; what the cache did not hold is `None` (the
  `old` of an update) or an ID to fetch.
- **Shards are automatic.** `Shards::Auto` is the default: a client runs as many shards
  as Discord recommends.
- **Errors are values.** A failed request returns an error (`djs_rest::Error`, with
  Discord's `ErrorCode`) instead of throwing; `?` in a handler passes it on, to be logged.
- **Interactions have routers.** `client.command("ping", …)`, `client.component("vote", …)`
  and `client.modal("form", …)` replace a `switch` on `commandName` or `customId`; what
  follows a `:` in a custom ID is the handler's `args`.
- **Types are exact.** A payload's fields are typed as Discord documents them: an
  optional field is an `Option`, an edit's field a `Patch` (keep, set, or clear), a
  channel an enum of its kinds (`Channel::Text`, `Channel::Voice`, …). The compiler checks
  what discord.js checks at run time.
- **Handlers run concurrently.** Each event runs its handlers in a task of its own (the
  default `HandlerMode::Concurrent`), so a slow handler holds nothing up;
  `HandlerMode::Sequential` keeps the order of events.
- **No eval.** Cluster commands are declared types compiled into the bot, not code sent as
  text.

## Performance

The repository measures djs-rs and discord.js on the same machine with the same mock
Discord servers: decoding, an event flood, memory per server, REST throughput. The numbers
are in [`docs/VS_NODE.md`](https://github.com/kotoanidev/djs-rs/blob/main/docs/VS_NODE.md).

[`Context`]: ../api/djs_rs/struct.Context.html
