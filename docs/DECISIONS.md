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

### D-196 — Options that cannot work are rejected even though Discord accepts them
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like D-190, `build()` rejects an option whose `min_value` is above its
  `max_value` (it can never be filled) and an option with both `choices` and
  `autocomplete` (forbidden by the docs; discord.js rejects it too), although the live API
  accepted both (D-195). Everything else follows what Discord rejected in D-195.
- **Alternatives:** reject only choices with autocomplete; accept both, exactly like Discord.

### D-197 — Buttons: one constructor per style
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ButtonBuilder::primary(custom_id)`, `::secondary`, `::success`, `::danger`,
  `::link(url)` and `::premium(sku_id)`, then `.label()`, `.emoji()`, `.disabled()`; a link
  button cannot get a `custom_id` and no required field can be forgotten. `build()` returns a
  `Component`.
- **Alternatives:** discord.js-like `ButtonBuilder::new().style(…).custom_id(…)` with
  `Missing` errors; `ButtonBuilder::new(style, custom_id)` plus `link` / `premium`.

### D-198 — What a row or container may hold is checked in `build()`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like D-193, one `ActionRowBuilder` (and container / section builder) takes
  `.component(impl Into<Component>)`; a wrong mix (six buttons, a button with a select menu,
  a text display in a row) is a `ValidationError` from `build()`.
- **Alternatives:** typed constructors and marker traits (`ActionRowBuilder::buttons`,
  `::select`), compile-time errors.

### D-199 — Message-wide component limits: a function for the list
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like D-189, a public `validate_components(&[Component], …)` checks what only
  the whole list shows — at most 5 action rows for a legacy message; at most 40 components and
  4000 characters of text for a Components V2 message; unique `custom_id`s — for the Phase 8
  message options. Each builder checks only itself.
- **Alternatives:** no list check (Discord reports those limits).

### D-200 — Unclear component rules are checked on the live API with messages
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** about 20 messages with components sent by the test bot to the test channel
  and deleted at once, like D-188: how `custom_id`s and labels are counted (code points?
  trimmed?), blank labels, a button without label and emoji, repeated `custom_id`s,
  `min_values` above `max_values`, link URL schemes, how the 40 components and 4000 characters
  of Components V2 are counted. Modals cannot be sent without a user's interaction, so their
  rules come from the docs only. The token stays in `.env`.
- **Live result, 2026-11-12** (20 messages; the 5 accepted ones deleted at once):
  - **counting:** a `custom_id` of 100 × 🔥 accepted, 101 letters rejected
    (`BASE_TYPE_BAD_LENGTH`, 1-100); a whitespace-only `custom_id` accepted (not trimmed, not
    blank-checked); a button label of 80 × 🔥 accepted (code points);
  - **buttons:** a whitespace-only label and a button without label or emoji both rejected as
    `label: BASE_TYPE_REQUIRED`; a link URL must be `http`, `https` or `discord`
    (`ftp://` rejected with "Scheme must be one of ('http', 'https', 'discord')",
    `discord://-/channels/@me` accepted);
  - **uniqueness:** a repeated `custom_id` rejected within a row and across rows
    (`COMPONENT_CUSTOM_ID_DUPLICATED`); repeated select option values rejected
    (`COMPONENT_OPTION_VALUE_DUPLICATED`);
  - **select menus:** `min_values` 3 with `max_values` 2 rejected (`min_values` ≤ `max_values`);
    `max_values` 5 with 3 options rejected (at least `max_values` options); a whitespace-only
    option label rejected (`BASE_TYPE_BAD_LENGTH`, 1-100);
  - **rows:** an empty action row rejected (1-5 components);
  - **Components V2:** a container with 39 text displays (40 components counting the
    container) accepted, with 40 rejected (`COMPONENT_MAX_TOTAL_COMPONENTS_EXCEEDED`: nested
    components count); text displays of 2000 + 2001 letters rejected
    (`COMPONENT_DISPLAYABLE_TEXT_SIZE_EXCEEDED`, 4000 across all components); a
    whitespace-only text display rejected (1-4000 after trimming); one text display of
    4000 × 🔥 answered `413 Payload Too Large` (`40005`), so the unit of the 4000 limit is
    still open.
- **Alternatives:** the docs only, unclear cases documented as such.

### D-201 — A four-message live test settles the unit of the 4000-character V2 limit
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** four Components V2 messages with one text display each, sent by the test bot
  and deleted at once if accepted: 4000 letters, 2000 × 🔥, 2001 × 🔥 (2001 code points, 4002
  UTF-16 units, 8004 bytes) and 3000 × € (3000 code points and UTF-16 units, 9000 bytes) —
  which separates code points, UTF-16 units and bytes.
- **Live result, 2026-11-12:** all four were accepted (and deleted): 2001 × 🔥 (4002 UTF-16
  units, 8004 bytes) and 3000 × € (9000 bytes) included. The 4000 limit counts **code
  points**, like the others (D-183). The `413 Payload Too Large` for 4000 × 🔥 (16 000 bytes)
  is a separate byte limit somewhere above 9000 bytes of text; it is documented, not checked.
- **Alternatives:** count code points without a test (D-183) and document the 413; count
  UTF-16 units for this limit to be safe.

### D-202 — Premium buttons have their own builder type
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ButtonBuilder::premium(sku_id)` returns a `PremiumButtonBuilder` with only
  `.disabled()` and `.id()`, so a label or emoji (which Discord forbids and `PremiumButton`
  cannot hold) is a compile-time error instead of being lost when the button goes into a row.
- **Alternatives:** one `ButtonBuilder` with the error in its `build()` (lost inside a row);
  rows and containers keeping unbuilt builders.

### D-203 — Select menu options: a shortcut plus a builder
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `StringSelectMenuBuilder::option(label, value)` for a plain option,
  `.option_with(StringSelectMenuOptionBuilder::new(label, value).description(…).emoji(…)
  .default(true))` for a full one, `.options(iter)` for several (builders or `SelectOption`s).
- **Alternatives:** the option builder only; one `.option(impl Into<…>)` taking a
  `(label, value)` tuple or a builder.

### D-204 — Components V2 builders: the accessory in `new()`, text and item shortcuts
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `SectionBuilder::new(accessory)` (a button or a thumbnail; it cannot be
  forgotten); `.text("…")` on sections and containers as a shortcut for a
  `TextDisplayBuilder`, `.component(…)` for everything else (D-198); media galleries follow
  D-203: `.item(url)` plus `.item_with(MediaGalleryItemBuilder::new(url).description(…)
  .spoiler(true))`. The other builders are `ContainerBuilder`, `TextDisplayBuilder`,
  `ThumbnailBuilder`, `MediaGalleryBuilder`, `FileBuilder` and `SeparatorBuilder`.
- **Alternatives:** discord.js-like `.accessory(…)` with a `Missing` error and no shortcuts;
  the accessory in `new()` without shortcuts.

### D-205 — Unclear Components V2 rules are checked on the live API
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** about ten Components V2 messages from the test bot, like D-200 (one with a
  small uploaded text file for `attachment://`), accepted ones deleted at once: an empty
  container, a section with 0 and with 4 texts, a file with an `https` URL, a thumbnail with
  an `ftp` URL, a gallery with 0 and 11 items, descriptions of 1024 × 🔥 and 1025 letters.
- **Live result, 2026-11-12** (10 messages; the 3 accepted ones deleted at once):
  - **rejected (`50035`):** an empty container (`BASE_TYPE_BAD_LENGTH`, 1-40 components), a
    section with 0 or 4 texts (1-3), a file component with an `https` URL
    (`UNFURLED_MEDIA_ITEM_MUST_USE_ATTACHMENT_PROTOCOL`: files need `attachment://`), a gallery
    with 0 or 11 items (1-10), a thumbnail description of 1025 letters (1-1024);
  - **accepted:** a file component with `attachment://probe.txt` and the uploaded file, a
    thumbnail with an `ftp://` URL (like embed images, D-188 / D-190), a thumbnail
    description of 1024 × 🔥 (code points).
- **Alternatives:** the docs only.

### D-206 — Modal builders: shortcuts as for embeds, menus and V2
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ModalBuilder::new(custom_id, title)` with `.label(text, field)` (a label
  without a description), `.component(LabelBuilder::new(text, field).description(…))` and
  `.text(…)`; `TextInputBuilder::short(custom_id)` / `::paragraph(custom_id)` (like D-197);
  `RadioGroupBuilder` / `CheckboxGroupBuilder` with `.option(label, value)` and
  `.option_with(…)` (like D-203); `FileUploadBuilder`, `CheckboxBuilder`. `build()` returns
  `ModalCallbackData`.
- **Alternatives:** no shortcuts: always `LabelBuilder`, `TextInputBuilder::new(id, style)`
  and option builders.

### D-207 — Modals are built from labels only
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ModalBuilder` takes labels and text displays; an action row in a modal (the
  deprecated way, a text input with its own label) is a `ValidationError`, and
  `TextInputBuilder` has no deprecated `.label()`. The old format stays possible with
  `build_unchecked()` and the model types.
- **Alternatives:** support both, like discord.js v14.

### D-208 — Phase 7 order: formatters, then collection; util only as needed
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `djs-formatters` first (mentions, timestamps, markdown, escaping), then
  `djs-collection`. `djs-util` only receives what other crates need (for example the shard
  ID calculation), as the ROADMAP says.
- **Alternatives:** the ROADMAP order (collection, formatters, util); collection first.

### D-209 — `Collection<K, V>` keeps insertion order with `indexmap`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Collection` is built on `indexmap::IndexMap`, so `first`, `last`, `at` and
  `key_at` follow insertion order like discord.js' `Collection` (a JS `Map`). `indexmap` is a
  new direct dependency of `djs-collection`; it is already compiled in a full djs-rs build
  (through `hyper` → `h2`).
- **Alternatives:** an own ordered map (`Vec` + `HashMap` of indexes); a `BTreeMap` ordered
  by key (creation order for snowflakes); a `HashMap` without order.

### D-210 — `Collection` API: discord.js names where they fit Rust, plus Rust traits
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** discord.js' methods in snake_case where they make sense in Rust (`first`,
  `last`, `at`, `key_at`, `find`, `find_key`, `filter` → `Collection`, `partition`, `sweep`,
  `ensure`, `has_all`, `has_any`, `map_values`, `sort`, `union`, `intersection`,
  `difference`, `random`, …) and the standard traits (`IntoIterator`, `FromIterator`,
  `Extend`, `Index`). `random` needs no dependency (randomness from std's `RandomState`).
  Methods that only repeat iterator adapters (`each`, `reduce`, `some`, `every`) are left
  out, with docs pointing to `.iter()`.
- **Alternatives:** full parity with all ~40 methods; a minimal `Deref` to the map plus a
  few extras.

### D-211 — Formatters return `Display` types and take typed IDs
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** each formatter returns a small type implementing `Display`, so nothing is
  allocated until the caller writes it (`format!`, `write!`, `.to_string()`). Mentions take
  the typed IDs of `djs-model` (`UserId`, `ChannelId`, …), so `djs-formatters` depends on
  `djs-model`. Escaping returns `Cow<str>`, borrowing when there is nothing to escape.
- **Alternatives:** functions returning `String` like discord.js; `Display` types taking any
  `impl Display` as an ID (no `djs-model` dependency, less type safety).

### D-212 — Markdown escaping: own clear rules with discord.js' options
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `escape_markdown` takes the same options with the same defaults as
  discord.js, and the single escapers escape the same sequences (`**`, `*`, `__`, `_`, `~~`,
  `||`, backticks, `\`; headings, lists and masked links on request), so ordinary text gives
  the same result. Links (`http://`, `https://`) and `<…>` tokens (mentions, emoji,
  timestamps) are left untouched by one clear rule instead of discord.js' regex lookbehinds.
  One pass without regular expressions, returning `Cow<str>` (no allocation when nothing is
  escaped). Differences from discord.js in unusual input are documented.
- **Alternatives:** reproduce discord.js' regex results exactly, quirks included; escape
  every special character.

### D-213 — Optional parts of formatters are methods on the `Display` types
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a formatter function takes what is required; optional parts are set with
  methods on the returned type, like the builders: `code_block(src).language("rust")`,
  `hyperlink(text, url).title(…)`, `time(ts).style(TimestampStyle::RelativeTime)`,
  `channel_link(channel).guild(guild)`, `message_link(channel, message).guild(guild)`,
  `slash_command_mention(name, id).subcommand(…)` / `.group(group, subcommand)`,
  `emoji(id).name(…).animated(true)`, `heading(text).level(HeadingLevel::Two)`,
  `ordered_list(items).start(3)`.
- **Alternatives:** one function per variant (`code_block_with_language`, …); `Option`
  parameters like discord.js' overloads.

### D-214 — `Collection` methods that build a new collection consume `self`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `filter`, `partition`, `map_values`, `to_sorted`, `to_reversed`, `union`,
  `intersection`, `difference`, `symmetric_difference`, `concat`, `merge`, `flat_map` take
  the collection by value and clone nothing; keep the original by cloning it first.
  In-place changes (`sweep`, `sort`, `reverse`) take `&mut self`, and reads (`find`, `first`,
  `map` → `Vec`, …) take `&self`.
- **Alternatives:** clone from `&self` like discord.js (needs `K: Clone`, `V: Clone`; the
  recommendation); both versions of every method.

### D-215 — `Collection::remove` keeps the order
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `remove` keeps insertion order like `delete` on a JS `Map` (O(n): later
  entries move up); `swap_remove` removes in O(1) by moving the last entry into the gap;
  `sweep` removes many in one O(n) pass.
- **Alternatives:** `remove` as the O(1) swap, with `shift_remove` keeping the order.

### D-216 — `Collection<K, V, S>` hashes with foldhash by default
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the default `S` is `foldhash::fast::RandomState`, as in `djs-cache` (D-158);
  `foldhash` becomes a dependency of `djs-collection` (already a workspace dependency).
  Another hasher can be chosen through `S`.
- **Alternatives:** std's SipHash (no extra dependency, resistant to HashDoS, slower).

### D-217 — `Collection` has its own API, with access to the `IndexMap`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Collection` offers its own map methods (`get`, `get_mut`, `insert`,
  `remove`, `contains_key`, `len`, `iter`, `keys`, `values`, …) and the discord.js methods,
  with its own iterator types; `as_index_map`, `as_index_map_mut`, `into_index_map` and
  `From<IndexMap>` give access to the inner `IndexMap` for rare operations, so a new major
  version of `indexmap` only touches those.
- **Alternatives:** `Deref` to `IndexMap` (its whole API, but its versions become ours);
  no access at all.

### D-218 — `djs-util` stays empty for now; Phase 7 is closed
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `djs-util` stays an unpublished placeholder with no items. It is filled only
  when two or more crates need the same helper; the shard ID calculation of
  `@discordjs/util` already lives in `djs-model` (`ShardId::for_guild`), and its other
  helpers duplicate std (`LazyLock`, ranges) or are JavaScript-specific. Phase 7 is done with
  `djs-formatters` and `djs-collection`; Phase 8 is next.
- **Alternatives:** remove the crate from the workspace; fill it with Rust counterparts of
  `@discordjs/util` now.

### D-219 — Phase 8 order: events, structures, interactions, collectors, partials, examples
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Phase 8 is built in the ROADMAP order: (1) every event, (2) structures with
  methods and managers (fetch / cache), (3) interactions (replies, deferrals, follow-ups,
  modals, autocomplete), (4) collectors and `await_*` helpers, (5) partials and intents
  guidance, then the examples directory.
- **Alternatives:** interactions right after the events; interactions first with a minimal
  set of events.

### D-220 — Structures with methods are extension traits taking `&ctx`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** following D-006 (`msg.reply(&ctx, "Pong!")`), the methods of discord.js
  structures (`reply`, `kick`, `send`, …) are extension traits on the `djs-model` types
  (`MessageExt`, `GuildMemberExt`, …, in `djs_rs::ext` and the prelude); every method takes
  the `Context` (or the `Client`) as an argument. Event payloads, cached data and REST
  results stay the same types, with no per-object handle.
- **Alternatives:** own djs-rs structures holding the data and a client handle
  (`msg.reply("Pong!")`, +8–16 B and an `Arc` clone per object); both levels.

### D-221 — Events with a previous state hand it in one payload
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** handlers keep the `|ctx, event|` signature for every event (D-171). Events
  that change cached state get a payload holding both, for example
  `{ old: Option<Arc<CachedMessage>>, new: … }`; `old` is `None` when nothing was cached
  (no cache, disabled resource, or never seen), like a discord.js partial.
- **Alternatives:** separate `|ctx, old, new|` arguments (different `on` signatures per
  event); the previous state through `ctx.previous()`.

### D-222 — `Events` uses discord.js' event names, plus `Raw`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `Events` covers discord.js' events by their names (`Events::GuildMemberAdd`,
  …), including the events discord.js derives where they make sense (for example
  `GuildAvailable` from `GUILD_CREATE`), plus `Events::Raw`, which hands every gateway
  dispatch as its `djs-model` value so nothing is out of reach.
- **Alternatives:** one event per gateway dispatch with Discord's names; gateway events and
  `Raw` only, derived events later.

### D-223 — Derived events without a cache: the client keeps only unavailable guild IDs
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the `Client` always keeps, per shard, the IDs of the guilds it knows to be
  unavailable (from `READY` and from `GUILD_DELETE` with `unavailable`; 8 B per guild, freed
  as they arrive), so `GuildCreate` (joined), `GuildAvailable` and `GuildUnavailable` are
  told apart with or without a cache. The per-item events derived by diffing a list
  (`EmojiCreate` / `EmojiDelete` / `EmojiUpdate`, the sticker ones, …) need that resource in
  the cache; without it, only the event carrying the whole list is emitted, and the
  documentation says so.
- **Alternatives:** every derived event needs the cache (`GUILD_CREATE` is always
  `GuildCreate`); the client always keeps the guild IDs and the emoji and sticker lists.

### D-224 — `ShardReady` and `ClientReady` fire at `READY`, without waiting for guilds
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ShardReady` fires at each shard's `READY`; `ClientReady` fires once, at the
  `READY` of the last shard to become ready. Neither waits for the guilds' `GUILD_CREATE`s
  (discord.js waits up to `waitGuildTimeout`), so the cache can still be filling; the
  documentation says so.
- **Alternatives:** discord.js' behaviour (wait for the guilds or a timeout); unchanged
  (`ClientReady` at every shard's `READY`, no `ShardReady`).

### D-225 — Handler payloads: the `djs-model` type where it suffices
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** an event with nothing to add hands its `djs-model` payload unchanged (no
  copy). Events with a previous state (D-221) or derived by the client get their own types in
  `djs_rs::event::payload`, named after the event (`payload::MessageUpdate { old, new }`,
  `payload::EmojiUpdate { old, new }`, …).
- **Alternatives:** an own type for every event, wrapping the model.

### D-226 — Shard lifecycle events; diagnostics stay in `tracing`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ShardDisconnect`, `ShardReconnecting`, `ShardResume`, `ShardError` and
  `Invalidated` are events, because bots react to them; discord.js' `debug`, `warn`,
  `error` and `cacheSweep` stay `tracing` output only (D-026).
