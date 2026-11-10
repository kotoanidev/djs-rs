# Contributing to djs-rs

Thank you for your interest! djs-rs is in early development; the architecture is documented in
[`docs/DECISIONS.md`](docs/DECISIONS.md) and the plan in [`ROADMAP.md`](ROADMAP.md).

## Ground rules

- **No copying from discord.js.** discord.js (Apache-2.0) is a behavioural reference only. Write
  code and docs from scratch so the project stays cleanly `MIT OR Apache-2.0`.
- **Everything public is documented.** `missing_docs` is enforced and CI builds docs with
  `-D warnings`. Include examples, `# Errors` / `# Panics` sections, and links to the Discord API
  docs and the discord.js counterpart where relevant.
- **Performance claims need benchmarks.**
- **`unsafe` is denied** workspace-wide; exceptions require maintainer approval and a
  `// SAFETY:` comment.
- **New dependencies** require maintainer approval.

## Development setup

Install Rust via [rustup](https://rustup.rs/). The repository's `rust-toolchain.toml` selects
the stable toolchain with `rustfmt` and `clippy`.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
```

MSRV check (1.89):

```bash
rustup toolchain install 1.89
cargo +1.89 check --workspace --all-features
```

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): summary`, where
`scope` is usually a crate without the `djs-` prefix, for example:

- `feat(rest): add per-route rate limit buckets`
- `fix(gateway): resume after close code 4000`
- `docs(model): document permission overwrites`

## Changelog

Notable changes go into [`CHANGELOG.md`](CHANGELOG.md) under `Unreleased`, following
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## License

By contributing, you agree that your contributions will be dual licensed under MIT OR
Apache-2.0, without any additional terms or conditions.
