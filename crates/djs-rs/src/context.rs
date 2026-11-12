//! The [`Context`] passed to event handlers.

use std::sync::Arc;

use djs_cache::{Backend, Cache, DashMapBackend};
use djs_gateway::MessageSender;
use djs_model::gateway::ShardId;
use djs_rest::Rest;

/// Everything an event handler needs to act: the REST client, the cache and the shard that
/// received the event.
///
/// Cheap to clone (a single reference count increment). `B` is the cache backend of the
/// [`Client`](crate::Client) (D-172); code that does not name it gets the default.
///
/// discord.js counterpart: the `client` reachable from every structure (`message.client`).
#[derive(Debug)]
pub struct Context<B: Backend = DashMapBackend> {
    inner: Arc<Inner<B>>,
}

#[derive(Debug)]
struct Inner<B: Backend> {
    rest: Rest,
    shard: ShardId,
    sender: MessageSender,
    cache: Option<Arc<Cache<B>>>,
}

impl<B: Backend> Clone for Context<B> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<B: Backend> Context<B> {
    pub(crate) fn new(
        rest: Rest,
        shard: ShardId,
        sender: MessageSender,
        cache: Option<Arc<Cache<B>>>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                rest,
                shard,
                sender,
                cache,
            }),
        }
    }

    /// Returns the REST client.
    #[must_use]
    pub fn rest(&self) -> &Rest {
        &self.inner.rest
    }

    /// Returns the cache, if the client has one ([`ClientBuilder::cache`](crate::ClientBuilder::cache),
    /// D-170). The event being handled is already applied to it (D-171).
    ///
    /// discord.js counterpart: the managers' `cache` collections (`client.guilds.cache`, …).
    #[must_use]
    pub fn cache(&self) -> Option<&Cache<B>> {
        self.inner.cache.as_deref()
    }

    /// Returns the ID of the shard that received the event.
    #[must_use]
    pub fn shard_id(&self) -> ShardId {
        self.inner.shard
    }

    /// Returns a sender for gateway commands on the shard that received the event.
    #[must_use]
    pub fn shard(&self) -> &MessageSender {
        &self.inner.sender
    }
}
