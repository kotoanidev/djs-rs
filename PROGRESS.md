# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 0 — Foundation: complete** (pending green CI run). Waiting for the owner to approve
the phase order in `ROADMAP.md` before starting Phase 1.

## Done
- 2026-11-10: Owner answered the initial decision round (D-001 … D-019 in `docs/DECISIONS.md`).
- 2026-11-10: Rust stable 1.99.1 + 1.85 (MSRV) installed via rustup on the dev machine.
- 2026-11-10: Foundation documents: `CLAUDE.md`, `docs/DECISIONS.md`, `ROADMAP.md`,
  `PROGRESS.md`, `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, licenses, `NOTICE`.
- 2026-11-10: Public repo created: https://github.com/kotoanidev/djs-rs
- 2026-11-10: Cargo workspace with 9 placeholder crates (each with README used as crate docs),
  workspace lints, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`,
  GitHub Actions CI (`.github/workflows/ci.yml`), `docs/ARCHITECTURE.md`.
  Local gate passes: fmt, clippy `-D warnings`, tests, rustdoc `-D warnings`, MSRV 1.85 check.

## In progress
- Nothing.

## Next step
1. Confirm the first CI run on GitHub is green (`gh run list --limit 1`); fix if not.
2. Get the owner's approval of the phase order in `ROADMAP.md`.
3. Start Phase 1 (or whatever the owner picks) — first ask the open questions relevant to it
   (see `docs/DECISIONS.md` → "Open questions").

## Blocked on the owner
- Approval of the phase order in `ROADMAP.md`.
- Open questions listed in `docs/DECISIONS.md` (asked when the relevant phase starts).
