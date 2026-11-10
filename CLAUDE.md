# CLAUDE.md — instructions for AI assistants working on djs-rs

This file is loaded automatically by Claude Code. **Read it fully before doing anything.**

## What this project is

`djs-rs` is a from-scratch Rust rebuild of the **discord.js v14** ecosystem: a library for
writing Discord bots in Rust, designed for **maximum performance** and **maximum documentation**.

- Repository: https://github.com/kotoanidev/djs-rs (public)
- Discord API version targeted: **v10** (the version discord.js v14 uses)
- License: `MIT OR Apache-2.0`

## Start-of-session checklist (mandatory)

1. Read [`PROGRESS.md`](PROGRESS.md) — the current state and the **exact next step**.
2. Read [`docs/DECISIONS.md`](docs/DECISIONS.md) — every decision made so far. They are binding.
3. Skim [`ROADMAP.md`](ROADMAP.md) to see where the next step sits in the bigger picture.
4. Run `git status` and `git log --oneline -10`; if there is uncommitted work, it is from an
   interrupted session — inspect it, finish or discard it **after asking the user**.

## Hard rules

### 1. Never make significant decisions for the user

The owner explicitly requires that **every significant decision is asked, not assumed**:
architecture, new dependencies, public API shape, naming of public items, scope changes,
phase ordering, `unsafe` usage, MSRV bumps, breaking changes, licensing, publishing.

- Stop and ask (in Polish — see below), offering options and marking your recommendation.
- Record the answer in `docs/DECISIONS.md` (next free `D-NNN` number) **before** implementing.
- Minor, mechanical choices (private helper names, test layout, formatting) may be made
  directly — but if a choice is non-obvious, log it in the "Defaults chosen by Claude" section
  of `docs/DECISIONS.md` so the owner can review it later.
- Questions that will need answering later go to the "Open questions" section of
  `docs/DECISIONS.md` so they are not forgotten between sessions.

### 2. Commit and push after every significant change

The owner wants work to survive even if an assistant session ends abruptly (usage limits).

- Work directly on `main` (owner's explicit choice — no feature branches).
- After each meaningful unit of work: update `PROGRESS.md`, then `git commit` **and** `git push`.
- Prefer many small commits over one big one. Never leave hours of work uncommitted.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
  `feat(rest): add global rate limiter`, `docs(model): document Snowflake layout`, etc.
- Before committing code, run the local quality gate (see "Commands"). Do not push code that
  fails `fmt`, `clippy` or tests. If something must be pushed in a broken state to avoid losing
  it, say so explicitly in the commit message (`wip:` prefix) and in `PROGRESS.md`.

### 3. Keep `PROGRESS.md` resumable

`PROGRESS.md` must always answer: *what is done, what is in progress, what exactly is next,
and what is blocked on the owner*. A fresh session with zero context must be able to continue
from it alone.

### 4. discord.js is a reference, not a source (D-004)

discord.js is Apache-2.0; this project is `MIT OR Apache-2.0`. Read discord.js and the official
Discord API docs to understand **behaviour and API surface**, but **write all code and rustdoc
from scratch**. Never copy or translate code or doc text verbatim. Acknowledgements live in
`NOTICE`.

### 5. Language

- Talk to the owner in **Polish** (with correct diacritics).
- Everything in the repository — code, identifiers, rustdoc, README, guides, commit messages —
  is in **English** (D-009).

## Engineering standards

### Performance ("max optimization")

- The gateway event path is the hottest path: avoid allocations, avoid needless clones,
  prefer borrowing / zero-copy deserialization where it is sound and measurable.
- Every optimization claim must be backed by a benchmark (benchmarks live in each crate's
  `benches/`). Do not trade correctness or API clarity for unmeasured speed.
- Keep struct sizes in mind for cached types; add `static_assertions`-style size tests for
  hot types once they exist (ask before adding the dependency).

### Documentation ("max documentation")

- `missing_docs` is enforced; CI builds docs with `-D warnings`.
- Every public item gets: a summary line, an explanation, and — where it makes sense — a
  runnable `# Examples` section, `# Errors`, `# Panics` and a link to the corresponding
  Discord API docs page and the discord.js counterpart (e.g. "discord.js: `Client#login`").
- Each crate has a `README.md` that is also its crate-level rustdoc (`#![doc = include_str!(...)]`).
- Prefer intra-doc links (`[`Snowflake`]`) over plain text.

### Safety

- `unsafe_code` is denied workspace-wide (see `docs/DECISIONS.md`). Any exception needs the
  owner's approval, a local `#[allow(unsafe_code)]`, and a `// SAFETY:` comment.

### Dependencies

- Adding **any** new dependency is a significant decision → ask first.
- Shared dependency versions are declared once in the root `Cargo.toml`
  (`[workspace.dependencies]`) and referenced with `dep.workspace = true`.
- Keep default features minimal; expose optional functionality via cargo features.

## Repository layout

```
Cargo.toml            workspace root (shared metadata, lints, dependency versions)
crates/
  djs-rs/             main crate — high-level discord.js-like API (Client, events, structures)
  djs-model/          Discord API types (≈ discord-api-types)
  djs-rest/           REST client + rate limiting (≈ @discordjs/rest)
  djs-gateway/        gateway / WebSocket shards (≈ @discordjs/ws)
  djs-cache/          in-memory cache with configurable policies
  djs-builders/       builders for embeds, commands, components (≈ @discordjs/builders)
  djs-collection/     Collection type (≈ @discordjs/collection)
  djs-formatters/     markdown/mention/timestamp formatters (≈ @discordjs/formatters)
  djs-util/           shared utilities (≈ @discordjs/util)
docs/
  DECISIONS.md        decision log (binding)
  ARCHITECTURE.md     how the crates fit together
PROGRESS.md           resume point
ROADMAP.md            phased plan with checkboxes
```

## Commands

```bash
source ~/.cargo/env                                        # if cargo is not on PATH
cargo fmt --all                                            # format
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo +1.89 check --workspace --all-features               # MSRV check (rustup toolchain install 1.89 first)
```

Local quality gate before every code commit: `fmt` → `clippy` → `test` → `doc`.

## Toolchain

- Installed via rustup (`~/.cargo`). Stable channel for development.
- Edition 2024, MSRV **1.89** (D-031; bump only with owner approval + CHANGELOG note).
