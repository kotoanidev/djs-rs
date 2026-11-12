# Decision log

Every significant decision in djs-rs is recorded here. Decisions marked **Owner** were made by
the project owner and are binding; changing them requires asking the owner again.

Format: `D-NNN` — title. Date, who decided, the decision, alternatives that were considered,
and why.

---

## Owner decisions

### D-001 — Project name: `djs-rs`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** The project, repository and main crate are named `djs-rs` (import path
  `djs_rs`). Sub-crates use the `djs-` prefix (`djs-rest`, `djs-gateway`, …). All names were
  verified to be free on crates.io on 2026-11-10.
- **Alternatives:** `djs`, `kotocord`, `discord-kit`.

### D-002 — Public GitHub repository, work directly on `main`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Public repository `github.com/kotoanidev/djs-rs`. Every significant change is
  committed to `main` and pushed immediately so work survives interrupted sessions.
- **Alternatives:** private repository; feature branches + PRs.

### D-003 — License: `MIT OR Apache-2.0`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Dual-licensed under MIT or Apache-2.0 at the user's option (Rust ecosystem
  convention, same as tokio and serde).
- **Alternatives:** Apache-2.0 only (same as discord.js), MIT only, no license.

### D-004 — discord.js is a behavioural reference only
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** discord.js (Apache-2.0) and the official Discord API documentation are used as
  references for behaviour and API surface. All code and documentation in this repository is
  written from scratch; nothing is copied or translated verbatim. `NOTICE` acknowledges
  discord.js as the inspiration.
- **Why:** keeps the dual license clean.
- **Alternatives:** adapting fragments of discord.js under Apache-2.0 with attribution.

### D-005 — Toolchain: rustup stable, edition 2024, fixed MSRV 1.85
> **Amended by D-031:** MSRV raised to **1.89**.

- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Rust installed via rustup (stable). Edition 2024. MSRV fixed at **1.85** (first
  release with edition 2024); raised only deliberately, with owner approval and a CHANGELOG note.
  Cargo resolver v3 (MSRV-aware dependency resolution).
- **Alternatives:** Fedora `dnf` packages; MSRV = stable − 2; latest stable only.

### D-006 — API style: hybrid
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Low-level layers (`djs-model`, `djs-rest`, `djs-gateway`, `djs-cache`) are
  idiomatic Rust. On top of them, `djs-rs` provides a high-level layer that closely mirrors the
  discord.js developer experience. Reference shape approved by the owner:

  ```rust,ignore
  // high level (discord.js-like)
  let client = Client::builder(token)
      .intents(Intents::GUILDS | Intents::GUILD_MESSAGES)
      .build();

  client.on(Events::MessageCreate, |ctx, msg| async move {
      if msg.content == "!ping" {
          msg.reply(&ctx, "Pong!").await?;
      }
      Ok(())
  });

  // low level (separate crate)
  let rest = djs_rest::Rest::new(token);
  rest.create_message(channel_id, &body).await?;
  ```
- **Why:** mirrors how discord.js itself is layered (`@discordjs/rest`, `@discordjs/ws` →
  `discord.js`) while keeping the low layers zero-overhead.
- **Alternatives:** faithful 1:1 port (Arc/locks everywhere); fully idiomatic Rust design.

### D-007 — Cargo workspace mirroring the discord.js monorepo
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Cargo workspace with one crate per concern:

  | Crate            | discord.js counterpart   | Purpose                                  |
  |------------------|--------------------------|------------------------------------------|
  | `djs-rs`         | `discord.js`             | High-level client, events, structures    |
  | `djs-model`      | `discord-api-types`      | All Discord API types                    |
  | `djs-rest`       | `@discordjs/rest`        | REST client + rate limiting              |
  | `djs-gateway`    | `@discordjs/ws`          | Gateway shards, sharding                 |
  | `djs-cache`      | (part of `discord.js`)   | In-memory cache                          |
  | `djs-builders`   | `@discordjs/builders`    | Embed/command/component builders         |
  | `djs-collection` | `@discordjs/collection`  | `Collection` type                        |
  | `djs-formatters` | `@discordjs/formatters`  | Markdown/mention/timestamp formatting    |
  | `djs-util`       | `@discordjs/util`        | Shared utilities                         |
  | `djs-voice`      | `@discordjs/voice`       | Voice (later phase, see D-017)           |
- **Alternatives:** single crate with feature flags.

### D-008 — Own Discord API models, written from scratch
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs-model` is written from scratch (no `twilight-model` dependency), giving full
  control over memory layout, zero-copy deserialization, ID types and documentation.
- **Alternatives:** reuse `twilight-model`.

### D-009 — Documentation language: English
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** All rustdoc, READMEs, guides and repository text in English.
- **Alternatives:** English code docs + Polish guide; bilingual; Polish only.

### D-010 — HTTP client: hyper 1.x directly
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs-rest` uses `hyper` 1.x + `hyper-util` directly (full control over the
  connection pool, HTTP/2, buffers), no `reqwest`.
- **Alternatives:** `reqwest`.

### D-011 — WebSocket: `tokio-websockets`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs-gateway` uses `tokio-websockets` (SIMD masking, RFC-compliant, used by
  twilight-gateway ≥ 0.16).
- **Alternatives:** `fastwebsockets`, `tokio-tungstenite`.

### D-012 — TLS: rustls by default, native-tls optional
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `rustls` is the default TLS backend; a `native-tls` cargo feature offers the
  system TLS library instead.
- **Alternatives:** rustls only; native-tls only.

### D-013 — JSON: serde_json by default, simd-json optional
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `serde_json` by default; a `simd-json` cargo feature switches hot-path parsing to
  `simd-json`.
- **Outcome:** measured in Phase 4 and not added — no clear gain (D-150).
- **Alternatives:** simd-json by default; serde_json only.

### D-014 — Async runtime: tokio only
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** The library targets tokio exclusively (required by hyper and tokio-websockets).
- **Alternatives:** runtime-agnostic abstraction.

### D-015 — Gateway transport compression: zstd-stream default, zlib-stream optional
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `zstd-stream` transport compression is enabled by default; `zlib-stream` is
  available as a cargo feature; both can be disabled (no compression).
- **Alternatives:** zlib-stream default; no compression by default.

### D-016 — Cache: configurable, pluggable, DashMap by default
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Cache behaviour is configurable like discord.js `makeCache` / `sweepers`
  (enable/disable per resource, size limits, periodic sweeping). Storage is abstracted behind a
  trait so users can plug their own backend; the default backend uses `DashMap`.
- **Alternatives:** `papaya` (lock-free), `scc::HashMap`.

### D-017 — Voice: later phase
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Voice support (`djs-voice`, ≈ `@discordjs/voice`, incl. DAVE E2EE, Opus, RTP) is
  planned as a separate crate **after** the core is complete.
- **Alternatives:** out of scope; in parallel with the core.

### D-018 — Sharding: in-process first, multi-process later
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** First implement in-process sharding (each shard is a tokio task). A
  multi-process `ShardingManager` with IPC (like discord.js) follows in a later phase.
- **Alternatives:** in-process only; both from the start.

### D-019 — CI: full GitHub Actions pipeline
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** CI runs `rustfmt`, `clippy -D warnings`, tests on Linux/Windows/macOS, rustdoc
  with `-D warnings` (guarantees complete docs), an MSRV (1.85) check and `cargo-deny`
  (licenses, advisories, bans, sources).
- **Alternatives:** basic (fmt/clippy/test on Linux); none.

### D-020 — Build order: vertical slice first
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** First build a minimal end-to-end "ping-pong" bot that exercises every layer
  (core models, REST, gateway, `Client::on`), then widen each layer to the full API surface.
  This validates the architecture before hundreds of types are written.
- **Alternatives:** layer-by-layer (each crate complete before the next); full `djs-model` first.

### D-021 — Copyright holder: "The djs-rs contributors"
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `LICENSE-MIT` and `NOTICE` name "The djs-rs contributors".
- **Alternatives:** "KotoAni Dev Team", "kotoanidev".

### D-022 — crates.io: publish at the first real release
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Crates are not published (and names not reserved) until they contain real
  functionality — crates.io policy forbids name-squatting placeholders. `publish = false`
  stays until then.
- **Alternatives:** publish minimal 0.0.1 placeholders now; never publish (git dependency only).

### D-023 — Snowflake IDs: generic `Id<Marker>` over `NonZeroU64`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** One generic ID type `Id<T>` wrapping `NonZeroU64` with a zero-sized marker
  (`PhantomData`), plus aliases (`UserId = Id<UserMarker>`, `ChannelId = Id<ChannelMarker>`,
  …). Mixing ID kinds is a compile error; `Id<T>` and `Option<Id<T>>` are both 8 bytes.
  Snowflake helpers (`timestamp()`, `get()`, …) live on `Id<T>`.
- **Alternatives:** separate newtypes per entity (serenity style); a single `Snowflake` type.

### D-024 — Model strings: `CompactString`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Model string fields use `compact_str::CompactString` (24 bytes, strings ≤ 24
  bytes stored inline without allocation).
- **Alternatives:** `Box<str>`, `String`.

### D-025 — Errors: `thiserror`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Error types are derived with `thiserror`, kept small (kind + boxed source).
- **Alternatives:** hand-written error types.

### D-026 — Diagnostics: `tracing`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** The library emits diagnostics through `tracing` (spans + structured fields);
  users choose the subscriber. No separate `Debug`/`Warn` client events.
- **Alternatives:** `tracing` + `Events::Debug`/`Events::Warn`; `log`; events only.

### D-027 — rustls crypto provider: `ring` default, `aws-lc-rs` optional
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `ring` is the default rustls crypto provider (builds everywhere without cmake);
  an `aws-lc-rs` cargo feature switches to aws-lc-rs.
- **Alternatives:** aws-lc-rs default + ring feature; aws-lc-rs only.

### D-028 — Handler execution: configurable, spawn-per-event by default
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** By default every dispatched event runs its handlers in a new tokio task (a slow
  handler never blocks others — discord.js-like behaviour). A client option switches to
  sequential, in-order handling per shard.
- **Alternatives:** always spawn; always sequential.

### D-029 — Event APIs: closures + `Stream`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Besides closure registration (`client.on(Events::X, |ctx, ev| …)`, D-006), the
  client exposes an event `Stream` (`client.events()`) for users who want full control.
  No `EventHandler` trait.
- **Alternatives:** closures + `EventHandler` trait; all three; closures only.

### D-030 — Root certificates: `webpki-roots`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** TLS trust anchors come from the bundled Mozilla set (`webpki-roots`); works in
  minimal containers and is reproducible.
- **Alternatives:** platform verifier (system store); both behind a feature.

### D-031 — MSRV raised to 1.89 (amends D-005)
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** MSRV is **1.89** (released August 2025). Needed because maintained versions of
  `tokio-websockets` (≥ 0.13) require 1.89 and `simd-json` (≥ 0.16) requires 1.88. The policy
  from D-005 otherwise stands: fixed MSRV, raised only deliberately with a CHANGELOG note.
- **Alternatives:** stay on 1.85 with pinned old versions (tokio-websockets 0.12.x,
  simd-json 0.15.1); per-crate MSRV.

### D-032 — zstd implementation: `zstd` (C libzstd bindings)
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** zstd-stream decompression uses the `zstd` crate (bundled official libzstd built
  with `cc`; requires a C compiler). Fastest and most mature.
- **Alternatives:** `ruzstd` (pure Rust, slower, MSRV 1.87).

### D-033 — Dependency set for the Phase 1 vertical slice
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Approved dependencies (minimal feature sets), versions checked on crates.io on
  2026-11-10:

  | Crate         | Dependencies |
  |---------------|--------------|
  | `djs-model`   | `serde` (derive), `compact_str` (serde), `bitflags` |
  | `djs-rest`    | `tokio`, `hyper` (client, http1, http2), `hyper-util` (client-legacy, tokio), `http`, `http-body-util`, `bytes`, `hyper-rustls` (ring, webpki-roots), `serde_json`, `thiserror`, `tracing` |
  | `djs-gateway` | `tokio`, `tokio-websockets` (client, ring, rustls-webpki-roots), `zstd`, `futures-util` (Sink/Stream), `fastrand` (heartbeat jitter), `serde_json`, `thiserror`, `tracing` |
  | `djs-rs`      | `tokio`, `futures-core` (Stream), `thiserror`, `tracing` |
  | dev           | `tokio` (test-util, macros) |

  Any dependency beyond this list still requires the owner's approval.

### D-034 — Live testing via a local `.env` token
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** The owner creates a test bot + test guild; the token lives in a local `.env`
  file (git-ignored, never committed). Claude runs examples locally against real Discord. CI
  uses mocks only.
- **Alternatives:** owner runs examples manually; token in GitHub Secrets for live CI.

### D-035 — `rustls` as a direct dependency for explicit crypto provider selection
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs-rest` and `djs-gateway` depend on `rustls` directly (same version that
  `hyper-rustls`/`tokio-websockets` already pull in) and pass
  `rustls::crypto::ring::default_provider()` explicitly. This avoids rustls' process-level
  provider auto-detection, which panics when an application enables both `ring` and
  `aws-lc-rs` (common, e.g. via reqwest).
- **Alternatives:** rely on the process-level default provider.

### D-036 — `tokio-rustls` + `webpki-roots` as direct `djs-gateway` dependencies
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs-gateway` builds its own rustls `ClientConfig` (explicit ring provider,
  Mozilla roots) and passes it to `tokio-websockets` as `Connector::Rustls`. This requires
  `tokio-rustls` and `webpki-roots` directly (same versions already in the tree), because
  `tokio_websockets::Connector::new()` relies on the process-level provider (same panic risk as
  D-035).
- **Alternatives:** install ring as the process-wide default provider when none is set.

### D-037 — `Shard` is poll-driven
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs_gateway::Shard::next_event().await` drives the whole connection
  (heartbeats, reconnects, resumes) while it is being polled. No background tasks or event
  channels inside the shard; commands from other tasks go through a cloneable `MessageSender`.
  The high-level `djs-rs` client wraps shards in tasks for the user.
- **Alternatives:** shard spawns its own task and delivers events over an mpsc channel.

### D-038 — `tracing-subscriber` as a dev-dependency for examples and tests
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Examples (and tests, when useful) install a `tracing-subscriber` `fmt`
  subscriber filtered by `RUST_LOG`, so library diagnostics are visible. It is a
  dev-dependency only; library users never get it.
- **Alternatives:** examples use `println!` only.

### D-039 — Vertical-slice architecture accepted; continue with Phase 2
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** The Phase 1 architecture (model → REST → gateway → client) and the Claude
  defaults C-008 … C-016 are accepted. Next: Phase 2 (full `djs-model` coverage).
- **Alternatives:** sharding (Phase 4) first; architecture changes.

### D-040 — Permissions bitfield: `u64`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `Permissions` is a `bitflags` type over `u64` (all current permissions fit;
  serialized as a decimal string like Discord). Unknown bits are preserved.
- **Alternatives:** `u128` (future-proof, closer to discord.js BigInt).

### D-041 — Optimization and extreme-load tests after every completed stage
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Every completed phase/stage ends with (1) optimization benchmarks of its hot
  paths and (2) extreme-load / stress tests (throughput, concurrency, memory) — results recorded
  in `docs/PERFORMANCE.md` before the stage counts as done. Applies retroactively to Phase 1.
- **Alternatives:** none (owner requirement).

### D-042 — Benchmark & stress-test tooling
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Benchmarks use `divan` (0.1.21, dev-dependency) including its `AllocProfiler`
  (allocations per operation). CI only compiles benchmarks (`cargo bench --no-run`); measurements
  run locally and are recorded in `docs/PERFORMANCE.md`. Stress tests measure peak memory via
  `/proc/self/status` (VmHWM/VmRSS, Linux only) — no extra dependency.
- **Alternatives:** criterion; measuring in CI with regression tracking; `dhat`.

### D-043 — Strict global REST limiter (GCRA) — amended by D-089 (sliding log)
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Replace the fixed one-second window of the global REST limiter with a strict
  GCRA (Generic Cell Rate Algorithm) limiter: never more than the limit in any sliding second,
  constant memory, no new dependency. Implemented as part of Phase 3 (full `djs-rest`); the
  stress test `global_limit_holds` must then assert `≤ 50` per sliding second.
- **Context:** the Phase 1 stress test showed 92–100 requests in a sliding second at window
  boundaries with the fixed-window limiter (same behaviour as discord.js).
- **Alternatives:** keep fixed windows like discord.js; fixed by default with an opt-in strict
  mode in `RestBuilder`.

### D-044 — `djs-model` module layout mirrors the Discord API docs
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** One public module per resource/topic page of docs.discord.com
  (`application`, `audit_log`, `auto_moderation`, `channel`, `emoji`, `entitlement`, `guild`,
  `guild_scheduled_event`, `interaction`, `invite`, `message`, `permissions`, `poll`, `sku`,
  `soundboard`, `stage_instance`, `sticker`, `user`, `voice`, `webhook`, …) plus the existing
  `gateway`, `id`, `rest` and `util`. A type lives in the module of the docs page that defines
  it, so a docs link tells you the import path. No flat re-export at the crate root.
- **Alternatives:** twilight-style nesting (`guild::Permissions`, `channel::message::…`);
  modules plus a flat crate-root re-export.

### D-045 — Permission bitflags are named `Permissions`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs_model::permissions::Permissions` (`u64`, D-040), constants named as in the
  Discord docs (`Permissions::SEND_MESSAGES`), consistent with `Intents`.
- **Alternatives:** `PermissionFlags` (closer to discord.js `PermissionFlagsBits`).

### D-046 — Undocumented values: official docs + discord-api-types
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `djs-model` covers everything in the official Discord docs **plus** items that
  exist in discord-api-types but not in the docs (e.g. `UserFlags::ACTIVE_DEVELOPER`). Such items
  are marked in rustdoc with an "**Undocumented**" note (may change without notice). Unknown
  values are still preserved (C-010).
- **Alternatives:** official docs only.

### D-047 — All bitflags types use `u64`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Every bitflags type (`MessageFlags`, `UserFlags`, `ChannelFlags`, …) is backed by
  `u64`, like `Permissions` and `Intents`: one uniform `bits()` type and room for future high
  bits. Integer-valued flags serialize as JSON integers; only `Permissions` uses strings.
- **Alternatives:** smallest fitting integer per type (smaller cached structs, but future bits
  could be lost and widening would be a breaking change).

### D-048 — Type naming: docs object names without the module prefix
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Public types are named after the Discord docs object, dropping a prefix that the
  module already provides: Guild Member → `guild::Member`, Guild Member Flags →
  `guild::MemberFlags`, Guild Invite Flags → `invite::InviteFlags`. The prefix stays where the
  short name would be misleading. As shown in the approved D-044 preview, `Role` (and
  `RoleFlags`) live in `guild` although the docs define them on the permissions page.
- **Alternatives:** full docs names (`guild::GuildMember`, closer to discord.js `GuildMember`).

### D-049 — Optional and nullable fields in received models
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Absent (`field?`), nullable (`?type`) and both collapse to `Option<T>` in received
  models. Serialization stays faithful to Discord's shape: `field?` fields are skipped when
  `None` (`skip_serializing_if`), `?type`-only fields are written as `null`. Absent booleans
  default to `false` (`#[serde(default)]`). Tri-state request bodies (PATCH: absent vs null) are
  decided separately in Phase 3.
- **Alternatives:** `Option<Option<T>>` wherever a field can be both absent and null.

### D-050 — Arrays are `Vec<T>`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Array fields in model types use `Vec<T>`. Memory-optimized representations, if
  any, belong to the cache layer (Phase 5).
- **Alternatives:** `Box<[T]>` (8 bytes smaller, no spare capacity, less ergonomic).

### D-051 — Partial objects get their own types
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Where the docs define a differently shaped object (member without `user` in
  `MESSAGE_CREATE`, interaction member with `permissions`, partial channels, …) it gets its own
  type (`guild::PartialMember`, `interaction::InteractionMember`, …) with precise, non-optional
  fields, like twilight and the discord-api-types variants.
- **Alternatives:** one type per object with `Option` for everything that is sometimes absent.

