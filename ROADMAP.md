# Roadmap

The phased plan for djs-rs. Checkboxes are updated as work lands on `main`.

> **Status of this plan:** phase *order* is a proposal awaiting the owner's approval
> (see `PROGRESS.md`). Scope decisions are in [`docs/DECISIONS.md`](docs/DECISIONS.md).

Legend: `[x]` done · `[~]` in progress · `[ ]` not started

---

## Phase 0 — Foundation
- [x] Decision log, CLAUDE.md, roadmap, progress file
- [x] Licenses (MIT OR Apache-2.0), NOTICE, README, CONTRIBUTING, CHANGELOG
- [ ] Cargo workspace skeleton with all crates (compiling, documented)
- [ ] Workspace lints, rustfmt/clippy config, cargo-deny config
- [ ] GitHub Actions CI (fmt, clippy, tests ×3 OS, docs, MSRV, cargo-deny)
- [ ] `docs/ARCHITECTURE.md`

## Phase 1 — Small standalone crates
Independent of the Discord API surface; good warm-up and immediately useful.
- [ ] `djs-collection` — `Collection<K, V>` with the discord.js `Collection` API surface
      (`find`, `filter`, `partition`, `sweep`, `first`, `last`, `random`, `sort`, …)
- [ ] `djs-formatters` — mentions, markdown (bold, italic, code blocks, spoilers, quotes,
      headings, lists, hyperlinks), timestamps with styles, escaping
- [ ] `djs-util` — shared helpers (as needed by other crates)

## Phase 2 — `djs-model` (≈ discord-api-types v10)
- [ ] Core primitives: Snowflake/ID types, timestamps, colours, image hashes, CDN URLs
- [ ] Bitflags: permissions, intents, message/user/channel/guild flags
- [ ] Users, members, roles, emojis, stickers
- [ ] Guilds (incl. onboarding, welcome screen, widgets, templates, scheduled events, auto-moderation, audit log)
- [ ] Channels & threads, forums/media, stage instances, invites, webhooks
- [ ] Messages, embeds, attachments, reactions, polls, message snapshots/forwarding
- [ ] Components (classic + Components V2), modals
- [ ] Interactions & application commands (slash, user, message, autocomplete, entry point), permissions
- [ ] Applications, entitlements, SKUs, subscriptions (monetization), soundboard
- [ ] Gateway opcodes, payloads, all dispatch events, close codes
- [ ] (De)serialization tests against real JSON fixtures; benchmarks

## Phase 3 — `djs-rest` (≈ @discordjs/rest)
- [ ] hyper-based HTTP client, TLS backends (rustls / native-tls)
- [ ] Rate limiting: per-route buckets, global limit, invalid-request limit, 429 handling
- [ ] Request/response pipeline, retries, timeouts, audit-log reason header
- [ ] Multipart uploads (attachments)
- [ ] All v10 REST endpoints as typed methods
- [ ] CDN URL builder
- [ ] Mock-server tests; benchmarks

## Phase 4 — `djs-gateway` (≈ @discordjs/ws)
- [ ] Single shard: connect, identify, heartbeat (+ jitter, zombie detection), resume, reconnect
- [ ] Transport compression: zstd-stream (default), zlib-stream (feature)
- [ ] Fast event parsing (serde_json / simd-json), lazy/zero-copy where measurable
- [ ] Gateway rate limiting (120 commands / 60 s), identify concurrency (`max_concurrency`)
- [ ] In-process sharding manager (auto shard count from `/gateway/bot`)
- [ ] Gateway commands: presence update, request guild members, voice state update, soundboard
- [ ] Tests (mock gateway), benchmarks

## Phase 5 — `djs-cache`
- [ ] Cache trait (pluggable backend) + DashMap default backend
- [ ] Per-resource enable/disable and size limits (≈ `makeCache`, `Options.cacheWithLimits`)
- [ ] Sweepers (≈ `sweepers` option), partials support
- [ ] Event → cache update logic for every dispatch event
- [ ] Memory/size tests, benchmarks

## Phase 6 — `djs-builders` (≈ @discordjs/builders)
- [ ] `EmbedBuilder` with Discord limits validation
- [ ] Slash command builders (options, subcommands, groups, localizations, choices)
- [ ] Context menu command builders
- [ ] Component builders (buttons, select menus, action rows, Components V2), modal builders
- [ ] Validation errors mirroring Discord limits

## Phase 7 — `djs-rs` high-level client (≈ discord.js)
- [ ] `Client` builder, login, ready lifecycle, graceful shutdown
- [ ] Event system (`client.on` / `client.once`, typed `Events`), handler concurrency
- [ ] Structures with methods (`Message::reply`, `Guild::members`, `Interaction::reply`, …)
- [ ] Managers (fetch/cache semantics like discord.js managers)
- [ ] Interactions: replies, deferrals, follow-ups, modals, autocomplete
- [ ] Collectors (message, reaction, interaction) and `await_*` helpers
- [ ] Partials, intents/privileged intents guidance
- [ ] Examples directory covering common bot patterns

## Phase 8 — Multi-process sharding
- [ ] `ShardingManager` spawning processes, IPC, `broadcastEval`-style messaging, respawn

## Phase 9 — `djs-voice` (≈ @discordjs/voice)
- [ ] Voice gateway, UDP/RTP, encryption modes, DAVE E2EE
- [ ] Opus encode/decode, audio players & resources, receiving audio

## Cross-cutting (continuous)
- [ ] Guide / book (topic TBD with owner)
- [ ] discord.js → djs-rs migration guide (API mapping table)
- [ ] Benchmarks vs discord.js / serenity / twilight
- [ ] Release process & crates.io publishing (owner decision)
