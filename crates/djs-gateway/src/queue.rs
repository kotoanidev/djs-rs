//! The IDENTIFY queue shared by shards (D-141, D-144).

use core::{fmt, future::Future, pin::Pin, time::Duration};
use std::sync::Arc;

use djs_model::gateway::ShardId;
use tokio::{
    sync::Mutex,
    time::{Instant, sleep_until},
};

/// Minimum time between two identifies of the same rate-limit bucket. Discord allows one per
/// 5 seconds; the extra 100 ms absorbs network jitter.
pub(crate) const IDENTIFY_INTERVAL: Duration = Duration::from_millis(5_100);

/// Decides when a shard may send its IDENTIFY payload.
///
/// Discord lets a bot start [`max_concurrency`] sessions every 5 seconds: shard `n` belongs to
/// the rate-limit bucket `n % max_concurrency`, and each bucket accepts one IDENTIFY per
/// 5 seconds. Identifying too fast gets the connection closed. Every [`Shard`] waits for its
/// turn here before identifying — on the first connection and whenever its session has to be
/// started again (resuming does not identify).
///
/// [`LocalIdentifyQueue`] coordinates the shards of one process and is the default. Implement
/// this trait to coordinate several processes, for example through a shared store.
///
/// discord.js counterpart: `IIdentifyThrottler` from `@discordjs/ws`.
///
/// [`max_concurrency`]: https://docs.discord.com/developers/events/gateway#session-start-limit-object
/// [`Shard`]: crate::Shard
///
/// # Examples
///
/// A queue that lets every shard identify at once — only correct if something else (here, a
/// proxy in front of the gateway) does the throttling:
///
/// ```
/// use core::{future::Future, pin::Pin};
///
/// use djs_gateway::IdentifyQueue;
/// use djs_model::gateway::ShardId;
///
/// #[derive(Debug)]
/// struct ProxyThrottles;
///
/// impl IdentifyQueue for ProxyThrottles {
///     fn wait_turn(&self, _shard: ShardId) -> Pin<Box<dyn Future<Output = ()> + Send>> {
///         Box::pin(async {})
///     }
/// }
/// ```
pub trait IdentifyQueue: fmt::Debug + Send + Sync {
    /// Returns a future that completes when `shard` may identify.
    ///
    /// The shard sends IDENTIFY right after the future completes, so an implementation should
    /// count the identify as made at that moment. The future is dropped without completing if
    /// the shard's connection closes while it waits; the shard then leaves the queue and asks
    /// again on its next connection.
    fn wait_turn(&self, shard: ShardId) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

/// The in-process [`IdentifyQueue`]: one IDENTIFY per rate-limit bucket every 5 seconds.
///
/// Shards of the same bucket identify one after another, in the order they asked; buckets
/// are independent. Share one queue (behind an [`Arc`]) between all shards of a process —
/// [`create_shards`](crate::create_shards) and cloned [`Config`](crate::Config)s do that.
///
/// discord.js counterpart: `SimpleIdentifyThrottler` from `@discordjs/ws`.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use djs_gateway::{Config, LocalIdentifyQueue};
/// use djs_model::gateway::Intents;
///
/// // `max_concurrency` comes from Get Gateway Bot's `session_start_limit`.
/// let queue = Arc::new(LocalIdentifyQueue::new(16));
/// let config = Config::builder("my-token", Intents::GUILDS)
///     .identify_queue(queue)
///     .build();
/// ```
#[derive(Clone)]
pub struct LocalIdentifyQueue {
    /// When each bucket last let a shard identify. The mutex is fair, so waiting shards are
    /// served in order.
    buckets: Arc<[Mutex<Option<Instant>>]>,
}

impl LocalIdentifyQueue {
    /// Creates a queue for a bot whose `max_concurrency` (from Get Gateway Bot's
    /// `session_start_limit`) is the given value. `0` is treated as `1`.
    #[must_use]
    pub fn new(max_concurrency: u32) -> Self {
        let buckets = (0..max_concurrency.max(1))
            .map(|_| Mutex::new(None))
            .collect();
        Self { buckets }
    }

    /// Returns the number of rate-limit buckets (the `max_concurrency` the queue was created
    /// with, at least `1`).
    #[must_use]
    pub fn max_concurrency(&self) -> u32 {
        // Created from a `u32`, so this never saturates.
        u32::try_from(self.buckets.len()).unwrap_or(u32::MAX)
    }
}

impl Default for LocalIdentifyQueue {
    /// A queue with `max_concurrency` 1: one IDENTIFY every 5 seconds.
    fn default() -> Self {
        Self::new(1)
    }
}

impl fmt::Debug for LocalIdentifyQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalIdentifyQueue")
            .field("max_concurrency", &self.buckets.len())
            .finish()
    }
}

impl IdentifyQueue for LocalIdentifyQueue {
    fn wait_turn(&self, shard: ShardId) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let buckets = Arc::clone(&self.buckets);
        let bucket = shard.number() as usize % buckets.len();
        Box::pin(async move {
            let mut last = buckets[bucket].lock().await;
            if let Some(last) = *last {
                sleep_until(last + IDENTIFY_INTERVAL).await;
            }
            *last = Some(Instant::now());
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{IDENTIFY_INTERVAL, IdentifyQueue, Instant, LocalIdentifyQueue, ShardId};

    async fn turn(queue: &LocalIdentifyQueue, number: u32) -> Instant {
        queue.wait_turn(ShardId::new(number, 64)).await;
        Instant::now()
    }

    // No sockets here, so tokio's paused clock is reliable.
    #[tokio::test(start_paused = true)]
    async fn one_identify_per_bucket_every_interval() {
        let queue = LocalIdentifyQueue::new(4);
        let start = Instant::now();
        // Shards 0..8: two per bucket.
        let mut turns = Vec::new();
        for number in 0..8 {
            turns.push(tokio::spawn({
                let queue = queue.clone();
                async move { turn(&queue, number).await }
            }));
        }
        let mut at = Vec::new();
        for task in turns {
            at.push(task.await.unwrap() - start);
        }
        // The first shard of every bucket identifies at once, the second one interval later.
        assert!(at[..4].iter().all(|&after| after.is_zero()), "{at:?}");
        assert!(
            at[4..].iter().all(|&after| after == IDENTIFY_INTERVAL),
            "{at:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_shard_waits_for_its_own_previous_identify() {
        let queue = LocalIdentifyQueue::default();
        let first = turn(&queue, 0).await;
        let again = turn(&queue, 0).await;
        assert_eq!(again - first, IDENTIFY_INTERVAL);
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_waits_take_no_turn() {
        let queue = LocalIdentifyQueue::new(1);
        let start = turn(&queue, 0).await;
        // A shard that gives up while waiting (its connection closed) takes no turn.
        let abandoned = queue.wait_turn(ShardId::new(1, 64));
        let waited = tokio::time::timeout(IDENTIFY_INTERVAL / 2, abandoned).await;
        assert!(waited.is_err(), "the turn came too early");
        let next = turn(&queue, 2).await;
        assert_eq!(next - start, IDENTIFY_INTERVAL);
        let after = turn(&queue, 3).await;
        assert_eq!(after - start, IDENTIFY_INTERVAL * 2);
    }

    #[test]
    fn zero_concurrency_means_one_bucket() {
        assert_eq!(LocalIdentifyQueue::new(0).max_concurrency(), 1);
        assert_eq!(LocalIdentifyQueue::new(16).max_concurrency(), 16);
    }
}
