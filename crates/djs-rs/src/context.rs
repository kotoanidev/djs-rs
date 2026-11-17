//! The [`Context`] passed to event handlers.

use core::fmt;
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use djs_cache::{Backend, Cache, DashMapBackend};
#[cfg(feature = "gateway")]
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
    #[cfg(feature = "gateway")]
    pub(crate) shards: RwLock<Vec<(u32, MessageSender)>>,
    /// The bot's total shard count, across processes, of the current `login` (0 before).
    pub(crate) total_shards: core::sync::atomic::AtomicU32,
    /// Set to `true` to stop the running client ([`Client::shutdown`](crate::Client::shutdown),
    /// [`Context::shutdown`], D-293); `login` sets it back to `false`.
    pub(crate) shutdown: watch::Sender<bool>,
    /// A cluster process's open connection to its manager, for its outgoing requests
    /// (D-307).
    #[cfg(feature = "sharding")]
    pub(crate) link: std::sync::Mutex<Option<Arc<crate::sharding::Link>>>,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            application_id: OnceLock::new(),
            collectors: Arc::default(),
            sequential: false,
            intents: Intents::empty(),
            #[cfg(feature = "gateway")]
            shards: RwLock::default(),
            total_shards: core::sync::atomic::AtomicU32::new(0),
            shutdown: watch::channel(false).0,
            #[cfg(feature = "sharding")]
            link: std::sync::Mutex::new(None),
        }
    }
}

#[cfg(feature = "gateway")]
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
    #[cfg(feature = "gateway")]
    sender: MessageSender,
    shared: Arc<Shared>,
    /// The cache's operations, or the converter's without a cache (D-289).
    ops: Arc<dyn CacheOps>,
    /// Where an HTTP interaction's initial response goes (D-399); `None` for gateway events.
    #[cfg(feature = "http-interactions")]
    inline: Option<crate::http_interactions::Inline>,
}

impl fmt::Debug for Core {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Core");
        debug.field("rest", &self.rest).field("shard", &self.shard);
        #[cfg(feature = "gateway")]
        debug.field("sender", &self.sender);
        debug.field("shared", &self.shared).finish_non_exhaustive()
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

    /// Takes the slot of `interaction`'s initial response, if the context is that HTTP
    /// interaction's and nothing has responded yet (D-399).
    #[cfg(feature = "http-interactions")]
    pub(crate) fn take_inline(
        &self,
        interaction: djs_model::id::InteractionId,
    ) -> Option<crate::http_interactions::InitialSender> {
        self.inline.as_ref()?.take(interaction)
    }

    /// The sender of the shard that receives `guild`'s events, if it runs in this client.
    #[cfg(feature = "gateway")]
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
        #[cfg(feature = "gateway")] sender: MessageSender,
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
                    #[cfg(feature = "gateway")]
                    sender,
                    shared: state,
                    ops,
                    #[cfg(feature = "http-interactions")]
                    inline: None,
                },
                cache,
            }),
        }
    }

    /// A context for one HTTP interaction, whose initial response goes to `inline` (D-399).
    #[cfg(feature = "http-interactions")]
    pub(crate) fn http(
        rest: Rest,
        shard: ShardId,
        #[cfg(feature = "gateway")] sender: MessageSender,
        cache: Option<Arc<Cache<B>>>,
        state: Arc<Shared>,
        inline: crate::http_interactions::Inline,
    ) -> Self {
        #[cfg(feature = "gateway")]
        let mut ctx = Self::new(rest, shard, sender, cache, state);
        #[cfg(not(feature = "gateway"))]
        let mut ctx = Self::new(rest, shard, cache, state);
        if let Some(inner) = Arc::get_mut(&mut ctx.inner) {
            inner.core.inline = Some(inline);
        }
        ctx
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
    ///
    /// discord.js counterpart: `Client#rest`.
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

    #[cfg(feature = "gateway")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gateway")))]
    /// Returns the ID of the shard that received the event.
    ///
    /// discord.js counterpart: the `shardId` some discord.js events pass; `Guild#shardId`.
    #[must_use]
    pub fn shard_id(&self) -> ShardId {
        self.inner.core.shard
    }

    #[cfg(feature = "gateway")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gateway")))]
    /// Returns a sender for gateway commands on the shard that received the event.
    ///
    /// discord.js counterpart: `Guild#shard`.
    #[must_use]
    pub fn shard(&self) -> &MessageSender {
        &self.inner.core.sender
    }

    #[cfg(feature = "gateway")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gateway")))]
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
