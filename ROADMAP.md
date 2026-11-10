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

## Phase 1 — Vertical slice: a working "ping-pong" bot
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
- [ ] `djs-gateway` core: single shard — connect, zstd-stream, hello/identify, heartbeat with
      jitter + ACK tracking, resume/reconnect, dispatch → typed events
- [ ] `djs-rs` core: `Client::builder`, `client.on(Events::MessageCreate, …)`, `msg.reply`,
      `login`, graceful shutdown
- [ ] `examples/ping.rs` working against real Discord; mock-based tests in CI
- [ ] Architecture review with the owner before widening

## Phase 2 — `djs-model` full coverage (≈ discord-api-types v10)
- [ ] Bitflags: permissions, message/user/channel/guild/application flags
- [ ] Users, members, roles, emojis, stickers
- [ ] Guilds (onboarding, welcome screen, widgets, templates, scheduled events,
      auto-moderation, audit log, integrations, bans)
- [ ] Channels & threads, forums/media, stage instances, invites, webhooks
- [ ] Messages, embeds, attachments, reactions, polls, message snapshots/forwarding
- [ ] Components (classic + Components V2), modals
- [ ] Interactions & application commands (slash, user, message, autocomplete, entry point),
      command permissions
- [ ] Applications, entitlements, SKUs, subscriptions, soundboard, lobbies
- [ ] Every gateway dispatch event and close code
- [ ] (De)serialization tests against real JSON fixtures; benchmarks

## Phase 3 — `djs-rest` full coverage (≈ @discordjs/rest)
- [ ] Complete rate limiting: invalid-request limit (10k/10 min), burst handler for interaction
      callbacks, per-route quirks (old message deletes, reactions), hash lifetime sweeps
- [ ] Retries, timeouts, audit-log reason header, `native-tls` feature
- [ ] Multipart uploads (attachments)
- [ ] Every v10 REST endpoint as a typed method
- [ ] CDN URL builder
- [ ] Mock-server tests; benchmarks

## Phase 4 — `djs-gateway` full coverage (≈ @discordjs/ws)
- [ ] zlib-stream feature, simd-json feature, zero-copy parsing where measurable
- [ ] Gateway send rate limiting (120 / 60 s), identify concurrency (`max_concurrency`)
- [ ] In-process sharding manager (auto shard count, resharding)
- [ ] Gateway commands: presence update, request guild members, voice state update,
      soundboard sounds
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
