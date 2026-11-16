# djs-rest

A rate-limit-aware client for the Discord REST API, built on hyper — part of
[djs-rs](https://github.com/kotoanidev/djs-rs).

> **Status:** alpha. Every Discord API v10 endpoint a bot or an app uses (232 routes; not the
> deprecated ones or the Slack- and GitHub-compatible webhooks), with rate limiting,
> retries, file uploads, CDN URLs and OAuth2. The API may still change between alpha releases; see the
> [changelog](https://github.com/kotoanidev/djs-rs/blob/main/CHANGELOG.md).

**discord.js counterpart:** `@discordjs/rest`

## Highlights

- **hyper 1.x + rustls (ring, webpki-roots).** HTTP/2 is negotiated automatically, so
  concurrent requests are multiplexed over one connection.
- **Rate limits handled for you.** Per-bucket sequential queues keyed by
  `X-RateLimit-Bucket` and the major parameter, a global limit (50/s by default), global
  pauses on global 429s, and automatic retries — the same model as `@discordjs/rest`.
- **Retries and timeouts.** Server errors, transport errors and timeouts are retried
  (3 times by default); every attempt is bounded by a timeout (15 s by default).
- **Cheap to clone.** Clones share the connection pool and rate-limit state.
- **Observable.** Every request emits [`tracing`](https://docs.rs/tracing) events; rate-limit
  hits are logged at `warn`.

## Example

```rust,no_run
use djs_model::{id::ChannelId, rest::CreateMessage};
use djs_rest::Rest;

# async fn run() -> Result<(), djs_rest::Error> {
let rest = Rest::new(std::env::var("DISCORD_TOKEN").unwrap());

let message = rest
    .create_message(ChannelId::new(1_234_567_890_123_456_789), &CreateMessage::from("Hello!"))
    .await?;
println!("sent message {}", message.id);
# Ok(())
# }
```

## Cargo features

| Feature | Default | Description |
|---|---|---|
| `native-roots` | | Also trust the operating system's root certificates (e.g. a corporate proxy's CA), in addition to the bundled Mozilla roots |

## Part of djs-rs

`djs-rest` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
(for example in HTTP-only interaction bots) or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
