# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice: ✅ done** (live test passed, benchmarks + extreme-load tests done,
D-041).
**Phase 2 — full `djs-model` coverage: ✅ done** (D-039; benchmarks + stress tests done).
**Phase 3 — full `djs-rest` coverage: ✅ done 2026-11-12** (every documented REST route, OAuth2,
CDN URLs; benchmarks + extreme-load tests in `docs/PERFORMANCE.md`).
**Phase 4 — full `djs-gateway` coverage: ✅ done 2026-11-12** (send limit, IDENTIFY queue +
sharding, presence, zlib-stream, JSON parsers measured; benchmarks + extreme-load tests and a
`perf`-profiled A/B in `docs/PERFORMANCE.md`). Left open on purpose: zero-copy parsing (no
measured need after D-150) and resharding while running (open question).
**Phase 5 — `djs-cache`: ✅ done 2026-11-12** (pluggable cache, every resource and event,
limits, sweeps, opt-in `Client` cache; benchmarks + extreme-load tests in
`docs/PERFORMANCE.md`, `docs/VS_NODE.md` re-measured with equal caches). Left open on purpose:
a lock-free backend (D-176), resolved roles/messages of interactions (open question).
**Phase 6 — `djs-builders`: ✅ done 2026-11-12** (validated builders for embeds, commands,
components incl. Components V2, and modals, with rules measured on the live API where Discord
allows it; benchmarks + extreme-load tests in `docs/PERFORMANCE.md`). Left open: removing
parts with the builders (open question), a container walked twice by `validate_components`.
**Phase 7 — small standalone crates: ✅ done 2026-11-12** (D-208–D-218; `djs-formatters` and
`djs-collection` built, benchmarked and stress-tested; `djs-util` stays empty until two crates
need a helper, D-218).
**Phase 8 — full `djs-rs` client: ✅ done 2026-11-13** (events with the cached old state,
structures with methods, managers, interactions and a router, collectors and gateway
requests, intents and partials guidance, example bots run on the test bot; benchmarks +
extreme-load tests in `docs/PERFORMANCE.md`).
**The serenity comparison: ✅ done 2026-11-13** (`docs/VS_SERENITY.md`, D-277–D-282, C-108,
C-109). The fixes found by the comparison (D-283) ✅: the rebuild time after an edit
(D-287–D-289, `docs/VS_SERENITY.md` re-measured) and the ergonomics gaps (D-290–D-295).
**Phase 9 — multi-process sharding: ✅ done 2026-11-14** (`djs_rs::sharding` behind the
`sharding` feature: processes of the same executable, IPC over `127.0.0.1`, the manager as
IDENTIFY queue, restarts, named commands and stats; benchmarks + extreme-load tests in
`docs/PERFORMANCE.md` "Phase 9"). **Next: Phase 10 — `djs-voice` (D-017)**: nothing decided
yet beyond D-017; ask the owner first (see "Next step").

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
- Topics/events pages check ✅: dispatch events equal discord-api-types' list; gateway opcodes
  and close codes complete; OAuth2 token bodies, rate-limit and error bodies belong to Phase 3
  (REST), voice opcodes/close codes to Phase 10; RPC is out of scope (D-077). Webhook events
  ✅ (D-076, D-078, D-079, C-038): `webhook_event` module, `LobbyMessage` extended, tests
  `tests/webhook_event.rs` on the docs' 11 examples (`webhook_events_examples.json`, extracted
  with a script that replaces `// user data` by a synthetic user).
- Phase 2 benchmarks and stress tests (D-041) ✅ — results in `docs/PERFORMANCE.md` ("Phase 2"):
  shared realistic payloads `djs-model/tests/common/payloads.rs` (also used by the benches via
  `#[path]` and by the `djs-gateway` stress test), `benches/decode.rs` extended (every hot
  event, full 1 000-member GUILD_CREATE vs `serde_json::Value`, commands), new
  `djs-model/tests/stress.rs` (1-thread / all-threads floods, 65 MB GUILD_CREATE, 2 500 guilds
  kept alive, plus non-ignored robustness tests: truncation/corruption, deep nesting), A/B
  against Phase 1 (`65210f8` worktree) for the decode benches and the gateway/client stress
  tests. The `djs-gateway` stress test's large-event scenario now uses an unknown event name
  (GUILD_CREATE is decoded now), plus a new full-GUILD_CREATE scenario.
- Findings decided by the owner: D-080 — `mimalloc` dev-dependency of `djs-rs`, new
  `djs-rs/tests/stress_mimalloc.rs`; mimalloc gives fan-out +80 %, stream +30…50 %, one
  concurrent handler ≈ +15 %, sequential −4 % → recommended in the `djs-rs` README (very large
  bots); the remaining concurrent-mode cost goes to Phase 8. D-081 — decoded model size
  (≈ 1.2× JSON) is handled with the cache in Phase 5.
- **Phase 3 started** (D-082 methods + body structs, D-083 tri-state enum, D-084 order:
  infrastructure first, D-085 invalid-request counter + opt-in fail-fast, D-086
  `reason: Option<&str>` parameter, D-087 files in body structs, D-088 `native-roots` feature,
  D-089 sliding-log global limiter). Done: strict global limiter (sliding log) — stress test
  now asserts ≤ 50 per sliding second (5/5 runs); invalid-request counter + opt-in limit
  (D-085, C-039); `Route<'a>` (D-090) generated by the `routes!` macro with the first 8
  interaction routes — token segments percent-encoded, tokens redacted in `Debug` and never
  logged (log fields use the route template), interaction endpoints skip the global limit,
  callbacks are burst routes that never queue behind one another (C-039; tests in
  `tests/mock_server.rs`, verified to fail without the burst handling); old-message-delete
  sublimit (`Route::DeleteMessage`, own bucket after 14 days, C-039); `X-Audit-Log-Reason`
  (`Request::reason`, D-086, C-039); multipart file uploads (D-087, D-093..D-096, C-040:
  `FileUpload`, generated `attachments`, zero-copy chunked body in `djs-rest/src/multipart.rs`;
  benchmark the multipart path with large files in the Phase 3 bench stage); API errors
  (D-097..D-099, C-041: `ErrorCode` with 245 codes generated by `scripts/gen_error_codes.py`
  — re-run it against the docs when they add codes —, `FieldError`, discord.js-like
  `Display`);
  `native-roots` feature (D-088, D-100, C-042), verified live with the `ping` example.
  **Phase 3 infrastructure is complete.**
- Endpoint prerequisites done: `Patch<T>` (D-101) and the query encoder
  (`djs-rest/src/query.rs`, C-043; `Request::query` is `#[expect(dead_code)]` until the first
  endpoint with query parameters uses it — then remove the attribute).
- Endpoints (D-102 order, C-044 conventions): **messages and reactions ✅** (17 methods incl.
  Search Guild Messages, D-104/C-045; `tests/messages.rs`).
- **Interactions ✅** (9 methods, `tests/interactions.rs`; D-106 two methods for
  `with_response`, D-107 `CreateFollowupMessage`; `EditWebhookMessage` lives in
  `rest/webhook.rs` and will be shared with the webhook endpoints).
