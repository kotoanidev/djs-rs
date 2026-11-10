# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Project foundation: decision log, roadmap, progress tracking, licensing (MIT OR Apache-2.0),
  contribution guidelines.
- Cargo workspace with nine placeholder crates (`djs-rs`, `djs-model`, `djs-rest`,
  `djs-gateway`, `djs-cache`, `djs-builders`, `djs-collection`, `djs-formatters`, `djs-util`).
- Workspace lints, rustfmt/clippy/cargo-deny configuration, GitHub Actions CI.
- `docs/ARCHITECTURE.md`.
- `djs-model` core (Phase 1): typed snowflake IDs `Id<Marker>` with 27 markers and aliases,
  `Timestamp` (dependency-free ISO 8601), compact `ImageHash`, `User`, `UnavailableGuild`,
  `Message` + `MessageType`, gateway `OpCode`, `CloseCode`, `Intents`, `ShardId`, payloads
  (`Hello`, `Ready`, `MessageCreate`, `Identify`, `Resume`, `Heartbeat`), `GatewayEvent` /
  `DispatchEvent` and the single-pass `GatewayEventDeserializer`.
- `djs-model`: `MessageReference` (+ type), `rest::{GatewayBot, SessionStartLimit, CreateMessage}`.
- `djs-rest` core (Phase 1): `Rest` client on hyper 1.x + rustls (ring, webpki-roots) with
  HTTP/2, `RestBuilder` (discord.js `RESTOptions` defaults), per-bucket sequential rate limiting
  with hash learning and state migration, global limiter + global 429 pauses, retries for
  5xx/transport/timeouts, `ApiError` parsing, typed `Route`s, `get_gateway_bot`,
  `create_message`, idle-bucket sweeping, token redaction.
- `djs-gateway` core (Phase 1): poll-driven `Shard` (connect/Hello timeouts, identify with
  per-shard 5 s limit, resume, heartbeats with jitter + zombie detection, reconnect backoff,
  close-code handling), `MessageSender`, `Config`/`ConfigBuilder`, `zstd-stream` compression
  (default feature), rustls with explicit ring provider.

### Changed
- MSRV raised from 1.85 to 1.89 (required by maintained `tokio-websockets` and `simd-json`).
