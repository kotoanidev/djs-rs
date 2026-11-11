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
cargo test -p djs-model   --release --test stress -- --ignored --nocapture --test-threads 1
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
   average rate is still 50/s. **Decision D-043:** switch to a strict GCRA limiter in Phase 3;
   until then the stress test asserts the fixed-window bound (≤ 100).
2. **REST futures are 1 KiB.** `create_message`/`get_gateway_bot` futures are 1 024/1 008 bytes,
   so every queued request costs at least that much. Candidate for Phase 3: box the inner
   request state machine so a queued, not-yet-polled future stays small.
3. **`Timestamp` formatting reallocates twice** (165 ns). Candidate: format into a fixed
   32-byte stack buffer and write it with a single `write_str`. This is not on the event path
   (only for outgoing payloads).
4. **Concurrent handler mode costs about 25 %** compared with sequential mode (one task spawn
   per event, see above). This is expected; documented as the trade-off of `HandlerMode`.

No regressions: this is the first baseline.

---

## Phase 2 — full `djs-model` (2026-11-11)

Same machine and toolchain (rustc 1.99.1, platform profile `performance`). Realistic payloads
for benchmarks and stress tests come from `djs-model/tests/common/payloads.rs` (shared by the
`djs-model` benchmarks and stress tests and the `djs-gateway` stress test).

### `djs-model` — A/B against Phase 1 (`benches/decode.rs`)

Phase 1 (commit `65210f8`, checked out in a git worktree) and Phase 2 were run back-to-back,
alternating, five times; the ranges are the medians of the last three runs (500 samples each).
The large unknown event is the same payload in both (Phase 1 did not decode `GUILD_CREATE`;
Phase 2 gives it an unknown name instead).

| Benchmark | Phase 1 | Phase 2 | Change | Allocations P1 → P2 |
|---|---:|---:|---:|---|
| `gateway::message_create` | 1.12–1.15 µs | 1.45–1.48 µs | **+29 %** | 3 / 305 B → 3 / 913 B |
| `gateway::ready` | 0.93–1.09 µs | 0.98–1.15 µs | +5 % | 4 / 307 B → 4 / 467 B |
| `gateway::skip_large_unknown_event` | 113–131 µs | 120–126 µs | ≈ 0 | 1 / 8 B |
| `gateway::scan_only` | 33–35 ns | 33–34 ns | 0 | 0 |
| `gateway::baseline_serde_json_value` | 1.93–1.96 µs | 1.91–1.96 µs | — | 76 / 5.95 KB |

`MESSAGE_CREATE` now decodes what Phase 1 skipped. Measured separately, `member` costs
≈ 230 ns (a `PartialMember` with roles and timestamps, in its own allocation) and `nonce`
≈ 45 ns; then come `flags`, `channel_type` and the (empty) arrays. `Message` itself grew to
656 bytes. Putting `MESSAGE_CREATE` first in the event-name `match` made no measurable
difference, so routing among the 78 events costs nothing noticeable. The typed decode is still
**1.3× faster than `serde_json::Value` with 25× fewer allocations**.

### `djs-model` — every hot event and the commands (`benches/decode.rs`)

One run, median of 300 samples (this laptop varies by ±20 %, see above).

