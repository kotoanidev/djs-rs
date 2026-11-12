# djs-gateway

A Discord gateway (WebSocket) client with transport compression — part of
[djs-rs](https://github.com/kotoanidev/djs-rs).

> **Status:** pre-alpha. Self-healing shards, sharding with a shared IDENTIFY queue, the
> gateway send limit and zstd/zlib transport compression are in place. Track progress in the
> [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/ws`

## Highlights

- **Poll-driven.** `shard.next_event().await` drives the connection: identify/resume,
  heartbeats with jitter, zombie detection, Hello timeout, exponential reconnect backoff and
  close-code handling all happen while you wait for the next event. No background tasks, no
  internal channels.
- **zstd-stream compression** by default (feature `zstd-stream`), or **zlib-stream** in pure
  Rust (feature `zlib-stream`), with a decompression buffer that is reused across messages
  and released after oversized payloads.
- **Sharding.** `create_shards(range, total, config)` creates the shards of a bot; they take
  turns identifying through a shared `IdentifyQueue` (`LocalIdentifyQueue` honours
  `max_concurrency`; implement the trait to coordinate several processes). Run each shard in
  its own task.
- **Send limit.** Commands sent through a `MessageSender` are paced under Discord's
  120-per-minute limit and wait in order instead of getting the connection closed.
- **Single-pass decoding** via `djs_model`'s `GatewayEventDeserializer`.
- **tokio-websockets + rustls** with an explicit ring provider and Mozilla roots, so it never
  panics because another crate enabled a different rustls provider.

## Example

```rust,no_run
use djs_gateway::Shard;
use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, ShardId};

# async fn run() {
let token = std::env::var("DISCORD_TOKEN").unwrap();
let mut shard = Shard::new(
    ShardId::ONE,
    token,
    Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
);

while let Some(event) = shard.next_event().await {
    match event {
        Ok(GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message))) => {
            println!("{}: {}", message.author.username, message.content);
        }
        Ok(_) => {}
        Err(error) if error.is_fatal() => break,
        Err(error) => eprintln!("gateway error: {error}"),
    }
}
# }
```

## Cargo features

| Feature | Default | Description |
|---|---|---|
| `zstd-stream` | ✅ | `zstd-stream` transport compression (needs a C compiler) |
| `zlib-stream` | | `zlib-stream` transport compression with the pure-Rust `zlib-rs` (no C compiler; compresses less and decodes more slowly than zstd) |
| `native-roots` | | Also trust the operating system's root certificates (e.g. a corporate proxy's CA), in addition to the bundled Mozilla roots |

## Part of djs-rs

`djs-gateway` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