- **Application commands ✅** (15 methods, `tests/application_commands.rs`, C-046):
  global/guild get/create/edit/delete/bulk overwrite and permissions; bodies
  `CreateApplicationCommand`, `EditApplicationCommand`, `EditApplicationCommandPermissions`,
  query `GetApplicationCommands`. `RestBuilder::bearer_token` (D-108) for the permissions
  edit; `RestBuilder`'s `Debug` now redacts the token.
- `util::ImageData` (D-113, hand-written streaming base64) ✅ — benchmark it with large
  images in the Phase 3 bench stage.
- **Channels and threads ✅** (28 methods, `tests/channels.rs`, D-109..D-112, D-114, D-115,
  C-047). The multipart writer accepts one file under its own field name
  (`Request::form_file`, crate-private, D-111) — reuse it for the invite target-user
  endpoints.
- **Guilds ✅** (45 methods, `tests/guilds.rs` — it reuses the docs fixtures of
  `djs-model`; D-116..D-118, C-048). `InviteMetadata` is optional (D-117). The widget image
  URL (D-116) belongs to the CDN builder step.
- **Users ✅** (12 methods, `tests/users.rs`, C-049).
- **Webhooks ✅** (14 methods, `tests/webhooks.rs`, D-119..D-121, C-050). Token routes
  skip the bot token (`Route::authenticated`, derived from a `*_token` parameter).
- `util::SoundData` ✅ (D-122); multipart text form fields (`multipart::Form`,
  `Request::form_text`, crate-private).
- **Showcase vs discord.js ✅ (D-123, C-051):** `docs/VS_NODE.md` (linked from the README),
  charts in `docs/vs-node/`, harness in `benches/vs-node/` (workspace crate `djs-vs-node`,
  `node/` scripts, `package.json` + lockfile pinning discord.js 14.28.1, @discordjs/ws 1.2.3,
  @discordjs/rest 2.6.3). Re-run with `benches/vs-node/run.sh` (≈ 5 min; afterwards check
  the numbers quoted in the prose of `VS_NODE.md`). Result 2026-11-11: idle memory 19× less,
  decode 3.6–5× faster, flood 2.2× (default `Client`) / 4.2× (sequential handlers), memory
  2.7–4.7× less, REST 8.6× the requests and 7× lower p99. Finding: the default
  `HandlerMode::Concurrent` costs ≈ 6.7 µs CPU per event under a flood (sequential: 1.8 µs)
  — recorded as an open question in `docs/DECISIONS.md`. Re-run the showcase when the cache
  (Phase 5) lands.
- **Emojis ✅** (10 methods, `tests/emojis.rs`, C-052).
- **Stickers ✅** (8 methods, `tests/stickers.rs`, C-053); `Request::form_text` is now used
  (its `#[expect(dead_code)]` is gone).
- **Soundboard ✅** (7 methods, `tests/soundboard.rs`, C-054).
- **Guild scheduled events ✅** (6 methods, `tests/scheduled_events.rs`, D-124, D-125,
  C-055). Fixed `GuildScheduledEventUser::member` (now `PartialMember`).
- **Stage instances ✅** (4 methods, `tests/stage_instances.rs`, C-056).
- **Invites ✅** (9 methods, `tests/invites.rs`, D-126, D-127, C-057).
- **Guild templates ✅** (6 methods, `tests/guild_templates.rs`, C-058).
- **Audit log ✅** (`get_guild_audit_log`, `tests/audit_log.rs`).
- **Auto moderation ✅** (5 methods, `tests/auto_moderation.rs`, D-128, D-129, C-059).
- **Polls ✅** (2 methods, `tests/polls.rs`, C-060).
- **Monetization ✅** (SKUs, entitlements, subscriptions: 9 methods,
  `tests/monetization.rs`, D-130, C-061).
- **Application ✅** (3 methods, `tests/applications.rs`, C-062).
- **Role connection metadata ✅** (2 methods, `tests/role_connections.rs`).
- **OAuth2 ✅** (6 methods, `oauth2::{OAuth2Credentials, AuthorizeUrl}`, `tests/oauth2.rs`
  in djs-rest and djs-model, D-131–D-133, C-063). `Request`'s `Debug` no longer prints the
  body.
- **Lobbies ✅** (16 methods, `tests/lobbies.rs`, D-134, C-064).
- **Voice ✅** (5 methods, `tests/voice.rs`). Every resource of D-102 is done.
- **Application identity profiles ✅** (5 methods) and **Get Gateway ✅**
  (`tests/identity_profiles.rs`, D-135, D-136, C-065). **Endpoint coverage is complete:**
  `python3 -I scripts/route_coverage.py` lists only the 8 routes left out on purpose
  (D-105, D-116, D-120, disabled Batch Edit Application Command Permissions).
- **CDN URL builder ✅** (`djs_rest::cdn::Cdn`, `Rest::cdn()`,
  `Rest::guild_widget_image_url`, D-137–D-139, C-066).
- **Phase 3 benchmarks + extreme-load tests ✅** (`djs-model/benches/encode.rs`,
  `djs-rest/benches/request.rs`, multipart stress tests; `docs/PERFORMANCE.md` "Phase 3"):
  base64 bodies 1.45 GB/s, `Route::path` 45–180 ns, 512 MiB of 8 MiB uploads at
  3.3–4.0 GiB/s with +5 MiB peak (zero-copy), 236k req/s, all rate-limit guarantees hold.
  Candidates recorded there: request future 1 448 B (was 1 024 B), JSON body buffer growth.
  **Phase 3 is done** (ROADMAP ticked).
- Local gate script: `scratchpad/gate.sh` style (`set -euo pipefail`, no pipes hiding errors) —
  a piped gate let one clippy failure through (fixed in `cda06db`).
- Perf note: after the user change, MESSAGE_CREATE/READY decode looked ~8-13 % slower relative
  to the `serde_json::Value` baseline (laptop was ~35 % slower overall during that run). At the
  end of Phase 2, run the decode benches back-to-back against the Phase 1 commit `65210f8`
  (git worktree) and record the comparison in `docs/PERFORMANCE.md`.

## Phase 4 steps (`djs-gateway` full coverage) — all done
Decisions taken 2026-11-12: D-140 (zlib-stream = `flate2` + `zlib-rs`), D-141 (shard factory
+ shared IDENTIFY queue, no djs-rest dependency), D-142 (commands over 120/60 s wait; heartbeats
reserved), D-143 (order below). One commit per step:
1. ✅ **Command send limit** (D-142, C-067) — `crates/djs-gateway/src/send_limit.rs`: sliding
   window, 115 commands per 61 s per connection, held command in `Shard::held`; unit tests +
   two mock-gateway tests. A real-time test over several windows (≈ 2–3 min, `#[ignore]`)
   belongs to step 6.
