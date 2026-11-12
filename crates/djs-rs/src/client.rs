//! The high-level [`Client`].

use core::{
    ops::Range,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context as TaskContext, Poll},
    time::Duration,
};
use std::{
    future::Future,
    sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock},
};

use djs_cache::{Backend, Cache, CacheConfig, DashMapBackend};
use djs_gateway::{
    Compression, Config, LocalIdentifyQueue, MessageSender, SendError, Shard, create_shards,
};
use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, payload::UpdatePresence};
use djs_rest::Rest;
use futures_core::Stream;
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
    time::{Instant, MissedTickBehavior},
};

use crate::{
    Context, Error,
    event::{Dispatch, Event, ShardEvent},
    handler::{ErasedHandler, Handler, HandlerError, Registry},
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How event handlers are executed (D-028).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HandlerMode {
    /// Every handler invocation runs in its own tokio task, so a slow handler never delays
    /// other events (the discord.js behaviour). Handlers may run in any order and concurrently.
    #[default]
    Concurrent,
    /// Handlers run one after another, in event order, on the shard's task. Use this when the
    /// order of events matters more than throughput; a slow handler delays all later events of
    /// its shard (and, if it blocks long enough, the shard's heartbeats).
    Sequential,
}

/// Which shards a [`Client`] runs (D-146).
///
/// A shard is one gateway connection; each carries the events of a part of the bot's guilds.
/// Discord requires sharding from 2 500 guilds on and recommends a shard count in Get Gateway
/// Bot. Every shard runs in its own tokio task, and shards take turns identifying as Discord's
/// `max_concurrency` allows.
///
/// discord.js counterpart: the `shards` and `shardCount` client options (where `'auto'` is
/// opt-in; here it is the default).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Shards {
    /// As many shards as Discord recommends, all in this process (the default).
    #[default]
    Auto,
    /// This many shards, all in this process.
    Count(u32),
    /// Only the shards numbered `range` out of `total`, to split a bot over several
    /// processes.
    ///
    /// Each process identifies its shards through its own queue, so start the processes one
    /// after another (or give them shards of different rate-limit buckets) to stay within
    /// Discord's identify limit.
    Range {
        /// The shards this process runs.
        range: Range<u32>,
        /// The bot's total shard count, across all processes.
        total: u32,
    },
}

/// A Discord bot client: connects to the gateway, keeps the session alive and dispatches events
/// to registered handlers.
///
/// Cheap to [`Clone`]; clones share the same handlers, connection and cache.
///
/// The client keeps a [`Cache`] only when one is configured with [`ClientBuilder::cache`]
/// (D-170); unlike discord.js, a bot that does not need one pays nothing for it. `B` is the
/// cache's storage [`Backend`] (D-172): code that does not name it gets [`DashMapBackend`],
/// and [`ClientBuilder::cache_backend`] picks another.
///
/// discord.js counterpart: `Client`.
///
/// # Examples
///
/// ```no_run
/// use djs_rs::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> Result<(), djs_rs::Error> {
///     let client = Client::builder(std::env::var("DISCORD_TOKEN").unwrap())
///         .intents(Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT)
///         .build();
///
///     client.on(Events::ClientReady, |_ctx, ready| async move {
///         println!("logged in as {}", ready.user.tag());
///         Ok(())
///     });
///
///     client.on(Events::MessageCreate, |ctx, msg| async move {
///         if msg.content == "!ping" {
///             msg.reply(&ctx, "Pong!").await?;
///         }
///         Ok(())
///     });
///
///     client.login().await
/// }
/// ```
pub struct Client<B: Backend = DashMapBackend> {
    inner: Arc<Inner<B>>,
}

