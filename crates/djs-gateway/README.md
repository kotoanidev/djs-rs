# djs-gateway

Discord gateway (WebSocket) client with sharding and transport compression.

> **Status:** pre-alpha — this crate is a placeholder and contains no functionality yet.
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/ws`

## Planned scope

- Shards built on `tokio-websockets`: identify, heartbeat, resume, reconnect
- `zstd-stream` (default) and `zlib-stream` transport compression
- Gateway send rate limiting and identify concurrency
- In-process sharding manager

## Part of djs-rs

`djs-gateway` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