2. ✅ **IDENTIFY queue + sharding** (D-141, D-144–D-147, C-068): `djs-gateway/src/queue.rs`
   (`IdentifyQueue`, `LocalIdentifyQueue`), `src/sharding.rs` (`create_shards`), djs-rs
   `Shards` + one task per shard + `Error::SessionStartLimit`. Tests: queue unit tests (paused
   clock, no sockets), mock-gateway `shards_share_the_identify_queue`, djs-rs end-to-end auto
   sharding / session limit / range.
3. ✅ **Gateway commands** (D-148, D-149, C-069): presence in `Config` / IDENTIFY,
   `MessageSender::update_presence` (kept for new sessions), `ClientBuilder::presence`,
   `Client::set_presence`, `Activity` constructors. Collectors (members, soundboard) and
   voice state helpers are Phase 6; any command already goes through `command()`.
4. ✅ **zlib-stream** (D-140, C-070): `Compression::ZlibStream` (`compression.rs`,
   `flate2` 1.1.11 + `zlib-rs`), split payloads buffered; unit + mock-gateway tests; stress
   A/B against zstd in `docs/PERFORMANCE.md` ("Phase 4").
5. ✅ **simd-json** (D-013, D-150): measured simd-json 0.19 and sonic-rs 0.5 against
   serde_json (alternating groups, pinned cores, P/E cores, `target-cpu=native`); no clear
   gain, so no feature — serde_json stays; experiment removed (kept in commit `af58a18`),
   results in `docs/PERFORMANCE.md`.
6. ✅ **Benchmarks + extreme-load tests** (D-041): `djs-gateway/benches/gateway.rs`, stress
   tests `thousand_shards_*`, `hundred_shards_long_streams_*`, `send_limit_in_real_time`,
   `type_sizes`, Discord-like compression (C-071), djs-rs `Client` with 16 shards (5.1–5.5×
   one shard). The A/B against `d9cffea` was profiled with `perf`: the large unpinned drop is
   P-core/E-core placement; pinned, the `Client` flood shows no regression and the gateway
   flood +1.5–3 % shard CPU (C-072, `docs/PERFORMANCE.md` "A/B against the start of Phase 4").

## Next step — Phase 5 (`djs-cache`, ROADMAP)
Decided 2026-11-12: D-016 (pluggable, DashMap default), D-081 (own compact cached types),
D-151 (reads return `Arc<T>`), D-152 (minimal defaults: guilds, channels, roles, current
user), D-153 (updates return the previous state), D-154 (order below), D-155–D-159 (step 1 API), D-160–D-163 (step 2 API). One commit per step:
1. ✅ Backend trait + DashMap backend + configuration (D-155–D-159, C-073):
   `crates/djs-cache/src/{backend.rs (Store, Backend), dashmap_store.rs (DashMapBackend,
   DashMapStore, foldhash Hasher), config.rs (Resource, Limit, CacheConfig, builder,
   pub(crate) Scope), limits.rs (pub(crate) Global / Scoped / Order: FIFO limits over the
   whole cache or per parent, with the parent → keys index), cache.rs (Cache skeleton)}`.
   `limits.rs` and `Resource::scope` carry `expect(dead_code)` until step 2 uses them.
2. ✅ Guilds, channels, roles, emojis, stickers, members, users (D-160–D-163, C-074):
   `model.rs` (`CachedGuild`, `CachedMember`), `view.rs` (`CacheView`, `GuildView`),
   `update.rs` (`UpdateCache` sealed + macro impls, `Previous`, `dispatch`), `cache.rs`
   (fields per resource, change helpers, views), `limits.rs` (`Scoped` keyed `(parent, id)`,
   `Index`, `GlobalOf` / `ScopedOf`); tests `crates/djs-cache/tests/events.rs` (payloads
   from `djs-model/tests/common/payloads.rs`).
3. ✅ Messages, reactions, presences, voice states, threads, scheduled events, stage
   instances, soundboard (D-164–D-166, C-075): `CachedMessage` / `CachedPresence` in
   `model.rs`, `ChannelView` in `view.rs`, `ReactionChange` in `cache.rs`,
   `djs_model::message::Reaction::new`; tests `crates/djs-cache/tests/messages_and_more.rs`.
4. ✅ Sweepers (D-167–D-169, C-076): `Sweep` / `SweepConfig` in `config.rs`, `Global::sweep`
   / `Scoped::sweep` in `limits.rs`, `Cache::sweep_*` (+ `sweep_per_guild!`) in `cache.rs`;
   tests `crates/djs-cache/tests/sweep.rs`.
5. ✅ djs-rs `Client` integration (D-170–D-172, C-077): `Client<B = DashMapBackend>`,
   `ClientBuilder<B>` (`cache`, `cache_backend`), `Context<B>` (`cache()`), generic
   `ErasedHandler<B>` / `Registry<B>` (`handler.rs`), `MessageExt::reply<B>`; `Client::dispatch`
   calls `cache.update_dispatch` first; `Client::spawn_sweepers` + `every` (one task per
   sweep, shut down when `login` returns); `djs_rs::cache` re-export. Tests in
   `crates/djs-rs/tests/end_to_end.rs` (cache filled before handlers, custom
   `CountingBackend`, sweeps run, no cache by default) and a unit test in `client.rs`.
6. **In progress: benchmarks + extreme-load tests (D-041).** Done: `crates/djs-cache/benches/cache.rs`
   (divan; hot events, reads, sweeps, `SipBackend` for D-158), `crates/djs-cache/tests/stress.rs`
   (startup memory, concurrency, churn, sweeps under load, type sizes), the shared-lock fast
   path in `limits.rs` (C-078); results in `docs/PERFORMANCE.md` "Phase 5". Findings: writer
   starvation under oversubscription (open question: lock-free backend), 424 B per member
   (open question), `TYPING_START` / `INTERACTION_CREATE` not applied (ask the owner).
   Decided 2026-11-12 (D-173–D-176). Left, in this order:
   a. ✅ D-173 (C-079): `TypingStart`, `InteractionCreate`, `ThreadMembersUpdate` in
      `update.rs`; `Cache::{apply_interaction, upsert_partial_member, update_thread_members}`
      in `cache.rs`; tests at the end of `tests/messages_and_more.rs`.
   b. ✅ D-175 → D-177 (C-080): measured in a fresh process (`startup_500_guilds` spawns a
      child per configuration) 546 B per member; with the compact `CachedUser` and
      `CachedMember` (`model.rs`) 418 B (−23 %).
   c. ✅ The djs-rs `Client` flood with and without a cache (`crates/djs-rs/tests/stress.rs`,
      `flood_1m_{sequential,concurrent}_with_cache`): −14 % sequential, −6 % concurrent.
   d. ✅ D-174: `docs/VS_NODE.md` re-measured (`benches/vs-node/run.sh`, flood `--cache
      discord-js`, memory through a cached `Client`); the flood client now uses
      `Shards::Count(1)` — since D-146 it had called the real Get Gateway Bot and hung.
   e. ✅ Phase 5 marked done (ROADMAP, PROGRESS).

