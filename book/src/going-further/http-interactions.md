# HTTP interactions

An app can receive its interactions (slash commands, components, autocomplete, modals) as
HTTP `POST` requests to its **Interactions Endpoint URL** instead of as gateway events.
Such a bot keeps no connection open: it wakes up for each request, so it runs in any web
server and on serverless platforms such as Cloudflare Workers. What it gives up is every
other event: no messages, members or presences reach it, only interactions.

djs-rs answers these requests with the same [`Client`], routes and `reply` methods as a
gateway bot. It has no HTTP server of its own: the server the bot already uses hands
each request's two signature headers and its body to an [`InteractionsEndpoint`], and
sends back what it returns.

This chapter's program is
[`http_interactions.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/http_interactions.rs),
served with [axum](https://docs.rs/axum). It needs the `http-interactions` feature; a bot
that never logs in to the gateway can leave the gateway out:

```toml
djs-rs = { version = "0.1.0-alpha.1", default-features = false, features = ["http-interactions", "hyper"] }
```

(`hyper` is the REST client's HTTP transport; the default features include it, together
with the gateway.)

## The routes

Nothing in the routes is specific to HTTP. This is what a gateway bot would register:

```rust,no_run
{{#include ../../code/examples/http_interactions.rs:routes}}
```

## The endpoint

[`Client::interactions_endpoint`] makes the endpoint from the app's public key (the
Developer Portal shows it on the app's *General Information* page). Its
[`handle`] does everything Discord requires of an endpoint:

```rust,no_run
{{#include ../../code/examples/http_interactions.rs:handler}}
```

1. It checks the request's Ed25519 signature, and that its timestamp is at most five
   minutes old, so a recorded request cannot be replayed. Discord tests this when the URL
   is saved: it sends forged requests too, which must be refused.
2. It answers a `PING` with a `PONG`.
3. It dispatches the interaction like a gateway `INTERACTION_CREATE`: the
   `InteractionCreate` handlers, then the routes.
4. It returns as soon as a handler gives the initial response (`reply`, `defer_reply`,
   `update`, `show_modal`, …), which becomes the HTTP response's body: no REST request is
   made for it.

| Status | When |
|---|---|
| `200` | the initial response (its JSON is the body), or the `PONG` |
| `202` | the initial response had files: it was sent through REST instead |
| `400` | the body is not an interaction |
| `401` | the signature or the timestamp is wrong |
| `500` | the handlers ended without an initial response |

`handle` also returns a future: the rest of the handlers' work, such as `/slow`'s edit
after its deferral. The server runs it after answering, here with `tokio::spawn`.

## Serving it

```rust,no_run
{{#include ../../code/examples/http_interactions.rs:main}}
```

Without a gateway there is no `READY` to register the commands in, so the program
registers them through REST when it starts. Discord only posts to an HTTPS URL that it can
reach: for a test, a tunnel such as `cloudflared tunnel --url http://localhost:8080` gives
the local server one. Saving that URL as the Interactions Endpoint URL in the Developer
Portal sends the first `PING`; from then on the app's interactions come over HTTP, not over
the gateway, until the URL is removed.

## On Cloudflare Workers

The repository's
[`examples/cloudflare-workers`](https://github.com/kotoanidev/djs-rs/tree/main/examples/cloudflare-workers)
is the same bot as a Worker, built for `wasm32-unknown-unknown`. There is no hyper there:
the REST client sends its requests through a [`Transport`] over the platform's `fetch`.

```rust,ignore
{{#include ../../../examples/cloudflare-workers/src/lib.rs:transport}}
```

Handlers must be `Send`, and JavaScript futures are not, so a handler that awaits one (a
`fetch`, a `Delay`) wraps it in `SendFuture`. tokio's timers do not run in a Worker, so
collectors are not available, and a handler waits with the platform's timers:

```rust,ignore
{{#include ../../../examples/cloudflare-workers/src/lib.rs:routes}}
```

The Worker's `fetch` handler builds the client once, passes each request to `handle`, and
keeps the Worker running for the background work with `ctx.wait_until`:

```rust,ignore
{{#include ../../../examples/cloudflare-workers/src/lib.rs:fetch}}
```

`npx wrangler dev` runs it locally (with the token and public key in `.dev.vars`), and
`npx wrangler deploy` publishes it.

## What changes without the gateway

- Only interactions reach the handlers, and the cache holds only what they carry.
- Every handler of a request runs one after another, whatever the client's
  [`HandlerMode`].
- `ctx.shard()` is the guild's shard only when the same client is also logged in to the
  gateway; otherwise it is a detached one, and gateway requests through it fail.
- A collector only sees the interactions of the same process. A platform that spreads
  requests over many instances (a Worker does) may hand the next click to another one.
- The three-second limit for the initial response holds as on the gateway: a slow command
  defers first.

## The end

That is the guide. The [API reference](../api/djs_rs/index.html) documents every type and
method, with its Discord and discord.js counterparts, and the repository's
[examples](https://github.com/kotoanidev/djs-rs/tree/main/crates/djs-rs/examples) are
complete bots. Coming from discord.js? The next page maps what you know to djs-rs.

[`Client`]: ../api/djs_rs/struct.Client.html
[`InteractionsEndpoint`]: ../api/djs_rs/http_interactions/struct.InteractionsEndpoint.html
[`Client::interactions_endpoint`]: ../api/djs_rs/struct.Client.html#method.interactions_endpoint
[`handle`]: ../api/djs_rs/http_interactions/struct.InteractionsEndpoint.html#method.handle
[`Transport`]: ../api/djs_rest/trait.Transport.html
[`HandlerMode`]: ../api/djs_rs/enum.HandlerMode.html
