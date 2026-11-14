//! The cluster side of a [`Client`]: [`ClusterClient`] (D-301, D-307, D-308).

use core::{fmt, future::Future, ops::Range};

use djs_cache::{Backend, DashMapBackend};
use serde_json::Value;

use super::{
    ClusterAssignment, ClusterCommand, ClusterError, ClusterReply,
    command::{BoxFuture, Commands, Failure, decode_one, decode_replies, encode, erase},
};
use crate::{Client, HandlerError};

/// A cluster process's view of its cluster: its assignment, the commands it handles and the
/// requests it sends to the other clusters (D-301, D-307). Returned by
/// [`Client::cluster`] for a client built with
/// [`ClientBuilder::cluster`](crate::ClientBuilder::cluster).
///
/// Cheap to clone (a [`Client`] clone). Requests go through the manager, so they need the
/// client to be logged in; handlers can be registered at any time.
///
/// discord.js counterpart: `ShardClientUtil` (`client.shard`), and discord-hybrid-sharding's
/// `ClusterClient`.
///
/// # Examples
///
/// ```no_run
/// use djs_rs::{
///     prelude::*,
///     sharding::{self, ClusterCommand},
/// };
///
/// /// How many guilds a cluster has in its cache.
/// struct GuildCount;
///
/// impl ClusterCommand for GuildCount {
///     const NAME: &'static str = "guild_count";
///     type Input = ();
///     type Output = usize;
/// }
///
/// # async fn run(token: String) -> Result<(), Box<dyn std::error::Error>> {
/// let assignment = sharding::process()?.expect("started by a ClusterManager");
/// let client = Client::builder(token)
///     .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
///     .cluster(assignment)
///     .build();
/// let cluster = client.cluster().expect("built with `cluster`");
/// cluster.handle::<GuildCount, _, _>(|client, ()| async move {
///     Ok(client.cache().map_or(0, |cache| cache.guilds().len()))
/// });
/// client.on(Events::MessageCreate, move |ctx, msg| {
///     let cluster = cluster.clone();
///     async move {
///         if msg.content == "!guilds" {
///             let total: usize = cluster
///                 .broadcast::<GuildCount>(())
///                 .await?
///                 .into_iter()
///                 .filter_map(|reply| reply.result.ok())
///                 .sum();
///             msg.reply(&ctx, format!("{total} guilds")).await?;
///         }
///         Ok(())
///     }
/// });
/// client.login().await?;
/// # Ok(())
/// # }
/// ```
pub struct ClusterClient<B: Backend = DashMapBackend> {
    client: Client<B>,
}

impl<B: Backend> ClusterClient<B> {
    pub(crate) const fn new(client: Client<B>) -> Self {
        Self { client }
    }

    fn assignment(&self) -> &ClusterAssignment {
        self.client
            .assignment()
            .expect("a cluster client is only made for a cluster process")
    }

    /// This process's cluster ID.
    ///
    /// discord.js counterpart: `ShardClientUtil#ids` (the shards there).
    #[must_use]
    pub fn id(&self) -> u32 {
        self.assignment().cluster_id()
    }

    /// The shards this process runs.
    ///
    /// discord.js counterpart: `ShardClientUtil#ids`.
    #[must_use]
    pub fn shards(&self) -> Range<u32> {
        self.assignment().shards()
    }

    /// The bot's shard count, over every process.
    ///
    /// discord.js counterpart: `ShardClientUtil#count`.
    #[must_use]
    pub fn total_shards(&self) -> u32 {
        self.assignment().total_shards()
    }

    /// Handles the command `C` in this cluster (D-308): `handler` gets a [`Client`] clone and
    /// the sender's input, and returns the output or an error (sent back as text, a
    /// [`ClusterError::CommandFailed`]). Replaces an earlier handler of the same command.
    /// Each request runs in its own task, so a slow handler holds up nothing else; a
    /// handler that panics is reported as failed.
    ///
    /// discord.js counterpart: the function given to `broadcastEval`, which runs in every
    /// shard.
    pub fn handle<C, F, Fut>(&self, handler: F) -> &Self
    where
        C: ClusterCommand,
        F: Fn(Client<B>, C::Input) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<C::Output, HandlerError>> + Send + 'static,
    {
        self.client
            .add_cluster_command(C::NAME, erase::<C, _, _, _>(handler));
        self
    }

    /// Sends the command `C` to every cluster, this one included, and returns each one's
    /// result by cluster ID (D-309); see [`ClusterManager::broadcast`](super::ClusterManager::broadcast).
    ///
    /// discord.js counterpart: `ShardClientUtil#broadcastEval`.
    ///
    /// # Errors
    ///
    /// [`ClusterError::NotConnected`] if this process is not connected to its manager (the
    /// client is not logged in), [`ClusterError::Encoding`] if `input` cannot be converted
    /// to JSON.
    pub async fn broadcast<C: ClusterCommand>(
        &self,
        input: C::Input,
    ) -> Result<Vec<ClusterReply<C::Output>>, ClusterError> {
        let link = self.client.link().ok_or(ClusterError::NotConnected)?;
        let input = encode(&input)?;
        Ok(decode_replies(link.ask(None, C::NAME, input).await?))
    }

    /// Sends the command `C` to cluster `cluster` (this one too) and returns its result.
    ///
    /// discord.js counterpart: `ShardClientUtil#broadcastEval` with the `shard` option.
    ///
    /// # Errors
    ///
    /// As [`ClusterManager::send`](super::ClusterManager::send), and
    /// [`ClusterError::NotConnected`] if this process is not connected to its manager.
    pub async fn send<C: ClusterCommand>(
        &self,
        cluster: u32,
        input: C::Input,
    ) -> Result<C::Output, ClusterError> {
        let link = self.client.link().ok_or(ClusterError::NotConnected)?;
        let input = encode(&input)?;
        decode_one(link.ask(Some(cluster), C::NAME, input).await?)
    }
}

impl<B: Backend> Clone for ClusterClient<B> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl<B: Backend> fmt::Debug for ClusterClient<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClusterClient")
            .field("assignment", self.assignment())
            .finish_non_exhaustive()
    }
}

/// The commands a cluster's connection runs: the client's handlers.
pub(crate) struct Table<B: Backend> {
    pub(crate) client: Client<B>,
}

impl<B: Backend> Commands for Table<B> {
    fn call(&self, name: &str, input: Value) -> BoxFuture<'static, Result<Value, Failure>> {
        match self.client.cluster_command(name) {
            Some(handler) => handler(self.client.clone(), input),
            None => Box::pin(async { Err(Failure::UnknownCommand) }),
        }
    }
}
