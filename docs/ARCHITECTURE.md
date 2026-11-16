# Architecture

How the djs-rs crates fit together and what happens inside them. The reasons behind each
choice are in [`DECISIONS.md`](DECISIONS.md) (the `D-NNN` references below); the
measurements in [`PERFORMANCE.md`](PERFORMANCE.md).

## The crates

djs-rs is layered like the discord.js ecosystem (D-006, D-007): low-level crates usable on
their own, and one crate, `djs-rs`, that joins them into a discord.js-like `Client`.

```text
┌─────────────────────────────────────────────────────────────────────┐
│ djs-rs: Client, events, managers, structure methods,                │
│ interaction routers, collectors, sharding                           │
└─────────────────────────────────────────────────────────────────────┘
        │                 │                 │                 │
┌───────────────┐ ┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│ djs-gateway   │ │ djs-rest      │ │ djs-cache     │ │ djs-voice     │
│ shards,       │ │ HTTP, rate    │ │ stores,       │ │ calls, DAVE,  │
│ compression,  │ │ limits, CDN,  │ │ limits,       │ │ Opus, player  │
│ IDENTIFY queue│ │ OAuth2        │ │ sweepers      │ │ (`voice`)     │
└───────────────┘ └───────────────┘ └───────────────┘ └───────────────┘
        │                 │                 │                 │
┌─────────────────────────────────────────────────────────────────────┐
│ djs-model: every Discord API v10 type                               │
└─────────────────────────────────────────────────────────────────────┘

Beside them, also on djs-model: djs-builders and djs-formatters; djs-collection depends
on nothing. djs-rs re-exports them all.
```

| Crate | discord.js counterpart | Depends on (djs) |
|---|---|---|
| `djs-model` | `discord-api-types` | — |
| `djs-rest` | `@discordjs/rest` | model |
| `djs-gateway` | `@discordjs/ws` | model |
| `djs-cache` | the caches of discord.js' managers | model |
| `djs-voice` | `@discordjs/voice` | model |
| `djs-builders` | `@discordjs/builders` | model |
| `djs-formatters` | `@discordjs/formatters` | model |
| `djs-collection` | `@discordjs/collection` | — |
| `djs-util` | `@discordjs/util` | — (an empty placeholder, D-218) |
| `djs-rs` | `discord.js` | all of the above but util |

The low-level crates do not depend on each other: `djs-gateway` does not call REST (the
shard count comes from `djs-rs`, D-141), `djs-cache` does not know the gateway (it is given
events), `djs-voice` does not know the client (`djs-rs` plugs it in, D-315). A program takes only the parts it needs: `djs-rest` alone for a script or an
HTTP-only interaction bot, `djs-model` alone for a webhook receiver.

## Runtime and I/O

- **tokio** only (D-014).
- **HTTP:** `hyper` 1.x and `hyper-util` (D-010), HTTP/1.1 and HTTP/2.
- **WebSocket:** `tokio-websockets` (D-011), for the gateway and the voice gateway.
- **TLS:** `rustls` with the `ring` provider and the bundled `webpki-roots`; the
  `native-roots` feature adds the system's certificates (D-012, D-027, D-100).
- **JSON:** `serde_json` (D-013). simd-json and sonic-rs were measured and not adopted
  (D-150).

## A gateway event, from the socket to a handler

This is the library's hottest path; every step was benchmarked (`docs/PERFORMANCE.md`,
Phases 3–5 and 8).

1. **Receive.** Each shard is a poll-driven state machine (D-037) running in a tokio task
   of its own. It reads a WebSocket frame and, with transport compression, inflates it
   into a buffer it reuses: `zstd-stream` by default, `zlib-stream` (pure-Rust `zlib-rs`)
   as an option (D-015, D-140).
2. **Decode.** `GatewayEventDeserializer` reads the envelope's `op` and `t` and decodes `d`
   straight into the right `djs-model` type, in one pass and without an intermediate
   `serde_json::Value`. Strings that are usually short are `CompactString`s, stored inline
   (D-024); an event the library does not know is skipped and reported as `Unknown` with
   its name instead of failing.
