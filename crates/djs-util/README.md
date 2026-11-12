# djs-util

Shared utilities used across the djs-rs crates.

> **Status:** pre-alpha — this crate is an intentional placeholder with no items yet (D-218).
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/util`

## Planned scope

Small, dependency-free helpers shared by two or more djs-rs crates, added only when such a
need appears. The shard ID calculation of `@discordjs/util` already lives in `djs-model`
(`ShardId::for_guild`); its other helpers are covered by the standard library or are
specific to JavaScript.

## Part of djs-rs

`djs-util` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
