//! The guide's "Sharding" chapter: a bot split over several processes by a cluster manager,
//! with a command every cluster answers and the clusters' statistics.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example sharded`; it starts
//! two shards in two processes (a small bot works with any number), and answers `!servers`
//! (Message Content intent) from any of them.

use djs_rs::{
    Shards,
    prelude::*,
    sharding::{self, ClusterAssignment, ClusterCommand, ClusterManager},
};

// ANCHOR: command
/// A command each cluster answers: the IDs of its shards. The name must be the same in
/// every process, which it is: they all run this program.
struct ShardIds;

impl ClusterCommand for ShardIds {
    const NAME: &'static str = "shard-ids";
    type Input = ();
    type Output = Vec<u32>;
}
// ANCHOR_END: command

// ANCHOR: cluster
/// The bot itself, in each process the manager starts.
async fn cluster(token: String, assignment: ClusterAssignment) -> Result<(), djs_rs::Error> {
    let client = Client::builder(token)
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
        .cluster(assignment)
        .build();
    let cluster = client.cluster().expect("a client built with `cluster`");

    // This process's answer to `ShardIds`.
    cluster.handle::<ShardIds, _, _>(|client, ()| async move {
        let cluster = client.cluster().expect("a cluster");
        Ok(cluster.shards().collect())
    });

    client.on(Events::MessageCreate, move |ctx, msg| {
        let cluster = cluster.clone();
        async move {
            if msg.content.trim() != "!servers" {
                return Ok(());
            }
            // Every cluster answers, this one included; one that is down fails alone.
            let stats = cluster.stats().await?;
            let guilds: u64 = stats
                .iter()
                .filter_map(|reply| reply.result.as_ref().ok()?.guilds)
                .sum();
            let shards = cluster.broadcast::<ShardIds>(()).await?;
            let answering = shards.iter().filter(|reply| reply.result.is_ok()).count();
            let answer = format!(
                "{guilds} servers over {} shards in {answering} processes; this is shard {}.",
                cluster.total_shards(),
                ctx.shard_id(),
            );
            msg.reply(&ctx, answer).await?;
            Ok(())
        }
    });
    client.login().await
}
// ANCHOR_END: cluster

// ANCHOR: main
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;
    // The manager starts this same program again for each cluster; `process` tells them
    // apart from the first one.
    if let Some(assignment) = sharding::process()? {
        cluster(token, assignment).await?;
        return Ok(());
    }
    // The first process: the manager.
    let manager = ClusterManager::builder(token)
        .shards(Shards::Count(2))
        .shards_per_process(1)
        .build();
    let stopping = manager.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            stopping.shutdown();
        }
    });
    // Returns once every process has exited.
    manager.spawn().await?;
    Ok(())
}
// ANCHOR_END: main