## Next step — Phase 6 (`djs-builders`, ROADMAP)
Decided 2026-11-12: D-178 (builders consume `self`), D-179 (validation in `build()` →
`Result`), D-180 (`build_unchecked()`), D-181 (order: embeds → commands → components),
D-182 (`ValidationError` enum, first violation), D-183 (limits count code points after
trimming), D-184 (verify the counting on the live API with the test bot), D-185 (discord.js
names, `djs_rs::builders` re-export), D-186 (`EmbedBuilder` API), D-187 / D-190 (URL
schemes), D-188 / D-191 (empty texts and empty embeds, measured live), D-189 (message-wide
`validate_embeds`), D-192 (command API: values passed to `.option()`), D-193 (mixing
checked in `build()`), D-194 (dependency-free name check), D-195 (command rules measured
live), D-196 (reject options that cannot work), D-197 (a button constructor per style),
D-198 (row/container children checked in `build()`), D-199 (`validate_components`), D-200 /
D-201 (component rules measured live), D-202 (`PremiumButtonBuilder`), D-203 (select option
shortcut + builder), D-204 (V2 API: accessory in `new()`, `.text()` / `.item()` shortcuts),
D-205 (V2 rules measured live), D-206 (modal API: shortcuts as before), D-207 (modals from
labels only; rows of text inputs rejected). Steps:
0. ✅ The live counting test (D-184): code points after trimming, recorded under D-183.
1. ✅ `EmbedBuilder` (C-081): `crates/djs-builders/src/{error.rs (ValidationError),
   check.rs (shared length / count / URL checks), embed.rs (EmbedBuilder,
   EmbedAuthorBuilder, EmbedFooterBuilder, embed_length, validate_embeds)}`; constructors
   for the embed types in `djs-model/src/message/embed.rs`; tests
   `crates/djs-builders/tests/embed.rs` (the live results as test cases); `djs_rs::builders`.
2. ✅ Command builders (D-192–D-196, C-082): `crates/djs-builders/src/command/{mod.rs
   (setter macros), validate.rs (all rules, Level, limits), slash.rs (SlashCommandBuilder,
   subcommand and group builders), option.rs (option_type! macro, nine option types),
   context_menu.rs}`; `check::Path` for nested error paths; `ValidationError::{InvalidName,
   Invalid}`; option constructors in `djs-model/src/application_command.rs`; tests
   `crates/djs-builders/tests/command.rs` (live results of D-195 as cases).
3. Components, decided D-197–D-203 (live results under D-200 / D-201), in three commits (C-083):
   a. ✅ Message components: `crates/djs-builders/src/component/{mod.rs, validate.rs (all
      rules), button.rs (ButtonBuilder, PremiumButtonBuilder), select.rs (string, user,
      role, mentionable, channel menus; StringSelectMenuOptionBuilder), action_row.rs,
      list.rs (validate_components, ComponentsVersion)}`; `ValidationError::TooFew`;
      `check::Path::Child`; tests `crates/djs-builders/tests/component.rs`.
   b. ✅ Components V2 (D-204, D-205 live, C-084): `component/{content.rs (TextDisplay,
      Thumbnail, MediaGallery + item, File builders), layout.rs (Container, Section,
      Separator builders)}`, `component_builder!` macro in `component/mod.rs`, V2 rules and the
      shared `Totals` walk in `component/validate.rs`; tests `tests/component_v2.rs`.
   c. ✅ Modals (D-206, D-207, C-085; rules from the docs only, D-200):
      `component/modal.rs` (ModalBuilder, LabelBuilder, TextInputBuilder, FileUploadBuilder,
      RadioGroupBuilder, CheckboxGroupBuilder + their option builders, CheckboxBuilder),
      modal rules in `component/validate.rs` (`modal`, `label`, `in_modal`, …); tests
      `tests/modal.rs`.
4. ✅ Benchmarks + extreme-load tests (D-041, C-086): `crates/djs-builders/benches/builders.rs`
   (`build()` vs `build_unchecked()` vs assembly for every builder kind, whole-message
   checks), `tests/stress.rs` (all cores on maximum values, huge inputs, 200 000-deep trees,
   builder sizes); results in `docs/PERFORMANCE.md` ("Phase 6"). Fixed on the way: quadratic
   `validate_components` on huge V2 lists; repeated `custom_id`s / option values found with a
   fingerprinted stack set (−35–43 %, no allocation).