- **Alternatives:** all of them as events too; no lifecycle events (only `tracing` and
  `Raw`).

### D-227 — Previous state only where discord.js hands an old or removed object
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a payload with the previous state (D-221) is given to the events whose
  discord.js counterpart hands an old or a removed object: updates (`MessageUpdate`,
  `GuildUpdate`, `ChannelUpdate`, `RoleUpdate`, `GuildMemberUpdate`, `PresenceUpdate`,
  `VoiceStateUpdate`, `ThreadUpdate`, `UserUpdate`, `StageInstanceUpdate`,
  `GuildScheduledEventUpdate`, the soundboard, emoji and sticker updates, …) and removals
  (`MessageDelete`, `MessageBulkDelete`, `GuildDelete`, `GuildMemberRemove`, `RoleDelete`,
  `ThreadDelete`, …). Other events (reactions, typing, …) keep their `djs-model` payload
  (D-225), even where the cache returns a previous state.
- **Alternatives:** every event for which the cache returns a previous state.

### D-228 — Previous-state payloads: `old` and `new`, with `Deref` to `new`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** those payloads have two public fields: `old` (an `Option` of the cached state
  before the event) and `new` (Discord's data for the event, as its `djs-model` type), and
  implement `Deref<Target = …>` to `new`, so `update.content` reads like it does on
  `MessageCreate`. For a removal, `new` is Discord's removal notice (`delete.id`,
  `delete.channel_id`) and `old` the removed object.
- **Alternatives:** the same fields without `Deref`; `old` and `event` without `Deref`.

### D-229 — Managers are lightweight objects on the `Context`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** discord.js' managers become small types borrowing the `Context` (its `Rest`
  and optional cache): `ctx.guilds().fetch(id)`, `ctx.members(guild_id).fetch(user_id)`,
  `ctx.messages(channel_id).fetch(id)`, `ctx.channels().fetch(id)`, … Each gathers the
  operations of one resource (`fetch`, list, create, edit, delete).
- **Alternatives:** flat methods on the `Context` (`ctx.fetch_member(guild, user)`);
  extension traits on IDs (`guild_id.members(&ctx).fetch(user)`).

### D-230 — `fetch` reads the cache first and writes REST results back
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like discord.js, a manager's `fetch` returns the cached value when the cache
  holds it, otherwise asks the REST API; a `force` option always asks the REST API. A REST
  result is stored in the cache when that resource is enabled. `djs-cache` gains a public
  write API for REST objects (for example `cache.insert_member(guild_id, &member)`).
- **Alternatives:** always the REST API, the cache only read explicitly; cache first without
  writing REST results back.

### D-231 — Lists return `Collection`; cache views stay iterators
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** list operations of the managers (`members.list()`, `messages.fetch_many()`,
  `bans.fetch()`, …) return a `djs_collection::Collection<Id, T>` in Discord's order, with
  discord.js' helpers. The `djs-cache` views keep returning iterators and views (no copy).
  Resolves the open question "`Collection` in the cache and the client".
- **Alternatives:** `Vec<T>` everywhere; `Collection` from the cache views too (a copy per
  call).

### D-232 — Structures and methods: the most used first
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** Phase 8 step 2 starts with what bots use most — messages (`reply`, `edit`,
  `delete`, `react`, `pin`, threads), channels (`send`, typing, bulk delete), members
  (`kick`, `ban`, roles, timeout, nickname), guilds (members, bans, roles) and users (DMs) —
  then the other resources, one commit per batch.
- **Alternatives:** resource by resource in discord.js' documentation order.

### D-233 — `fetch` returns the cached type, behind an `Arc`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a manager's `fetch` (and its lists) always return the type the cache stores:
  `Arc<CachedMember>`, `Arc<CachedMessage>`, `Arc<CachedGuild>`, `Arc<CachedUser>` for the
  compact cached types (D-081), `Arc<Channel>`, `Arc<Role>`, … where the cache stores the
  model type. A REST result is converted to it (and stored when the resource is enabled), so
  a cache hit is an `Arc` clone. The structure methods (D-220) are implemented for both the
  model and the cached types.
- **Alternatives:** always the model type (a cache hit converted back, allocating); an enum of
  the cached and the REST value.

### D-234 — Guild-wide channel operations get their own manager
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** operations on one channel by its ID (fetch, edit, delete, permission
  overwrites, typing) stay on `ctx.channels()`; operations on a guild's channels (create,
  list, positions) go to a guild-scoped manager, `ctx.guild_channels(guild_id)` /
  `guild.channels(&ctx)`, like discord.js' `GuildChannelManager` and consistent with
  `ctx.roles(guild)` and `ctx.members(guild)`.
- **Alternatives:** everything on `ctx.channels()`, with the guild ID as a parameter.

### D-235 — Permission overwrites: a typed target, Discord's semantics
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `set_permissions(channel, target, allow, deny, reason)` and
  `delete_permissions(channel, target, reason)`, where the target is a role or a member told
  apart by the ID type (`RoleId` / `UserId`). Setting replaces the whole overwrite, as
  Discord's PUT does.
- **Alternatives:** the REST shape (`GenericId` + `EditChannelPermissions`, the caller casts
  the ID and picks the type); discord.js' `edit` merging with the cached overwrite and
  `create` replacing it.

### D-236 — REST deletions leave the cache to the gateway event
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** when the bot deletes a channel, role, message, … through a manager, the
  manager only sends the request. The cache changes when the gateway event (`CHANNEL_DELETE`,
  …) arrives, and the delete event fires once, from it, with the previous state (`old`).
- **Alternatives:** like discord.js, remove from the cache at once and emit the delete event
  from the REST call (the gateway event then finds nothing); managers would need the client's
  event emitter.

### D-237 — Webhooks: one `ctx.webhooks()` manager
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** one manager by webhook ID — fetch, edit, delete, send with the webhook token
  and the webhook's messages — which also creates (`create(channel, …)`) and lists
  (`list_channel(channel)`, `list_guild(guild)`). Shortcuts on the structures:
  `channel.create_webhook` / `channel.webhooks`, `guild.webhooks`, and methods on `Webhook`
  (`send`, `edit`, `delete`). Webhooks are not cached: the `djs-model` types are returned.
- **Alternatives:** split like D-234, with a `ctx.channel_webhooks(channel)` manager for
  create and list.

### D-238 — `WebhookClient` now, in djs-rs
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** a small `WebhookClient` in `djs-rs` for a webhook's URL or ID and token,
  without a bot token or a gateway: `WebhookClient::from_url(url)`, `send`, fetch / edit /
  delete of its messages, `edit`, `delete` (discord.js: `WebhookClient`).
- **Alternatives:** the same type in `djs-rest`; later (users call `Rest` without a token).

### D-239 — Invites: one `ctx.invites()` manager
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** invite codes are global, so one manager without a guild: by code `fetch` and
  `delete`, plus `create(channel, …)`, `list_channel(channel)` and `list_guild(guild)`.
  Shortcuts: `channel.create_invite` / `channel.invites`, `guild.invites`. Invites are not
  cached: the `djs-model` types are returned.
- **Alternatives:** like discord.js, a guild-scoped `ctx.invites(guild)` plus
  `ctx.fetch_invite(code)` for other guilds' invites.

### D-240 — A webhook without a token is a `WebhookError::MissingToken`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the structure methods that need a webhook's token (sending, its messages)
  return `Result<_, WebhookError>`, where `WebhookError` is `MissingToken` (channel-follower
  and application webhooks have none) or `Rest(djs_rest::Error)`; like discord.js'
  `WebhookTokenUnavailable`. The other methods keep `djs_rest::Error`.
- **Alternatives:** `webhook.with_token() -> Option<…>` giving access to the token methods;
  a documented panic.

### D-241 — `WebhookClient::from_url` returns `Result<_, InvalidWebhookUrl>`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `WebhookClient::from_url(url)` parses `https://discord.com/api/webhooks/{id}/{token}`
  (also `discordapp.com`, the `canary.` / `ptb.` hosts and a versioned `/api/v10/` path) and
  fails with an `InvalidWebhookUrl` error; `WebhookClient::new(id, token)` takes the parts
  without parsing.
- **Alternatives:** `Option<WebhookClient>`.

### D-242 — Threads: a thread manager and a thread member manager
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like discord.js, `ctx.threads(parent_channel)` creates threads (`create`,
  `create_in_forum`) and lists the archived ones (public, private, joined private), and
  `ctx.thread_members(thread)` fetches, lists, adds and removes members, joins and leaves.
  A guild's active threads are listed by `ctx.guild_channels(guild).list_active_threads()`.
  Shortcuts: `channel.threads(&ctx)`, `channel.thread_members(&ctx)`. Editing and deleting a
  thread stay on `ctx.channels()`.
- **Alternatives:** everything on `ctx.channels()` with longer names (`start_thread`,
  `join_thread`, `add_thread_member`, …).

### D-243 — Discord's default soundboard sounds: a `Context` method
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ctx.default_soundboard_sounds()` lists them (discord.js:
  `Client#fetchDefaultSoundboardSounds`); a guild's sounds are on
  `ctx.soundboard_sounds(guild)`.
- **Alternatives:** `ctx.soundboard_sounds(guild).list_default()`.

### D-244 — Application commands: a global and a guild manager
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like discord.js, `ctx.commands()` manages the global commands (`fetch`,
  `list`, `create`, `edit`, `delete`, `set` overwriting all of them) and
  `ctx.guild_commands(guild)` a guild's, with reading the command permissions. Shortcut:
  `guild.commands(&ctx)`. Commands are not cached: the `djs-model` types are returned.
- **Alternatives:** one `ctx.commands()` taking an `Option<GuildId>` scope in every method.

### D-245 — The application ID comes from `READY`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the client keeps `application.id` from the first `READY`;
  `ctx.application_id()` is available to every handler without an extra request.
- **Alternatives:** `ClientBuilder::application_id(id)`, otherwise Get Current Application
  at login (one more request).

### D-246 — The audit log and integrations are on `GuildManager`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ctx.guilds().audit_log(guild, &query)`, `ctx.guilds().integrations(guild)`
  and `ctx.guilds().delete_integration(guild, id, reason)`, with the shortcuts
  `guild.audit_log(&ctx, …)` and `guild.integrations(&ctx)` (discord.js:
  `Guild#fetchAuditLogs`, `Guild#fetchIntegrations`).
- **Alternatives:** separate `ctx.audit_log(guild)` and `ctx.integrations(guild)` managers.

### D-247 — Application managers panic before the first `READY`
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `ctx.application_id()` returns `Option<ApplicationId>`; the managers that
  need it (`ctx.commands()`, `ctx.guild_commands(guild)`, application emojis) panic when
  called before the first `READY`, documented under `# Panics`. Handlers of gateway events
  always run after `READY`, so ordinary code needs no `unwrap`.
- **Alternatives:** `Option<CommandManager>`; reading the bot's ID from the token as a
  fallback before `READY`.

### D-248 — Interaction responses: a stateless extension trait
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** `InteractionExt` on `Interaction` (and the `InteractionCreate` payload):
  `interaction.reply(&ctx, …)`, `defer_reply`, `edit_reply`, `follow_up`, `defer_update`,
  `update`, `show_modal`, `respond` (autocomplete), …, one-to-one with Discord's callbacks
  (as D-220). The rules of their order (for example `defer_reply` then `edit_reply`) are
  documented; Discord rejects a wrong order.
- **Alternatives:** a stateful responder tracking `replied` / `deferred` like discord.js and
  failing locally (a `reply` after a deferral becoming an edit).

### D-249 — One interaction event with typed accessors
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** like discord.js, one `Events::InteractionCreate`; `Interaction` gets
  accessors — `command()`, `component()`, `modal()`, `autocomplete()` — returning the typed
  data when the interaction is of that kind (discord.js: `isChatInputCommand()`, …).
- **Alternatives:** extra events per kind (`CommandInteraction`, `ComponentInteraction`, …),
  which discord.js does not have.

### D-250 — Option and value getters are `djs-model` methods
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** the typed getters (`get_string("name")`, `get_user` with the resolved data,
  `subcommand()`, modal field values, component values) are inherent methods of
  `CommandData`, `ComponentData` and `ModalSubmitData` in `djs-model`, usable without
  `djs-rs` (for example by an HTTP interactions server). discord.js:
  `CommandInteractionOptionResolver`.
- **Alternatives:** extension traits in `djs-rs`, keeping `djs-model` data only.

### D-251 — A command router in Phase 8 step 3
- **Date:** 2026-11-12 · **Decided by:** Owner
- **Decision:** beyond discord.js, a simple router registered on the client dispatches
  interactions by command name (and component `custom_id`) — `client.command("ping",
  handler)` — next to `Events::InteractionCreate`. Its exact shape is asked when it is
  built.
- **Alternatives:** none now (a `match` on the name, as with discord.js), maybe after
  Phase 8.

### D-252 — Router handlers get a typed interaction wrapper
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** a handler registered with the router (D-251) receives `(ctx, CommandInteraction)`
  — and `ComponentInteraction`, `ModalInteraction`, `AutocompleteInteraction` for the other
  kinds — a wrapper holding the same `Arc<InteractionCreate>` (no extra allocation) that
  dereferences to the typed data (`get_string`, `subcommand`, …) and offers the responses of
  `InteractionExt` (`cmd.reply(&ctx, …)`).
- **Alternatives:** the event's `(ctx, Arc<InteractionCreate>)`, the data taken out with
  `interaction.command().unwrap()`.

### D-253 — Subcommands are routed by their full path
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the route key is the full command name as Discord shows it:
  `client.command("mod ban", …)`, `client.command("mod config set", …)`. A route for the
  top-level name (`"mod"`) catches the subcommands that have no route of their own.
- **Alternatives:** only the top-level name (the handler matches `subcommand()`); a nested
  router builder.

### D-254 — Component and modal `custom_id`s are routed by the prefix before `:`
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** `client.component("vote", …)` (and `client.modal(…)`) catches the custom IDs
  `vote` and `vote:42`; the rest after the first `:` (`42`) is available to the handler as
  its arguments. Routes are looked up in a hash map, no regular expressions.
- **Alternatives:** exact matches only; exact matches plus arbitrary prefixes (a linear scan).

### D-255 — The router only routes; it does not register commands
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the router dispatches incoming interactions to handlers; commands are
  registered with Discord as before (`ctx.commands().set(…)`, `ctx.guild_commands(g).set(…)`,
  for example in `ClientReady`). `Events::InteractionCreate` is still emitted for every
  interaction.
- **Alternatives:** routes registered with command builders and synchronised with a bulk
  overwrite at `READY`.

### D-256 — The values of `Resolved` are boxed (amends D-058)
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the six maps of interaction `Resolved` data are `BTreeMap<Id, Box<V>>`
  (users, members, roles, channels, messages, attachments). A B-tree leaf reserves room for
  11 values, so one-entry maps of large values cost ≈ 6.7 KB per interaction; boxed, an
  interaction takes 3.14 KB instead of 8.72 KB (measured, same decoding time; Phase 8 step
  3d). The other dictionaries of D-058 keep plain values. The getters of D-250 still return
  `&User`, `&Role`, …. Breaking, noted in the CHANGELOG.
- **Alternatives:** keep `BTreeMap<Id, V>`; a new compact sorted-`Vec` map type.

### D-257 — Heavy runs go through a resource guard
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the owner's laptop hard-froze on 2026-11-13 (03:55) while an assistant ran
  back-to-back release `cargo bench` builds after hours of builds and stress floods; the
  journal just stops (no OOM, thermal or hardware message). From now on every assistant
  `cargo` build, clippy, test, bench and stress test runs through a guard: AC power required;
  start only below 70 °C and a 1-minute load below 4; a `systemd-run --user --scope` with
  `CPUQuota=400%` (4 of 12 threads), `MemoryMax=6G`, `MemorySwapMax=0`, nice 19 and idle I/O;
  `CARGO_BUILD_JOBS`, `TOKIO_WORKER_THREADS` and `RUST_TEST_THREADS` set to 4; a watchdog
  stopping the whole scope at 90 °C (twice in a row), on unplugged AC or after a timeout.
  Heavy runs are never looped back to back. Numbers measured under the guard (4 workers) are
  not directly comparable with earlier ones (12 workers); `docs/PERFORMANCE.md` says which.
- **Alternatives:** 2 CPUs / 4 GB / 85 °C (coolest, ≈ 5× slower builds); 6 CPUs / 8 GB.

### D-258 — Smaller stress tests, run only after asking
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the heaviest stress tests are reduced (the 16-shard reply flood from 200 000 to
  50 000 interactions, a peak below 0.5 GiB), and stress tests run only after the owner agrees
  each time, always through the guard (D-257), never several back to back without a pause.
  Benchmarks run singly through the guard.
- **Alternatives:** keep the sizes; run stress tests only when the owner asks.

### D-259 — Heavy runs resume: no core limit, stop at 99 °C, a LAN monitor
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** after checking the computer, the owner lifts the pause and the core limit of
  D-257: builds, tests and stress tests use every core again. The guard keeps the other
  safeguards (AC power, `MemoryMax=6G` without swap, nice/idle I/O, a timeout, a cool start)
  and its overheating stop moves to **99 °C** (one reading, sampled every second; the CPU's
  limit is 100 °C). A minimal monitor page on the local network shows temperatures, load,
  memory, fan, power and the guard's state, and logs every sample to disk so that the last
  readings survive a freeze. Stress tests still run one at a time with a cool-down between
  them (D-258).
- **Alternatives:** E-core pinning at 88 °C; a balanced/quiet platform profile.

### D-260 — The fan runs at full speed during heavy runs
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** during every assistant build, test or benchmark — from the guard's cool-down
  wait to the end of the run — the ASUS fan is forced to full speed (`pwm1_enable` = 0 of the
  `asus` hwmon) against overheating; otherwise it stays under the firmware's automatic
  control (2). The guard restores automatic control on every exit, the monitor restores it if
  no guard run is active (a guard killed without its exit trap), and the guard refuses to
  start a run if it cannot force the fan. Writing `pwm1_enable` needs root once: a udev rule
  gives the `wheel` group write access to that one file.
