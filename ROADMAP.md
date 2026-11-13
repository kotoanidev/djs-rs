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
- [x] zlib-stream feature (D-140)
- [x] Faster JSON parsers measured (simd-json, sonic-rs): no gain, serde_json stays (D-150)
- [ ] Zero-copy parsing where measurable
- [x] Gateway send rate limiting (120 / 60 s; D-142, C-067)
- [x] Identify concurrency (`max_concurrency`; D-144)
- [x] In-process sharding: `create_shards`, auto shard count in `Client` (D-145, D-146)
- [ ] Resharding while running (open question)
- [x] Gateway commands: any command through `MessageSender::command`; presence kept across
      sessions (D-148). Member/soundboard collectors and voice state helpers in Phase 6 (D-149)
- [x] Mock gateway tests, benchmarks and extreme-load tests (`docs/PERFORMANCE.md`, Phase 4)

## Phase 5 — `djs-cache`
- [x] Cache trait (pluggable backend) + DashMap default backend (D-155, D-158)
- [x] Per-resource enable/disable and size limits (≈ `makeCache`; D-156, D-157, D-159)
- [x] Sweepers (≈ `sweepers` option; D-167–D-169). Partials are a Phase 8 (`djs-rs`) concern
- [x] Event → cache update logic for every dispatch event that carries cacheable state
      (D-160–D-166, C-074, C-075)
- [x] djs-rs `Client` integration: opt-in cache, filled before handlers, sweeps run by the
      client, custom backends (D-170–D-172, C-077)
- [x] Memory/size tests, benchmarks (D-041; `docs/PERFORMANCE.md` Phase 5, `docs/VS_NODE.md`
      with equal caches, D-174), compact cached types (D-177)

## Phase 6 — `djs-builders` (≈ @discordjs/builders)
- [x] `EmbedBuilder` with Discord limit validation (D-186–D-191; limits checked on the live API)
- [x] Slash and context-menu command builders (options, subcommands, localizations, choices;
      D-192–D-196, rules checked on the live API)
- [x] Component builders (buttons, select menus, action rows, Components V2), modal builders
      (D-197–D-207; message and V2 rules checked on the live API, modal rules from the docs)
- [x] Benchmarks + extreme-load tests of the builders (D-041, `docs/PERFORMANCE.md`)

## Phase 7 — Small standalone crates ✅
- [x] `djs-collection` — `Collection<K, V>` with the discord.js `Collection` API surface
      (D-209, D-210, D-214–D-217; benchmarked)
- [x] `djs-formatters` — mentions, markdown, timestamps, escaping (D-211–D-213; benchmarked)
- [x] `djs-util` — shared helpers, filled only when two or more crates need one; empty for
      now (D-218)

## Phase 8 — `djs-rs` full high-level client (≈ discord.js)
- [x] All events, structures with methods, managers (fetch/cache semantics)
- [x] Interactions: replies, deferrals, follow-ups, modals, autocomplete (D-248–D-255; typed
  getters, `InteractionExt`, a command router; benchmarked)
- [x] Collectors (message, reaction, interaction) and `await_*` helpers (D-263–D-270; with
  gateway member and soundboard requests; benchmarked)
- [x] Partials, intents/privileged intents guidance (D-271–D-273: `Event::INTENTS`, a login
  warning, `required_intents()`, a partials table)
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
