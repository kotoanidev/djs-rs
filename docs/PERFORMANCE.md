# Performance

Every completed stage is benchmarked and stress-tested before it is considered done (D-041).
This file records the results, the machine they were measured on and every regression or
optimization candidate found. Benchmarks use [divan](https://docs.rs/divan) with its allocation
profiler (D-042). They are measured locally; CI only checks that they compile.

## How to reproduce

```bash
# Micro-benchmarks (timings + allocations per operation)
cargo bench -p djs-model

# Extreme-load tests: ignored by default, run in release mode, print `[stress]` lines
cargo test -p djs-gateway --release --test stress -- --ignored --nocapture --test-threads 1
cargo test -p djs-rest    --release --test stress -- --ignored --nocapture --test-threads 1
cargo test -p djs-rs      --release --test stress -- --ignored --nocapture --test-threads 1
```

The stress tests run their mock Discord servers (gateway and REST) inside the same process, on
the same machine and runtime. The numbers are therefore **lower bounds** for the library: the
server's work (formatting, compressing, sending) competes with the client for the same CPUs.

Memory is read from `/proc/self/status` (`VmRSS` = current, `VmHWM` = peak). The peak is reset
with `/proc/self/clear_refs` right before the measured part, so it covers only that part (Linux
only; other systems print `n/a`).

## Machine (all results below unless stated otherwise)

| | |
|---|---|
| CPU | Intel Core i7-1355U (2 P-cores + 8 E-cores, 12 threads, up to 5.0 GHz), laptop |
| Memory | 15 GiB |
| OS | Fedora 44, Linux 7.2.15, platform profile `performance` |
| Toolchain | rustc 1.99.1, `release`/`bench` profile (`lto = "fat"`, `codegen-units = 1`) |

This is a laptop CPU with turbo and mixed core types. Repeated runs vary by about ±20 %, and the
same benchmark varied by up to 2× between cold and warm runs (`values::*` below). Compare
results only from the same machine, and prefer the median over the mean.

---

## Phase 1 — vertical slice (2026-11-10)

### `djs-model` — gateway event decoding (`benches/decode.rs`)

Median of 100 samples. "Allocations" are per decoded event (alloc count / bytes).

| Benchmark | Input | Median | Throughput | Allocations |
|---|---|---:|---:|---:|
| `gateway::message_create` | 1 504 B `MESSAGE_CREATE` fixture | 1.26 µs | 1.19 GB/s | 3 / 305 B |
| `gateway::ready` | 1 094 B `READY` fixture | 918 ns | 1.19 GB/s | 4 / 307 B |
| `gateway::skip_large_unknown_event` | ≈ 219 KB `GUILD_CREATE` (1 000 members), not decoded in Phase 1 | 127 µs | 1.72 GB/s | 1 / 8 B |
| `gateway::scan_only` | top-level `op`/`s`/`t` scan of `MESSAGE_CREATE` | 35 ns | — | 0 |
| `gateway::baseline_serde_json_value` | same `MESSAGE_CREATE` into `serde_json::Value` | 2.09 µs | 0.72 GB/s | 76 / 5.95 KB |

Our typed single-pass decode is **1.65× faster** than parsing into a generic JSON value (what a
naive library does before it even builds its own structures). It makes **25× fewer
allocations** (3 vs 76) and uses **20× fewer bytes** (305 B vs 5.95 KB). Unknown events are
skipped at 1.7 GB/s with a single allocation (the event-name string).

| Benchmark | Median | Allocations |
|---|---:|---:|
| `values::id_parse` (`"80351110224678912"` → `UserId`) | 13 ns | 0 |
| `values::id_to_json` | 26 ns | 1 / 128 B (output `String`) |
| `values::timestamp_parse` (ISO 8601 → `Timestamp`) | 16 ns | 0 |
| `values::timestamp_format` (`Timestamp` → `String`) | 165 ns | 1 + 2 reallocations |
| `values::image_hash_parse` | 20 ns | 0 |

### `djs-gateway` — `Shard` under an event flood (`tests/stress.rs`)

One shard, one WebSocket connection over loopback. The time runs from `READY` to the last
dispatch and every sequence number is checked to arrive in order.

| Scenario | Events | Time | Events/s | Wire | Memory (RSS → after, peak) |
|---|---:|---:|---:|---:|---|
| `MESSAGE_CREATE` (≈ 880 B each), plain text | 1 000 000 | 1.48 s | **674 000** | 596 MB/s | 4 → 4 MiB, peak 4 MiB |
| same, `zstd-stream` | 1 000 000 | 1.26 s | **793 000** | 18 MB/s (701 MB/s decompressed) | 62 → 40 MiB, peak 64 MiB¹ |
| `GUILD_CREATE` (≈ 219 KB each), plain text | 2 000 | 323 ms | 6 200 | 1 358 MB/s | 37 → 38 MiB, peak 38 MiB |
| same, `zstd-stream` | 2 000 | 337 ms | 5 900 | 8 MB/s (1 299 MB/s decompressed) | 38 → 38 MiB, peak 38 MiB |

¹ The RSS includes the test's own precomputed compressed frames (≈ 60 MiB), which are freed as
they are sent. Peak memory therefore did not grow during the flood.

- Memory stays flat no matter how many events flow through: the shard keeps no per-event state,
  and the zstd buffer is reused and shrunk back after huge payloads.
- `zstd-stream` is *faster* than plain text on loopback because far fewer bytes go through the
  WebSocket layer. The test payloads compress unrealistically well (39× and 163×: repetitive
  synthetic data), so real-world gains will be smaller.

### `djs-rest` — `Rest` and the rate limiter under load (`tests/stress.rs`)

The mock server enforces the limits it advertises and counts every request Discord would have
rejected.

| Scenario | Requests | Time | Requests/s | Rejected (429) | Peak memory |
|---|---:|---:|---:|---:|---:|
| Throughput: 10 000 channels (10 000 buckets) × 20, 256 in flight, no limits | 200 000 | 813 ms | **246 000** | 0 | 298 MiB² |
| Sequential round-trip latency, 1 channel | 20 000 | 850 ms | 23 500 (**42.5 µs/request**) | 0 | 40 MiB |
| Bucket limits: 100 channels × 100 queued at once, server limit 10 per 100 ms | 10 000 | 983 ms | 10 200 | **0** | 28 MiB |
| 429 storm: first request of each of 1 000 channels rejected (`retry_after` 50 ms) | 5 000 (+1 000 rejected) | 337 ms | 14 900 | exactly 1 000 | 41 MiB |
| Global limit (default 50/s): 250 channels × 1, all queued at once | 250 | 4.01 s | 62 | 0 | 28 MiB |

² All 200 000 requests are spawned as tasks up front (that is the stress), and each holds a
`create_message` future of **1 024 bytes**. The peak is therefore mostly queued futures, not
library state.

- The per-bucket limiter let **no** request through that the server would have rejected, with
  10 000 requests queued at once. The run time matches the theoretical minimum (10 windows of
  100 ms).
- After a 429, each bucket retries exactly once (6 000 requests = 5 000 + 1 000 rejections), so
  there are no retry storms.
- Idle-bucket sweeping kept the 10 000-bucket run stable.

### `djs-rs` — `Client` with handlers under an event flood (`tests/stress.rs`)

The full stack: mock gateway → `Shard` → decode → `Arc` payload → handler(s). The time runs
from `login()` until every handler call has completed.

| Scenario | Events | Handler calls | Time | Events/s | Handler calls/s | Peak memory |
|---|---:|---:|---:|---:|---:|---:|
| `HandlerMode::Concurrent` (default), 1 handler | 1 000 000 | 1 000 000 | 2.49 s | **402 000** | 402 000 | 6 MiB |
| `HandlerMode::Sequential`, 1 handler | 1 000 000 | 1 000 000 | 1.85 s | **540 000** | 540 000 | 6 MiB |
| Concurrent, 10 handlers (fan-out) | 200 000 | 2 000 000 | 1.94 s | 103 000 | **1 033 000** | 9 MiB |
| Concurrent, 1 handler + `client.events()` stream consumer | 1 000 000 | 1 000 000 (+1 000 000 streamed) | 2.78 s | 360 000 | 360 000 | 7 MiB |

- A million events in through one connection with **under 10 MiB peak memory**: the handlers
  keep up, so nothing piles up.
- Fan-out shares one `Arc` payload among all handlers (no per-handler clone). The cost is one
  task spawn per handler call.

### Findings and optimization candidates (Phase 1)

1. **Global limiter allows bursts across window boundaries.** The global limiter uses fixed
   one-second windows (the same approach as discord.js). With all requests queued at once, the
   server saw **92–100 requests in some sliding one-second window** in 4 of 5 runs, i.e. two
   windows' worth (requests sent at the end of one window and at the start of the next). The
   average rate is still 50/s. Whether to switch to a strict sliding-window/GCRA limiter is
   an open question for the owner (see `docs/DECISIONS.md`). The stress test currently asserts
   the fixed-window bound (≤ 100).
2. **REST futures are 1 KiB.** `create_message`/`get_gateway_bot` futures are 1 024/1 008 bytes,
   so every queued request costs at least that much. Candidate for Phase 3: box the inner
   request state machine so a queued, not-yet-polled future stays small.
3. **`Timestamp` formatting reallocates twice** (165 ns). Candidate: format into a fixed
   32-byte stack buffer and write it with a single `write_str`. This is not on the event path
   (only for outgoing payloads).
4. **Concurrent handler mode costs about 25 %** compared with sequential mode (one task spawn
   per event, see above). This is expected; documented as the trade-off of `HandlerMode`.

No regressions: this is the first baseline.
