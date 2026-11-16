# Your first bot

This chapter creates a bot on Discord's side, adds it to your test server and writes a
program that answers `!ping` with `Pong!`. It is small, but it has every part a bot has: a
client, the events it asks Discord for, handlers that react to them, and a clean shutdown.

## Creating the application

A bot belongs to an *application*, which you create in Discord's
[Developer Portal](https://discord.com/developers/applications):

1. Press **New Application**, give it a name (the bot's, which you can change later) and
   accept the terms.
2. Open the **Bot** page. Under **Token**, press **Reset Token** and copy the token it
   shows. It is shown only once: if you lose it, reset it again.
3. On the same page, under **Privileged Gateway Intents**, turn on **Message Content
   Intent**. The bot reads the text of messages to see `!ping`, and Discord only sends that
   text to bots that ask for it (more on intents below).

### Keep the token secret

The token is the bot's password: whoever has it can log in as your bot and do anything
it may do. Never write it into your code, never commit it, never paste it into a chat.
This guide reads it from an environment variable, `DISCORD_TOKEN`. If a token leaks,
reset it on the **Bot** page at once: the old one stops working.

## Adding the bot to your server

1. Open the **Installation** page. Under **Installation Contexts**, keep **Guild Install**.
2. Under **Install Link**, choose **Discord Provided Link**.
3. Under **Default Install Settings**, give **Guild Install** the scopes `bot` and
   `applications.commands` (the second one is for slash commands, two chapters on), and
   the permissions **View Channels**, **Send Messages** and **Read Message History** (a
   reply refers to the message it answers, which needs the history).
4. Copy the install link, open it in a browser, choose your test server and confirm.

The bot now shows in the server's member list, offline: nothing runs it yet.

## The code

Replace `src/main.rs` with this program:

```rust,no_run
{{#include ../../code/examples/first_bot.rs:all}}
```

Run it with the token in the environment. In a Unix shell:

```sh
DISCORD_TOKEN='<your token>' cargo run
```

and in PowerShell:

```powershell
$env:DISCORD_TOKEN = '<your token>'; cargo run
```

After a moment it prints the library's log lines and `Logged in as` with the bot's name,
and the bot shows as online. Send `!ping` in a channel of your server: it answers `Pong!`.
Press Ctrl+C to stop it.

The rest of this chapter goes through the program part by part.

## The client

```rust,no_run
{{#include ../../code/examples/first_bot.rs:client}}
```

A [`Client`] is the bot: it holds the token, the connections to Discord and the handlers.
[`Client::builder`] starts its configuration and `build` finishes it; the builder has more
settings (the cache, shards, the presence), which later chapters use.

The *intents* say which events the bot wants. Discord sends a bot only the events of the
intents it asks for, which saves both sides the traffic of everything else:

- `GUILDS`: the servers the bot is in, their channels and roles.
- `GUILD_MESSAGES`: messages sent in servers.
- `MESSAGE_CONTENT`: the text of those messages. It is a *privileged* intent, the one you
  turned on in the portal; without it, `msg.content` is empty for messages that do not
  mention the bot.

Asking for a privileged intent the portal does not allow makes the login fail with an
error that says so. The opposite mistake is quieter: listening to an event without its
intent means it never comes, so djs-rs logs a warning at startup for every event you listen
to without one of its intents.

(discord.js: `new Client({ intents })`.)

## Handlers

```rust,no_run
{{#include ../../code/examples/first_bot.rs:ready}}
```

[`Client::on`] registers a handler for an event, named by a constant of [`Events`]. The
handler is an async closure that receives two things: a [`Context`] (`ctx`), through which
it talks to Discord and reads the cache, and the event's data, here the [`Ready`] payload,
which says who the bot is. `ClientReady` comes once the bot is connected and ready to
work.

```rust,no_run
{{#include ../../code/examples/first_bot.rs:message}}
```

`MessageCreate` comes for every message the bot can see, including its own. This handler
answers only `!ping`, with [`reply`], which sends a message that refers to the one it
answers. Sending is a request to Discord's API, so it is `async` and can fail: `?` hands
the error back.

A handler returns `Result<(), HandlerError>`, and `?` turns any error into a
[`HandlerError`]. An error does not stop the bot: djs-rs logs it (with the event's name)
and goes on with the next events. Each event's handlers run in their own tokio tasks, so a
slow handler never holds up the others.

(discord.js: `client.on(Events.MessageCreate, …)` and `message.reply`.)

## Logging in, and out

```rust,no_run
{{#include ../../code/examples/first_bot.rs:login}}
```

[`Client::login`] connects to Discord's gateway, identifies the bot with its token and
intents, and then runs: it receives the events and calls the handlers, reconnects when a
connection drops, and returns only when the bot is shut down (`Ok`) or cannot go on, for
example with a wrong token (`Err`).

```rust,no_run
{{#include ../../code/examples/first_bot.rs:shutdown}}
```

[`Client::shutdown`] asks a running client to disconnect; `login` then returns `Ok(())`
and `main` ends. A `Client` is a cheap handle (cloning it shares the same bot), so a task
can keep a clone to stop it from elsewhere. Here that task waits for Ctrl+C.

(discord.js: `client.login(token)` and `client.destroy()`.)

## Logs

```rust,no_run
{{#include ../../code/examples/first_bot.rs:logs}}
```

djs-rs writes what it does to [`tracing`](https://docs.rs/tracing): the connection,
reconnections, warnings and a handler's errors. `tracing_subscriber::fmt::init()` prints
those lines to the terminal, at the `info` level and above by default. The `RUST_LOG`
variable chooses more or less, per crate: `RUST_LOG=djs_gateway=debug cargo run` shows
the gateway's traffic in detail.

## What's next

The bot reacts to one event. The next chapter looks at events and intents in general:
which events there are, what each needs, and what the [`Context`] gives a handler.

[`Client`]: ../api/djs_rs/struct.Client.html
[`Client::builder`]: ../api/djs_rs/struct.Client.html#method.builder
[`Client::on`]: ../api/djs_rs/struct.Client.html#method.on
[`Client::login`]: ../api/djs_rs/struct.Client.html#method.login
[`Client::shutdown`]: ../api/djs_rs/struct.Client.html#method.shutdown
[`Events`]: ../api/djs_rs/event/struct.Events.html
[`Context`]: ../api/djs_rs/struct.Context.html
[`Ready`]: ../api/djs_model/gateway/payload/incoming/struct.Ready.html
[`reply`]: ../api/djs_rs/ext/trait.MessageExt.html#method.reply
[`HandlerError`]: ../api/djs_rs/type.HandlerError.html
