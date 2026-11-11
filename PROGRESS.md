# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice: ✅ done** (live test passed, benchmarks + extreme-load tests done,
D-041).
**Current: Phase 2 — full `djs-model` coverage** (D-039; permissions = `u64`, D-040).

## Done
- 2026-11-10: Owner decisions D-001 … D-075 in `docs/DECISIONS.md`; Claude defaults C-001 …
  C-036 (reviewable) in the same file.
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
  - **Performance (D-041/D-042), results in `docs/PERFORMANCE.md`:** divan benches
    (`djs-model/benches/decode.rs`: MESSAGE_CREATE decode 1.26 µs / 3 allocations vs 2.09 µs /
    76 for `serde_json::Value`), ignored stress tests `tests/stress.rs` in djs-gateway (1M
    events: 674k/s plain, 793k/s zstd, flat memory), djs-rest (246k req/s, 0 rate-limit
    violations with 10k queued requests, 429 storm recovery) and djs-rs (Client 402k events/s
    concurrent, 540k/s sequential, 1M handler calls/s fan-out, < 10 MiB peak). CI compiles the
    benches (`cargo bench --workspace --no-run`).
- Reference checkouts (git-ignored `.reference/`): discord-api-docs, discord-api-types,
  discord.js v14 — re-clone with `git clone --depth 1` if missing (discord.js: `--branch v14`).

## In progress
- Phase 2, step 2 (bitflags) ✅ 2026-11-10: `Permissions` + all 13 integer flag types via the
  internal `int_flags!` macro (`macros.rs`, also used by `Intents`); `tests/flags.rs` checks
  every type against independently computed values. Component flags (unfurled media item,
  content scan metadata) come with the components step.
- Phase 2 step "users/members/roles/emojis/stickers": **user module ✅** (`User`,
  `CurrentUser` (D-052), avatar decoration, collectibles/nameplate, primary guild, premium type,
  `util::Locale`; non-exhaustive structs (D-053), boxed rare sub-objects with size guards
  (D-054)). Not yet: `Connection` / `ApplicationRoleConnection` (need guild integrations).
- `guild::{Member, PartialMember, Role, RoleColors, RoleTags}` ✅ (size guards: Member 304 B,
  PartialMember 160 B, Role 128 B, User 144 B, CurrentUser 216 B).
- `emoji::{Emoji, PartialEmoji}` (D-056) and `sticker` module ✅ — Phase 2 step
  "users/members/roles/emojis/stickers" is done.
- Phase 2 step 3 (guilds), part 1 ✅: `guild::Guild` + level enums, `Feature`, `Preview`,
  widget types, integrations, `Ban`, welcome screen, onboarding, `IncidentsData`;
  `oauth2::Scope`, `gateway::Status`; tests on the docs' JSON examples (`tests/guild.rs`,
  fixtures in `tests/fixtures/docs/`, C-022).
- Phase 2 step 3, part 2 ✅: `guild_scheduled_event`, `auto_moderation` (tests in
  `tests/guild.rs`).
- Deferred (need channel/webhook/command types first): `guild_template` (its
  `serialized_source_guild` has channels), `audit_log` (contains threads, webhooks, commands) →
  do them right after the types they depend on. User `Connection` +
  `ApplicationRoleConnection` can be done any time (integrations exist now).
- Phase 2 step 4, part 1 ✅: `channel` module (D-057 enum + per-kind structs, `RawChannel`
  single-pass conversion), tests `tests/channel.rs` on all docs examples; shared test helper
  `tests/common/mod.rs` (`round_trip`, `assert_subset`).
- Phase 2 step 4, part 2 ✅: `stage_instance`, `webhook`, `invite` (tests
  `tests/invite_webhook.rs`).
- `application` module ✅ (pulled forward: invites, Ready and interactions need it):
  `Application`, `PartialApplication` (→ `Invite::target_application`), integration types +
  config, install params, event webhook status/types, Activity instances; `team` module;
  `guild::PartialGuild`. Tests `tests/application.rs`; `round_trip` now accepts nested
  unmodelled paths (`a.b`, `a[].b`). Monetization (SKU/entitlements) stays in its own step.
- User connections ✅: `user::{Connection, ConnectionService, ConnectionVisibility,
  ApplicationRoleConnection}`, `guild::PartialIntegration`, module
  `application_role_connection_metadata`; maps are `BTreeMap` (D-058), nullable dictionaries
  via `serde_util::null_as_default`. Tests `tests/user.rs`.
- `guild_template` ✅ (D-059: separate snapshot types, `u64` placeholder IDs; C-028), test in
  `tests/guild.rs`. The test helper now compares timestamps as instants (Discord writes
  `+00:00` or `.000000+00:00`).
- Step 5 (messages) ✅: full `message::Message` (D-060 mentions, D-061 referenced message,
  D-062 order, D-064 floats; C-029), `poll`, `interaction` (types messages need). Tests
  `tests/message.rs`; size guards: Message 624 B, Mention 152 B, Attachment 312 B, Embed 208 B.
  Quick bench after the change: MESSAGE_CREATE decode 1.51 µs median, 3 allocations / 881 B
  (Phase 1: 1.26 µs, 3 / 305 B, when only a subset was decoded); `serde_json::Value` baseline
  2.37 µs in the same run.
