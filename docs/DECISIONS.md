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

### D-043 — Strict global REST limiter (GCRA)
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

---

## Open questions (to ask the owner when relevant)

- **Guide/book**: mdBook hosted on GitHub Pages?
