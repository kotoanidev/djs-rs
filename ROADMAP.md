# Roadmap

The phased plan for djs-rs. Checkboxes are updated as work lands on `main`.
Build order follows **D-020 (vertical slice first)**; scope decisions are in
[`docs/DECISIONS.md`](docs/DECISIONS.md).

Legend: `[x]` done · `[~]` in progress · `[ ]` not started

---

## Phase 0 — Foundation ✅
- [x] Decision log, CLAUDE.md, roadmap, progress file
- [x] Licenses (MIT OR Apache-2.0), NOTICE, README, CONTRIBUTING, CHANGELOG
- [x] Cargo workspace skeleton with all crates (compiling, documented)
- [x] Workspace lints, rustfmt/clippy config, cargo-deny config
- [x] GitHub Actions CI (fmt, clippy, tests ×3 OS, docs, MSRV, cargo-deny) — first run green
- [x] `docs/ARCHITECTURE.md`

## Phase 1 — Vertical slice: a working "ping-pong" bot ✅
Goal: the smallest end-to-end bot that touches every layer, to validate the architecture
before the API surface is widened. Everything here is built to final quality (docs, tests),
just with narrow scope.
- [x] Owner decisions for the slice (IDs, strings, errors, logging, TLS crypto provider,
      handler model, dependency set)
- [x] `djs-model` core: Snowflake/ID types, intents, timestamps; minimal `User`, `Message`,
      `Guild` (unavailable/partial), gateway opcodes + Hello/Identify/Ready/Resume/
      MessageCreate payloads
- [x] `djs-rest` core: hyper client + rustls, auth, per-route rate limiting basics,
      `GET /gateway/bot`, `POST /channels/{id}/messages`
- [x] `djs-gateway` core: single shard — connect, zstd-stream, hello/identify, heartbeat with
      jitter + ACK tracking, resume/reconnect, dispatch → typed events
- [x] `djs-rs` core: `Client::builder`, `client.on(Events::MessageCreate, …)`, `msg.reply`,
      `login`, graceful shutdown
- [x] `examples/ping.rs` — end-to-end mock tests pass in CI; live run against real Discord
      passed (2026-11-10)
- [x] Architecture review with the owner before widening (D-039)
- [x] Benchmarks + extreme-load tests (D-041) — results in `docs/PERFORMANCE.md`

## Phase 2 — `djs-model` full coverage (≈ discord-api-types v10)
- [x] Bitflags: permissions, message/user/channel/guild/application flags (+ attachment, embed,
      member, role, system channel, invite, SKU, activity)
- [x] Users (+ connections, role connections), members, roles, emojis, stickers
- [x] Guilds (onboarding, welcome screen, widgets, templates, scheduled events,
      auto-moderation, integrations, bans, audit log)
- [x] Channels & threads, forums/media, stage instances, invites, webhooks
- [x] Messages, embeds, attachments, reactions, polls, message snapshots/forwarding
      (+ components and `resolved`, D-062)
- [x] Components (classic + Components V2)
- [x] Interactions & application commands (slash, user, message, autocomplete, entry point),
      command permissions, interaction responses and modals (D-068)
- [x] Applications (+ teams, role connection metadata), entitlements, SKUs, subscriptions,
      voice states/regions, soundboard, lobbies, application identity profiles (D-072)
- [x] Every gateway dispatch event and close code, gateway commands, presences (D-073..D-075)
- [x] Webhook events, incl. Social SDK events (D-076, D-078, D-079); RPC is out of scope (D-077)
- [x] (De)serialization tests against real JSON fixtures; benchmarks and stress tests (D-041, D-080)

## Phase 3 — `djs-rest` full coverage (≈ @discordjs/rest)
- [x] Complete rate limiting: invalid-request limit (10k/10 min), burst handler for interaction
      callbacks, per-route quirks (old message deletes; reaction routes share one template),
      hash lifetime sweeps
- [x] Retries, timeouts, audit-log reason header (D-086), `native-roots` feature (D-088, D-100)
- [x] Strict global limiter (D-043 → sliding log, D-089) instead of fixed one-second windows
- [x] Multipart uploads (attachments, D-087, D-093..D-096)
- [x] API errors with JSON error codes and field errors (D-097..D-099)
- [x] Every v10 REST endpoint as a typed method (order: D-102; tri-state `Patch<T>`, D-101;
      query structs, D-092) — `scripts/route_coverage.py` lists only the routes left out on
      purpose (D-105, D-116, D-120, disabled batch command permissions)
- [x] CDN URL builder (`djs_rest::cdn`, D-137–D-139)
- [x] Mock-server tests; benchmarks and extreme-load tests (`docs/PERFORMANCE.md`, Phase 3)

## Phase 4 — `djs-gateway` full coverage (≈ @discordjs/ws)
- [ ] zlib-stream feature, simd-json feature, zero-copy parsing where measurable
- [x] Gateway send rate limiting (120 / 60 s; D-142, C-067)
- [x] Identify concurrency (`max_concurrency`; D-144)
- [x] In-process sharding: `create_shards`, auto shard count in `Client` (D-145, D-146)
- [ ] Resharding while running (open question)
- [x] Gateway commands: any command through `MessageSender::command`; presence kept across
      sessions (D-148). Member/soundboard collectors and voice state helpers in Phase 6 (D-149)
- [ ] Mock gateway tests, benchmarks

## Phase 5 — `djs-cache`
- [ ] Cache trait (pluggable backend) + DashMap default backend
- [ ] Per-resource enable/disable and size limits (≈ `makeCache`)
- [ ] Sweepers (≈ `sweepers` option), partials support
- [ ] Event → cache update logic for every dispatch event
- [ ] Memory/size tests, benchmarks

## Phase 6 — `djs-builders` (≈ @discordjs/builders)
- [ ] `EmbedBuilder` with Discord limit validation
- [ ] Slash and context-menu command builders (options, subcommands, localizations, choices)
- [ ] Component builders (buttons, select menus, action rows, Components V2), modal builders

## Phase 7 — Small standalone crates
- [ ] `djs-collection` — `Collection<K, V>` with the discord.js `Collection` API surface
- [ ] `djs-formatters` — mentions, markdown, timestamps, escaping
- [ ] `djs-util` — shared helpers (filled as other crates need them)

## Phase 8 — `djs-rs` full high-level client (≈ discord.js)
- [ ] All events, structures with methods, managers (fetch/cache semantics)
- [ ] Interactions: replies, deferrals, follow-ups, modals, autocomplete
- [ ] Collectors (message, reaction, interaction) and `await_*` helpers
- [ ] Partials, intents/privileged intents guidance
- [ ] Examples directory covering common bot patterns

## Phase 9 — Multi-process sharding (D-018)
- [ ] `ShardingManager` spawning processes, IPC, `broadcastEval`-style messaging, respawn

## Phase 10 — `djs-voice` (≈ @discordjs/voice, D-017)
- [ ] Voice gateway, UDP/RTP, encryption modes, DAVE E2EE
- [ ] Opus encode/decode, audio players & resources, receiving audio

## Cross-cutting (continuous)
- [ ] Guide / book (format TBD with owner)
- [ ] discord.js → djs-rs migration guide (API mapping table)
- [ ] Benchmarks vs discord.js / serenity / twilight
- [ ] First crates.io release (D-022)

> Phase 7 crates are independent and may be pulled earlier if the owner wants.
