# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice: functionally done; live test passed.** Remaining before Phase 1 is
closed: optimization benchmarks + extreme-load tests (D-041).
**Then: Phase 2 — full `djs-model` coverage** (D-039; permissions = `u64`, D-040).

## Done
- 2026-11-10: Owner decisions D-001 … D-039 in `docs/DECISIONS.md`; Claude defaults C-001 …
  C-016 (reviewable) in the same file.
- Toolchains (rustup): stable 1.99.1, 1.85, 1.89 (MSRV, D-031).
- Phase 0: foundation docs, public repo https://github.com/kotoanidev/djs-rs, 9-crate workspace,
  lints, CI (fmt, clippy all/no-default features, tests ×3 OS, rustdoc -D warnings, MSRV 1.89,
  cargo-deny) — all green.
- Phase 1:
  - `djs-model` core — IDs, Timestamp, ImageHash, User, Message(+Reference), gateway types,
    single-pass `GatewayEventDeserializer`, `rest::{GatewayBot, CreateMessage}`.
  - `djs-rest` core — hyper + rustls(ring) client, bucket rate limiting (+ hash migration),
    retries, `get_gateway_bot`, `create_message`.
  - `djs-gateway` core — poll-driven `Shard` (D-037), zstd-stream, resume/reconnect, heartbeat.
  - `djs-rs` core — `Client`, `Events`, `client.on`, `HandlerMode`, `client.events()`,
    `MessageExt::reply`, `examples/ping.rs` (tracing-subscriber dev-dep, D-038).
  - 109+ tests (unit, doctests, mock REST, mock gateway, end-to-end).
  - **Live test 2026-11-10:** `cargo run -p djs-rs --example ping` with the owner's `.env` token
    connected to real Discord: TLS + zstd OK, READY OK ("djs-rs tests#3165", 1 guild),
    ClientReady handler OK, `!ping` → `Pong!` reply in 682 ms (incl. Discord API round trip),
    clean Ctrl+C shutdown (close 1000). **Phase 1 live test passed.**
- Reference checkouts (git-ignored `.reference/`): discord-api-docs, discord-api-types,
  discord.js v14 — re-clone with `git clone --depth 1` if missing (discord.js: `--branch v14`).

## In progress
- D-041 for Phase 1: benchmark harness choice (ask owner: criterion vs divan), then benchmarks
  of hot paths (gateway decode, zstd, ID/timestamp parsing, rate limiter) + stress tests
  (mock gateway event flood, concurrent REST through rate limiter, handler dispatch fan-out,
  memory per event) → `docs/PERFORMANCE.md`.

## Next step — Phase 2 (`djs-model` full coverage)
1. Permissions are `u64` bitflags (D-040), serialized as decimal strings.
2. Implement in this order (ROADMAP Phase 2): bitflags (permissions, message/user/channel/guild/
   application flags) → users/members/roles/emojis/stickers → guilds → channels/threads →
   messages/embeds/attachments/reactions/polls → components (incl. V2)/modals → interactions &
   commands → applications/monetization/soundboard → all dispatch events + close codes.
   Verify every value against `.reference/discord-api-docs` (and discord-api-types), compute
   expected test values independently (two earlier hand-written expectations were wrong).
3. Each sub-area: types + rustdoc (links to docs.discord.com + discord.js counterpart) +
   fixture tests; commit + push per sub-area.

## Blocked on the owner
- Benchmark harness choice (criterion vs divan) for D-041.