struct Inner<B: Backend> {
    token: Box<str>,
    intents: Intents,
    rest: Rest,
    compression: Compression,
    gateway_url: Option<Box<str>>,
    shards: Shards,
    presence: Option<UpdatePresence>,
    mode: HandlerMode,
    cache: Option<Arc<Cache<B>>>,
    /// The senders of the running shards (empty while not running).
    senders: Mutex<Vec<MessageSender>>,
    registry: RwLock<Arc<Registry<B>>>,
    subscribers: Mutex<Vec<mpsc::UnboundedSender<ShardEvent>>>,
    has_subscribers: AtomicBool,
    running: AtomicBool,
    shutdown: watch::Sender<bool>,
}

impl<B: Backend> Clone for Client<B> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<B: Backend> core::fmt::Debug for Client<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Client")
            .field("intents", &self.inner.intents)
            .field("mode", &self.inner.mode)
            .field("cache", &self.inner.cache)
            .field("running", &self.inner.running.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Returns a builder for a client authenticated with `token` (without the `Bot ` prefix).
    pub fn builder(token: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            token: token.into(),
            intents: Intents::empty(),
            rest: None,
            compression: Compression::default(),
            gateway_url: None,
            shards: Shards::default(),
            presence: None,
            mode: HandlerMode::default(),
            cache: None,
            backend: DashMapBackend::new(),
        }
    }
}

