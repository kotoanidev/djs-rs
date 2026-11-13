//! The error type of the high-level client.

/// An error that stops [`Client::login`](crate::Client::login).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A REST request needed to start the client failed (for example fetching the gateway URL
    /// with an invalid token).
    #[error("a REST request failed")]
    Rest(#[from] djs_rest::Error),
    /// The gateway connection failed fatally (for example an invalid token or disallowed
    /// intents).
    #[error("the gateway connection failed")]
    Gateway(#[from] djs_gateway::ReceiveError),
    /// Discord's daily [session start limit] does not allow starting every shard; nothing was
    /// connected. The limit (usually 1 000 per day) counts every identify, including those of
    /// restarts.
    ///
    /// [session start limit]: https://docs.discord.com/developers/events/gateway#session-start-limit-object
    #[error(
        "not enough session starts left to start {required} shards ({remaining} remaining, \
         reset in {reset_after:?})"
    )]
    SessionStartLimit {
        /// The number of shards to start.
        required: u32,
        /// The session starts left until the reset.
        remaining: u32,
        /// The time until the limit resets.
        reset_after: core::time::Duration,
    },
    /// [`Client::login`](crate::Client::login) was called while the client was already running.
    #[error("the client is already logged in")]
    AlreadyRunning,
    /// The connection of a cluster process to its manager failed
    /// ([`ClientBuilder::cluster`](crate::ClientBuilder::cluster), D-300).
    #[cfg(feature = "sharding")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sharding")))]
    #[error("the connection to the cluster manager failed")]
    Cluster(#[from] crate::sharding::ClusterError),
}
