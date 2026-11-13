//! The [`Context`] passed to event handlers.

use core::fmt;
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use djs_cache::{Backend, Cache, DashMapBackend};
use djs_gateway::MessageSender;
use djs_model::{
    gateway::{Intents, ShardId},
    id::{ApplicationId, GuildId},
};
use djs_rest::Rest;
use tokio::sync::watch;

use crate::{
    cache_ops::{CacheOps, converter},
    collector::Collectors,
};

/// What the client and every shard's [`Context`] share.
#[derive(Debug)]
pub(crate) struct Shared {
    /// Set by the client at the first `READY` (D-245).
    pub(crate) application_id: OnceLock<ApplicationId>,
    /// The running collectors (D-264).
    pub(crate) collectors: Arc<Collectors>,
    /// Whether handlers run one after another (collectors then warn, D-267).
    pub(crate) sequential: bool,
    /// The client's intents (collectors warn about missing ones, D-271).
    pub(crate) intents: Intents,
    /// The senders of the running shards, by shard number (empty while not running).
    pub(crate) shards: RwLock<Vec<(u32, MessageSender)>>,
    /// Set to `true` to stop the running client ([`Client::shutdown`](crate::Client::shutdown),
    /// [`Context::shutdown`], D-293); `login` sets it back to `false`.
    pub(crate) shutdown: watch::Sender<bool>,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            application_id: OnceLock::new(),
            collectors: Arc::default(),
            sequential: false,
            intents: Intents::empty(),
            shards: RwLock::default(),
            shutdown: watch::channel(false).0,
        }
    }
}

impl Shared {
    /// The sender of shard `number`, if it runs in this client.
    pub(crate) fn shard(&self, number: u32) -> Option<MessageSender> {
        self.shards
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|(shard, _)| *shard == number)
            .map(|(_, sender)| sender.clone())
    }
}

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
    core: Core,
    cache: Option<Arc<Cache<B>>>,
}

/// The part of a [`Context`] that does not depend on the cache backend: code that needs
/// only this is not generic, so it is compiled once, in djs-rs (D-289).
pub(crate) struct Core {
    rest: Rest,
    shard: ShardId,
    sender: MessageSender,
    shared: Arc<Shared>,
    /// The cache's operations, or the converter's without a cache (D-289).
    ops: Arc<dyn CacheOps>,
}

impl fmt::Debug for Core {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Core")
            .field("rest", &self.rest)
            .field("shard", &self.shard)
            .field("sender", &self.sender)
            .field("shared", &self.shared)
            .finish_non_exhaustive()
    }
}

impl Core {
    /// The running collectors (D-264).
    pub(crate) fn collectors(&self) -> &Arc<Collectors> {
        &self.shared.collectors
    }

    /// The ID of the shard that received the event.
    pub(crate) const fn shard_id(&self) -> ShardId {
        self.shard
    }

    /// The cache operations (D-289): the cache's, or a converter's that keeps nothing.
    pub(crate) fn ops(&self) -> &dyn CacheOps {
        &*self.ops
    }

    /// The sender of the shard that receives `guild`'s events, if it runs in this client.
    pub(crate) fn guild_shard(&self, guild: GuildId) -> Option<MessageSender> {
        let own = self.shard;
        let number = ShardId::for_guild(guild, own.total()).number();
        if number == own.number() {
            Some(self.sender.clone())
        } else {
            self.shared.shard(number)
        }
    }
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
        state: Arc<Shared>,
    ) -> Self {
        // Created here, in the client loop, so that for the default backend the cache
        // operations are compiled in djs-rs (D-289).
        let ops = match &cache {
            Some(cache) => Arc::clone(cache) as Arc<dyn CacheOps>,
            None => converter(),
        };
        Self {
            inner: Arc::new(Inner {
                core: Core {
                    rest,
                    shard,
                    sender,
                    shared: state,
                    ops,
                },
                cache,
            }),
        }
    }

    /// The part of the context that does not depend on the cache backend (D-289).
    pub(crate) fn core(&self) -> &Core {
        &self.inner.core
    }

    /// The cache operations (D-289): the cache's, or a converter's that keeps nothing.
    pub(crate) fn ops(&self) -> &dyn CacheOps {
        self.inner.core.ops()
    }

    /// The running collectors (D-264).
    pub(crate) fn collectors(&self) -> &Arc<Collectors> {
        self.inner.core.collectors()
    }

    /// Whether handlers run one after another (D-267).
    pub(crate) fn is_sequential(&self) -> bool {
        self.inner.core.shared.sequential
    }

    /// The client's intents.
    pub(crate) fn intents(&self) -> Intents {
        self.inner.core.shared.intents
    }

    /// Returns the bot's application ID, known from the first `READY` (D-245); `None` only in
    /// handlers that run before it (`ShardError`, `Invalidated`, `ShardDisconnect` while
    /// logging in).
    ///
    /// discord.js counterpart: `client.application.id`.
    #[must_use]
    pub fn application_id(&self) -> Option<ApplicationId> {
        self.inner.core.shared.application_id.get().copied()
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
        &self.inner.core.rest
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
        self.inner.core.shard
    }

    /// Returns a sender for gateway commands on the shard that received the event.
    #[must_use]
    pub fn shard(&self) -> &MessageSender {
        &self.inner.core.sender
    }

    /// Asks the client to disconnect, as [`Client::shutdown`](crate::Client::shutdown) does:
    /// every shard closes, the session is invalidated (the bot appears offline) and
    /// [`Client::login`](crate::Client::login) returns `Ok(())`. Has no effect once the client
    /// has stopped.
    ///
    /// discord.js counterpart: `client.destroy()` (through `message.client`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use djs_rs::{model::id::UserId, prelude::*};
    ///
    /// # fn register(client: &Client) {
    /// const OWNER: UserId = UserId::new(123_456_789_012_345_678);
    /// client.on(Events::MessageCreate, |ctx, msg| async move {
    ///     if msg.content == "!stop" && msg.author.id == OWNER {
    ///         msg.reply(&ctx, "Bye!").await?;
    ///         ctx.shutdown();
    ///     }
    ///     Ok(())
    /// });
    /// # }
    /// ```
    pub fn shutdown(&self) {
        self.inner.core.shared.shutdown.send_replace(true);
    }
}
