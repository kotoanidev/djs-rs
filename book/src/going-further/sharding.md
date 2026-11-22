# Sharding

A gateway connection is a *shard*, and Discord sends each shard the events of a share of the
bot's servers. A bot in fewer than 2 500 servers may use one shard; above that Discord
requires more, about one per 1 000 servers. djs-rs shards by itself: the question is only
where the shards run.

This chapter's program is
[`sharded.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/sharded.rs).
It needs the `sharding` feature of djs-rs:

```toml
djs-rs = { version = "0.1.0-alpha.1", features = ["sharding"] }
```

## In one process

A client asks Discord how many shards the bot should have and runs them all, in its one
process: [`Shards::Auto`], the default. Nothing in a bot changes with the shard count. The
events of every shard reach the same handlers, `ctx.shard_id()` tells which shard an event
came from, and the client keeps Discord's limit on how fast shards may connect. One
process holds a lot of shards: the repository's benchmarks run a thousand in one.

`Shards::Count(n)` fixes the number, and `Shards::Range { range, total }` runs only some of
them, for a bot split over machines by hand.

(discord.js: the `shards: "auto"` client option, which is not the default there.)

## In several processes

A bot that outgrows one process (its memory, a restart of everything at once) runs its
shards in several. A [`ClusterManager`] starts them: one program that, run, either *is*
the manager or is one of the processes it started, a *cluster*:

```rust,no_run
{{#include ../../code/examples/sharded.rs:main}}
```

`sharding::process()` returns the process's [`ClusterAssignment`] (its shards) when the
manager started it, and `None` in the first process, which becomes the manager. The
manager asks Discord for the shard count once, starts the processes one after another
(`shards_per_process` shards each), passes their connections through one queue so they
never connect too fast, and restarts a process that fails.

(discord.js: `ShardingManager`, which runs a separate script.)

## A cluster

Each cluster is an ordinary client, given its assignment:

```rust,no_run
{{#include ../../code/examples/sharded.rs:cluster}}
```

Its cache holds only its own shards' servers, so a question about all of them goes to
every cluster. `cluster.stats()` asks each one for its servers, users, shards and memory.
For anything else, a bot declares a *command*:

```rust,no_run
{{#include ../../code/examples/sharded.rs:command}}
```

A command has a name and the types of its input and output, which travel between the
processes as JSON. `handle` answers it in each cluster, and `broadcast` sends it to every
cluster (`send` to one) and returns each one's answer, or why it gave none: a cluster that
is restarting fails alone instead of spoiling the others.

(discord.js: `broadcastEval` and `fetchClientValues`. Rust cannot run code sent to it at run
time, so the code a command runs is compiled into the bot.)

## Limits worth knowing

- Discord limits how many sessions a bot may start per day (1 000 for most bots); every
  shard's connection is one. The manager checks that enough are left before it starts.
- Each process has its own REST client, and Discord's rate limits are per bot: processes
  that send a lot at once see 429 answers, which the client waits out.
- The processes talk to the manager over `127.0.0.1`, with a secret token only the
  processes it started know.

## What's next

The next chapter joins voice channels.

[`Shards::Auto`]: ../api/djs_rs/enum.Shards.html#variant.Auto
[`ClusterManager`]: ../api/djs_rs/sharding/struct.ClusterManager.html
[`ClusterAssignment`]: ../api/djs_rs/sharding/struct.ClusterAssignment.html