- Components ✅ (D-065 one `Component` enum, D-066 button variants; C-030): `component`
  module, `Message`/`SnapshotMessage`/`Embed::components`; tests `tests/component.rs` on all 26
  message/modal examples of the docs (JSONC → `fixtures/docs/component_reference_examples.json`).
  Size guards: Component 120 B, Message 648 B, Embed 232 B. The modal itself (an interaction
  response) and the modal-submit / component interaction data come with interactions.
- Interactions step, part 1 ✅: `application_command` module (D-069, C-031), tests
  `tests/application_command.rs` on the docs' 11 command examples. Fixtures for part 2 are
  already extracted: `application_command_interaction_examples.json` (3 interactions),
  `application_command_autocomplete_response.json`.
- Interactions step, part 2 ✅: `entitlement` module (pulled forward), `interaction::{Interaction,
  InteractionData (D-067), CommandData + option values, ComponentData, ModalSubmitData +
  submitted components, Resolved, InteractionMember, InteractionChannel, InteractionGuild}`,
  responses + callback response objects (D-068), `poll::CreatePoll`,
  `message::AttachmentRequest`, `Message::resolved` (C-032). Tests `tests/interaction.rs` (the
  docs' 3 example interactions + autocomplete example, synthetic component/modal payloads, every
  response type). Size guards: Interaction 288 B, InteractionData 16 B, Message 656 B.
- `audit_log` ✅ (D-070 own JSON value type for change values, D-071 options struct; C-033),
  tests `tests/audit_log.rs` (synthetic log around the docs' example objects).
- SKUs, subscriptions ✅ and `voice` (voice states, regions; was missing from the plan) ✅
  (C-034); tests in `tests/application.rs` and `tests/voice.rs`.
- `soundboard`, `lobby` ✅ (C-035), tests `tests/soundboard_lobby.rs`.
- `application_identity_profile` ✅ (D-072, C-036; `util::JsonValue` shared with the audit
  log), test in `tests/application.rs`. All resource pages of the docs are now modelled.
- Dispatch events ✅ (D-073 own payload types, D-074 presence users, D-075 Unix times as
  `Timestamp`; C-037): `gateway::presence::{Presence, PresenceUser, ClientStatus, Activity +
  parts}`, payload types for every dispatch event in
  `gateway/payload/incoming/{mod,guild,channel,message}.rs` (`newtype_event!`,
  `guild_id_event!`, `serde_util::extras` single-pass "object + extra fields" combinator;
  GUILD_CREATE recognizes unavailable guilds through `Extras::recover` + a replayed `id`),
  all 78 events routed in `gateway::DispatchEvent` (+ `Serialize`), gateway commands (request
  guild members / soundboard sounds / channel info, voice state, presence; `Identify` presence
  and capabilities). Tests `tests/gateway_dispatch.rs`: a synthetic or docs payload for every
  event decoded from a full envelope and round-tripped, GUILD_CREATE available/unavailable/
  errors, extras in any key order and with escaped keys, presences with the docs' activities,
  Unix seconds, redaction, commands equal to the docs' examples. Gateway close codes were
  already complete (checked against the docs).
- **Next:** topics pages check (anything unmodelled that is not REST/voice — rate-limit and
  error bodies are Phase 3) → Phase 2 benches and stress tests (D-041): extend
  `djs-model/benches/decode.rs` (every event family, large GUILD_CREATE), extreme-load decode
  tests, A/B against the Phase 1 commit `65210f8`, record in `docs/PERFORMANCE.md`.
- Local gate script: `scratchpad/gate.sh` style (`set -euo pipefail`, no pipes hiding errors) —
  a piped gate let one clippy failure through (fixed in `cda06db`).
- Perf note: after the user change, MESSAGE_CREATE/READY decode looked ~8-13 % slower relative
  to the `serde_json::Value` baseline (laptop was ~35 % slower overall during that run). At the
  end of Phase 2, run the decode benches back-to-back against the Phase 1 commit `65210f8`
  (git worktree) and record the comparison in `docs/PERFORMANCE.md`.

## Next step — Phase 2 (`djs-model` full coverage)
1. Permissions are `u64` bitflags (D-040), serialized as decimal strings, named
   `djs_model::permissions::Permissions` (D-045). Module layout = one module per Discord docs
   resource/topic page (D-044).
2. Implement in this order (ROADMAP Phase 2): bitflags (permissions, message/user/channel/guild/
   application flags) → users/members/roles/emojis/stickers → guilds → channels/threads →
   messages/embeds/attachments/reactions/polls → components (incl. V2)/modals → interactions &
   commands → applications/monetization/soundboard → all dispatch events + close codes.
   Verify every value against `.reference/discord-api-docs` (and discord-api-types), compute
   expected test values independently (two earlier hand-written expectations were wrong).
3. Each sub-area: types + rustdoc (links to docs.discord.com + discord.js counterpart) +
   fixture tests; commit + push per sub-area.
4. When Phase 2 is done: extend `djs-model/benches/decode.rs` (all events, large GUILD_CREATE
   decoded for real) + stress tests, update `docs/PERFORMANCE.md` (D-041).
5. Optimization candidates noted in `docs/PERFORMANCE.md` (1 KiB REST futures, Timestamp
   formatting) are for Phase 3 / when touching that code. Phase 3 also replaces the global
   REST limiter with strict GCRA (D-043).

## Blocked on the owner
- Nothing.
