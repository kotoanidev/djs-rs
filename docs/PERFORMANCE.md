# Performance

Every completed stage is benchmarked and stress-tested before it is considered done (D-041).
This file records the results, the machine they were measured on and every regression or
optimization candidate found. Benchmarks use [divan](https://docs.rs/divan) with its allocation
profiler (D-042). They are measured locally; CI only checks that they compile.

The side-by-side comparison with discord.js on Node.js (D-123) is a separate page:
[`VS_NODE.md`](VS_NODE.md).

## How to reproduce

```bash
# Micro-benchmarks (timings + allocations per operation)
cargo bench -p djs-model   # gateway decoding (decode) and REST bodies (encode)
cargo bench -p djs-rest    # routes, requests, CDN and OAuth2 URLs
cargo bench -p djs-cache   # applying events, reads, sweeps, foldhash vs SipHash

# Extreme-load tests: ignored by default, run in release mode, print `[stress]` lines.
# For A/B comparisons on a hybrid CPU, pin both builds to the same cores (taskset -c 0,2; C-072)
cargo test -p djs-model   --release --test stress -- --ignored --nocapture --test-threads 1
cargo test -p djs-gateway --release --test stress -- --ignored --nocapture --test-threads 1
cargo test -p djs-rest    --release --test stress -- --ignored --nocapture --test-threads 1
cargo test -p djs-rs      --release --test stress -- --ignored --nocapture --test-threads 1
cargo test -p djs-cache   --release --test stress -- --ignored --nocapture --test-threads 1
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

---

## Phase 3 — full `djs-rest` (2026-11-12)

Same machine as above, now on Linux 7.3.3 (rustc 1.99.1, platform profile `performance`).
Every documented REST route has a typed method (`scripts/route_coverage.py`), plus OAuth2
token exchange, the CDN URL builder and lobbies. New benches: `djs-model/benches/encode.rs`
(request bodies) and `djs-rest/benches/request.rs` (routes, requests, URLs); new stress tests:
multipart uploads.

```bash
cargo bench -p djs-model --bench encode
cargo bench -p djs-rest
```

### `djs-model` — request bodies (`benches/encode.rs`)

Medians; allocations per operation from divan's `AllocProfiler`.

| Benchmark | Median | Throughput | Allocations |
|---|---:|---:|---|
| `CreateMessage::from("Pong!")` → JSON | 38.6 ns | | 1 (128 B) |
| `CreateMessage` with 3 embeds (≈ 800-character descriptions) → JSON | 1.19 µs | | 1 + growth |
| `CreateGuildScheduledEvent` with a weekly recurrence → JSON | 416 ns | | 1 + growth |
| `ModifyGuildScheduledEvent` with `Patch::Clear` and two fields → JSON | 171 ns | | 1 |
| `ImageData` 256 KiB → base64 data URI in JSON | 179 µs | **1.46 GB/s** | 1 + 10 grows |
| `ImageData` 10 MiB → JSON | 7.2 ms | 1.45 GB/s | 1 + 15 grows |
| `SoundData` 512 KiB → JSON | 360 µs | 1.46 GB/s | 1 + 11 grows |
| `FileUpload::target_users` with 10 000 user IDs (CSV) | 93 µs | | 2 + growth |

### `djs-rest` — building requests and URLs (`benches/request.rs`)

| Benchmark | Median | Allocations |
|---|---:|---|
| `Route::path`, one ID (`/channels/{id}/messages`) | 44.9 ns | 1 (64 B) |
| `Route::path`, two IDs (scheduled event users) | 101 ns | 1 |
| `Route::path`, percent-encoded Unicode reaction | 129 ns | 1 |
| `Route::path`, 68-character webhook token | 179 ns | 1 |
| `Request::new(..).json(&CreateMessage).reason(..)` | 31.6 ns | 2 (152 B) |
| `Cdn::user_avatar`, animated hash, `size=256` | 75.3 ns | 1 (122 B) |
| `AuthorizeUrl::code(..)` with 3 scopes, permissions and state | 194 ns | 5 (478 B) |

Building a request is well under a microsecond; it is noise next to the network round trip
(≈ 40 µs to a local server, tens of milliseconds to Discord).

### `djs-rest` — extreme load (`tests/stress.rs`), A/B against Phases 1 and 2

Three runs (ranges). The mock server now counts request bodies while they stream in instead of
buffering them, so the peak memory of the upload tests is the client's.

| Scenario | Phase 1 | Phase 2 | Phase 3 |
|---|---:|---:|---:|
| Throughput: 200 000 requests, 10 000 buckets, 256 in flight | 246 000 /s | 227 000 /s | 236 000–238 000 /s |
| Peak memory of that run (200 000 queued tasks) | 298 MiB | — | 381–385 MiB |
| Sequential round trip, 1 channel | 42.5 µs | 44.4 µs | 39.7–42.3 µs |
| Bucket limits: 10 000 queued, 10 per 100 ms | 0 rejected | 0 rejected | **0 rejected**, 986–990 ms |
| 429 storm, 1 000 channels | exactly 1 000 | exactly 1 000 | **exactly 1 000**, 15 000–20 000 /s |
| Global limit 50/s, 250 queued | ≤ 50 per s | ≤ 50 per s | **≤ 50 per s**, 4.21 s |
| **New:** 64 uploads of an 8 MiB file, 8 in flight (512 MiB) | | | **3 300–4 000 MiB/s**, peak **+4–5 MiB** |
| **New:** 50 000 uploads of a 1 KiB file, 256 in flight | | | **203 000–206 000 /s** |
| `create_message` future size | 1 024 B | — | 1 448 B |

Other futures: `get_gateway_bot` 1 432 B, `exchange_code` 1 472 B, `update_target_users`
1 656 B, `search_guild_messages` 1 248 B, `create_lobby` 1 440 B.

- **Uploads are zero-copy.** 64 × 8 MiB went through with 4–5 MiB of extra peak memory: file
  data is shared `Bytes` from the `FileUpload` to the socket, never copied per request or per
  retry attempt.
- The multipart path costs about as much as a JSON body (≈ 205 000 vs ≈ 237 000 requests/s, with
  bigger bodies). The 50 000-upload run's +86 MiB peak is its 50 000 tasks spawned up front.
- The rate-limit guarantees are unchanged under load: no 429 got through, every 429 is retried
  exactly once, and the global limit holds in every sliding second.

### Findings and optimization candidates (Phase 3)

1. **The request future grew from 1 024 B to 1 448 B** (+41 % since Phase 1; retries, invalid
   request limit, multipart bodies, query strings), which is why 200 000 queued requests peak
   at ≈ 383 MiB instead of 298 MiB. Requests are rarely queued by the hundred thousand, but a
   smaller state machine (e.g. keeping the retry loop's locals in one struct, or boxing the
   rarely-used error paths) would cut it. Candidate for a measured change.
2. **JSON bodies grow from 128 bytes.** `serde_json::to_vec` starts small, so a 10 MiB image
   is copied through 15 reallocations (1.2–1.5× the output in extra copies). Base64 still runs at
   1.45 GB/s; reserving the encoded size first could save part of that for image and sound
   uploads only. Low priority: those uploads are rare and network-bound.
3. Request building allocates only its output, except `AuthorizeUrl` (5 allocations: its
   scopes, state and redirect URI, then the URL); it is not on a hot path.

### Strict global limiter (D-089)

The fixed one-second windows of Phase 1 were replaced with a sliding log. `global_limit_holds`
(250 requests to 250 channels queued at once, default limit 50/s), five runs:

| | Phase 1 (fixed windows) | Phase 3 (sliding log) |
|---|---:|---:|
| Max requests the server saw in any sliding second | 92–100 in 4 of 5 runs | **50 in 5 of 5 runs** |
| Time for 250 requests | 4.01 s | 4.21 s |

The extra 0.2 s is the 50 ms offset by which every window is extended (so that latency jitter
cannot squeeze 51 requests into Discord's second); a burst of 50 still goes out at once.

## Phase 4 — full `djs-gateway` (in progress, 2026-11-12)

Same machine (Intel Core i7-1355U, 12 threads, Linux 7.3.3, rustc 1.99.1). Results are added
step by step; the full Phase 4 benchmarks and extreme-load tests follow in step 6.

### `zlib-stream` vs `zstd-stream` (`djs-gateway/tests/stress.rs`, D-140)

`zlib-stream` uses `flate2` with the pure-Rust `zlib-rs` backend. The mock gateway compresses
like the real one — one stream per connection, flushed after every payload, with the
parameters read from the first frame Discord's gateway sends (C-071): **zstd with a 64 KiB
window** (`windowLog` 16, level 3 here) and **zlib at the best level** with its 32 KiB window.
`--release --ignored`, each test in its own process:

| Flood | Compression | Events/s | Ratio | Decompressed | Wire bytes |
|---|---|---:|---:|---:|---:|
| 1M `MESSAGE_CREATE` | plain | 508 000 | 1× | — | 884 MB |
| | `zlib-stream` | 526 000 | 29.3× | 465 MB/s | 30.1 MB |
| | `zstd-stream` | 582 000 | 38.7× | 515 MB/s | 22.8 MB |
| 2k ~200 KB unknown events | plain | 6 130 | 1× | — | 438 MB |
| | `zlib-stream` | 4 200 | 36.5× | 919 MB/s | 12.0 MB |
| | `zstd-stream` | 5 900 | 71.1× | 1 293 MB/s | 6.2 MB |

- zlib decodes 10 % fewer small events and 30 % fewer large ones per second than zstd, and
  sends 1.3× (small) to 2× (large) more bytes; zstd stays the default (D-015). zlib still keeps
  up with plain text on loopback for small events.
- zlib holds less memory per shard (see "Many shards" below): ~45–60 KiB against zstd's
  ~140–170 KiB once the window has filled.
- A payload split over several WebSocket messages is buffered until Discord's sync-flush
  suffix; the common single-message case inflates straight from the message (no copy).
- An earlier run used zstd level 3 with its default 2 MiB window and zlib level 6, which
  overstated zstd's ratio on large events (163×) and its memory per shard (2.2 MiB); the
  numbers above replace it.

Reproduce:

```bash
cargo test --release -p djs-gateway --all-features --test stress -- --ignored --nocapture \
  --exact flood_1m_message_creates_zstd   # one test per run, see the names in tests/stress.rs
```

### JSON parsers: simd-json and sonic-rs (step 5, D-013, D-143, D-150)

`djs-model/benches/simd.rs` decodes the same payloads with the same `GatewayEventDeserializer`
seed through `serde_json` and through `simd-json` 0.19.0 (serde deserializer, reused
`Buffers`, including the copy into a mutable buffer that in-place parsing needs).

Method: on this laptop CPU, timings drift by ±15 % between runs (turbo, heat), and divan runs
one group after the other. The final numbers therefore alternate the two groups
(`--bench serde_json` / `--bench simd_json`), pinned to one core with `taskset`, and keep each
case's best median over 3–4 alternations. simd-json ratio (`< 1` = simd-json faster):

| Payload | P-core (CPU 2) | P-core (CPU 3) | E-core (CPU 6) |
|---|---:|---:|---:|
| `MESSAGE_CREATE` (1.5 KB fixture) | 0.87 | 0.91 | 0.89 |
| `MESSAGE_UPDATE` | 0.85 | 0.93 | 0.89 |
| `MESSAGE_CREATE`, 2 000 chars + 3 embeds (8 KB) | 0.94 | 0.98 | 0.97 |
| `INTERACTION_CREATE` | 0.93 | 0.99 | 1.00 |
| `PRESENCE_UPDATE`, `TYPING_START`, `READY`, member, voice, reaction, channel events | 0.94–1.04 | 0.97–1.03 | 0.93–1.03 |
| `MESSAGE_DELETE` (smallest) | 1.19 | 1.19 | 1.18 |
| `GUILD_CREATE` (1 000 members, 430 KB) | 1.00 | 1.02 | 1.01 |
| unknown 200 KB event (skipped) | 1.59 | 1.63 | 1.40 |

- simd-json wins 7–15 % (≈ 0.14 µs) on message events only; most events are within noise,
  tiny payloads lose 18 %, and skipping a large unhandled event costs 40–63 % more (simd-json
  indexes the whole document before serde sees it, while `serde_json` skips by scanning).
- Peak memory for the `GUILD_CREATE` is 3.9 MB instead of 0.85 MB (simd-json's tape).
- Decoding through simd-json's `BorrowedValue` DOM is 1.2–4× slower than serde_json: dropped.
- `-C target-cpu=native` (AVX2) does not change the picture (ratios 0.85–1.19, skip 1.55).
- An earlier single run had suggested 10–22 % on `MESSAGE_CREATE`; it was largely noise from
  running the groups at different times.

**sonic-rs 0.5.10** (D-150), same method, three groups alternated in rotating order (ratio to
serde_json, `< 1` = faster):

| Payload | P-core | E-core | P-core, `target-cpu=native` |
|---|---:|---:|---:|
| `MESSAGE_CREATE` / `MESSAGE_UPDATE` | 1.32–1.34 | 1.58–1.59 | 1.03 |
| `MESSAGE_CREATE`, 2 000 chars + 3 embeds | 1.01 | 1.03 | 1.04 |
| `READY` | 1.33 | 1.55 | 1.07 |
| other hot events | 0.97–1.12 | 1.13–1.38 | 0.97–1.10 |
| `MESSAGE_DELETE` | 1.28 | 1.02 | 1.31 |
| `GUILD_CREATE` (1 000 members) | 0.99 | 1.18 | 1.01 |
| unknown 200 KB event (skipped) | 1.08 | 0.99 | 1.12 |

In the same runs simd-json stayed at 0.89–0.92 on message events and 1.00–1.12 elsewhere
(1.40–1.69 when skipping).

**Conclusion:** neither parser clearly beats serde_json for gateway payloads — the decode time
goes mostly into building the typed values (IDs, timestamps, strings), not into tokenizing,
and serde_json skips unhandled data by scanning. serde_json stays the only parser; the
experiment (`benches/simd.rs`, dev-dependencies `simd-json` and `sonic-rs`) was removed and
lives in commit `af58a18` for re-runs with future versions:

```bash
git show af58a18:crates/djs-model/benches/simd.rs   # plus the two dev-dependencies
taskset -c 2 target/release/deps/simd-<hash> --bench serde_json   # alternate the groups
```

### `djs-gateway` API (`benches/gateway.rs`)

| Operation | Time | Allocations |
|---|---:|---:|
| `MessageSender::command(&UpdatePresence)` (serialize + queue) | 144 ns | 1 |
| `MessageSender::update_presence` (also kept for new sessions) | 203 ns | 2 |
| `MessageSender::command(&RequestGuildMembers)` | 119 ns | 1 |
| `LocalIdentifyQueue::wait_turn`, free bucket | 100 ns | 1 |
| `create_shards`: 1 / 100 / 1 000 shards | 173 ns / 8.9 µs / 117 µs | 5 per shard |

Sizes (`type_sizes`): `Shard` 2 032 B (mostly the inline TLS WebSocket connection),
`MessageSender` 16 B, `Config` 248 B, the `next_event` future 3 952 B.

### Many shards in one process (`tests/stress.rs`, D-141)

`create_shards` with a `LocalIdentifyQueue` of one bucket per shard (so all identify at once),
one task per shard, a mock gateway serving every connection in the same process, Discord-like
compression parameters; each test in its own process. Memory is the whole process (client
shards and mock server connections):

| Shards × events each | Compression | Ready in | Idle per shard | Events/s (all shards) | Process RSS after |
|---|---|---:|---:|---:|---:|
| 1 000 × 1 000 | plain | 1.5–2.3 s | 21 KiB | 2.37–2.45 M | 30 MiB |
| 1 000 × 1 000 | `zlib-stream` | 1.5–2.3 s | 44 KiB | 2.44–2.45 M | 73 MiB |
| 1 000 × 1 000 | `zstd-stream` | 1.5–2.3 s | 54 KiB | 2.62 M | 169 MiB (peak 191) |
| 100 × 10 000 | plain | 16 ms | 40 KiB | 2.47 M | 25 MiB |
| 100 × 10 000 | `zlib-stream` | 10 ms | 64 KiB | 2.53 M | 31 MiB |
| 100 × 10 000 | `zstd-stream` | 10 ms | 75 KiB | 2.66 M | 42 MiB |

- Sharding scales across cores: 1 000 shards decode ~2.5 M events/s together (one shard:
  ~0.5–0.6 M/s), with every shard's sequence order checked.
- Starting 1 000 shards (TCP, WebSocket upgrade, Hello, IDENTIFY, READY) takes 1.5–2.3 s.
- Once its window has filled, a zstd shard holds ~140–170 KiB (64 KiB window plus block
  buffers), a zlib shard ~45–60 KiB, a plain shard ~20 KiB.

### Send limit in real time (`send_limit_in_real_time`, D-142, ~62 s)

230 member requests queued at once while the server sends a `MESSAGE_CREATE` every 100 ms:

| Measure | Result |
|---|---|
| Commands in any 60 s (as the server received them) | **115** (Discord allows 120) |
| First 115 commands | within 1.1 ms of READY |
| Command 116 | 61.00 s after the first |
| Events received while commands waited | 611, lag p50 190 µs, p99 326 µs, max 410 µs |

Held commands never delay incoming events.

### `djs-rs` — `Client` with automatic sharding (`tests/stress.rs`, D-146)

1M `MESSAGE_CREATE` in total, one counting handler; with 16 shards the client goes through Get
Gateway Bot (mock REST recommending 16 shards, `max_concurrency` 16), as a bot does by default.
Each test in its own process:

| Run | Events/s | vs 1 shard | Peak memory |
|---|---:|---:|---:|
| 1 shard, `Concurrent` | 211 000–219 000 | — | 10 MiB |
| 16 shards, `Concurrent` | **1 109 000** | 5.1× | 156 MiB |
| 1 shard, `Sequential` | 372 000–409 000 | — | 7 MiB |
| 16 shards, `Sequential` | **2 159 000** | 5.5× | 9 MiB |
| 1 shard, 10 handlers (fan-out) | 96 000 (961 000 handler calls/s) | — | 10 MiB |
| 1 shard, handler + `events()` stream | 214 000 | — | 11 MiB |

Shards run in their own tasks and scale across the 12 hardware threads.

### A/B against the start of Phase 4 (`d9cffea`) — explained with `perf`

The same floods built from `d9cffea` (git worktree) and from this commit, run alternately:

| Measure | Before Phase 4 | Now | Change |
|---|---:|---:|---:|
| `djs-gateway` 1M plain flood, events/s | 554 000–638 000 | 490 000–523 000 | −8 … −20 % |
| — shard thread CPU per event (`/proc/thread-self/schedstat`) | 1 664–1 742 ns | 1 692–1 826 ns | +1 … +5 % |
| — whole process CPU (shard + mock server) | 2.00–2.09 s | 2.10–2.13 s | +2.5 % |
| `djs-rs` `Sequential`, events/s | 403 000–578 000 (bimodal) | 382 000–409 000 | −5 … −30 % |
| — whole process CPU | 2.2–2.8 s | 2.8–3.0 s | +10 … +25 % |
| `djs-rs` `Concurrent`, events/s | 211 000–302 000 | 211 000–260 000 | 0 … −25 % |

- Bisecting the gateway flood over the Phase 4 commits: the send limit (`8d105f7`) −8 %, the
  IDENTIFY queue (`b0c9d23`) −7 %, presence ±0, zlib (`477df45`, not used by a plain flood)
  −4 %. The shard's own CPU per event grew only 30–80 ns, so most of the throughput loss on
  loopback comes from how the shard and the mock server (same process) are scheduled — the
  "before" runs switch between a fast and a slow mode, the current ones stay in the slow one.
- Running a lone shard directly on the `login` task instead of a spawned one did not change
  the `djs-rs` numbers (tried, reverted).
- The table above is unpinned; the profiling below shows it mostly measures where the OS
  puts the threads, not the code.

**Profiling (`perf` 7.3, both builds with `debug = true` and frame pointers, `cycles:u` /
`instructions:u`):**

- **Fast and slow mode = P-cores and E-cores.** In unpinned `djs-rs` sequential floods, the
  fast "before" runs (565 000–585 000 events/s) spend 7.3–7.4 G cycles on P-cores and
  0.8 G on E-cores; the slow ones (and the current build, 353 000 events/s) spend 7.0 G on
  E-cores and 3.2 G on P-cores. `perf stat` shows the same split in instructions: a fast run
  executes 41 G instructions, a slow one 63 G (in either build) — the extra work is idle
  runtime workers spinning on the slower cores, not djs-rs code.
- **Pinned to the same cores, the builds execute the same work:**

  | Pinned to | Flood | Before Phase 4 | Now | Instructions (whole process) |
  |---|---|---:|---:|---:|
  | CPU 0–3 (2 P-cores + HT) | `djs-rs` sequential | 531 000–558 000 | 510 000–530 000 | 28.1 G vs 28.1–28.2 G |
  | CPU 4–11 (8 E-cores) | `djs-rs` sequential | 319 000–325 000 | 305 000–310 000 | 28.1 G vs 28.1–28.2 G |
  | CPU 0 + 2 (2 P-cores, no HT) | `djs-rs` sequential | 532 000–571 000 | 537 000–577 000 | — |
  | CPU 0 + 2 | `djs-gateway` plain | 554 000–611 000 | 518 000–565 000 | 27.30 G vs 27.35–27.37 G |

  With the shard and the mock server on separate P-cores, the `Client` flood shows **no
  regression**. The gateway flood keeps a 3–8 % gap: +50–70 instructions per event (+0.2 %)
  and +1.5–3 % shard thread CPU (25–50 ns of ~1 600 ns).
- **No hot spot.** A per-symbol diff of the shard thread (pinned, 4 kHz sampling) spreads the
  difference over serde_json and model deserializers that Phase 4 did not touch (±75 M
  cycles each way, i.e. code layout after fat LTO); `Shard::step` itself costs the same
  (218 vs 195 M cycles).
- The likely source of the extra instructions is the sixth `select!` branch in `Shard::step`
  (the held command). Disabling the empty branches with `select!` preconditions instead of
  polling `optional(None)` was tried: no change in instructions or throughput (reverted).
- **Conclusion:** Phase 4 costs at most 1.5–3 % shard CPU on the hottest path; the larger
  unpinned swings are P-core/E-core placement on this hybrid CPU. A/B runs on this laptop
  must pin both builds to the same cores (C-072).

## Phase 5 — `djs-cache` (2026-11-12)

Step 6 of Phase 5 (D-041). The micro-benchmarks ran pinned to one P-core (`taskset -c 2`), the
stress tests to CPU 0–3 (2 P-cores with hyperthreading) unless marked "unpinned" (all 12
CPUs). Each micro-benchmark was run two or three times; ranges are the medians of the runs.

**Correction:** the first version of this section gave 424 B per member. That run measured
the startup after other tests in the same process, whose freed memory the allocator handed
to the cache, so the RSS grew less than the cache's size. `startup_500_guilds` now measures
each configuration in a child process (a fresh heap): 546 B.

### Applying events (`benches/cache.rs`, `update`)

A cache configured like discord.js (`CacheConfig::discord_js()`: everything, 200 messages per
channel) holding the event's guild (1000 members with presences, 20 channels, 10 roles) and
a full message channel. The events are decoded before the timed part.

| Event | Time | Allocations | What it does |
|---|---:|---:|---|
| `GUILD_CREATE`, 1000 members, discord.js config | 289–320 µs | 6 182 (1.57 MB) | guild, channels, roles, emojis, members, users, presences, voice states |
| `GUILD_CREATE`, 1000 members, default config | 4.9–5.2 µs | 65 | guild, channels, roles only |
| `MESSAGE_CREATE`, channel at its limit | 228 ns | 3 | insert, evict the oldest; author shared with users |
| `MESSAGE_CREATE`, messages not cached (default) | 2.6 ns | 0 | reads the configuration |
| `MESSAGE_UPDATE` | 38 ns | — | replaces the cached message |
| `MESSAGE_DELETE` | 27 ns | — | |
| `MESSAGE_REACTION_ADD` | 100 ns | — | member upsert + the message's reactions |
| `PRESENCE_UPDATE` | 94–106 ns | 3 (1 KB) | copies two activities |
| `GUILD_MEMBER_ADD` | 75 ns | — | |
| `GUILD_MEMBER_UPDATE` | 84–103 ns | 2 (208 B) | |
| `VOICE_STATE_UPDATE` | 149–159 ns | — | voice state + its member |
| `CHANNEL_UPDATE` | 66–68 ns | — | |
| `TYPING_START` (D-173) | 62–86 ns | — | member upsert |
| `INTERACTION_CREATE` (D-173) | 154–163 ns | — | user, member, resolved user and member |

- **The cost of copying events (D-161).** Updates borrow the event and copy what the cache
  keeps. Cloning a whole decoded `GUILD_CREATE` takes 134–141 µs (3 050 allocations), about
  half of applying it (289–320 µs, plus 627 µs to decode it, `VS_NODE.md`); cloning a
  `MESSAGE_CREATE` 117 ns, a `PRESENCE_UPDATE` 129 ns, a `GUILD_MEMBER_UPDATE` 62 ns. Taking
  events by value could save at most that copy — about a fifth of decode + cache for a
  `GUILD_CREATE`, less for the hot events, where the map work dominates.
- **A cache that is off costs nothing:** a disabled resource is one configuration check
  (2.6 ns), and the default configuration applies a 1000-member `GUILD_CREATE` in 5 µs.

### Reads and sweeps (`read`, `sweep`)

| Operation | Time |
|---|---:|
| `guilds().get`, `members(g).get`, `presences(g).get` | 14–16 ns |
| `messages(c).get`, a missing member | 11.6 ns |
| `channels().in_guild(g)` (20 channels) | 26–32 ns |
| `members(g).values()` (1000 `Arc`s) | 14–17 µs |
| `sweep_members`, 1000 scanned, none removed | 16.8–17.5 µs (17 ns each) |
| `sweep_messages_older_than`, 200 scanned, none removed | 3.6–4.1 µs |
| `sweep_messages_older_than`, 2000 removed from 10 channels | 204–243 µs (≈ 110 ns each) |

### foldhash vs SipHash (D-158)

The same cache with stores hashing with the standard library's SipHash-1-3
(`std::hash::RandomState`), through the public `Backend` trait (`SipBackend` in the bench;
the internal indexes keep foldhash in both).

| Operation | foldhash (default) | SipHash | SipHash slower by |
|---|---:|---:|---:|
| `members(g).get` | 15.2–16.5 ns | 19.9–22.2 ns | 30–34 % |
| `GUILD_MEMBER_UPDATE` | 84–90 ns | 97–110 ns | 16–22 % |
| `PRESENCE_UPDATE` | 94–106 ns | 101–135 ns | 7–28 % |
| `GUILD_CREATE`, 1000 members | 289–320 µs | 357–358 µs | 12–24 % |

The claim behind D-158 holds: foldhash makes lookups about a third faster and whole updates
15–20 % faster. Discord's IDs are not attacker-chosen keys, so SipHash's DoS resistance buys
nothing here (D-158).

### Extreme load (`tests/stress.rs`)

| Test | Result |
|---|---|
| Startup: 500 guilds × 1000 members (unique users), discord.js config, own process | **after D-177: 155 ms (3.2 M members/s, 0.31 ms per guild); +199 MiB RSS, 418 B per member**; before: 166–199 ms, +260 MiB, 546 B (member, user, channels, roles, emojis, voice states, indexes; no presences) |
| Same, members + users only | after D-177: 147 ms, +194 MiB, **406 B per member**; before: 176–182 ms, 534 B |
| Same, default config | 4 ms; +5 MiB (11 B per member: guilds, channels, roles only) |
| Message churn: 2 M `MESSAGE_CREATE`s over 1000 channels, 200 per channel | 1.7–2.1 M messages/s on 1 thread, 3.0–3.8 M/s on 4 (including cloning each event); always 200 000 cached; RSS flat from the first round |
| Sweeping under load: 4 writers + a sweep every millisecond, 2 s | 2.0–2.4 M inserts/s, 606–852 sweeps, nothing left after the last; no deadlock |
| Guild churn: 300 × join + leave of a 1000-member guild | 218–226 ms; RSS flat (65.8–65.9 MiB every 50 rounds); nothing left in any resource |
| Type sizes (x86_64) | `CachedGuild` 440 B, `CachedMember` 96 B (was 176 B), `CachedUser` 104 B (new; `User` is 144 B), `CachedMessage` 512 B, `CachedPresence` 152 B, `Channel` 200 B, `Role` 128 B, `VoiceState` 80 B, `Cache` 1 696 B (guarded by `type_sizes`) |

**Concurrent readers and writers** (one 10 000-member guild; writers apply member and
presence updates, readers look members and presences up; millions per second):

| Writers + readers | Pinned to 4 CPUs, before → after the fix below | Unpinned (12 CPUs), before → after |
|---|---|---|
| 1 + 0 | 4.95 → 4.78 updates | 4.79 → 4.25 updates |
| 0 + 1 | 55.9 → 55.5 reads | 55.0 → 53.4 reads |
| 2 + 2 | 4.80 / 42.7 → 4.21 / 40.4 | 3.70 / 62.4 → 4.51 / 52.1 |
| 4 + 4 | **0.66** / 63.5 → **1.96** / 54.7 | 6.02 / 69.4 → 5.73 / 67.3 |
| 2 + 8 | **0.03–0.07** / 66.6 → **0.04–0.11** / 64.4 | 2.69 / 107.7 → 2.81 / 101.8 |
| 8 + 2 | 4.18 / 38.2 → 4.90 / 32.7 | 6.60 / 31.6 → **9.43** / 36.5 |

### `djs-rs` — `Client` flood with and without a cache (`djs-rs/tests/stress.rs`)

1 000 000 `MESSAGE_CREATE`s through one shard and one handler, pinned to CPU 0–3; the cached
runs use `CacheConfig::discord_js()` (every message cached, 200 per channel — the flood's
one channel evicts its oldest on every event —, with its author and member). Two runs each.

| Handlers | Without a cache | With discord.js' default cache | Cost of the cache |
|---|---:|---:|---:|
| `Sequential` | 487 000–490 000 events/s | 415 000–422 000 events/s | −14 % (+0.34 µs per event) |
| `Concurrent` (default) | 258 000–259 000 events/s | 243 000–245 000 events/s | −6 % |

Peak RSS stays at 7–8 MiB with the cache: 200 messages, one user and one member.

### Findings and optimization candidates (Phase 5)

- **Fixed: updates of one guild queued behind each other.** `Scoped::insert` held the
  guild's index entry exclusively while writing the store, so every update of a guild's
  members or presences took the same lock. An update of a key the guild already has now
  takes the entry's lock shared (re-inserting never moves a key in the FIFO order, so the
  index is unchanged; removals still lock it exclusively); `Index::insert` checks with a
  shared lock first. Writers now scale (8 writers unpinned: 6.6 → 9.4 M updates/s), and
  single-threaded updates got faster (`MESSAGE_CREATE` at the limit 314 → 228 ns,
  `GUILD_MEMBER_UPDATE` 109 → 84–103 ns, `VOICE_STATE_UPDATE` 177 → 149–159 ns;
  `GUILD_CREATE` unchanged within noise, 284 → 289–320 µs) (C-078).
- **Open: writers starve when readers outnumber CPUs.** With 10 threads on 4 CPUs, 2 writers
  next to 8 busy readers manage only 0.04–0.11 M updates/s (unpinned: 2.8 M). DashMap's
  `RwLock` lets readers in whenever no writer holds it, and a reader preempted while holding
  a shard's read lock (≈ 10 ns per `get`, millions of times per second) blocks a writer of
  that shard for a whole time slice. No lock order fixes lock-holder preemption; a backend
  with lock-free reads would. A tokio bot runs one worker per CPU and reads far less than
  64 M times per second, so this needs oversubscription and synthetic read rates — but it
  is recorded as an open question (a lock-free `Backend`, which needs a new dependency).
- **Fixed: 546 → 418 B per member (−23 %).** Where the 546 B went (allocation profiler, one
  guild): the `Arc<CachedMember>` block (192 B requested, 208 B from malloc), the shared
  `Arc<User>` (160 B, 176 B), the role list (16 B, 32 B), and ≈ 60–90 B of hash tables (the
  member's `(guild, user)` key and pointer, the user's, the guild → members index). Internal
  changes could not shrink the blocks; the compact `CachedMember` (96 B) and `CachedUser`
  (104 B), which keep their rarely set fields behind one box, did (D-177, C-080). Applying
  the startup guilds also got faster (166–199 → 155 ms for 500 000 members). What remains:
  the two blocks (≈ 112 + 120 B with their `Arc` headers), the role list's block and the hash
  tables. That is still more than the 373 B of the `VS_NODE.md` stand-in, which kept the
  decoded guilds in one `Vec` per guild without any lookup structure.
- **`CachedPresence` is 152 B** for two IDs, a status and a box of activities:
  `ClientStatus` holds four `Option<Status>`, and `Status` can carry an unknown status as
  text. A compact cached form (one byte per platform) would halve it.
- **Fixed: `TYPING_START` and `INTERACTION_CREATE` were not applied** (1.6 ns: nothing
  done). Both carry a member, an interaction also its user and resolved users and members;
  discord.js caches them. They are applied now, with `THREAD_MEMBERS_UPDATE` (D-173).


---

## Phase 6 — `djs-builders` (2026-11-12)

Step 4 of Phase 6 (D-041). Unpinned; the micro-benchmarks ran two to four times, and ranges
are the medians of the runs. A builder is measured three ways (`benches/builders.rs`):
`validate` is `build()` of a builder cloned before the timed part — the checks alone;
`unchecked` is `build_unchecked()` of the same clone — a move; `assemble` is every setter
from scratch plus `build()`, including the `format!` calls of the benchmark itself.

### Building and checking (`benches/builders.rs`)

| Builder | `build()` | `build_unchecked()` | Assemble + `build()` | Allocations (assemble) |
|---|---:|---:|---:|---:|
| Embed, every part, 25 fields (≈ 3400 characters) | 396–470 ns | 7.6 ns | 2.3–2.7 µs | 58 |
| Embed, title + description + color | 24–27 ns | 6.2 ns | 23–26 ns | 0 |
| Command, 5 groups × 5 subcommands × 4 options (205 names, ≈ 7000 characters) | 7.3–7.8 µs | 7–13 ns | 28–35 µs | 669 |
| `/ping` | 24–29 ns | 4.1 ns | 22–25 ns | 0 |
| String select menu, 25 options | 365–375 ns | 6.6 ns | 1.6 µs | 51 |
| Checkbox group, 10 options | 138–215 ns | 7 ns | 0.59–0.67 µs | 21 |
| Button with label and emoji | 15–19 ns | 7–11 ns | 36–64 ns | 0 |
| Section, thumbnail + 2 texts | 28–31 ns | 5 ns | 64–73 ns | 3 |
| Modal, a text + 4 labels | 230–340 ns | 2 ns | 0.63–0.66 µs | 16 |

| Whole message | Time |
|---|---:|
| `validate_components`, legacy, 5 rows × 5 buttons | 0.83–0.93 µs (was 1.49–1.54 µs) |
| `validate_components`, V2, 30 components (container, sections, gallery, menu) | 1.00–1.06 µs (was 1.57–1.60 µs) |
| Assemble the V2 message + `validate_components` | 3.6–4.5 µs |
| `validate_embeds`, 10 full embeds (rejected: over 6000 characters) | 3.2–3.7 µs |
| `validate_embeds`, 10 small embeds | 110–144 ns |
| `embed_length`, the full embed | 357–368 ns |

- **The checks allocate nothing**, on any builder or whole message (`validate_components`
  allocated 1–2 times before this step).
- **A builder costs nothing over its value:** `build_unchecked()` is a move (2–13 ns), and
  every builder has exactly the size of the `djs-model` value it builds (`EmbedBuilder`
  232 B, `SlashCommandBuilder` 192 B, `ButtonBuilder` 112 B, `ModalBuilder` 72 B; asserted in
  `tests/stress.rs`). What assembling allocates are the caller's strings.
- **Checking text is about 0.1 ns per character** (`embed_length` of 3400 characters:
  360 ns); a command's 27 ns per name also checks each character against the name pattern.

### Extreme load (`tests/stress.rs`)

| Test | Result |
|---|---|
| Embed at the limits (25 fields, 5900–6000 multi-byte characters), clone + `build()` | 0.57 M/s on 1 thread, 2.71 M/s on 12 (4.7×) |
| Command at the limits (25 subcommands × 10 options, ≈ 7800 characters), clone + `build()` | 37 k/s on 1 thread, 187 k/s on 12 (5.1×) |
| V2 message at the limits (40 components, 3900 characters), `validate_components` | 0.64 M/s on 1 thread, 3.26 M/s on 12 (5.1×) |
| Modal at the limits (4000-character input, 25-option menu, 10-option groups, 10 files), clone + `build()` | 0.23 M/s on 1 thread, 1.15 M/s on 12 (5.0×) |
| A 10 MB embed title | rejected in 0.9–1.0 ms (counted once) |
| 1 000 000 embed fields, select options, command options, container or modal components | rejected in 4–16 ms (dropping the value; the count is checked first) |
| 1 000 000 embeds | rejected in 16–28 ns |
| A V2 message of 1 000 000 buttons | rejected in 30–32 ms (was > 2 min, see below) |
| A legacy message of 1 000 000 buttons | rejected in 55–127 ns |
| Action rows, containers or sections nested 200 000 deep | rejected at the first level on a 2 MB test-thread stack (debug build) |

All threads share nothing, so the 4.7–5.1× on 12 threads is the CPU: 2 hyper-threaded P-cores
and 8 slower E-cores.

### Findings and optimization candidates (Phase 6)

- **Fixed: `validate_components` took quadratic time on huge V2 lists.** The 40-component
  limit was checked after the walk, which compared every `custom_id` with every earlier one:
  a list of a million buttons ran for minutes. Past 40 components the walk keeps counting
  but no longer compares `custom_id`s, since the list is rejected for its size anyway:
  30 ms (C-086).
- **Fixed: repeated `custom_id`s and option values were found by comparing strings.** A set
  that keeps the first 40 on the stack with a fingerprint each (length, first and last 8
  bytes) compares the bytes only when fingerprints match: legacy rows 1.5 → 0.9 µs (−38 %),
  the V2 message 1.6 → 1.0 µs (−35 %), a 25-option menu 656 → 375 ns (−43 %), and no
  allocation (C-086).
- **Command names:** ASCII characters take a shortcut past the Unicode case check, 7.8 →
  7.3–7.4 µs for the large command (≈ −6 %, near the noise). Commands are registered once,
  at startup, so the rest was left as is.
- **Open, small:** `validate_components` walks a container twice — once for the
  container's own `custom_id`s, once for the message's — about a tenth of a V2 check.

---

## Phase 7 — `djs-formatters` (2026-11-12)

Step 1b of Phase 7 (D-041). Unpinned, as Phase 6; two runs of the micro-benchmarks, the
second after the optimization below.

### Escaping and formatting (`benches/formatters.rs`)

| Text (`escape_markdown`, defaults) | 64 B | 2 KB | 64 KB | Result |
|---|---:|---:|---:|---|
| Plain English, nothing to escape | 30 ns (2.1 GB/s) | 0.87 µs (2.35 GB/s) | 27 µs (2.39 GB/s) | borrowed, no allocation |
| Polish with a little markdown | 70 ns (0.92 GB/s) | 1.8 µs (1.15 GB/s) | 59 µs (1.11 GB/s) | one allocation |
| Links, mentions and emoji (copied as they are) | 50 ns (1.29 GB/s) | 1.7 µs (1.23 GB/s) | 52 µs (1.26 GB/s) | |
| Markdown on every few bytes | 250 ns (256 MB/s) | 6.3 µs (324 MB/s) | 200 µs (328 MB/s) | |
| The same, code content skipped | 275 ns (233 MB/s) | 6.1 µs (336 MB/s) | 202 µs (324 MB/s) | |
| Headings, lists and masked links on (`LINES`) | 208 ns (308 MB/s) | 4.5 µs (450 MB/s) | 130 µs (506 MB/s) | |
| A display name, `xX_Gamer_Xx` | 29 ns | | | one allocation (27 B) |

| Formatter | Time | Allocations |
|---|---:|---:|
| A giveaway message (bold, mention, timestamp, command mention, link) written into a reused `String` | 130–150 ns | 0 |
| `user_mention(id).to_string()` | 55–60 ns | 1 |
| `format!("<@{}>", id)` by hand, for comparison | 54–56 ns | 1 |
| `time(t).style(RelativeTime).to_string()` | 40–42 ns | 1 + growth |
| `code_block(1 KB).language("rust").to_string()` | 92–95 ns | 1 + 2 growths |
| `ordered_list(25 items).to_string()` | 545–576 ns | 1 + 6 growths |

- **A formatter costs what the equivalent `format!` costs** (the mention: 60 vs 56 ns, both
  dominated by the `String` and the number); written into a buffer, nothing is allocated.
  `to_string()` grows its `String` as it goes (the standard `ToString` has no size hint);
  writing into a buffer avoids that.

### Extreme load (`tests/stress.rs`)

| Test | Result |
|---|---|
| 20 000 random texts (markdown characters, link and token starts, line breaks, multi-byte characters) × 5 option sets | the output is always the input with backslashes inserted, borrowed exactly when none is; escaping twice only inserts more |
| 4 MB of plain ASCII | 1.8 ms (2.35 GB/s) |
| 4 MB of adversarial text: one run of `*` / `*a` / `[` / `[a](b` / `<` / `<a` / `</a b` / `#` / `https://` / empty lines | 3.5–22 ms each (190–1200 MB/s) |
| 4 MB of backticks with code content skipped: one run / `` `a `` / ```` ```a ```` | 15–23 ms (184–272 MB/s; was quadratic, see below) |
| 3 MB of random pieces, each option set | 11–19 ms (169–286 MB/s) |
| `escape_markdown` of 2 KB of chat text, all cores | 0.41 M/s on 1 thread, 1.99 M/s on 12 (4.9×) |
| A giveaway message formatted into a new `String`, all cores | 7.4 M/s on 1 thread, 40.8 M/s on 12 (5.5×) |

### Findings and optimization candidates (Phase 7, formatters)

- **Fixed: plain text was scanned byte by byte at 0.6 GB/s.** Every byte went through the
  `match`, and every `h` started a link check. The scanner now skips to the next byte in a
  256-entry table (markdown characters, `<`, `:`), finds links at their `://` (the scheme
  before it never needs escaping), and tracks line starts only when a line option is on:
  plain text 0.61 → 2.39 GB/s (3.9×), Polish text 0.54 → 1.11 GB/s, a display name 46 →
  29 ns, markdown-heavy text +15 % (C-088).
- **Fixed: a long run of backticks with code content skipped took quadratic time.** Each
  code block measured the whole remaining run before taking three backticks of it; 4 MB ran
  for minutes. The block now only checks for three backticks: 23 ms.
- **Open, small:** markdown-heavy text (≈ 330 MB/s) is bound by inserting one backslash at
  a time; a message is at most 4000 characters (≈ 12 µs), so it was left as is. Skipping
  plain text faster still would take SIMD (`memchr`, a new dependency) or `unsafe`.
