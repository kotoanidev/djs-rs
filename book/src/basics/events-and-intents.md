# Events and intents

Almost everything a bot does starts with an event: a message was sent, a member joined, a
button was pressed. This chapter shows which events there are, how a handler receives
them, which intents bring them, and the two ways to consume them: handlers and a stream.

The listings come from two programs:
[`events.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/events.rs)
and
[`in_order.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/in_order.rs).

## The events

Discord's gateway sends a bot *dispatches*, one per thing that happened, and djs-rs turns
each into an event of [`Events`]: `Events::MessageCreate`, `Events::GuildMemberAdd`,
`Events::MessageReactionAdd`, and so on, one for each of discord.js's events, under the
same names. The [`event`] module lists them all, each with the intents it needs and its
discord.js name.

An event's constant also fixes the type of what its handler receives. A handler of
`Events::GuildMemberAdd` gets the member and the guild's ID:

```rust,no_run
{{#include ../../code/examples/events.rs:member}}
```

and one of `Events::MessageReactionAdd` gets the IDs of the user, the channel and the
message, and the emoji:

```rust,no_run
{{#include ../../code/examples/events.rs:reaction}}
```

If you write a field that the payload does not have, the program does not compile: the
compiler knows each event's payload, and so does your editor's completion.

### Before and after

Some events are about something that changed or went away: a message edited or deleted, a
member's roles changed, a channel renamed. For those, the handler gets both sides: `old`
is the thing as the [cache](#the-context) held it just before the event, and the payload
itself (it dereferences to the new data) is what Discord sent.

```rust,no_run
{{#include ../../code/examples/events.rs:update}}
```

`old` is `None` when the cache did not hold it: when the bot runs without a cache (the
default), when that kind of object is not cached, or when it was never seen (a message sent
before the bot started). This program turns on a cache of the last 200 messages of each
channel when it builds the client (the cache has a chapter of its own):

```rust,no_run
{{#include ../../code/examples/events.rs:client}}
```

> **From discord.js:** discord.js drops events about objects it has not cached unless you
> enable *partials*. djs-rs has no partials: every event arrives, and what was not cached
> is `None` (or an ID, where Discord sends only an ID), which you can complete with a
> request when you need it. The [`event`] module has the table of what each partial
> becomes.

## Intents

When a bot connects, it tells Discord which groups of events it wants: its *intents*. Every
event belongs to one or more of them, and Discord does not send the events of intents the
bot did not ask for. Ask only for what the bot uses: every intent is traffic that Discord
sends and the bot decodes.

The most used ones:

| Intent | Brings |
|---|---|
| `GUILDS` | servers, channels, threads, roles; almost every bot needs it |
| `GUILD_MEMBERS` | members joining, leaving and changing (privileged) |
| `GUILD_MODERATION` | bans and the audit log |
| `GUILD_EXPRESSIONS` | emojis, stickers and soundboard sounds |
| `GUILD_VOICE_STATES` | who is in which voice channel (needed for voice) |
| `GUILD_PRESENCES` | members' statuses and activities (privileged) |
| `GUILD_MESSAGES` | messages in servers |
| `GUILD_MESSAGE_REACTIONS` | reactions in servers |
| `DIRECT_MESSAGES` | messages in direct messages |
| `MESSAGE_CONTENT` | the text, embeds, attachments and components of messages (privileged) |

[`Intents`] has every one, with its events. Intents combine with `|`.

### Privileged intents

Three intents give access to data Discord considers sensitive: `GUILD_MEMBERS`,
`GUILD_PRESENCES` and `MESSAGE_CONTENT`. A bot can ask for them only after they are
turned on for it on the **Bot** page of the Developer Portal (Server Members Intent,
Presence Intent, Message Content Intent); once a bot is in 100 servers or more, Discord
also has to approve them. A bot that asks for a privileged intent it is not allowed is
refused at login, with an error that names the problem.

`MESSAGE_CONTENT` is unlike the others: it brings no events of its own, but without it the
messages of other users arrive with their text, embeds, attachments and components empty
(except in direct messages and messages that mention the bot).

### Checking the intents

Listening to an event that no intent of the client brings is a quiet mistake: the handler
is never called. djs-rs warns about it in its log when the bot logs in, and
[`Client::required_intents`] tells which intents the registered handlers need, so a bot
can check itself:

```rust,no_run
{{#include ../../code/examples/events.rs:check}}
```

## The context

Every handler receives a [`Context`] before the event's data. It is how a handler acts:

- `ctx.rest()` is the REST client, which sends requests to Discord's API (replying to a
  message uses it). The methods on the structures, like `msg.reply(&ctx, …)`, take the
  context for this reason.
- `ctx.cache()` is the cache, or `None` when the client has none.
- `ctx.shard_id()` is the shard the event came on, and `ctx.shard()` sends that shard's
  connection commands (a presence, a request for members).
- The managers, such as `ctx.guilds()`, `ctx.members(guild)` and `ctx.messages(channel)`,
  fetch and change Discord's objects, with the cache in between when there is one.

```rust,no_run
{{#include ../../code/examples/events.rs:context}}
```

A context is cheap to clone, and a clone can be moved into a task the handler spawns.

## How handlers run

By default each call of a handler runs in its own tokio task. Events are handled
concurrently: a handler that waits for a slow request holds up nothing else, and two events
can be handled at the same time, in any order. That is what most bots want, and what
discord.js does.

When the order matters more than throughput (a log that must show a message's creation
before its deletion, say), [`HandlerMode::Sequential`] makes each shard call its handlers
one at a time, in the order of its events:

```rust,no_run
{{#include ../../code/examples/in_order.rs:sequential}}
```

The price is that a slow handler delays every later event of its shard; if it takes long
enough, it delays the shard's heartbeats too and Discord drops the connection. In this
mode, handlers should finish quickly and spawn tasks for slow work.

## A stream of events

Handlers are the convenient way. For full control, [`Client::events`] gives a stream of
every event the client receives, as a [`Dispatch`] with the shard it came on:

```rust,no_run
{{#include ../../code/examples/in_order.rs:stream}}
```

Each call of `events()` creates its own stream, which receives every event from then on,
next to the handlers. A stream keeps the events its reader has not taken yet without a
limit: read it steadily, or drop it.

## What's next

Messages that start with `!` are the old way of taking commands. [The next
chapter](slash-commands.md) turns to slash commands, which Discord shows to users with their options, checks their input, and
needs no `MESSAGE_CONTENT` at all.

[`Events`]: ../api/djs_rs/event/struct.Events.html
[`event`]: ../api/djs_rs/event/index.html
[`Intents`]: ../api/djs_model/gateway/struct.Intents.html
[`Client::required_intents`]: ../api/djs_rs/struct.Client.html#method.required_intents
[`Client::events`]: ../api/djs_rs/struct.Client.html#method.events
[`Context`]: ../api/djs_rs/struct.Context.html
[`HandlerMode::Sequential`]: ../api/djs_rs/enum.HandlerMode.html#variant.Sequential
[`Dispatch`]: ../api/djs_rs/event/enum.Dispatch.html