impl<B: Backend> Client<B> {
    /// Registers a handler for an event.
    ///
    /// The handler receives a [`Context`] and the event payload behind an [`Arc`] (shared by all
    /// handlers of the event). It returns `Result<(), HandlerError>`: use `?` freely — errors are
    /// logged through `tracing` at `error` level. Handlers can be added at any time, including
    /// while the client is running.
    ///
    /// discord.js counterpart: `client.on(event, listener)`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.on(Events::MessageCreate, |ctx, msg| async move {
    ///     if msg.content == "!ping" {
    ///         msg.reply(&ctx, "Pong!").await?;
    ///     }
    ///     Ok(())
    /// });
    /// ```
    pub fn on<E, F, Fut>(&self, event: E, handler: F) -> &Self
    where
        E: Event,
        F: Fn(Context<B>, Arc<E::Payload>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
    {
        let _ = event;
        let handler: Arc<dyn ErasedHandler<B>> = Arc::new(Handler::<E, F>::new(handler));
        let mut registry = self
            .inner
            .registry
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        *registry = Arc::new(registry.with(E::KIND, handler));
        self
    }

    /// Returns a stream of every event the client receives (D-029).
    ///
    /// This is the alternative to [`on`](Self::on) for full control. Each call creates an
    /// independent subscriber that receives every event from the moment it was created.
    ///
    /// **Note:** the stream is unbounded — consume it promptly, or drop it, to avoid
    /// accumulating events in memory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::{event::Dispatch, prelude::*};
    /// # async fn run(client: Client) {
    /// let mut events = client.events();
    /// tokio::spawn(async move {
    ///     while let Some(event) = events.next().await {
    ///         if let Dispatch::MessageCreate(message) = event.event {
    ///             println!("{}", message.content);
    ///         }
    ///     }
    /// });
    /// client.login().await.unwrap();
    /// # }
    /// ```
    #[must_use]
    pub fn events(&self) -> EventStream {
        let (tx, rx) = mpsc::unbounded_channel();
        lock(&self.inner.subscribers).push(tx);
        self.inner.has_subscribers.store(true, Ordering::Release);
        EventStream { rx }
    }

    /// Returns the REST client.
    #[must_use]
    pub fn rest(&self) -> &Rest {
        &self.inner.rest
    }

    /// Returns the cache, or `None` if the client was built without one (the default, D-170).
    ///
    /// Every event is applied to the cache before its handlers run (D-171), so a handler sees
    /// the state after its event. Handlers reach the cache through [`Context::cache`] too.
    ///
    /// discord.js counterpart: the managers' `cache` collections (`client.guilds.cache`,
    /// `client.users.cache`, …), which always exist there.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use djs_rs::{cache::CacheConfig, prelude::*};
    ///
    /// let client = Client::builder("token")
    ///     .intents(Intents::GUILDS)
    ///     .cache(CacheConfig::default())
    ///     .build();
    ///
    /// client.on(Events::MessageCreate, |ctx, msg| async move {
    ///     let cache = ctx.cache().expect("the client has a cache");
    ///     if let Some(guild) = msg.guild_id.and_then(|id| cache.guilds().get(id)) {
    ///         println!("a message in {}", guild.name);
    ///     }
    ///     Ok(())
    /// });
    /// ```
    #[must_use]
    pub fn cache(&self) -> Option<&Cache<B>> {
        self.inner.cache.as_deref()
    }

    /// Connects to Discord and processes events until [`shutdown`](Self::shutdown) is called or
    /// the connection fails fatally.
    ///
    /// The connection heals itself: disconnects are resumed and failures retried with backoff.
    ///
    /// The client asks Discord ([Get Gateway Bot]) for the gateway URL, the recommended shard
    /// count and `max_concurrency`, then runs every shard chosen with
    /// [`ClientBuilder::shards`] in its own task. A fatal error on any shard stops them all.
    /// Get Gateway Bot is skipped only if both [`ClientBuilder::gateway_url`] and a fixed
    /// shard count are configured; shards then identify one at a time.
    ///
    /// [Get Gateway Bot]: https://docs.discord.com/developers/events/gateway#get-gateway-bot
    ///
    /// discord.js counterpart: `Client#login`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Rest`] if the gateway URL cannot be fetched (for example because the
    /// token is invalid), [`Error::SessionStartLimit`] if Discord's daily session start limit
    /// does not allow starting every shard, [`Error::Gateway`] on a fatal gateway close (invalid
    /// token, disallowed intents, …), or [`Error::AlreadyRunning`] if the client is already
    /// logged in.
    ///
    /// # Panics
    ///
    /// Re-raises a panic of a shard's task (only possible with [`HandlerMode::Sequential`], when
    /// a handler panics).
    pub async fn login(&self) -> Result<(), Error> {
        if self.inner.running.swap(true, Ordering::AcqRel) {
            return Err(Error::AlreadyRunning);
        }
        let result = self.run().await;
        self.inner.running.store(false, Ordering::Release);
        result
    }

    /// Updates the bot's presence (status and activities) on every running shard. Each shard
    /// also keeps it for its later sessions (D-148).
    ///
    /// Has no effect if the client is not running; set the presence to start with through
    /// [`ClientBuilder::presence`].
    ///
    /// discord.js counterpart: `ClientUser#setPresence`.
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Closed`] if a shard has stopped (the client is shutting down);
    /// the other shards are still updated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use djs_rs::{
    ///     model::gateway::{Activity, Status, payload::UpdatePresence},
    ///     prelude::*,
    /// };
    ///
    /// # let client = Client::builder("token").build();
    /// let handle = client.clone();
    /// client.on(Events::MessageCreate, move |_ctx, msg| {
    ///     let client = handle.clone();
    ///     async move {
    ///         if msg.content == "!busy" {
    ///             let busy = UpdatePresence::new(Status::DoNotDisturb, [Activity::custom("Busy")]);
    ///             client.set_presence(&busy)?;
    ///         }
    ///         Ok(())
    ///     }
    /// });
    /// ```
    pub fn set_presence(&self, presence: &UpdatePresence) -> Result<(), SendError> {
        let mut result = Ok(());
        for sender in lock(&self.inner.senders).iter() {
            if let Err(error) = sender.update_presence(presence) {
                result = result.and(Err(error));
            }
        }
        result
    }

    /// Asks a running client to disconnect; [`login`](Self::login) then returns `Ok(())`.
    ///
    /// The session is invalidated (the bot appears offline). Has no effect if the client is not
    /// running.
    ///
    /// discord.js counterpart: `Client#destroy`.
    pub fn shutdown(&self) {
        self.inner.shutdown.send_replace(true);
    }

    async fn run(&self) -> Result<(), Error> {
        let inner = &*self.inner;
        inner.shutdown.send_replace(false);
        // Subscribed before anything can wait, so a shutdown during start-up is not missed.
        let shutdown = inner.shutdown.subscribe();

        let plan = self.plan().await?;
        tracing::info!(
            shards = ?plan.range,
            total = plan.total,
            max_concurrency = plan.max_concurrency,
            "starting shards"
        );
        let mut config = Config::builder(&*inner.token, inner.intents)
            .gateway_url(plan.url)
            .compression(inner.compression)
            .identify_queue(Arc::new(LocalIdentifyQueue::new(plan.max_concurrency)));
        if let Some(presence) = &inner.presence {
            config = config.presence(presence.clone());
        }
        let config = config.build();

        let mut sweepers = self.spawn_sweepers();
        let mut tasks = JoinSet::new();
        for shard in create_shards(plan.range, plan.total, config) {
            lock(&inner.senders).push(shard.sender());
            let (client, shutdown) = (self.clone(), shutdown.clone());
            tasks.spawn(async move { client.run_shard(shard, shutdown).await });
        }

        let mut result = Ok(());
        while let Some(finished) = tasks.join_next().await {
            match finished {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    if result.is_ok() {
                        // A fatal error on one shard stops the whole client.
                        inner.shutdown.send_replace(true);
                        result = Err(error);
                    }
                }
                // Tasks are only cancelled when the set is dropped, so this is a panic.
                Err(error) => {
                    if let Ok(panic) = error.try_into_panic() {
                        std::panic::resume_unwind(panic);
                    }
                }
            }
        }
        lock(&inner.senders).clear();
        sweepers.shutdown().await;
        result
    }

