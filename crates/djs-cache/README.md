# djs-cache

Configurable, pluggable in-memory cache for Discord entities.

> **Status:** pre-alpha — this crate is a placeholder and contains no functionality yet.
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** the caching layer inside `discord.js` (`makeCache`, `sweepers`)

## Planned scope

- A cache trait for pluggable storage backends; `DashMap` by default
- Per-resource enable/disable and size limits
- Periodic sweepers
- Updating the cache from every gateway event

## Part of djs-rs

`djs-cache` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
