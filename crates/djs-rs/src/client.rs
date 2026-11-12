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

use djs_gateway::{
    Compression, Config, LocalIdentifyQueue, MessageSender, SendError, Shard, create_shards,
};
use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, payload::UpdatePresence};
use djs_rest::Rest;
use futures_core::Stream;
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
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
/// Cheap to [`Clone`]; clones share the same handlers and connection.
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
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    token: Box<str>,
    intents: Intents,
    rest: Rest,
    compression: Compression,
    gateway_url: Option<Box<str>>,
    shards: Shards,
    presence: Option<UpdatePresence>,
    mode: HandlerMode,
    /// The senders of the running shards (empty while not running).
    senders: Mutex<Vec<MessageSender>>,
    registry: RwLock<Arc<Registry>>,
    subscribers: Mutex<Vec<mpsc::UnboundedSender<ShardEvent>>>,
    has_subscribers: AtomicBool,
    running: AtomicBool,
    shutdown: watch::Sender<bool>,
}

impl core::fmt::Debug for Client {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Client")
            .field("intents", &self.inner.intents)
            .field("mode", &self.inner.mode)
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
        }
    }

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
        F: Fn(Context, Arc<E::Payload>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
    {
        let _ = event;
        let handler: Arc<dyn ErasedHandler> = Arc::new(Handler::<E, F>::new(handler));
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
        result
    }

    /// Drives one shard until shutdown or a fatal error.
    async fn run_shard(
        &self,
        mut shard: Shard,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), Error> {
        let ctx = Context::new(self.inner.rest.clone(), shard.id(), shard.sender());
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

    async fn dispatch(&self, ctx: &Context, event: DispatchEvent) {
        let inner = &*self.inner;
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

/// What [`Client::login`] starts: the gateway URL, the shards and the identify concurrency.
struct Plan {
    url: String,
    range: Range<u32>,
    total: u32,
    max_concurrency: u32,
}

/// Configures and builds a [`Client`]. Created by [`Client::builder`].
///
/// discord.js counterpart: `ClientOptions`.
#[derive(Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct ClientBuilder {
    token: String,
    intents: Intents,
    rest: Option<Rest>,
    compression: Compression,
    gateway_url: Option<Box<str>>,
    shards: Shards,
    presence: Option<UpdatePresence>,
    mode: HandlerMode,
}

impl ClientBuilder {
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

    /// Builds the client. It does not connect until [`Client::login`] is called.
    #[must_use]
    pub fn build(self) -> Client {
        let rest = self.rest.unwrap_or_else(|| Rest::new(self.token.clone()));
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
