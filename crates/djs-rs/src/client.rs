//! The high-level [`Client`].

use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context as TaskContext, Poll},
};
use std::{
    future::Future,
    sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock},
};

use djs_gateway::{Compression, Config, Shard};
use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, ShardId};
use djs_rest::Rest;
use futures_core::Stream;
use tokio::sync::{mpsc, watch};

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
    mode: HandlerMode,
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
    /// **Coverage:** Phase 1 runs a single shard. If Discord recommends more shards, a warning is
    /// logged; automatic sharding arrives in Phase 4.
    ///
    /// discord.js counterpart: `Client#login`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Rest`] if the gateway URL cannot be fetched (for example because the
    /// token is invalid), [`Error::Gateway`] on a fatal gateway close (invalid token, disallowed
    /// intents, …), or [`Error::AlreadyRunning`] if the client is already logged in.
    pub async fn login(&self) -> Result<(), Error> {
        if self.inner.running.swap(true, Ordering::AcqRel) {
            return Err(Error::AlreadyRunning);
        }
        let result = self.run().await;
        self.inner.running.store(false, Ordering::Release);
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
        let mut shutdown = inner.shutdown.subscribe();

        let gateway_url = self.gateway_url().await?;

        let config = Config::builder(&*inner.token, inner.intents)
            .gateway_url(gateway_url)
            .compression(inner.compression)
            .build();
        let mut shard = Shard::with_config(ShardId::ONE, config);
        let ctx = Context::new(inner.rest.clone(), shard.id(), shard.sender());

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
                    Some(Err(error)) => tracing::warn!(%error, "gateway error, recovering"),
                    None => return Ok(()),
                },
            }
        }
    }

    /// Returns the configured gateway URL, or asks Discord for it.
    async fn gateway_url(&self) -> Result<String, Error> {
        if let Some(url) = &self.inner.gateway_url {
            return Ok(url.to_string());
        }
        let gateway = self.inner.rest.get_gateway_bot().await?;
        if gateway.shards > 1 {
            tracing::warn!(
                recommended = gateway.shards,
                "Discord recommends more than one shard; this version runs a single shard"
            );
        }
        Ok(gateway.url.into())
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

    /// Connects to this gateway URL instead of asking Discord for it (useful for proxies and
    /// tests).
    pub fn gateway_url(mut self, url: impl Into<String>) -> Self {
        self.gateway_url = Some(url.into().into_boxed_str());
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
                mode: self.mode,
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
