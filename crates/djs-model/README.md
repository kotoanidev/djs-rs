# djs-model

Discord API v10 types — users, guilds, messages, gateway payloads and events — for
[djs-rs](https://github.com/kotoanidev/djs-rs).

> **Status:** pre-alpha. Phase 1 (vertical slice) covers the types a minimal bot needs; the
> rest of the API surface arrives in Phase 2. Track progress in the
> [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `discord-api-types` (v10)

This crate contains **only data types** and their (de)serialization. It has no networking and
no async runtime, so it can be used by anything that speaks the Discord API: bots, dashboards,
HTTP interaction servers, webhook receivers or tooling.

## Highlights

- **Typed IDs.** [`Id<T>`](id::Id) wraps a `NonZeroU64` with a zero-sized marker, so a
  `UserId` can never be passed where a `ChannelId` is expected. `Id<T>` and `Option<Id<T>>`
  are both 8 bytes.
- **Compact values.** Strings are [`CompactString`](compact_str::CompactString) (short strings
  stored inline, without allocation); image hashes are stored as 16 bytes instead of a
  34-character string; timestamps are a single `i64`.
- **Forward compatible.** Unknown fields are ignored and unknown enum values are preserved
  (`MessageType::Unknown(250)`), so new Discord features never break deserialization.
- **Fast gateway decoding.** [`GatewayEventDeserializer`](gateway::GatewayEventDeserializer)
  decodes each payload in a single pass straight into its concrete type, without an
  intermediate JSON value.

## Example

```rust
use djs_model::gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer};
use serde::de::DeserializeSeed;

let json = r#"{"t":"MESSAGE_CREATE","s":7,"op":0,"d":{
    "id":"1300000000000000000","channel_id":"1200000000000000000",
    "author":{"id":"80351110224678912","username":"nelly","discriminator":"0",
              "global_name":"Nelly","avatar":null},
    "content":"!ping","timestamp":"2026-11-10T19:42:07.123000+00:00",
    "edited_timestamp":null,"tts":false,"mention_everyone":false,"pinned":false,"type":0
}}"#;

let seed = GatewayEventDeserializer::from_json(json).unwrap();
let event = seed.deserialize(&mut serde_json::Deserializer::from_str(json)).unwrap();

if let GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message)) = event {
    assert_eq!(message.content, "!ping");
    assert_eq!(message.author.display_name(), "Nelly");
}
```

## Part of djs-rs

`djs-model` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