## Next step — Phase 7 (small standalone crates, ROADMAP)
Decided 2026-11-12: D-208 (order: `djs-formatters`, then `djs-collection`; `djs-util` only
as needed), D-209 (`Collection` on `indexmap`, insertion order), D-210 (`Collection` API:
discord.js names where they fit + Rust traits; no `each`/`reduce`/`some`/`every`), D-211
(formatters return `Display` types, mentions take `djs-model` IDs, escaping returns
`Cow<str>`), D-212 (escaping: own clear rules, discord.js' options and defaults), D-213
(optional parts are methods on the `Display` types). Steps:
1. `djs-formatters` (D-211–D-213, C-087):
   a. ✅ The crate: `crates/djs-formatters/src/{mention.rs (mentions, emoji,
      GuildNavigation), timestamp.rs (time, TimestampStyle, UnixSeconds), markdown.rs
      (wrapper! macro for the emphasis / quote formatters, code blocks, headings, hyperlinks,
      lists with the sealed ListEntry, Face), link.rs (channel / message / App Directory,
      email, phone_number), escape.rs (EscapeOptions, escape_markdown, single escapers — one
      byte scanner, lazy Cow)}`; tests `tests/{escape.rs, format.rs}`; `djs_rs::formatters`.
   b. ✅ Benchmarks + extreme-load tests (D-041, C-088): `benches/formatters.rs`,
      `tests/stress.rs` (randomized invariants, adversarial megabytes, all cores); results
      in `docs/PERFORMANCE.md` ("Phase 7 — djs-formatters"). Fixed on the way: plain text
      scanned byte by byte (now a lookup-table skip, 3.9× faster) and a quadratic case for
      long backtick runs with code content skipped.
2. `djs-collection` (D-209, D-210, D-214–D-217, C-089):
   a. ✅ The crate: `crates/djs-collection/src/{collection.rs (Collection, DefaultHashBuilder;
      map basics, discord.js helpers, consuming transformations, trait impls), iter.rs
      (iterator! macro wrapping indexmap's iterators), random.rs (RandomState-based picks,
      Floyd / partial Fisher–Yates)}`; tests `tests/collection.rs`; `djs_rs::collection`.
      Whether `djs-cache` / the `Client` return it is left to Phase 8 (open question).
   b. ✅ Benchmarks + extreme-load tests (D-041, C-090): `benches/collection.rs` (get /
      build / iterate vs `IndexMap` and `HashMap`, removal patterns, every helper),
      `tests/stress.rs` (model check, random-pick fairness, 10 000 000 entries with memory
      per entry, adversarial removals, all cores); results in `docs/PERFORMANCE.md`
      ("Phase 7 — djs-collection"). Fixed on the way: `random_n` hashed a fresh
      `RandomState` per index (now wyrand, 3–4× faster); `partition` rehashed every entry
      (now `extract_if`, 1.7× faster).
3. ✅ `djs-util`: stays an empty placeholder until two or more crates need a helper (D-218).

## Next step — Phase 8 (`djs-rs` full high-level client, ROADMAP)
Decided 2026-11-12: D-219 (order: events → structures + managers → interactions →
collectors → partials / intents → examples), D-220 (methods are extension traits on the
`djs-model` types taking `&ctx`), D-221 (one payload with the previous state), D-222
(discord.js event names + `Raw`), D-223 (the client keeps only unavailable guild IDs;
per-item diff events need the cache), D-224 (`ShardReady` / `ClientReady` at `READY`, no
waiting), D-225 (`djs-model` payload where it suffices, own types in
`djs_rs::event::payload`), D-226 (shard lifecycle events; diagnostics in `tracing`), D-227
(previous state only where discord.js hands an old or removed object), D-228 (`old` + `new`
fields with `Deref` to `new`). Steps:
1. Every event (D-221–D-228, C-091):
   a. ✅ `crates/djs-rs/src/event/{mod.rs (events! macro: Events, one marker type + Event impl
      per event, EventKind, Dispatch), payload.rs (previous-state payloads with old/new +
      Deref, derived emoji/sticker payloads, lifecycle payloads)}`, `src/dispatch.rs` (convert:
      cache update, Raw, previous state, guild availability tracking, list diffs; payloads only
      for wanted events), `client.rs` (shard loop: lifecycle events, replay count,
      `ClientReady` once; `emit`); tests `tests/events.rs` (scripted mock gateway).
   b. ✅ Benchmarks + extreme-load tests (D-041, C-092): `tests/stress.rs` gained update /
      `Raw` / unwatched floods; A/B against `e65961a` pinned to two P-cores: −2.4 % (+45 ns
      per event, spread bookkeeping, same `perf` profile); results in `docs/PERFORMANCE.md`
      ("Phase 8 — djs-rs events").
2. ✅ Structures with methods and managers (D-220, D-229–D-247): managers on the `Context`
   (`ctx.members(guild).fetch(user)`), `fetch` cache-first with `force` and REST write-back
   (new public write API in `djs-cache`), lists as `Collection`, `fetch` returning the cached
   type behind an `Arc` (REST results converted), methods for model and cached types; the
   most used first (D-232). Steps:
   a. ✅ `djs-cache` write API: `Cache::store_{user,member,message,channel,role,guild}`
      (convert, store when enabled, return the stored `Arc`; tests `tests/store.rs`).
      Without a cache, the managers convert through a private cache with every resource
      disabled.
   b. ✅ The first managers (C-093): `crates/djs-rs/src/manager/{mod.rs (Fetch builder with
      `force` / `is_cached`, `IntoFuture`; the context's `store_*` helpers; the accessors
      `ctx.guilds()` … `ctx.bans(guild)`), guild.rs, channel.rs, user.rs, member.rs,
      message.rs, role.rs, ban.rs}`; `src/ext.rs` (`MessageExt`, `ChannelExt`, `UserExt`,
      `MemberExt`, `GuildExt` over sealed ID traits, for model + cached types); tests
      `tests/managers.rs` (routing mock REST API, `Context` from the `ClientReady` handler).
      `reply` now returns `Arc<CachedMessage>` (breaking, CHANGELOG).
   c. ✅ The other resources, one batch per commit, the most used first (D-232):
      (1) ✅ writes on the existing resources (D-234 guild channel manager, D-235 typed
      overwrites, D-236 deletions leave the cache to the gateway; C-094) — channels
      edit / delete / overwrites, `ctx.guild_channels(guild)` list / create / positions,
      roles create / edit / delete / positions, guild edit / leave, member search / prune /
      `edit_me`, reactions (users, remove); tests in `tests/managers.rs`.
      (2) ✅ emojis, stickers, invites, webhooks (D-237 one webhook manager, D-238
      `WebhookClient`, D-239 one invite manager, D-240 `WebhookError::MissingToken`, D-241
      `InvalidWebhookUrl`; C-095): `manager/{emoji,sticker,invite,webhook}.rs`,
      `src/webhook.rs`, `Cache::store_emoji` / `store_sticker`.
      (3) ✅ threads and thread members (D-242), scheduled events, stage instances,
      soundboard (D-243 default sounds on the `Context`; C-096):
      `manager/{thread,scheduled_event,stage_instance,soundboard}.rs`.
      (4) ✅ application commands (D-244, global + guild managers), the application ID
      from `READY` (D-245; the managers panic before it, D-247), audit log and integrations
      on `GuildManager` (D-246), auto moderation, application emojis, standard stickers
      (C-097): `manager/{command,auto_moderation,application_emoji}.rs`. Each with methods
      on the structures where discord.js has them; 25 tests in `tests/managers.rs`.
   d. ✅ Benchmarks + extreme-load tests (D-041, C-098): `benches/managers.rs` (a cache hit
      costs ≈ 1 ns over the view and allocates nothing; a request boxes one 1.48 KB
      future), `djs-cache` `store` benches (0.02–0.3 µs), `tests/stress_managers.rs` (hits
      42 → 117 M/s on 1 → 48 tasks; REST floods within noise of the bare `Rest`; 354 B per
      member stored from REST); results in `docs/PERFORMANCE.md` ("Phase 8 — managers").
3. **Interactions** (ROADMAP Phase 8). Decided 2026-11-12: D-248 (stateless
   `InteractionExt`, one-to-one with Discord's callbacks), D-249 (one `InteractionCreate`
   event; `interaction.command()` / `component()` / `modal()` / `autocomplete()`
   accessors), D-250 (typed getters as `djs-model` methods), D-251 (a command router on the
   client, shape to be asked). Steps:
   a. ✅ The getters in `djs-model` (D-250, C-099): `crates/djs-model/src/interaction/getters.rs`
      (`CommandData::get_*`, `subcommand`, `leaf_options`, `focused_value`, `target_*`;
      `ComponentData::{users, members, roles, channels, mentionables}`; `ModalSubmitData`
      field getters; `Mentionable`) and `Interaction::{command, autocomplete, component,
      modal}` (D-249); tests in `crates/djs-model/tests/interaction_getters.rs`.
   b. ✅ `InteractionExt` (D-248, C-100) in `crates/djs-rs/src/ext.rs`: the initial
      responses, the original response and follow-ups; 3 tests in `tests/managers.rs`
      (every callback body, the webhook routes without the bot token, and an end-to-end
      `INTERACTION_CREATE` → handler → `reply` through `mock_gateway_with`).
   c. ✅ The router (D-251–D-255, C-101): `crates/djs-rs/src/router.rs` (wrappers, routes in
      the copy-on-write `Registry`, allocation-free lookup), `Client::{command, autocomplete,
      component, modal}`; tests in `tests/managers.rs` (every route kind and fallback, the
      `InteractionCreate` event alongside, both handler modes, the `:` panic).
   d. ✅ Benchmarks + extreme-load tests (D-041, C-102): `djs-model/benches/interaction.rs`,
      `djs-gateway` `flood_1m_interaction_creates_plain`, `djs-rs/tests/stress_interactions.rs`
      (+ `_mimalloc`); results in `docs/PERFORMANCE.md` ("Phase 8 — interactions"). Getters
      2–22 ns without allocations; the router ≈ 1 % over a `match`; decoding (4.8 µs of shard
      CPU per 1.9 KB interaction) dominates.
   e. ✅ The values of `Resolved` are boxed (D-256): `data.rs`, `getters.rs` (private
      unboxing helpers on `Resolved`); the 16-shard reply flood reduced to 50 000 (D-258).
      Gate passed (931 tests); `decode_dispatch` 3.54 µs, 18 allocations, **3.14 KB** (was
      8.72 KB); stress re-run under the guard in the balanced profile: 16 shards route
      447 000/s (mimalloc 627 000/s), the 50 000-reply backlog peaks at 318 MiB
      (`docs/PERFORMANCE.md`, "Re-run after D-256").
4. **Collectors and gateway requests** (ROADMAP Phase 8). Decided 2026-11-13: D-263 (streams
   with `time` / `idle` / `max` / `filter` builders and `await_*` shortcuts, no callbacks),
   D-264 (a shared index of collectors by channel / message / interaction / user / nonce /
   guild ID: one lookup per event, one atomic read without collectors), D-265 (message,
   reaction with dispose, component and modal collectors, plus gateway member requests and
   soundboard sounds), D-266 (collected events still reach the handlers and routes), D-267
   (`Sequential` mode: documented + a `tracing` warning), D-268 (`await_component` /
   `await_modal_submit` → `Result<T, CollectorError>`; `await_messages` / `await_reactions`
   → `Collected` = `Collection` + `reason()`), D-269 (short names: `message_collector`,
   `reaction_collector`, `component_collector`, `await_messages`, `await_reactions`,
   `await_component`, `await_modal_submit`), D-270 (`members(guild).fetch_all()` /
   `fetch_by_query(q).limit(n)` / `fetch_many(ids)` with `presences` and `time`, chunks by
   nonce, cached, `Collection<UserId, Arc<CachedMember>>`). Steps:
   a. ✅ The collector core and the four collectors (C-103): `crates/djs-rs/src/collector/
      {mod.rs (`Collector<T>` stream, `CollectorBuilder<T, M>` with `mode::{Stream, First,
      All}`, `AwaitFirst` / `AwaitAll`, `EndReason`, `CollectorError`, `Collected`,
      `ReactionEvent`, `CollectedReaction`, `reaction_key`, the `Context` methods by channel
      ID), index.rs (`Collectors`: entries by raw ID, atomic counts, `deliver`, deletions,
      `end_all` at shutdown; `Registration` unregisters on drop)}`; `context.rs` `Shared`
      (application ID, collectors, `sequential`, shard senders); the client's `wants` / `emit`
      feed the index; ext methods on messages, channels and interactions; tests
      `tests/collectors.rs` (scripted mock gateway, 11 tests).
   b. ✅ Gateway member requests and soundboard sounds (D-270, C-104):
      `crates/djs-rs/src/manager/gateway.rs` (`FetchMembers`, `GatewayRequestError`,
      `fetch_soundboard_sounds`); `MemberManager::{fetch_all, fetch_by_query, fetch_many}`,
      `GuildManager::fetch_soundboard_sounds`; the index's `MemberReply` (chunks and
      `RATE_LIMITED` by nonce); 4 tests in `tests/collectors.rs` (the scripted gateway answers
      opcodes 8 and 31).
   c. ✅ Benchmarks + extreme-load tests (D-041, C-105): `tests/stress_collectors.rs`; A/B
      pinned to two P-cores against `4085a57`: within noise (452 000–465 000 vs 452 000–
      461 000 messages/s), 10 000 collectors of the same or another kind cost nothing
      measurable, a busy collector ≈ 0.33 µs per message; 1M clicks 110 000/s; register +
      drop 263 ns; `fetch_all` of 100 000 members 77–117 ms (`docs/PERFORMANCE.md`,
      "Phase 8 — collectors"). `deliver` now checks the per-kind counter before locking.