    /// Spawns one task for each periodic sweep of the cache's [`SweepConfig`](djs_cache::SweepConfig)
    /// (D-167). The tasks run until the returned set is shut down or dropped.
    fn spawn_sweepers(&self) -> JoinSet<()> {
        let mut tasks = JoinSet::new();
        let Some(cache) = &self.inner.cache else {
            return tasks;
        };
        let sweepers = cache.config().sweepers();
        if let Some(sweep) = sweepers.messages {
            let cache = Arc::clone(cache);
            tasks.spawn(every(sweep.interval, "messages", move || {
                cache.sweep_messages_older_than(sweep.lifetime)
            }));
        }
        if let Some(sweep) = sweepers.threads {
            let cache = Arc::clone(cache);
            tasks.spawn(every(sweep.interval, "threads", move || {
                cache.sweep_archived_threads(sweep.lifetime)
            }));
        }
        tasks
    }

    /// Drives one shard until shutdown or a fatal error.
    async fn run_shard(
        &self,
        mut shard: Shard,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), Error> {
        let ctx = Context::new(
            self.inner.rest.clone(),
            shard.id(),
            shard.sender(),
            self.inner.cache.clone(),
        );
        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        shard.close().await;
                        return Ok(());
                    }
                }
                event = shard.next_event() => match event {
                    Some(Ok(GatewayEvent::Dispatch(_, event))) => self.dispatch(&ctx, event).await,
                    Some(Ok(_)) => {}
                    Some(Err(error)) if error.is_fatal() => return Err(error.into()),
                    Some(Err(error)) => {
                        tracing::warn!(shard = %shard.id(), %error, "gateway error, recovering");
                    }
                    None => return Ok(()),
                },
            }
        }
    }

    /// Works out the gateway URL, the shards to run and the identify concurrency, asking
    /// Discord unless both the URL and the shard count are configured.
    async fn plan(&self) -> Result<Plan, Error> {
        let inner = &*self.inner;
        let fixed = match &inner.shards {
            Shards::Auto => None,
            Shards::Count(count) => Some((0..*count, *count)),
            Shards::Range { range, total } => Some((range.clone(), *total)),
        };
        if let (Some(url), Some((range, total))) = (&inner.gateway_url, fixed.clone()) {
            return Ok(Plan {
                url: url.to_string(),
                range,
                total,
                max_concurrency: 1,
            });
        }

        let gateway = inner.rest.get_gateway_bot().await?;
        let (range, total) = fixed.unwrap_or_else(|| {
            let total = gateway.shards.max(1);
            (0..total, total)
        });
        let limit = &gateway.session_start_limit;
        let required = range.end - range.start;
        if limit.remaining < required {
            return Err(Error::SessionStartLimit {
                required,
                remaining: limit.remaining,
                reset_after: Duration::from_millis(limit.reset_after),
            });
        }
        let url = match &inner.gateway_url {
            Some(url) => url.to_string(),
            None => gateway.url.into(),
        };
        Ok(Plan {
            url,
            range,
            total,
            max_concurrency: limit.max_concurrency,
        })
    }

    async fn dispatch(&self, ctx: &Context<B>, event: DispatchEvent) {
        let inner = &*self.inner;
        // Handlers see the state after their event (D-171); the previous state reaches them
        // in Phase 8.
        if let Some(cache) = &inner.cache {
            drop(cache.update_dispatch(&event));
        }
        let dispatch = match event {
            DispatchEvent::MessageCreate(message) => Dispatch::MessageCreate(Arc::from(message)),
            DispatchEvent::Ready(ready) => Dispatch::Ready(Arc::from(ready)),
            DispatchEvent::Resumed => Dispatch::Resumed,
            DispatchEvent::Unknown(name) => Dispatch::Unknown(Arc::from(name.as_str())),
            other => Dispatch::Unknown(Arc::from(other.name())),
        };

        if inner.has_subscribers.load(Ordering::Acquire) {
            let mut subscribers = lock(&inner.subscribers);
            subscribers.retain(|subscriber| {
                subscriber
                    .send(ShardEvent {
                        shard: ctx.shard_id(),
                        event: dispatch.clone(),
                    })
                    .is_ok()
            });
            inner
                .has_subscribers
                .store(!subscribers.is_empty(), Ordering::Release);
        }

        let Some(kind) = dispatch.kind() else {
            return;
        };
        let registry = Arc::clone(
            &inner
                .registry
                .read()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let handlers = registry.get(kind);
        match inner.mode {
            HandlerMode::Concurrent => {
                for handler in handlers {
                    handler.spawn(ctx, &dispatch);
                }
            }
            HandlerMode::Sequential => {
                for handler in handlers {
                    if let Some(future) = handler.call(ctx, &dispatch) {
                        future.await;
                    }
                }
            }
        }
    }
}

