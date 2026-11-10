# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice: done** (except confirming the live `!ping` → `Pong!` reply).
**Next: Phase 2 — full `djs-model` coverage** (owner approved, D-039).

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
    ClientReady handler OK, no warnings. The `!ping` → `Pong!` reply was awaiting the owner
    typing `!ping` when this file was written.
- Reference checkouts (git-ignored `.reference/`): discord-api-docs, discord-api-types,
  discord.js v14 — re-clone with `git clone --depth 1` if missing (discord.js: `--branch v14`).

## In progress
- Confirming the live reply (ask the owner whether the bot answered `!ping`; if a session is
  running, the log is in the scratchpad `ping-live.log`).

## Next step — Phase 2 (`djs-model` full coverage)
1. Ask the owner first (significant): **permissions bitfield type** — `u64` bitflags (twilight
   style; Discord sends permissions as decimal strings, currently < 2^64) vs `u128` (future
   proof, discord.js uses BigInt). Also the order of Phase 2 sub-items if they care.
2. Implement in this order (ROADMAP Phase 2): bitflags (permissions, message/user/channel/guild/
   application flags) → users/members/roles/emojis/stickers → guilds → channels/threads →
   messages/embeds/attachments/reactions/polls → components (incl. V2)/modals → interactions &
   commands → applications/monetization/soundboard → all dispatch events + close codes.
   Verify every value against `.reference/discord-api-docs` (and discord-api-types), compute
   expected test values independently (two earlier hand-written expectations were wrong).
3. Each sub-area: types + rustdoc (links to docs.discord.com + discord.js counterpart) +
   fixture tests; commit + push per sub-area.

## Blocked on the owner
- Confirmation that the bot replied to `!ping` (live test).
- Phase 2 question above (permissions type).