5. ✅ **Intents and partials** (D-271–D-273, C-106): `[intents]` per entry of the `events!`
   macro → `Event::INTENTS`, `EventKind::{ALL, intents, name}`; `client::warn_missing_intents`
   at login, in `Client::on` while running and when a collector starts;
   `Client::required_intents()`; `Shared::intents`; the partials table in the `event` module
   docs and the README; tests: a unit test of the warning (captured `tracing` output) and
   `required_intents_cover_handlers_routes_and_collectors`.
6. ✅ **Examples written** (D-274–D-276, C-107): `examples/common/mod.rs` (`.env`, logging,
   Ctrl+C, `DISCORD_TEST_GUILD_ID`, `--cleanup`), `ping.rs` on it, `slash_commands.rs`,
   `components.rs`, `moderation.rs`, `advanced.rs`; README "More examples", `.env.example`
   (`DISCORD_TEST_GUILD_ID`, `DISCORD_WEBHOOK_URL` optional). Gate passed.
   **Live runs (D-275, all four approved by the owner, 2026-11-13, 5 minutes each):**
   `slash_commands` — login, 6 guild commands registered, `/slow` (defer → edit → follow-up)
   and `/feedback` (modal via `await_modal_submit`) used by the owner, `--cleanup` removed the
   commands; `moderation` — privileged intents accepted, edit (×2) and deletion logs with the
   cached old content posted, `--cleanup` done; `components` and `advanced` — logged in and
   shut down cleanly (Shard 0/1 ready, presence set), but **not exercised** (no messages in
   their windows). No handler error in any run. The 5 bot messages were deleted with a
   temporary `zz_` probe (deleted, never committed); the owner's 2 test messages were kept
   (deleted afterwards on request, D-281).
   The owner chose not to rerun them and had their 2 messages deleted too (D-281). ✅
   Phase 8 closed.
