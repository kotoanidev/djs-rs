# djs-rs vs twilight — benchmark harness

The harness behind `docs/VS_TWILIGHT.md` (D-385). It measures djs-rs and
[twilight](https://github.com/twilight-rs/twilight) 0.18 on the same machine, with the same
inputs and the same mock Discord servers as [`benches/vs-node`](../vs-node/README.md) and
[`benches/vs-serenity`](../vs-serenity/README.md): decoding, an event flood, memory per
server, REST throughput, and the build of the same `!ping` bot. Linux only (`/proc`,
`taskset`, `unshare`).

It is its own Cargo workspace with its own `Cargo.lock`, so the repository's quality gate
never builds twilight.

## How it fits together

| Part | Where |
|------|-------|
| Mock gateway and plain mock API, payloads, decode fixtures | `vs-node mock-gateway`, `vs-node mock-rest`, `vs-node fixtures` |
| djs-rs runners | `vs-node decode`, `vs-node flood --cache twilight` / `--cache none`, `vs-node memory --cache twilight`, `vs-node rest` |
| twilight runners | `djs-vs-twilight decode`, `flood`, `memory`, `rest` |
| Shared measuring code | `djs_vs_node::metrics` (`Window`, `measure`, `/proc` readers) |
| Two identical `!ping` bots (build time, size, dependencies; built, never run) | `../vs-serenity/bots/djs-rs`, `bots/twilight` |

## Running

```bash
benches/vs-twilight/run.sh     # performance: decode, flood, memory, REST → out/results.jsonl
benches/vs-twilight/build.sh   # resources: builds, sizes, dependencies → out/build.jsonl
```

`build.sh` builds offline: fetch the bots' dependencies first (`cargo fetch` in each of
`../vs-serenity/bots/djs-rs` and `bots/twilight`). `out/` is git-ignored.

## Configuration

Both libraries run as their documentation shows a bot:

- **twilight**: one `Shard` from `twilight-gateway`, `DefaultInMemoryCache` from
  `twilight-cache-inmemory` and `twilight-http`'s `Client`, a spawned task per event (the
  pattern of twilight's README). Its default cache keeps every resource type and 100
  messages per channel; djs-rs is measured with the same cache (`--cache twilight`) and
  with none on both sides (`--cache none`).
- **Transport compression** is off on both sides: twilight's `zstd` feature always asks for
  `zstd-stream`, and the mock gateway sends plain JSON, so the gateway is built without
  default features (`rustls-platform-verifier`, `twilight-http` only).
- **REST**: twilight's `Client` is pointed at the plain-HTTP mock with its documented
  `proxy(host, true)`; its rate limiter is on with the proactive global limit lifted
  (`RateLimiter::new(u16::MAX)`, as djs-rs's limit is lifted; the default 50 requests per
  second would make the run last two hours), and off (`ratelimiter(None)`), as a bound.

## Safety

Every run happens inside `unshare -rn`: a network namespace with only a loopback interface,
so nothing can reach Discord even by mistake (C-108). The runners also refuse any URL that is
not `127.0.0.1`.
