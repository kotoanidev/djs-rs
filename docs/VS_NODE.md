# djs-rs vs discord.js

How much faster and leaner is a Discord bot built on djs-rs than the same bot on
[discord.js](https://discord.js.org) running on Node.js? This page answers it with numbers
measured on one machine, with the same inputs, the same mock Discord servers and the same
measuring code on both sides. Nothing here is estimated: every number comes from
[`benches/vs-node/run.sh`](../benches/vs-node/run.sh), and anyone can re-run it
([Reproduce](#reproduce)).

## At a glance

<!-- vs-node:summary:start -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/summary-dark.svg">
  <img alt="How many times better djs-rs is than discord.js — ratio per scenario (equal caches, otherwise default configurations; system allocator), higher is better. The table below has the numbers." src="vs-node/summary-light.svg">
</picture>

| Scenario | djs-rs | discord.js | djs-rs advantage |
| --- | ---: | ---: | ---: |
| Decode `MESSAGE_CREATE` | 1.40 µs | 4.75 µs | **3.4× faster** |
| Decode `GUILD_CREATE` (1000 members) | 667.78 µs | 3.20 ms | **4.8× faster** |
| Event flood, full clients, equal caches (events/s) | 251,061 | 130,653 | **1.9× more** |
| Event flood, gateway layer (events/s) | 573,140 | 245,529 | **2.3× more** |
| Memory, idle bot (RSS, connected, no guilds) | 6.2 MiB | 102.6 MiB | **17× less** |
| Memory, 500,000 members (RSS) | 203.1 MiB | 503.7 MiB | **2.5× less** |
| REST throughput (requests/s) | 231,392 | 25,621 | **9.0× more** |
| REST p99 latency | 0.71 ms | 5.92 ms | **8.3× lower** |
<!-- vs-node:summary:end -->

In short: a djs-rs bot starts at **about a seventeenth of the memory** of a discord.js bot,
decodes Discord's events **3.4–4.8× faster**, handles **1.9× the events** while caching the
same data, needs **2.5–4.6× less memory** for the same cached guilds and members, and its
REST client pushes **9× more requests** through one machine with **8× lower tail latency**.
The details, the exact configurations and the caveats follow.

## How it is measured

- **The same input, byte for byte.** Every gateway payload and REST response comes from one
  generator ([`src/payloads.rs`](../benches/vs-node/src/payloads.rs)) shaped like what
  Discord sends to bots: full member and user objects, roles, channels with permission
  overwrites, emojis. Every ID is unique, so neither library can deduplicate data that real
  Discord would send as distinct objects.
- **The same mock Discord.** The mock gateway and the mock HTTP API are a separate Rust
  process pinned to their own CPU cores, so they never compete with the client under test.
  Their CPU use is recorded for every run (`server_cpu_ms` in
  [`results.json`](../benches/vs-node/results.json)) to show they were not the bottleneck.
- **The same clients' CPUs.** Every client — Rust or Node.js — runs pinned to the same four
  CPU threads. Single-threaded decode loops run on one pinned core.
- **The same metrics.** CPU time (user + system, all threads) and resident memory (`VmRSS`,
  peak `VmHWM`) are read from `/proc` by identical code in
  [`metrics.rs`](../benches/vs-node/src/metrics.rs) and
  [`common.mjs`](../benches/vs-node/node/common.mjs); the decode timing loop (1 s warm-up,
  15 rounds of ≈ 200 ms, median) is the same on both sides.
- **Equal caches, otherwise default configurations (D-174).** discord.js always caches
  (guilds, members, users, channels, roles, emojis, the last 200 messages per channel, …);
  a djs-rs `Client` caches only when asked to. So the summary compares discord.js' defaults
  with a djs-rs `Client` given `CacheConfig::discord_js()`, which caches what discord.js
  caches by default. Everything else is as a bot gets it by default (system allocator,
  `#[tokio::main]` multi-threaded runtime, `HandlerMode::Concurrent`). Variants — djs-rs
  without a cache (its default), mimalloc, sequential handlers, discord.js without the
  message cache — are listed separately and named as such.
- **The same transport.** The gateway connection is uncompressed for both (discord.js only
  compresses when the optional `zlib-sync` package is installed; djs-rs' default
  `zstd-stream` is turned off), over loopback.
- **Pinned versions.** discord.js and its packages are pinned in
  [`package.json`](../benches/vs-node/package.json) with a committed lockfile and installed
  with `npm ci`.

## 1. Decoding gateway events

How long it takes to turn the JSON text of a gateway event into the library's objects — the
work behind every event a bot receives.

- **djs-rs** decodes into fully typed structures (`djs_model::gateway::GatewayEvent`): IDs
  become integers, timestamps are parsed, enums and flags are checked, unknown fields are
  skipped.
- **discord.js** runs `JSON.parse` and builds the structures its event handlers build
  (`new Message(client, data)` with the message's guild and channel in the cache;
  `new Guild(client, data)`, with the client-wide user and channel caches emptied after each
  call so every call builds its users and channels from scratch, as on a real first
  `GUILD_CREATE`).
- **JSON.parse only** is the floor for any Node.js library: V8's native parser producing
  untyped objects, without any discord.js code.

<!-- vs-node:decode:start -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/decode-message-dark.svg">
  <img alt="Decoding MESSAGE_CREATE (1 KiB) — payloads per second on one core, higher is better · ratios vs discord.js · median of 15 rounds. The table below has the numbers." src="vs-node/decode-message-light.svg">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/decode-guild-dark.svg">
  <img alt="Decoding GUILD_CREATE, 1000 members (508 KiB) — payloads per second on one core, higher is better · ratios vs discord.js · median of 15 rounds. The table below has the numbers." src="vs-node/decode-guild-light.svg">
</picture>

| Payload | Implementation | Time per payload | Payloads/s | vs discord.js |
| --- | ---: | ---: | ---: | ---: |
| `MESSAGE_CREATE` | djs-rs (djs-model) | 1.40 µs | 716,143 | **3.4×** |
| `MESSAGE_CREATE` | djs-rs (djs-model) + mimalloc | 1.42 µs | 705,441 | **3.3×** |
| `MESSAGE_CREATE` | Node.js: JSON.parse only | 2.17 µs | 460,656 | **2.2×** |
| `MESSAGE_CREATE` | discord.js: JSON.parse + new Message() | 4.75 µs | 210,594 | baseline |
| `GUILD_CREATE` | djs-rs (djs-model) + mimalloc | 665.71 µs | 1,502 | **4.8×** |
| `GUILD_CREATE` | djs-rs (djs-model) | 667.78 µs | 1,498 | **4.8×** |
| `GUILD_CREATE` | Node.js: JSON.parse only | 1.05 ms | 953 | **3.1×** |
| `GUILD_CREATE` | discord.js: JSON.parse + new Guild() | 3.20 ms | 312 | baseline |
<!-- vs-node:decode:end -->

- djs-rs decodes into strict Rust types **faster than V8's native `JSON.parse` can build
  untyped JavaScript objects** (1.6× faster for both payloads) — before discord.js has done
  any of its own work. discord.js then spends more than half of its time
  building its structures on top.
- The gap grows with the payload: large `GUILD_CREATE`s (4.8×) are where Rust's compact,
  allocation-light types pay off most.

## 2. Event flood through a gateway

A bot connects to the mock gateway, which sends `READY`, one 1000-member `GUILD_CREATE` with
the channel, and then a flood of `MESSAGE_CREATE` events (authors cycling through the
members) as fast as the client reads them. The window runs from the first event that reaches
the bot's code to the last; CPU time is that of the whole client process in the window.

- **Full client:** a djs-rs `Client` with one `Events::MessageCreate` handler vs a discord.js
  `Client` with one `messageCreate` listener. With a cache, every message is cached before
  the handler runs (in djs-rs as in discord.js; the flood's one channel keeps its latest 200),
  with its author and member.
- **Gateway layer only:** the libraries the clients are built on — a djs-gateway `Shard`
  yielding typed events vs an @discordjs/ws `WebSocketManager` emitting parsed JSON.

<!-- vs-node:flood:start -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/flood-throughput-dark.svg">
  <img alt="MESSAGE_CREATE flood through a mock gateway — events per second, higher is better · 1 connection, 4 client CPUs · median of runs. The table below has the numbers." src="vs-node/flood-throughput-light.svg">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/flood-cpu-dark.svg">
  <img alt="CPU time per event during the flood — microseconds of CPU (all threads) per event, lower is better. The table below has the numbers." src="vs-node/flood-cpu-light.svg">
</picture>

| Layer | Implementation | Events/s | vs Node.js | CPU per event | vs Node.js | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| full client | djs-rs Client, sequential handlers, no cache | 524,797 | **4.0×** | 1.88 µs | **4.3×** less | 7.5 MiB |
| full client | djs-rs Client, sequential handlers + cache like discord.js | 441,259 | **3.4×** | 2.24 µs | **3.6×** less | 8.1 MiB |
| full client | djs-rs Client + mimalloc, no cache | 305,718 | **2.3×** | 6.13 µs | **1.3×** less | 21.1 MiB |
| full client | djs-rs Client (default: no cache) | 274,314 | **2.1×** | 6.95 µs | **1.2×** less | 9.0 MiB |
| full client | djs-rs Client + cache like discord.js | 251,061 | **1.9×** | 7.22 µs | **1.1×** less | 9.0 MiB |
| full client | discord.js Client, message cache off | 137,796 | **1.1×** | 7.68 µs | **1.1×** less | 129.2 MiB |
| full client | discord.js Client (default) | 130,653 | baseline | 8.15 µs | baseline | 126.3 MiB |
| gateway layer | djs-gateway Shard | 573,140 | **2.3×** | 1.72 µs | **2.4×** less | 6.8 MiB |
| gateway layer | @discordjs/ws WebSocketManager | 245,529 | baseline | 4.16 µs | baseline | 120.1 MiB |
<!-- vs-node:flood:end -->

- **Full clients caching the same data: 1.9× the throughput** of discord.js and 14× less
  peak memory (9 MiB vs 126 MiB) while doing it. Without a cache (its default), the djs-rs
  client handles 2.1× the events.
- **The cache costs djs-rs 8 % of its throughput with concurrent handlers** (274 000 →
  251 000 events/s) and 16 % with sequential ones (525 000 → 441 000): about 0.35 µs per
  message to cache it, evict the oldest of its channel and update its member.
- **The gateway layers alone: 2.3× the throughput at 2.4× less CPU per event** — and the
  djs-gateway `Shard` delivers fully typed events, while @discordjs/ws delivers parsed JSON
  that discord.js still has to turn into structures.
- **An honest weak spot:** djs-rs' default `HandlerMode::Concurrent` runs every handler call
  in its own tokio task. Under a flood on a multi-threaded runtime, each spawn wakes an idle
  worker thread, so the cached client burns **7.2 µs of CPU per event — barely less than
  discord.js (8.2 µs)**, even though it delivers 1.9× the events. With
  `HandlerMode::Sequential` (handlers run in event order on the shard's task, which suits
  fast handlers) the same cached client handles **3.4× the events of discord.js at 3.6× less
  CPU per event**. Making the concurrent mode as cheap is an open optimization item.
- discord.js barely gains from turning its message cache off (1.05×): its cost is in the
  event pipeline and structure building, not in the cache.

## 3. Memory of the guilds a bot receives

At startup Discord sends a `GUILD_CREATE` for every guild of the bot; with the guild members
intent it contains the members. The mock gateway sends 100 or 500 guilds of 1000 members each
(plus 20 channels, 10 roles and 20 emojis per guild), and the resident memory of the process
is read once all have arrived. "Connected, no guilds" is the idle footprint.

- **discord.js** keeps everything in its default caches (guilds, members, users, channels,
  roles, emojis); the garbage collector runs before measuring.
- **djs-rs** is a `Client` with `CacheConfig::discord_js()`: the same guilds, members,
  users, channels, roles, emojis and voice states, in `djs-cache`. Without a cache (its
  default) a djs-rs bot keeps none of it; a bot that only needs some of it can cache only
  that.

<!-- vs-node:memory:start -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/memory-dark.svg">
  <img alt="Memory after receiving the guilds at startup — resident memory (RSS) of the whole process, lower is better · Node.js after a full GC. The table below has the numbers." src="vs-node/memory-light.svg">
</picture>

| Guilds | Implementation | RSS | vs discord.js | RSS added per member | JS heap used |
| --- | ---: | ---: | ---: | ---: | ---: |
| connected, no guilds | djs-rs Client + cache like discord.js | 6.2 MiB | **17×** less | — | — |
| connected, no guilds | djs-rs Client + cache like discord.js + mimalloc | 14.3 MiB | **7.2×** less | — | — |
| connected, no guilds | discord.js Client (default caches) | 102.6 MiB | baseline | — | 16.7 MiB |
| 100 guilds · 100,000 members | djs-rs Client + cache like discord.js | 43.6 MiB | **4.6×** less | 391 B | — |
| 100 guilds · 100,000 members | djs-rs Client + cache like discord.js + mimalloc | 62.9 MiB | **3.2×** less | 509 B | — |
| 100 guilds · 100,000 members | discord.js Client (default caches) | 198.5 MiB | baseline | 1,019 B | 91.5 MiB |
| 500 guilds · 500,000 members | djs-rs Client + cache like discord.js | 203.1 MiB | **2.5×** less | 413 B | — |
| 500 guilds · 500,000 members | djs-rs Client + cache like discord.js + mimalloc | 229.1 MiB | **2.2×** less | 450 B | — |
| 500 guilds · 500,000 members | discord.js Client (default caches) | 503.7 MiB | baseline | 843 B | 382.6 MiB |
<!-- vs-node:memory:end -->

- **Idle, a connected djs-rs bot is 6.2 MiB; a discord.js bot is 103 MiB** before it has
  received a single guild — the price of the V8 engine, the Node.js runtime and the library
  code loaded at startup. For bots run in containers or on small VPSes this is the most
  visible difference.
- **Per member, the djs-rs cache needs 391–413 bytes and discord.js 843–1,019 bytes** (RSS,
  after a full garbage collection on the Node.js side). At 500,000 members that is 203 MiB vs
  504 MiB.
- An earlier version of this page kept the decoded guilds in a `HashMap` as a stand-in for
  the cache (373–383 bytes per member) and said the cache could only need the same or less.
  That was wrong: a cache needs lookup structures (a map per resource, an index of each
  guild's members) and one allocation per cached member and user, which a `Vec` of decoded
  members does not. The compact cached types (D-177) brought it from 546 to about 410 bytes.
- mimalloc trades some memory for speed here (it keeps per-thread free lists); the system
  allocator is the leaner choice for memory-bound bots.

## 4. REST throughput

64 concurrent workers call Create Message (`POST /channels/{id}/messages`) against the mock
HTTP API, each in its own channel so that every request has its own rate-limit bucket; the
mock answers with a message and rate-limit headers that never run out. The proactive global
limit (50 requests per second by default in both libraries, like Discord's) is lifted in both,
so what is measured is the client's own overhead: building the request, the rate limiter,
HTTP and decoding the response (typed `Message` in djs-rest, parsed JSON in @discordjs/rest).
A warm-up round opens the connections first.

<!-- vs-node:rest:start -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/rest-throughput-dark.svg">
  <img alt="REST: concurrent Create Message calls against a mock API — requests per second, higher is better · 64 in flight, 4 client CPUs. The table below has the numbers." src="vs-node/rest-throughput-light.svg">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="vs-node/rest-latency-dark.svg">
  <img alt="REST: latency of each call under that load — milliseconds from the call to the decoded response, lower is better. The table below has the numbers." src="vs-node/rest-latency-light.svg">
</picture>

| Implementation | Requests/s | vs Node.js | p50 | p99 | CPU per request | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| djs-rest | 231,392 | **9.0×** | 0.26 ms | 0.71 ms | 14.3 µs | 12.1 MiB |
| djs-rest + mimalloc | 194,978 | **7.6×** | 0.30 ms | 0.96 ms | 15.7 µs | 32.2 MiB |
| @discordjs/rest | 25,621 | baseline | 2.12 ms | 5.92 ms | 50.0 µs | 216.1 MiB |
<!-- vs-node:rest:end -->

- **9.0× the requests per second at 3.5× less CPU per request, with 8.3× lower p99
  latency** and an 18× smaller memory footprint.
- The mock API was given eight CPU threads and used 3.8 of them on average during the djs-rest
  runs (0.48 during the @discordjs/rest runs), so it was not saturated; the djs-rest figure
  is, if anything, a lower bound.
- Against the real Discord API the global rate limit caps a bot long before either library
  is busy; what carries over is the CPU each request costs (14.3 µs vs 50.0 µs) and the
  memory the client needs.

## What these numbers do and do not mean

- **They measure the libraries, not Discord.** Over the internet, TLS and network latency
  dominate REST latency, and Discord's rate limits cap how many requests a bot may send. The
  numbers show the headroom a library leaves: how many events or requests one CPU core can
  handle, how much memory a large bot needs and how long a slow moment lasts.
- **One machine.** A laptop CPU with frequency scaling; repeated runs vary by a few percent
  (each configuration's range is in `results.json`). The ratios are stable across runs; the
  absolute numbers depend on the machine.
- **Same work, different guarantees.** djs-rs validates and types every field it decodes;
  discord.js builds rich JavaScript objects. Neither shortcut is taken on either side.
- **What is not compared.** Features djs-rs does not have yet — the high-level structures
  beyond messages, slash-command builders, voice — are not part of this comparison. Sharding
  is not compared either: both connect one shard here. The page will grow as features land.

## Reproduce

On Linux with Rust (stable), Node.js 22+ and npm:

```bash
benches/vs-node/run.sh            # ≈ 7 minutes, 3 runs per configuration
RUNS=1 benches/vs-node/run.sh     # quicker, noisier
```

The script builds the harness, installs the pinned npm packages with `npm ci`, runs every
scenario and rewrites the tables and charts on this page and
[`results.json`](../benches/vs-node/results.json). The CPU pinning
(`CLIENT_CPUS`, `SERVER_CPUS`, `DECODE_CPU`) defaults to this machine's layout; set them to
match yours. `node benches/vs-node/node/report.mjs` re-renders the page from `results.json`
alone. [`benches/vs-node/README.md`](../benches/vs-node/README.md) describes the harness.

## Environment

<!-- vs-node:environment:start -->
|  |  |
| --- | ---: |
| Date | 2026-11-12 |
| CPU | 13th Gen Intel(R) Core(TM) i7-1355U (12 threads), 15 GiB RAM |
| OS | Linux 7.3.3-200.fc44.x86_64 |
| Rust | `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, release profile (`lto = "fat"`, `codegen-units = 1`) |
| Node.js | v22.23.1, discord.js 14.28.1, @discordjs/ws 1.2.3, @discordjs/rest 2.6.3 |
| CPU pinning | clients on CPUs 0-3, mock servers on CPUs 4-11, decode loops on CPU 2 |
| Runs | 3 per configuration (the median run is shown; decode: median of 15 rounds) |
<!-- vs-node:environment:end -->