7. **The serenity comparison ✅ (2026-11-13)** (D-277–D-282, C-108, C-109):
   `benches/vs-serenity/` (own workspace and `Cargo.lock`, serenity `=0.12.5` with default
   features, the main workspace's release profile) and `docs/VS_SERENITY.md`, generated by
   `benches/vs-serenity/report.mjs` from the committed `results.json` (raw runs in the
   git-ignored `out/`). Final numbers (equal profiles, medians of 3; peaks 83 °C / 66 °C):
   - **Performance:** decode `MESSAGE_CREATE` 1.61 vs 7.94 µs (4.9×), `GUILD_CREATE` 0.73 vs
     4.51 ms (6.2×); flood with equal caches 243,081 vs 61,228 events/s (4.0×, CPU 7.70 vs
     22.6 µs/event); memory idle 6.6 vs 10.0 MiB, 500k members 203.5 vs 998.4 MiB (4.9×);
     REST 255,872 vs 114,124 req/s (2.2×; serenity without TLS and limiter 159,421, so
     1.6×).
   - **Resources:** 85 vs 152 dependencies; clean release build 46.6 vs 73.7 s; stripped
     release binary 8.7 vs 10.8 MiB; **serenity rebuilds faster after a `main.rs` edit**
     (release 9.91 vs 4.95 s, debug 0.79–1.38 vs 0.58 s) — an open question in
     `docs/DECISIONS.md`.
   - **Reliability:** equal on a dropped connection, op 7, close 4000 (resume in ~3 ms) and
     4004 (both stop); zombie: djs-rs resumes after 1.8 s, nothing lost, serenity restarts
     the shard (new session, 6.0 s, ~6000 events lost; its restart waits the full 5 s
     `ShardManager::shutdown` timeout); invalid session: 3.1 s (djs-rs's IDENTIFY queue, 5 s
     after the previous IDENTIFY) vs 5.0 s; 429 of 0.5 s: djs-rs waits 0.55 s, serenity
     1.0 s (whole-second header), none early; 502s: djs-rs retries (40/40), serenity does
     not (32/40, error "Could not decode json").
   - The first S1 run had built the serenity runners without fat LTO; it was re-run with
     equal profiles (serenity gained ~6 % in decode and flood, idle memory 12.5 → 10.0 MiB).
   - **Every run inside `unshare -rn`** (C-108: one request reached Discord in the first
     smoke test, serenity's rate limiter ignores its proxy).
8. **Next: fixes before Phase 9 (D-283).**
   - ✅ **Rebuild time** (D-287, D-288, D-289; `docs/PERFORMANCE.md`, "Rebuild time after
     an edit"): the REST futures are built in `djs-rest` (`c88492a`), the client loop in
     `djs-rs` through the `Compiled<B>` function table set by the non-generic
     `Client::builder` (`77e8b0e`), and the managers' cache code goes through
     `dyn CacheOps` (`crates/djs-rs/src/cache_ops.rs`, created in `Context::new`, which only
     the client loop calls) with the backend-independent part of the context in
     `context::Core`. Default release profile, after a `main.rs` edit: `!ping` bot
     10.8 → 0.77 s, moderation example 12.7 → 2.1 s; flood and `fetch_all` unchanged, REST
     +0.22 µs CPU per request (−2.2 % throughput in the default profile, equal with fat
     LTO). The handler spawn stays unboxed (D-288: boxing would give 1.7 s but −1.9 % flood
     with a cache). The scratch workspace for these measurements is the git-ignored
     `.reference/tmp-ab/` (`rebuild.sh`, `ir.sh`, `flood.sh`, `rest.sh`, `build*.sh`).
     `docs/VS_SERENITY.md` re-measured on 2026-11-13 after D-289 (`run.sh` and `build.sh`,
     both under the guard; the laptop ran warmer, so both libraries lost 3–10 %): release
     rebuild 0.73 s against serenity's 5.32 s (7.3×), debug about the same; clean release
     build 51.2 against 76.4 s; REST 2.1× (a same-session fat A/B of `b0bb7c6` against
     D-289: −2.1 %, +0.25 µs CPU per request).
   - **Ergonomics gaps** (C-107), decided 2026-11-13 (D-290–D-293), one commit each:
     a. ✅ Setters on the message bodies + `From` conversions (D-290, D-291, C-110):
        `crates/djs-model/src/rest/setters.rs`, tests `crates/djs-model/tests/message_bodies.rs`,
        the examples rewritten with them.
     b. ✅ `ctx.shutdown()` (D-293): the shutdown `watch::Sender` moved from the client's
        `Inner` into `context::Shared`, which the client and every context share; test
        `end_to_end.rs::a_handler_shuts_the_client_down`.
     c. ✅ Member permissions (D-292, D-294 timeouts, D-295 implicit denials, C-111): a pure computation in `djs-model`,
        `Cache::member_permissions(guild, user)` / `member_permissions_in(channel, user)`,
        and `member.permissions(&ctx)` / `permissions_in(&ctx, channel)` in `djs-rs`
        (`Option<Permissions>`). `crates/djs-model/src/permissions.rs`,
        `crates/djs-cache/src/permissions.rs`, `MemberExt` in `crates/djs-rs/src/ext.rs` +
        `CacheOps`; tests `djs-model/tests/permissions.rs`, `djs-cache/tests/permissions.rs`,
        `end_to_end.rs`; benchmark in `docs/PERFORMANCE.md` ("Member permissions").
   - **D-283 is done.** Next: Phase 9.
9. **Then Phase 9 — multi-process sharding** (D-018, D-284–D-286): `djs_rs::sharding`
   behind the `sharding` feature, TCP 127.0.0.1 + length-prefixed JSON with a per-process
   token, built-in values + named typed commands instead of `broadcastEval`. Still to ask:
   the process model (re-spawn the same executable or a given binary), shards per process,
   respawn policy, IDENTIFY coordination across processes. **Decided 2026-11-14:** D-296
   (the same executable, role from an environment variable), D-297 (`shards_per_process`,
   default 1), D-298 (restart with a growing delay), D-299 (the manager is the IDENTIFY
   queue), D-300 (`sharding::process()` → assignment → `ClientBuilder::cluster`, else the
   manager), D-301 (`ClusterManager`, `Cluster`, `ClusterClient` via `client.cluster()`),
   D-302 (REST limits per process), D-303 (order: IPC + spawning → IDENTIFY queue →
   restarts → values and commands → benchmarks), D-304 (`tokio` `process`), D-305 (the
   manager asks Get Gateway Bot once for every process), D-306 (restart failures by default,
   `Restart` enum, `restart_delay`), D-307–D-310 (commands from the manager and the
   clusters, the `ClusterCommand` trait, one result per cluster, `ClusterStats`). Steps, one
   commit each:
   1. ✅ IPC, starting the processes, shard assignment (D-285, D-296, D-297, D-300, C-112):
      `crates/djs-rs/src/sharding/{mod.rs (process(), ClusterError), ipc.rs (frames,
      ToManager / ToCluster, Token), cluster.rs (ClusterAssignment, the process's
      connection: a shutdown or a lost manager → `Shared::shutdown`), manager.rs
      (ClusterManager + builder, Cluster, ClusterEvent(s); supervise → start / wait /
      exited, accept / handshake)}`; `ClientBuilder::cluster` and the connection in
      `Client::run`; `Error::Cluster`. Tests: unit tests in the three modules, and
      `crates/djs-rs/tests/sharding.rs`, which starts the test binary again as the cluster
      processes (an `#[ignore]`d `cluster_process` test) against a mock REST API and gateway.
   2. ✅ The manager as the IDENTIFY queue (D-299, D-305, C-113): `ToManager::Identify` /
      `ToCluster::Identify`; `cluster.rs` `ManagerQueue` (the process's `IdentifyQueue`,
      from `Connection::identify_queue`, used by `Client::run`); `manager.rs` `Session`
      (token, `Gateway`, one `LocalIdentifyQueue`) and `serve` (turns per connection,
      dropped with it); the manager's single Get Gateway Bot and the session-start check;
      `Client::plan` takes the URL and `max_concurrency` from the assignment. Tests:
      `tests/sharding.rs::processes_identify_in_turn_by_bucket` (4 shards, 2 processes,
      `max_concurrency` 2: each bucket's shards ≥ 5 s apart across processes),
      `too_few_session_starts_start_nothing`, unit tests in `cluster.rs` / `manager.rs`.
   3. ✅ Restarts with a growing delay (D-298, D-306, C-114): `manager.rs` `Run` (one
      `spawn`: `start(id)`, `wait`, `exited`, `restart_later`, the pure `restart_delay`),
      `Restart`, `ClusterManagerBuilder::{restart, restart_delay}`,
      `ClusterEvent::Restarting`. Tests: `tests/sharding.rs` (`sh -c "exit N"` for the
      policies and delays, a `cluster_process` that crashes once after READY and comes back,
      identifying again ≥ 5 s later in the same queue), a unit test for the delays.
   4. Built-in values and named commands (D-286, D-307–D-310), two commits:
      a. ✅ Named commands (C-115): `sharding/command.rs` (`ClusterCommand`,
         `ClusterReply`, `Failure`, `Commands`, `erase`), `sharding/cluster_client.rs`
         (`ClusterClient`, `Table`), `Link` in `cluster.rs` (a process's own requests,
         kept in `Shared::link`), `ClusterManager::{broadcast, send, ask}` and the routing
         in `serve`, `command_timeout`, `Client::cluster()` and the handler table in the
         client's `Inner`. Tests: unit tests in `command.rs` / `manager.rs`,
         `tests/sharding.rs::clusters_answer_commands`.
      b. ✅ `ClusterStats` (D-310, C-116): `sharding/stats.rs` (`Stats`, `ClusterStats`,
         `ShardStats`, `ShardStatus`, `ShardSlot`, `Reporter`, `resident_memory`); the
         client's `shard_slots` (set in `run` for a cluster process, written by
         `run_shard` through a `Reporter`), the built-in command in `Table`, `stats()` on
         both sides. Test: `tests/sharding.rs::clusters_report_their_stats`.
   5. ✅ Benchmarks + extreme-load tests (D-041): the fix found while planning them
      (C-117, `2d0ed1e`); `crates/djs-rs/benches/sharding.rs` (the bench binary is also the
      cluster processes); `crates/djs-rs/tests/stress_sharding.rs` (64 processes, command
      floods, payloads, a restart storm, bogus connections; shared mocks in
      `tests/support/sharding.rs`). Results in `docs/PERFORMANCE.md` "Phase 9": 64
      processes connected in 57 ms and identified in 15.3 s at `max_concurrency` 16, 6.8 MiB
      each; 259k cluster answers/s; no leaked descriptor or pending request.
   **Phase 9 is done.**
10. **Next: Phase 10 — `djs-voice`** (≈ `@discordjs/voice`, D-017: voice gateway, UDP/RTP,
   encryption modes, DAVE E2EE, Opus, players and resources, receiving audio).
   **Decided 2026-11-14:** D-311 (our own stack shaped like `@discordjs/voice`, crates for
   the hard parts; DAVE is mandatory since 2026-03-01), D-312 (`davey` for DAVE), D-313
   (`audiopus` for Opus), D-314 (RustCrypto `chacha20poly1305` + `aes-gcm`), D-315
   (standalone `djs-voice` with an adapter trait + djs-rs `voice` feature), D-316 (API
   shaped like **songbird**: a call per guild, a track queue, track handles; names asked
   per step), D-317 (inputs: FFmpeg process, native Ogg/Opus, PCM), D-318 (order:
   connection + DAVE + ready Ogg/Opus → player → receiving → benchmarks), D-319
   (`VoiceManager`, `Call`, later `Track`/`TrackHandle`/`TrackQueue`; `client.voice()`),
   D-320 (`Call` is a cheap-clone handle, `&self` methods), D-321 (events as streams:
   `call.events()` → `CallEvent`), D-322 (`join` returns once gateway + UDP + DAVE are
   ready; 10 s timeout). `davey` 0.2 checked: MIT, safe Rust, openmls 0.8, sync
   `DaveSession` API (MSRV 1.89 to verify on the first build), D-323 (step 1 already has
   `Input` (Ogg/Opus) and `call.play(input) -> TrackHandle`), D-324 (`VoiceGateway` trait
   + `manager.process(&event)`), D-325 (`davey =0.2.0`, `chacha20poly1305` 0.11,
   `aes-gcm` 0.11, our own Ogg page reader). **Step 1a in progress:** `crates/djs-voice`
   exists (manifest, README as crate docs, empty `lib.rs`; workspace deps added) and builds
   on MSRV 1.89 **with `davey` too**. `davey`'s tree has four `hpke-rs*` crates under
   MPL-2.0: D-326 allows MPL-2.0 for exactly those in `deny.toml` (+ README/NOTICE note);
   D-327 keeps the AEAD crates at 0.11 (openmls compiles 0.10 too).
   **Next: implement step 1** in commits:
   (a) the crate skeleton (done), dependencies (`davey` back in with the D-326 exception)
   and the Ogg reader;
   (b) the voice gateway v8 protocol types and the transport encryption (unit-tested with
   known vectors); (c) the connection task (gateway, UDP, IP discovery, heartbeats, resume)
   against a local mock voice server; (d) DAVE through `davey`; (e) `VoiceManager`,
   `VoiceGateway`, `Call`, `CallEvent`, `Input`, `TrackHandle`, playback pacing; (f) djs-rs
   `voice` feature; (g) a live test only with the owner's go-ahead (D-275). Live tests need the
   owner's go-ahead (D-275) and a voice channel on the test guild.

> **Heavy runs (D-257, D-259–D-262):** the owner's laptop hard-froze on 2026-11-13 under
> assistant load. Every cargo build/test/bench/stress test goes through
> `.reference/tools/safe.sh` (git-ignored, persists): AC (or a battery at 80 %+), cool start,
> `MemoryMax=6G` without swap, the fan forced to full speed during the run (it refuses to
> start without fan control). It stops the heavy command only on a real danger (D-262): the
> package at 99 °C for 10 s, `acpitz` within 3 °C of its 103 °C critical trip, a battery
> < 80 % without AC, < 1 GiB RAM, or its timeout; throttling and ≥ 95 °C only warn. The
> owner runs the **balanced** profile and wants peaks below 95 °C: a clean gate on all 12 CPUs
> peaked at 79 °C, so no core limit. The LAN monitor `.reference/tools/monitor.py`
> (http://192.168.0.20:8765) shows temperatures, load, the runs on a time axis and a
> comparison table; it logs to `.reference/tools/monitor-log.csv`. Stress tests one at a time
> (D-258).

- Open optimization candidates from Phase 3 (not blocking): the REST future size and JSON
  body pre-sizing (`docs/PERFORMANCE.md`, "Findings … (Phase 3)").

## Blocked on the owner
- Nothing.
