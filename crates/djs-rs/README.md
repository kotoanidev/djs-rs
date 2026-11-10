# djs-rs

High-level, discord.js-style framework for building Discord bots in Rust.

> **Status:** pre-alpha — this crate is a placeholder and contains no functionality yet.
> Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `discord.js`

## Planned scope

- `Client` builder, login, ready lifecycle and graceful shutdown
- A typed event system (`client.on` / `client.once`)
- Rich structures with methods (`Message::reply`, `Interaction::reply`, …)
- Managers with discord.js-like fetch/cache semantics
- Collectors, partials, interaction helpers
- Re-exports of every lower-level djs-rs crate

## Part of djs-rs

`djs-rs` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
