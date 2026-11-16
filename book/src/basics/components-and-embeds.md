# Components and embeds

Text is only part of what a message can carry. An **embed** is a framed card with a title,
a colour, fields and images; **components** are what users can click or pick in a message
(buttons and select menus) and the fields of a **modal**, the form a bot can open over the
chat. This chapter builds a `/card` command whose reply has all of them.

This chapter's program is
[`buttons_and_embeds.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/buttons_and_embeds.rs).
It registers its command like the previous chapter's program does.

## The card

[`EmbedBuilder`] makes the embed; [`ActionRowBuilder`] holds a row of components: up to
five buttons, or one select menu.

```rust,no_run
{{#include ../../code/examples/buttons_and_embeds.rs:card}}
```

- **Embeds** have a title (which `url` turns into a link), a description, a colour, up to
  25 fields (`inline_field` puts them side by side), a footer, an author, an image, a
  thumbnail and a timestamp, which each user sees in their own time zone. A message can have
  up to 10 embeds, with 6 000 characters in all.
- **Buttons** come in styles: `primary` (the accent colour), `secondary` (grey), `success`
  (green), `danger` (red), each with a *custom ID* the bot receives when it is clicked, and
  `link`, which opens a URL and sends the bot nothing.
- **Select menus** offer a list (`StringSelectMenuBuilder`, up to 25 options) or Discord's
  own pickers of users, roles, mentionables and channels (`UserSelectMenuBuilder`, …).
  `min_values` and `max_values` allow picking several.

`build` checks Discord's limits for what it builds (lengths, counts, a label or an emoji on
each button) and says which one is broken, before anything is sent; `validate_components`
checks a message's rows together, custom IDs unique across them included.

(discord.js: `EmbedBuilder`, `ActionRowBuilder`, `ButtonBuilder`, `StringSelectMenuBuilder`.)

## Sending it

Embeds and components go into the reply's data:

```rust,no_run
{{#include ../../code/examples/buttons_and_embeds.rs:reply}}
```

The same `embeds` and `components` exist on a message sent to a channel (`CreateMessage`)
and on an edit (`EditMessage`).

## Answering clicks

A click is an interaction, like a slash command, and must be answered within 3 seconds.
[`Client::component`] routes clicks by custom ID: the route `"like"` receives the custom ID
`like`, and every custom ID that starts with `like:`. What follows the `:` is the
handler's `args`, so a button can carry what its handler needs:

```rust,no_run
{{#include ../../code/examples/buttons_and_embeds.rs:buttons}}
```

The like button carries its count (`like:3`), and the handler sends the card again with the
count one higher. `update` replaces the message the button is on and answers the click in
one step; `reply` would send a new message instead, and `defer_update` answers without a
change, for a handler that edits the message later.

Keeping the state in the custom ID means nothing is lost when the bot restarts: the buttons
of a week-old message still work. A custom ID holds up to 100 characters; for more, keep a
key in it and the data in a database.

(discord.js: `client.on("interactionCreate")` with `interaction.isButton()` and a check of
`customId`; `interaction.update`.)

### Waiting for a click where it happens

When a click only matters right after a message is sent, a handler can wait for it with a
*collector* instead of a route: `message.await_component(&ctx).time(…)` returns the next
click on that message, and `message.component_collector(&ctx)` all of them until it ends.
The repository's
[`components.rs`](https://github.com/kotoanidev/djs-rs/blob/main/crates/djs-rs/examples/components.rs)
turns pages with one. A route survives a restart; a collector lives as long as its handler.

## Select menus

A select menu sends its custom ID with the picked `values`:

```rust,no_run
{{#include ../../code/examples/buttons_and_embeds.rs:select}}
```

The values are the options' values (`"vanilla"`), not their labels. Discord's user, role
and channel menus send IDs, with the users, roles or channels themselves in the
interaction's `resolved` data.

## Modals

A modal is a form with up to five fields. It can only be the first answer to an
interaction (a command or a click), and its submission arrives as an interaction of its
own, routed by the modal's custom ID with [`Client::modal`]:

```rust,no_run
{{#include ../../code/examples/buttons_and_embeds.rs:show_modal}}
```

Each field has a label and an input: `TextInputBuilder::short` for one line,
`::paragraph` for several, with lengths and a placeholder; fields are required unless set
otherwise. `text_value` reads a field by its custom ID. A user can close a modal without
sending it, and then nothing arrives; a slash command that opens a modal and waits for it
in place (`cmd.await_modal_submit`) needs a time limit for that reason.

(discord.js: `ModalBuilder`, `TextInputBuilder`, `interaction.showModal`,
`ModalSubmitInteraction#fields`.)

## What's next

The bot has been asking Discord for what it needs. The next chapter shows what the client
keeps on its own: the cache.

[`EmbedBuilder`]: ../api/djs_builders/struct.EmbedBuilder.html
[`ActionRowBuilder`]: ../api/djs_builders/struct.ActionRowBuilder.html
[`Client::component`]: ../api/djs_rs/struct.Client.html#method.component
[`Client::modal`]: ../api/djs_rs/struct.Client.html#method.modal