### D-052 — `User` and `CurrentUser` are separate types
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `user::User` holds the fields Discord sends for any user (message authors,
  members, …). `user::CurrentUser` holds those plus the fields only sent for the current user
  (`mfa_enabled`, `locale`, `verified`, `email`, `flags`, `premium_type`); it is used for READY
  and `GET /users/@me`. Keeps the hot-path `User` small.
- **Alternatives:** a single `User` with every field optional (like discord-api-types `APIUser`).

### D-053 — Received model structs are `#[non_exhaustive]`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Structs that model data received from Discord (`User`, `Message`, `Ready`, …)
  are `#[non_exhaustive]`, so new Discord fields can be added in minor releases. They are
  built by deserialization (or by constructors where one is useful). Types users build
  themselves (request bodies, outgoing gateway commands) stay constructible; their shape is
  decided with Phase 3 / Phase 6 (builders).
- **Alternatives:** exhaustive public structs (struct literals possible, every new field is a
  breaking change).

### D-054 — Rarely present sub-objects of hot types are boxed
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** In hot types (`User`, `Member`, `Message`, …) optional nested objects of 32 bytes
  or more that are usually absent are stored as `Option<Box<T>>` (e.g. `User::collectibles`,
  `primary_guild`, `avatar_decoration_data`): `User` shrinks from 288 to 144 bytes, and the
  allocation only happens when the field is present. Size guards in unit tests keep hot types
  from growing unnoticed.
- **Alternatives:** keep everything inline (no extra allocations, larger structs).

### D-055 — Exception to D-049: absent "available" means `true`
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** Booleans whose documented meaning defaults to true when omitted (`available` on
  emojis and stickers: Discord omits it for usable items such as standard stickers) default to
  `true` instead of `false`. Every such exception is stated in the field's rustdoc. All other
  absent booleans default to `false` (D-049).
- **Alternatives:** `Option<bool>`; strictly `false`.

### D-056 — Partial emojis are an enum
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** The partial emoji used by reactions, components and polls is
  `emoji::PartialEmoji::{Custom { id, name: Option<_>, animated }, Unicode { name }}`, with a
  hand-written (non-buffering) `Deserialize`. Full guild/application emojis are `emoji::Emoji`.
- **Alternatives:** a struct with optional `id`/`name` (like discord.js `PartialEmoji`).

### D-057 — Channels are an enum of precise variants
- **Date:** 2026-11-10 · **Decided by:** Owner
- **Decision:** `channel::Channel` is an enum with one variant per kind of channel (text,
  announcement, voice, stage, category, forum, media, thread, DM, group DM, directory, unknown),
  each holding a struct with the fields that kind actually has (like discord.js classes and the
  discord-api-types unions). Deserialization stays single-pass and non-buffering: a private flat
  struct with every field is decoded first and converted by `type`. Common fields are exposed
  through methods (`id()`, `kind()`, `guild_id()`, `name()`, …).
- **Alternatives:** one flat struct with every field optional (twilight); flat struct plus typed
  views.

### D-058 — JSON dictionaries are `BTreeMap<K, V>`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Discord's dictionary objects (`{key: value}`) are `std::collections::BTreeMap`:
  application role connection `metadata`, localization dictionaries (`BTreeMap<Locale, _>`),
  interaction `resolved` data (`BTreeMap<Id, _>`), … Serialization is deterministic (sorted by
  key; snowflake keys therefore come out in chronological order). The maps are small (≤ 32
  entries), where a B-tree is a single node and needs no hashing. Like arrays (D-050), absent,
  `null` and empty maps collapse into an empty map.
- **Alternatives:** `HashMap` (random output order, SipHash costs on tiny maps);
  `Vec<(K, V)>` (keeps input order, O(n) lookup, no `.get()`); `IndexMap` (new dependency).

### D-059 — Guild template snapshots use separate types with `u64` placeholder IDs
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `guild_template::SerializedSourceGuild` and its `TemplateRole`,
  `TemplateChannel`, `TemplateOverwrite` (+ `TemplateForumTag`) are separate, fully typed
  structs. Their placeholder IDs (`0` = `@everyone`, channels `1`, `2`, …) are plain `u64`
  (written as integers, also read from numeric strings), because `Id<T>` cannot hold `0` and
  the snapshot has its own field set. The types can be reused for the "create guild" REST body
  in Phase 3.
- **Alternatives:** the same types without the `Template` prefix (`guild_template::Role`,
  `Channel`, clashing with `guild::Role` / `channel::Channel` in imports); an untyped
  `serde_json::Value` snapshot.

### D-060 — Message mentions are `Mention { user, member }`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `Message::mentions` is `Vec<Mention>` with `Mention { user: User, member:
  Option<Box<PartialMember>> }` (the gateway adds a partial member to every mentioned user in
  guilds), and `Message::member: Option<Box<PartialMember>>` holds the author's member.
  `Mention` is decoded in a single pass without buffering (a private flat struct converted into
  `User` + member, like `RawChannel`). `User` keeps only user fields (D-052).
- **Alternatives:** a `member` field on `User` (serenity); dropping mention members.

### D-061 — `referenced_message` is a three-state enum
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** an exception to D-049 for fields whose docs give absent and `null` different
  meanings. `Message::referenced_message` is `ReferencedMessage::{NotFetched, Deleted,
  Message(Box<Message>)}` (absent = Discord did not try to fetch it, `null` = deleted), with
  `as_message()`; it serializes faithfully (absent / `null` / object).
- **Alternatives:** `Option<Box<Message>>` (as discord.js and twilight; loses "deleted").

### D-062 — Messages before components; `components`/`resolved` added later
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Phase 2 order stays: the full message model now, without `components` and
  `resolved`; those fields are added in the components and interactions steps (non-breaking
  thanks to `#[non_exhaustive]`, D-053).
- **Alternatives:** components first, then messages.

### D-063 — Every `type`-discriminated union is an enum of precise variants
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** D-057 becomes the general rule: every documented union keyed by a `type` field
  (message interaction metadata, components, interaction data, command options, audit log
  changes, …) is an enum with one precise struct per variant plus an `Unknown` variant. Decoding
  stays single-pass and non-buffering (a private flat struct converted by `type`); shared
  fields are exposed through methods.
- **Alternatives:** flat structs with every variant's fields optional; deciding case by case.

### D-064 — Floats are `f64`; types containing them derive only `PartialEq`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** JSON floats (`Attachment::duration_secs`, `number` command options, …) are `f64`.
  Types that contain one, directly or through a field (e.g. `Message` via `Attachment`), derive
  `PartialEq` but not `Eq`/`Hash`. All other types keep `Eq` (and `Hash` where they had it).
- **Alternatives:** a public newtype with bitwise `Eq`/`Hash`; other representations (text,
  integer milliseconds).

### D-065 — One `Component` enum for every nesting level
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `component::Component` is a single enum with one precise struct per component
  type (D-063) plus `Unknown`. Children are `Vec<Component>` (action rows, sections,
  containers) or `Box<Component>` (section accessory, label child), so a known type decodes in
  any position, including positions Discord allows only later. Valid nesting when sending is
  checked by the builders (Phase 6).
- **Alternatives:** one union per context (`ActionRowChild`, `ContainerChild`,
  `SectionAccessory`, `LabelChild`, …) like discord-api-types; newly allowed children would end
  up as `Unknown` until a library update.

### D-066 — `Button` is an enum keyed by `style`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** in the spirit of D-063, `component::Button` is
  `Button::{Interactive(InteractiveButton), Link(LinkButton), Premium(PremiumButton),
  Unknown(UnknownButton)}`: styles 1-4 carry a `custom_id` (+ label/emoji), style 5 a `url`
  (+ label/emoji), style 6 only an `sku_id`. Impossible combinations cannot be built; shared
  fields have methods.
- **Alternatives:** one struct with optional `custom_id`/`url`/`sku_id` (discord.js, twilight).

### D-067 — `Interaction` is a struct with an `InteractionData` enum
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `interaction::Interaction` holds the fields every interaction has (id, token,
  guild/channel, member/user, locales, entitlements, permissions, …) directly, and
  `data: InteractionData` is the enum keyed by the interaction `type` (D-063):
  `Ping`, `ApplicationCommand(CommandData)`, `MessageComponent(ComponentData)`,
  `Autocomplete(CommandData)`, `ModalSubmit(ModalSubmitData)`, `Unknown`. Decoding stays
  single-pass.
- **Alternatives:** an enum of whole interactions (`Interaction::Command(CommandInteraction)`,
  …), duplicating the common fields in every variant (closer to discord.js subclasses).

### D-068 — Interaction responses are modelled in Phase 2
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** the interaction response object (callback type + data: messages, deferrals,
  updates, autocomplete results, modals, activity launch) and the callback response objects are
  part of Phase 2 in `interaction`, with the minimal request types they need (`poll::CreatePoll`,
  `message::AttachmentRequest`). All other REST request bodies stay in Phase 3.
- **Alternatives:** model every request body, including interaction responses, in Phase 3.

### D-069 — Command options are an enum keyed by option `type`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** D-063 also covers structures that the docs describe as one table with
  type-dependent fields. `application_command::CommandOption` is an enum by option type
  (subcommand, subcommand group, string, integer, boolean, user, channel, role, mentionable,
  number, attachment, unknown), each variant with exactly its fields; choices are typed
  (`Choice<CompactString>`, `Choice<i64>`, `Choice<f64>`). Received option values in
  interactions are an enum by type as well.
- **Alternatives:** one struct with every type-specific field optional (the docs' table,
  twilight).

### D-070 — Audit log change values are an own JSON value type
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `audit_log::AuditLogChange { key, old_value, new_value }` keeps the "mixed"
  values as an own JSON-like enum `AuditLogValue` (null, bool, number, string, array, object)
  with a conversion to any `Deserialize` type (e.g. `change.new_value_as::<Permissions>()`).
  No new dependency; the approach of discord.js.
- **Alternatives:** an enum of changes keyed by `key` with typed values (twilight; 100+
  variants, the same key has different types on different objects); `serde_json::Value`
  (serde_json would become a public dependency of `djs-model`).

### D-071 — Optional audit entry info is one struct (exception to D-069)
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** the entry's `options` is one struct `AuditLogEntryOptions` with `Option`
  fields (numbers that Discord sends as strings are decoded as numbers, the overwrite `type`
  as `OverwriteType`), because its fields are shared by many event types. This is an explicit
  exception to D-069.
- **Alternatives:** an enum by `action_type` with each event's option fields (≈ 70 variants).

### D-072 — Application identity profiles are modelled; arbitrary JSON is `util::JsonValue`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** the `application_identity_profile` module is part of Phase 2 (discord-api-types
  has it, discord.js v14 does not). The D-070 value type moves to `util` as the general
  `JsonValue` (error: `JsonValueError`) and is used wherever the API has arbitrary JSON: audit
  log change values and the profile's `metadata`.
- **Alternatives:** reuse `audit_log::AuditLogValue` under its name; skip identity profiles for
  now.

### D-073 — Gateway events with extra fields get their own payload types
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** an event whose payload is an object plus extra fields gets its own type that
  holds the object and the extras (e.g. `GuildMemberAdd { guild_id, member }` dereferencing to
  `Member`; `GuildCreate::Available(AvailableGuild { guild, members, channels, … })`), decoded
  in a single pass like `Mention` (D-060), without `serde(flatten)` on the decode path. Base
  types stay lean for the cache; every event has its own payload type, like `MessageCreate`.
- **Alternatives:** add the extras as `Option`/`Vec` fields to the base types (as for
  `Message::member`).

### D-074 — Partial presence users have typed optional fields
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** the `user` of `PRESENCE_UPDATE` is a `PresenceUser { id, …optional typed
  fields }`, decoded in one pass (fast on the most frequent event). A field with an unexpected
  type makes the event fail to decode.
- **Alternatives:** keep only `id`; optional fields decoded leniently (invalid → `None`, needs
  buffering).

### D-075 — Unix-time integers are decoded as `Timestamp`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Unix-time integers in events (`Activity::created_at` and timestamps in ms,
  `TypingStart::timestamp` and `voice_start_time` in s) are `util::Timestamp`s, written back as
  integers in the same unit.
- **Alternatives:** raw `u64`s with the unit in the docs.

### D-076 — Webhook events are modelled in Phase 2, Social SDK events included
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `djs-model` models the [webhook events](https://docs.discord.com/developers/events/webhook-events)
  Discord sends over HTTP to an app's events URL: the outer payload (PING / event), the event
  body and the data of every event type, including the Social SDK ones (`LOBBY_MESSAGE_*`,
  `GAME_DIRECT_MESSAGE_*`), reusing the existing types (entitlements, lobby messages, messages).
- **Alternatives:** only the events in discord-api-types (no Social SDK events); postpone to
  Phase 3/8 together with receiving HTTP events.

### D-077 — RPC is out of scope
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The local client RPC API (IPC/WebSocket to the Discord app, used by game
  clients; `rpc/` in discord-api-types) is not modelled: discord.js v14 has no RPC and this
  project targets bots.
- **Alternatives:** a later phase (e.g. a `djs-rpc` crate); model it in `djs-model` now.

### D-078 — Webhook lobby messages reuse `LobbyMessage`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `LOBBY_MESSAGE_CREATE`/`UPDATE` webhook events carry `lobby::LobbyMessage`,
  extended for them: `application_id` becomes `Option<ApplicationId>` (optional in webhook
  events) and optional `timestamp` / `edited_timestamp` fields are added (sent by updates).
- **Alternatives:** a separate webhook-only lobby message type.

### D-079 — Game direct messages are one lenient type
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `GAME_DIRECT_MESSAGE_*` webhook events carry one `GameDirectMessage` type that
  covers both the "message object" and the "SDK DM message object" of the docs: only `id`,
  `content` and `author` are required, every other field is optional or defaults, so all the
  docs' (inconsistent) examples decode.
- **Alternatives:** an enum of `Message` and an SDK DM type (heuristic distinction, the docs'
  examples would not decode); reuse `Message` with relaxed fields (weakens a hot-path type).

### D-080 — Measure the concurrent-mode regression with mimalloc (dev-dependency only)
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `mimalloc` is added as a **dev-dependency of `djs-rs` only** (stress tests), to
  check whether the Phase 2 drop in `HandlerMode::Concurrent` (per-event allocations freed on
  tokio worker threads) is caused by glibc malloc. If it is, the documentation recommends a
  global allocator; the library API and dependencies do not change.
- **Alternatives:** reduce per-event allocations in the model now (public type changes, maybe a
  new dependency); only record it and revisit the dispatch design in Phase 8.

### D-081 — Decoded model size is addressed in Phase 5
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The decoded model being ≈ 1.2× the JSON (large `Activity`, `Presence` /
  `ClientStatus`, `PresenceUser`; 24-byte string enums) is addressed together with the cache
  in Phase 5, which stores its own compact types, and measured on real cache workloads.
- **Alternatives:** shrink the raw types now (box rare `Activity` parts and `PresenceUser`
  fields, smaller `Unknown` in string enums); only change the string enums now.

### D-082 — REST endpoints are methods on `Rest` taking body structs
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Every REST endpoint is an `async` method on `Rest` taking its path parameters
  and, where it has one, a reference to a plain body struct from `djs_model::rest` (C-012), as
  in Phase 1: `rest.create_message(channel_id, &body).await`. Validating builders come with
  `djs-builders`; `Rest::request` stays as the low-level escape hatch.
- **Alternatives:** per-endpoint request builders with `IntoFuture` (twilight style); only
  generic routes and raw requests (discord.js `REST#post` style).

### D-083 — Tri-state request fields use an own enum
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Request body fields that can be omitted (leave unchanged), `null` (clear) or set
  use an own three-variant enum in `djs_model::util` (working name `Patch<T>` with `Keep`,
  `Clear`, `Set(T)`; the final names are confirmed with the owner when it is implemented),
  skipped when it is `Keep`. Completes D-049 for request bodies.
- **Alternatives:** `Option<Option<T>>`.

### D-084 — Phase 3 order: infrastructure first
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Phase 3 is done in this order: (1) infrastructure — complete rate limiting
  (GCRA global limiter D-043, invalid-request limit, per-route quirks), retries and timeouts,
  the audit-log reason header, multipart uploads, API errors with JSON error codes; (2) every
  endpoint, resource by resource; (3) the CDN URL builder; (4) tests, benchmarks and stress
  tests (D-041).
- **Alternatives:** endpoints first on the current infrastructure, then the rest.

### D-085 — Invalid requests: count, warn, optional fail-fast threshold
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `djs-rest` counts invalid requests (401, 403 and 429 except `shared`-scope
  429s) in a sliding 10-minute window, warns through `tracing` like discord.js, exposes the
  count on `Rest`, and offers an opt-in threshold in `RestBuilder`: once reached, further
  requests fail immediately with an error instead of risking Discord's IP ban (10 000 per
  10 minutes). Off by default.
- **Alternatives:** only count and warn (discord.js); block by default.

### D-086 — Audit-log reasons are an `Option<&str>` parameter
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Every endpoint method that supports `X-Audit-Log-Reason` takes a last
  parameter `reason: Option<&str>` (sent URL-encoded). `Request` gets the same option for the
  low-level API.
- **Alternatives:** a `reason` field in the body structs (plus option structs for endpoints
  without a body); a `rest.reason(..)` wrapper with the same methods.

### D-087 — File uploads are part of the body structs
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Bodies of endpoints that accept files have a `files` field of uploads (name +
  data; not serialized into the JSON part); `Rest` sends `multipart/form-data` automatically
  when files are present, like `{ files }` in discord.js. The upload type lives in
  `djs_model::rest` (C-012).
- **Alternatives:** a separate `files: &[File]` parameter; separate `*_with_files` methods.

### D-088 — Optional `native-roots` feature
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** An optional cargo feature `native-roots` loads the operating system's root
  certificates through the new optional dependency `rustls-native-certs`, keeping rustls as
  the only TLS stack (useful behind corporate proxies with own CAs). `webpki-roots` stays the
  default. Replaces the ROADMAP's "`native-tls` feature".
- **Alternatives:** a `native-tls` feature (`hyper-tls` + `native-tls`, a second TLS stack);
  nothing beyond rustls + webpki-roots.

### D-089 — The strict global limiter is a sliding log (amends D-043)
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The global REST limiter keeps the send times of the last `limit` requests (a
  sliding log, ≈ 8 bytes per allowed request, bounded by the limit) instead of GCRA: never
  more than the limit in any sliding second, while still allowing a burst of `limit` requests
  at once. D-043's goal (strict, bounded memory, no new dependency) is kept.
- **Context:** GCRA is strict only without bursts (requests evenly spaced, 20 ms apart at
  50/s); with a burst of 50 it allows up to ~99 requests in a sliding second.
- **Alternatives:** GCRA without bursts; GCRA with bursts (not strict).

### D-090 — `Route<'a>` borrows tokens
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Routes with tokens (interaction callbacks and follow-ups, webhooks with a
  token) borrow them: `Route<'a>` holds `&'a str`, stays `Copy` and allocates nothing;
  `Request` becomes `Request<'a>`.
- **Alternatives:** owned tokens (`CompactString`) in `Route` (no lifetimes, one allocation
  per request with a token, `Clone` only).

### D-091 — Endpoint methods are named after the Discord docs
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Endpoint methods use the endpoint names of the Discord API docs in snake_case
  (`modify_guild`, `get_channel_messages`, `create_guild_ban`, `edit_message`), so a docs link
  maps 1:1 to a method (like D-044 for modules). Supersedes the `edit_guild` example in D-082.
- **Alternatives:** discord.js-style verbs (`edit_guild`, `fetch_messages`, `ban_member`).

### D-092 — Query parameters: structs written by own code
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Query parameters of an endpoint are a struct in `djs_model::rest` (e.g.
  `GetChannelMessages { limit, before, … }`), written to the query string by a small encoder
  in `djs-rest` (numbers, IDs, booleans, percent-encoded text). No new dependency.
- **Alternatives:** the same structs serialized with `serde_urlencoded`.

### D-093 — File data is `bytes::Bytes`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The data of an upload is `bytes::Bytes`: cheap to clone (retries, the same
  file sent to many channels), no copy from `include_bytes!` (`Bytes::from_static`) or a
  `Vec<u8>`, and a multipart body that does not copy file data. `bytes` becomes a dependency
  of `djs-model` (it was already in the tree through `hyper` and `djs-rest`).