| Benchmark | Payload | Median | Throughput | Allocations |
|---|---|---:|---:|---:|
| `events::hot::MESSAGE_CREATE` / `MESSAGE_UPDATE` | 1.5 KB fixture | 1.69–1.70 µs | 885 MB/s | 3 / 913 B |
| `events::hot::INTERACTION_CREATE` | 2.1 KB slash command, 3 options, resolved user + member | 3.10 µs | 681 MB/s | 11 / 6.68 KB |
| `events::hot::PRESENCE_UPDATE` | 2 activities | 1.59 µs | 390 MB/s | 3 / 1.81 KB |
| `events::hot::TYPING_START` | with member | 1.58 µs | 425 MB/s | 3 / 376 B |
| `events::hot::MESSAGE_REACTION_ADD` | with member | 1.14 µs | 707 MB/s | 3 / 456 B |
| `events::hot::VOICE_STATE_UPDATE` | with member | 1.75 µs (fastest 0.97 µs) | 478 MB/s | 4 / 448 B |
| `events::hot::GUILD_MEMBER_ADD` (extras path) | member + `guild_id` | 848 ns | 678 MB/s | 2 / 344 B |
| `events::hot::GUILD_MEMBER_UPDATE` | | 799 ns | 724 MB/s | 2 / 352 B |
| `events::hot::CHANNEL_UPDATE` | text channel, 2 overwrites | 880 ns | 555 MB/s | 2 / 328 B |
| `events::hot::MESSAGE_DELETE` | | 209 ns | 652 MB/s | 1 / 24 B |
| `events::guild_create_1000_members` | 711 KB: 1 000 members, 250 presences, 100 channels, 50 roles, 50 emojis | **853 µs** | 834 MB/s | 1 629 / 429 KB |
| `events::guild_create_baseline_serde_json_value` | same, into `serde_json::Value` | 2.70 ms | 264 MB/s | 53 225 / 5.05 MB |
| `events::guild_create_unavailable` | `{"id", "unavailable": true}` | 320 ns | — | 3 / 88 B |
| `commands::identify_to_json` | with a presence | 245 ns | — | 1 / 128 B |
| `commands::update_presence_to_json` | | 72 ns | — | 1 / 128 B |

A full `GUILD_CREATE` decodes **3.2× faster than into `serde_json::Value`, with 33× fewer
allocations**. The "object + extras" events (D-073) decode in a single pass as fast as plain
ones (`GUILD_MEMBER_ADD` ≈ `GUILD_MEMBER_UPDATE`).

### `djs-model` — extreme load and robustness (`tests/stress.rs`)

| Scenario | Result |
|---|---|
| 2.2 M hot events (11 kinds × 1 000 variants × 200 rounds), 1 thread | **697 000 events/s**, 627 MB/s; RSS 21 → 21 MiB |
| 13.2 M hot events, 12 threads | **3.04 M events/s**, 2.7 GB/s (4.4× one thread on 2 P + 8 E cores); RSS 20 → 21 MiB |
| `GUILD_CREATE`, 100 000 members, 25 000 presences, 500 channels, 250 roles (64.9 MB) | **102–116 ms = 560–640 MB/s**; peak RSS 154–156 MiB with the 65 MB input held |
| 2 500 `GUILD_CREATE` (200 members, 50 presences, 30 channels, 20 roles), all kept alive | 3 551 guilds/s, 566 MB/s; decoded model **462 MiB for 380 MiB of JSON (1.21×)** |

Robustness (these run with the normal test suite):

- Every prefix of 12 real payloads (≈ 23 000 prefixes) and three single-byte corruptions at
  every position of three payloads either decode or return an error, without panicking. No
  strict prefix decodes.
- 100 000 levels of nesting never overflow the stack. serde_json skips ignored data (unknown
  events, unknown fields, including through the extras adapter) iteratively, so it decodes;
  arbitrary-JSON values (`JsonValue`) stop at serde_json's recursion limit with an error.

Sizes of the main decoded types (bytes): `User` 144, `Member` 304, `Presence` 296
(`PresenceUser` 144, `ClientStatus` 96), `Activity` 336, `Channel` 200, `Role` 128, `Emoji` 72,
`VoiceState` 80, `Guild` 488, `Message` 656, `DispatchEvent` ≤ 32.

### `djs-gateway`, `djs-rs` and `djs-rest` stress tests — A/B against Phase 1

Phase 1 and Phase 2 stress tests were run alternately, twice each (ranges = both runs).
Payloads are identical, except that the large-event scenario now uses an unknown event name
(it was an undecoded `GUILD_CREATE` in Phase 1).

| Scenario | Phase 1 | Phase 2 | Change |
|---|---:|---:|---:|
| `djs-gateway`: 1 M `MESSAGE_CREATE`, plain | 627 000–730 000 /s | 542 000 /s | −15 … −26 % |
| `djs-gateway`: same, `zstd-stream` | 708 000–719 000 /s | 550 000–554 000 /s | −23 % |
| `djs-gateway`: 2 000 large unknown events (219 KB), plain / zstd | 5 493–5 608 / 5 516–5 540 /s | 5 347–5 483 / 5 444–5 468 /s | ≈ −2 % (noise) |
| `djs-gateway`: 2 000 **full** `GUILD_CREATE` (711 KB), decoded (new) | — | 863–876 /s, 614–623 MB/s; RSS 38 → 41 MiB | — |
| `djs-rs`: `Concurrent`, 1 handler | 347 000–362 000 /s | 200 000–265 000 /s | **−27 … −43 %** |
| `djs-rs`: `Concurrent` + `events()` stream | 348 000–360 000 /s | 198 000–267 000 /s | −25 … −45 % |
| `djs-rs`: `Sequential`, 1 handler | 490 000–512 000 /s | 462 000–496 000 /s | −4 % |
| `djs-rs`: fan-out, 10 handlers (handler calls/s) | 1 019 000–1 023 000 | 873 000–1 021 000 | 0 … −15 % |

