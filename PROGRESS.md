# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice: ✅ done** (live test passed, benchmarks + extreme-load tests done,
D-041).
**Phase 2 — full `djs-model` coverage: ✅ done** (D-039; benchmarks + stress tests done).
**Phase 3 — full `djs-rest` coverage: ✅ done 2026-11-12** (every documented REST route, OAuth2,
CDN URLs; benchmarks + extreme-load tests in `docs/PERFORMANCE.md`).
**Next: Phase 4 — full `djs-gateway` coverage** (see "Next step" below; starts with owner
decisions).

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

## Next step — Phase 4 (`djs-gateway` full coverage, ROADMAP)
1. Ask the owner first (significant decisions): zlib-stream support (new dependency, e.g.
   `flate2` with a zlib backend, behind a feature), simd-json (new dependency, feature) and
   whether zero-copy/borrowed decoding is pursued; the sharding manager's API (auto shard
   count, `max_concurrency` buckets, resharding); the gateway send rate limiter's behaviour
   when full (wait vs error).
2. Then, one commit each: send rate limiting (120 per 60 s), identify concurrency, the
   sharding manager, the gateway commands (presence update, request guild members, voice
   state update, request soundboard sounds), mock-gateway tests.
3. When Phase 4 is done: gateway benches + extreme-load tests (D-041) in
   `docs/PERFORMANCE.md`.
- Open optimization candidates from Phase 3 (not blocking): the REST future size and JSON
  body pre-sizing (`docs/PERFORMANCE.md`, "Findings … (Phase 3)").

## Blocked on the owner
- Nothing.