/// Runs `sweep` every `period`, first after one `period`, logging what it swept as `resource`.
///
/// A zero period disables the sweep (it would run without pause); a period too long to
/// schedule never runs.
async fn every(period: Duration, resource: &'static str, sweep: impl Fn() -> usize) {
    if period.is_zero() {
        tracing::warn!(resource, "a cache sweep with a zero interval is disabled");
        return;
    }
    let Some(start) = Instant::now().checked_add(period) else {
        return;
    };
    let mut interval = tokio::time::interval_at(start, period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let swept = sweep();
        tracing::debug!(resource, swept, "swept the cache");
    }
}

/// What [`Client::login`] starts: the gateway URL, the shards and the identify concurrency.
struct Plan {
    url: String,
    range: Range<u32>,
    total: u32,
    max_concurrency: u32,
}

/// Configures and builds a [`Client`]. Created by [`Client::builder`].
///
/// `B` is the cache backend (D-172), switched with [`cache_backend`](Self::cache_backend).
///
/// discord.js counterpart: `ClientOptions`.
#[derive(Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct ClientBuilder<B: Backend = DashMapBackend> {
    token: String,
    intents: Intents,
    rest: Option<Rest>,
    compression: Compression,
    gateway_url: Option<Box<str>>,
    shards: Shards,
    presence: Option<UpdatePresence>,
    mode: HandlerMode,
    cache: Option<CacheConfig>,
    backend: B,
}

