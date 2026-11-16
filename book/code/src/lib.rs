//! The code of the djs-rs guide (`book/`, D-378): each chapter's listings are an example in
//! `examples/`, which the chapter includes by its anchors (`ANCHOR: name` …
//! `ANCHOR_END: name`). Cargo builds the examples with the workspace's tests and lints, so a
//! listing that stops compiling fails the gate, not a reader.
//!
//! Run one with `cargo run -p djs-rs-guide --example <name>` (they read `DISCORD_TOKEN`).
