# djs-rs

A high-performance, discord.js-style framework for building Discord bots in Rust.

> **Status:** pre-alpha. A working end-to-end bot: client with automatic sharding, typed
> events, replies, presence and an optional cache. The full discord.js surface arrives in
> later phases — see the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

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
| Cache | `djs-cache` | `djs_rs::cache` |

Handlers run concurrently in their own tokio tasks by default (like discord.js); switch to
`HandlerMode::Sequential` for strictly ordered processing. If you prefer an explicit loop,
`client.events()` returns a stream of every event.

Structures stay plain data; behaviour such as `msg.reply(&ctx, …)` comes from extension traits
in the prelude that take the handler `Context` explicitly — no client reference is stored in
every structure.

## The cache

Unlike discord.js, the client keeps a cache only when asked to: a bot that does not read it
pays nothing for it. `ClientBuilder::cache` takes a `CacheConfig` choosing what to keep
(by default guilds, channels and roles; `CacheConfig::discord_js()` keeps what discord.js
keeps). Every event is applied to the cache before its handlers run, and the client runs the
configured periodic sweeps while it is logged in.

```rust,no_run
use djs_rs::{
    cache::{CacheConfig, Limit},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<(), djs_rs::Error> {
    let client = Client::builder(std::env::var("DISCORD_TOKEN").unwrap())
        .intents(Intents::GUILDS | Intents::GUILD_MEMBERS | Intents::GUILD_MESSAGES)
        .cache(CacheConfig::builder().members(Limit::Unlimited).build())
        .build();

    client.on(Events::MessageCreate, |ctx, msg| async move {
        let cache = ctx.cache().expect("the client has a cache");
        if let Some(member) = msg.guild_id.and_then(|id| cache.members(id).get(msg.author.id)) {
            println!("{} wrote a message", member.display_name());
        }
        Ok(())
    });

    client.login().await
}
```

The cache's storage is pluggable: `ClientBuilder::cache_backend` takes any
`djs_rs::cache::Backend`, and the client's type follows it (`Client<MyBackend>`); code that
does not name a backend uses the default `DashMapBackend`.

## Performance: the global allocator

With the default concurrent handlers, events are decoded on the shard's task and dropped on
tokio worker threads. The default allocator on Linux (glibc) handles this producer/consumer
pattern poorly. In the stress tests (`tests/stress_mimalloc.rs`, see `docs/PERFORMANCE.md`),
using [mimalloc](https://docs.rs/mimalloc) as the bot's global allocator raised the
throughput by about 80 % with 10 handlers per event, by 30–50 % with an `events()` stream and
by about 15 % with one handler; with `HandlerMode::Sequential` it made no difference. These are
rates of hundreds of thousands of events per second per shard, so only very large bots need
to care. To switch, add `mimalloc = "0.1"` to your bot's dependencies and declare:

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
# fn main() {}
```

## Cargo features

| Feature | Default | Description |
|---|---|---|
| `zstd-stream` | ✅ | `zstd-stream` gateway compression (needs a C compiler) |
| `zlib-stream` | | `zlib-stream` gateway compression in pure Rust (no C compiler) |
| `native-roots` | | Also trust the operating system's root certificates (e.g. a corporate proxy's CA), in addition to the bundled Mozilla roots, for REST and gateway connections |

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