impl<B: Backend> ClientBuilder<B> {
    /// Sets the gateway intents — the groups of events to receive (default: none).
    ///
    /// Only request what the bot needs. Privileged intents
    /// ([`Intents::PRIVILEGED`]) must be enabled in the Developer Portal first.
    pub const fn intents(mut self, intents: Intents) -> Self {
        self.intents = intents;
        self
    }

    /// Uses a preconfigured REST client (for example with a proxy or custom timeouts) instead
    /// of one built from the token.
    pub fn rest(mut self, rest: Rest) -> Self {
        self.rest = Some(rest);
        self
    }

    /// Sets the gateway transport compression (default: `zstd-stream` when the feature is
    /// enabled).
    pub const fn compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    /// Connects to this gateway URL instead of the one from Get Gateway Bot (useful for
    /// proxies and tests).
    ///
    /// Get Gateway Bot is still called for [`Shards::Auto`]; with a fixed shard count too, the
    /// client does not call it at all and shards identify one at a time (`max_concurrency` 1).
    pub fn gateway_url(mut self, url: impl Into<String>) -> Self {
        self.gateway_url = Some(url.into().into_boxed_str());
        self
    }

    /// Sets which shards to run (default [`Shards::Auto`]: as many as Discord recommends).
    ///
    /// # Panics
    ///
    /// Panics on a shard count of `0`, or a range that is empty or reaches past `total`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rs::{Client, Shards};
    ///
    /// // Shards 0–7 of 16 run in this process; another process runs 8–15.
    /// let client = Client::builder("my-token")
    ///     .shards(Shards::Range { range: 0..8, total: 16 })
    ///     .build();
    /// ```
    #[track_caller]
    pub fn shards(mut self, shards: Shards) -> Self {
        match &shards {
            Shards::Auto => {}
            Shards::Count(count) => assert!(*count > 0, "the shard count must not be zero"),
            Shards::Range { range, total } => assert!(
                range.start < range.end && range.end <= *total,
                "shard range {range:?} must be non-empty and within the total of {total}"
            ),
        }
        self.shards = shards;
        self
    }

