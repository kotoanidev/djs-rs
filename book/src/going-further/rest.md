# The REST API

Everything a bot *does* (sending a message, banning a member, registering a command) is a
request to Discord's REST API. The managers and the structures' methods of the earlier
chapters make those requests through the client's [`Rest`] client; this chapter uses it
directly, for the endpoints the managers do not wrap and for programs without a gateway
connection at all.

This chapter's program is
[`rest_calls.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/rest_calls.rs).
It never connects to the gateway: it needs a token and the ID of a channel the bot can
write in (enable Developer Mode in Discord's settings, then right-click the channel → Copy
Channel ID).

## A client

In a bot, `ctx.rest()` is the client's REST client. On its own, it is built from a token:

```rust,no_run
{{#include ../../code/examples/rest_calls.rs:client}}
```

The client is cheap to clone, and clones share the connection and the rate limits: keep
one and clone it into tasks. The `djs-rest` crate can be used alone, for a command-line
tool, a web dashboard or an HTTP-only interactions bot.

(discord.js: `new REST().setToken(token)` from `@discordjs/rest`; `client.rest`.)

## One method per endpoint

Every endpoint of the API is a method, named after Discord's documentation: Create
Message is `create_message`, Get Guild Member `get_guild_member`, and so on. Each takes
the IDs in the path, then the request's body or query as a type of `djs_rs::model::rest`,
and returns the decoded response:

```rust,no_run
{{#include ../../code/examples/rest_calls.rs:send}}
```

Bodies are plain structs with builder-style setters (`content`, `embeds`, `components`,
…) and `Default`; a `&str` turns into a message body. Fields that an edit can leave alone,
set or clear are a `Patch` (`Keep`, `Set(…)`, `Clear`), so an edit sends only what it
changes. Requests that show up in the server's audit log take a `reason`.

## Lists, page by page

Endpoints that list return one page. Messages come newest first, up to 100 per request,
and the query's `before` asks for the page before a given message:

```rust,no_run
{{#include ../../code/examples/rest_calls.rs:pages}}
```

Members are listed oldest first with `after`, bans and reactions the same way; the query
types document their limits.

## Rate limits

Discord limits how many requests a bot may make: per route (a *bucket*, shared for
example by all messages sent to one channel), globally (50 requests per second), and
against too many failed requests (10 000 invalid ones in 10 minutes ban the IP for a
while). The client keeps to all of them by itself:

- requests to one bucket wait in a queue until Discord's headers allow the next one;
- a `429 Too Many Requests` pauses the bucket, or every request for a global limit, and the
  request is sent again;
- server errors, timeouts and connection failures are retried (`retries`, 3 by default);
- invalid requests (401, 403, 429) are counted (`rest.invalid_requests()`), and with
  `invalid_request_limit(9_000)` the client stops sending before the ban; there is no such
  limit by default.

A bot does nothing for any of this. Hitting a limit is logged through `tracing` at the
`warn` level, which is worth watching: it usually means a loop sends more than it should.

(discord.js: the same model, in `@discordjs/rest`.)

## Errors

A failed request returns a [`djs_rest::Error`]: the request could not be sent, it timed
out, the response could not be decoded, or Discord answered with an error status. In the
last case `status()` is the HTTP status and `api_error()` the error Discord sent, with its
JSON error code:

```rust,no_run
{{#include ../../code/examples/rest_calls.rs:errors}}
```

`ErrorCode` names Discord's codes (`UnknownMessage`, `MissingPermissions`,
`MissingAccess`, …): match on them rather than on texts, which Discord may change. In a
handler, `?` passes the error on, and the client logs it with the event's name.

(discord.js: `DiscordAPIError` with `code` and `status`, and `RESTJSONErrorCodes`.)

## What's next

One connection to the gateway serves up to 2 500 servers. The next chapter splits a bigger
bot into shards.

[`Rest`]: ../api/djs_rest/struct.Rest.html
[`djs_rest::Error`]: ../api/djs_rest/struct.Error.html
