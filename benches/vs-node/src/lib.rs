//! The Rust side of the djs-rs vs discord.js showcase (`docs/VS_NODE.md`, D-123).
//!
//! This is a benchmark harness, not a library for bots. `run.sh` drives its two binaries —
//! `vs-node` (system allocator) and `vs-node-mimalloc` — which share every subcommand:
//!
//! | Subcommand     | What it does |
//! |----------------|--------------|
//! | `fixtures`     | Writes the decode fixtures that the Node scripts read too. |
//! | `decode`       | Times decoding of the fixtures into djs-model types. |
//! | `mock-gateway` | Serves `GET /gateway/bot` and a gateway that sends `READY`, guilds and a message flood. |
//! | `mock-rest`    | Serves a fake Create Message endpoint with rate-limit headers. |
//! | `flood`        | A djs-rs `Client` counting `MESSAGE_CREATE` handler calls. |
//! | `shard-flood`  | A bare djs-gateway `Shard` counting decoded `MESSAGE_CREATE` events. |
//! | `memory`       | A djs-gateway `Shard` keeping every decoded guild, then reporting RSS. |
//! | `rest`         | Concurrent Create Message calls through djs-rest. |
//!
//! Every client subcommand prints one JSON line with its results.

use std::{collections::HashMap, str::FromStr};

mod decode;
mod flood;
mod memory;
pub mod metrics;
mod mock_gateway;
mod mock_rest;
pub mod payloads;
mod rest;

/// The token every client uses. It is not a real token and authenticates nothing.
pub const TOKEN: &str = "not-a-real-token-only-for-benchmarking-0123456789";

/// Parsed `--key value` options.
#[derive(Debug, Default)]
pub struct Options(HashMap<String, String>);

impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Self {
        let mut map = HashMap::new();
        while let Some(key) = args.next() {
            let key = key
                .strip_prefix("--")
                .unwrap_or_else(|| panic!("expected `--option`, got `{key}`"))
                .to_owned();
            let value = args
                .next()
                .unwrap_or_else(|| panic!("`--{key}` needs a value"));
            map.insert(key, value);
        }
        Self(map)
    }

    /// The value of `--key`, parsed, or `default`.
    ///
    /// # Panics
    ///
    /// Panics if the value does not parse.
    #[must_use]
    pub fn get<T: FromStr>(&self, key: &str, default: T) -> T {
        self.0.get(key).map_or(default, |value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("invalid value for --{key}: {value}"))
        })
    }

    /// The value of `--key`.
    ///
    /// # Panics
    ///
    /// Panics if the option is missing.
    #[must_use]
    pub fn required(&self, key: &str) -> &str {
        self.0
            .get(key)
            .unwrap_or_else(|| panic!("--{key} is required"))
    }
}

/// Runs the subcommand named by the first argument. `allocator` names the global allocator
/// of the calling binary and is included in every result.
///
/// # Panics
///
/// Panics on unknown subcommands, invalid options and any failure of the scenario.
pub fn main(allocator: &'static str) {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let options = Options::parse(args);
    // `--runtime current` runs a client on a single-threaded runtime instead of the default
    // multi-threaded one (what `#[tokio::main]` builds).
    let current_thread = options.get("runtime", String::from("multi")) == "current";
    let runtime = || {
        let mut builder = if current_thread {
            tokio::runtime::Builder::new_current_thread()
        } else {
            tokio::runtime::Builder::new_multi_thread()
        };
        builder
            .enable_all()
            .build()
            .expect("the tokio runtime starts")
    };
    match command.as_str() {
        "fixtures" => decode::write_fixtures(&options),
        "decode" => decode::run(&options, allocator),
        "mock-gateway" => runtime().block_on(mock_gateway::run(&options)),
        "mock-rest" => runtime().block_on(mock_rest::run()),
        "flood" => runtime().block_on(flood::client(&options, allocator)),
        "shard-flood" => runtime().block_on(flood::shard(&options, allocator)),
        "memory" => runtime().block_on(memory::run(&options, allocator)),
        "rest" => runtime().block_on(rest::run(&options, allocator)),
        _ => panic!(
            "unknown subcommand `{command}`; see the crate docs (fixtures, decode, mock-gateway, \
             mock-rest, flood, shard-flood, memory, rest)"
        ),
    }
}
