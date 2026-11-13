//! Multi-process sharding (D-284): a [`ClusterManager`] runs a bot's shards in several
//! processes of the same executable (D-296), `shards_per_process` shards each (D-297).
//!
//! Behind the `sharding` feature. The bot has one entry point (D-300): [`process`] tells a
//! process the manager started (it gets its [`ClusterAssignment`], given to
//! [`ClientBuilder::cluster`](crate::ClientBuilder::cluster)) from the first process, which
//! builds the manager.
//!
//! ```no_run
//! use djs_rs::{prelude::*, sharding::{self, ClusterManager}};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let token = std::env::var("DISCORD_TOKEN")?;
//!     match sharding::process()? {
//!         Some(assignment) => {
//!             let client = Client::builder(token)
//!                 .intents(Intents::GUILDS)
//!                 .cluster(assignment)
//!                 .build();
//!             client.on(Events::ClientReady, |ctx, _| async move {
//!                 println!("shard {} is ready", ctx.shard_id());
//!                 Ok(())
//!             });
//!             client.login().await?;
//!         }
//!         None => {
//!             let manager = ClusterManager::builder(token).shards_per_process(2).build();
//!             manager.spawn().await?;
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! # How the processes talk
//!
//! Each process connects back to the manager over TCP on `127.0.0.1`, with frames of a
//! length and JSON, and presents a secret token the manager put in its environment (D-285):
//! a process the manager did not start cannot connect. The bot keeps its standard output.
//! When the manager asks a process to stop, or the manager goes away, the process's client
//! shuts down and [`Client::login`](crate::Client::login) returns.
//!
//! The manager asks Discord for the gateway's details once and passes them to the
//! processes (D-305), and it is their IDENTIFY queue (D-299): shards in different processes
//! never identify faster than Discord's `max_concurrency` allows.
//!
//! Each process has its own REST client, so the REST rate limits are not shared between
//! processes (D-302): what goes over Discord's limits ends in a 429, which the client waits
//! out and retries.
//!
//! discord.js counterpart: `ShardingManager`, `Shard` and `ShardClientUtil`; the names follow
//! discord-hybrid-sharding's clusters (D-301).

mod cluster;
mod ipc;
mod manager;

use std::sync::{Mutex, MutexGuard, PoisonError};

pub(crate) use self::cluster::Connection;
pub use self::{
    cluster::ClusterAssignment,
    manager::{Cluster, ClusterEvent, ClusterEvents, ClusterManager, ClusterManagerBuilder},
};

/// Tells whether this process was started by a [`ClusterManager`] (D-300): its assignment
/// (cluster ID, shards, the way to the manager) if it was, `None` in the first process.
///
/// The assignment comes from an environment variable the manager sets; it is checked
/// (shards within the total, the manager on a loopback address) but nothing is connected
/// yet: the client connects when it logs in.
///
/// discord.js counterpart: the environment `ShardClientUtil` reads.
///
/// # Errors
///
/// [`ClusterError::Assignment`] if the variable is set but is not a valid assignment.
pub fn process() -> Result<Option<ClusterAssignment>, ClusterError> {
    ClusterAssignment::from_env()
}

/// Locks `mutex`, ignoring poisoning: nothing is left half-updated by a panic here.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// An error of multi-process sharding.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClusterError {
    /// The manager could not ask Discord for the gateway's details (Get Gateway Bot, D-305).
    #[error("could not fetch the gateway's details")]
    Rest(#[from] djs_rest::Error),
    /// Discord's daily [session start limit] does not allow starting every shard; no process
    /// was started.
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
    /// Listening, connecting or starting a process failed.
    #[error("a cluster process or connection failed")]
    Io(#[source] std::io::Error),
    /// The assignment in the environment is not valid.
    #[error("the cluster assignment in the environment is not valid")]
    Assignment,
    /// The manager did not accept this process (a wrong token, an unknown or already
    /// connected cluster, or no answer in time).
    #[error("the cluster manager refused this process")]
    Refused,
    /// The manager is already running.
    #[error("the cluster manager is already running")]
    AlreadyRunning,
}
