# The cache

Discord tells a bot about everything that changes in its servers, through gateway events.
A *cache* keeps what those events said: the servers, their channels, roles and members,
recent messages. A bot that reads the cache answers without asking Discord's API, which is
faster and spends none of its rate limits.

This chapter's program is
[`cache_tour.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/cache_tour.rs).
It needs the **Server Members** and **Message Content** intents, enabled in the Developer
Portal (Bot → Privileged Gateway Intents).

## Choosing what to keep

A client has no cache until it is given one, and a cache keeps only what its
[`CacheConfig`] lists:

```rust,no_run
{{#include ../../code/examples/cache_tour.rs:config}}
```

- **`CacheConfig::default()`** keeps guilds, channels and roles, which most bots read and
  which cost little.
- **Each resource** (members, users, messages, presences, voice states, threads, emojis,
  stickers, scheduled events, stage instances, soundboard sounds) has a [`Limit`]:
  `Disabled`, `Max(n)` or `Unlimited`. A limit counts where it makes sense: messages per
  channel, members and roles per server, guilds and users over the whole cache. When a
  scope is full, its oldest entry makes room.
- **`CacheConfig::discord_js()`** keeps what discord.js keeps by default: everything, with
  the latest 200 messages of each channel. It is the most memory a bot can spend.
- **Intents** decide what can arrive: members need `GUILD_MEMBERS`, presences
  `GUILD_PRESENCES`, messages `GUILD_MESSAGES`. A resource cached without its intent stays
  empty.

The repository's [comparison with discord.js](https://github.com/kotoanidev/djs-rs/blob/main/docs/VS_NODE.md)
measures what each choice costs per server and member.

(discord.js: the `makeCache` option with `Options.cacheWithLimits`; discord.js always has a
cache.)

## Reading it

`ctx.cache()` is the client's cache (`None` without one). Each resource is read through a
*view*: `cache.guilds()`, `cache.members(guild_id)`, `cache.messages(channel_id)`, … with
`get`, `contains`, `len`, `keys` and `values`.

```rust,no_run
{{#include ../../code/examples/cache_tour.rs:read}}
```

What a view returns is an `Arc`: a shared handle, cheap to clone, which holds no lock, so a
handler can keep it across `.await` while the cache goes on changing. It is the state at the
time it was read.

Every event goes into the cache **before** its handlers run, so a handler sees the state
after its event: in a `GuildMemberAdd` handler the new member is already cached.

(discord.js: `client.guilds.cache`, `guild.members.cache`, `channel.messages.cache`.)

## The state before an event

An update or a deletion replaces or removes what the cache held, so the client hands it to
the event's handlers first. The payloads of those events have an `old` field:

```rust,no_run
{{#include ../../code/examples/cache_tour.rs:old}}
```

`old` is `None` when the cache did not hold the object: without a cache, with that
resource disabled, or for a message older than the cache's limit. The event still
arrives, with what Discord sent (for a deleted message, only its ID and channel).

(discord.js: `oldMessage`, `oldMember` and the other first arguments; discord.js drops
such events about uncached objects unless *partials* are enabled, djs-rs always sends
them.)

## Fetching through the cache

The managers (`ctx.members(guild)`, `ctx.channels()`, `ctx.messages(channel)`, …) read the
cache first and ask Discord only for what is missing, caching the answer:

```rust,no_run
{{#include ../../code/examples/cache_tour.rs:fetch}}
```

`fetch` returns a value that can be checked before it is awaited: `is_cached` says whether
it will be answered from the cache. `force` skips the cache, for when the cached state
might be stale (an event the bot missed while offline, say).

(discord.js: `guild.members.fetch(id)` and its `force` option.)

## Sweeping

Limits bound how much the cache holds; *sweeps* forget what has grown old. A sweep set in
the configuration (`sweep_messages`, `sweep_threads`) runs periodically while the client is
logged in; by default, like discord.js, threads archived for more than 4 hours are swept
every hour. Each resource can also be swept by hand, with a filter:

```rust,no_run
{{#include ../../code/examples/cache_tour.rs:sweep}}
```

(discord.js: the `sweepers` option and `client.sweepers`.)

## What's next

The managers have been calling Discord's API through the cache. The next chapter calls the
API directly: the REST client and its rate limits.

[`CacheConfig`]: ../api/djs_cache/struct.CacheConfig.html
[`Limit`]: ../api/djs_cache/enum.Limit.html
