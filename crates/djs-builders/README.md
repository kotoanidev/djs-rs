# djs-builders

Validated builders for Discord embeds, application commands, components and modals.

> **Status:** pre-alpha. The embed builder is done; command, component and modal builders
> follow. Track progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/builders`

## How the builders work

Every builder takes and returns itself, so calls chain; a value that depends on a condition
is set by reassigning (`builder = builder.field(…)`). Setters only store what they are given
and never fail. `build()` checks every Discord limit at the end and returns the finished
[`djs-model`](https://docs.rs/djs-model) type, or the first problem it found as a
[`ValidationError`] that names the offending part the way Discord does (`fields[3].name`).
`build_unchecked()` skips the checks, for example when Discord raises a limit before this
crate does.

```rust
use djs_builders::{EmbedBuilder, ValidationError};

# fn main() -> Result<(), ValidationError> {
let mut builder = EmbedBuilder::new()
    .title("Leaderboard")
    .color(0xFE_E75C)
    .timestamp_now();
for (rank, (name, points)) in [("Ada", 31), ("Grace", 27)].into_iter().enumerate() {
    builder = builder.inline_field(format!("#{}", rank + 1), format!("{name}: {points}"));
}
let embed = builder.build()?;
assert_eq!(embed.fields.len(), 2);

let error = EmbedBuilder::new().title("🔥".repeat(257)).build().unwrap_err();
assert_eq!(error.to_string(), "title is too long: 257 characters, at most 256 allowed");
# Ok(())
# }
```

## What is checked

The rules follow Discord's documentation and were checked on the live API with a test bot:

- **Text lengths** are counted in Unicode code points after removing leading and trailing
  whitespace, as Discord counts them. An emoji such as 🔥 is one character (discord.js counts
  two UTF-16 units); a family emoji made of seven code points is seven.
- **URLs** must start with an allowed scheme (any letter case): `http://` or `https://` for
  links, and also `attachment://` for images and icons. The rest of the address is not
  parsed; Discord rejects a malformed one.
- **Empty texts** are allowed, because Discord accepts them, but an embed needs some content
  (a non-blank title, description, footer or author name, a field, an image or a thumbnail).
- Limits that span a whole message — at most 10 embeds and 6000 characters across them —
  are checked by [`validate_embeds`]; [`embed_length`] measures one embed.

## Builders

| Builder | Builds | discord.js |
|---------|--------|------------|
| [`EmbedBuilder`] | [`Embed`](https://docs.rs/djs-model/latest/djs_model/message/struct.Embed.html) | `EmbedBuilder` |
| [`EmbedAuthorBuilder`] | `EmbedAuthor` | `EmbedAuthorOptions` |
| [`EmbedFooterBuilder`] | `EmbedFooter` | `EmbedFooterOptions` |

## Part of djs-rs

`djs-builders` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate, as `djs_rs::builders`.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
