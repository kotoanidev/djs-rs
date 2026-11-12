# Architecture

This document explains how the djs-rs crates fit together. It describes the **intended**
architecture; sections are refined as each phase is implemented. Binding decisions live in
[`DECISIONS.md`](DECISIONS.md).

## Layers

djs-rs is layered the same way discord.js is (D-006, D-007):

```text
                        ┌──────────────────────────────────────────┐
  your bot  ─────────▶  │ djs-rs   (Client, events, structures,    │   ≈ discord.js
                        │           managers, collectors)          │
                        └───────┬───────────────┬──────────────┬───┘
                                │               │              │
                   ┌────────────▼───┐   ┌───────▼──────┐  ┌────▼──────────┐
                   │ djs-gateway    │   │ djs-rest     │  │ djs-cache     │
                   │ (shards, ws,   │   │ (hyper, rate │  │ (DashMap,     │
                   │  compression)  │   │  limiting)   │  │  sweepers)    │
                   └────────┬───────┘   └──────┬───────┘  └──────┬────────┘
                            │                  │                 │
                            └──────────┬───────┴─────────────────┘
                                       │
                               ┌───────▼────────┐
                               │ djs-model      │  ≈ discord-api-types
                               │ (all API types)│
                               └───────┬────────┘
                                       │
                               ┌───────▼────────┐
                               │ djs-util       │  ≈ @discordjs/util
                               └────────────────┘

  Standalone helpers (no networking):
    djs-builders   ≈ @discordjs/builders    (depends on djs-model)
    djs-formatters ≈ @discordjs/formatters
    djs-collection ≈ @discordjs/collection
```

### Low level: idiomatic Rust, zero overhead

- **`djs-model`** owns every Discord API v10 type. It has no networking and no runtime
  dependency, so it can be used by anything that speaks the Discord API (bots, dashboards,
  webhook receivers, interaction HTTP servers).
- **`djs-rest`** is a standalone REST client: requests are typed, rate limits are handled
  transparently, and it can be used without a gateway connection (e.g. for HTTP-only
  interaction bots).
- **`djs-gateway`** manages WebSocket shards and yields typed events. It does not cache and
  does not call REST except to discover gateway information.
- **`djs-cache`** consumes gateway events and keeps an in-memory view of Discord state, with
  per-resource policies and a pluggable storage backend.

These crates do not depend on each other more than shown above, so users can combine only
the parts they need.

### High level: the discord.js experience

**`djs-rs`** wires the low-level crates together into a `Client` that feels like discord.js:
register handlers with `client.on(Events::…)`, receive rich structures whose methods call
REST for you (`msg.reply(&ctx, …)`), and read cached state through managers.

## Data flow of a gateway event (target design)

1. `djs-gateway` receives a compressed WebSocket frame and decompresses it
   (`zstd-stream` by default, D-015).
2. The payload is parsed with `serde_json` into a `djs-model` event, in a single pass (simd-json
   and sonic-rs were measured and not adopted, D-150).
3. `djs-cache` applies the event to the cache (if the resource is cached, D-016).
4. `djs-rs` dispatches the event to the registered handlers.

Step 2 is the hottest path in the whole library; see the performance rules in `CLAUDE.md`.

## Runtime and I/O

- **tokio** only (D-014).
- **HTTP:** `hyper` 1.x + `hyper-util` (D-010). **TLS:** `rustls` default, `native-tls`
  optional (D-012).
- **WebSocket:** `tokio-websockets` (D-011).
- **Sharding:** in-process shards as tokio tasks first; a multi-process `ShardingManager`
  later (D-018).

## Open design points

Tracked in the "Open questions" section of [`DECISIONS.md`](DECISIONS.md).
