//! Creating the shards of a bot (D-141, D-145).

use core::ops::Range;

use djs_model::gateway::ShardId;

use crate::{Config, Shard};

/// Creates the shards numbered `range` of a bot that runs `total` shards.
///
/// Every shard gets a clone of `config`, so they all share its [IDENTIFY
/// queue](crate::ConfigBuilder::identify_queue) and take turns identifying. Pass `0..total` to
/// run every shard in this process, or a part of it to split the bot over several processes
/// (each process then needs a queue shared with the others, see
/// [`IdentifyQueue`](crate::IdentifyQueue)).
///
/// The shards do not connect until polled: run each one in its own task, calling
/// [`Shard::next_event`] in a loop. Discord recommends a shard count in Get Gateway Bot
/// (`shards`), together with the `max_concurrency` to create the queue with; Discord requires
/// sharding from 2 500 guilds on.
///
/// discord.js counterpart: the `shardCount` and `shardIds` options of `WebSocketManager` from
/// `@discordjs/ws`.
///
/// # Panics
///
/// Panics if `total` is `0` or `range` reaches past `total` (`range.end > total`).
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
///
/// use djs_gateway::{Config, LocalIdentifyQueue, create_shards};
/// use djs_model::gateway::{GatewayEvent, Intents};
///
/// # async fn run() {
/// let token = std::env::var("DISCORD_TOKEN").unwrap();
/// // `shards` and `max_concurrency` as returned by Get Gateway Bot.
/// let (total, max_concurrency) = (16, 16);
/// let config = Config::builder(token, Intents::GUILDS)
///     .identify_queue(Arc::new(LocalIdentifyQueue::new(max_concurrency)))
///     .build();
///
/// let mut tasks = Vec::new();
/// for mut shard in create_shards(0..total, total, config) {
///     tasks.push(tokio::spawn(async move {
///         while let Some(event) = shard.next_event().await {
///             match event {
///                 Ok(GatewayEvent::Dispatch(_, event)) => println!("{}: {}", shard.id(), event.name()),
///                 Ok(_) => {}
///                 Err(error) if error.is_fatal() => break,
///                 Err(error) => eprintln!("{}: {error}", shard.id()),
///             }
///         }
///     }));
/// }
/// for task in tasks {
///     task.await.unwrap();
/// }
/// # }
/// ```
#[track_caller]
pub fn create_shards(
    range: Range<u32>,
    total: u32,
    config: Config,
) -> impl ExactSizeIterator<Item = Shard> + DoubleEndedIterator {
    assert!(total > 0, "the shard total must not be zero");
    assert!(
        range.end <= total,
        "shard range {range:?} reaches past the total of {total}"
    );
    range.map(move |number| Shard::with_config(ShardId::new(number, total), config.clone()))
}
