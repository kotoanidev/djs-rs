# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 1 — Vertical slice ("ping-pong" bot)** — not started; first step is a round of
owner decisions (see "Next step").

## Done
- 2026-11-10: Owner decisions D-001 … D-026 recorded in `docs/DECISIONS.md`.
- 2026-11-10: Rust stable 1.99.1 + 1.85 (MSRV) installed via rustup on the dev machine.
- 2026-11-10: Phase 0 complete — foundation docs, public repo
  https://github.com/kotoanidev/djs-rs, 9-crate workspace skeleton, lints/configs, CI.
  First CI run fully green (fmt, clippy, tests ×3 OS, rustdoc, MSRV 1.85, cargo-deny).
- 2026-11-10: Roadmap restructured for D-020 (vertical slice first).

## In progress
- Nothing.

## Next step
1. Ask the owner the Phase 1 decision round (from "Open questions" in `docs/DECISIONS.md`):
   (D-023..D-026 already answered: IDs, strings, errors, tracing.) Remaining:
   rustls crypto provider, event handler concurrency model, and the
   exact dependency set for the slice (tokio, hyper, hyper-util, hyper-rustls/rustls,
   tokio-websockets, serde, serde_json, zstd, …).
2. Record answers as D-023+ and implement the slice in this order:
   `djs-model` core → `djs-rest` core → `djs-gateway` core → `djs-rs` core → `examples/ping.rs`.

## Blocked on the owner
- Phase 1 decision round (above).
