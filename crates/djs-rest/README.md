# djs-rest

Rate-limit-aware Discord REST API client built on hyper.

> **Status:** pre-alpha — this crate is a placeholder and contains no functionality yet.
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/rest`

## Planned scope

- `hyper` 1.x HTTP client with `rustls` (default) or `native-tls`
- Per-route bucket, global and invalid-request rate limiting
- Typed methods for every v10 endpoint, multipart uploads, CDN URLs

## Part of djs-rs

`djs-rest` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
