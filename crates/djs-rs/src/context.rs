//! The [`Context`] passed to event handlers.

use std::sync::{Arc, OnceLock};

use djs_cache::{Backend, Cache, DashMapBackend};
use djs_gateway::MessageSender;
use djs_model::{gateway::ShardId, id::ApplicationId};
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
    /// Set by the client at the first `READY` (D-245), shared by every shard.
    application_id: Arc<OnceLock<ApplicationId>>,
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
        application_id: Arc<OnceLock<ApplicationId>>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                rest,
                shard,
                sender,
                cache,
                application_id,
            }),
        }
    }

    /// Returns the bot's application ID, known from the first `READY` (D-245); `None` only in
    /// handlers that run before it (`ShardError`, `Invalidated`, `ShardDisconnect` while
    /// logging in).
    ///
    /// discord.js counterpart: `client.application.id`.
    #[must_use]
    pub fn application_id(&self) -> Option<ApplicationId> {
        self.inner.application_id.get().copied()
    }

    /// The application ID for the application managers (D-247).
    #[track_caller]
    pub(crate) fn expect_application_id(&self) -> ApplicationId {
        self.application_id()
            .expect("the application ID is known from the first READY; this handler ran before it")
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
