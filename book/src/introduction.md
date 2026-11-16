# Introduction

djs-rs is a library for writing Discord bots in Rust. Its API follows
[discord.js](https://discord.js.org) v14 closely (the same clients, events, managers and
builders, under Rust names), so what you know from discord.js carries over, and what you
learn here carries over to discord.js's documentation when you need it. Under that API it is
built for speed: the gateway's events are decoded without needless copies, the cache can be
tuned or turned off, and every hot path is benchmarked.

This guide walks you from an empty project to a bot that answers commands, keeps state,
scales to many shards and talks in voice channels. Each chapter builds on the ones before
it, so if you are new to Discord bots, read them in order.

## What you need to know

The guide assumes you can read Rust and have written a little async code. If `async fn`,
`.await` or `Result` and `?` are new to you, two short detours will make everything here
easier:

- [The Rust Programming Language](https://doc.rust-lang.org/book/), chapters 1 to 9 and 17
  (async).
- The [tokio tutorial](https://tokio.rs/tokio/tutorial), up to "Spawning": djs-rs runs on
  tokio.

You do not need to know discord.js. When a djs-rs item has a discord.js counterpart, the
guide names it in passing ("discord.js: `Client#login`"), which helps if you are moving a
bot over, and can be skipped otherwise.

## How the guide is written

Every listing in this guide is part of a program that the project's tests compile, so the
code you read here compiles with the djs-rs it describes. The complete programs live in
[`book/code/examples`](https://github.com/kotoanidev/djs-rs/tree/main/book/code/examples):
run any of them with `cargo run -p djs-rs-guide --example <name>` from a checkout of the
repository.

## The state of djs-rs

djs-rs is **pre-alpha**: it works end to end (the guide's bots run against Discord), but it
is not on crates.io yet and its API can still change between commits. The
[roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md) says what is done and
what comes next. The [API reference](api/djs_rs/index.html) is published next to this
guide, for the current `main`; `cargo doc --open` in your project shows it for the exact
version you use.