3. **Cache.** If the client has a cache, it applies the event by reference (D-161) and
   returns the state it replaced, typed per event (D-153, D-163); only the resources the
   configuration lists are kept (D-152, D-156).
4. **Dispatch.** `djs-rs` looks the event's handlers up in a table indexed by event kind and
   runs them: by default each event in a task of its own, so a slow handler holds nothing
   up; `HandlerMode::Sequential` keeps the order instead (D-028, D-288). The same event
   reaches `Client::events` streams (D-029), the collectors waiting for it, found through an
   index by message, channel or interaction ID rather than by a scan (D-264), and, for an
   interaction, the router (D-251).

Handlers get a `Context`: the REST client, the cache, the managers and the shard the event
came from. A payload is put behind an `Arc` once and shared by every handler, stream and
collector that receives it.

## REST

`djs_rest::Rest` sends typed requests: each endpoint is a method taking a typed body and
returning a typed response. Underneath, every request:

1. finds its **bucket** from its route and major parameter, and waits behind the requests
   already queued in it, in order;
2. waits for the **global limit** (50 per second by default) and any global pause after a
   global 429;
3. is sent on a pooled HTTP/2 or HTTP/1.1 connection. A 429 is retried after the time
   Discord asks for; a 5xx, a transport error or a timeout a few times (`retries`).

The same crate builds CDN URLs (`Cdn`, D-137) and covers OAuth2 (D-131 to D-133).
`djs-rs`'s managers sit on top of it: `fetch` reads the cache first and writes what REST
returns back into it (D-230), and deletions are left for the gateway's event to apply
(D-236).

## The cache

`djs_cache::Cache` keeps one `Store` per resource (guilds, channels, members, messages, …),
created by a `Backend` (D-155); the default backend is a `DashMap` per resource, with
`foldhash` keys (D-158). Each resource has a `Limit`: disabled, a maximum, or unlimited
(D-156). By default a client caches nothing (D-170); `CacheConfig::default()` keeps guilds,
channels and roles, `CacheConfig::discord_js()` what discord.js does (D-152). Reads return
`Arc<T>` (D-151). Members and users are stored in compact types of their own (D-177);
sweepers remove old entries on a schedule the client runs (D-167).

## Sharding

**In one process** (D-141, D-146): `djs-gateway` creates the shards and gives them a shared
IDENTIFY queue that respects Discord's `max_concurrency`; by default the client asks Discord
how many shards to run (`Shards::Auto`). Commands sent to a shard wait in its queue when
they would exceed Discord's send limit, with room reserved for heartbeats (D-142).

**In several processes** (feature `sharding`, D-284): a `ClusterManager` runs the same
executable again for each cluster of shards (D-296), restarts the ones that crash (D-298),
and is the IDENTIFY queue of all of them (D-299). Manager and clusters talk over TCP on
`127.0.0.1` with JSON frames and a secret token (D-285). Questions to every cluster are
typed commands compiled into the bot (`ClusterCommand`, D-308), not code sent as text.

## Voice

`djs-voice` (feature `voice`, D-315) implements Discord's voice protocol itself, with
established crates for the hard parts (D-311):

- the **voice gateway** (a WebSocket per call) negotiates the session;
- **UDP** carries RTP packets, encrypted with AES-256-GCM or XChaCha20-Poly1305 (RustCrypto,
  D-314);
- **DAVE**, Discord's end-to-end encryption, through the `davey` crate (D-312, D-333);
- **Opus** through `opus2` with a bundled libopus (D-350).

`VoiceManager` holds one `Call` per guild (D-319). A call's player sends a 20 ms Opus frame
on a timer from its current track: Ogg/Opus is sent without re-encoding, PCM and FFmpeg
inputs are encoded on the way (D-317). Receiving decrypts and decodes only the users a call
is subscribed to.

## Where to read the code

Every crate's `src/lib.rs` starts with the crate's overview (its `README.md`). The path of
an event: the shard in `djs-gateway/src/shard.rs`, the decoder in
`djs-model/src/gateway/event.rs`, the cache in `djs-cache/src/update.rs`, the client's
shard tasks in `djs-rs/src/client.rs` and the conversion to handler payloads in
`djs-rs/src/dispatch.rs`. The rate limiter is `djs-rest/src/ratelimit.rs`.