- **Alternatives:** none (the owner's requirement).

### D-261 — Power and degradation rules of the guard
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** AC power may be unplugged at times: unplugged for 30 s is only a warning (the
  guard's output, the monitor). The guard stops the heavy command — never the workflow around
  it — only on a significant deterioration: the package at 99 °C, a battery below 80 % without
  AC, less than 1 GiB of available RAM, or sustained thermal throttling (100+ package
  throttle events in 30 s). It starts on battery only at 80 % or more. Verified the same day:
  12 threads at 100 % for 2 minutes with the fan forced to full speed peaked at 83 °C (77–79 °C
  steady, P-cores 2.9–3.3 GHz, no throttling), against 94 °C at 4 threads with the fan on
  automatic control.
- **Alternatives:** stopping at once when AC is unplugged (the first rule of D-257).

### D-262 — The guard stops only on a real danger; the balanced profile and its sweet spot
- **Date:** 2026-11-13 · **Decided by:** Owner (the thresholds chosen by Claude on the
  owner's instruction)
- **Decision:** the guard must not stop a heavy command over a theoretical risk, only when
  the computer is really in danger of shutting down. The owner switched the platform profile
  to **balanced** (PL1 23 W, PL2 39 W) and asked for the load "sweet spot" that uses the safe
  maximum with an **absolute peak of 95 °C**. The guard now stops (exit 124) only when the
  CPU package stays at 99 °C for 10 s in a row (the CPU's limit is 100 °C, where it throttles
  itself; a single reading no longer counts), when the ACPI zone `acpitz` comes within 3 °C of
  its critical trip (103 °C here, where the kernel powers the laptop off), on a battery below
  80 % without AC (the owner's rule of D-261), below 1 GiB of available RAM (a freeze), or on
  its timeout. Thermal throttling and temperatures at 95 °C and above are warnings only; each
  run records its seconds at 95 °C+ and its throttle events. Measured sweet spot: a clean
  quality gate (clippy, tests, docs, everything built from scratch) on all 12 CPUs in the
  balanced profile peaked at **79 °C** with no throttling in 126 s, so no core limit is
  needed; the release stress tests confirm it (`docs/PERFORMANCE.md`).
- **Alternatives:** the throttling stop of D-261 (it ended the D-256 gate during `cargo doc`
  on single-core turbo bursts in the performance profile, peak 96 °C); a core limit.

### D-263 — Collectors are streams, with `await_*` shortcuts
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** a collector is a `Stream` of the collected items, configured with a builder
  (`time`, `idle`, `max`, `filter`, …): `while let Some(item) = collector.next().await`, and
  `collector.end_reason()` afterwards. Shortcuts `await_*` wait for one item or collect into a
  `Collection`. No callbacks.
- **Alternatives:** discord.js-style callbacks (`on_collect`, `on_end`) in a spawned task;
  both.

### D-264 — Events reach collectors through an index by ID
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the client keeps one shared registry of collectors keyed by the channel,
  message or interaction they watch. Each event costs one lookup (one atomic read when no
  collector exists), whatever the number of collectors.
- **Alternatives:** every collector receives every event and filters it (like
  `client.events()`), O(collectors) per event.

### D-265 — Step 4 scope: four collectors and the gateway member requests
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** Phase 8 step 4 brings the message, reaction (with removals / dispose),
  component and modal collectors of discord.js, and the gateway requests left open by D-149:
  fetching members through `REQUEST_GUILD_MEMBERS` (assembling `GUILD_MEMBERS_CHUNK`) and the
  soundboard sounds (`SOUNDBOARD_SOUNDS`).
- **Alternatives:** only the four collectors (the member requests later); only the
  interaction collectors first.

### D-266 — Collected events still reach the handlers and the router
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** a collector is one more listener, as in discord.js: the event handlers
  (`InteractionCreate`, `MessageCreate`, …) and the router's routes receive every event
  whether or not a collector collected it. Avoiding a double reply is the bot's job (for
  example distinct `custom_id` prefixes).
- **Alternatives:** a collected interaction skips the router's routes.

### D-267 — Collectors in `Sequential` mode: documented, with a warning
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** a handler that awaits a collector in `HandlerMode::Sequential` blocks its
  shard, so the collector receives nothing from that shard until it ends. This is documented
  in rustdoc and the client logs a `tracing` warning when a collector is created in
  `Sequential` mode; the default `Concurrent` mode has no such limit. Nothing is added to the
  event path.
- **Alternatives:** the shard keeps reading while a handler waits, feeding the collectors and
  queueing the events for the handlers (memory grows while it waits); an error in
  `Sequential` mode.

### D-268 — What the `await_*` shortcuts return
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the single-item shortcuts (`await_component`, `await_modal_submit`) return
  `Result<T, CollectorError>`, the error carrying the end reason (time, message deleted, …),
  like discord.js' rejection. The multi-item shortcuts (`await_messages`, `await_reactions`)
  return `Collected`: the `Collection` (through `Deref`) with its `reason()`; an empty result
  is not an error (discord.js' default without `errors`).
- **Alternatives:** `Option<T>` and a plain `Collection` (the reason is lost); `Result`
  everywhere, an empty result being an error.

### D-269 — Short collector method names
- **Date:** 2026-11-13 · **Decided by:** Owner (not the recommendation)
- **Decision:** the extension methods (D-220) are `message_collector` / `await_messages`,
  `reaction_collector` / `await_reactions`, `component_collector` / `await_component` and
  `await_modal_submit`, shorter than discord.js' `createMessageCollector`,
  `createMessageComponentCollector`, `awaitMessageComponent`; the rustdoc of each names its
  discord.js counterpart.
- **Alternatives:** discord.js' names in snake_case (`create_message_collector`,
  `await_message_component`, …; the recommendation).

### D-270 — Gateway member requests: separate methods with a builder
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the member manager gains `fetch_all()`, `fetch_by_query(query)` (with
  `limit`) and `fetch_many(ids)`, each with `presences(bool)` and `time(Duration)`; they send
  `REQUEST_GUILD_MEMBERS` on the guild's shard, assemble the `GUILD_MEMBERS_CHUNK` replies
  matched by nonce, store the members in the cache and return
  `Collection<UserId, Arc<CachedMember>>`. `fetch(user)` stays on REST.
- **Alternatives:** one `fetch_with(FetchMembers { … })` method with options, like
  discord.js' `guild.members.fetch({ … })`.

### D-271 — Missing intents are warned about at login
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** every event carries the intents Discord sends it under (an `INTENTS`
  constant from the `events!` macro, following Discord's documentation). At `login()` the
  client logs a `tracing` warning naming the events that registered handlers, collectors and
  routes listen to but whose intents the client does not request. The bot still starts and
  nothing is added to the event path.
- **Alternatives:** an `Error::MissingIntents` from `login()`; documentation only.

### D-272 — `required_intents()` shows the intents, setting them stays explicit
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** `client.required_intents()` returns the intents the registered handlers,
  collectors and routes need, to print or compare; the intents are still set explicitly
  with `ClientBuilder::intents`, as in discord.js.
- **Alternatives:** an automatic mode computing the intents at login (`intents_auto()`, with
  privileged intents allowed separately); no helper.

### D-273 — No `partials` option; a guide maps discord.js' partials
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** djs-rs keeps delivering every event whether or not its objects are cached
  (the previous state is `old: None` when unknown, D-227/D-228). The documentation gains a
  table from each discord.js partial to what djs-rs gives instead (`old: None`, the partial
  payload type) and how to complete it (`fetch()`).
- **Alternatives:** a discord.js-like `ClientBuilder::partials(…)` dropping events about
  uncached objects unless enabled.

### D-274 — Four groups of examples
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** Phase 8 step 6 adds examples of: slash commands with the router (guild
  command registration, subcommands, autocomplete, a modal with `await_modal_submit`,
  ephemeral and deferred replies); components and collectors (a paginated button menu, a
  quiz with `await_messages`, a reaction poll with `await_reactions`); moderation and the
  cache (welcomes, an edit and deletion log with the previous state, timeouts and bans,
  `fetch_all`, cache and intents configuration); advanced use (sharding and presence, the
  `client.events()` loop, `WebhookClient` without a bot, `HandlerMode::Sequential`, mimalloc
  as the global allocator).
- **Alternatives:** any subset.

### D-275 — Examples are verified briefly on the test bot
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** each example is run once against the owner's test guild with the token from
  `.env`, after asking the owner each time and saying what it will do (some need the owner's
  clicks in Discord). Commands are registered as guild commands and removed afterwards; test
  messages are deleted (as for D-184 / D-195).
- **Alternatives:** compile only and test the logic on mocks; the owner runs them.

### D-276 — Single-file examples
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** every example is one file `crates/djs-rs/examples/<name>.rs`
  (`cargo run --example <name>`), like `ping.rs`, with how to run it in its header; the
  shared `.env` reading lives in a small module included with `#[path]`.
- **Alternatives:** also a template bot crate in the workspace, split into modules.

### D-277 — A comparison with serenity, after Phase 8
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** djs-rs is compared with [serenity](https://github.com/serenity-rs/serenity),
  the Rust library the owner wants djs-rs to beat, in a step of its own started once
  Phase 8 is closed (examples included, with their live runs), before Phase 9.
- **Alternatives:** right after the examples are committed, before their live runs; at once,
  setting the last example aside.

### D-278 — The serenity comparison is a crate outside the workspace and `docs/VS_SERENITY.md`
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the harness is `benches/vs-serenity/`, a `publish = false` crate kept out of
  the workspace (its own `Cargo.lock`, serenity pinned at 0.12.5, the latest release), so
  the quality gate never builds serenity. It reuses the mock Discord servers and the
  payload generator of `benches/vs-node/` (D-123): the same input byte for byte, the mock in
  its own process, pinned CPUs, the same `/proc` metrics. The results go to
  `docs/VS_SERENITY.md` (linked from the main README), with tables and SVG charts generated
  from the measured results; every number is measured, never estimated, and where serenity
  wins it is written down as it is.
- **Alternatives:** a workspace member like `benches/vs-node` (serenity in the main lockfile,
  built by every gate); a third column in `docs/VS_NODE.md`.

### D-279 — What is compared with serenity: performance, resources, reliability
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** three groups. **Performance:** decoding `MESSAGE_CREATE` and a 1000-member
  `GUILD_CREATE`, an event flood through the mock gateway (events/s, CPU), REST throughput
  and p50/p99 latency against the mock API. **Resources:** the memory of an idle bot and of
  500 000 cached members, the binary size, the build time and the number of dependencies.
  **Reliability:** failure scenarios of the mock gateway (a dropped connection, `RESUME`,
  `INVALID_SESSION`, a missing heartbeat ACK) and of the mock API (429, 5xx), counting lost
  and duplicated events and the time to recover. Ease of use and API coverage are not part
  of it.
- **Alternatives:** also ease of use and availability (the same bot in both libraries, a
  coverage table of Discord's features, documentation coverage, MSRV).

### D-280 — serenity is measured with its default configuration
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** serenity 0.12.5 runs with its default features and settings, as most bots
  use it; djs-rs is measured with a cache equal to serenity's default cache and with its own
  defaults, as in `docs/VS_NODE.md` (D-174).
- **Alternatives:** also serenity's fastest configuration (`simd_json`, a tuned cache); also
  its unreleased `next` branch.

### D-281 — Phase 8 is closed without exercising two examples live
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** the live runs of D-275 count as done: `slash_commands` and `moderation` were
  exercised on the test guild; `components` and `advanced` logged in, became ready and shut
  down cleanly but received no test messages, and are not run again. Their behaviour is
  covered by the collector, router, webhook and event-stream tests on the mock servers. The
  owner's two test messages of the runs were deleted with the bot's on the owner's request.
- **Alternatives:** rerun both, or only `components`, with the owner at the keyboard.

### D-282 — serenity reaches the mocks through a TLS mock, with its default rate limiter
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** serenity 0.12.5's default rate limiter builds its requests without the
  `HttpBuilder::proxy` URL (`ratelimiting.rs`, `build(…, None)`), so a proxy only works with
  the limiter disabled. Instead, serenity keeps its default configuration and gets one
  harness-built `reqwest::Client` (`HttpBuilder::client`) that resolves `discord.com` to a
  local HTTPS mock and trusts only a test certificate authority (no built-in roots), so it
  cannot talk to the real Discord. The HTTPS mock answers exactly like the plain one (the same
  `vs-node` response code). New dependencies, in `benches/vs-serenity` only: `rcgen` (the test
  CA and certificate), `tokio-rustls` (the TLS server) and `reqwest` (the version serenity
  resolves); `hyper`, `hyper-util`, `http-body-util` and `bytes` are already in the project.
  Since serenity then pays for TLS on loopback while djs-rs talks plain HTTP, the REST
  scenario also measures serenity with its limiter disabled over plain HTTP (its documented
  proxy setup), labelled, as a bound. The 429/5xx reliability scenarios use the default
  limiter.
- **Alternatives:** serenity with its limiter disabled everywhere (no 429/5xx scenarios for
  it); TLS for both with a djs-rest option to trust extra root certificates.

### D-283 — After the serenity comparison: fixes before Phase 9
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** before Phase 9, (1) investigate and shorten the time a bot takes to rebuild
  after an edit (`docs/VS_SERENITY.md`: release 9.9 s with djs-rs vs 5.0 s with serenity,
  the only scenario serenity wins), and (2) close the ergonomics gaps the example bots found
  (the open question of C-107: setters for message bodies, `From<String>` for
  `EditWebhookMessage`, `Vec` vs `Option<Vec>`, member permissions from the cache, shutting
  down from a handler). Each public API change is still asked first.
- **Alternatives:** only the rebuild time, then Phase 9; Phase 9 at once, fixes later.

### D-284 — Multi-process sharding lives in `djs-rs`
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** Phase 9's process manager (≈ discord.js `ShardingManager`, `Shard`,
  `ShardClientUtil`) is a module of the main crate, `djs_rs::sharding`, behind an opt-in
  cargo feature `sharding`; the shard side integrates with `Client` (≈ `client.shard`).
- **Alternatives:** a separate `djs-sharding` crate re-exported by `djs-rs`.

### D-285 — Sharding IPC: TCP on 127.0.0.1 with JSON frames
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** shard processes talk to the manager over TCP on `127.0.0.1`, with
  length-prefixed JSON frames and a one-time token passed in an environment variable to
  authenticate each process. The same on Linux, macOS and Windows, without new dependencies,
  and the bot keeps its stdin/stdout.
- **Alternatives:** the child's stdin/stdout (takes the bot's standard output); Unix domain
  sockets / Windows named pipes (separate code per platform).

### D-286 — Instead of `broadcastEval`: built-in values and named commands
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Decision:** Rust cannot evaluate code sent at run time, so the manager offers
  (1) built-in values fetched from every shard (≈ `fetchClientValues`: guild and user
  counts, ping, uptime, …) and (2) commands registered by name on the shard side, with typed
  arguments and results (serde), which the manager can broadcast to every shard or send to
  one (≈ `broadcastEval`).
- **Alternatives:** only named commands; only built-in values.

### D-287 — Bots rebuild fast: the library builds the futures of its public async API
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context (measured, D-283):** after editing `main.rs`, the `!ping` bot of
  `benches/vs-serenity/bots/djs-rs` rebuilt in 9.9 s (release) against 4.95 s for serenity.
  `-Z time-passes` showed 95 % of it in LLVM for the bot crate, whose IR was 2.3× serenity's
  (727k vs 312k lines): the bodies of `async fn`s are instantiated in the crate that polls
  them, so `client.login().await` pulled the whole client (shard loop, cache update, event
  conversion, hyper/h2) into the bot crate; everything generic over the cache backend
  (`Client<B>`, `Context<B>`, managers) is too; and serde's (de)serialization of the model
  types (`Message`, `Component`, `CreateMessage`, …) is instantiated where it is used.
  Prototypes (release rebuild, median of 3): login and REST unchanged 7.1 s without and
  7.8 s with a `msg.reply`; login boxed in the library 0.98 s without REST; login and reply
  both built in the library **0.97 s**.
- **Decision:** the public async methods — every REST method of `djs-rest`, `Client::login`,
  and the manager and structure methods of `djs-rs` — return `impl Future<Output = …> + Send`
  whose future is created inside the library (boxed), so its body is compiled once, in the
  library. For the default cache backend (`DashMapBackend`), the code that depends on the
  backend is compiled into `djs-rs` through a table of functions that the non-generic
  `Client::builder()` sets; `cache_backend::<C>()` swaps in the generic table, so custom
  backends work as today (their part compiles in the bot). Calls (`.await`) do not change.
  Each call costs one heap allocation; it is benchmarked against the current numbers.
- **Alternatives:** removing the backend type parameter (`Client`/`Context` without `<B>`,
  the cache behind a trait object: a breaking change); boxing only the REST methods and
  `login` (the generic managers stay in the bot crate).

### D-288 — Event handlers stay spawned directly (no boxed handler future)
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context (measured, D-287 step 3):** after D-287 steps 1–2 the moderation example bot
  rebuilt in 2.2 s (release, default profile) and the `!ping` bot in 0.89 s. In the remaining
  IR of the moderation bot, tokio's task machinery for the spawned handler futures was the
  largest share (~21 %). A prototype that boxed every handler future before `tokio::spawn`
  (one shared, non-generic task type compiled in the library) rebuilt in **1.7 s**, but the
  event flood with the cache on was **1.9 % slower** (about +0.2 µs per event).
- **Decision:** keep `tokio::spawn` of the concrete handler future (no extra box) in
  `ErasedHandler::spawn` and in the command router. The hot event path wins over 0.5 s of
  rebuild time.
- **Alternatives:** box the handler futures (1.7 s rebuild, −1.9 % flood throughput).

### D-289 — The manager and structure methods are built in the library too
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context:** D-287's scope named the manager and structure methods (`ban`, `fetch`,
  `fetch_all`, `reply`, …). Their REST part is already built in `djs-rest` (step 1); what
  still compiles in the bot crate is the code that depends on the cache backend (the
  `store_*` conversions and the cache reads of the managers), because `Context<B>` and the
  managers are generic. Estimated gain: 0.1–0.2 s of the moderation bot's 2.2 s. Claude
  recommended stopping at steps 1–2; the owner chose to do it as well.
- **Decision:** route the backend-dependent code of the managers, the structure extension
  methods and the `Context` store helpers through code compiled once in `djs-rs` for the
  default backend (the same idea as D-287's function table: set where the non-generic code
  creates the context), keeping the public API and the custom-backend path unchanged.
  Benchmarked before it is committed, like D-287.
- **Alternatives:** stop after D-287 steps 1–2 (Claude's recommendation).

### D-290 — Message bodies get chainable setters (ergonomics gaps 1 and 2)
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context (C-107, D-283):** the message bodies are `#[non_exhaustive]`, so an ephemeral or
  embed reply needs `let mut reply = MessageCallbackData::from(…); reply.flags = …;`, and
  `EditWebhookMessage` lacks the `From<String>` of its siblings.
- **Decision:** chainable, consuming setters on the body types themselves — `CreateMessage`,
  `MessageCallbackData`, `EditMessage`, `EditWebhookMessage`, `ExecuteWebhook`,
  `CreateFollowupMessage` — such as `.ephemeral()`, `.embed(…)`, `.component(…)`,
  `.allowed_mentions(…)`, `.no_mentions()`, `.file(…)`, where the field exists on that type;
  plus `From<&str>`, `From<String>` and `From<CompactString>` on every one of them. No new
  types and no run-time cost; validation stays in `djs-builders` (`validate_embeds`,
  `validate_components`).
- **Alternatives:** a validating `MessageBuilder` in `djs-builders`; both.

### D-291 — The body fields keep their types; the setters hide the difference (gap 3)
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context:** `CreateMessage` holds `Vec`s (empty = not sent) where `MessageCallbackData`
  holds `Option<Vec>`s, because an update-message response clears the embeds with
  `Some(vec![])` and keeps them with `None`.
- **Decision:** the field types stay as they are (they follow Discord's semantics); the D-290
  setters behave the same on every body, so bot code does not see the difference. No breaking
  change.
- **Alternatives:** `Option<Vec>` everywhere (breaking for `CreateMessage`, `ExecuteWebhook`, …).

### D-292 — Member permissions from the cache: model, cache and member methods (gap 4)
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context:** a prefix command cannot check a moderator's rights: nothing computes a
  member's permissions (discord.js `GuildMember#permissions`, `GuildMember#permissionsIn`,
  `GuildChannel#permissionsFor`). Interactions already carry Discord's computed permissions.
- **Decision:** three layers. A pure computation in `djs-model` (guild owner, `@everyone`,
  the member's roles, a channel's overwrites, `ADMINISTRATOR` grants everything);
  `Cache::member_permissions(guild, user)` and `Cache::member_permissions_in(channel, user)`
  in `djs-cache`; and methods in `djs-rs` on the member (`member.permissions(&ctx)`,
  `member.permissions_in(&ctx, channel)`). The cache and member methods return
  `Option<Permissions>`: `None` when the cache lacks what the computation needs.
- **Alternatives:** model + cache only; only the pure function.

### D-293 — `Context::shutdown` (gap 5)
- **Date:** 2026-11-13 · **Decided by:** Owner
- **Context:** shutting down from a handler needs a captured `Client` clone.
- **Decision:** `ctx.shutdown()`, the same as `Client::shutdown`: every shard closes and
  `login` returns `Ok(())`. The context holds a clone of the shutdown sender (no reference
  cycle).
- **Alternatives:** `ctx.client()` returning a client handle (through a `Weak`), as
  discord.js's `message.client`; leaving it to a captured clone.

### D-294 — The permission computation applies timeouts, as Discord does
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context (D-292):** Discord takes every permission except `VIEW_CHANNEL` and
  `READ_MESSAGE_HISTORY` from a timed-out member (owners and `ADMINISTRATOR` are exempt);
  discord.js's `GuildMember#permissions` ignores timeouts.
- **Decision:** applied. The cache and member methods compare the member's
  `communication_disabled_until` with the current time; the pure function in `djs-model`
  takes whether the member is timed out as an argument. The result is what Discord allows.
- **Alternatives:** ignore timeouts, as discord.js (the result may overstate a timed-out
  member's rights).

### D-295 — Channel permissions apply the two documented implicit denials
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context (D-292):** Discord ignores a member's other channel permissions without
  `VIEW_CHANNEL`, and `MENTION_EVERYONE`, `SEND_TTS_MESSAGES`, `ATTACH_FILES` and
  `EMBED_LINKS` without `SEND_MESSAGES`; the voice rule is described only by example ("such
  as `MANAGE_CHANNELS`"). discord.js applies none of them.
- **Decision:** the channel computation (`permissions_in`, `Cache::member_permissions_in`)
  removes the permissions that depend on `VIEW_CHANNEL` and `SEND_MESSAGES`; the voice rule
  is only documented, since Discord gives no complete list.
- **Alternatives:** Discord's pseudocode only (as discord.js); both a filtered and a raw
  method.

### D-296 — Shard processes run the same executable
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the manager starts shard processes from `std::env::current_exe()` with an
  environment variable that marks the process as a shard (with its shard IDs and the IPC
  address and token of D-285); the bot's `main` has one entry point that recognizes its
  role. One binary to build and deploy. Starting another executable stays possible as an
  option.
- **Alternatives:** a separate bot binary given by path, as discord.js's
  `new ShardingManager('./bot.js')`.

### D-297 — Shards per process: configurable, one by default
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** one shard per process by default, as discord.js; a `shards_per_process`
  option groups shards into one process (a cluster), so that a large bot does not run
  hundreds of processes, each with its own cache, runtime and REST client.
- **Alternatives:** always one per process; clusters sized automatically from the CPU count.

### D-298 — Crashed shard processes are restarted with a growing delay
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** by default the manager restarts a process that exits; after repeated quick
  failures the delay grows (for example 1 s, 2 s, 4 s, … up to a limit) so that a failing
  bot does not loop. Configurable and can be turned off; every exit is also an event for the
  manager's code.
- **Alternatives:** discord.js's fixed 500 ms respawn; no restart (an event only).

### D-299 — The manager is the IDENTIFY queue of every process
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** shard processes ask the manager for an IDENTIFY slot over the IPC of D-285,
  and the manager grants them by `max_concurrency` buckets, one per bucket every 5 s (what
  `LocalIdentifyQueue` does within one process). Starts are as fast as Discord allows and
  stay correct when a process is restarted later.
- **Alternatives:** starting processes one after another with a fixed delay (~5.5 s, as
  discord.js) and letting each identify on its own.

### D-300 — The entry point: recognize the role, then a normal `Client`
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** a function of `djs_rs::sharding` reads the environment of D-296. In a shard
  process it returns the process's assignment (its shards, the IPC connection), which is
  given to the `Client` builder; the bot is then written as usual. In the manager process it
  returns `None` and the code builds the manager, with full access to it and its events.
  ```rust,ignore
  match sharding::process()? {
      Some(assignment) => {
          let client = Client::builder(&token).cluster(assignment).build();
          client.on(Events::MessageCreate, …);
          client.login().await
      }
      None => ClusterManager::builder(&token).shards_per_process(1).build().spawn().await,
  }
  ```
- **Alternatives:** one `run(|client| …)` call that does both, with the manager's code given
  as options.

### D-301 — Phase 9's types are named after clusters
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `ClusterManager` (the manager, ≈ discord.js `ShardingManager`), `Cluster`
  (one process and its shards, ≈ `Shard`) and `ClusterClient`, reached with
  `client.cluster()` (≈ `ShardClientUtil` / `client.shard`), as in discord-hybrid-sharding:
  the names say that a process can hold several shards (D-297). The module and the feature
  stay `sharding` (D-284).
- **Alternatives:** discord.js's names (`ShardingManager`, `ShardProcess`, `ShardingClient`).

### D-302 — REST rate limits stay per process
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context:** the global limit (50 requests/s per token) and the route buckets are shared by
  every process of a bot, while each process has its own REST client.
- **Decision:** each process limits itself, as in discord.js; what goes over ends in a 429,
  which the REST client already waits out and retries. Documented; a shared limiter can be
  added later if measurements show the need.
- **Alternatives:** splitting the global budget (50/N per process); a limiter in the manager
  asked over IPC for every request.

### D-303 — Phase 9's order
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** one commit per step: (1) the IPC protocol, starting processes and assigning
  shards (D-285, D-296, D-297, D-300); (2) the manager's IDENTIFY queue (D-299); (3) restarts
  with a growing delay (D-298); (4) built-in values and named commands (D-286); (5)
  benchmarks and extreme-load tests (D-041).
- **Alternatives:** values and commands right after step 1.

### D-304 — Shard processes are run with tokio's `process` feature
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the manager starts, watches and kills its processes with
  `tokio::process::Command` (tokio's `process` feature, enabled only by `sharding`): waiting
  for an exit is asynchronous, in the same runtime. On Unix it adds `signal-hook-registry`
  (part of the tokio ecosystem, for `SIGCHLD`) to a bot's dependencies when `sharding` is on.
- **Alternatives:** `std::process` with one blocking thread per process waiting for its exit
  (no new crate; hundreds of threads for hundreds of processes).

### D-305 — The manager asks Get Gateway Bot once, for every process
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the `ClusterManager` always calls Get Gateway Bot once when it starts, also
  with a fixed shard count: it checks `session_start_limit.remaining` for all its shards at
  once, keeps `max_concurrency` for its IDENTIFY queue (D-299), and passes the gateway URL
  and `max_concurrency` to every process in its assignment. A cluster process does not call
  Get Gateway Bot: 1 request instead of N + 1 at start, none when a process is restarted.
- **Alternatives:** a `ClusterManagerBuilder::max_concurrency` option (fetched only with
  `Shards::Auto`, processes still asking for the URL); processes reporting `max_concurrency`
  with their IDENTIFY requests (N requests at start).

### D-306 — Which cluster exits are restarted, and how it is configured
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** by default the manager restarts a process only when it fails: a non-zero
  exit code or a signal (a panic, an error from `login`, an out-of-memory kill). A clean
  exit (code 0, for example after `ctx.shutdown()` in the process) means the bot ended that
  cluster on purpose; exits the manager asked for (`shutdown`) are never restarted. The
  policy is `ClusterManagerBuilder::restart(Restart)`, a non-exhaustive enum
  `Restart::{OnFailure (default), Always, Never}`, and the delays are
  `ClusterManagerBuilder::restart_delay(first, max)`: 1 s by default, doubled after each
  restart up to 60 s, back to the first delay once a process has run for at least the
  maximum delay. A restart is reported as `ClusterEvent::Restarting { cluster, delay }`
  before the process is started again.
- **Alternatives:** restarting every exit the manager did not ask for (discord.js's
  `respawn: true`); `restart(bool)` without a choice of exits; a `RestartPolicy` struct
  with every threshold configurable.

### D-307 — The manager and every cluster can query the clusters
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** built-in values and named commands (D-286) can be sent by the manager
  (`ClusterManager`) and by any cluster process (`ClusterClient`, from `client.cluster()`,
  D-301), to every cluster (`broadcast`) or to one (`send`); a cluster's requests go through
  the manager. This is what `client.shard.broadcastEval` / `fetchClientValues` do in
  discord.js, for example a `!stats` command that sums every cluster's guilds.
- **Alternatives:** only the manager sends requests.

### D-308 — Named commands are types implementing `ClusterCommand`
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** a command is a type implementing the trait `ClusterCommand` with
  `const NAME: &str`, `type Input` and `type Output` (both serde); a cluster handles it with
  `cluster.handle::<C>(|client, input| async move { … })` and anyone sends it with
  `broadcast::<C>(input)` / `send::<C>(cluster, input)`. The name and types are written
  once in the one executable both sides run, so they cannot disagree.
- **Alternatives:** names as strings with the types given at each call
  (`handle("name", …)`, `broadcast::<In, Out>("name", …)`).

### D-309 — A broadcast returns one result per cluster
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** a broadcast returns `Vec<ClusterReply<T>>`, one per cluster in ID order,
  each with its `cluster` and `result: Result<T, ClusterError>`: a cluster that is down or
  restarting fails alone and the others' results stay usable; all-or-nothing is
  `.collect::<Result<Vec<_>, _>>()`.
- **Alternatives:** `Result<Vec<T>, ClusterError>`, failing as a whole (discord.js).

### D-310 — The built-in values are one `ClusterStats` per cluster
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `stats()` (on the manager and on `ClusterClient`) returns, per cluster, a
  `ClusterStats`: the cluster's ID, its shards (each with its ID, state and heartbeat
  latency), the numbers of cached guilds and users (`None` without a cache), the client's
  uptime and the process's memory (resident set size; `None` where the system does not
  tell). One typed request for everything, cheap to compute.
- **Alternatives:** discord.js-style keys (`fetch_values(Value::Guilds)`), one value per
  request; no built-in values, only commands.

### D-311 — Phase 10: our own voice stack, with proven crates for the hard parts
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `djs-voice` implements the voice gateway, UDP/RTP, players and resources
  itself, shaped like `@discordjs/voice` and consistent with the rest of djs-rs (and
  benchmarked, D-041), and uses established crates for the specialised parts: DAVE
  (D-312), the Opus codec (D-313) and transport encryption (D-314). Since 2026-03-01
  Discord only accepts end-to-end encrypted (DAVE) calls, so DAVE is part of the first
  usable version.
- **Alternatives:** an adapter over songbird (serenity's voice library, ISC: fastest to a
  working voice, but its API and architecture, and a large dependency tree); everything
  from scratch, MLS/DAVE included (months of cryptography that is easy to get wrong).

### D-312 — DAVE through the `davey` crate
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the DAVE protocol (the MLS group, epoch transitions, sender keys, frame
  encryption) comes from `davey` (0.2, a Rust implementation built on openmls); djs-voice
  handles the DAVE voice gateway opcodes and feeds them to it. Being 0.x, its API may
  change: pin it and wrap it behind a private module.
- **Alternatives:** our own DAVE layer on `openmls`; a binding to Discord's C++ `libdave`
  (FFI, `unsafe`, a C++ toolchain).

### D-313 — Opus through `audiopus`
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** Opus encoding and decoding (48 kHz stereo) use `audiopus`, a binding to
  libopus built statically with the crate (a C compiler and cmake at build time, no system
  library; the `unsafe` is inside the dependency, as songbird does).
- **Alternatives:** `opus` 0.4 (usually links the system's libopus); no codec at first
  (only ready Opus packets until PCM is supported).

### D-314 — Transport encryption with RustCrypto
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the two AEAD modes Discord offers use RustCrypto, pure Rust:
  `chacha20poly1305` for `aead_xchacha20_poly1305_rtpsize` (always offered, required) and
  `aes-gcm` for `aead_aes256_gcm_rtpsize` (preferred when offered; AES-NI accelerated).
- **Alternatives:** `ring` (already in the tree through rustls) for AES-256-GCM, with
  RustCrypto only for XChaCha20, which `ring` lacks.

### D-315 — `djs-voice` is a standalone crate, plugged into djs-rs by a feature
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `djs-voice` works on its own through a small adapter trait (send Opcode 4
  Voice State Update through the main gateway; receive the bot's `VOICE_STATE_UPDATE` and
  `VOICE_SERVER_UPDATE`), as discord.js's adapters do; djs-rs gets a `voice` feature with
  `djs_rs::voice`, which plugs the adapter into `Client` and forwards those events. A bot
  without the feature does not compile DAVE or Opus.
- **Alternatives:** voice only as a module of djs-rs, without the adapter trait (not usable
  with djs-gateway alone).

### D-316 — The voice API is shaped like songbird's
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the public voice API follows songbird's concepts rather than
  `@discordjs/voice`'s: a call per guild with a queue of tracks and a handle per track
  (play, pause, stop, volume, track events), convenient for music bots. The names and the
  exact shape of each type are asked before each step (D-001), and the migration guide maps
  the `@discordjs/voice` concepts to them.
- **Alternatives:** `@discordjs/voice`'s shape (`joinVoiceChannel` → `VoiceConnection`,
  `AudioPlayer`, `AudioResource`, `subscribe`, with discord.js's states and names).

### D-317 — Audio inputs: FFmpeg, Ogg/Opus and PCM
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** a track plays any format or URL through FFmpeg run as a separate process
  (as prism-media does for discord.js; FFmpeg must be installed, no new Rust dependency),
  Ogg/Opus files and streams natively (no FFmpeg, no re-encoding), and raw 48 kHz PCM.
- **Alternatives:** decoding in Rust with `symphonia` (MP3, FLAC, AAC, … without FFmpeg; a
  large dependency); only Opus and PCM supplied by the bot.

### D-318 — Phase 10's order: connection → player → receiving → benchmarks
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** (1) the voice gateway (v8), UDP, IP discovery, transport encryption and
  DAVE, playing ready Ogg/Opus: the first working voice, tested live with the owner's
  go-ahead (D-275); (2) the player: tracks and their queue, PCM, Opus encoding, FFmpeg,
  volume; (3) receiving audio; (4) benchmarks and extreme-load tests (D-041).
- **Alternatives:** receiving before the player (for recording bots).

### D-319 — Voice names: `VoiceManager` and `Call`
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** songbird's concepts and names (`Call` for a guild's voice connection, and in
  step 2 `Track`, `TrackHandle`, `TrackQueue`), with the manager named descriptively and
  like `ClusterManager`: `djs_voice::VoiceManager`, reached in djs-rs as `client.voice()`.
- **Alternatives:** songbird's names verbatim (`Songbird` as the manager: another project's
  brand); discord.js names on songbird's shape (`VoiceConnection`, `AudioResource`).

### D-320 — `Call` is a cheap-clone handle
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `Call` holds an `Arc` (like `Client` and `ClusterManager`) and its methods
  take `&self` (`call.play(…)`, `call.leave().await`): the connection's state lives in its
  task, and the methods send it commands. The bot never locks anything.
- **Alternatives:** `Arc<Mutex<Call>>` as songbird returns, locked for each operation.

### D-321 — Voice events are streams
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `call.events()` returns a stream of `CallEvent` (connected, disconnected,
  reconnecting, …) and, in step 2, `track.events()` a stream of the track's events, as
  `client.events()` and `ClusterManager::events()` do: one pattern in the whole project.
- **Alternatives:** songbird's `EventHandler` trait registered with `add_global_event` /
  `add_event` (with periodic and delayed events).

### D-322 — `join` returns once the call can send audio
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `VoiceManager::join(guild, channel).await` returns the `Call` once the voice
  gateway, UDP and the DAVE session are ready to send, or an error (a timeout of 10 s,
  missing permissions, …), as songbird's `join`; connections lost later are repaired in the
  background and reported as events.
- **Alternatives:** returning at once after Opcode 4, the connection progressing in the
  background (audio buffered until `Connected`).

### D-323 — Step 1 plays through the first part of the final API
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** step 1 already has `Input` (only Ogg/Opus for now, from a file or any
  `AsyncRead`) and `call.play(input) -> TrackHandle` (play, pause, stop for now); step 2
  adds the queue, PCM, FFmpeg and volume to the same types.
- **Alternatives:** a raw `call.send_opus(packet)` (the bot keeps the 20 ms pace) kept as an
  advanced API; no playback API until step 2.

### D-324 — The gateway adapter is a trait plus `process`
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `VoiceManager::new(bot_user_id, shard_count, gateway)` takes
  `gateway: impl VoiceGateway`, a trait with one method that sends Opcode 4 through the
  shard of a guild; the client feeds the gateway's events with `manager.process(&event)`,
  which keeps only the bot's `VOICE_STATE_UPDATE` and the `VOICE_SERVER_UPDATE`s (as
  songbird does with twilight). djs-rs's `voice` feature does both by itself.
- **Alternatives:** channels (a receiver of "send Opcode 4" commands and a sender of
  updates), wired by the user.

### D-325 — Step 1's dependencies, and our own Ogg reader
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** `davey = "=0.2.0"` (pinned: 0.x), `chacha20poly1305 = "0.11"`,
  `aes-gcm = "0.11"`; the voice gateway uses the `tokio-websockets` already in the
  workspace. Ogg pages are read by a small parser of our own (a simple format, measured by a
  benchmark), not the `ogg` crate.
- **Alternatives:** the `ogg` crate (BSD-3) for the container.

### D-326 — MPL-2.0 allowed only for the `hpke-rs` crates under `davey`
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context:** `davey` 0.2 (D-312) builds on MSRV 1.89, but its MLS stack
  (`openmls_rust_crypto`) depends on `hpke-rs`, `hpke-rs-crypto`, `hpke-rs-libcrux` and
  `hpke-rs-rust-crypto`, licensed MPL-2.0 (file-level copyleft), which `deny.toml` did not
  allow. Used unmodified from crates.io, they leave djs-rs `MIT OR Apache-2.0`.
- **Decision:** `deny.toml` allows MPL-2.0 for exactly these four crates
  (`[[licenses.exceptions]]`); any other MPL crate is still refused. The djs-voice README and
  `NOTICE` say that the voice crate links MPL-2.0 code.
- **Alternatives:** MPL-2.0 for the whole tree; our own DAVE on another MLS stack
  (`mls-rs`) instead of `davey`.

### D-327 — `chacha20poly1305` and `aes-gcm` stay at 0.11
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context:** openmls uses the 0.10 lines, so both 0.10 and 0.11 are compiled (a few small
  crates twice; same speed).
- **Decision:** keep 0.11 (D-325); the duplicates go when openmls moves to 0.11.
- **Alternatives:** 0.10, shared with openmls.

### D-328 — The six RustSec advisories under `davey` 0.2 are ignored, with reasons
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context:** the workspace had no advisories; `davey` 0.2 → openmls 0.8 brings six that
  cargo-deny fails on, and no compatible update removes them (openmls 0.9.2 exists but needs
  Rust 1.91, and `davey` has not moved to it). RUSTSEC-2026-0387 (`crypto-bigint` 0.5.5,
  timing variability; reachable through P-256 signatures and ECDH in MLS, exploitable only
  with a precise side channel on the bot's machine); RUSTSEC-2026-0207 and -0208
  (`libcrux-sha3` 0.0.8, SHAKE; `hpke-rs` calls it only for the X-Wing and ML-KEM KEMs,
  which DAVE's P-256 suite does not use); RUSTSEC-2026-0212 (`libcrux-secrets` 0.0.5,
  swap/select on aarch64, not called); RUSTSEC-2024-0384 (`instant`) and RUSTSEC-2026-0441
  (`pin-utils`), unmaintained, through `fluvio-wasm-timer`. songbird has the same tree.
- **Decision:** `deny.toml` ignores exactly these six IDs, each with its reason. They are
  removed when `davey` moves to openmls 0.9 (which also needs an MSRV answer from the
  owner); an open question tracks it.
- **Alternatives:** do the voice steps without `davey` until it updates; port `davey` to
  openmls 0.9 ourselves (MSRV 1.91).

### D-329 — A seventh advisory ignored; the unused MPL-2.0 exception removed
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Context:** CI's cargo-deny found an advisory the local check (over the host's build
  graph) missed: RUSTSEC-2026-0173, `proc-macro-error2` 2.0.1 unmaintained, a helper of the
  procedural macros `hax-lib-macros` under libcrux (`hpke-rs` → openmls → `davey`). It runs
  only in the compiler and is not in a bot's binary; no fixed version exists. cargo-deny
  also warned that D-326's exception for `hpke-rs-libcrux` matches nothing: that crate is
  an optional dependency nobody enables (it is in `Cargo.lock`, not in the build graph).
- **Decision:** RUSTSEC-2026-0173 joins D-328's ignores, with its reason, and goes with
  them; the `hpke-rs-libcrux` exception is removed (if it ever enters the graph, cargo-deny
  says so and the owner is asked).
- **Alternatives:** remove `hax-lib-macros` from the tree (a patched libcrux).

### D-330 — djs-voice has its own copy of the TLS connector
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** the voice gateway's `wss://` connector (rustls with the ring provider,
  `webpki-roots`, D-030) is a copy of djs-gateway's small `tls.rs` in djs-voice, with the
  same workspace versions and its own `native-roots` feature (D-088, D-100), which djs-rs's
  `native-roots` feature also enables. The crates stay independent.
- **Alternatives:** djs-voice depends on djs-gateway, which makes its connector public;
  the connector moves to djs-util.

### D-331 — A lost voice connection is retried a few times, then reported
- **Date:** 2026-11-14 · **Decided by:** Owner
- **Decision:** after a close that allows resuming (4015, or a connection lost without a
  code) the call resumes at once, then tries up to 5 times with exponential delays (1, 2,
  4, 8, 16 s, with jitter); if all fail, a `CallEvent` reports the disconnection and the
  call ends (the bot may join again). After 4006 or 4009 it identifies anew once.
- **Alternatives:** retry forever, as the main gateway's shards do; never retry.

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

### C-082 — Command builder details (D-192–D-196)
- **Model constructors:** `SubCommandOption`, `StringOption`, `IntegerOption`,
  `NumberOption`, `BasicOption`, `ChannelOption` and `AttachmentOption` gained
  `new(name, description)` in `djs-model` (optional, no choices or bounds).
- **New `ValidationError` variants:** `InvalidName { field, character }` (the first
  character that is not allowed; the message suggests the lower-case form when there is one)
  and `Invalid { field, reason }` for rules between values (mixing subcommands with options,
  repeated names, required after optional, choices with autocomplete, `max_value` below
  `min_value`, a non-finite number). `min_length` / `max_length` out of range is
  `OutOfRange`, with `min` raised to `min_length` as Discord reports it. Blank descriptions
  and choice names are `Missing` (Discord: `BASE_TYPE_REQUIRED` / `BAD_LENGTH`).
- **Paths:** nested values are written like Discord writes them (`options[0].options[1].name`,
  `name_localizations.pl`) through a stack-built `check::Path`, formatted only on errors;
  embeds use the same `check::count` / `check::text` helpers.
- **Order of checks:** name, its translations, description, its translations, then per level
  the count, and for each option: its place (mixing / group / subcommand rules), a repeated
  name, required order, its own name, description and content; the 8000 total last.
- **Setters:** `name_localizations` / `description_localizations` add to those already set
  (discord.js replaces them); `contexts`, `integration_types`, `channel_types` and
  `file_types` replace. `option` takes `impl Into<CommandOption>`, so a `djs-model`
  `CommandOption` also works, and `subcommand` / `subcommand_group` are typed shortcuts for
  it. Every option type, subcommand and group also has `build()` (paths start at its fields,
  no 8000 total) and `build_unchecked()`. `SlashCommandAttachmentOption::file_types` exposes
  the model's `file_types` (no discord.js counterpart).
- **Not tried live, decided by the docs or by analogy:** integer limits ±(2^53 − 1) and number
  limits ±2^53 (the docs); string choice values may be empty and count trimmed toward 8000;
  integer and number choice values do not count toward 8000; context-menu names count
  trimmed code points and have no character restriction; a group without subcommands is not
  rejected.
- **Not offered:** a builder for `PRIMARY_ENTRY_POINT` commands (discord.js has none either),
  and starting a builder from a received `ApplicationCommand`.

### C-083 — Message component builder details (D-197–D-203)
- **Step 3 is split in three commits** following D-181's order: message components (buttons,
  select menus, action rows, `validate_components`), then Components V2, then modals.
- **Builders hold model values:** every component builder converts into `djs-model`'s
  `Component` (`From` impls), rows take `impl Into<Component>`, and the rules are checked on
  the model values (`component/validate.rs`), so builders and raw model components mix and a
  raw value gets the same checks. `check::Path` gained `Child` for single children
  (`components[1].accessory`).
- **`validate_components(&[Component], ComponentsVersion)`:** the second argument is a new
  enum `ComponentsVersion { Legacy, V2 }` (`#[non_exhaustive]`). Legacy: at most 5 top-level
  components, all action rows. V2: only action rows, text displays, sections, media
  galleries, files, separators and containers at the top level; 40 components counting
  nested ones (section accessories too); 4000 code points across text displays (trimmed);
  unique `custom_id`s in both. The V2 total counts only text display content — whether
  Discord also counts labels or placeholders as "displayable text" was not tried.
- **New error variant:** `ValidationError::TooFew { field, min, actual }` (an empty row,
  fewer options than `max_values`, fewer default values than `min_values`).
- **Select menus:** `min_values` / `max_values` default to 1 as on Discord; `min_values`
  above `max_values` is `OutOfRange` on `min_values` (as Discord reports it); default values
  must number within `min_values..=max_values` (the docs) and match the menu's type; option
  values follow the `custom_id` rule (1-100, not trimmed) — not tried live; `default_users`,
  `default_roles`, `default_channels` add; `channel_types` replaces; `required` (modals) is
  offered on every menu, as in discord.js.
- **Buttons:** an interactive button whose style is link or premium (only possible with a raw
  model value) is `Invalid`; a blank label with an emoji is accepted (not tried live); a link
  URL is checked for at most 512 characters after trimming and its scheme (any case).
  `PremiumButtonBuilder::build` returns `Result` for uniformity although it cannot fail.
- **Row children:** a text input alone in a row was accepted (legacy modals) — superseded by
  D-207 / C-085: a text input in a row is now `Invalid`.

### C-084 — Components V2 builder details (D-204, D-205)
- **Thumbnails and gallery items** follow D-190 by analogy: `http://`, `https://` or
  `attachment://` (any case), although Discord accepted an `ftp://` thumbnail (D-205) — the
  owner may want to revisit this; file components accept `attachment://` only, as Discord
  requires.
- **Texts:** a text display has 1-4000 code points after trimming; a media description, when
  set, 1-1024 (blank is `Missing`, by analogy with Discord's 1-1024 range).
- **Sections** hold text displays only (the docs list no other child yet), 1-3 of them; the
  accessory must be a button or a thumbnail. `SectionBuilder` has `text` and `component`
  (no `components(iter)` for at most three children).
- **Containers** hold action rows, text displays, sections, media galleries, separators and
  files, 1-40 of them, with an accent color up to `0xFFFFFF`; their `build()` also checks
  that `custom_id`s inside are unique (the walk shared with `validate_components`,
  `component::validate::Totals`). No `accent_color_rgb` (not asked for; embeds have
  `color_rgb`).
- **Paths** of media URLs are written like Discord's: `media.url`, `items[1].media.url`,
  `file.url`, `accessory.label`.
- **Shared macro:** `component_builder!` gives every V2 builder `id`, `build`,
  `build_unchecked` and `From<…> for Component`.

### C-085 — Modal builder details (D-206, D-207)
- **Rules from the documentation only** (a modal needs a user's interaction, D-200): title
  1-45 code points after trimming; 1-5 top-level components, labels or text displays only;
  label 1-45, label description at most 100; a label holds a text input, a select menu, a file
  upload, a radio group, a checkbox group or a checkbox.
- **Fields in a modal:** a select menu cannot be `disabled`, and a required field (select menu,
  file upload, checkbox group) with `min_values` 0 is `Invalid` — both checked only when the
  field sits in a label, so a disabled menu still builds for a message.
- **Text inputs:** `min_length` 0-4000, `max_length` from `max(1, min_length)` to 4000
  (`OutOfRange` on `max_length`), value at most 4000 and placeholder at most 100 code points
  after trimming.
- **File uploads:** `min_values` 0-10, `max_values` 1-10 (default 1), at most 10 file types;
  `file_types` replaces (like `channel_types`).
- **Radio groups:** 2-10 options, at most one `default` (the second one is reported at
  `options[i].default`). **Checkbox groups:** 1-10 options, `max_values` defaults to the
  number of options and needs as many options. Group option labels 1-100, values follow the
  `custom_id` rule (1-100, not trimmed) and are unique in the group, descriptions at most 100.
- **Option builders:** `RadioGroupOptionBuilder` and `CheckboxGroupOptionBuilder` are two
  types with the same methods (one macro), named after discord.js; `options(iter)` takes
  either or `djs-model` `GroupOption`s.
- **Unique `custom_id`s:** the fields' `custom_id`s are unique in the modal (the shared
  `Totals` walk, which now descends into labels); the modal's own `custom_id` is separate.
  The error keeps the wording "another component of the message has the same custom_id".
- **Action rows:** text inputs no longer count as a row's single wide child, so the row
  messages are now "a select menu fills a whole action row" and "an action row can only hold
  buttons or one select menu".
- `CheckboxBuilder` has no `required` (the model has none: a checkbox is never required).

### C-086 — Builder benchmarks and the optimizations they led to (Phase 6 step 4)
- **Layout:** `crates/djs-builders/benches/builders.rs` (divan with the allocation profiler,
  like the other crates; `divan` added as a dev-dependency of `djs-builders`, already a
  workspace dependency) measures each builder's `build()`, `build_unchecked()` and assembly;
  `tests/stress.rs` holds the ignored load tests (all cores, huge inputs) and three quick
  tests (maximum values are valid, nested trees, builder sizes).
- **Repeated `custom_id`s and option values** are found with a private set
  (`component::validate::Values<N>`): the first `N` (40 `custom_id`s, 25 menu or 10 group
  values) in stack arrays with a fingerprint each — the length and the first and last 8
  bytes — and a `Vec` beyond. Bytes are compared only when fingerprints match, so the result
  is exact.
- **`validate_components` (V2) stops comparing `custom_id`s past 40 components**: the list is
  rejected for its size, and comparing every `custom_id` of a huge list took quadratic time.
  The other components are still checked and counted, so `TooMany` reports the real count;
  a repeated `custom_id` after the 40th component is no longer reported (the size error is).
- **Command names:** an ASCII shortcut in the name check, with a unit test that it agrees
  with the full check on all 128 ASCII characters.

### C-087 — Formatter details (D-211–D-213)
- **Names:** discord.js' names in snake_case, except `slash_command_mention`
  (`chatInputApplicationCommandMention`) and `emoji` (`formatEmoji`), as in the D-213
  preview; `underscore` (deprecated alias of `underline`) is left out. Timestamp styles are
  named after Discord's docs (`MediumTime`, `LongDateShortTime`, `FullDateShortTime`, …),
  without discord.js' deprecated aliases. The game-profile mention `<@$GAME_ID>` from
  Discord's docs is not offered (no game ID type in `djs-model`, not in discord.js).
- **Time input:** a public trait `UnixSeconds` (implemented for `i64` seconds,
  `djs_model::util::Timestamp`, `SystemTime` and references), so other crates' time types can
  be passed; fractions round down, also before 1970.
- **`Display` output ignores width and fill flags** (`{:>10}`): the parts are written with
  `write!`, so a flag never pads an ID inside a mention.
- **Lists:** flat lists take any `Display` items; nesting uses `ListItem::{Text, Nested}`. The
  bound is a sealed trait `ListEntry`. Ordered lists number each line (discord.js repeats the
  first number; Discord shows both alike), nested ordered lists count from 1, `start(0)` is 1.
- **`phone_number` returns `Option`** (`None` without a leading `+`; discord.js throws).
- **`email` headers** are `&[(&str, &str)]`; names are lower-cased (ASCII) and both parts
  encoded like `URLSearchParams` (`application/x-www-form-urlencoded`).
- **Escaping:** the discord.js option `escape` is `EscapeOptions::backslash` (`escape` is the
  method that escapes text); the single escaper keeps the name `escape_escape`.
  `EscapeOptions::none()` is added as a start for a few kinds; the single escapers use the
  same link and token rules as `escape_markdown`. A link runs to the next ASCII whitespace;
  a `<…>` token is protected when it is on one line without another `<`, and either starts
  with `/` and ends with `:` + digits (command mention, spaces allowed) or has no whitespace
  and starts with `@`, `#` or `+` or contains `:` or `@`.
- **`djs_rs::formatters`** re-exports the crate, like the other layers.

### C-088 — Formatter benchmarks and the escaping scanner (Phase 7 step 1b)
- **Layout:** `crates/djs-formatters/benches/formatters.rs` (divan with the allocation
  profiler and byte counters; `divan` added as a dev-dependency, already a workspace
  dependency) and `tests/stress.rs` (two quick randomized invariant tests with an in-test
  xorshift generator, and ignored load tests).
- **Scanner:** skips to the next interesting byte with a static 256-entry table (a second
  table adds `\n`, `#`, `-`, `.` when a line option is on); a link is recognized at its `:`
  by looking back for `http` / `https` (ASCII case-insensitive) and forward for `://` plus a
  non-space byte. Line starts are only tracked when a line option is on.

### C-089 — `Collection` details (D-209, D-210, D-214–D-217)
- **Callback arguments** follow discord.js' order: value first, key second
  (`find(|value, key| …)`, `sort_by(|a, b, key_a, key_b| …)`); `map_values` passes the value
  by value and the key by reference, `flat_map` both by value.
- **Left out or folded in:** the `amount` overloads of `first` / `last` (use
  `.values().take(n)` / `.rev()`), `firstKey(n)` and the like; `random(amount)` is
  `random_n(count)` (distinct values in random order); discord.js' default `sort()` compares
  values as strings, ours needs `V: Ord`; `toJSON` (no serde dependency yet); `clone` and
  `equals` are `Clone` and `PartialEq` (order-insensitive, as `equals`).
- **Added for Rust:** `retain`, `index_of`, `get_key_value`, `remove_entry`, `swap_remove`
  (D-215), `into_keys`, `into_values`, `values_mut`, `iter_mut`, `Index<&Q>` (panics on a
  missing key, like `HashMap`), `From<[(K, V); N]>`, `FromIterator`, `Extend`; own iterator
  types in `djs_collection::iter` (D-217); lookups take `&Q` where `K: Borrow<Q>` (std's
  convention, so `indexmap`'s `Equivalent` stays out of the API).
- **Randomness:** std's `RandomState` seeds each call (C-090), and the indexes are mapped
  with a multiply-shift; `random_n` shuffles all indexes partially when it takes a quarter or
  more, otherwise samples with Floyd's algorithm and shuffles the picks. Not for
  unpredictability.
- **Hashers:** transformations that build a new map (`partition`, `map_values`, `merge`)
  need `S: Clone` to reuse the hasher; `combine_entries` and `flat_map` need `S: Default`.
- **`djs_rs::collection`** re-exports the crate, like the other layers.


### C-090 — `djs-collection` benchmarks, stress tests and two optimizations (D-041)
- **Benchmarks** (`benches/collection.rs`, divan with `AllocProfiler`): `get` / build /
  iterate against `IndexMap` + foldhash, `HashMap` + foldhash and `HashMap` + SipHash at 100 /
  10 000 / 1 000 000 entries; `remove` vs `swap_remove` vs `sweep` of the front tenth; every
  helper on 10 000 entries. `indexmap` is also a dev-dependency (already a dependency).
- **Stress tests** (`tests/stress.rs`): a model check of random operations against a `Vec`
  of entries, distinctness and fairness of the random picks (both quick, run in CI), and
  ignored load tests: 10 000 000 entries with resident memory per entry from
  `/proc/self/status`, adversarial removal patterns, all cores reading one
  `Arc<Collection>`.
- **Random picks:** one seed per call from std's `RandomState`, then a wyrand generator
  (a 64-bit add and a 128-bit multiply per number), instead of hashing a fresh
  `RandomState` for every index; Floyd's sampling keeps its picks in a foldhash set. Still
  no dependency (D-210) and still not for unpredictability.
- **`partition`** moves the passing entries out with `IndexMap::extract_if` (the others stay
  in the original table, unhashed) instead of reinserting every entry into two new maps.
- **`remove`'s rustdoc** now points to one `sweep` / `retain` pass for removing many entries,
  after the stress test showed 1 000 front removals of a million taking seconds (D-215 keeps
  `remove` order-preserving).

### C-091 — Phase 8 events: details of D-221–D-228
- **Names:** the keys of discord.js' `Events` object (`GuildRoleUpdate`, `GuildEmojiCreate`,
  `MessageBulkDelete`), with its string values as `Event::NAME` (`"roleUpdate"`,
  `"emojiCreate"`, `"messageDeleteBulk"`). Gateway dispatches without a discord.js event get an
  event named after the dispatch: `ChannelInfo`, `IntegrationCreate` / `Update` / `Delete`,
  `VoiceChannelStatusUpdate`, `VoiceChannelStartTimeUpdate`, `RateLimited`, and the whole-list
  `GuildEmojisUpdate` / `GuildStickersUpdate` (D-223), so every dispatch is typed.
- **No `old` where the cache keeps nothing:** `AutoModerationRuleUpdate`,
  `EntitlementUpdate`, `SubscriptionUpdate` and `ThreadMemberUpdate` keep their model payload
  (an `old` would always be `None`); reactions keep theirs (D-227).
- **Removals:** `ChannelDelete`, `StageInstanceDelete`, `GuildScheduledEventDelete` and
  `GuildSoundboardSoundDelete` also carry `old` (discord.js hands the removed object).
  `MessageBulkDelete::old` is a `Vec` (empty without a message cache). `GuildUnavailable::old`
  is the guild the cache still holds (it keeps unavailable guilds).
- **Derived items:** `GuildEmojiCreate` / `Delete` hold `guild_id` + `emoji` (the new one, or
  the cached `Arc`), `GuildEmojiUpdate` holds `guild_id`, `old: Arc<Emoji>` and `new`; the
  sticker ones likewise. Order: the list event, then each new item in order (created or
  updated), then the deleted ones (discord.js' order).
- **Not emitted:** `GuildMemberAvailable` (discord.js emits it when its member cache lacked a
  member, an artefact of its caching), `Debug`, `Warn`, `Error`, `CacheSweep` (D-226).
- **Lifecycle payloads:** `ShardDisconnect` hands the `CloseCode`, `ShardError` the
  `ReceiveError`, `ShardResume` the number of replayed events (dispatches received while
  resuming); `ShardReconnecting` and `Invalidated` are unit structs. `ShardReconnecting` fires
  at the first event after the connection was lost (at the latest the new hello), as the
  shard reconnects inside `next_event`. `Invalidated` marks an `INVALID_SESSION` that cannot
  be resumed (discord.js v14 lists the event but no longer emits it).
- **`ClientReady`'s payload** is the `READY` of the last shard to become ready, once per
  `login`; later re-identifies only emit `ShardReady`.
- **`Raw`** is built (a clone of the dispatch) only when a `Raw` handler is registered, and is
  never sent to `events()` streams; unknown dispatches reach streams as `Dispatch::Unknown`.
  No payload is built for an event nobody listens to.

### C-092 — Phase 8 event benchmarks (D-041)
- The dispatch path is measured end to end with the existing `Client` floods
  (`djs-rs/tests/stress.rs`), extended with `MESSAGE_UPDATE` floods (100 messages created
  first, so every update has a previous state; the test prints how many had one), a `Raw`
  handler, and a flood nobody listens to (one handler waits for a final `MESSAGE_UPDATE`).
- The A/B builds the previous commit in a scratch `git worktree` and alternates runs pinned to
  `taskset -c 0,2` (C-072); `perf record -e cycles:u` works without root
  (`perf_event_paranoid` = 2) and compares the profiles.

### C-093 — Phase 8 managers and structure methods: details of D-229–D-233
- **Managers** (`djs_rs::manager`) are `Copy` values holding `&Context` (and the guild or
  channel ID); their methods consume `self`, so `ctx.members(guild).kick(user, None).await`
  needs no binding. Each has `cache()` (the cache view, `None` without a cache), and the
  scoped ones `guild_id()` / `channel_id()`.
- **`fetch(id)` returns a `Fetch` builder** that reads the cache when `fetch` is called
  (not when awaited), has `force()` and `is_cached()`, and implements `IntoFuture`: a cache
  hit allocates nothing, the REST future is boxed only when it runs. `GuildManager::fetch`
  asks Discord `with_counts`, so a guild the cache does not hold gets its approximate member
  count.
- **Naming**: lists are uniformly `list` (discord.js overloads `fetch()` without an ID);
  actions take discord.js' names in snake case (`kick`, `ban`, `unban`, `timeout`,
  `set_nickname`, `add_role`, `remove_role`, `send`, `edit`, `delete`, `bulk_delete`,
  `react`, `pin`, `unpin`, `start_thread`, `crosspost`, `send_typing`, `create_dm`).
  Audit-log reasons are a trailing `Option<&str>` where Discord accepts one.
- **Created and edited objects go through `Cache::store_*` too**: `send`, `edit`,
  `crosspost`, member edits, `create_dm` and `start_thread` return the cached type in an
  `Arc`, like `fetch`. **Breaking**: `MessageExt::reply` returns `Arc<CachedMessage>`
  instead of `Message`.
- **Bans** are never cached: `BanManager` returns the `djs-model` `Ban`, and its `fetch`
  always asks Discord (`is_cached()` is `false`). `MemberManager::ban` / `unban` delegate to
  it (discord.js' `GuildMemberManager#ban`).
- **Extension traits** (`MessageExt`, `ChannelExt`, `UserExt`, `MemberExt`, `GuildExt`) have
  default methods over a sealed trait giving the IDs, delegate to the managers, and are
  implemented for the model and the cached types. `MemberExt` is implemented for
  `CachedMember` and the `GuildMemberAdd` / `GuildMemberUpdate` payloads, because the model
  `Member` does not carry its guild. `GuildExt::leave` calls Leave Guild directly.
- **`UserManager::send` opens the DM channel every time** (Create DM; Discord returns the
  existing channel): the cache has no user → DM channel index to skip the request.
- Tests: `tests/managers.rs` — a routing mock REST API and a gateway that only sends `READY`;
  the `ClientReady` handler hands its `Context` to the test.

### C-094 — Phase 8 step 2c batch 1: writes on the first resources (D-234–D-236)
- **`ChannelManager`** (by ID): `edit` (Modify Channel), `edit_thread` (Modify Thread),
  `delete` — returns the model `Channel` Discord sends back, not stored (D-236) —
  `set_permissions` / `delete_permissions` (D-235). The target is `OverwriteTarget`
  (`Role(RoleId)` / `Member(UserId)`, `From` both ID types; `@everyone` is
  `guild_id.cast()`); the overwrite methods convert the target before the request starts, so
  their futures are `Send` whatever the argument type.
- **`GuildChannelManager`** (`ctx.guild_channels(guild)`, `GuildExt::channels`): `list`
  (Get Guild Channels, cached), `create`, `set_positions`. It has no `cache()`: the cache has
  no per-guild channel view (`ctx.channels().cache()` and `in_guild` list them).
- **`RoleManager`**: `create`, `edit`, `delete`, `set_positions` (Discord returns every role:
  a `Collection`, cached). No `RoleExt`: the model `Role` does not know its guild.
- **`GuildManager`**: `edit`, `leave`; `GuildExt::edit` and `GuildExt::leave` delegate.
- **`MemberManager`**: `search`, `edit_me` (discord.js `editMe`), `prune_count`
  (discord.js `prune({ dry: true })`) and `prune`.
- **`MessageManager`** reactions: `reaction_users` (takes a `ReactionEmoji`; the users are
  cached), `remove_reaction(message, emoji, Option<UserId>)` (`None`: the bot's own),
  `remove_emoji_reactions`, `remove_all_reactions`; `MessageExt` gained `remove_reaction`
  and `remove_all_reactions`, `ChannelExt` gained `edit`, `delete`, `set_permissions` and
  `delete_permissions`.
- Left for later batches: Add Guild Member (OAuth2), Modify Group DM, voice states.

### C-095 — Phase 8 step 2c batch 2: emojis, stickers, invites, webhooks (D-237–D-241)
- **`EmojiManager` / `StickerManager`** (`ctx.emojis(guild)`, `ctx.stickers(guild)`,
  `GuildExt::emojis` / `stickers`) work like `RoleManager`: `cache`, `fetch`, `list`,
  `create`, `edit`, `delete`. `djs-cache` gained `Cache::store_emoji` / `store_sticker`.
  Standard stickers (Get Sticker, sticker packs) and application emojis wait for batch 4,
  which brings the application ID with the application commands.
- **Uncached `fetch`** (invites, webhooks) returns a `Fetch` too, never cached, like bans,
  so every `fetch` reads the same; `InviteManager::fetch` borrows the code and the query.
- **Invite lists are keyed by the code as a `String`**: `djs-model` does not re-export
  `CompactString` and `djs-rs` does not depend on `compact_str`.
- **`WebhookManager`**: the methods using the webhook's token take `token: &str`; `send`
  waits for the message (`?wait=true`) and caches it, as do `fetch_message` and
  `edit_message`; `fetch_with_token` reads a webhook of another application.
- **`WebhookExt`** has `send` (`WebhookError`), `edit` and `delete` (D-237); a webhook's
  messages go through the manager.
- **`WebhookClient`** lives in `djs_rs::webhook` (with `WebhookError` and
  `InvalidWebhookUrl`) and is re-exported at the crate root like `Client`; `rest(Rest)`
  swaps its REST client (proxy, tests); it returns model `Message`s (no cache);
  `from_url` accepts only `https://` URLs.
- Tests (`tests/managers.rs`) check that webhook-token routes carry no `Authorization`
  header (D-121) for the bot's webhooks and for `WebhookClient`.

### C-096 — Phase 8 step 2c batch 3: threads, scheduled events, stages, soundboard (D-242, D-243)
- **`ThreadManager`** (`ctx.threads(parent)`): `create` (Start Thread without Message),
  `create_in_forum` (returns the thread; the starter message, whose ID is the thread's, is
  cached too), `list_public_archived`, `list_private_archived`,
  `list_joined_private_archived`. **`ThreadMemberManager`** (`ctx.thread_members(thread)`):
  `fetch(user, &GetThreadMember)` (never cached, a `Fetch` like the other uncached ones),
  `list` (keyed by user ID; entries without one are skipped), `add`, `remove`, `join`,
  `leave`. Neither has `cache()`: threads are cached per guild, not per parent.
- **`FetchedThreads`** (discord.js' name): `threads` as a cached `Collection`, the bot's
  `members`, `has_more`; returned by the archived lists and by
  `GuildChannelManager::list_active_threads` (`has_more` always `false`).
- **`ScheduledEventManager`** (`ctx.scheduled_events(guild)`): `cache`, `fetch`, `list` —
  both ask for `with_user_count` — `create`, `edit`, `delete` (Discord takes no reason),
  `users` (the model `GuildScheduledEventUser`, keyed by user ID, not cached).
- **`StageInstanceManager`** (`ctx.stage_instances(guild)`, guild-scoped like discord.js so
  the cache can be read): methods take the stage *channel*; `fetch(channel)` finds the
  cached stage by scanning the guild's stages.
- **`SoundboardSoundManager`** (`ctx.soundboard_sounds(guild)`): like the emoji manager.
  `ctx.default_soundboard_sounds()` returns the uncached model sounds keyed by ID (D-243);
  `ChannelManager::send_soundboard_sound` / `ChannelExt::send_soundboard_sound` play one.
- `djs-cache` gained `store_scheduled_event`, `store_stage_instance` and
  `store_soundboard_sound`. Shortcuts: `ChannelExt::threads` / `thread_members`,
  `GuildExt::scheduled_events` / `stage_instances` / `soundboard_sounds`.

### C-097 — Phase 8 step 2c batch 4: commands, auto moderation, audit log (D-244–D-247)
- **The application ID** (D-245): the client and every shard's `Context` share an
  `Arc<OnceLock<ApplicationId>>`, set at each shard's `READY` (the first one wins; the bot
  is the same application on every shard and login). The `READY` handling moved into
  `Client::on_ready`.
- **`CommandManager`** (`ctx.commands()`) and **`GuildCommandManager`**
  (`ctx.guild_commands(guild)`, `GuildExt::commands`): `fetch` (an uncached `Fetch`),
  `list(&GetApplicationCommands)`, `create`, `edit`, `delete`, `set` (bulk overwrite; lists
  are keyed by command ID). The guild manager also reads `permissions()` (keyed by the
  command ID, or the application's ID for application-wide permissions) and
  `command_permissions(command)`; editing them needs a user's `OAuth2` token, so it stays on
  `Rest`. The accessors are `#[track_caller]` and panic before `READY` (D-247).
- **`ApplicationEmojiManager`** (`ctx.application_emojis()`, panics before `READY`):
  `fetch`, `list`, `create`, `edit`, `delete`; not cached.
- **`AutoModerationRuleManager`** (`ctx.auto_moderation_rules(guild)`,
  `GuildExt::auto_moderation_rules`): `fetch`, `list`, `create`, `edit`, `delete`; not
  cached.
- **`GuildManager`** (D-246): `audit_log(guild, &GetGuildAuditLog)` returns the model
  `AuditLog`; `integrations(guild)` a `Collection` by integration ID; `delete_integration`.
- **Standard stickers** follow D-243: `ctx.sticker(id)`, `ctx.sticker_packs()`,
  `ctx.sticker_pack(id)`, not cached.

### C-098 — Phase 8 manager benchmarks (D-041)
- `djs-rs` gained `divan` as a dev-dependency (already a workspace dependency used by the
  other crates' benches, as in C-090) for `benches/managers.rs`: the cache path, polled once
  with a no-op waker (`Waker::noop`, stable since 1.85) so no runtime is measured, with
  `AllocProfiler` counting allocations; a client connected to a local mock gateway provides
  the `Context`.
- `djs-cache/benches/cache.rs` gained a `store` group (`store_user`, `store_member`,
  `store_message`).
- `djs-rs/tests/stress_managers.rs` (ignored): cache-hit floods on 1 to 48 tasks, REST
  floods with 64 requests in flight against the bare `Rest` calls, and 200 000 members
  stored from REST with the RSS per member. The mock REST API runs in-process with
  `global_requests_per_second(0)`.

### C-099 — Phase 8 step 3a: interaction getters (D-249, D-250)
- The getters live in `djs-model/src/interaction/getters.rs` as inherent impls of
  `CommandData`, `ComponentData` and `ModalSubmitData`; every one returns `Option` (`None`
  for a missing option or component, another type, or missing resolved data) — never a
  panic, where discord.js throws for a wrong type or a missing required option.
- Command getters search the options of the subcommand that was run (`leaf_options`, the
  hoisted options of discord.js). As in discord.js, `get_user` / `get_member` accept user and
  mentionable options and `get_role` role and mentionable options; `get_mentionable` takes
  mentionable options only. `get_number` reads number options only (not integers).
- A member is returned as `(&User, &InteractionMember)`: resolved members have no user, so the
  pair saves a second lookup (like discord.js' `GuildMember#user`). `get_member` and the
  member iterators skip users who are not members.
- New public type `Mentionable<'a>` (`User { user, member }` or `Role`) for mentionable
  options and selections, with `id()`.
- Extra getters beyond the D-250 list: `get(name)` (any type), `focused_value()`,
  `target_user()` / `target_member()` / `target_message()` of context menu commands.
- `ComponentData::{users, members, roles, channels, mentionables}` follow the order of
  `values`; modal getters are named after discord.js' `ModalSubmitFields`
  (`string_select_values`, `checkbox_group_values`, `radio_group_value`, `checkbox_value`,
  `selected_users` / `_members` / `_roles` / `_channels` / `_mentionables`,
  `uploaded_files`) and return `Option<impl Iterator>` for selections, not tied to the
  lifetime of the custom ID argument.
- `Interaction::command()` returns only command interactions (D-249: one kind per accessor);
  the older `InteractionData::command()` keeps covering autocomplete too.

### C-100 — Phase 8 step 3b: `InteractionExt` (D-248)
- `InteractionExt` is implemented for the model `Interaction` only; the `InteractionCreate`
  payload reaches it by auto-deref (it dereferences to the interaction), so no second impl.
- Initial responses: `reply`, `defer_reply(ctx, flags)`, `update`, `defer_update`,
  `show_modal(ctx, ModalCallbackData)`, `respond(ctx, AutocompleteChoices)`,
  `launch_activity`; they return `()` (Discord answers 204 without `with_response`), and
  `fetch_reply` gets the message when needed. `defer_reply` takes `MessageFlags`
  (`EPHEMERAL` or empty) rather than a `bool`. `Pong` (HTTP endpoints only) and the
  deprecated `PremiumRequired` have no method.
- Original response and follow-ups: `fetch_reply`, `edit_reply`, `delete_reply`,
  `follow_up`, `fetch_follow_up`, `edit_follow_up`, `delete_follow_up` (discord.js'
  `followUp` / `editReply({ message })` …). Messages they return are cached and returned as
  `Arc<CachedMessage>` (D-233), ephemeral ones included.
- `reply`, `update` take `impl Into<MessageCallbackData>`, `edit_reply` / `edit_follow_up`
  `impl Into<EditWebhookMessage>` and `follow_up` `impl Into<CreateFollowupMessage>` (all
  accept `&str`), converted eagerly so the futures stay `Send`; `edit_reply` is how a
  deferred reply is sent, so it takes text like a send.
- Tests: the interaction routes join the mock REST API of `tests/managers.rs`, and its mock
  gateway can send dispatches after `READY` (`mock_gateway_with`), for an end-to-end
  `INTERACTION_CREATE` → handler → `reply` test.

### C-101 — Phase 8 step 3c: the interaction router (D-251–D-255)
- `djs_rs::router` holds the wrappers (`CommandInteraction`, `AutocompleteInteraction`,
  `ComponentInteraction`, `ModalInteraction`; not in the prelude, closures infer them) with
  `interaction()`, `event()` and, for components and modals, `args()`. Their `Debug` prints
  the interaction (token redacted). `Deref` to the data matches the variant, with an
  `unreachable!` for the kind the constructor already checked.
- Registration: `Client::{command, autocomplete, component, modal}(name, handler)`, next to
  `on`. Registering a name again replaces its handler. A component or modal route name
  containing `:` panics (`#[track_caller]`, `# Panics`), as it could never match.
  Autocomplete has its own routes, matched like commands.
- The routes live in the copy-on-write handler `Registry`, so dispatching takes no lock;
  `InteractionCreate` is decoded when there are routes even without an event handler. The
  route handlers run after the `InteractionCreate` handlers (in sequential mode, after them in
  order) and their errors are logged with a `route` field.
- Command lookup tries the full path (`mod config set`), then the group (`mod config`), then
  the command (`mod`); the key is joined in a 256-byte stack buffer (the heap beyond), so a
  routed interaction allocates nothing more than the handler's own future. Commands of
  different types sharing a name share the route (the handler reads `kind`). Route names
  are compared verbatim (no case or whitespace normalisation).

### C-102 — Phase 8 step 3d: interaction benchmarks and stress tests (D-041)
- `djs-model/benches/interaction.rs` (new `[[bench]]`): the getters on realistic payloads
  and the decoding of the stress test's `INTERACTION_CREATE`.
- `djs-gateway/tests/stress.rs` gained `flood_1m_interaction_creates_plain` (shard CPU per
  interaction); `djs-rs/tests/stress_interactions.rs` (ignored) floods 1M interactions
  through an event handler, the router (A/B with the same handler work), both, sequential
  mode and 16 shards, and 100k–200k replies through a mock REST API (sequential, 64 in
  flight with a semaphore, 16 shards); `stress_interactions_mimalloc.rs` runs the same with
  mimalloc (D-080), like `stress_mimalloc.rs`.
- The `Box<V>` prototype of `Resolved` was measured locally and reverted; it is asked as a
  change of D-058, not applied.

### C-103 — Phase 8 step 4a: the collector core and the four collectors (D-263–D-269)
- `djs_rs::collector` (public module): `Collector<T>` (`Stream` + `next`, `stop`,
  `reset_timer`, `end_reason`, `is_ended`, `collected`, `processed`), one generic
  `CollectorBuilder<T, M>` whose mode marker (`mode::Stream` → `build()`, `mode::First` /
  `mode::All` → `IntoFuture`) avoids three builder types; the futures `AwaitFirst` and
  `AwaitAll`; `EndReason` (discord.js' reasons plus `Stopped` for `stop()` and `Shutdown`
  when the client stops), `CollectorError`, `Collected`, `ReactionEvent`
  (`Added` / `Removed` / `RemovedAll`), `CollectedReaction`, `reaction_key` (custom emoji ID
  or the Unicode emoji, discord.js' key), the sealed `Collectable`.
- Items: `Arc<MessageCreate>` (the handlers' payload), the router's `ComponentInteraction` and
  `ModalInteraction`, `ReactionEvent`. Options per kind as in discord.js: `max_users` and
  `component_type` for components, `dispose`, `max_users`, `max_emojis` for reactions;
  `time`, `idle`, `max`, `max_processed`, `filter` (an `FnMut`, run where the collector is
  polled) for all.
- Entry points: the extension methods of D-269 on `Message` / `CachedMessage` (reactions and
  components of the message), `Channel` (messages and components of the channel) and every
  interaction type (components of its reply, keyed by the interaction through
  `message.interaction_metadata`, or by the clicked message for a component interaction as
  discord.js' `update` does; `await_modal_submit(ctx, time)` with `time` required as in
  discord.js, keyed by the user); and `Context::{message_collector, await_messages,
  component_collector, await_component}` taking a channel ID, for handlers that hold only
  IDs (the `ctx.messages(channel)` pattern).
- The index (`collector/index.rs`): one `Mutex` over maps by raw 64-bit ID with a private
  Fibonacci hasher (snowflakes are unique numbers; no new dependency), an atomic entry count
  checked first, and a per-`EventKind` count that makes the client convert the events a
  collector needs even without a handler. Deletions are matched by the entry's guild and
  channel (a scan, as these events are rare). Collectors and the shards' senders live in a
  `Shared` struct of the client and every `Context` (replacing the separate application ID
  `Arc` and sender list).
- Semantics: a removal with `dispose` is yielded only for a collected reaction and gives its
  place back under `max`, as discord.js' `total--`; the item reaching a limit is still
  yielded; a component of another `component_type` is not even processed.

### C-104 — Phase 8 step 4b: gateway member and soundboard requests (D-265, D-270)
- `manager/gateway.rs`: `FetchMembers` (`limit`, `presences`, `time`; `IntoFuture`, boxed
  once per request), `GatewayRequestError` (`ShardNotRunning`, `Send`, `Timeout`,
  `RateLimited { retry_after }`, `Ended(EndReason)`) — a separate error type, as
  `djs_rs::Error` is the login error and the managers return REST errors;
  `GuildSoundboardSounds`.
- Defaults: 120 s for members and an explicit `time` for the sounds (discord.js: 120 s and
  10 s); a query asks for 100 members unless `limit` says otherwise (Discord requires a
  limit with a query), `fetch_all` sends the empty query with limit 0; `fetch_many` splits
  the IDs into requests of 100 (Discord's maximum), sent one after another; each request has
  its own nonce `djs-rs:<n>`, so other libraries' and the user's own requests are not
  mistaken for ours.
- A `RATE_LIMITED` event for our nonce ends the request with `RateLimited`; a deletion of
  the guild or the shutdown with `Ended`. The returned members are the cache's (stored while
  handling each chunk), converted when the cache keeps no members.
- `GuildManager::fetch_soundboard_sounds(ids, time)` groups the guilds by shard (one
  command per shard) and waits for every guild.

### C-105 — Phase 8 step 4c: collector benchmarks and stress tests (D-041)
- `djs-rs/tests/stress_collectors.rs` (ignored): a mock gateway that floods only after the
  test registered its collectors (a `Notify`), message floods with 0 / 10 000 unrelated /
  10 000 other-kind / 1 busy collector, a 1M-click storm, register + drop churn, and
  `fetch_all` of 100 000 members with and without a cache.
- The A/B against the commit before the collectors ran from a temporary `git worktree` in
  the git-ignored `.reference/ab-base` (removed afterwards), pinned to two P-cores.
- Fixed on the way: `Collectors::deliver` takes the event kind and returns before locking
  when no collector wants that kind.

### C-106 — Phase 8 step 5: intents and partials (D-271–D-273)
- The `events!` macro takes each event's intents as `[GUILD_MESSAGES | DIRECT_MESSAGES]`
  (`[]` for none), from Discord's list of intents; derived events (per-emoji, per-sticker,
  `GuildAvailable`, …) take their source's. `Event::INTENTS` is public (a new associated
  constant on the sealed trait); `EventKind::{ALL, intents, name}` are internal.
- An event counts as covered when the client requests any of its intents. The warning is
  one `tracing::warn!` listing the events and their intents: at `login()` for every
  registered handler, route and running collector, at `Client::on` for a handler added
  while running, and when a collector starts (its collected event: messages and reactions;
  interactions need none).
- `Client::required_intents()` unions the intents of the handlers, routes and the running
  collectors' events (including the deletion events that end collectors, all `GUILDS`); it
  never adds `MESSAGE_CONTENT`, which the docs explain separately.
- The partials guide is a table in the `event` module documentation (and a README section),
  one row per discord.js partial.

### C-107 — Phase 8 step 6: the example bots (D-274–D-276)
- Files: `examples/common/mod.rs` (`init_tracing`, `config_value` — empty values count as
  unset —, `token`, `test_guild`, `cleanup_requested`, `shutdown_on_ctrl_c`; included with
  `#[path]`, `#![allow(dead_code)]` because each example uses only some helpers), `ping.rs`
  moved onto it, `slash_commands.rs`, `components.rs`, `moderation.rs`, `advanced.rs`.
- Settings: `DISCORD_TOKEN`; `DISCORD_TEST_CHANNEL_ID` is the log channel of `moderation`;
  new optional `DISCORD_TEST_GUILD_ID` (the commands' guild, else the first guild of
  `READY`) and `DISCORD_WEBHOOK_URL` (`advanced` announces its start through it).
- Commands are guild commands set with `ctx.guild_commands(guild).set(…)` at `ClientReady`
  (they appear at once; `set` replaces the bot's commands in the guild, so two examples do
  not mix); `-- --cleanup` sets an empty list and shuts the client down — the D-275 cleanup
  is part of the examples.
- Safety in the examples: echoed or quoted user text is sent with empty `AllowedMentions`;
  the welcome may ping the new member only; moderation commands use
  `default_member_permissions` and `contexts([Guild])` (the docs say a real bot would also
  compare the moderator's and the target's roles); `/purge` skips messages older than 14 days
  (bulk deletion's limit) and deletes a single message with `delete`.
- `components`: `!menu` answers clicks by other users with an ephemeral message (a filter
  would leave them "failed") and disables its buttons by editing the message once idle;
  `!poll` filters out the bot's own reactions by the poll message's author and uses
  `dispose(true)`.
- `advanced`: `HandlerMode::Sequential` with the slow work (REST) spawned through a
  `spawn_logged` helper; statistics from `client.events()`; `!webhook` creates a webhook,
  sends, edits and deletes a message through `WebhookClient::new`, and deletes the webhook
  whatever happened.

### C-108 — The serenity harness runs without a network (after a request reached Discord)
- On 2026-11-13 the first smoke test of `benches/vs-serenity` sent one request to the real
  Discord: `GET https://discord.com/api/v10/gateway` with the fake benchmark token
  (`djs_vs_node::TOKEN`) and serenity's User-Agent, because serenity's default rate limiter
  ignores `HttpBuilder::proxy` (D-282). Discord answered with its public gateway URL; the
  runner's check (`ws://127.0.0.1:` only) stopped it before any gateway connection. No real
  token or data was sent; the owner was told.
- Since then every run of the serenity harness — mocks and clients of both libraries — runs
  in a network namespace without a route out (`unshare -rn`, loopback only; checked that
  connecting outside fails), so no future mistake can reach Discord. The runners keep their
  own checks (only `127.0.0.1` URLs; only the test CA trusted).

### C-109 — How the serenity comparison is measured (the harness's defaults)
- **Equal caches** (D-280): serenity's default cache `Settings` keep guilds with their
  members, roles, emojis, stickers, channels, threads, presences, voice states, stage
  instances and scheduled events, plus channels and users, and no messages
  (`max_messages: 0`); `djs_vs_node::serenity_cache()` gives a djs-rs `Client` the same
  (messages and soundboard sounds disabled). djs-rs without a cache (its default) is shown
  as a variant.
- **Equal optimization:** the harness workspace has the main workspace's release profile
  (`lto = "fat"`, `codegen-units = 1`), so the serenity runners are built like the djs-rs
  runners of `benches/vs-node`. The two `!ping` bots of `build.sh` use the default profiles
  of a new bot project, both alike.
- **REST:** serenity over the HTTPS mock (D-282) is the baseline; serenity with its limiter
  off over plain HTTP is a bound, shown and named as such. djs-rs's proactive global limit
  (50/s by default) is off in the throughput run, as in `docs/VS_NODE.md`; in the
  reliability run both libraries keep their defaults, and 40 requests stay under that limit.
- **Builds:** `cargo build` from an empty target directory with every CPU, then the rebuild
  after touching `main.rs`; two runs alternated, the faster shown with the range; offline,
  in the network namespace. Dependencies: `cargo tree -e normal` for this platform.
- **Gateway faults:** one message per ms, 10,000 in all, the fault at message 2000; a
  heartbeat interval of 1000 ms (Discord's is about 41 s) so a zombie connection is noticed
  within a run; the time without events is the mock's, from the fault to the first `READY`
  or `RESUMED`, the same clock for both libraries. A client that does not get every message
  waits until the production ends plus 5 s.
- **API faults:** the 429s ask for 0.5 s in the body's `retry_after` and send a
  whole-second `Retry-After` header, rounded up as Discord does (its examples: `65` for
  `64.57`); a request within 25 ms of a 429 counts as already in flight, a later one inside
  the window as early. The 502s hit the first attempt at every 4th message of a channel, as
  an HTML page from a proxy, so a client that retries sends everything.

### C-110 — The setters of the message bodies (D-290, D-291)
- One crate-private macro (`djs-model/src/rest/setters.rs`, `message_body!`) writes the shared
  setters and the `From<&str>`, `From<String>`, `From<CompactString>` conversions of the six
  bodies; two private traits (`Slot` for `Option`/`Patch`, `List` for `Vec`/`Option<Vec>`/
  `Patch<Vec>`) let one setter serve every field shape.
- Shared setters (where the field exists): `content`, `embed` (adds) / `embeds` (replaces),
  `component` (adds, takes `impl Into<Component>`) / `components` (replaces),
  `allowed_mentions`, `no_mentions` (`AllowedMentions::default()`), `flags` (replaces),
  `suppress_embeds`, `ephemeral` (interaction bodies only), `suppress_notifications`
  (`CreateMessage`, `ExecuteWebhook`, the interaction bodies), `file` (adds) / `files`
  (replaces), `attachments` (replaces), `poll`, `tts(bool)`. The flag setters add to the flags
  set before; `flags` replaces them. On `Patch` lists, `embed`/`component` on a kept list set
  it to that one item.
- Own setters: `CreateMessage::{reply_to(MessageId), message_reference, sticker, stickers,
  nonce(nonce, enforce), shared_client_theme}`; `ExecuteWebhook::{username, avatar_url,
  thread_name, applied_tags, thread_id, with_components}`;
  `EditWebhookMessage::{thread_id, with_components}`.
- No `clear_*` setters: `Patch::Clear` is set by hand (shown in `EditMessage`'s example).
- The examples (`slash_commands`, `moderation`, `components`, `advanced`) and the rustdoc
  examples use the setters; their `ephemeral` helpers are gone.

### C-111 — The shape of the permission computation (D-292, D-294, D-295)
- **`djs-model`:** three composable steps on `Permissions`: `Permissions::guild_level(owner,
  everyone, roles)` (Discord's `compute_base_permissions`), `.in_channel(guild_id, user_id,
  role_ids, overwrites)` (`compute_overwrites` + the implicit denials) and
  `.during_timeout()`, plus the constants `NEEDS_SEND_MESSAGES` and `DURING_TIMEOUT`. The
  timeout is a separate last step because overwrites must not give a timed-out member
  anything back; the owner and `ADMINISTRATOR` are recognized by the `ADMINISTRATOR` bit
  (`guild_level` returns `Permissions::all()` for both).
- **Without `VIEW_CHANNEL` the channel result is empty**, guild-wide permissions such as
  `KICK_MEMBERS` included (they are not "in the channel"; use the guild-level result for
  them). Overwrites cannot grant `ADMINISTRATOR` (its bit is dropped after them).
- **`djs-cache`:** `Cache::member_permissions(guild_id, user_id)` and
  `Cache::member_permissions_in(&Channel, user_id)`: the channel is passed as a value
  because threads are cached per guild (an ID alone cannot find one) and the bot often holds
  the channel; a thread uses its cached parent's overwrites. Any missing piece (guild,
  member, one of its roles, `@everyone`, a thread's parent) gives `None`, never a partial
  result. The current time comes from `Timestamp::now()`.
- **`djs-rs`:** `MemberExt::permissions(&ctx)` and `MemberExt::permissions_in(&ctx,
  ChannelId)` (the member's guild finds threads), through `CacheOps` so that their code is
  compiled in djs-rs (D-289); they read the cache's current member, not the value they are
  called on.

### C-112 — The first step of multi-process sharding (D-284–D-286, D-296–D-303, step 1)
- **The wire:** `{"op": "hello", "cluster", "token"}` from the process, then `welcome` and
  `shutdown` from the manager, as JSON frames after a 4-byte big-endian length, at most
  16 MiB each so a peer cannot make the other side allocate more. The process sends
  `hello` first and has 30 s to be welcomed; the manager gives a new connection 10 s to say
  `hello`. A wrong token, an unknown cluster or a second connection for a connected cluster
  is closed without an answer (the process gets `ClusterError::Refused`).
- **The token:** 128 bits from two `std::collections::hash_map::RandomState` hashes, whose
  `SipHash` keys the standard library takes from the operating system's random source:
  no `rand`/`getrandom` dependency for a secret that only guards a loopback port against
  other local programs (anyone who can read the process's environment could read memory
  anyway). Compared in constant time, and never printed (`Debug` redacts it, and
  `ClusterAssignment`'s `Debug` leaves it out).
- **The assignment:** the environment variable `DJS_RS_CLUSTER` holds the JSON of
  `ClusterAssignment` (cluster, shard range, total, the manager's address, the token).
  `sharding::process()` rejects a range that is empty or past the total, and a manager that
  is not on a loopback address.
- **The processes:** the manager's own executable (`std::env::current_exe`) with the
  manager's own arguments unless `executable`/`args` say otherwise, the manager's
  environment plus `env(...)` entries, standard input closed, standard output and error
  shared with the manager, `kill_on_drop` so that a manager that is dropped or dies does not
  leave them behind. Cluster *n* runs shards `[n × shards_per_process, …)`; the last may run
  fewer.
- **Stopping:** `ClusterManager::shutdown` sends `shutdown` to every connected process (and
  to any that connects afterwards), waits `shutdown_timeout` (30 s by default, like the
  gateway's own grace periods), then kills what is still running; `spawn` returns once every
  process has exited, or with the first error (`AlreadyRunning` when the manager is already
  running). A process whose connection to the manager closes shuts its client down, as if
  asked.
- **The API:** `ClusterManager` (cheap to clone, like `Client`) with `builder`, `spawn`,
  `shutdown`, `events()` (a stream of `ClusterEvent::{Spawned, Connected, Exited}`, a new
  receiver per call, as `Client::events`) and `clusters()` (a snapshot of `Cluster { id,
  shards, pid, connected }`); `ClientBuilder::cluster(assignment)` sets the process's shard
  range and connects to the manager in `login`. `ClusterError` is separate from `Error`
  (the manager has no client), and `Error::Cluster` wraps it for `login`.
- **Until step 2 (D-299, done in C-113)** each process identified through its own local
  queue: two processes could start sessions in the same 5-second window of a
  `max_concurrency` bucket.
- **Tests:** the cross-process tests start the test binary again with arguments that run
  only an `#[ignore]`d test (`cluster_process --exact --ignored`), which reads its
  assignment and connects to the mock REST API and gateway; `sleep` stands in for a process
  that never connects, to test the kill after the grace period (Unix only).

### C-113 — The IDENTIFY queue across processes (D-299, D-305, step 2)
- **The wire:** `{"op": "identify", "shard": n}` from a process asks a turn; the same frame
  from the manager grants it. The manager grants turns with djs-gateway's
  `LocalIdentifyQueue` (the same 5.1 s per bucket, first come first served within a bucket),
  one queue for every process of a `spawn`, so a restarted process (step 3) shares it.
- **Edge cases:** a turn asked for a shard outside the cluster's range is ignored (logged);
  turns still waiting when a process's connection ends are dropped without using their
  bucket's slot. In the process, a turn that arrives after its shard stopped waiting (its
  connection closed) goes to the next waiter of that shard, or is dropped (delaying that
  bucket once at most). If the manager is gone, a waiting shard never identifies: the client
  is shutting down anyway, and identifying unthrottled could get the bot's sessions refused.
- **D-305 details:** `ClusterError::SessionStartLimit` mirrors `Error::SessionStartLimit`;
  `max_concurrency` 0 from Discord is treated as 1. The assignment carries `gateway_url` and
  `max_concurrency` but only crate-private accessors (no public API for them yet). A
  `ClientBuilder::gateway_url` set in a cluster process wins over the manager's URL (for a
  proxy or tests).

### C-114 — Restarting cluster processes (D-298, D-306, step 3)
- **How long a process ran** is measured from its start to its exit as the manager sees
  them; a run of at least the largest delay resets the delay to the first one before the
  next restart (so with the defaults, a process that crashes after a minute waits 1 s).
- **A restart that cannot start** (the executable was removed, say) is logged and tried
  again after the next, longer delay, with a new `Restarting` event: only the first start
  of a `spawn` returns `ClusterError::Io`.
- **A shutdown** cancels the restarts still waiting (no `Spawned` follows their
  `Restarting`); a process that exits once the shutdown was asked is never restarted, under
  any policy.
- **`restart_delay(first, max)`** panics on a zero `first` (a failing bot would restart in a
  loop) or a `first` longer than `max`, like the builder's other checks; it is a `const fn`.
- **The restarted process** gets the same cluster ID, shards and token, connects again
  (its old connection was cleared when its exit was seen) and waits for its IDENTIFY turn
  in the same queue: the queue lives as long as the `spawn` (D-299).

### C-115 — Commands between clusters (D-286, D-307–D-309, step 4)
- **Names:** `ClusterCommand` (trait: `NAME`, `Input`, `Output`), `ClusterReply<T>`
  (`cluster`, `result`, non-exhaustive), `ClusterClient<B>` (from `Client::cluster()`, a
  `Client` clone inside: `id`, `shards`, `total_shards`, `handle`, `broadcast`, `send`),
  and on `ClusterManager` `broadcast` / `send`. New `ClusterError` variants:
  `NotConnected` (also an unknown cluster ID, and a cluster process that is not logged in),
  `UnknownCommand`, `CommandFailed(String)` (the handler's error as text: errors are not
  serializable), `Timeout`, `Encoding(String)` (serde_json's error as text, so that
  serde_json stays out of the public API).
- **Handlers** are `Fn(Client<B>, C::Input) -> Future<Output = Result<C::Output,
  HandlerError>>`, as event handlers return `HandlerError`; each request runs in its own
  task, and a panic becomes `CommandFailed("the command handler panicked")`, the process
  going on. A second `handle` of the same command replaces the first. `handle` returns
  `&Self` to chain, like `Client::on`.
- **The timeout** is the manager's, `ClusterManagerBuilder::command_timeout` (30 s by
  default): one deadline for every cluster of a request, after which the cluster's result
  is `Timeout` and its late reply is dropped. A cluster process's own request waits for
  the manager's answer (it always comes within that deadline, or the connection ends).
- **The wire:** `request` / `reply` / `replies` frames; inputs and outputs are
  `serde_json::Value` inside the frames (the manager does not need to know the types). The
  manager numbers its requests to the clusters; a cluster numbers its own, and the manager
  answers them under that number.
- **Later:** a `Context::cluster()` for handlers that do not capture the `ClusterClient`
  is in the open questions.

### C-116 — The built-in cluster stats (D-310, step 4)
- **Names:** `ClusterStats` (`cluster`, `shards`, `guilds`, `users`, `uptime`, `memory`),
  `ShardStats` (`id`, `status`, `latency`), `ShardStatus` (`Disconnected`, `Identifying`,
  `Resuming`, `Active`, `Closed`: djs-gateway's `ShardState` with both closed states as
  one, since a close code does not need to cross the processes), all non-exhaustive; the
  built-in command `Stats` (`NAME = "djs-rs:stats"`, public so that `send::<Stats>(id, ())`
  asks one cluster), behind `stats()` on `ClusterManager` and `ClusterClient`. Every
  cluster answers it before looking at the bot's handlers.
- **Where the numbers come from:** each shard's task writes its state (only when it
  changes) and its latency (on each heartbeat acknowledgement) to a slot of atomics, only in
  a cluster process; `guilds` / `users` are the cache's current counts (`None` without a
  cache); `uptime` is from the start of `login`; `memory` is `VmRSS` from
  `/proc/self/status` on Linux, `None` elsewhere (no dependency, no `unsafe`).
- **The test gateway** now heartbeats every second and acknowledges each heartbeat, also
  while a shard waits for its IDENTIFY turn, so that the stats have latencies.

### C-117 — Limits of the cluster connections (found while planning step 5)
- **Before its token is checked**, a connection's first frame may be at most 4 KiB
  (`MAX_HELLO`): another local program could otherwise make the manager allocate 16 MiB
  per connection by announcing a large frame.
- **A message larger than a frame (16 MiB)** is not written and does not end the
  connection: a request fails at once with `ClusterError::Encoding` (in the manager for its
  requests, in the cluster for the cluster's own), a handler's result too large is replaced
  by that error, and a broadcast's replies too large together (each fits, the sum does not)
  are each replaced by it. Before, the writing task stopped: the connection stayed half
  open, later requests timed out, and a cluster's own request waited forever.
- **Dropping a cluster's connection** (the end of `login`) fails its requests still
  waiting with `NotConnected` instead of leaving them waiting.

### C-118 — The Ogg/Opus reader (D-317, D-325, step 1a)
- **Internal and sans-IO:** `ogg::OggOpus` is crate-private (its public face is the coming
  `Input`, whose names are asked in step 1e). It is fed bytes (`push`, `finish`) and returns
  audio packets (`packet`), so files, pipes and memory share it; a packet within one page is
  returned without copying, one that crosses pages is assembled in a reused buffer. The
  buffer is compacted only once at least as many bytes were read as are left (amortized
  copying); it stays at about twice the read size.
- **Which stream:** the first logical stream whose first page carries `OpusHead`; pages of
  other streams are skipped; after its last page, the next chained Opus stream is read
  (its `OpusHead` replaces the previous). An input whose first link has no Opus, or that
  starts without the identification header, fails with `NotOpus`; no page in the first
  64 KiB fails with `NotOgg`; a malformed `OpusHead` (major version other than 0, no
  channels, family 0 with more than 2, a short mapping table) with `BadHead`; a missing
  `OpusTags` with `BadTags`. Empty audio packets are skipped.
- **Damage:** a page with a wrong checksum, version or capture pattern is skipped and the
  next capture pattern searched; a packet missing a part (a page lost, found by the page
  sequence number, or cut by a page without the continuation flag) is dropped, and the rest
  of a packet whose beginning was lost is skipped. At the end of the input an incomplete
  page is searched past, so a false capture pattern cannot hide the pages after it.
- **Checksum by slicing-by-8** (eight 256-entry tables, 8 KiB, built at compile time):
  3.9 times the byte-at-a-time speed (`docs/PERFORMANCE.md`, "Phase 10").
- **Measured by an ignored test** (`ogg::tests::throughput`) until `Input` exists; a divan
  benchmark through `Input` comes with step 1e. The tests read a 4.5 KiB file encoded by
  libopus and written by `ffmpeg` (`tests/data/sine.opus`), so the parser is checked
  against an independent writer, and synthetic streams for crossing, damage, interleaving
  and chaining.
- **`opus::packet_samples`**: a packet's duration from its TOC byte (RFC 6716, 3.1), for
  pacing packets sent unchanged; `None` for one that cannot be valid.

### C-119 — Transport encryption and the UDP framing (D-314, step 1b)
- **Internal:** `crypto::{Mode, Cipher}` and `udp::{rtp_header, discovery_request,
  discovery_response}` are crate-private; the connection (step 1c) uses them.
- **Mode choice:** `aead_aes256_gcm_rtpsize` whenever the voice server offers it (Discord's
  advice), else `aead_xchacha20_poly1305_rtpsize`; no mode is chosen (an error, later) if
  it offers neither. The RustCrypto AES uses AES-NI when the CPU has it, detected at run
  time.
- **In place, no allocation:** `seal(packet, header, payload, nonce)` encrypts the payload
  after the header and appends the 16-byte tag and the 4-byte big-endian nonce counter;
  `open(packet)` authenticates the header (with its CSRCs and the extension preamble),
  decrypts, and returns the payload's range after the decrypted extension body. Malformed
  or unauthentic packets give `None`. The AES-GCM cipher is boxed (its key schedule is some
  hundred bytes, the XChaCha20 one 32), so the enum stays small.
- **Test vectors from libsodium**, which Discord uses: packets sealed by the system's
  `libsodium` through Python's `ctypes` (both modes, two nonce counters, and a received-style
  packet with a CSRC and a header extension) must match byte for byte.
- **Not yet:** zeroizing the key on drop (the cipher crates' `zeroize` features); the JSON
  the key arrives in leaves copies anyway. Left for the owner if wanted.
- **IP discovery** responses are checked (type, length, SSRC) and their address parsed as
  IPv4 or IPv6; anything else is ignored rather than an error, so stray packets on the
  socket do not break the handshake.

### C-120 — The voice gateway's messages (step 1b)
- **Dependencies from the workspace:** `djs-voice` now uses `serde` (derive), `serde_json`
  and `djs-model` (for `GuildId`/`UserId` and, later, the voice events of D-324): all
  already in the workspace and used by the other crates, under D-325's "already in the
  workspace". The connection (step 1c) will add the TLS and WebSocket crates djs-gateway
  uses, the same way.
- **Internal `payload` module:** commands are built as strings (JSON) or byte vectors
  (binary DAVE opcodes 26 and 28: the opcode byte and the MLS messages one after the other);
  server messages decode to `Incoming { seq, event: Event }`, the JSON envelope through
  `serde_json::Value` (messages are few: no hot path), binary ones as
  `[sequence u16 big-endian][opcode][payload]`, the transition IDs of 29 and 30 as u16
  big-endian (MLS's TLS encoding). Undocumented opcodes Discord sends (12, 14, 15, 18, 20, …)
  decode to `Event::Other(op)`, not errors. Opcode 24 is read as `{epoch,
  protocol_version}` (the whitepaper has no transition ID there); opcode 23 sends only the
  transition ID. The secret key is redacted from `Debug`.
- **Sequence numbers** are kept as `u64` (JSON `seq`; the binary u16 widened): the client
  only needs the last one seen, for `seq_ack`.
- **After a close:** resume on 4015 and on a connection lost without a code; identify again
  on 4006 and 4009; stop on 4004, 4011, 4014 (kicked, moved or the main session dropped: the
  bot's voice state says which), 4017, 4021, 4022 and on the codes that mean a client bug
  (4001, 4002, 4003, 4005, 4012, 4016, 4020).

### C-121 — The voice connection (D-322, D-330, D-331, step 1c)
- **Internal `connection` module:** `connect(info, config)` runs the handshake within
  `Config::timeout` (10 s, D-322): the WebSocket (`wss://{endpoint}/?v=8`; an endpoint that
  already starts with `ws://` or `wss://` is used as it is, for tests and proxies),
  Identify (`max_dave_protocol_version` = `davey::DAVE_PROTOCOL_VERSION`), Hello and Ready
  in either order, the UDP socket (bound to the unspecified address of the server's family
  and connected to it), IP discovery (asked again every second until answered), Select
  Protocol with the discovered address and the chosen mode, and Session Description (whose
  mode must be the one chosen). Messages that arrive during the handshake are delivered to
  the call after it; heartbeats start at Hello, also during the handshake.
- **The connection's task** heartbeats with `seq_ack` (a heartbeat still unacknowledged
  when the next is due counts as a lost connection), sends the UDP keepalive (the SSRC in 4
  bytes every 5 s, songbird's format; Discord does not document one), forwards the other
  messages as `ConnectionEvent::Gateway`, and takes `Command`s (Speaking, DAVE text and
  binary replies, Close). After a loss it follows `after_close` and D-331: resume at once and
  after 1, 2, 4, 8, 16 s (each halved by up to half at random), identify anew once on 4006
  or 4009 (also when a resume is refused with them), stop on the rest. A resumed session
  keeps its UDP socket, SSRC and key; a new one replaces them for the senders (a `watch`
  channel) and the last speaking flags are sent again. During the waits a Close ends the
  connection, speaking flags are kept and DAVE replies (of the lost session) dropped.
- **Ending:** `ConnectionEvent::Ended(End::{Closed, Stopped(code), Lost(error)})` is sent
  first, then the WebSocket's closing handshake runs with a 1 s bound.
- **`Sender`:** builds and encrypts the RTP packets of the current session in a reused
  buffer; the RTP sequence and timestamp start at random values (RFC 3550, 5.1), the nonce
  counter at 0, all three continuing across new sessions.
- **Tests** run the client against a mock voice server on `127.0.0.1` (a WebSocket server
  and a UDP socket that answers IP discovery with another address and decrypts the audio):
  the handshake fields, audio, sequence numbers in heartbeats, resume after 4015 and after a
  dropped connection or unanswered heartbeats, identify anew after 4006 (new SSRC and key),
  stop codes, giving up after the 6 attempts, a refused resume, closing, commands,
  keepalives and the handshake's failures. 20 repeated runs: no failure.

---

## Open questions (to ask the owner when relevant)

- **Drop the D-328 advisory ignores when `davey` updates (Phase 10):** once a `davey`
  release depends on openmls 0.9 (or later), update it (ask the owner: openmls 0.9 needs
  MSRV 1.91), re-run the advisory check and remove the ignores from `deny.toml` that no
  longer apply.

- **Bounded channels between the manager and its clusters (Phase 9 findings):** the queues
  of frames to and from each process are unbounded; a process that stops reading while it
  is flooded with commands grows the manager's memory. No measured load needed a bound
  (`docs/PERFORMANCE.md`, "Phase 9"); a bound needs a policy when full (wait, fail the
  command, drop the connection). Ask the owner if a bot needs it.

- **`Context::cluster()` (Phase 9, C-115):** event handlers reach the `ClusterClient` only
  by capturing a clone (`client.cluster()`), the pattern `ctx.shutdown()` removed for
  shutdowns (D-293). A non-generic handle for sending (`broadcast`, `send`, `stats`) could
  be reachable from `Context`; `handle` needs the client's backend type and would stay on
  `ClusterClient`. Ask the owner whether to add it.

- **djs-rs bots rebuild slower than serenity bots** (`docs/VS_SERENITY.md`, C-109): after
  editing `main.rs` of the same `!ping` bot, a release rebuild takes about twice as long
  with djs-rs as with serenity (debug too, by a smaller margin), although clean builds are
  faster. A likely cause is that the generic `Client` and the handler closures are
  monomorphized and optimized in the bot's crate; erasing types at the handler boundary or
  moving work into non-generic functions could help. Worth a measurement and a decision
  before 1.0, since it is what a bot author feels on every edit. **Resolved (D-287–D-289):**
  re-measured on 2026-11-13, a release rebuild takes 0.73 s against serenity's 5.32 s (debug:
  about the same).
- **Ergonomics gaps found by the examples (C-107)** — decided 2026-11-13: D-290–D-295, all implemented: (1) an ephemeral or embed reply needs
  `let mut reply = MessageCallbackData::from(…); reply.flags = Some(MessageFlags::EPHEMERAL);`
  — the body types are `#[non_exhaustive]`, so struct literals are impossible outside the
  crate; setters (`.ephemeral()`, `.embed(…)`, `.components(…)`) or a message builder in
  `djs-builders` would shorten every bot. (2) `EditWebhookMessage` (so `edit_reply`) has no
  `From<String>`, unlike its siblings: `format!(…).as_str()` is needed. (3) `CreateMessage`
  holds `Vec`s where `MessageCallbackData` holds `Option<Vec>`s. (4) No member permission
  calculation from the cache (discord.js `GuildMember#permissions` / `permissionsIn`), so
  prefix commands cannot check a moderator's rights. (5) Shutting down from a handler needs a
  captured `Client` clone (no `Context::shutdown`).
- **Committing the resource guard** (D-257): the guard script is kept in the git-ignored
  `.reference/tools/safe.sh` (with `gate.sh`), outside the repository's history; committing it
  (for example `scripts/safe.sh`) would share it with the repository.
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
