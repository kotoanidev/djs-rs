//! The high-level [`Client`].

use core::{
    ops::Range,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    task::{Context as TaskContext, Poll},
    time::Duration,
};
use std::{
    future::Future,
    sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock},
};

use djs_cache::{Backend, Cache, CacheConfig, DashMapBackend};
use djs_gateway::{
    Compression, Config, LocalIdentifyQueue, ReceiveErrorKind, SendError, Shard, ShardState,
    create_shards,
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
    collector::EndReason,
    context::Shared,
    dispatch::{ReadySeen, ShardTracker, convert},
    event::{Dispatch, Event, EventKind, ShardEvent, payload},
    handler::{ErasedHandler, Handler, HandlerError, Registry},
    router::{
        AutocompleteInteraction, CommandInteraction, ComponentInteraction, ModalInteraction, Route,
    },
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Logs a warning naming the events of `kinds` that none of `intents` covers: Discord never
/// sends them to this client (D-271).
pub(crate) fn warn_missing_intents(intents: Intents, kinds: impl IntoIterator<Item = EventKind>) {
    use core::fmt::Write as _;

    let mut missing = String::new();
    for kind in kinds {
        let needed = kind.intents();
        if !needed.is_empty() && !intents.intersects(needed) {
            let separator = if missing.is_empty() { "" } else { ", " };
            let _ = write!(missing, "{separator}{} ({needed:?})", kind.name());
        }
    }
    if !missing.is_empty() {
        tracing::warn!(
            events = %missing,
            "these events are never received without one of their intents; add them with \
             ClientBuilder::intents"
        );
    }
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
    registry: RwLock<Arc<Registry<B>>>,
    subscribers: Mutex<Vec<mpsc::UnboundedSender<ShardEvent>>>,
    has_subscribers: AtomicBool,
    running: AtomicBool,
    /// The shards of the current `login` and how many of them have been ready (D-224).
    shard_count: AtomicU32,
    ready_shards: AtomicU32,
    /// The application ID, the collectors and the shards' senders, shared with every
    /// [`Context`].
    shared: Arc<Shared>,
    compiled: Compiled<B>,
    /// The assignment of a cluster process (D-300).
    #[cfg(feature = "sharding")]
    cluster: Option<crate::sharding::ClusterAssignment>,
}

/// A future boxed inside this crate (D-287).
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The entry points of the client's code that depends on the cache backend, called through
/// function pointers (D-287). [`Client::builder`] — not generic — sets the ones compiled in
/// this crate for [`DashMapBackend`], and [`ClientBuilder::cache_backend`] the generic ones, so
/// a bot that keeps the default backend does not compile the client's event loop in its own
/// crate again on every edit.
struct Compiled<B: Backend> {
    build: fn(ClientBuilder<B>) -> Client<B>,
    login: for<'a> fn(&'a Client<B>) -> BoxFuture<'a, Result<(), Error>>,
}

impl<B: Backend> Compiled<B> {
    /// The entry points for `B`, compiled in the crate that names `B`.
    fn generic() -> Self {
        fn login<B: Backend>(client: &Client<B>) -> BoxFuture<'_, Result<(), Error>> {
            Box::pin(client.login_inner())
        }
        Self {
            build: ClientBuilder::build_inner,
            login: login::<B>,
        }
    }
}

impl<B: Backend> Clone for Compiled<B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for Compiled<B> {}

