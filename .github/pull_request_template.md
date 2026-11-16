<!-- Thanks for contributing! See CONTRIBUTING.md. Never include a bot token. -->

## What and why

<!-- What this changes, and the issue it closes (`Closes #123`), if any. -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      `cargo test --workspace --all-features` and
      `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` pass
- [ ] New public items are documented (summary, explanation, example where it helps, the
      Discord API docs link and the discord.js counterpart)
- [ ] Tests cover the change
- [ ] A performance claim comes with a benchmark (`benches/`)
- [ ] `CHANGELOG.md` has an entry under `Unreleased` for a user-visible change
- [ ] No new dependency, or it was agreed in an issue first
- [ ] The code and docs are my own, not copied from discord.js or another library