- **Alternatives:** `Vec<u8>`; `Cow<'static, [u8]>`.

### D-094 — The `attachments` array is generated from the files
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Like discord.js, every file of a body gets an entry in the serialized
  `attachments` array (`id` = its `files[n]` placeholder index, plus its file name and
  metadata such as the description), appended to the attachments the caller lists explicitly
  (e.g. existing ones kept when editing).
- **Alternatives:** the caller fills `attachments` with the placeholders by hand.

### D-095 — Content type guessed from the file name extension
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** A file part without an explicit content type gets one from its file name
  extension (own small table of the common image, video, audio and text types), otherwise
  `application/octet-stream`. No dependency.
- **Alternatives:** detection from the file's magic bytes (like discord.js with
  magic-bytes.js); no guessing (explicit type or `application/octet-stream`).

### D-096 — The upload type is named `FileUpload`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `djs_model::rest::FileUpload` (avoids clashing with the model's `Attachment`
  and the future `AttachmentBuilder` of `djs-builders`).
- **Alternatives:** `RawFile` (like `@discordjs/rest`); `UploadFile`.

### D-097 — JSON error codes are an enum
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `djs_model::rest::ErrorCode`, an `int_enum!` of every documented JSON error
  code plus `Unknown(u32)` for new ones; `ApiError::code` uses it.
- **Alternatives:** a `u32` newtype with associated constants; keeping a plain `u32`.

### D-098 — Error code variants use discord-api-types' names
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Variants are named like discord-api-types' `RESTJSONErrorCodes` (e.g.
  `UnknownChannel`, `MissingPermissions`); documented codes missing there get names derived
  from their docs description. The docs only describe codes in sentences.
- **Alternatives:** own names derived from every docs description.

### D-099 — Form field errors are parsed and flattened
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The nested `errors` of an error response (code 50035, `_errors` leaves) are
  parsed into a flat list of field errors (path such as `embeds[0].title`, code, message) on
  `ApiError`, and the error's `Display` lists them like discord.js
  (`embeds[0].title[BASE_TYPE_REQUIRED]: This field is required`). The raw JSON stays
  available.
- **Alternatives:** only the raw JSON value.

### D-100 — `native-roots` covers REST and the gateway, adding to the bundled roots
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Completes D-088. The `native-roots` feature exists in `djs-rest` and
  `djs-gateway` and is forwarded by `djs-rs` (a TLS-intercepting proxy affects both kinds of
  connection). When enabled, the operating system's certificates are trusted *in addition
  to* the bundled `webpki-roots`: building a client never fails because of the system store;
  certificates that cannot be loaded are skipped with a `tracing` warning.
- **Alternatives:** REST only; trusting only the system's certificates (like reqwest's
  `rustls-tls-native-roots`), failing when the store is empty or unreadable.

### D-101 — The tri-state enum is `Patch<T>` with `Keep`, `Clear`, `Set(T)`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Confirms the working names of D-083: `djs_model::util::Patch<T>` with
  `Keep` (the default; the field is left out), `Clear` (sent as `null`) and `Set(T)`, plus
  `From<T>` (→ `Set`) and `From<Option<T>>` (`None` → `Clear`).
- **Alternatives:** `Nullable<T>` with `Absent`/`Null`/`Value`; `Update<T>` with
  `Unchanged`/`Remove`/`Set`.

### D-102 — Endpoint order: by how much bots use them
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Endpoints are implemented resource by resource, one commit per resource, in
  this order: messages and reactions → interactions and application commands → channels and
  threads → guild (guild, members, roles, bans) → users → webhooks → emojis, stickers,
  soundboard → scheduled events, stage instances, invites, templates, audit log, auto
  moderation, polls → monetization (SKUs, entitlements, subscriptions) → application, OAuth2,
  role connections, lobbies, voice.
- **Alternatives:** the order of the docs' resource pages.

### D-103 — Reaction emojis are a typed enum
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Reaction endpoints take `ReactionEmoji<'a>`: `Unicode(&'a str)` or
  `Custom { name: &'a str, id: EmojiId }`, written into the path percent-encoded (`name:id`
  for custom emojis), with conversions from the model's emoji types.
- **Alternatives:** a `&str` in Discord's format (`"👍"`, `"name:id"`), like discord.js'
  emoji identifiers.

### D-104 — A not-yet-indexed message search is an error with `retry_after`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** When Search Guild Messages answers `202` because the guild is not indexed
  yet, the method returns an error of the new kind `ErrorKind::NotYetIndexed { retry_after,
  documents_indexed }`; callers retry after `retry_after` if they want.
- **Alternatives:** retrying automatically inside `Rest`; a result enum with a `NotIndexed`
  case.

### D-105 — Deprecated endpoints with a successor are left out
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Endpoints the docs mark as deprecated and that have a replacement are not
  implemented (e.g. the old `/channels/{id}/pins` routes; the `/channels/{id}/messages/pins`
  ones are). They stay reachable through `Rest::request` only if a `Route` is added later.
- **Alternatives:** adding them with `#[deprecated]` pointing to the successor.

### D-106 — A query flag that changes the response type gives two methods
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** When a query parameter changes what the endpoint returns, there are two
  methods, so the return type always matches: `create_interaction_response(..) -> ()` and
  `create_interaction_response_with_response(..) -> InteractionCallbackResponse` (the
  endpoint name plus the parameter name).
- **Alternatives:** one method with a query struct (D-092) returning an `Option`; one method
  with a `bool` returning an `Option`, like discord.js' `withResponse`.

### D-107 — Follow-up messages have their own body
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Create Followup Message takes `CreateFollowupMessage`, the Execute Webhook
  body without the fields follow-ups do not support (`username`, `avatar_url`,
  `thread_name`, `applied_tags`), like discord-api-types. Editing the original response or a
  follow-up uses the shared `EditWebhookMessage` body (the fields are identical).
- **Alternatives:** one `ExecuteWebhook` body for webhooks and follow-ups.

### D-108 — Bearer tokens get their own client
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `RestBuilder::bearer_token(access_token)` builds a client that authenticates
  with `Bearer …` (OAuth2 access tokens), like `authPrefix: 'Bearer'` in discord.js. Discord
  limits requests per token, so such a client also has its own rate-limit state. Endpoints
  that need a bearer token (e.g. Edit Application Command Permissions) are ordinary methods
  whose docs say so.
- **Alternatives:** a `bearer_token: &str` parameter on those methods, overriding the header
  for one request of the bot's client.

### D-109 — Modify Channel has one body and one method per channel kind
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The three parameter sets of Modify Channel become `ModifyGuildChannel`,
  `ModifyThread` and `ModifyGroupDm`, sent by `modify_channel`, `modify_thread` and
  `modify_group_dm` (all `PATCH /channels/{channel_id}`), so a thread field cannot be sent to
  a guild channel by mistake; like `GuildChannel#edit` / `ThreadChannel#edit` in discord.js.
- **Alternatives:** one `ModifyChannel` with every field (discord-api-types); three bodies
  behind one method taking an enum.

### D-110 — Computed channel permissions are returned as `Permissions`
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Get Channel Member Permissions and Get Channel Role Permissions return
  `Permissions`, unwrapping the `{"permissions": "…"}` response object.
- **Alternatives:** a response struct with a `permissions` field.

### D-111 — Invites support both target user lists
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Create Channel Invite supports `target_user_ids` (JSON) and
  `target_users_file` (a CSV `FileUpload`, sent as `multipart/form-data`); the multipart
  writer learns form fields with a custom name, also for the invite target-user endpoints
  later.
- **Alternatives:** only `target_user_ids` (max 1000 users) for now.

### D-112 — Forum and media thread messages have their own body
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Start Thread in Forum or Media Channel takes a `ForumThreadMessage` with
  exactly the documented fields (content, embeds, allowed mentions, components, sticker IDs,
  attachments, flags) plus files with generated `attachments`, like D-107.
- **Alternatives:** reuse `CreateMessage` (allows fields Discord rejects there).

### D-113 — Image data has its own type with a hand-written base64 encoder
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `djs_model::util::ImageData` holds the image as `bytes::Bytes` plus its MIME
  type (detected from the file signature — PNG, JPEG, GIF, WebP, AVIF — or given) and
  serializes as a `data:<type>;base64,<data>` URI. The base64 encoder is written in the crate
  (no dependency) and streams into the serializer without an intermediate copy. Used for
  every image field of request bodies (group DM icons, avatars, guild/role icons, emojis…).
- **Alternatives:** the same type using the `base64` crate; plain data-URI strings.

### D-114 — Forum tags sent in requests have their own type
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Request bodies take `djs_model::rest::ForumTagData` (`id: Option<TagId>`, so
  new tags have none), like `GuildForumTagData` in discord.js, with `From<ForumTag>` to send
  existing tags back unchanged. The model's `ForumTag` keeps its required `id`.
- **Alternatives:** make `ForumTag::id` optional.

### D-115 — Starting a forum thread returns the thread and its message
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Start Thread in Forum or Media Channel returns
  `djs_model::rest::ForumThread { channel: Channel, message: Option<Message> }`, keeping the
  nested starter message of the response.
- **Alternatives:** return only the `Channel`, like discord.js.

### D-116 — The guild widget image is a URL, not a request
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Get Guild Widget Image (a public PNG) gets no `Rest` method; the CDN/URL
  builder step adds a function returning its URL with `?style=` (`WidgetStyle`), like
  `Guild#widgetImageURL` in discord.js.
- **Alternatives:** a method returning the PNG bytes; both.

### D-117 — Invite metadata is optional
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** The metadata fields move into `djs_model::invite::InviteMetadata`, and
  `InviteWithMetadata { invite, metadata: Option<InviteMetadata> }` (both flattened) serves
  every invite list, so Get Guild Invites without `MANAGE_GUILD` (no metadata) still
  decodes. `Deref<Target = Invite>` stays.
- **Alternatives:** a separate `GuildInvite` type for Get Guild Invites; `Vec<Invite>`.

### D-118 — Onboarding requests have their own types and generated prompt IDs
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Modify Guild Onboarding takes `djs_model::rest::OnboardingPromptData` and
  `PromptOptionData` (emoji as `emoji_id`/`emoji_name`/`emoji_animated`, as Discord requires
  there), with `id: Option`. A prompt without an ID gets one generated from the current time
  when serialized (Discord requires an ID even for new prompts; discord.js does the same).
  `From<OnboardingPrompt>`/`From<PromptOption>` send existing ones back.
- **Alternatives:** the same types with required IDs.

### D-119 — Webhook query parameters live in the body structs
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `ExecuteWebhook` and `EditWebhookMessage` carry `thread_id` and
  `with_components` as fields that are sent in the query string, not in the JSON (like the
  `threadId`/`withComponents` options of discord.js). Get and Delete Webhook Message take
  `thread_id: Option<ChannelId>` as an argument. Execute Webhook's `wait` follows D-106:
  `execute_webhook` (no response) and `execute_webhook_and_wait` (returns the `Message`).
- **Alternatives:** separate query structs as an extra argument; plain method arguments.

### D-120 — Slack- and GitHub-compatible webhooks are not modelled
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Execute Slack-Compatible Webhook and Execute GitHub-Compatible Webhook are
  left out: they take third-party payloads meant to be sent by those services, and
  discord.js does not support them either.
- **Alternatives:** methods taking the raw JSON payload (and the `X-GitHub-Event` header).

### D-121 — Token-authenticated routes are sent without the bot token
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** Routes authenticated by a webhook or interaction token in their path
  (`/webhooks/{id}/{token}…`, `/interactions/{id}/{token}/callback`) are sent without the
  `Authorization` header, like `auth: false` in discord.js.
- **Alternatives:** always send the client's token.

### D-122 — Sounds have their own data-URI type
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `djs_model::util::SoundData` holds a sound as `Bytes` with its media type
  (detected from the MP3 or OGG signature, or given) and serializes as a data URI with the
  same streaming base64 encoder as `ImageData` (D-113), so an image cannot be sent as a
  sound by mistake. Used by Create Guild Soundboard Sound.
- **Alternatives:** `ImageData::with_content_type`; renaming `ImageData` to a general
  `DataUri`.

### D-123 — A reproducible showcase against discord.js on Node.js
- **Date:** 2026-11-11 · **Decided by:** Owner
- **Decision:** `docs/VS_NODE.md` (linked from the main README) compares djs-rs with
  discord.js on the same machine and workload, as tables with multipliers plus SVG bar charts
  generated by a script from the measured results. The harness lives in `benches/vs-node/`:
  a `publish = false` workspace crate with the Rust runners and a mock Discord server (run as
  its own process, so it never shares a runtime with the client under test), and Node scripts
  whose `package.json` pins discord.js 14.x with @discordjs/ws and @discordjs/rest at the
  versions discord.js itself resolves; `package-lock.json` is committed, `node_modules` is
  git-ignored and `npm ci` runs before measuring. Four scenarios: event decoding
  (`MESSAGE_CREATE`, a 1000-member `GUILD_CREATE`), an end-to-end event flood through a mock
  gateway (events/s, CPU, peak RSS), the memory of cached guilds, and REST throughput against
  a mock HTTP API (req/s, p50/p99). Every number is measured, never estimated, and caveats
  (for example that djs-rs has no cache layer yet) are written next to the numbers.
- **Alternatives:** a section of `docs/PERFORMANCE.md`; fewer scenarios; tables only;
  unpinned npm dependencies.

### D-124 — Recurrence rules are sent as `RecurrenceRuleData`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Scheduled event bodies take `djs_model::rest::RecurrenceRuleData`, which has
  only the fields a bot may set (`start`, `frequency`, `interval`, `by_weekday`,
  `by_n_weekday`, `by_month`, `by_month_day`; not Discord's `end`, `count`, `by_year_day`),
  constructors for the shapes Discord accepts (daily, weekly, every other week, monthly on
  the n-th weekday, yearly on a date) and `From<RecurrenceRule>`. Same pattern as
  `ForumTagData` (D-114) and `OnboardingPromptData` (D-118).
- **Alternatives:** a constructor on `RecurrenceRule` with empty fields skipped (Discord
  rejects the system fields if set); `RecurrenceRuleData` without shape constructors.

### D-125 — Create Guild Scheduled Event has one constructor per entity type
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `CreateGuildScheduledEvent` is one struct with public fields and the
  constructors `stage(channel_id, name, start)`, `voice(channel_id, name, start)` and
  `external(location, name, start, end)`, which set `entity_type`, the fields that type
  requires and `privacy_level` `GUILD_ONLY` (the only accepted value).
- **Alternatives:** an enum of entity kinds with custom serialization; one generic
  `new(name, entity_type, start)`.

### D-126 — Get Target Users returns parsed user IDs
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Rest::get_target_users` parses the CSV Discord returns (a `user_id` header,
  then one ID per line) into `Vec<UserId>`; a line that is not an ID is a decode error.
- **Alternatives:** the raw CSV bytes, like discord.js' `fetchTargetUsers`; both methods.

### D-127 — Target user files have a helper constructor
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Rest::update_target_users` takes a `&FileUpload` (consistent with D-111),
  and `FileUpload::target_users(ids)` builds `users.csv` (one ID per line, `text/csv`) from
  user IDs, for Update Target Users and Create Channel Invite alike.
- **Alternatives:** two methods (IDs and file); only `&FileUpload`.

### D-128 — Create Auto Moderation Rule has one constructor per trigger type
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `CreateAutoModerationRule` is one struct with public fields and the
  constructors `keyword(name, keywords, actions)`, `spam(name, actions)`,
  `keyword_preset(name, presets, actions)`, `mention_spam(name, limit, actions)` and
  `member_profile(name, keywords, actions)`, which set `trigger_type`, the matching
  `event_type` (`MESSAGE_SEND`, or `MEMBER_UPDATE` for member profiles) and the trigger
  metadata that type requires. The rest (allow list, regexes, `enabled`, exemptions) is set
  through the public fields. Same pattern as D-125.
- **Alternatives:** one generic `new(name, event_type, trigger_type, actions)`.

### D-129 — Auto Moderation actions have one constructor per action type
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `auto_moderation::Action` gets `block_message()`,
  `block_message_with(custom_message)`, `send_alert_message(channel_id)`,
  `timeout(duration_seconds: u32)` and `block_member_interaction()`, each setting the kind
  and the metadata it requires.
- **Alternatives:** a generic `Action::new(kind, metadata)` with `ActionMetadata`
  constructors; `timeout` taking a `core::time::Duration`.

### D-130 — Create Test Entitlement has guild and user constructors
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `djs_model::rest::CreateTestEntitlement` has public fields and the
  constructors `guild(sku_id, guild_id)` and `user(sku_id, user_id)`, which set `owner_id`
  and `owner_type`. `owner_type` is a new `int_enum!`
  `entitlement::EntitlementOwnerType { Guild = 1, User = 2 }` (discord.js has the same enum).
- **Alternatives:** an `EntitlementOwner::{Guild, User}` enum with custom serialization; a
  generic `new(sku_id, owner_id, owner_type)`.

### D-131 — OAuth2: full scope in djs-rest
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** djs-rest covers all of `topics/oauth2.mdx`: Get Current Bot Application
  Information, Get Current Authorization Information, the token endpoint (authorization code
  exchange, refresh, client credentials) and token revocation, plus an authorization URL
  builder. discord.js itself only generates invite URLs.
- **Alternatives:** like discord.js (the two GET endpoints and a URL builder, tokens left to
  other crates); only the GET endpoints now.

### D-132 — OAuth2 token endpoints are `Rest` methods taking credentials
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Rest::exchange_code(&credentials, code, redirect_uri)`,
  `refresh_access_token(&credentials, refresh_token)`,
  `client_credentials_grant(&credentials, scopes)` and
  `revoke_token(&credentials, token, hint)`. `djs_rest::oauth2::OAuth2Credentials
  { client_id, client_secret }` redacts the secret in `Debug`. Requests are
  `application/x-www-form-urlencoded` with `client_id` and `client_secret` in the form (which
  Discord accepts instead of HTTP Basic, so no base64 code or dependency), and never carry the
  client's own token. The response, `djs_model::oauth2::AccessTokenResponse`, redacts its
  tokens in `Debug`.
- **Alternatives:** credentials stored in `RestBuilder`; a separate `OAuth2Client` type.

### D-133 — Authorization URLs are built with `djs_rest::oauth2::AuthorizeUrl`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** A builder in djs-rest: `AuthorizeUrl::bot(client_id)` (bot invite with
  permissions, guild, guild select lock) and `AuthorizeUrl::code(client_id, redirect_uri)`
  (authorization code grant with scopes, state, prompt), producing the
  `https://discord.com/oauth2/authorize?…` URL. djs-rs' future `Client::generate_invite`
  builds on it.
- **Alternatives:** a builder in djs-util; only `Client::generate_invite` in djs-rs.

### D-134 — Lobby member bodies: `LobbyMemberData`, `AddLobbyMember`, `BulkLobbyMember`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Create and Modify Lobby take `LobbyMemberData::new(user_id)` (`metadata`,
  `flags`, `additional_name: Patch` — `Clear` removes it); Add a Member to a Lobby takes
  `AddLobbyMember` (`Default`, no ID, the user is in the path); Bulk Update Lobby Members takes
  `&[BulkLobbyMember]`, built with `BulkLobbyMember::upsert(LobbyMemberData)` or
  `BulkLobbyMember::remove(user_id)`.
- **Alternatives:** bulk as two lists (`upsert: &[LobbyMemberData]`, `remove: &[UserId]`);
  one `LobbyMemberData` with a `remove_member` flag for every endpoint.

### D-135 — Update Application Identity Profile returns `Option<ApplicationIdentityProfile>`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Rest::update_application_identity_profile` returns
  `Some(profile)` when Discord answers `201 Created` with the profile (the first write, which
  creates the identity) and `None` for `204 No Content` (later updates).
- **Alternatives:** always `()`, ignoring the body.

### D-136 — Get Gateway returns `djs_model::rest::Gateway`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Rest::get_gateway()` returns a new `#[non_exhaustive]`
  `djs_model::rest::Gateway { url }`, next to `GatewayBot`.
- **Alternatives:** returning the URL alone, unwrapped from `{"url": …}`.

