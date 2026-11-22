# Installation

## Rust

djs-rs needs **Rust 1.89 or newer**. Install Rust with [rustup](https://rustup.rs) if you
do not have it, or update it:

```sh
rustup update
rustc --version
```

Two features need a C compiler, because they build C libraries: `zstd-stream` (the
default gateway compression) and `voice` (which also needs CMake). On Linux install `gcc`
or `clang` (and `cmake` for voice) from your distribution; on macOS, the Xcode command line
tools (`xcode-select --install`, and CMake from Homebrew); on Windows, the "Desktop
development with C++" workload of Visual Studio Build Tools, and CMake. If you would rather
not need a C compiler, see [Cargo features](#cargo-features) below.

## A Discord server to test in

Bots are tested best in a server of your own, where nothing can go wrong: in the Discord
app, press the **+** at the bottom of the server list and create one. The next chapter adds
your bot to it.

## The project

Create a binary crate:

```sh
cargo new my-bot
cd my-bot
```

and add djs-rs and an async runtime to its `Cargo.toml`:

```toml
[dependencies]
djs-rs = "0.1.0-alpha.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- **djs-rs** comes from crates.io. It is in alpha, and its API can still change between
  alpha releases; since `cargo update` may move to a newer alpha, write `"=0.1.0-alpha.2"`
  to upgrade only when you choose to. The newest, unreleased code is in its Git
  repository: `djs-rs = { git = "https://github.com/kotoanidev/djs-rs" }`.
- **tokio** runs the bot. `macros` gives `#[tokio::main]`, `rt-multi-thread` the runtime
  that spreads the bot's work over your CPU's cores, `signal` a way to notice Ctrl+C.
- **tracing-subscriber** prints the library's logs: djs-rs reports what it does
  (connecting, reconnecting, a handler's error) through
  [`tracing`](https://docs.rs/tracing), and without a subscriber those messages go
  nowhere.

Run `cargo build` once now. The first build compiles every dependency and takes a few
minutes; later builds only compile what changed.

## Cargo features

djs-rs keeps optional parts behind features, so a bot only builds what it uses:

| Feature | On by default | What it adds |
|---|---|---|
| `gateway` | yes | The gateway: `client.login()`, the shards and their events. |
| `hyper` | yes | The REST client's HTTP transport. |
| `zstd-stream` | yes | Compresses the gateway's traffic with zstd (needs a C compiler). |
| `zlib-stream` | | The same with zlib, in pure Rust: no C compiler. |
| `native-roots` | | Trusts your operating system's certificates too (for a corporate proxy). |
| `sharding` | | Runs the bot's shards in several processes (the *Sharding* chapter). |
| `voice` | | Joins voice channels, plays and receives audio (the *Voice* chapter). |
| `http-interactions` | | Answers interactions at an Interactions Endpoint URL (the *HTTP interactions* chapter). |

To build without a C compiler, turn the default off and pick zlib:

```toml
djs-rs = { version = "0.1.0-alpha.2", default-features = false, features = ["zlib-stream"] }
```

With the project set up, the next chapter creates the bot on Discord's side and writes its
first lines.
