# djs-rs

A high-performance, discord.js-style framework for building Discord bots in Rust.

> **Status:** pre-alpha. Phase 1 (vertical slice) delivers a working end-to-end bot: client,
> typed events, replies. The full discord.js surface arrives in later phases — see the
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

## How it fits together

`djs-rs` wires the lower-level crates into a discord.js-like client:

| Layer | Crate | Re-exported as |
|---|---|---|
| Discord API types | `djs-model` | `djs_rs::model` |
| REST client | `djs-rest` | `djs_rs::rest` |
| Gateway | `djs-gateway` | `djs_rs::gateway` |

Handlers run concurrently in their own tokio tasks by default (like discord.js); switch to
`HandlerMode::Sequential` for strictly ordered processing. If you prefer an explicit loop,
`client.events()` returns a stream of every event.

Structures stay plain data; behaviour such as `msg.reply(&ctx, …)` comes from extension traits
in the prelude that take the handler `Context` explicitly — no client reference is stored in
every structure.

## Cargo features

| Feature | Default | Description |
|---|---|---|
| `zstd-stream` | ✅ | `zstd-stream` gateway compression (needs a C compiler) |

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