`djs-rest` does not decode gateway events; its stress results are unchanged within noise
(throughput 227 000 req/s, sequential 44.4 µs/request, bucket limits 0 rejected, 429 storm
exactly 1 000 rejections, global limit ≤ 50 per sliding second).

The gateway numbers drop by the decode cost above (`member` etc.). The concurrent-mode drop in
`djs-rs` is larger than that and was traced with an experiment: **without `member` in the
stress payload, `Concurrent` is back at 347 000–353 000 events/s (Phase 1 level)**. The two
extra allocations per event (`Box<PartialMember>` and its roles `Vec`) are made on the
dispatcher thread and freed on tokio worker threads; `Sequential` mode, which frees them on the
same thread, is not affected. This matches the producer/consumer pattern that glibc malloc
handles poorly (raising `glibc.malloc.tcache_count` did not help; other allocators could not be
tried without a new dependency).

### Allocator comparison (D-080)

`djs-rs/tests/stress_mimalloc.rs` runs the same `Client` stress tests with
[mimalloc](https://docs.rs/mimalloc) as the global allocator (a dev-dependency only). Runs
alternated with the glibc runs; ranges over 2–4 runs each.

| Scenario | glibc malloc | mimalloc | Change |
|---|---:|---:|---:|
| `Concurrent`, 1 handler (events/s) | 205 000–285 000 (median 267 000) | 254 000–370 000 (median 310 000) | ≈ +15 % |
| `Concurrent` + `events()` stream (events/s) | 222 000–263 000 | 333 000–344 000 | +30 … +50 % |
| `Sequential`, 1 handler (events/s) | 500 000–504 000 | 476 000–484 000 | −4 % |
| Fan-out, 10 handlers (handler calls/s) | 885 000–893 000 | 1 601 000–1 622 000 | **+80 %** |
| Peak memory | 7–12 MiB | 16–20 MiB | +8 MiB |

The allocator explains most of the cost of freeing on other threads: with mimalloc, fan-out and
the stream are well above Phase 1 (with glibc). A single concurrent handler improves less and
varies a lot between runs (bimodal: probably which core type, P or E, the dispatcher runs on),
so the rest is the per-event task spawn and the cache misses of dropping the event on another
core. The `djs-rs` README now recommends mimalloc for very large bots; the library itself does
not depend on it.

### Findings and optimization candidates (Phase 2)

1. **Concurrent handler mode is sensitive to allocations freed on other threads**
   (−27 … −43 % for a `MESSAGE_CREATE` with a member). Real bots get far fewer events per shard,
   but this is the default mode. Measured with mimalloc (D-080, above): the allocator accounts
   for most of it, so the `djs-rs` README recommends mimalloc for very large bots. The
   remaining per-event cost (task spawn, dropping on another core) is for the dispatch design
   in Phase 8.
2. **The decoded model is about 1.2× the size of the JSON** for `GUILD_CREATE`. Big
   contributors: `Activity` (336 B), `Presence` (296 B, of which `ClientStatus` is 96 B because
   every `Option<Status>` is 24 B: string enums carry an `Unknown(CompactString)`),
   `PresenceUser` (144 B for what is usually only an ID) and the slack of growing `Vec`s. This
   matters for caching, so it is addressed in Phase 5 (D-081).
3. An unavailable `GUILD_CREATE` costs 320 ns and 3 allocations, because the failed guild
   decode builds a serde error before the extras recover. Unavailable guilds only come with
   outages, so this is acceptable.
4. `MESSAGE_CREATE` decoding is 29 % slower than in Phase 1. That is the price of the complete
   model (`member`, `nonce`, …), not a regression in the shared machinery: skipping, scanning
   and routing are unchanged.

