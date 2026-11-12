# djs-cache

Configurable, pluggable in-memory cache for Discord entities.

> **Status:** pre-alpha, Phase 5 step 3 — every resource is cached from its gateway events
> (guilds, channels, threads, roles, emojis, stickers, members, users, messages with their
> reactions, presences, voice states, scheduled events, stage instances, soundboard sounds and
> the current user); sweepers follow.
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** the caching layer inside `discord.js` (the managers' `cache`
collections, the `makeCache` and `sweepers` client options)

## Overview

- **What is cached** is set per [`Resource`] with a [`Limit`] in a [`CacheConfig`]:
  [`Limit::Disabled`], [`Limit::Max`] or [`Limit::Unlimited`], like discord.js
  `Options.cacheWithLimits`.
- **The default is minimal:** guilds, channels and roles (and always the current user).
  [`CacheConfig::discord_js`] keeps what discord.js keeps by default: everything, with the
  latest 200 messages per channel.
- **Limits count where discord.js counts them:** per guild for guild resources (members,
  roles, emojis, presences, …), per channel for messages, over the whole cache for guilds,
  channels and users. A full scope evicts the entry inserted first.
- **Events go in by reference:** [`Cache::update`] takes any payload implementing
  [`UpdateCache`] and returns its previous state, typed per event (a member update returns
  the member before it); [`Cache::update_dispatch`] takes a whole `DispatchEvent`.
- **Reads go through views**, like a discord.js manager's `cache`: `cache.guilds().get(id)`,
  `cache.members(guild_id).values()`, … They return [`Arc`](std::sync::Arc)s: cheap to clone,
  holding no lock, safe to keep across `.await`.
- **Own types only where needed:** roles, emojis, stickers, users, channels, threads, voice
  states, scheduled events, stage instances and soundboard sounds are the `djs-model` types;
  [`CachedGuild`] leaves out the lists kept elsewhere, [`CachedMember`] and
  [`CachedMessage`] share their user with the user resource, and [`CachedPresence`] keeps
  only its user's ID.
- **Storage is pluggable:** the cache keeps each resource in a [`Store`] created by a
  [`Backend`]. The default [`DashMapBackend`] uses sharded concurrent maps with `foldhash`.

| Resource | Default | `Limit::Max` counts | discord.js manager |
|---|---|---|---|
| [`Guild`](Resource::Guild) | unlimited | whole cache | `GuildManager` |
| [`Channel`](Resource::Channel) | unlimited | whole cache | `ChannelManager` |
| [`Role`](Resource::Role) | unlimited | per guild | `RoleManager` |
| [`User`](Resource::User) | disabled | whole cache | `UserManager` |
| [`Member`](Resource::Member) | disabled | per guild | `GuildMemberManager` |
| [`Message`](Resource::Message) | disabled | per channel | `MessageManager` |
| [`Presence`](Resource::Presence) | disabled | per guild | `PresenceManager` |
| [`VoiceState`](Resource::VoiceState) | disabled | per guild | `VoiceStateManager` |
| [`Thread`](Resource::Thread), [`Emoji`](Resource::Emoji), [`Sticker`](Resource::Sticker), [`ScheduledEvent`](Resource::ScheduledEvent), [`StageInstance`](Resource::StageInstance), [`SoundboardSound`](Resource::SoundboardSound) | disabled | per guild | the guild's managers |

## Examples

Feeding the cache and reading it:

```rust
use djs_cache::{Cache, CacheConfig, Limit};
use djs_model::gateway::DispatchEvent;

fn on_event(cache: &Cache, event: &DispatchEvent) {
    if let DispatchEvent::GuildMemberUpdate(update) = event {
        // The member before the update, if it was cached.
        let before = cache.update(&**update);
        let after = cache.members(update.guild_id).get(update.user.id);
        if let (Some(before), Some(after)) = (before, after) {
            if before.nick != after.nick {
                println!("{} is now {}", before.display_name(), after.display_name());
            }
        }
    } else {
        cache.update_dispatch(event);
    }
}

let cache = Cache::with_config(CacheConfig::builder().members(Limit::Unlimited).build());
on_event(&cache, &DispatchEvent::Resumed);
```

Choosing what to cache:

```rust
use djs_cache::{Cache, CacheConfig, CacheConfigBuilder, Limit, Resource};

// Guilds, channels and roles only.
let small = Cache::new();
assert!(!small.config().is_enabled(Resource::Member));

// Members too, and the latest 50 messages of every channel.
let config = CacheConfig::builder()
    .members(Limit::Unlimited)
    .messages(Limit::Max(50))
    .build();
let cache = Cache::with_config(config);
assert_eq!(cache.config().limit(Resource::Message), Limit::Max(50));

// discord.js' defaults without presences.
let config = CacheConfigBuilder::from(CacheConfig::discord_js())
    .presences(Limit::Disabled)
    .build();
assert!(config.is_enabled(Resource::VoiceState));
```

## Part of djs-rs

`djs-cache` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
