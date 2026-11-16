# Contributing to djs-rs

Thank you for your interest! djs-rs is in alpha: the API may still change between
`0.1.0-alpha` releases, and feedback on it is as welcome as code.

- **Found a bug?** Open a [bug report](https://github.com/kotoanidev/djs-rs/issues/new?template=bug_report.yml).
- **Missing something?** Open a [feature request](https://github.com/kotoanidev/djs-rs/issues/new?template=feature_request.yml).
- **A question or an idea?** [Discussions](https://github.com/kotoanidev/djs-rs/discussions).
- **A vulnerability?** Report it privately, as [`SECURITY.md`](SECURITY.md) explains.

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md).

## Before you write code

A fix or a small addition can go straight to a pull request. Anything bigger (a new public
type, a change of the API's shape, a new dependency, a new cargo feature) starts as an
issue, so the design is agreed before the work. The project records every such decision in
[`docs/DECISIONS.md`](docs/DECISIONS.md) (`D-NNN` entries, referenced from the code); read
the entries near what you change, they explain why things are the way they are.

## Ground rules

- **No copying.** discord.js (Apache-2.0), serenity, twilight and the Discord docs are
  references for *behaviour*. Write code and docs from scratch, so the project stays cleanly
  `MIT OR Apache-2.0`.
- **Everything public is documented.** `missing_docs` is enforced and CI builds the docs
  with `-D warnings`. A public item has a summary line, an explanation, an example where one
  helps, `# Errors` and `# Panics` sections where they apply, a link to its page of the
  [Discord API docs](https://docs.discord.com/developers) and its discord.js counterpart,
  written as `discord.js counterpart: `Message#reply`.` (or `none (…)` with the reason).
- **Performance claims need benchmarks.** An optimization comes with the measurement that
  shows it (see [Benchmarks](#benchmarks)); none trades correctness or a clear API for
  speed nobody measured.
- **`unsafe` is denied** workspace-wide.
- **New dependencies** need the maintainer's agreement first (in an issue). Versions are
  declared once, in the root `Cargo.toml`'s `[workspace.dependencies]`.

## Setting up

You need [Rust](https://rustup.rs/) (the repository's `rust-toolchain.toml` selects stable
with `rustfmt` and `clippy`), a C compiler (for `zstd`) and CMake (for libopus, built by
the `voice` feature). The guide needs [mdBook](https://rust-lang.github.io/mdBook/) 0.5.4:
`cargo install mdbook --version 0.5.4 --locked`.

What CI checks, and what a pull request must pass:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
mdbook build book
cargo +1.89 check --workspace --all-features   # the MSRV; `rustup toolchain install 1.89` first
```

## Where things are

| Path | What |
|---|---|
| `crates/<crate>/src` | the crates; `crates/djs-rs` is the one bots use |
| `crates/<crate>/tests` | integration tests, against mock Discord servers (no network, no token) |
| `crates/<crate>/tests/stress.rs` | extreme-load tests, `#[ignore]`d by default |
| `crates/<crate>/benches` | [divan](https://docs.rs/divan) benchmarks |
| `crates/djs-rs/examples` | complete example bots |
| `book/` | the guide; its listings are the examples of `book/code` |
| `benches/vs-node`, `vs-serenity`, `vs-twilight` | the comparisons with other libraries |
| `docs/` | architecture, decisions, measurements |

## Tests

Unit tests sit next to the code; integration tests in `tests/` run against local mock
servers and need nothing but `cargo test`. A test never talks to Discord. A change of
behaviour comes with a test that fails without it.

Stress tests are ignored by default; run them in release mode, one crate at a time:

```bash
cargo test -p djs-gateway --all-features --release --test stress -- --ignored --nocapture
```

## Benchmarks

```bash
cargo bench -p djs-model --bench decode
```

Numbers depend on the machine: compare a change against `main` on the same machine, and
put both in the pull request. [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) records the
project's measurements and how they were taken. The comparisons with discord.js, serenity
and twilight (`benches/vs-*/run.sh`) run on Linux, with Node.js for the first and
`unshare` for the others; their READMEs explain them.

## The guide

The guide's chapters (`book/src`) include their code from `book/code/examples`, between
`// ANCHOR: name` and `// ANCHOR_END: name` lines, so every listing compiles and is checked
by clippy. Example names are unique across the workspace.

## Commits and the changelog

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): summary`, where `scope` is usually the crate without its `djs-` prefix:

- `feat(rest): add per-route rate limit buckets`
- `fix(gateway): resume after close code 4000`
- `docs(model): document permission overwrites`

A change users can see gets an entry in [`CHANGELOG.md`](CHANGELOG.md) under
`Unreleased`, following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## License

By contributing, you agree that your contributions are dual licensed under MIT OR
Apache-2.0, without any additional terms or conditions.