impl<B: Backend> core::fmt::Debug for Compiled<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Compiled").finish_non_exhaustive()
    }
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
        Self::builder_with(token.into())
    }

    /// [`builder`](Self::builder) without its generic argument, so that it — and with it the
    /// client's code for the default backend, through [`Compiled`] — is compiled in this crate
    /// only (D-287). Never inlined into a bot's crate, for the same reason.
    #[inline(never)]
    fn builder_with(token: String) -> ClientBuilder {
        ClientBuilder {
            token,
            intents: Intents::empty(),
            rest: None,
            compression: Compression::default(),
            gateway_url: None,
            shards: Shards::default(),
            presence: None,
            mode: HandlerMode::default(),
            cache: None,
            backend: DashMapBackend::new(),
            compiled: Compiled::generic(),
            #[cfg(feature = "sharding")]
            cluster: None,
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
        if self.inner.running.load(Ordering::Acquire) {
            // Registered while running: login already checked the others (D-271).
            warn_missing_intents(self.inner.intents, [E::KIND]);
        }
        let handler: Arc<dyn ErasedHandler<B>> = Arc::new(Handler::<E, F>::new(handler));
        let mut registry = self
            .inner
            .registry
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        *registry = Arc::new(registry.with(E::KIND, handler));
        self
    }

    /// Replaces the handler table with one changed by `change` (copy-on-write).
    fn change_registry(&self, change: impl FnOnce(&mut Registry<B>)) -> &Self {
        let mut registry = self
            .inner
            .registry
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let mut next = Registry::clone(&registry);
        change(&mut next);
        *registry = Arc::new(next);
        self
    }

    /// Routes the command `name` to `handler` (the [router](crate::router), D-251–D-253).
    ///
    /// `name` is the command's full name as Discord shows it, with single spaces: `"ping"`,
    /// `"mod ban"` (a subcommand), `"mod config set"` (a subcommand in a group), or the name
    /// of a user or message command. A route for `"mod"` or `"mod config"` also receives the
    /// subcommands below it that have no route of their own. Registering a name again
    /// replaces its handler; routes can be added while the client runs.
    ///
    /// The handler receives a [`CommandInteraction`](crate::router::CommandInteraction): the
    /// command's data (`get_string`, …) and its responses (`reply`, …). Like event handlers it
    /// returns `Result<(), HandlerError>`, and errors are logged. The router only routes:
    /// commands are registered with Discord through [`Context::commands`] (D-255), and
    /// [`Events::InteractionCreate`](crate::Events::InteractionCreate) still fires.
    ///
    /// discord.js has no counterpart (a `switch` on `interaction.commandName`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.command("ping", |ctx, cmd| async move {
    ///     cmd.reply(&ctx, "Pong!").await?;
    ///     Ok(())
    /// });
    /// client.command("mod ban", |ctx, cmd| async move {
    ///     let reason = cmd.get_string("reason").unwrap_or("no reason");
    ///     cmd.reply(&ctx, format!("Banned: {reason}")).await?;
    ///     Ok(())
    /// });
    /// ```
    pub fn command<F, Fut>(&self, name: &str, handler: F) -> &Self
    where
        F: Fn(Context<B>, CommandInteraction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
    {
        let route = Arc::new(Route::new(name, handler));
        self.change_registry(|registry| {
            registry.routes.commands.insert(name.into(), route);
        })
    }

    /// Routes the autocomplete interactions of the command `name` to `handler`; `name` is
    /// matched like in [`command`](Self::command). The handler receives an
    /// [`AutocompleteInteraction`](crate::router::AutocompleteInteraction) and answers with
    /// [`respond`](crate::ext::InteractionExt::respond).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::{model::{application_command::Choice, interaction::AutocompleteChoices}, prelude::*};
    /// # let client = Client::builder("token").build();
    /// client.autocomplete("search", |ctx, search| async move {
    ///     let typed = search.focused_value().unwrap_or_default();
    ///     let choices = vec![Choice::new(format!("{typed}…"), typed.into())];
    ///     search.respond(&ctx, AutocompleteChoices::String(choices)).await?;
    ///     Ok(())
    /// });
    /// ```
    pub fn autocomplete<F, Fut>(&self, name: &str, handler: F) -> &Self
    where
        F: Fn(Context<B>, AutocompleteInteraction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
    {
        let route = Arc::new(Route::new(name, handler));
        self.change_registry(|registry| {
            registry.routes.autocomplete.insert(name.into(), route);
        })
    }

    /// Routes the buttons and select menus whose custom ID is `name`, or starts with `name:`,
    /// to `handler` (D-254). What follows the `:` is the handler's
    /// [`args`](crate::router::ComponentInteraction::args): a button `delete:123` reaches the
    /// route `"delete"` with the arguments `123`. Registering a name again replaces its
    /// handler.
    ///
    /// # Panics
    ///
    /// Panics if `name` contains a `:` (it could never match).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.component("delete", |ctx, button| async move {
    ///     let message_id = button.args();
    ///     button.update(&ctx, format!("Deleted {message_id}")).await?;
    ///     Ok(())
    /// });
    /// ```
    #[track_caller]
    pub fn component<F, Fut>(&self, name: &str, handler: F) -> &Self
    where
        F: Fn(Context<B>, ComponentInteraction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
    {
        assert!(
            !name.contains(':'),
            "a component route name cannot contain `:`: {name:?}"
        );
        let route = Arc::new(Route::new(name, handler));
        self.change_registry(|registry| {
            registry.routes.components.insert(name.into(), route);
        })
    }

    /// Routes the modals whose custom ID is `name`, or starts with `name:`, to `handler`, like
    /// [`component`](Self::component). The handler receives a
    /// [`ModalInteraction`](crate::router::ModalInteraction) with the submitted fields.
    ///
    /// # Panics
    ///
    /// Panics if `name` contains a `:` (it could never match).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.modal("feedback", |ctx, modal| async move {
    ///     let text = modal.text_value("text").unwrap_or_default();
    ///     modal.reply(&ctx, format!("Thanks for: {text}")).await?;
    ///     Ok(())
    /// });
    /// ```
    #[track_caller]
    pub fn modal<F, Fut>(&self, name: &str, handler: F) -> &Self
    where
        F: Fn(Context<B>, ModalInteraction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
    {
        assert!(
            !name.contains(':'),
            "a modal route name cannot contain `:`: {name:?}"
        );
        let route = Arc::new(Route::new(name, handler));
        self.change_registry(|registry| {
            registry.routes.modals.insert(name.into(), route);
        })
    }

    /// The intents the registered handlers, routes and running collectors need (D-272): the
    /// union of their events' [`Event::INTENTS`], to print or compare with the client's.
    ///
    /// Requesting all of them receives every such event; a bot may still leave some out (for
    /// example `DIRECT_MESSAGES` when it only reads guild messages). It does not include
    /// [`Intents::MESSAGE_CONTENT`], which changes what message events contain rather than
    /// which arrive. The client's intents stay what [`ClientBuilder::intents`] set; at login
    /// it warns about events none of them covers (D-271).
    ///
    /// discord.js has no counterpart.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// let client = Client::builder("token").intents(Intents::GUILDS).build();
    /// client.on(Events::MessageReactionAdd, |_ctx, _reaction| async move { Ok(()) });
    /// assert!(client.required_intents().contains(Intents::GUILD_MESSAGE_REACTIONS));
    /// ```
    #[must_use]
    pub fn required_intents(&self) -> Intents {
        self.listened()
            .fold(Intents::empty(), |intents, kind| intents | kind.intents())
    }

    /// The events registered handlers, routes and running collectors listen to.
    fn listened(&self) -> impl Iterator<Item = EventKind> {
        let registry = self.registry();
        let collectors = Arc::clone(&self.inner.shared.collectors);
        EventKind::ALL.into_iter().filter(move |&kind| {
            !registry.get(kind).is_empty()
                || (kind == EventKind::InteractionCreate && !registry.routes.is_empty())
                || collectors.wants(kind)
        })
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
    pub fn login(&self) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        (self.inner.compiled.login)(self)
    }

    /// [`login`](Self::login)'s body, reached through [`Compiled`].
    async fn login_inner(&self) -> Result<(), Error> {
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
        let shards = self
            .inner
            .shared
            .shards
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        for (_, sender) in shards.iter() {
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
        self.inner.shared.shutdown.send_replace(true);
    }

    async fn run(&self) -> Result<(), Error> {
        let inner = &*self.inner;
        warn_missing_intents(inner.intents, self.listened());
        inner.shared.shutdown.send_replace(false);
        // Subscribed before anything can wait, so a shutdown during start-up is not missed.
        let shutdown = inner.shared.shutdown.subscribe();
        // A cluster process stays connected to its manager while it runs (D-300).
        #[cfg(feature = "sharding")]
        let _manager = match &inner.cluster {
            Some(assignment) => Some(assignment.connect(Arc::clone(&inner.shared)).await?),
            None => None,
        };

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

        inner
            .shard_count
            .store(plan.range.end - plan.range.start, Ordering::Release);
        inner.ready_shards.store(0, Ordering::Release);

        let mut sweepers = self.spawn_sweepers();
        let mut tasks = JoinSet::new();
        for shard in create_shards(plan.range, plan.total, config) {
            inner
                .shared
                .shards
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .push((shard.id().number(), shard.sender()));
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
                        inner.shared.shutdown.send_replace(true);
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
        inner
            .shared
            .shards
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        // The collectors waiting for this client's events end with it.
        inner.shared.collectors.end_all(EndReason::Shutdown);
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

    /// A shard's `READY`: keeps the application ID (D-245) and, at the last shard's first
    /// `READY`, emits `ClientReady` (D-224).
    fn on_ready(
        &self,
        seen: ReadySeen,
        counted_ready: &mut bool,
        registry: &Registry<B>,
        out: &mut Vec<Dispatch>,
    ) {
        // The same application on every shard and every login.
        let _ = self.inner.shared.application_id.set(seen.application_id);
        if *counted_ready {
            return;
        }
        *counted_ready = true;
        let ready_shards = self.inner.ready_shards.fetch_add(1, Ordering::AcqRel) + 1;
        if ready_shards == self.inner.shard_count.load(Ordering::Acquire)
            && let Some(ready) = seen.ready
            && self.wants(registry, EventKind::ClientReady)
        {
            out.push(Dispatch::ClientReady(ready));
        }
    }

    /// Drives one shard until shutdown or a fatal error, emitting its events and its lifecycle
    /// events (D-226).
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
            Arc::clone(&self.inner.shared),
        );
        let mut tracker = ShardTracker::default();
        let mut out = Vec::new();
        // Whether the session was established and not lost since (for `ShardReconnecting`).
        let mut active = false;
        // The dispatches received while resuming: Discord's replay (for `ShardResume`).
        let mut replayed = 0;
        // Whether this shard already counted towards `ClientReady`.
        let mut counted_ready = false;
        loop {
            let event = tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        shard.close().await;
                        return Ok(());
                    }
                    continue;
                }
                event = shard.next_event() => event,
            };
            let registry = self.registry();
            match event {
                Some(Ok(GatewayEvent::Dispatch(_, event))) => {
                    if shard.state() == ShardState::Resuming {
                        replayed += 1;
                    }
                    let resumed = matches!(event, DispatchEvent::Resumed);
                    convert(
                        event,
                        self.inner.cache.as_deref(),
                        &mut tracker,
                        |kind| self.wants(&registry, kind),
                        &mut out,
                    );
                    if resumed {
                        let replay = core::mem::take(&mut replayed);
                        if self.wants(&registry, EventKind::ShardResume) {
                            out.push(Dispatch::ShardResume(Arc::new(payload::ShardResume {
                                replayed: replay,
                            })));
                        }
                    }
                    if let Some(seen) = tracker.take_ready() {
                        self.on_ready(seen, &mut counted_ready, &registry, &mut out);
                    }
                }
                Some(Ok(GatewayEvent::InvalidSession(false))) => {
                    if self.wants(&registry, EventKind::Invalidated) {
                        out.push(Dispatch::Invalidated(Arc::new(payload::Invalidated)));
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(error)) if error.is_fatal() => {
                    if let ReceiveErrorKind::FatallyClosed { code } = error.kind() {
                        let disconnect = Dispatch::ShardDisconnect(Arc::new(*code));
                        self.emit(&ctx, &registry, disconnect).await;
                    }
                    return Err(error.into());
                }
                Some(Err(error)) => {
                    tracing::warn!(shard = %shard.id(), %error, "gateway error, recovering");
                    if self.wants(&registry, EventKind::ShardError) {
                        out.push(Dispatch::ShardError(Arc::new(error)));
                    }
                }
                None => return Ok(()),
            }
            // A lost connection shows as the shard leaving `Active`; it reconnects by itself.
            let state = shard.state();
            if active
                && !matches!(
                    state,
                    ShardState::Active | ShardState::Closed | ShardState::FatallyClosed(_)
                )
                && self.wants(&registry, EventKind::ShardReconnecting)
            {
                out.push(Dispatch::ShardReconnecting(Arc::new(
                    payload::ShardReconnecting,
                )));
            }
            active = state == ShardState::Active;
            for dispatch in out.drain(..) {
                self.emit(&ctx, &registry, dispatch).await;
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

    /// The current handlers (registration replaces the table, so this is a snapshot).
    fn registry(&self) -> Arc<Registry<B>> {
        Arc::clone(
            &self
                .inner
                .registry
                .read()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    /// Whether anyone listens to `kind`: a handler, or an event stream (which never receives
    /// [`Raw`](crate::event::Raw)).
    fn wants(&self, registry: &Registry<B>, kind: EventKind) -> bool {
        !registry.get(kind).is_empty()
            || (kind == EventKind::InteractionCreate && !registry.routes.is_empty())
            || self.inner.shared.collectors.wants(kind)
            || (kind != EventKind::Raw && self.inner.has_subscribers.load(Ordering::Acquire))
    }

    /// Delivers one event to the event streams and to its handlers (D-028).
    async fn emit(&self, ctx: &Context<B>, registry: &Registry<B>, dispatch: Dispatch) {
        let inner = &*self.inner;
        if !matches!(dispatch, Dispatch::Raw(_)) && inner.has_subscribers.load(Ordering::Acquire) {
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
        // The collectors watching the event's IDs (D-264); they do not take it from the
        // handlers (D-266).
        inner.shared.collectors.deliver(kind, &dispatch);
        let handlers = registry.get(kind);
        // The interaction routes run after the `InteractionCreate` handlers (D-255).
        let routed = match &dispatch {
            Dispatch::InteractionCreate(event) if !registry.routes.is_empty() => Some(event),
            _ => None,
        };
        match inner.mode {
            HandlerMode::Concurrent => {
                for handler in handlers {
                    handler.spawn(ctx, &dispatch);
                }
                if let Some(event) = routed {
                    registry.routes.spawn(ctx, event);
                }
            }
            HandlerMode::Sequential => {
                for handler in handlers {
                    if let Some(future) = handler.call(ctx, &dispatch) {
                        future.await;
                    }
                }
                if let Some(future) = routed.and_then(|event| registry.routes.call(ctx, event)) {
                    future.await;
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
    compiled: Compiled<B>,
    #[cfg(feature = "sharding")]
    cluster: Option<crate::sharding::ClusterAssignment>,
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

    /// Makes this client the one of a cluster process started by a
    /// [`ClusterManager`](crate::sharding::ClusterManager) (D-300): it runs the assignment's
    /// shards ([`Shards::Range`]), connects to the manager when it logs in, and shuts down
    /// when the manager asks or goes away. Get the assignment from
    /// [`sharding::process`](crate::sharding::process).
    ///
    /// discord.js counterpart: what `new Client()` reads from the environment in a shard
    /// process (`client.shard`).
    #[cfg(feature = "sharding")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sharding")))]
    pub fn cluster(mut self, assignment: crate::sharding::ClusterAssignment) -> Self {
        self.shards = Shards::Range {
            range: assignment.shards(),
            total: assignment.total_shards(),
        };
        self.cluster = Some(assignment);
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
            compiled: Compiled::generic(),
            #[cfg(feature = "sharding")]
            cluster: self.cluster,
        }
    }

    /// Builds the client. It does not connect until [`Client::login`] is called.
    #[must_use]
    pub fn build(self) -> Client<B> {
        (self.compiled.build)(self)
    }

    /// [`build`](Self::build)'s body, reached through [`Compiled`].
    fn build_inner(self) -> Client<B> {
        let compiled = self.compiled;
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
                registry: RwLock::new(Arc::new(Registry::default())),
                subscribers: Mutex::new(Vec::new()),
                has_subscribers: AtomicBool::new(false),
                running: AtomicBool::new(false),
                shard_count: AtomicU32::new(0),
                ready_shards: AtomicU32::new(0),
                shared: Arc::new(Shared {
                    sequential: self.mode == HandlerMode::Sequential,
                    intents: self.intents,
                    ..Shared::default()
                }),
                compiled,
                #[cfg(feature = "sharding")]
                cluster: self.cluster,
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
    use std::{
        io,
        sync::{Arc, Mutex},
    };

    use djs_model::gateway::Intents;

    use super::{every, warn_missing_intents};
    use crate::event::EventKind;

    /// A log writer into a shared buffer.
    #[derive(Clone, Default)]
    struct Logs(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Logs {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn events_without_their_intents_are_warned_about() {
        let logs = Logs::default();
        let writer = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            warn_missing_intents(
                Intents::GUILDS | Intents::DIRECT_MESSAGES,
                [
                    EventKind::MessageReactionAdd,
                    EventKind::GuildCreate,
                    EventKind::InteractionCreate,
                    // One of its two intents is enough.
                    EventKind::MessageCreate,
                    EventKind::PresenceUpdate,
                ],
            );
            warn_missing_intents(Intents::GUILDS, [EventKind::GuildCreate]);
        });
        let logs = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
        assert_eq!(logs.lines().count(), 1, "{logs}");
        assert!(logs.contains("messageReactionAdd"), "{logs}");
        assert!(logs.contains("presenceUpdate"), "{logs}");
        assert!(!logs.contains("guildCreate"), "{logs}");
        assert!(!logs.contains("messageCreate"), "{logs}");
    }

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
