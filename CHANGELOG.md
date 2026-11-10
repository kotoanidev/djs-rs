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

### Changed
- MSRV raised from 1.85 to 1.89 (required by maintained `tokio-websockets` and `simd-json`).
