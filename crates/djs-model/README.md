# djs-model

Discord API v10 types — users, guilds, channels, messages, interactions and gateway events.

> **Status:** pre-alpha — this crate is a placeholder and contains no functionality yet.
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `discord-api-types` (v10)

## Planned scope

- Snowflake/ID types, timestamps, colours, image hashes
- Permission, intent and flag bitsets
- Every REST and gateway payload of Discord API v10
- Fast (de)serialization tuned for the gateway hot path

## Part of djs-rs

`djs-model` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
