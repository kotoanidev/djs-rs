# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice ("ping-pong" bot).** All decisions for the slice are made
(D-023 … D-034). Implementation starting with `djs-model` core.

## Done
- 2026-11-10: Owner decisions D-001 … D-034 recorded in `docs/DECISIONS.md`.
- 2026-11-10: Rust stable 1.99.1 + 1.85 + 1.89 (MSRV, D-031) installed via rustup.
- 2026-11-10: Phase 0 complete — foundation docs, public repo
  https://github.com/kotoanidev/djs-rs, 9-crate workspace skeleton, lints/configs, CI
  (first run fully green).
- 2026-11-10: Roadmap restructured for D-020 (vertical slice first).
- 2026-11-10: MSRV raised to 1.89 everywhere (Cargo.toml, clippy.toml, CI, docs). `.env.example`.
- 2026-11-10: **Phase 1 step 1 done — `djs-model` core.** 35 unit tests + 17 doctests +
  3 integration tests on realistic fixtures (`crates/djs-model/tests/fixtures/`). Claude defaults
  C-008 … C-011 logged in `docs/DECISIONS.md` for owner review. Reference checkouts
  (discord-api-docs, discord-api-types, discord.js v14) live in git-ignored `.reference/` —
  re-clone with `git clone --depth 1` if missing.
- 2026-11-10: **Phase 1 step 2 done — `djs-rest` core** (D-035: direct `rustls` dep for an explicit
  ring provider). 11 unit + 11 mock-server integration tests + 6 doctests. A real bug was caught
  by tests and fixed: bucket state is migrated when a route's hash is first learned.
- 2026-11-10: **Phase 1 step 3 done — `djs-gateway` core** (D-036 tokio-rustls + webpki-roots,
  D-037 poll-driven Shard). 3 unit (zstd) + 7 mock-gateway integration tests (identify →
  dispatch → reconnect → resume, heartbeats/latency, zombie + identify limit, fatal close,
  command queueing, zstd-stream). CI also runs clippy with `--no-default-features`.

## In progress
- Phase 1, step 4: `djs-rs` core (high-level Client).

## Next step (Phase 1 plan, in order)
1. ~~**`djs-model` core**~~ ✅ — `Id<Marker>` (D-023) with markers + aliases, Snowflake helpers
   (timestamp, worker/process/increment), (de)serialize from JSON strings; `Timestamp`;
   `Intents` (bitflags); minimal `User`, `Message`, `UnavailableGuild`; gateway `OpCode`,
   `Hello`, `Identify` (+ properties), `Ready`, `Resume`, `CloseCode`, dispatch envelope,
   `MessageCreate`. Strings are `CompactString` (D-024). Tests with real JSON fixtures.
2. ~~**`djs-rest` core**~~ ✅ — hyper + hyper-rustls(ring, webpki-roots), `Authorization: Bot`,
   user-agent, per-route bucket limiter (basic) + 429 handling, `get_gateway_bot`,
   `create_message`. `thiserror` errors, `tracing` spans.
3. ~~**`djs-gateway` core**~~ ✅ — single shard over tokio-websockets, zstd-stream, hello →
   identify, heartbeat with jitter + ACK/zombie detection, resume/reconnect on close codes,
   typed event stream.
4. **`djs-rs` core** — `Client::builder(token).intents(..).build()`, `client.on(Events::
   MessageCreate, |ctx, msg| async move {..})` (spawn-per-event default, sequential option,
   D-028), `client.events()` Stream (D-029), `msg.reply(&ctx, ..)`, `login()`, shutdown.
5. `crates/djs-rs/examples/ping.rs`; run live with `.env` token (D-034); mock tests in CI.
6. Architecture review with the owner before Phase 2.

## Blocked on the owner
- A test bot token in `.env` (copy `.env.example`) is needed for step 5 only.
