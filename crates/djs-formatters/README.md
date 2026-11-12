# djs-formatters

Markdown, mention and timestamp formatting helpers for Discord messages.

> **Status:** pre-alpha. Every formatter and escaper of `@discordjs/formatters` is here. Track
> progress in the [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/formatters`

## How the formatters work

Each formatter returns a small value that implements [`Display`](core::fmt::Display), so
nothing is allocated until it is written: put it in `format!` or `write!`, or call
`.to_string()`. Formatters take any `Display` content, so they nest (`bold(italic("x"))` is
`**_x_**`), and mentions take the typed IDs of [`djs-model`](https://docs.rs/djs-model).
Optional parts are methods on the returned value.

```rust
use djs_formatters::{TimestampStyle, bold, code_block, slash_command_mention, time, user_mention};
use djs_model::id::{CommandId, UserId};

let winner = UserId::new(80_351_110_224_678_912);
let text = format!(
    "{} {} won {}!",
    bold("Giveaway:"),
    user_mention(winner),
    time(1_618_953_630).style(TimestampStyle::RelativeTime),
);
assert_eq!(text, "**Giveaway:** <@80351110224678912> won <t:1618953630:R>!");

let help = slash_command_mention("tag", CommandId::new(1)).subcommand("show");
assert_eq!(help.to_string(), "</tag show:1>");
assert_eq!(code_block("fn main() {}").language("rust").to_string(), "```rust\nfn main() {}\n```");
```

Formatters do not escape their content. Text from users goes through [`escape_markdown`]
first, which returns a `Cow<str>` and borrows the text when there is nothing to escape:

```rust
use djs_formatters::{EscapeOptions, bold, escape_markdown};

let name = "**not bold**";
assert_eq!(bold(escape_markdown(name)).to_string(), r"**\*\*not bold\*\***");

// Links and mentions are left alone, so they keep working.
assert_eq!(escape_markdown("https://x.dev/a_b <@1>"), "https://x.dev/a_b <@1>");

// discord.js' options, with its defaults.
let strict = EscapeOptions::new().heading(true).bulleted_list(true);
assert_eq!(strict.escape("# Title\n- item"), "\\# Title\n\\- item");
```

## Formatters

| Formatter | Output | discord.js |
|---|---|---|
| [`user_mention`], [`channel_mention`], [`role_mention`] | `<@id>`, `<#id>`, `<@&id>` | `userMention`, `channelMention`, `roleMention` |
| [`linked_role_mention`], [`GuildNavigation`] | `<id:linked-roles:id>`, `<id:browse>`, … | `linkedRoleMention`, `GuildNavigationMentions` |
| [`slash_command_mention`] `.subcommand(…)` / `.group(…, …)` | `</name sub:id>` | `chatInputApplicationCommandMention` |
| [`emoji`] `.name(…)` `.animated(…)` | `<a:name:id>` | `formatEmoji` |
| [`time`] `.style(…)`, [`TimestampStyle`] | `<t:seconds:R>` | `time`, `TimestampStyles` |
| [`bold`], [`italic`], [`underline`], [`strikethrough`], [`spoiler`] | `**x**`, `_x_`, `__x__`, `~~x~~`, `\|\|x\|\|` | the same names |
| [`inline_code`], [`code_block`] `.language(…)` | `` `x` ``, ```` ```lang ```` | `inlineCode`, `codeBlock` |
| [`quote`], [`block_quote`], [`subtext`], [`heading`] `.level(…)` | `> x`, `>>> x`, `-# x`, `## x` | the same names |
| [`unordered_list`], [`ordered_list`] `.start(…)`, [`ListItem`] | `- x`, `1. x`, nested | `unorderedList`, `orderedList` |
| [`hyperlink`] `.title(…)`, [`hide_link_embed`] | `[x](url "title")`, `<url>` | `hyperlink`, `hideLinkEmbed` |
| [`channel_link`], [`message_link`] `.guild(…)` | `https://discord.com/channels/…` | `channelLink`, `messageLink` |
| [`application_directory`] `.sku(…)` | `https://discord.com/application-directory/…` | `applicationDirectory` |
| [`email`] `.headers(…)`, [`phone_number`] | `<a@b.c?subject=…>`, `<+1…>` | `email`, `phoneNumber` |
| [`Face`] | `¯\_(ツ)_/¯`, … | `Faces` |
| [`escape_markdown`], [`EscapeOptions`], `escape_bold`, `escape_italic`, … | | `escapeMarkdown`, `escapeBold`, … |

## Escaping rules

The same options and defaults as discord.js, and the same result for ordinary text. Links
(`http://`, `https://`) and `<…>` tokens (mentions, emoji, timestamps, e-mail and phone
links) are copied as they are, so escaping never breaks them. In unusual input the rules
differ on purpose; see [`EscapeOptions`] for the full list.

## Part of djs-rs

`djs-formatters` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate, as `djs_rs::formatters`.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
