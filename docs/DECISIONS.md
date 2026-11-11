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

---

## Open questions (to ask the owner when relevant)

- **Guide/book**: mdBook hosted on GitHub Pages?