### D-137 — CDN URLs are built by `djs_rest::cdn::Cdn`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Like `REST#cdn` in @discordjs/rest: a `Cdn` struct holding the CDN and media
  proxy base URLs (defaults `https://cdn.discordapp.com` and `https://media.discordapp.net`,
  configurable on `RestBuilder`), with one method per CDN endpoint returning a `String`.
  `Rest::cdn()` returns the client's `&Cdn`; `Cdn::default()` works without a client. The
  guild widget image URL (D-116) is `Rest::guild_widget_image_url`, as it uses the API base.
- **Alternatives:** free functions with a fixed base URL; URL methods on the model types
  (those can still come later in djs-rs on top of `Cdn`).

### D-138 — Animated images default to animated WebP
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Without an explicit format, an animated hash (`a_…`) gives
  `.webp?animated=true` (what Discord's docs recommend; WebP and AVIF uploads convert badly to
  GIF), and other images `.webp`. `force_static` gives a static `.webp`.
- **Alternatives:** `.gif` for animated hashes, as discord.js v14 does.

### D-139 — Image formats and sizes are enums
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `cdn::ImageFormat { Webp, Png, Jpeg, Gif }` and
  `cdn::ImageSize { S16, S32, S64, S128, S256, S512, S1024, S2048, S4096 }`, so an invalid size
  cannot be written and URL building never fails.
- **Alternatives:** `size: u16` validated at runtime (`Result` or panic), like discord.js'
  `RangeError`.

### D-140 — zlib-stream uses `flate2` with the `zlib-rs` backend
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** The optional `zlib-stream` feature (D-015) adds `flate2` with the `zlib-rs`
  backend (pure Rust, about as fast as zlib-ng, no C compiler). zstd stays the default.
- **Alternatives:** `flate2` with `miniz_oxide` (pure Rust, slower); `flate2` with `zlib-ng`
  (C, needs cmake).

### D-141 — In-process sharding: a shard factory and a shared IDENTIFY queue
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** djs-gateway creates the shards of a range or a total count and gives them a
  shared IDENTIFY queue that honours `max_concurrency` (bucket `shard_id % max_concurrency`,
  one IDENTIFY per bucket every 5 s). Each shard stays poll-driven (D-037) and runs in its own
  task (djs-rs does that). The recommended shard count comes from `get_gateway_bot` in djs-rs,
  so djs-gateway does not depend on djs-rest.
- **Alternatives:** a `ShardManager` owning every shard behind one merged event stream; both.

### D-142 — Gateway commands over the send limit wait in a queue
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Commands beyond Discord's 120 per 60 s per connection wait in the shard and
  are sent when the window allows; heartbeats have reserved room and always go out (like
  @discordjs/ws). `MessageSender::command` keeps not blocking.
- **Alternatives:** returning `SendError::RateLimited` when the window is full.

### D-143 — Phase 4 order
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** 1) the command send limit (D-142), 2) the IDENTIFY queue and sharding
  (D-141), 3) the gateway commands (presence update, request guild members, voice state
  update, request soundboard sounds), 4) zlib-stream (D-140), 5) simd-json (D-013), kept only
  if benchmarks show a gain, 6) mock-gateway tests, benchmarks and extreme-load tests (D-041).
- **Alternatives:** compression and simd-json first.

### D-144 — The IDENTIFY queue is a trait with a local default
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `djs_gateway::IdentifyQueue` is a trait (`wait_turn(&self, ShardId)` returning
  a boxed `Send` future that completes when the shard may identify); `LocalIdentifyQueue::
  new(max_concurrency)` is the in-process implementation (one IDENTIFY per bucket
  `shard % max_concurrency` every 5 s). `Config` holds it as `Arc<dyn IdentifyQueue>`
  (`ConfigBuilder::identify_queue`); the default is a local queue with `max_concurrency` 1. A
  queue shared by several processes (e.g. Redis) can be plugged in later without breaking the
  API (like discord.js `IIdentifyThrottler`, twilight `Queue`).
- **Alternatives:** only a concrete queue type.

### D-145 — Shards are created with `djs_gateway::create_shards`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `create_shards(range, total, config)` returns an iterator over the shards of
  `range` out of `total`; they share the config's IDENTIFY queue (a cloned `Config` shares
  it). `Shard::with_config` keeps working for manual setups.
- **Alternatives:** a `ShardsBuilder` with `total`, `range` and `build() -> Vec<Shard>`.

### D-146 — `Client` shards automatically by default
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** In Phase 4 the djs-rs `Client` runs one tokio task per shard, with the shard
  count Discord recommends and `max_concurrency` from Get Gateway Bot by default.
  `ClientBuilder::shards(Shards::Auto | Shards::Count(n) | Shards::Range { range, total })`
  overrides it. (discord.js defaults to one shard; `'auto'` is opt-in.)
- **Alternatives:** one shard by default with auto opt-in; leaving `Client` single-sharded
  until the djs-rs phase.

### D-147 — Not enough session starts is an error
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** If `session_start_limit.remaining` is lower than the number of shards to
  start, `Client::login` returns an error with the remaining count and the time until the
  reset, without connecting (like discord.js).
- **Alternatives:** waiting `reset_after` (up to 24 h) and connecting then.

### D-148 — Presence: an initial one plus the last one set
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ConfigBuilder::presence(UpdatePresence)` is sent in every IDENTIFY.
  `MessageSender::update_presence(&UpdatePresence)` sends a presence update and also makes
  it the presence of later IDENTIFYs (a new session keeps the last presence, unlike
  discord.js, which falls back to the initial one). A plain `command(&UpdatePresence)` only
  sends it (documented).
- **Alternatives:** only the initial presence (like @discordjs/ws).

### D-149 — djs-rs gets presence helpers now, collectors in Phase 6
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ClientBuilder::presence(UpdatePresence)` (initial presence of every shard) and
  `Client::set_presence(&UpdatePresence)` (every running shard) arrive in Phase 4. Requesting
  guild members and soundboard sounds (collecting the response chunks) and updating the voice
  state come in Phase 6 with discord.js-like structures (`guild.members.fetch()`); until then
  any command can be sent with `ctx.shard().command(…)`.
- **Alternatives:** also `request_guild_members`, `request_soundboard_sounds` and
  `update_voice_state` on `Client` now (like @discordjs/core); nothing in djs-rs until Phase 6.

### D-150 — simd-json: investigate further, then try `sonic-rs` in the benchmark
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** After the first simd-json measurements the owner chose to keep investigating;
  after the careful re-measurement (≤ 15 % gain on message events only, losses elsewhere,
  `docs/PERFORMANCE.md`) the owner chose to try `sonic-rs` 0.5.10 the same way, as a
  bench-only dev-dependency. If it clearly wins, adding a feature is a separate decision; if
  not, both experiments (benches and dev-dependencies) are removed and the D-013 feature is
  closed as measured, not added.
- **Alternatives:** dropping simd-json without further tries; adding simd-json as an opt-in
  feature anyway.
- **Outcome (2026-11-12):** sonic-rs was slower than serde_json on almost every payload, also
  with `-C target-cpu=native`; simd-json won only ~10 % on message events. Neither clearly
  won, so both experiments were removed (the code is in commit `af58a18`), no feature was
  added and serde_json stays the only parser (`docs/PERFORMANCE.md`, Phase 4).

### D-151 — Cache reads return `Arc<T>`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** reading from `djs-cache` (e.g. a guild by ID) returns an `Arc` of the cached
  value: a cheap clone that holds no lock, so it can be kept across `.await`. An update
  replaces the `Arc` (one allocation per changed entity). The backend trait (D-016) works on
  `Arc` values and needs no lifetimes or GATs.
- **Alternatives:** DashMap guards as in twilight/serenity (no copy, in-place updates, but
  deadlocks when held across `.await` or while writing the same map shard); owned clones.

### D-152 — Minimal cache by default
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** by default the cache keeps only guilds, channels, roles and the current user.
  Every other resource (threads, emojis, stickers, members, users, voice states, presences,
  messages, scheduled events, stage instances, soundboard sounds, …) is opt-in. This differs
  from discord.js, which caches everything its intents deliver (messages 200 per channel);
  the documentation says so and shows how to enable the discord.js set.
- **Alternatives:** discord.js defaults (everything, messages limited to 200 per channel);
  everything without limits.

### D-153 — Cache updates return the previous state
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** applying an event to the cache always returns the previous version of what it
  changed or removed (an `Arc`, cheap with D-151), so the djs-rs `Client` can give handlers
  the old and the new state like discord.js (`messageUpdate(oldMessage, newMessage)`,
  Phase 8). A resource that is not cached has no previous state.
- **Alternatives:** only when enabled in the configuration; never.

### D-154 — Phase 5 order
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** one commit per step: (1) backend trait + DashMap backend + configuration
  (resources on/off, size limits); (2) guilds, channels, roles, emojis, stickers, members,
  users; (3) messages, presences, voice states, threads, scheduled events, stage instances,
  soundboard; (4) sweepers; (5) the djs-rs `Client` integration; (6) benchmarks,
  extreme-load tests and a new `docs/VS_NODE.md` run.
- **Alternatives:** a minimal guild cache wired into `Client` first, then the rest.

### D-155 — Cache backend: a `Store` per resource, created by a `Backend`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `djs_cache::Store<K, V>` is a small trait for one resource's map (`get`,
  `insert`, `remove`, `len`, `retain`, `values`, all on `Arc<V>`, D-151);
  `djs_cache::Backend` creates one store per resource (`type Store<K, V>`, `fn store(resource,
  limit)`). `Cache<B: Backend = DashMapBackend>` is generic over the backend, so calls are
  static. The default `DashMapBackend` creates `DashMapStore`s. This mirrors discord.js
  `makeCache`, which creates a collection per manager.
- **Alternatives:** one large cache trait with a method per resource and operation;
  `Box<dyn Store>` per resource from a factory (a backend per resource at run time, a vtable
  call per access).

### D-156 — Cache configuration: one `Limit` per resource
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `CacheConfig::builder()` takes a `Limit` per resource (`Limit::Disabled`,
  `Limit::Max(n)`, `Limit::Unlimited`), which both turns the resource on or off and bounds its
  size, like discord.js `Options.cacheWithLimits`. `CacheConfig::discord_js()` is a preset with
  discord.js' defaults (everything, messages `Max(200)` per channel); `CacheConfig::default()`
  is the minimal set of D-152.
- **Alternatives:** bitflags to enable resources plus separate size-limit methods (twilight).

### D-157 — A full resource evicts its oldest inserted entry
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** when a resource limited to `Max(n)` is full, inserting a new entry evicts the
  entry inserted first (FIFO), like discord.js `LimitedCollection`. Reads stay plain reads; the
  insertion order costs one key per entry, only in limited resources. Updating an existing
  entry does not move it.
- **Alternatives:** least recently used (every read becomes a write); smallest snowflake (needs
  an ordered map).

### D-158 — Cache keys are hashed with `foldhash`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** new dependency `foldhash` 0.2 for the `DashMapStore` hasher (with `dashmap` 6.2, D-016)
  (`foldhash::fast::RandomState`): about 1–2 ns per snowflake key instead of 15–20 ns for
  std's SipHash, with good mixing of the high bits DashMap uses to pick a shard and minimal
  DoS resistance. To be confirmed by the Phase 5 benchmarks.
- **Alternatives:** `rustc-hash` (FxHash; no DoS resistance, weaker mixing); std SipHash (no
  new dependency, slower).

### D-159 — Limits of guild resources count per guild
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Limit::Max(n)` counts per guild for resources that belong to a guild
  (members, presences, voice states, emojis, stickers, roles, threads, scheduled events, stage
  instances, soundboard sounds), per channel for messages, and over the whole cache for guilds,
  channels and users — the scopes of discord.js managers (`GuildMemberManager` per guild,
  `MessageManager` per channel, `UserManager` per client). The cache enforces the limits
  through its guild → IDs indexes, which it keeps anyway (e.g. for `GUILD_DELETE`).
- **Alternatives:** every limit over the whole cache (simpler, but large guilds push out
  small ones, unlike discord.js).

### D-160 — Cached types: djs-model types plus `CachedGuild` and `CachedMember`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** roles, emojis, stickers, users and channels are cached as their djs-model
  types (already compact). Guilds and members get their own types: `CachedGuild` is a guild
  without the role, emoji and sticker lists (those live in their own stores), plus the
  `GUILD_CREATE` extras (`joined_at`, `large`, `member_count`); `CachedMember` holds its user
  as an `Arc<User>`, shared with the user store when users are cached.
- **Alternatives:** an own type for every resource (smaller, but seven parallel types to
  maintain); only model types, guilds with their lists (duplicated data).

### D-161 — The cache reads events by reference
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `cache.update(&event)` borrows the event and copies what it keeps, so the
  cache does not depend on how the djs-rs `Client` hands payloads to handlers. The cost of the
  copies is measured in step 6.
- **Alternatives:** take events by value and move their data into `Arc`s (no copies, but the
  handler payloads of Phase 8 would follow the cached types).

### D-162 — Reads go through per-resource views, like discord.js managers
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** each resource is read through a borrowed view with a uniform API, like a
  discord.js manager's `cache`: `cache.guilds().get(id)`, `cache.guilds().len()`,
  `cache.channels().get(id)`, `cache.roles(guild_id).get(id)` / `.values()`,
  `cache.members(guild_id).get(user_id)` / `.keys()`, plus `cache.current_user()`.
- **Alternatives:** a method on `Cache` per operation, like twilight's `InMemoryCache`.

### D-163 — The previous state is typed per event
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** an `UpdateCache` trait, implemented by every event payload the cache
  handles, has an associated `Previous` type (e.g. `Option<Arc<CachedMember>>` for a member
  update, `Option<Arc<CachedGuild>>` for a guild delete), so `cache.update(&payload)` returns
  a statically typed previous state. `cache.update_dispatch(&DispatchEvent)` handles the
  whole dispatch enum and returns an enum.
- **Alternatives:** only `cache.update(&DispatchEvent)` returning one enum to match on.

### D-164 — Presences are cached as `CachedPresence`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a cached presence holds its user's ID instead of the `PresenceUser` (user data
  goes to the user resource), the model's `Status` and `ClientStatus`, and the activities as a
  `Box<[Activity]>` of the model type, complete. About half the size of `Presence` (296 B),
  without duplicating the user. This is the compact presence type D-081 announced.
- **Alternatives:** also an own `CachedActivity` with the common fields and one-byte statuses
  (smallest, loses rare activity fields); the model `Presence` unchanged.

### D-165 — Messages are cached as `CachedMessage`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a cached message has the fields of `Message`, with the author as an
  `Arc<User>` shared with the user resource (like `CachedMember`, D-160) and without the
  `member` field (members have their own resource), so an author of many messages is stored
  once.
- **Alternatives:** the model `Message` unchanged (author and member copied into every
  message).

### D-166 — Reaction events update cached messages
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `MESSAGE_REACTION_ADD`, `MESSAGE_REACTION_REMOVE`,
  `MESSAGE_REACTION_REMOVE_ALL` and `MESSAGE_REACTION_REMOVE_EMOJI` update the reactions of a
  cached message (copy-on-write through its `Arc`, only when the message is cached) and return
  the message before the change, like discord.js.
- **Alternatives:** leave a cached message's reactions as `MESSAGE_CREATE` / `MESSAGE_UPDATE`
  sent them.

### D-167 — Sweepers: cache methods, scheduled by the djs-rs `Client`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `djs-cache` offers synchronous sweep methods that return how many entries they
  removed, and a sweep configuration (interval and lifetime) inside `CacheConfig`, but runs
  nothing by itself, so it stays free of tokio. The djs-rs `Client` runs the configured sweeps
  on their intervals (Phase 5, step 5).
- **Alternatives:** an own tokio task in `djs-cache` behind a feature
  (`Cache::spawn_sweepers`); manual sweep methods only, without a configuration.

### D-168 — A sweep with a filter for every resource, plus lifetime presets
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** every resource can be swept with a filter (`cache.sweep_users(|user| …)`),
  like discord.js `client.sweepers.sweepUsers(filter)`; two presets cover discord.js'
  lifetimes: messages older than a lifetime, and threads archived for longer than one.
- **Alternatives:** only the two presets.

### D-169 — Default sweeps follow discord.js
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** by default, every hour, threads archived more than 4 hours ago are swept
  (discord.js `Options.DefaultSweeperSettings`); this only matters when threads are cached,
  which is off by default. Messages are not swept by default: their per-channel limit bounds
  them.
- **Alternatives:** nothing swept by default.

### D-170 — The `Client` caches only on request
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a djs-rs `Client` has no cache unless one is configured
  (`ClientBuilder::cache(CacheConfig)`); `client.cache()` and `ctx.cache()` return
  `Option<&Cache>`. Bots without a cache pay nothing for it. This differs from discord.js,
  where the cache always exists; the documentation says so.
- **Alternatives:** always a cache, the minimal `CacheConfig::default()` unless configured.

### D-171 — Handlers get the previous state in Phase 8
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the `Client` applies each event to the cache before its handlers run (the
  discord.js order), so handlers read the new state through `ctx.cache()`. Old/new pairs for
  handlers (`messageUpdate(old, new)`, …) come in Phase 8 with the djs-rs structures and
  events, so the event API is designed once.
- **Alternatives:** hand handlers the previous state now (e.g. `ctx.previous()`), likely to
  change in Phase 8.

### D-172 — The `Client` is generic over the cache backend
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Client<B: Backend = DashMapBackend>` (and its builder and `Context`) take the
  cache backend as a type parameter with the default backend as default, and
  `ClientBuilder::cache_backend(backend)` switches it, so custom backends (D-016) work through
  the `Client` too. Code naming `Client` / `Context` without a parameter keeps working.
- **Alternatives:** the `Client` always uses `Cache<DashMapBackend>` (custom backends only with
  `djs-cache` on its own; a type parameter could come in Phase 8).

