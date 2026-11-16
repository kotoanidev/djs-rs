# Slash commands

A slash command is a command Discord itself knows about: when a user types `/`, Discord
lists the bot's commands with their descriptions, asks for their options with the right
kind of input (a user picker, a number, a list of choices), checks the input, and sends the
bot an *interaction* with it. The bot needs no `MESSAGE_CONTENT` intent and parses nothing.

This chapter's program is
[`commands.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/commands.rs).
It needs the bot to be installed with the `applications.commands` scope, which the first
chapter's install link includes.

## Two steps: register, then answer

A command exists in two places, and a bot takes care of both:

1. **Registering** tells Discord the command's name, description and options. It is a
   request to Discord's API, made once and kept by Discord: the commands stay registered
   while the bot is offline.
2. **Answering** happens each time a user runs the command: Discord sends the interaction
   and the bot replies to it.

## Defining commands

[`SlashCommandBuilder`] describes a command, and an option builder each of its options:

```rust,no_run
{{#include ../../code/examples/commands.rs:define}}
```

- A **name** is lowercase, without spaces, at most 32 characters; a **description** at most
  100. Each option has the same two.
- **Option kinds**: strings, integers, numbers, booleans, users, roles, channels,
  mentionables (a user or a role) and attachments, each with a builder (`SlashCommandStringOption`, …) and the
  limits that make sense for it (`max_length`, `min_value`, …).
- **Required** options come before the optional ones.
- **Choices** turn an option into a fixed list: `/echo times` offers "Once", "Twice" and
  "Three times", and the bot receives 1, 2 or 3.
- **Subcommands** split a command into several (`/info user`, `/info server`); a command
  with subcommands cannot be run without one.

`build` checks all of Discord's rules (lengths, characters, the order of the options, at
most 25 options or choices) and returns a [`ValidationError`] that says which one a
command breaks, before anything is sent.

(discord.js: `SlashCommandBuilder` and the same option builders.)

## Registering them

Commands are registered in one of two scopes:

- **Guild commands** belong to one server and update instantly: the scope for developing
  and testing.
- **Global commands** are available in every server that has the bot (and, if you allow
  it, in direct messages): the scope for a released bot. A bot can have 100 global slash
  commands.

Registering needs the bot's application ID, which the client knows once it is ready, so the
program registers in a `ClientReady` handler:

```rust,no_run
{{#include ../../code/examples/commands.rs:register}}
```

`ctx.guild_commands(guild).set(…)` replaces all the bot's commands in that server with the
given list; an empty list removes them (the program does that with `-- --remove`).
`ctx.commands().set(…)` does the same for the global commands. Commands left unchanged keep
their identity, and only new ones count toward Discord's limit of 200 command creations per
server and day, so setting the same list at every start is fine.

## Answering: the router

Interactions arrive as `Events::InteractionCreate`, whatever the intents. A handler of that
event could look at each interaction's command name, but the client has a *router* for it:
[`Client::command`] sends the interactions of one command to one handler.

```rust,no_run
{{#include ../../code/examples/commands.rs:ping}}
```

The handler receives a [`CommandInteraction`] (`cmd`), with the command's data and the ways
to answer it; `cmd.reply` sends the answer. Anything that converts into a message's data
can be the reply: a `&str`, a `String`, or a [`MessageCallbackData`] with more in it.

Every interaction must be answered **within 3 seconds**, or Discord shows the user that
the bot did not respond. A reply is the usual answer; a deferred one (below) buys time.

(discord.js: `interaction.reply`; a `switch` on `interaction.commandName` instead of the
router.)

## Options

The interaction carries the options the user gave, read by name with a getter for their
kind: `get_string`, `get_integer`, `get_number`, `get_boolean`, `get_user`, `get_channel`,
`get_role`, `get_attachment`… Each returns an `Option`, `None` when the user left an
optional option out (a required option is always there).

```rust,no_run
{{#include ../../code/examples/commands.rs:echo}}
```

Two things in this reply:

- **`ephemeral`** makes the answer visible only to the user who ran the command, with a note
  that only they see it. Use it for errors, settings and anything private.
- **`no_mentions`** keeps the text from pinging anyone. `/echo` repeats what a user wrote,
  so without it anyone could make the bot ping `@everyone`. Whenever a reply contains text
  a user chose, turn mentions off.

(discord.js: `interaction.options.getString` and the other getters, `ephemeral` as a
message flag, `allowedMentions`.)

## Subcommands

A route's name is the command as Discord shows it, with single spaces, so each subcommand
can have a route of its own:

```rust,no_run
{{#include ../../code/examples/commands.rs:subcommands}}
```

`cmd.interaction()` is the whole interaction: who ran the command (`author`), where
(`guild_id`, `channel_id`), and more. `/info user` falls back to the user who ran it when
the option is left out; `/info server` fetches the server through the guild manager.

A route for `"info"` alone would receive the subcommands that have no route of their own.

## Taking longer: deferred replies

When an answer takes more than 3 seconds to prepare (a slow API, a computation), answer
first with a *deferred* reply: Discord shows "Bot is thinking…" and the interaction stays
open for 15 minutes.

```rust,no_run
{{#include ../../code/examples/commands.rs:defer}}
```

`edit_reply` replaces the "thinking" message with the real answer (it also edits a normal
reply), and `follow_up` sends more messages after it, for as long as the interaction is
open. Pass `MessageFlags::EPHEMERAL` to `defer_reply` to make the answer ephemeral.

(discord.js: `deferReply`, `editReply`, `followUp`.)

## Autocomplete

Choices are fixed when the command is registered, and limited to 25. For a longer or
changing list, an option can ask for *autocomplete*: while the user types, Discord sends
the bot what was typed so far, and the bot answers with up to 25 suggestions.

```rust,no_run
{{#include ../../code/examples/commands.rs:autocomplete}}
```

[`Client::autocomplete`] routes those requests like `command` routes commands. The answer
must come within 3 seconds, and cannot be deferred. The suggestions only help: the user
can still send anything, so the command's handler checks the value it receives.

(discord.js: `interaction.respond` in an `isAutocomplete()` branch.)

## Putting it together

```rust,no_run
{{#include ../../code/examples/commands.rs:client}}
```

Run the program, type `/` in your server and the five commands appear with their options.

## What's next

Replies so far are text. The next chapter makes them richer: embeds, buttons and select
menus, and the modals that ask a user to fill in a form.

[`SlashCommandBuilder`]: ../api/djs_builders/struct.SlashCommandBuilder.html
[`ValidationError`]: ../api/djs_builders/enum.ValidationError.html
[`Client::command`]: ../api/djs_rs/struct.Client.html#method.command
[`Client::autocomplete`]: ../api/djs_rs/struct.Client.html#method.autocomplete
[`CommandInteraction`]: ../api/djs_rs/router/struct.CommandInteraction.html
[`MessageCallbackData`]: ../api/djs_model/interaction/struct.MessageCallbackData.html