    /// Sets the presence (status and activities) every shard starts with (default: none, the
    /// bot appears online without an activity). Change it later with [`Client::set_presence`].
    ///
    /// discord.js counterpart: the `presence` client option.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rs::{
    ///     Client,
    ///     model::gateway::{Activity, Status, payload::UpdatePresence},
    /// };
    ///
    /// let client = Client::builder("my-token")
    ///     .presence(UpdatePresence::new(Status::Online, [Activity::playing("with Rust")]))
    ///     .build();
    /// ```
    pub fn presence(mut self, presence: UpdatePresence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// Sets how handlers are executed (default [`HandlerMode::Concurrent`]).
    pub const fn handler_mode(mut self, mode: HandlerMode) -> Self {
        self.mode = mode;
        self
    }

    /// Gives the client a [`Cache`] configured by `config` (default: no cache, D-170).
    ///
    /// The client applies every event to the cache before running its handlers (D-171), and
    /// runs the periodic sweeps of [`CacheConfig::sweepers`] while it is logged in. Read it
    /// with [`Client::cache`] and [`Context::cache`]. What the cache can fill depends on the
    /// [intents](Self::intents): members need [`Intents::GUILD_MEMBERS`], presences
    /// [`Intents::GUILD_PRESENCES`], messages [`Intents::GUILD_MESSAGES`] or
    /// [`Intents::DIRECT_MESSAGES`], and so on.
    ///
    /// discord.js counterpart: the `makeCache` and `sweepers` client options. discord.js
    /// always caches; [`CacheConfig::discord_js`] caches what it caches by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rs::{
    ///     Client,
    ///     cache::{CacheConfig, Limit, Resource},
    ///     model::gateway::Intents,
    /// };
    ///
    /// let config = CacheConfig::builder()
    ///     .members(Limit::Unlimited)
    ///     .messages(Limit::Max(50))
    ///     .build();
    /// let client = Client::builder("my-token")
    ///     .intents(Intents::GUILDS | Intents::GUILD_MEMBERS | Intents::GUILD_MESSAGES)
    ///     .cache(config)
    ///     .build();
    /// let cache = client.cache().unwrap();
    /// assert!(cache.config().is_enabled(Resource::Member));
    /// assert!(!cache.config().is_enabled(Resource::Presence));
    /// ```
    pub fn cache(mut self, config: CacheConfig) -> Self {
        self.cache = Some(config);
        self
    }

    /// Keeps the cache in stores created by `backend` instead of the default
    /// [`DashMapBackend`] (D-172), and enables the cache: with
    /// [`CacheConfig::default`] unless [`cache`](Self::cache) is called too (in any order).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rs::{Client, cache::DashMapBackend};
    ///
    /// // Any `djs_cache::Backend` works; this is the default one, named explicitly.
    /// let client: Client<DashMapBackend> = Client::builder("my-token")
    ///     .cache_backend(DashMapBackend::new())
    ///     .build();
    /// assert!(client.cache().is_some());
    /// ```
    pub fn cache_backend<C: Backend>(self, backend: C) -> ClientBuilder<C> {
        ClientBuilder {
            token: self.token,
            intents: self.intents,
            rest: self.rest,
            compression: self.compression,
            gateway_url: self.gateway_url,
            shards: self.shards,
            presence: self.presence,
            mode: self.mode,
            cache: Some(self.cache.unwrap_or_default()),
            backend,
        }
    }

    /// Builds the client. It does not connect until [`Client::login`] is called.
    #[must_use]
    pub fn build(self) -> Client<B> {
        let rest = self.rest.unwrap_or_else(|| Rest::new(self.token.clone()));
        let backend = self.backend;
        let cache = self
            .cache
            .map(|config| Arc::new(Cache::with_backend(config, backend)));
        Client {
            inner: Arc::new(Inner {
                token: self.token.into_boxed_str(),
                intents: self.intents,
                rest,
                compression: self.compression,
                gateway_url: self.gateway_url,
                shards: self.shards,
                presence: self.presence,
                mode: self.mode,
                cache,
                senders: Mutex::new(Vec::new()),
                registry: RwLock::new(Arc::new(Registry::default())),
                subscribers: Mutex::new(Vec::new()),
                has_subscribers: AtomicBool::new(false),
                running: AtomicBool::new(false),
                shutdown: watch::channel(false).0,
            }),
        }
    }
}

/// A stream of every event a [`Client`] receives. Created by [`Client::events`].
///
/// Implements [`Stream`]; [`next`](Self::next) is provided so no extension trait is needed.
#[derive(Debug)]
pub struct EventStream {
    rx: mpsc::UnboundedReceiver<ShardEvent>,
}

impl EventStream {
    /// Waits for the next event. Returns `None` once the client has been dropped.
    pub async fn next(&mut self) -> Option<ShardEvent> {
        self.rx.recv().await
    }
}

impl Stream for EventStream {
    type Item = ShardEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<ShardEvent>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::every;

    #[tokio::test]
    async fn sweeps_that_cannot_be_scheduled_return_at_once() {
        for period in [Duration::ZERO, Duration::MAX] {
            let sweep = every(period, "test", || unreachable!("must not sweep"));
            tokio::time::timeout(Duration::from_secs(5), sweep)
                .await
                .unwrap();
        }
    }
}