### D-173 — Typing, interactions and thread members update the cache like discord.js
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the cache applies `TYPING_START` (its member), `INTERACTION_CREATE` (its
  user, its member, and the resolved users and members) and `THREAD_MEMBERS_UPDATE` (the
  thread's `member_count` and the added members as guild members), like discord.js. Partial
  channels resolved in interactions are not cached, so they never replace full ones. Each
  returns its previous state like the other member events. Found by the Phase 5 benchmarks
  (`docs/PERFORMANCE.md`).
- **Alternatives:** typing and interactions only; leave them out as a documented difference.

### D-174 — `VS_NODE.md` compares with equal caches
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the summary of `docs/VS_NODE.md` compares djs-rs with
  `CacheConfig::discord_js()` (what discord.js caches by default) against discord.js'
  defaults; djs-rs without a cache (its default, D-170) is a separate row. The memory section
  uses the real cache instead of the `HashMap` stand-in.
- **Alternatives:** keep "default configurations" in the summary (djs-rs without a cache);
  both rows in the summary.

### D-175 — Cache memory is optimized now, internally
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** before Phase 5 closes, the cache's memory per entry (424 B per member with
  discord.js' defaults — corrected to 546 B when measured in a fresh process, see D-177) is
  reduced with measured internal changes only. A change that needs a new public type (e.g. a
  compact cached user) is asked separately.
- **Alternatives:** also allow new public types now; leave it for later.

### D-177 — Compact cached user and member types
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** measured in a fresh process, the cache needs 546 B per member (discord.js'
  defaults; the earlier 424 B reused memory freed by earlier tests). Internal changes cannot
  reduce it much: ~416 B are the `Arc<CachedMember>` (208 B), `Arc<User>` (176 B) and role
  list (32 B) blocks. So the cache gets a compact public `CachedUser` (held by
  `cache.users()`, `CachedMember::user` and, to share it, `CachedMessage::author`) and a
  compact `CachedMember`: rarely set fields (banner, decorations, collectibles, primary
  guild, boost and timeout times, …) move into one `Option<Box<…>>` read through accessor
  methods, and roles become a `Box<[RoleId]>`. A breaking change, acceptable before the first
  release. Every step is measured; a change that does not save about 20 % is reverted and
  reported.
- **Alternatives:** a compact `CachedMember` only (keep `djs-model`'s `User`); keep 546 B.

### D-176 — A lock-free cache backend stays open
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the writer starvation under oversubscription (`docs/PERFORMANCE.md`,
  Phase 5) is not addressed now; a backend with lock-free reads (and its dependency) remains
  an open question.
- **Alternatives:** evaluate `papaya` / `scc` now as a benchmark dependency.

### D-178 — Builders consume `self`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the `djs-builders` builders take and return `self`
  (`EmbedBuilder::new().title("x").field(…).build()`), like `ClientBuilder`, `RestBuilder` and
  `CacheConfigBuilder`; conditional changes reassign (`builder = builder.field(…)`).
- **Alternatives:** `&mut self` methods (discord.js-like mutation); both styles.

### D-179 — Builders validate in `build()`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** setters only store; `build()` checks every Discord limit and rule and returns
  `Result<ModelType, ValidationError>`. Calls can come in any order, and nothing panics in a
  bot's code. Differs from discord.js, whose setters throw at once.
- **Alternatives:** setters panic on bad values (discord.js-like); setters return `Result` and
  `build()` checks cross-field rules.

### D-180 — `build_unchecked()` skips validation
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** every builder also has `build_unchecked()`, which builds without any check
  (for example when Discord raises a limit before the library does). Chosen per call; no
  global state. discord.js counterpart: `disableValidators()`.
- **Alternatives:** `build()` only; a process-wide switch like discord.js.

### D-181 — Phase 6 order: embeds, commands, components
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the ROADMAP order, one commit (with tests) per step: `EmbedBuilder` first (it
  sets the validation pattern), then slash and context-menu command builders, then
  components, Components V2 and modals.
- **Alternatives:** embeds → components → commands; commands first.

### D-182 — One structured validation error, the first found
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `build()` returns the first violation as a `#[non_exhaustive]` enum
  `ValidationError` with variants such as `TooLong { field, max, actual }`,
  `TooMany { … }` and `Missing { field }`, and a readable `Display`.
- **Alternatives:** all violations at once (`Vec<ValidationError>`); a message-only error.

### D-183 — Limits count Unicode code points after trimming
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a text limit counts the Unicode code points (`chars().count()`) of the text
  without leading and trailing whitespace, which Discord's documentation says it trims.
  discord.js counts UTF-16 code units, so it counts an emoji such as 🔥 twice.
- **Confirmed on the live API (D-184), 2026-11-12:** embed titles of 256 ASCII letters, of
  256 × 🔥 (512 UTF-16 units, 1 024 bytes), of 256 two-byte letters, of 51 family emoji + `a`
  (256 code points, 52 graphemes) and of 256 letters wrapped in spaces were accepted; 257
  letters, 257 × 🔥 and 52 family emoji (260 code points, 52 graphemes) were rejected
  (`50035 Invalid Form Body`). Discord counts code points after trimming — not UTF-16 units,
  graphemes or bytes. The accepted test messages were deleted.
- **Alternatives:** UTF-16 code units like discord.js; decide only after the live test.

### D-184 — Character counting is checked on the live API with the test bot
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a one-off test sends a few messages with embeds at the title limit (ASCII,
  emoji outside the BMP, multi-code-point emoji, surrounding whitespace) from the owner's test
  bot to a channel of its test guild, deletes them, and records which are accepted. The token
  stays in `.env` and is never printed.
- **Alternatives:** no requests to Discord; document the uncertainty.

### D-185 — Builders are named like discord.js and re-exported as `djs_rs::builders`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `EmbedBuilder`, `SlashCommandBuilder`, `ContextMenuCommandBuilder`,
  `ActionRowBuilder`, `ButtonBuilder`, `ModalBuilder`, … as in `@discordjs/builders`; the crate
  is re-exported as `djs_rs::builders` (like `model`, `rest`, `gateway`, `cache`); the prelude
  is unchanged.
- **Alternatives:** the same names, the most common ones also in the prelude; Rust-style entry
  points on the model types (`Embed::builder()`).

### D-186 — `EmbedBuilder` API: piece builders with shortcuts
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `EmbedBuilder::new()` with `title`, `url`, `description`, `color(u32)`,
  `color_rgb(r, g, b)`, `author(…)`, `footer(…)`, `field(name, value)`,
  `inline_field(name, value)`, `fields(iter)`, `image(url)`, `thumbnail(url)`,
  `timestamp(ts)`, `timestamp_now()`, `build()` / `build_unchecked()`. The author and the
  footer are either a plain text (`.author("Bot")`) or a small builder
  (`EmbedAuthorBuilder::new("Bot").icon_url(…).url(…)`, `EmbedFooterBuilder::new(…)
  .icon_url(…)`). `EmbedBuilder::from(embed)` starts from an existing (received) embed.
- **Alternatives:** flat methods (`author_icon_url`, `footer_icon_url`, `field(n, v, inline)`);
  the model types with `with_*` constructors as the pieces.

### D-187 — Embed URLs: the scheme is checked, nothing more
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `build()` requires `http://` or `https://` for the title and author URLs and
  also allows `attachment://` for images, the thumbnail and icons (the protocols discord.js
  allows). The rest of the address is not parsed (no new dependency); Discord rejects a
  malformed one with `50035`.
- **Alternatives:** full parsing with the `url` crate; no URL checks.

### D-188 — Empty texts and empty embeds: checked on the live API first
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a one-off test with the owner's test bot, like D-184, sends embeds with an
  empty or whitespace-only title, description, footer, author name, field name and field
  value, an embed with no content and URLs with a wrong scheme; accepted messages are deleted
  at once and the token stays in `.env`. `build()` then rejects exactly what Discord rejects.
- **Live result, 2026-11-12** (19 messages; the 11 accepted ones deleted at once):
  - **accepted:** an empty or whitespace-only title, an empty description, footer text,
    author name, field name or field value (each next to a non-empty title or description);
    an `ftp://` image URL; an `attachment://` image URL without an uploaded file; an
    upper-case `HTTPS://` title URL; 256 letters padded with NBSP and newlines (so Discord
    trims Unicode whitespace like Rust's `str::trim`);
  - **rejected (`50035`):** an embed with no content — `{}` or only a color, with or without
    message content — as `embeds[0].description: BASE_TYPE_REQUIRED`; an `ftp://` title URL
    (`URL_TYPE_INVALID_SCHEME`, "must be one of ('http', 'https')"); a bare `https://`
    (`URL_TYPE_INVALID_URL`); an `attachment://` author URL (`URL_TYPE_INVALID_SCHEME`); the
    color `0x1000000` (`NUMBER_TYPE_MAX`, at most `16777215`).
- **Alternatives:** follow discord.js (empty title / description / footer / author rejected,
  empty field names and values allowed); no checks for empty texts.

### D-189 — Message-wide embed limits: a function for the list
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `EmbedBuilder::build()` checks the 6000-character total of its own embed; the
  crate also offers `embed_length(&Embed)` (discord.js: `embedLength`) and
  `validate_embeds(&[Embed])` (total ≤ 6000, at most 10 embeds) for a whole message, to be
  used by the Phase 8 message options.
- **Alternatives:** only the per-embed check; a message builder now (scope change).

### D-190 — Image and icon URLs keep the D-187 scheme check
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** although Discord accepts an image URL with any scheme (it just does not show
  it, D-188), `build()` still rejects images, the thumbnail and icons whose URL is not
  `http://`, `https://` or `attachment://`, like discord.js. The title and author URLs allow
  `http://` and `https://` only, as Discord does. Schemes are compared without regard to case
  (Discord accepted `HTTPS://`).
- **Alternatives:** exactly like Discord: check the title and author URLs only.

### D-191 — A second short live test finds what counts as embed content
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** about a dozen messages, each with an embed holding a single part (an empty or
  whitespace-only title, an empty description, an image, a thumbnail, a footer, an author, a
  field, a URL, a timestamp, an empty footer, an empty field) and no message content, sent by
  the test bot like D-184 / D-188, accepted ones deleted at once. `build()` then rejects
  exactly the embeds Discord rejects as empty.
- **Live result, 2026-11-12** (12 messages; the 5 accepted ones deleted at once): an embed
  with only an image, only a thumbnail, only a footer `"f"`, only an author `"a"` or only a
  field — even one with an empty name and value — was accepted. An embed with only an empty
  or whitespace-only title, only an empty description, only a URL, only a timestamp or only an
  empty footer was rejected as `embeds[0].description: BASE_TYPE_REQUIRED`. So an embed has
  content when it has a title, description, footer text or author name that is not blank, an
  image, a thumbnail or at least one field.
- **Alternatives:** a cautious rule without a test (reject only an embed with nothing but a
  color); no empty-embed check.

### D-192 — Command builders: values passed to one `.option()`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `SlashCommandBuilder::new(name, description)` (name and description cannot be
  forgotten), options as values of option types named like discord.js
  (`SlashCommandStringOption::new(name, description).required(true)`, …) passed to a single
  `.option(…)`, subcommands through `.subcommand(SlashCommandSubcommandBuilder)` and
  `.subcommand_group(SlashCommandSubcommandGroupBuilder)`, context-menu commands through
  `ContextMenuCommandBuilder::user(name)` / `::message(name)`. Localizations, default member
  permissions, contexts, integration types and NSFW are setters. `build()` returns
  `CreateApplicationCommand`.
- **Alternatives:** closures like discord.js (`string_option(|o| o.name(…))`, `Missing`
  errors for unset names); values with one method per option type (`.string_option(…)`).

### D-193 — Mixing subcommands and options is checked in `build()`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** one `SlashCommandBuilder` type; a level that mixes subcommands (or groups)
  with other options is a `ValidationError` from `build()`, so a command can be assembled in a
  loop.
- **Alternatives:** type states like discord.js (`SlashCommandSubcommandsOnlyBuilder`), a
  compile-time error.

### D-194 — Command names: a dependency-free approximation of Discord's pattern
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** slash command, subcommand and option names (and their localizations) must have
  1-32 characters, each a Unicode letter or number by the standard library
  (`char::is_alphabetic` / `char::is_numeric`), `-`, `_`, `ʼ` (U+02BC), or from the
  Devanagari or Thai blocks, and no letter that has a lowercase form. The accepted set is a
  little wider than Discord's `^[-_\u02BC\p{L}\p{N}\p{sc=Deva}\p{sc=Thai}]{1,32}$` (for
  example combining marks of other scripts), so a valid name is never rejected; Discord
  rejects the rest.
- **Alternatives:** the exact pattern with the `regex` crate (new dependency); only the length
  and lowercase check.

### D-195 — Unclear command rules are checked on the live API in the test guild
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** about 20 guild commands created by the test bot in its test guild (Discord
  allows 200 creations per guild per day), each deleted at once, to settle what the docs leave
  open: how the 8000-character total and descriptions are counted (code points? trimmed?),
  edge-case names, repeated option names, `min` above `max`, empty values. The token stays in
  `.env`.
- **Live result, 2026-11-12** (20 guild commands in the test guild, which had none; the 10
  accepted ones deleted at once, none left):
  - **descriptions** count code points after trimming: 100 × 🔥 accepted, 101 rejected
    (`BASE_TYPE_MAX_LENGTH`); 100 letters padded with spaces accepted; a whitespace-only
    description rejected as `BASE_TYPE_REQUIRED`;
  - **names:** an upper-case `Ż` rejected (`APPLICATION_COMMAND_INVALID_NAME`), an ASCII
    apostrophe rejected (`STRING_TYPE_REGEX`), `ʼ` (U+02BC), Devanagari with vowel signs and
    32 code points of two-byte letters accepted; a user command named with spaces and 🔥
    accepted;
  - **options:** a repeated option name rejected (`APPLICATION_COMMAND_OPTIONS_NAME_INVALID`),
    a required option after an optional one rejected (`APPLICATION_COMMAND_OPTIONS_REQUIRED_INVALID`),
    `min_length` 10 with `max_length` 5 rejected (`max_length` must be at least `min_length`),
    a whitespace-only choice name rejected (`BASE_TYPE_BAD_LENGTH`, 1-100); **accepted although
    the docs forbid or it cannot work:** `min_value` 10 with `max_value` 5, and choices together
    with `autocomplete: true`;
  - **the 8000 total** counts code points of the command, option and choice names,
    descriptions and string choice values: exactly 8000 ASCII accepted, 8001 rejected
    (`APPLICATION_COMMAND_TOO_LARGE`), 8000 code points with 🔥 (15 452 UTF-16 units) accepted,
    and a localized name one longer than the default pushed 8000 to a rejected 8001 (the longest
    localization of a field counts).
- **Alternatives:** the documentation and discord.js only, unclear cases documented as such.

---

## Defaults chosen by Claude (reviewable)

These are lower-impact choices made during setup. The owner may override any of them.

### C-001 — Discord API v10
discord.js v14 targets API v10; so does djs-rs. (Effectively dictated by the goal.)

### C-002 — Workspace lints
`unsafe_code = "deny"` (exceptions need owner approval + `// SAFETY:`), `missing_docs = "warn"`
(CI turns warnings into errors), `clippy::pedantic` as warnings with
`module_name_repetitions` allowed, rustdoc broken intra-doc links denied.

### C-003 — Crate layout `crates/<name>/`, `publish = false` until release
All crates live under `crates/`. Every crate has `publish = false` so nothing is published to
crates.io by accident; publishing is an open question for the owner.

### C-004 — Release profile for workspace binaries
`lto = "fat"`, `codegen-units = 1` for examples/benchmarks built inside this workspace. (This
does not affect downstream users, who control their own profiles.)

### C-005 — Conventional Commits, Keep a Changelog
Commit messages follow Conventional Commits; `CHANGELOG.md` follows Keep a Changelog.

### C-007 — `Cargo.lock` is committed
Reproducible CI and MSRV checks (current Cargo guidance for libraries as well).

### C-008 — `ImageHash` stored as 16 bytes + animated flag
Image hashes (avatars, icons, …) are decoded from 32 hex chars (+ optional `a_`) into
`[u8; 16]` + `bool` (17 bytes, `Copy`) instead of a 34-byte heap string. Strict parsing: a hash
that is not 32 hex chars fails deserialization (same trade-off twilight has made for years).

### C-009 — Gateway decoding via `GatewayEventDeserializer` (DeserializeSeed)
`djs-model` provides a format-agnostic `DeserializeSeed` that is given `op`/`s`/`t` up front and
decodes `d` in one pass into the concrete type; unhandled events are skipped with `IgnoredAny`.
A JSON helper (`from_json`) scans only top-level keys (stops before `d` in Discord's key order).

### C-010 — Forward-compatibility policy
Integer enums carry an `Unknown(raw)` variant and are `#[non_exhaustive]`; equality/hash are by
raw value. Structs ignore unknown fields (no `deny_unknown_fields`). Bitflags keep unknown bits.

### C-011 — Secrets are redacted in `Debug`
Payloads carrying the bot token (`Identify`, `Resume`) implement `Debug` manually and print
`<redacted>`.

### C-012 — REST request/response bodies live in `djs-model::rest`
Mirrors discord-api-types, which contains both payload and REST body types (D-007 mapping).
`djs-rest` contains only the client, routing and rate limiting. Body structs are plain data with
`Default` (+ `From<&str>` for message bodies); validated builders come with `djs-builders`.

### C-013 — Structures = plain model types + extension traits taking `&Context`
Follows the approved D-006 shape (`msg.reply(&ctx, "Pong!")`): behaviour lives in sealed
extension traits in `djs_rs::ext` (re-exported by the prelude) instead of wrapper structs that
store a client reference. Keeps structures small and free of reference cycles.

### C-014 — Handlers return `Result<(), HandlerError>` (`Box<dyn Error + Send + Sync>`)
Lets users write bare `Ok(())` and use `?` on any error (verified: rustc infers the async
block's output from the bound). Errors are logged via `tracing` at `error` level.

### C-015 — `Events` as associated constants; handler payloads as `Arc<Payload>`
`Events::MessageCreate` is a zero-sized value whose type selects the payload type at compile
time. Payloads are converted once from `Box` to `Arc` and shared by all handlers/subscribers.
Concurrent mode spawns handler futures directly (no extra boxing); sequential mode boxes.

### C-016 — `client.events()` streams are unbounded
Each subscriber gets an unbounded channel (documented: consume promptly or drop). Closed
subscribers are pruned automatically; there is no cost when nobody subscribes.

### C-017 — Stress tests live in `tests/stress.rs`, ignored by default
Each crate's extreme-load tests (D-041) are `#[ignore]`d integration tests in
`tests/stress.rs`, run with `--release --ignored --nocapture`. They print `[stress]` lines,
use in-process mock servers that also check correctness (sequence order, 429s the real API
would return), and read memory from `/proc/self/status` (`VmRSS`/`VmHWM`, peak reset through
`/proc/self/clear_refs`). Micro-benchmarks live in `benches/` (divan). Results go to
`docs/PERFORMANCE.md`.

### C-018 — String-valued enums use `str_enum!` with an `Unknown` variant
The string counterpart of `int_enum!` (C-010): `#[non_exhaustive]`, `Unknown(CompactString)`,
equality/hash/order by the string value, (de)serialized as the bare string. Used for
`Locale`, `NameplatePalette` and every other string enum.

### C-019 — Items from the docs' "Reference" page live in `util` / `id`
The Reference page defines snowflakes (`id`), ISO 8601 timestamps, image hashes and locales;
they stay in `id` and `util` (`util::Locale`) instead of a `reference` module, because they are
used everywhere.

### C-020 — `clippy::struct_excessive_bools` is allowed in `djs-model`
Model types mirror Discord's JSON objects field by field; several have more than three boolean
fields. Allowed crate-wide in `djs-model` with a reason; still denied elsewhere.

### C-021 — Strict D-048 naming plus `#[doc(alias)]` for discord.js names
Types follow D-048 strictly (`guild::Feature`, `guild::Preview`, `guild::AgeRestrictionLevel`
for the docs' "Guild Age-Restriction Level"), and carry `#[doc(alias = "...")]` with the
discord.js / older names (`GuildFeature`, `GuildPreview`, `NsfwLevel`) so rustdoc search finds
them.

### C-022 — JSON examples from the Discord docs are used as test fixtures
`crates/djs-model/tests/fixtures/docs/*.json` hold the example payloads of the Discord API docs
(factual payload data, not prose). Tests deserialize them and check that re-serializing loses
no field (absent/null/false/empty may collapse, D-049).

### C-023 — Defensive defaults for required fields missing from older payloads
Required fields that the docs' own examples omit (`Guild::nsfw_level`,
`premium_progress_bar_enabled`, `Onboarding::mode`) get `#[serde(default)]` so older or trimmed
payloads still parse (C-010 spirit).

### C-024 — `#[serde(flatten)]` only on cold paths
`flatten` buffers the input (allocations), so it is never used for gateway hot-path types. It is
used where an object "extends" another on REST-only/rare paths (`InviteWithMetadata`) and for
serializing `Channel` variants with their `type`.

### C-025 — `ImageHash` stays strict
Image hashes must be 32 hex digits (optionally `a_`-prefixed); placeholder values like the
docs' `deadbeef` are rejected. Fixtures use real-looking hashes instead.

### C-026 — Outdated docs examples are corrected in the fixture, not in the model
When a docs example omits a required field that has no meaningful default (the application
example's team members lack `role`, added in 2023), the field stays required — as in the docs
table and discord-api-types — and the fixture adds a valid value. Fields with a sensible default
keep following C-023. `Application::flags_new` (string-encoded flags) uses the generic
`serde_util::option_flags_string` helper.

### C-027 — Names where D-048's prefix rule does not fit
- `application_role_connection_metadata::MetadataType` (Discord:
  `ApplicationRoleConnectionMetadataType`): dropping the whole module prefix would leave just
  `Type`; the object itself keeps its full name `ApplicationRoleConnectionMetadata`, like `Team`
  in `team`.
- `user::ConnectionService` / `ConnectionVisibility`: the prefix is the object (`Connection`), not
  the module, so it stays (discord-api-types names). `ConnectionService::X` serializes as
  `"twitter"` (doc alias `Twitter`).
- Every such name has a `#[doc(alias)]` for the Discord / discord.js name (C-021).

### C-028 — Field optionality of the template snapshot types
The snapshot follows the "create guild" format, where most fields are optional: those are
`Option`s (levels, AFK timeout, region, channel settings). Fields every snapshot has, and that
other parts reference, are required (`name`, `preferred_locale`, role/channel `id`, channel
`type`). Role and overwrite fields that may be `null` in that format (`permissions`, `color`,
`hoist`, `mentionable`, `allow`, `deny`) read `null` as the default
(`serde_util::null_as_default`).

### C-029 — Message model details
- **Names** (D-048 "prefix stays where the short name would be misleading"): `Message` stays on
  generic or clashing names (`MessageType`, `MessageFlags`, `MessageReference`,
  `MessageActivity` (vs presence activities), `MessageCall`, `MessageSnapshot`, `MessagePin`);
  the rest are short (`Embed*`, `Attachment`, `Reaction`, `ChannelMention`, `AllowedMentions`,
  `AllowedMentionType`, `RoleSubscriptionData`, `SharedClientTheme`, `BaseTheme`, `Mention`,
  `ReferencedMessage`, `Nonce`). The snapshot's partial message is `SnapshotMessage`; the
  interaction metadata variants are `CommandMetadata`, `ComponentMetadata`,
  `ModalSubmitMetadata`, `UnknownMetadata` (as in the D-063 preview). All with doc aliases.
- `GuildInstallOwner::{Guild(GuildId), BotDm}` represents the guild-install owner, which Discord
  sends as `"0"` for DMs with the bot (`Id` cannot be `0`).
- `Attachment::application` is a `PartialApplication` (clips carry a partial object).
- Required arrays (`mentions`, `mention_roles`, `attachments`, `embeds`) default to empty when
  absent (C-023) and are always written. Flags of received types (`Message`, `Attachment`) are
  plain flags defaulting to empty, like `Channel`; `Embed::flags` stays an `Option` because
  embeds are also sent.
- `Nonce` is `Integer(i64)` or `String`; integers beyond `i64` are kept as their text.
- `message` is split into private submodules (`embed`, `attachment`, `interaction_metadata`)
  re-exported from `message`, so public paths stay `message::Embed` etc.

### C-030 — Component model details
- Components are sent as well as received, but their structs are `#[non_exhaustive]` (D-053):
  each has a `new(..)` constructor taking the required fields; optional fields are public and set
  afterwards. `From` converts every struct (and every button kind) into `Component`.
- The user, role and mentionable selects share `AutoPopulatedSelect` (identical structures; like
  `TextChannel` for two channel variants); radio and checkbox group options share
  `GroupOption`.
- `id` is `Option<i32>` (a "32 bit integer"); `min_values`/`max_values` are `Option<u8>`,
  `min_length`/`max_length` `Option<u16>` (Discord's defaults differ per component).
- `required` (inputs) and `divider` (separator) are `true` when absent and always written, like
  `available` (D-055).
- `Button::Unknown` is boxed (it keeps every button field), so `Component` stays at 120 bytes.
- The undocumented `ContentInventoryEntry` (16) is a known `ComponentType` decoded as
  `Component::Unknown`; the undocumented loading state's `0` is `LoadingState::Unspecified`
  (`Unknown` is taken by the macro's catch-all variant).
- The docs' component examples are JSONC; the fixture `component_reference_examples.json` holds
  them with comments removed. Interaction-data examples (with `...additionalInteractionFields`)
  are left for the interactions step.

### C-031 — Application command model details
- Names: the object keeps its full name `ApplicationCommand` (module = object, like
  `GuildTemplate`); the rest drop "Application" (`CommandType`, `CommandOption`,
  `CommandOptionType`, `CommandPermission`, `CommandPermissionType`, `GuildCommandPermissions`,
  `EntryPointHandlerType`) and the choice is `Choice<T>`; doc aliases give the Discord names.
- Shared structs: `SubCommandOption` (subcommand and group), `BasicOption` (boolean, user, role,
  mentionable). `CommandOption::common()` returns an `OptionInfo` with the fields every option
  has.
- Choices and bounds are checked against the option type when decoding (a string choice on an
  integer option is an error); integers convert to `f64` for number options.
- `name_localized` / `description_localized` (returned by list endpoints instead of the full
  dictionaries) are modelled on commands and options.
- The deprecated `dm_permission` / `default_permission` are `Option<bool>` (their default is
  `true`, so absent ≠ `false`).
- The docs show creation bodies; the tests add `id`, `application_id`, `version` to decode them
  as returned commands.

### C-032 — Interaction model details
- Layout: `interaction/{mod,data,response}.rs`. The data variants are boxed
  (`InteractionData` is 16 B, `Interaction` ≤ 288 B); `Interaction::guild` stays inline.
  `version` defaults to `1`, `app_permissions` to empty (pings have none), `entitlements` and
  `authorizing_integration_owners` to empty. `Debug` redacts the token (C-011).
- Command option values: a focused autocomplete option keeps the typed text as a string
  (`CommandOptionValue::Focused { kind, value }`), because it need not be valid yet; other
  integer/number options also accept numeric strings. Unknown option types, submitted
  component types and callback types keep only their type (and `id`).
- Modal submits: user/role/mentionable/channel selects are one generic
  `SubmittedEntitySelect<T>` with typed IDs; file uploads have `Vec<AttachmentId>`; string
  selects and checkbox groups share `SubmittedSelect`. A null radio group value is `None`, an
  absent checkbox value is `false`. Helpers: `ModalSubmitData::{get, text_value}`.
- Message component data keeps `values` as strings (they are option values or IDs depending
  on the component); `ComponentData::ids::<T>()` parses them.
- `InteractionChannel` also has `app_permissions` (listed among the partial channel fields) and
  `recipients`; `InteractionMember` dereferences to `PartialMember` like `Mention` (D-060).
- Response arrays (`MessageCallbackData::{embeds, components, attachments}`) are
  `Option<Vec<_>>`: for `UpdateMessage`, absent keeps the message's value and `Some(vec![])`
  clears it. The general tri-state PATCH question (D-049) stays for Phase 3.
- `AutocompleteChoices` is typed (string/integer/number); when decoding, a list with any
  fractional number is a number list and an empty list is a string list.
- `AttachmentRequestId`: numbers that fit in `u32` are upload indices (`files[n]`), strings
  and larger numbers are attachment IDs. `CreatePoll::answers` is `Vec<Media>` written as
  `[{"poll_media": …}]`; `Media::new(text)` builds text media.
- `Message::resolved` was added (Message size guard now 656 B).

### C-033 — Audit log model details
- Names: `AuditLog`, `AuditLogEntry`, `AuditLogEvent` (= discord.js), `AuditLogEntryOptions`
  (doc alias `OptionalAuditEntryInfo`), `AuditLogChange`, `AuditLogValue` (renamed to
  `util::JsonValue` / `JsonValueError` by D-072) with variants
  `Null`, `Bool`, `Unsigned` (non-negative integers), `Signed` (negative), `Float`, `String`,
  `Array`, `Object` (a `BTreeMap`, D-058), and the conversion error `AuditLogValueError`.
- `&AuditLogValue` implements serde's `Deserializer` (borrowed strings, options, newtypes,
  unit enum variants from strings); `AuditLogChange::{old_value_as, new_value_as}` return
  `Result<Option<T>, _>` (absent = `Ok(None)`).
- `AuditLogEntry::target_id` is `Option<GenericId>` (the docs say `?string`; every documented
  target is a snowflake, as in twilight). `guild_id` is added for the
  `GUILD_AUDIT_LOG_ENTRY_CREATE` gateway event (like `Message::member`). `options` is boxed.
- Option counts (`count`, `delete_member_days`, `members_removed`) are `u32`; the trigger type
  and overwrite type are their enums; all are written back as strings, and numbers are
  accepted when decoding. `integration_type` is `guild::IntegrationType`.
- `guild::PartialIntegration::application_id` was added (shown in the docs' audit log example,
  absent from the integration table).
- The docs have no full audit log example: the test builds one from the docs' tables around
  the docs' example thread, webhook, auto moderation rule, command and partial integration.

### C-034 — SKU, subscription and voice details
- `sku::Sku` (`kind: SkuType`), `subscription::{Subscription, SubscriptionStatus}`: the
  nullable `renewal_sku_ids` is `Option<Vec<SkuId>>` (`null` means unchanged, written as
  `null` per D-049); the example SKU's undocumented fields are not modelled.
- `voice::{VoiceState, VoiceRegion}` (one module per docs page, D-044): `member` is boxed
  (D-054, guard ≤ 96 B); absent voice state booleans are `false` (C-023, the docs' example
  omits `self_video`) and are always written. The `GUILD_CREATE` extras (voice states, members,
  channels, …) come with the dispatch events.

### C-035 — Soundboard and lobby details
- `soundboard::SoundboardSound` (`PartialEq` only, `volume: f64`, D-064); absent `available`
  is `true` (D-055). The `{"items": […]}` list wrapper and the send/create bodies are REST
  (Phase 3).
- `lobby::{Lobby, LobbyMember, LobbyMemberFlags, LobbyMessage, LobbyMessageMember,
  LobbyInvite}`: the nullable string dictionaries are `BTreeMap`s where `null`/absent are empty
  (D-058, like `name_localizations`), skipped when empty. Request bodies are Phase 3.

### C-036 — Application identity profile details
- One `ApplicationIdentity` type with an optional `user_id` (the identity lists include it,
  the identity table does not; discord-api-types has two interfaces).
- `ApplicationIdentityProfile::metadata` is `Option<BTreeMap<CompactString, JsonValue>>`
  (`?object`, written as `null` when `None`, D-049/D-072). `ProfileData` doubles as the update
  body (`Default` + public fields). The media object is `ProfileMedia` (not `Media`, which is
  the poll media).
- Dynamic fields follow the option-value pattern (C-032): `DynamicField { name, value }` with a
  `DynamicFieldValue` enum by type; the raw value is decoded as a `JsonValue` and converted
  after reading `type`. Integer stats are `u64`, `playtime_hours` is `f64`.
- `JsonValue` moved from `audit_log` to `util` (D-072).

### C-037 — Gateway dispatch event and command details
- Events whose payload is "an object plus extra fields" (D-073) decode in a single pass with
  `serde_util::extras`: a `MapAccess` adapter that hands the keys the event's extras type
  recognises to it and every other key to the base type. Extras can `Replay` a key to the base
  type as well (`GUILD_CREATE` reads `id` for both) and can `recover` when the base type fails
  (an unavailable `GUILD_CREATE` lacks the guild fields). Escaped keys are supported.
- Two private macros: `newtype_event!` (transparent newtype with `Deref`/`DerefMut`/`From`) for
  events whose data is an existing object, and `guild_id_event!` (`{ guild_id, <object> }`
  with `Deref` to the object) for the "object + `guild_id`" events.
- `DispatchEvent` variants hold payloads of at most 24 bytes inline and box the rest, keeping
  the enum at 32 bytes. Unknown events keep their name and skip their data; `RESUMED` ignores
  its data. `DispatchEvent` also implements `Serialize` (it writes the `d` data, `null` for
  `Resumed`/`Unknown`) for tests, proxies and recordings.
- `Identify` gained `presence` and `capabilities` (empty capabilities are not written);
  per-payload `compress` is not modelled (transport compression is used instead, D-015).
  Gateway commands are `Serialize`-only: bots never receive them.
- `GUILD_CREATE`: the fields only it carries default when absent; `joined_at` is an `Option`.
  `GUILD_MEMBER_UPDATE`: `deaf`, `mute` and `pending` are `Option<bool>` (optional in the event,
  unlike in `Member`). `THREAD_LIST_SYNC`: `channel_ids` is an `Option` (absent means the whole
  guild was synced, which differs from an empty list). `GUILD_MEMBERS_CHUNK`: `not_found` is a
  `Vec<UserId>`.

### C-038 — Webhook event details
- Module `webhook_event` (singular like the other modules, for the "Webhook Events" page,
  D-044). `WebhookEventType` moved there from `application`, because that page defines it.
  The `type` of the outer payload is `PayloadType` (`Ping`/`Event`), so it does not clash with
  `webhook::WebhookType`.
- `WebhookEvent { timestamp, data: WebhookEventData }` follows the "enum by type" pattern
  (D-067): the raw `data` is buffered as a `JsonValue` and converted once `type` is known (it
  may come first; webhook events are rare, so the second pass is cheap). Unknown event types
  keep only their type; `QUEST_USER_ENROLLMENT` ignores any data.
- Event body times have no offset and are read as UTC through the crate-private
  `Timestamp::parse_assume_utc`; the public `Timestamp::parse` stays strict. They are written
  back with `+00:00`.
- `ApplicationAuthorized::guild` is boxed, `scopes` defaults to empty (C-023).
  `GameDirectMessage` (D-079): `kind` is an `Option` (`MessageType` has no default),
  `recipient_id` is a `GenericId` (undocumented; in the docs' example it equals the channel's
  ID), `flags` default to empty, and message fields it does not list are ignored.

### C-039 — REST infrastructure details (Phase 3)
- Global limiter (D-089): the window is extended by the rate-limit offset (default 50 ms), so
  a full burst of 50 is followed by the next request 1.05 s after the first; the log grows
  only as far as requests are actually sent within a window.
- Invalid requests (D-085): `Rest::invalid_requests()`, `RestBuilder::invalid_request_limit(n)`
  (`0` = off, the default, like `global_requests_per_second(0)`), `ErrorKind::InvalidRequestLimit
  { count, limit }`. The window is 60 slots of 10 s (constant memory, errs on the safe side);
  a `tracing` warning at every 1 000 counted requests. The limit is checked before every
  attempt, retries included.
- Routes (D-090, D-091): every endpoint is one line of the private `routes!` macro (variant,
  fields, method, path template, major parameters, limits), which generates `Route<'a>`
  (`Copy`, `#[non_exhaustive]`, variant docs show `METHOD /path`), `method()`, `path()`, the
  rate-limit key and a `Debug` that prints `<redacted>` for fields whose name ends in `token`.
  Text segments (tokens) are percent-encoded (everything but RFC 3986 unreserved characters),
  so a token cannot change the path. A route with several major parameters (webhook id +
  token) combines them; a token enters the key as its FNV-1a hash, so the key stays a `u64`.
- Interaction limits: interaction endpoints (`/interactions/...`, `/webhooks/{application_id}/
  {interaction_token}...`) skip the global limiter and do not pause it on a global 429
  (Discord: they are not bound to the global limit). Callbacks are additionally *burst*
  routes (like discord.js' `BurstHandler`): they never wait for their bucket, so concurrent
  callbacks are sent at once; a 429 delays only the request that got it.
- Old message deletes: `Route::DeleteMessage` for a message older than 14 days (from its ID,
  against the system clock) uses its own bucket template, like discord.js (Discord limits
  them separately, discord-api-docs issue 1295). Same major parameter.
- Audit log reason (D-086): `Request::reason(Option<&'a str>)` (the same type as the endpoint
  methods' `reason` parameter, so they pass it through) and the getter
  `Request::audit_log_reason()`. The text is percent-encoded with the path encoder
  (everything but RFC 3986 unreserved characters — a superset of `encodeURIComponent`, which
  Discord decodes the same way), so line breaks or non-ASCII text can never break the header.
  Like discord.js, `Some("")` sends an empty header.
- Logging: request log fields name the route template (`POST /interactions/{interaction_id}/
  {interaction_token}/callback`), never the URI, which can contain tokens.

### C-040 — File upload details (D-087, D-093..D-096)
- `FileUpload { name, data: Bytes, content_type, description, title, duration_secs, waveform,
  is_spoiler: bool }` (`#[non_exhaustive]`, `FileUpload::new(name, impl Into<Bytes>)`); its
  `Debug` shows the data size, not the data. The metadata fields mirror the docs' attachment
  request structure.
- Bodies with files so far: `CreateMessage` (gained `attachments` + `files`, lost `Eq` because
  of `duration_secs: f64`) and `MessageCallbackData` (gained `files`); both serialize through a
  borrowed wire struct, destructuring `self` without `..` so a new field cannot be forgotten.
  `InteractionResponse::files()` returns the files of its message data.
- Generated `attachments` entries come after the explicit ones; a file whose `files[n]` index
  already has an explicit entry gets no generated one (the explicit entry wins). An explicit
  `Some(vec![])` without files is still sent (it removes attachments when editing).
- `Request::files(&'a [FileUpload])` borrows the files; getter `Request::uploads()`. The body
  is built once per request as `Bytes` chunks (part headers + each file's data as is) and
  re-sent from those chunks on retries, with an exact `Content-Length`; parts are `files[n]`
  in order, then `payload_json`.
- The boundary is `djs-rs-` + 128 random bits from `std::hash::RandomState` (no dependency).
  File names are escaped like browsers do (`"` → `%22`, CR → `%0D`, LF → `%0A`). A given
  content type containing control or non-ASCII characters is ignored in favour of the guess.
- Guessed text types carry `; charset=utf-8`; `.opus` maps to `audio/ogg` (Ogg container).

### C-041 — API error details (D-097..D-099)
- `ErrorCode` has the 245 codes of the docs table, generated into
  `djs-model/src/rest/error_code.rs` (so `rest.rs` became `rest/mod.rs`) by
  `scripts/gen_error_codes.py` — the repository's first dev script, kept so the enum can be
  regenerated when the docs add codes (new codes need a description in the script). Code 20017, which discord-api-types still lists but the docs do not, is
  left out (it decodes as `Unknown(20017)`). Three documented codes have no discord-api-types
  name yet: 30084 `InviteTargetUsersLimitReached`, 40114
  `InviteTargetUsersAlreadyBeingProcessed`, 40115 `InviteTargetUsersFileNotProcessed`.
- Acronyms are written as words (C-021): `UnknownSku`, `InvalidApiVersion`,
  `IdTokenJwtExpired`, `NsfwChannelMessageReferenceNotAllowed`… (`OAuth2` stays as written);
  every renamed variant has `#[doc(alias)]` with the original name, and every variant has
  its number as an alias, so rustdoc search finds `50013`. Each variant's doc starts with
  its number, followed by the library's own description.
- `ApiError` is `#[non_exhaustive]` with `code: ErrorCode`, `message`, `field_errors:
  Vec<FieldError>` (parsed when the error is decoded) and the raw `errors`. `FieldError {
  path, code, message }` (all `String`, cold path). Paths: root keys as is (`0.code` for
  top-level indices), nested keys `.name`, digit-only keys `[n]`; keys starting with `_` do
  not extend the path; plain string values become errors with an empty code.
  `Display`: `message (code N)` then one `path[CODE]: message` line per field error.

### C-042 — `native-roots` details (D-088, D-100)
- `djs-rest` now builds its rustls `ClientConfig` itself (`src/tls.rs`, passed to
  `hyper_rustls::HttpsConnectorBuilder::with_tls_config`; ALPN still set by the builder for
  HTTP/2 + HTTP/1.1), so `hyper-rustls` lost its `webpki-roots` feature and `webpki-roots`
  became a direct dependency of `djs-rest`, as in `djs-gateway`.
- The root-store function is duplicated in `djs-rest/src/tls.rs` and `djs-gateway/src/tls.rs`
  (25 lines; the crates do not depend on each other and `djs-util` has no TLS dependency).
- The system store is read once per client: when `Rest` is built and when a gateway
  configuration is built (one connector shared by its shards).
- Verified live (2026-11-11, Fedora): the `ping` example with `native-roots` loaded 118 system
  certificates, `GET /gateway/bot` returned 200 and the shard became ready.

### C-043 — `Patch<T>` and query details (D-092, D-101)
- `Patch<T>` is `Copy` when `T` is, `Default` = `Keep`; helpers `is_keep`/`is_clear`/`is_set`,
  `into_set`, `as_ref`, `map`. Serializing `Keep` by itself returns a serde error (there is no
  JSON value for "leave out") instead of silently sending `null`; bodies always skip it with
  `skip_serializing_if = "Patch::is_keep"`. It also deserializes (`null` → `Clear`, missing
  field with `#[serde(default)]` → `Keep`).
- Query strings: `djs-rest` has a private `Query` trait implemented for the query structs of
  `djs_model::rest` (allowed by the orphan rule, as the trait is local), and `Request` borrows
  the query struct (`&'a dyn Query`) so the pairs are written straight into the URI when it
  is built — no intermediate string. Values: integers, booleans, IDs, and text
  percent-encoded like path segments.

### C-044 — Endpoint conventions (from the message endpoints on)
- Body and query structs are `#[non_exhaustive]` with `Default` (fields can be added without
  breaking callers; build them with `default()`/`From` and assign fields). `CreateMessage`
  became `#[non_exhaustive]` too. They live in one `djs_model::rest` submodule per docs page
  (`rest/message.rs`, …), re-exported flat (`djs_model::rest::CreateMessage`).
- Create bodies use `Vec` fields skipped when empty and `Option` for the rest; modify bodies
  use `Patch` (D-101). Query field `type` is named `kind` (as in the model). Responses that
  the docs do not name get the resource name (`ChannelPins` for Get Channel Pins).
- Rest methods live in `djs-rest/src/endpoints/<page>.rs`; `reason: Option<&str>` is the last
  parameter exactly where the docs say the endpoint supports `X-Audit-Log-Reason` (so not on
  Delete Message, whose page does not mention it). Each method documents the permissions it
  needs, the discord.js counterpart and the most likely error codes.
- Reaction emojis are never part of the rate-limit key (Discord limits reactions per
  channel); a custom emoji whose name is unknown is written as `_:id`.
- Tests: `djs-rest/tests/common/mod.rs` holds the mock API (it records method, path, query,
  headers and body); one test file per resource checks every request of its endpoints.

### C-045 — Message search details (D-104)
- Query struct `SearchGuildMessages` with one field per documented parameter; list
  parameters are `Vec`s written as repeated keys (`?author_id=1&author_id=2`).
- Negatable filters (author types, "has" types) are `SearchFilter<T>` with `Include(T)` and
  `Exclude(T)` (written with a leading `-`), `From<T>` = include. Enum names follow the docs'
  tables: `SearchAuthorType`, `SearchHasType`, `SearchEmbedType`, `SearchSortMode`, plus
  `SearchSortOrder` (`Ascending` = `asc`, `Descending` = `desc`; the docs have no table).
- The response is `MessageSearchResults` (the docs only call it "Response Body").
- `ErrorKind::NotYetIndexed { retry_after: Duration, documents_indexed: u64 }`; a missing or
  malformed 202 body gives zeros. `Rest::execute` now also returns the status code
  (crate-private) so a method can tell a 202 from a 200.

### C-046 — Application command endpoint details
- `CreateApplicationCommand` serves Create (global/guild) and, as a slice, the bulk
  overwrites. The deprecated `dm_permission` and `default_permission` request fields are left
  out (like the deprecated endpoints of D-105; they have successors); the response type
  still reads them.
  Constructors `new(name, kind)`, `chat_input(name, description)`, `user(name)`,
  `message(name)`; `kind` is always sent so a body never relies on Discord's default.
- `EditApplicationCommand`: `Option` fields, `Patch` for the nullable ones
  (`name_localizations`, `description_localizations`, `default_member_permissions`).
- The guild ID in `/applications/{application_id}/guilds/{guild_id}/...` is **not** a major
  parameter (Discord's major parameters lead the path; discord.js agrees), so guild command
  routes share buckets across guilds. Being wrong here only serialises more requests.
- Batch Edit Application Command Permissions is not modelled (Discord disabled it).
- `edit_application_command_permissions` documents that it needs a `bearer_token` client.

### C-047 — Channel endpoint details
- Route variants follow the docs' endpoint names: `DeleteChannel` for "Delete/Close
  Channel"; Start Thread without Message and Start Thread in Forum or Media Channel are two
  variants of the same `POST /channels/{channel_id}/threads` (so they share a bucket).
- Permission overwrite endpoints take `overwrite_id: GenericId` (a role or user ID, e.g.
  `role_id.cast()`) next to the body, like the path; `EditChannelPermissions::new(kind)`.
- `StartThreadWithoutMessage` has a required `kind` (always sent; Discord's default is
  slated to go away). Modify bodies use `Patch` for the nullable fields, create bodies
  `Option`. `ModifyGroupDm::icon` is a `Patch<ImageData>` (null removes the icon).
- `modify_group_dm` has no `reason` (DMs have no audit log); thread member endpoints and
  lists have none either (the docs do not mention the header).
- Create Channel Invite returns `InviteWithMetadata` (like discord-api-types'
  `APIExtendedInvite`); its JSON body is always sent, `{}` when empty, as Discord requires.
- Endpoints whose response the docs do not describe (Set Voice Channel Status, Trigger
  Typing Indicator, Group DM Add/Remove Recipient) discard any body
  (`Rest::request_ignoring_body`, crate-private).
- `GroupDmAddRecipient` redacts its access token in `Debug` (C-011).
- Query structs: `GetChannelPermissions` (shared by member and role permissions),
  `GetThreadMember`, `ListThreadMembers`, `ListArchivedThreads` (public and private),
  `ListJoinedPrivateArchivedThreads`; `limit` is `u8` where the docs give a maximum of 100,
  `u16` where they give none.

### C-048 — Guild endpoint details
- Deprecated request fields are left out (like C-046): Modify Guild `region`, role
  `color` (use `colors`), Create Guild Ban `delete_message_days` (use
  `delete_message_seconds`), the Begin Guild Prune body `reason` (the method's `reason` sets
  the header). Modify Current User Nick is left out (D-105).
- Single-value responses are unwrapped like D-110: Get Guild Prune Count → `u32` (`0` when
  Discord sends `null`), Begin Guild Prune → `Option<u32>` (`None` without
  `compute_prune_count`). Get Guild Role Member Counts → `HashMap<RoleId, u32>`. Add Guild
  Member → `Option<Member>` (`None` on 204, already a member).
- Get Guild Prune Count's `include_roles` is comma-separated as documented
  (`QueryWriter::comma_separated`); every other list query uses repeated keys.
- Positions endpoints take slices: `&[ModifyGuildChannelPosition]`,
  `&[ModifyGuildRolePosition]` (constructors `new(id, position)`).
- `AddGuildMember` redacts its access token in `Debug` (C-011). Generated onboarding prompt
  IDs use the current time plus a process-wide 12-bit counter in the increment bits.
- Model types used inside request bodies need public constructors because they are
  `#[non_exhaustive]`: added for `Overwrite`, `DefaultReaction` and `WelcomeScreenChannel`
  (check this for every new request that embeds a model type).
- `endpoints/guild.rs` groups its methods in several `impl Rest` blocks (guild, members,
  bans, roles, prune…, widget…) so the rustdoc page stays navigable.

### C-049 — User endpoint details
- Get Current User Guilds returns `djs_model::rest::CurrentUserGuild` (the docs' "partial
  guild" with `owner`, `permissions` and the approximate counts; discord.js `OAuth2Guild`).
- Create Group DM is included (not deprecated itself, no successor; only its SDK is).
  `CreateGroupDm` redacts its access tokens in `Debug` and keys `nicks` by `UserId` in a
  `BTreeMap` (stable output).
- Update Current User Application Role Connection takes the model's
  `ApplicationRoleConnection` as its body (same fields; it has `Default`).
- `/users/@me/...` routes have no major parameter; Delete Current User Application Role
  Connection discards its (undocumented) response body.

### C-050 — Webhook endpoint details
- One `ModifyWebhook` body serves Modify Webhook and Modify Webhook with Token; the latter
  documents that `channel_id` is not accepted (Discord rejects it). Modify Webhook with
  Token and Delete Webhook with Token take no `reason` (the docs list the header only for
  the bot-authenticated variants).
- `WebhookQuery` (crate-private) writes `wait`, `thread_id` and `with_components`; the
  interaction edit methods send the `EditWebhookMessage` query fields too.
- `execute_webhook` sends no `wait` (Discord's default, false) and discards the response.
- Webhook routes with a token use webhook ID + token as the major parameter (like the
  interaction webhook routes); without a token, the webhook ID.
- `Route::authenticated()` is derived from the route's parameters: any `*_token` parameter
  means the path authenticates the request (D-121); an invalid client token is then not an
  error either.

### C-051 — How the discord.js showcase measures (D-123)
- @discordjs/ws is pinned to 1.2.3 and @discordjs/rest to 2.6.3: the versions discord.js
  14.28.1 itself depends on, so npm installs one copy of each (the @discordjs/ws 2.x line
  belongs to discord.js 15).
- One generator (`benches/vs-node/src/payloads.rs`) produces every payload, with IDs unique
  per guild; the mock gateway and mock API are one Rust process on CPUs 4-11, clients run on
  CPUs 0-3 and decode loops on CPU 2 (the laptop's performance cores are 0-3).
- CPU time is `utime + stime` of `/proc/self/stat` (`USER_HZ` = 100), memory `VmRSS` /
  `VmHWM` of `/proc/self/status`, read the same way in Rust and Node.js. Decode: 1 s warm-up,
  15 rounds of ≈ 200 ms, median. Flood: the window runs from the first to the last counted
  event. REST: 64 workers, one channel each, 300 000 requests after a 30 000-request warm-up,
  the proactive global limit lifted in both, rate-limit headers that never run out. Three runs
  per configuration; the median run is shown.
- Workloads: 1 000 000 `MESSAGE_CREATE` (≈ 0.9 KiB) after one 1000-member guild; memory with
  0, 100 and 500 guilds of 1000 members, 20 channels, 10 roles and 20 emojis (no presences).
  The gateway is uncompressed for both.
- The Rust memory runner uses a djs-gateway `Shard` (the `Client` does not surface
  `GUILD_CREATE` yet) and keeps `Box<AvailableGuild>` per guild in a `HashMap`; discord.js is
  measured after `gc()` (`--expose-gc`). The discord.js decode runner calls the exported
  `Guild` / `Message` constructors that its handlers use and empties the client-wide user
  and channel caches after each `GUILD_CREATE`.
- Variants shown besides the defaults: the mimalloc binary, `HandlerMode::Sequential`, the
  gateway layers alone (djs-gateway `Shard` vs @discordjs/ws `WebSocketManager`), discord.js
  without its message cache, and `JSON.parse` alone as the Node.js floor.
- Charts are light/dark SVG pairs embedded with `<picture>`; blue is djs-rs and orange Node.js
  (the dataviz reference palette, validated for colour-vision deficiencies).
- The harness crate is a workspace member so the quality gate keeps it compiling.

### C-052 — Emoji endpoint details
- `list_application_emojis` unwraps Discord's `{"items": [...]}` into `Vec<Emoji>` (like
  the permission endpoints, D-110) through a private generic `Items<T>` in
  `djs-rest/src/endpoints/mod.rs`, reused by later list endpoints with the same wrapper.
- Guild emoji routes use the guild ID as the major parameter (Discord limits them per
  guild); application emoji routes have none.
- `ModifyApplicationEmoji` follows the other modify bodies (`Default`, optional `name`) even
  though the name is its only field; `ModifyGuildEmoji::roles` is a `Patch`, whose `Clear`
  (`null`) makes the emoji usable by everyone.

### C-053 — Sticker endpoint details
- `CreateGuildSticker` is not `Serialize`: Discord takes its parameters as
  `multipart/form-data` fields. `create_guild_sticker` sends the file as `file`, then `name`,
  `description` and `tags` as text fields (the order @discordjs/rest uses: files first). The
  description is always sent, empty by default, like discord.js does.
- `CreateGuildSticker::new(name, tags, file)` takes the tags because Discord requires them.
- `list_sticker_packs` unwraps `{"sticker_packs": [...]}` (as C-052) through a private
  struct in `endpoints/sticker.rs`; the field name differs from `Items<T>`.
- Guild sticker routes use the guild ID as the major parameter; Get Sticker and the sticker
  pack routes have none.

### C-054 — Soundboard endpoint details
- The sound bodies keep `emoji_id` and `emoji_name` as two fields, like `SoundboardSound`
  itself, instead of one emoji enum.
- `send_soundboard_sound` discards the response (Discord does not document one) and uses the
  channel ID as the major parameter; guild sound routes use the guild ID; the default sounds
  route has none. `list_guild_soundboard_sounds` unwraps `{"items": [...]}` with `Items<T>`
  (C-052).
- Bodies with a `volume: f64` derive `PartialEq` only (no `Eq`/`Hash`).

### C-055 — Guild scheduled event endpoint details
- Method names follow the Discord docs (`list_scheduled_events_for_guild`, D-091); the route
  parameter is `event_id`; the `with_user_count` flags are query structs, like `GetGuild`.
- Delete Guild Scheduled Event takes no `reason`: the docs do not list the audit log header
  for it and discord.js sends none.
- `RecurrenceRuleData`'s lists are `Vec`s skipped when empty (discord.js omits them too).
- Model fix: `GuildScheduledEventUser::member` is a `PartialMember`, because Discord sends the
  subscriber's member without its user (discord.js merges `user` into it). `EntityMetadata`
  and `RecurrenceNWeekday` got constructors, as `#[non_exhaustive]` types used in bodies
  need them (C-048).

### C-056 — Stage instance endpoint details
- The stage instance routes have no major parameter: the channel ID does not lead their path
  (`/stage-instances/{channel_id}`), and Discord's major parameters only lead a path, as
  discord.js also assumes.
- The bodies keep the optional `privacy_level` (discord.js does too) and document that
  `Public` is deprecated.

### C-057 — Invite endpoint details
- Invite codes are `&str` route parameters, not major parameters; the methods take the code
  only (not a `discord.gg/…` URL).
- `get_target_users` reads the raw body through `Rest::execute` and skips an optional
  `user_id` header, blank lines and surrounding whitespace; the error of a bad line names
  the line number.
- `FileUpload::target_users` writes IDs separated by `\n` without a header, like discord.js,
  as `users.csv` with `text/csv`.
- One `BulkTargetUsers` body serves Bulk-Add and Bulk-Delete Target Users. Update Target
  Users discards its (undocumented) response; the other target user endpoints return 204.
- The job status type is `invite::InviteTargetUsersJobStatus` with the code enum
  `InviteTargetUsersJobStatusCode` (discord-api-types names).

### C-058 — Guild template endpoint details
- Template codes are `&str` route parameters; Get Guild Template
  (`/guilds/templates/{code}`) has no major parameter, the others use the guild ID.
- None of the template endpoints takes an audit log `reason` (the docs list none).
- Create Guild from Guild Template is not modelled: it is no longer in Discord's docs (bots
  cannot create guilds).

### C-059 — Auto Moderation endpoint details
- `CreateAutoModerationRule::trigger_metadata` is a plain `TriggerMetadata` (so
  `body.trigger_metadata.allow_list = ...` works) and is left out of the request when it
  equals the default (`Spam` rules have no metadata).
- Constructors take `impl IntoIterator` for keywords, presets and actions, so arrays and
  `Vec`s both work.
- `ModifyAutoModerationRule` uses `Option` for every field (no `Patch`): none of them is
  nullable; an empty list clears exemptions.
- All five routes use the guild ID as major parameter; create, modify and delete take an
  audit log `reason`.

### C-060 — Poll endpoint details
- `get_answer_voters` takes the `answer_id` as a plain `u32` (the model's type for
  `Answer::answer_id`; it is not a snowflake), and routes got a `Param` impl for `u32`.
- Get Answer Voters returns `Vec<User>`, unwrapped from Discord's `{"users": [...]}` (as
  C-052/C-053 for other wrapped lists).
- Both poll routes use the channel ID as major parameter; neither takes an audit log reason.

### C-061 — Monetization endpoint details
- Endpoints are split by resource page: `endpoints/{sku,entitlement,subscription}.rs`, with
  one test file, `tests/monetization.rs`.
- The routes have no major parameter (`/applications/{id}` and `/skus/{id}` are not major
  parameters in Discord's rate limits).
- `ListEntitlements::sku_ids` is a `Vec<SkuId>` sent comma-separated (`sku_ids=1,2`), left
  out when empty. The other list filters are `Option`s.
- Create Test Entitlement's partial response decodes into `Entitlement` (its `starts_at` and
  `ends_at` are already `Option`s); `subscription_id` and `promotion_id`, which appear only in
  Discord's examples, are still not modelled.

### C-062 — Application endpoint details
- `EditCurrentApplication` uses `Patch` only for `icon` and `cover_image` (the only fields
  Discord documents as nullable); everything else is an `Option`.
- Activity instance IDs (`i-…`) are `&str` route parameters; the application routes have no
  major parameter.
- `InstallParams` got a `new(scopes, permissions)` constructor so it can be used in the body
  (C-048).

### C-063 — OAuth2 details
- The token routes (`OAuth2TokenExchange`, `OAuth2TokenRevocation`) use the versioned base
  (`/api/v10/oauth2/token`, as in Discord's examples) and the normal rate limits, counted
  globally like every other route; `Route::authenticated` lists them explicitly next to the
  `*_token` parameter rule.
- `AccessTokenResponse` keeps `scope` as Discord's space-separated string, with a `scopes()`
  iterator parsing it; `access_token` and `refresh_token` are `String`s (the `.expose()` in
  the D-132 question's preview was only illustrative — like other tokens, they are plain
  fields with a redacting `Debug`, C-011). `guild` and `webhook` are boxed.
- `AuthorizeUrl` writes its parameters in a fixed order (`response_type`, `client_id`,
  `scope`, `state`, `redirect_uri`, `prompt`, `permissions`, `guild_id`,
  `disable_guild_select`, `integration_type`), scopes joined with `%20`; it has no implicit
  grant constructor (Discord's docs discourage it). `Prompt::{Consent, None}` mirrors
  Discord's values.
- `revoke_token` ignores the response body (Discord documents none).
- Found while adding secrets to request bodies: `Request` derived `Debug`, which printed the
  JSON body — including access tokens of `AddGuildMember` / `GroupDmAddRecipient`. It now
  prints only the body size and form part names.
- The fixture of the bot authorization token response gets role `colors` and string
  `permissions` (the docs' example predates both).

### C-064 — Lobby endpoint details
- Member and join metadata are `Option<LobbyMetadata>` (`Some` of an empty map clears),
  while `CreateLobby::metadata` and `SendLobbyMessage::metadata` are plain maps skipped when
  empty (nothing to keep there). `LobbyMetadata` is an alias of
  `BTreeMap<CompactString, CompactString>`, like the model's metadata fields.
- Link and Unlink Channel share one path and method (`PATCH …/channel-linking`) but are two
  routes and two methods; unlinking sends `{}`. The body type is private.
- `BulkLobbyMember` flattens `LobbyMemberData` and adds `remove_member` (skipped when
  `false`).
- The lobby routes have no major parameter. Methods that need a player's access token say so
  in their docs; the client does not check the token kind.
- `CreateOrJoinLobby` redacts its `secret` in `Debug` (anyone who knows it can join).

### C-065 — Application identity profile endpoints and coverage check
- Identity profiles were not in D-102's list but are v10 REST endpoints (ROADMAP Phase 3:
  every endpoint) of a resource modelled since D-072, so they came last, with Get Gateway.
- External IDs (`provider_type`, `provider_issued_user_id`) are `&str` route parameters; the
  identity lists are unwrapped from `{"identities": [...]}` (as C-052); Delete Application
  Identity always sends its optional body (`{}` when empty); the routes have no major
  parameter.
- `scripts/route_coverage.py` compares the `<Route>` tags of the docs with `route.rs` by
  method and parameter positions.

### C-066 — CDN URL builder details
- An explicit `Webp` with an animated hash also adds `?animated=true`; `Png`/`Jpeg` give a
  still image, `Gif` an animated GIF (for animated hashes). Query order: `animated`, `size`.
- Emojis have no hash, so `Cdn::emoji` takes `animated: bool`; stickers take their
  `StickerFormatType` and pick PNG / Lottie JSON / GIF (media proxy) from it; endpoints of
  a fixed format (default avatars, avatar decorations, stickers) take no `ImageOptions`.
- Methods do not reject formats an endpoint does not list (e.g. GIF for splashes); their docs
  list the supported ones, like discord.js.
- Added beyond Discord's table, as discord.js has them: group DM `channel_icon`,
  `soundboard_sound` (from the soundboard page). Achievement icons and store page assets,
  which discord.js lacks, are included.
- `ImageFormat` and `ImageSize` are `#[non_exhaustive]`; `ImageSize::from_pixels` converts a
  number.

### C-067 — Gateway send limit details (D-142)
- A **sliding** window over the send times of the last commands (`send_limit.rs`), not a
  counter reset every minute like @discordjs/ws: it never allows more than the limit in any
  span, however Discord aligns its own window.
- **115 commands per 61 s**: Discord's 120 per 60 s minus 5 payloads of room for heartbeats
  (≤ 2 per minute at the usual 41.25 s interval), heartbeats Discord asks for and identify or
  resume, which bypass the limit; plus one second of margin for network jitter.
- One command at a time is taken from the queue and held while the window is full; the
  queue waits behind it, so order is kept. A new connection clears the window and releases
  a held command as soon as its session is active.
- The mock-gateway tests use real time and check the hold and the reset; the window's
  refilling is unit-tested. Tokio's paused clock cannot be used with real sockets: an I/O
  wake-up during its zero-timeout park does not stop the clock from jumping ahead (tokio
  1.54), which fired heartbeats and timeouts too early.

### C-068 — Sharding details (D-144–D-147)
- `LocalIdentifyQueue` keeps 5 100 ms between identifies of a bucket (Discord's 5 s plus
  100 ms for jitter, the previous per-shard interval); a tokio `Mutex` per bucket serves
  waiting shards in order, and a wait dropped before its turn takes no turn. A
  `max_concurrency` of `0` is treated as `1`. `LocalIdentifyQueue` is also `Clone` (shared
  inside) and `Default` (concurrency 1).
- `Shard` stays `Send + Sync`: its queue turn (a `Send`-only future) is kept in a
  `std::sync::Mutex` that is never locked (polled through `get_mut`); a compile-time
  assertion guards `Shard`, `MessageSender` and `Config`.
- `create_shards` panics on a zero total or a range past it (`#[track_caller]`) and returns
  `impl ExactSizeIterator + DoubleEndedIterator`.
- djs-rs: Get Gateway Bot is skipped only when both `gateway_url` and a fixed shard count are
  set (shards then identify one at a time); `Shards::Auto` always asks Discord. Tests that
  only configure a gateway URL use `Shards::Count(1)`. `ClientBuilder::shards` panics on a
  zero count or an empty or out-of-range range.
- The D-147 error is `Error::SessionStartLimit { required, remaining, reset_after }`.
- One tokio task per shard (in a `JoinSet`); the first fatal error shuts the other shards down
  and is returned; a panicking shard task is re-raised by `login`.

### C-069 — Presence details (D-148, D-149)
- `Activity::{playing, streaming, listening, watching, competing, custom}` were added because
  the approved D-148/D-149 examples use them; `custom(text)` sets `state` and the name
  `"Custom Status"` (what Discord's clients send for type 4).
- `MessageSender::update_presence` stores the presence as soon as the command is queued, so
  an IDENTIFY that happens before the queued update goes out already carries it.
- `Client::set_presence` updates every shard even if one fails and returns the first error;
  it does nothing while the client is not running (the initial presence is
  `ClientBuilder::presence`). The running shards' senders live in `Inner::senders`, cleared
  when `login` returns.

### C-070 — zlib-stream details (D-140)
- `Compression::default()` is `ZstdStream` with the `zstd-stream` feature, otherwise
  `ZlibStream` with `zlib-stream`, otherwise `None`.
- Following Discord's docs, messages are buffered until one ends with the `Z_SYNC_FLUSH`
  suffix `00 00 ff ff`; a message that already ends with it while nothing is buffered is
  inflated directly (the usual case). `Decompressor::decompress` returns `None` for the start
  of a split payload. Buffers above 1 MiB are released after use, like zstd's.
- A payload that does not inflate completely, or a stream end (Discord never ends it), is a
  `Decompress` error, which reconnects like other decompression errors.
- `flate2 = { version = "1.1.11", default-features = false }` in the workspace, with the
  `zlib-rs` feature in djs-gateway; djs-rs forwards a `zlib-stream` feature.

### C-071 — Stress-test compression matches Discord's parameters
- The mock gateways compress like the real gateway, whose first frames (read once from
  `wss://gateway.discord.gg` with a throwaway program, no token sent, 2026-11-12) show a zstd
  frame with a 64 KiB window (`windowLog` 16) and a zlib header with a 32 KiB window at the
  best level (`78 da`). The stress tests use zstd level 3 with `windowLog` 16 and zlib
  `Compression::best()`; Discord's zstd level is not visible in the header.
- Memory per shard is measured with each test in its own process: running them one after
  another in one process carries the allocator's retained memory into the next baseline.

### C-072 — A/B performance runs pin both builds to the same cores
- On the development laptop (i7-1355U: CPU 0–3 = 2 P-cores with HT at 5 GHz, CPU 4–11 =
  8 E-cores at 3.7 GHz), unpinned stress runs land on P-cores or E-cores by chance, which
  moved single-shard throughput by up to 40 % between identical runs (`perf`, Phase 4).
- Comparisons between commits therefore run both builds alternately, pinned with `taskset`
  to the same cores (`0,2`: two P-cores without HT siblings, or `0-3`, or `4-11`), and also
  compare `perf stat` instruction counts, which do not depend on placement. Absolute
  numbers in `docs/PERFORMANCE.md` stay unpinned unless a section says otherwise.

### C-073 — Cache core details (Phase 5, step 1)
- `dashmap` 6.2.1, the latest stable release (7.0 is a release candidate).
- The cache enforces every limit itself, on top of the stores (`limits.rs`: `Global` for
  limits over the whole cache, `Scoped` with the guild/channel → keys index for the others);
  `Backend::store` receives the limit for information only, so custom stores never evict.
  Disabled resources get no store.
- The insertion order keeps a sequence number with each key, so a key removed and inserted
  again counts as new (exact FIFO). Removed keys stay in the queue until popped or until the
  queue is compacted (when stale entries exceed live ones + 32), so removal is amortised
  O(1). Cost per entry of a limited resource: the key and a `u64` in the order's map plus the
  same in its queue.
- A limited resource over the whole cache takes one mutex around each insert and removal,
  which keeps the order exact; unlimited resources take no extra lock. `Scoped` changes hold
  the parent's index entry around the store call (lock order: index → store).
- `Limit::Max(0)` is stored as `Limit::Disabled`. The current user is always kept and is not
  a `Resource`. Threads, scheduled events, stage instances and soundboard sounds count per
  guild (D-159).
- Public helpers beyond the approved sketch: `Store::{is_empty, contains_key}` (default
  methods), `Resource::ALL`, `Limit::{is_enabled, max}`, `CacheConfig::{limit, is_enabled}`,
  `CacheConfigBuilder::limit` plus one method per resource, `From<CacheConfig> for
  CacheConfigBuilder`, `Cache::{new, with_config, with_backend, config, backend}`,
  `DashMapBackend::new` (a non-exhaustive unit struct, so fields can be added later).

### C-074 — Cache event semantics (Phase 5, step 2)
- `GUILD_CREATE` replaces the guild's roles, emojis and stickers and adds its channels and
  members (Discord sends only some members). `GUILD_UPDATE` replaces roles, emojis and
  stickers only when its list is non-empty: the model cannot tell an omitted list from an
  empty one, and the resources' own events cover removals.
- `GUILD_DELETE` with `unavailable` (an outage) keeps the cached data and lists the guild in
  `Cache::unavailable_guilds`, returning no previous state; otherwise the guild, its channels,
  roles, emojis, stickers and members are removed (users stay, like in discord.js). A guild
  evicted by a guild limit takes its parts along too. `READY` lists all guilds as unavailable
  until their `GUILD_CREATE`.
- `GUILD_MEMBER_UPDATE` of an uncached member adds it (discord.js does too); fields it omits
  (`deaf`, `mute`, `pending`, `flags`, `joined_at`) are kept from the cached member or
  default. `GUILD_MEMBER_ADD` of a new member and `GUILD_MEMBER_REMOVE` adjust the cached
  guild's `member_count` (copy-on-write of the guild).
- Users seen in member events are cached when users are enabled, also without members. An
  incoming user equal to the cached one reuses its `Arc`, so members of many guilds share
  one user; users are only removed by their limit (sweepers come in step 4).
- Guild channels from `GUILD_CREATE` get their `guild_id` filled in. Threads in channel
  events are skipped: they are `Resource::Thread` (step 3).
- Guild resources are stored under `(guild_id, id)`, so `cache.roles(guild).get(id)` is one
  exact lookup; channels count over the whole cache and have a separate guild → channels
  index (`CacheView::in_guild`).
- The previous state of `GUILD_CREATE` / `GUILD_DELETE` is the guild only, not its parts;
  `GUILD_MEMBERS_CHUNK` returns the members it replaced.
- `Store` gained a required `keys` method (for the views' `keys()`). `UpdateCache` is sealed
  and its method hidden. Public names added: `CacheView`, `GuildView`, `CacheView::in_guild`,
  `Cache::unavailable_guilds`, `CachedMember::{id, display_name}`, the `Previous` variants.

### C-075 — Cache event semantics (Phase 5, step 3)
- Messages: `MESSAGE_UPDATE` replaces only a cached message (like discord.js); its author is
  cached either way. Webhook authors are not cached as users (discord.js does the same). The
  `member` of `MESSAGE_CREATE`, of `MESSAGE_REACTION_ADD` and of a voice state is cached as a
  member. Deleting a channel, thread or guild drops its messages; a guild → channels-with-
  messages index makes that work when the channel resource is off. A channel's
  `last_message_id` is not updated on `MESSAGE_CREATE` (it would copy the channel on the
  hottest event).
- Reactions (D-166): matched by ID for custom emojis (names may be missing) and by the emoji
  for Unicode ones; `me` / `me_burst` follow the current user; a reaction that drops to zero
  is removed. The changed message keeps its place in the channel's insertion order.
  `djs_model::message::Reaction::new(emoji)` was added: model types are non-exhaustive, so the
  cache could not build a new reaction otherwise.
- Presences: an `offline` presence removes the cached one (no presence = offline or unknown),
  which keeps memory to online members; discord.js keeps offline presence objects. The
  partial user of a presence is not applied to the user resource (member updates carry user
  changes).
- Voice states are the model type with `member` cleared and `guild_id` set; a state without a
  channel (the user left voice) removes it.
- Threads are the model `Channel`, per guild. `THREAD_LIST_SYNC` replaces the active
  (unarchived) threads of the synced parents, or of the whole guild without `channel_ids`.
  Thread members (`THREAD_MEMBER(S)_UPDATE`), scheduled event user counts and poll votes are
  not tracked.
- `GUILD_SOUNDBOARD_SOUNDS_UPDATE` updates the sounds it lists; `SOUNDBOARD_SOUNDS` (the
  answer to a request) replaces the guild's sounds.

### C-076 — Cache sweeping details (Phase 5, step 4)
- Filters return `true` for what to sweep (like discord.js). Resources kept per guild or per
  channel pass the guild or channel to the filter (`FnMut(GuildId, &Role) -> bool`), since
  some values (roles, emojis) do not carry it; resources over the whole cache pass the value
  only.
- Sweeping a guild drops its parts and its unavailable mark; sweeping a channel or thread
  drops its messages, as deleting them does. Users are not part of a guild.
- A limited resource holds its insertion order during the sweep and forgets the swept keys,
  so they free their room; per-guild/channel sweeps hold each parent's index entry while
  sweeping it.
- Message age is the last edit, else the creation time (discord.js compares the edit time
  first); a thread's age is its `archive_timestamp`, and only archived threads are swept by
  the preset. "Now" is the system clock (`Timestamp::now`).
- Public names: `Sweep { interval, lifetime }` (`Sweep::new`), `SweepConfig { messages,
  threads }`, `CacheConfig::sweepers`, `CacheConfigBuilder::{sweep_messages, sweep_threads}`,
  `Cache::sweep_{guilds, channels, users, threads, messages, roles, emojis, stickers, members,
  presences, voice_states, scheduled_events, stage_instances, soundboard_sounds}`,
  `Cache::{sweep_archived_threads, sweep_messages_older_than}`. `CacheConfig`'s `Debug` now
  prints `CacheConfig { limits: {…}, sweepers: … }`.

### C-077 — Client cache details (Phase 5, step 5)
- `ClientBuilder::cache_backend(backend)` also enables the cache, with
  `CacheConfig::default()` unless `ClientBuilder::cache` is called too (in either order):
  choosing a backend is a request for a cache, and a silently unused backend would be a trap.
- The previous state returned by `Cache::update_dispatch` is dropped until Phase 8 (D-171).
  The update runs on the shard's task, before subscribers (`client.events()`) and handlers
  see the event.
- Sweeps: `login` spawns one tokio task per configured `Sweep` after planning the shards; the
  first run is one interval after the start, missed ticks are delayed
  (`MissedTickBehavior::Delay`), and the sweep runs on a runtime worker (no `spawn_blocking`;
  discord.js sweeps on its only thread). The tasks are stopped and awaited before `login`
  returns. A zero interval disables the sweep with a `warn` log (it would loop without
  pause); an interval too long to schedule never runs. Swept counts are logged at `debug`.
- The cache belongs to the client, not to a session: `shutdown` does not clear it, and a
  later `login` refills it from `READY` / `GUILD_CREATE` (entries removed while offline stay
  until swept or deleted).
- `djs_cache` is re-exported as `djs_rs::cache` (like `model`, `rest`, `gateway`); the prelude
  is unchanged. `Client`'s `Debug` shows the cache. djs-rs enables tokio's `time` feature for
  the sweep intervals (already used by djs-gateway).

### C-078 — Updates of existing keys take the index lock shared (Phase 5, step 6)
- `Scoped::insert` checks the parent's index entry under a shared lock first: when the key
  is already there (an update), it writes the store while holding that shared lock and
  returns; only new keys take the entry exclusively. `Index::insert` likewise returns early
  when the key is listed. Re-inserting a key never moves it in the FIFO order (D-157), so
  the semantics are unchanged. Removals and sweeps keep the exclusive lock, so the index and
  the store stay consistent; the lock order (index → store) is unchanged. Measured in
  `docs/PERFORMANCE.md` (Phase 5): parallel writers scale, single-threaded updates got
  faster.

### C-079 — Typing, interaction and thread-member details (D-173)
- Resolved members carry neither `deaf` nor `mute`: a cached member keeps its own when a
  resolved one replaces it. The interaction's own member (with both) replaces it as is.
- A resolved user without a resolved member (or any user in a DM) is cached as a user only.
  Typing in a DM carries no user object, so it caches nothing.
- `THREAD_MEMBERS_UPDATE` caches the added members' guild members even when the thread is
  not cached; the presences sent with them are not cached (not part of D-173), nor are thread
  members themselves (no thread-member resource).
- Previous state: `TypingStart` and `InteractionCreate` return the member they replaced
  (`Previous::Member` through `update_dispatch`), `ThreadMembersUpdate` the thread before.

### C-080 — Compact cached types (D-177)
- Rare user fields (behind `CachedUser`'s box): `banner`, `accent_color`,
  `avatar_decoration_data`, `collectibles`, `primary_guild`. Inline: `id`, `username`,
  `discriminator`, `global_name`, `avatar`, `bot`, `system`, `public_flags`.
- Rare member fields (behind `CachedMember`'s box): `avatar`, `banner`, `premium_since`,
  `communication_disabled_until`, `avatar_decoration_data`, `collectibles`. Inline:
  `guild_id`, `user`, `nick`, `roles`, `joined_at`, `deaf`, `mute`, `pending`, `flags`.
- The box holds its fields unboxed (no box in a box) and exists only when one of them is
  set; accessors are named like the fields. `CachedUser` has `display_name` and `tag` like
  `User`. Whether a user seen again is the cached one is decided by comparing every field
  (crate-internal `CachedUser::is`), as before with `User`'s `==`.
- Sizes (x86_64): `CachedMember` 176 → 96 B, `CachedUser` 104 B (`User`: 144 B); guarded in
  `tests/stress.rs` (`type_sizes`).

### C-081 — `EmbedBuilder` details (D-186–D-191)
- **Model constructors:** the embed types are `#[non_exhaustive]`, so `djs-model` gained
  `EmbedField::new(name, value, inline)`, `EmbedAuthor::new(name)`, `EmbedFooter::new(text)`
  and `EmbedImage::new(url)` (no validation in the model).
- **`ValidationError`:** `field` is a `Cow<'static, str>` path written like Discord's error
  paths (`fields[3].name`, `author.url`), formatted only when an error occurs;
  `ValidationError::field()` returns it. Variants: `TooLong`, `TooMany`, `Missing` (for the
  command builders), `OutOfRange { min, max, actual: i64 }`, `InvalidUrl { allowed }` (the
  URL itself is not stored, so it cannot leak into logs), `Empty { needs }` (the hint shown in
  the message). Hand-written `Display` / `Error` (no `thiserror` in this crate).
- **Pieces:** `EmbedAuthorBuilder` / `EmbedFooterBuilder` also have `build()` (paths without
  the `author.` / `footer.` prefix) and `build_unchecked()`, following D-180's "every
  builder"; they convert from `&str`, `&String`, `String`, `CompactString` and the model
  type. `fields(iter)` takes `EmbedField`s.
- **Order of checks:** title, description, URL, color, fields (count, then each name and
  value), footer, image, thumbnail, author, the 6000 total (`field: "embed"`), content.
- **Not tried live, decided by analogy:** an author with a blank name does not count as
  content (a blank footer does not); URLs are checked as given, not trimmed; the 6000 total
  counts trimmed code points like the individual limits.
- **Unchanged parts:** `build()` does not touch `type`, so an embed taken from a received
  message keeps its type; setters cannot remove a part (see the open question).
- **Dependencies:** `djs-builders` uses the existing workspace crates `djs-model` and
  `compact_str` (`std`), and `serde_json` as a dev-dependency.

---

## Open questions (to ask the owner when relevant)

- **Removing parts with the builders**: discord.js can clear a part (`setTitle(null)`) and
  replace or splice fields (`setFields`, `spliceFields`). `EmbedBuilder` only sets and adds;
  clearing (for example `clear_footer()` or `Option` setters) would matter when editing a
  received embed.

- **Resolved roles, messages and channels of interactions**: discord.js also caches the
  resolved roles (full objects), messages (full objects) and channels (partial) of an
  interaction. D-173 covers users and members only; roles and messages could follow, partial
  channels would replace full ones.

- **A cache backend with lock-free reads** (deferred, D-176): with more busy threads than CPUs, DashMap's
  reader-preferring `RwLock` and lock-holder preemption starve writers (2 writers + 8 readers
  on 4 CPUs: 0.04–0.11 M updates/s, `docs/PERFORMANCE.md` Phase 5). A second `Backend` on a
  lock-free map (e.g. `papaya` or `scc`) would avoid it; it needs a new dependency.
- **Cache memory per member**: 546 B with discord.js' defaults (fresh process); being
  reduced with compact cached types (D-177).

- **Identify coordination across processes in `Client`**: with `Shards::Range`, every process
  has its own `LocalIdentifyQueue`. A `ClientBuilder::identify_queue(Arc<dyn IdentifyQueue>)`
  (or a shared-store queue) would let processes take turns; not added without a decision.
- **Resharding** (ROADMAP Phase 4, "In-process sharding manager"): restarting with a new
  recommended shard count while running, as discord.js can; not started.

- **`OAuth2` error bodies**: the token endpoints answer errors with
  `{"error": "invalid_grant", "error_description": "…"}`, which is not Discord's usual error
  shape, so `ErrorKind::Response` has `error: None` for them. Options: parse them into a new
  `OAuth2Error` (a variant of the error kind or a field of `ApiError`); keep the raw body on
  the error. Also open: PKCE (`code_verifier` / `code_challenge`), which Discord's docs
  recommend but do not document; it would need SHA-256 and random bytes (new dependencies).
- **Cost of `HandlerMode::Concurrent` under event floods** (found by the showcase, D-123):
  spawning a tokio task per handler call wakes idle worker threads, so the default client
  uses ≈ 6.7 µs CPU per `MESSAGE_CREATE` vs 1.8 µs with `HandlerMode::Sequential`
  (`docs/VS_NODE.md`, section 2). Options: keep it; run the handler calls of the events read
  in one poll in one task; make `Sequential` the default; a per-shard dispatcher. Ask before
  the next change to the `Client` (Phase 4/5).

- **Guide/book**: mdBook hosted on GitHub Pages?
