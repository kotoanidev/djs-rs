# djs-rs vs twilight

[twilight](https://github.com/twilight-rs/twilight) is a set of low-level Discord crates for
Rust, known for its speed and its small, separate building blocks: a gateway
(`twilight-gateway`), an HTTP client (`twilight-http`), a cache (`twilight-cache-inmemory`)
and the models (`twilight-model`). This page compares a bot built on djs-rs with the same bot
built on twilight 0.18, as twilight's documentation builds it, on **performance and
resources** (D-385). The numbers were measured on one machine, with the same inputs and the
same mock Discord servers for both libraries. Ease of use and API coverage are not compared
here. Every number comes from the scripts in
[`benches/vs-twilight`](../benches/vs-twilight/README.md), and anyone can re-run them
([Reproduce](#reproduce)). Where twilight does better, this page says so.

## At a glance

<!-- vs-twilight:summary:start -->
<!-- vs-twilight:summary:end -->

## How it is measured

- **The same input, byte for byte.** Every gateway payload and REST response comes from the
  generator of [`docs/VS_NODE.md`](VS_NODE.md)
  ([`payloads.rs`](../benches/vs-node/src/payloads.rs)), the same as for
  [serenity](VS_SERENITY.md). It is shaped like what Discord sends to bots, and every ID is
  unique.
- **The same mock Discord.** The mock gateway and the mock HTTP API run as separate
  processes pinned to their own CPU cores, so they never compete with the client under
  test. Their CPU use is recorded for every run (`server_cpu_ms` in
  [`results.json`](../benches/vs-twilight/results.json)).
- **twilight as its README writes a bot.** One `Shard`, a `DefaultInMemoryCache` updated
  with every event, and a task spawned per event for the handler. twilight's default cache
  keeps every resource type and 100 messages per channel; the djs-rs `Client` is given a
  cache with the same contents (`twilight_cache()` in
  [`lib.rs`](../benches/vs-node/src/lib.rs)). Both libraries are also measured without a
  cache, djs-rs's own default.
- **How twilight reaches the mocks.** `twilight-http` takes the mock API's address with its
  documented `proxy(host, true)` (plain HTTP). Its rate limiter stays on, with its proactive
  global limit (50 requests per second by default) lifted the way twilight documents,
  `RateLimiter::new(u16::MAX)`, as djs-rs's is lifted; the REST section also shows it with
  the rate limiter disabled, as a bound. The gateway
  is built without twilight's `zstd` feature, which always asks Discord for `zstd-stream`
  compression while the mock sends plain JSON; djs-rs is set to no compression as well.
- **The same CPUs and metrics.** Both clients run pinned to the same four CPU threads, and
  single-threaded decode loops to one core. CPU time and resident memory are read from
  `/proc` by the same code ([`metrics.rs`](../benches/vs-node/src/metrics.rs)).
- **Equally optimized.** The djs-rs runners and the twilight runners are built with the same
  release profile (`lto = "fat"`, `codegen-units = 1`). The two bots of the build section
  use the default profiles of a new project.
- **No network.** Every run happens in a network namespace with only a loopback interface,
  so nothing can reach Discord, even by mistake (C-108).

## 1. Decoding gateway events

How long it takes to turn one gateway payload (JSON text) into the library's typed event, on
one CPU core: djs-rs decodes into `djs-model` types, twilight into `twilight-model`'s
`GatewayEvent` with its `GatewayEventDeserializer`, as their shards do. `MESSAGE_CREATE` is
the most frequent event of a busy bot. `GUILD_CREATE` carries a whole guild at startup,
here with 1000 members.

<!-- vs-twilight:decode:start -->
<!-- vs-twilight:decode:end -->

## 2. Event flood through a gateway

The mock gateway sends `MESSAGE_CREATE` events as fast as one connection allows. Each event
reaches a handler of the bot (`client.on(Events::MessageCreate, …)` in djs-rs, a task
spawned for each event `shard.next_event` returns in twilight), and the cache stores what it
keeps from it.

<!-- vs-twilight:flood:start -->
<!-- vs-twilight:flood:end -->

## 3. Memory of the guilds a bot receives

The bot connects, receives guilds of 1000 members each (members, roles, channels, emojis,
presences) and caches them. Resident memory is measured once every guild is cached.

<!-- vs-twilight:memory:start -->
<!-- vs-twilight:memory:end -->

## 4. REST throughput

64 concurrent workers call Create Message (`POST /channels/{id}/messages`) against the mock
HTTP API, each in its own channel so that every request has its own rate-limit bucket. The
mock answers with a message and rate-limit headers that never run out. djs-rs's proactive
global limit (50 requests per second by default, like Discord's) is lifted, and so is
twilight's. So what is
measured is the client's own overhead: building the request, the rate limiter, HTTP and
decoding the response into a typed message.

<!-- vs-twilight:rest:start -->
<!-- vs-twilight:rest:end -->

## 5. Building a bot

The same `!ping` → `Pong!` bot
([`bots/djs-rs`](../benches/vs-serenity/bots/djs-rs/src/main.rs),
[`bots/twilight`](../benches/vs-twilight/bots/twilight/src/main.rs)), each in a project of
its own with default features, built with `cargo build` from an empty target directory on
every CPU. The second time is the rebuild after editing `main.rs`, which a bot author waits
for on every change.

<!-- vs-twilight:build:start -->
<!-- vs-twilight:build:end -->

## What these numbers do and do not mean

- **They measure the libraries, not Discord.** Over the internet, network latency dominates
  REST latency, and Discord's rate limits cap how many requests a bot may send. The numbers
  show the headroom a library leaves: how many events one CPU core handles and how much
  memory a large bot needs.
- **No payload compression.** The mock gateway does not compress, so both libraries decode
  plain JSON text; in production both default to `zstd-stream`.
- **Different shapes of library.** twilight leaves the event loop, the cache updates and the
  handler tasks to the bot; djs-rs's `Client` does them. The twilight bot here is written
  the way twilight's own README writes one.
- **One machine.** A laptop CPU with frequency scaling. Repeated runs vary by a few percent,
  and each configuration's range is in `results.json`. The ratios are stable. The absolute
  numbers depend on the machine.
- **What is not compared.** Ease of use and API coverage, voice, sharding across processes,
  reliability under faults (measured against serenity in [`VS_SERENITY.md`](VS_SERENITY.md))
  and twilight's optional features (`zlib` compression, other TLS backends). Both libraries
  run with their defaults, but for the compression noted above.

## Reproduce

On Linux with Rust (stable), `unshare` (util-linux) and Node.js 22+ (for the report):

```bash
benches/vs-twilight/run.sh      # performance, 3 runs per configuration
benches/vs-twilight/build.sh    # resources: builds of the two bots (fetch their dependencies first)
node benches/vs-twilight/report.mjs benches/vs-twilight/out   # rewrites this page and results.json
```

The CPU pinning (`CLIENT_CPUS`, `SERVER_CPUS`, `DECODE_CPU`) defaults to this machine's
layout; set them to match yours. `node benches/vs-twilight/report.mjs` re-renders the page
from [`results.json`](../benches/vs-twilight/results.json) alone.
[`benches/vs-twilight/README.md`](../benches/vs-twilight/README.md) describes the harness.

## Environment

<!-- vs-twilight:environment:start -->
<!-- vs-twilight:environment:end -->
