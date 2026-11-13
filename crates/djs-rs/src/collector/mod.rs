//! Collectors: streams of the messages, reactions, component interactions and modal
//! submissions that arrive while a bot waits for them (D-263–D-269).
//!
//! A collector watches one thing — the messages of a channel, the reactions or components of
//! a message, the components of an interaction's reply, the modals a user submits — and
//! yields what arrives there as a [`Stream`]: read it with `while let Some(item) =
//! collector.next().await`. It is configured with a [`CollectorBuilder`]:
//!
//! - [`time`](CollectorBuilder::time): end after this long;
//! - [`idle`](CollectorBuilder::idle): end when nothing was collected for this long;
//! - [`max`](CollectorBuilder::max): end after collecting this many;
//! - [`max_processed`](CollectorBuilder::max_processed): end after this many arrived, whether
//!   the filter accepted them or not;
//! - [`filter`](CollectorBuilder::filter): collect only what the filter accepts;
//! - for reactions and components, `max_users`; for reactions, `max_emojis` and
//!   `dispose` (also yield removals); for components, `component_type`.
//!
//! The collector also ends when what it watches is deleted (its message, channel, thread or
//! guild), when [`Collector::stop`] is called, or when the client shuts down; afterwards
//! [`Collector::end_reason`] tells why. The `await_*` shortcuts wait for one item
//! ([`await_component`](crate::ext::MessageExt::await_component),
//! [`await_modal_submit`](crate::ext::InteractionExt::await_modal_submit): a
//! `Result<_, CollectorError>`) or collect until the end
//! ([`await_messages`](crate::Context::await_messages),
//! [`await_reactions`](crate::ext::MessageExt::await_reactions): a [`Collected`]).
//!
//! Collected events still reach the event handlers and the router's routes (D-266): a
//! collector is one more listener, as in discord.js, so a bot must not answer the same
//! interaction from a collector and from a route.
//!
//! # `Sequential` handlers
//!
//! With [`HandlerMode::Sequential`](crate::HandlerMode::Sequential), a handler that waits for
//! a collector blocks its shard: the collector receives nothing from that shard until it
//! ends by time (D-267). The client logs a warning when a collector is created in this mode;
//! the default `Concurrent` mode has no such limit.
//!
//! # Examples
//!
//! ```no_run
//! # use std::time::Duration;
//! # use djs_rs::prelude::*;
//! # let client = Client::builder("token").build();
//! client.on(Events::MessageCreate, |ctx, msg| async move {
//!     if msg.content != "!quiz" {
//!         return Ok(());
//!     }
//!     msg.reply(&ctx, "What is 6 × 7? You have 30 seconds.").await?;
//!     let author = msg.author.id;
//!     let answers = ctx
//!         .await_messages(msg.channel_id)
//!         .filter(move |answer| answer.author.id == author)
//!         .max(1)
//!         .time(Duration::from_secs(30))
//!         .await;
//!     match answers.first() {
//!         Some(answer) if answer.content.trim() == "42" => answer.reply(&ctx, "Correct!").await?,
//!         Some(answer) => answer.reply(&ctx, "No, it is 42.").await?,
//!         None => msg.reply(&ctx, "Time is up: it was 42.").await?,
//!     };
//!     Ok(())
//! });
//! ```
//!
//! discord.js counterpart: `Collector` and its subclasses `MessageCollector`,
//! `ReactionCollector` and `InteractionCollector`.

mod index;

use core::{
    fmt,
    future::{Future, IntoFuture},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context as TaskContext, Poll},
    time::Duration,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    component::ComponentType,
    emoji::PartialEmoji,
    gateway::Intents,
    gateway::payload::{
        MessageCreate, MessageReactionAdd, MessageReactionRemove, MessageReactionRemoveAll,
    },
    id::{ChannelId, GuildId, MessageId, UserId},
};
use futures_core::Stream;
use tokio::{
    sync::mpsc,
    time::{Instant, Sleep},
};

pub(crate) use self::index::{
    Collectors, Key, MemberReply, NONCE_PREFIX, Registration, Signal, Sink,
};
use crate::{
    Context,
    router::{ComponentInteraction, ModalInteraction},
};

/// Why a collector ended.
///
/// discord.js counterpart: the `reason` of the `end` event (`"time"`, `"limit"`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EndReason {
    /// The [`time`](CollectorBuilder::time) elapsed (discord.js: `"time"`).
    Time,
    /// Nothing was collected for the [`idle`](CollectorBuilder::idle) time (`"idle"`).
    Idle,
    /// [`max`](CollectorBuilder::max) items were collected (`"limit"`).
    Limit,
    /// [`max_processed`](CollectorBuilder::max_processed) items arrived (`"processedLimit"`).
    ProcessedLimit,
    /// `max_users` different users were collected (`"userLimit"`).
    UserLimit,
    /// `max_emojis` different emojis were collected (`"emojiLimit"`).
    EmojiLimit,
    /// The watched message was deleted (`"messageDelete"`).
    MessageDelete,
    /// The channel was deleted (`"channelDelete"`).
    ChannelDelete,
    /// The thread was deleted (`"threadDelete"`).
    ThreadDelete,
    /// The guild was deleted, or the bot left it (`"guildDelete"`).
    GuildDelete,
    /// [`Collector::stop`] was called (discord.js: `"user"`, the default reason of `stop`).
    Stopped,
    /// The client shut down.
    Shutdown,
}

impl EndReason {
    /// A short description, as in the error message of [`CollectorError`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Time => "the time elapsed",
            Self::Idle => "nothing was collected for the idle time",
            Self::Limit => "the limit was reached",
            Self::ProcessedLimit => "the processed limit was reached",
            Self::UserLimit => "the user limit was reached",
            Self::EmojiLimit => "the emoji limit was reached",
            Self::MessageDelete => "the message was deleted",
            Self::ChannelDelete => "the channel was deleted",
            Self::ThreadDelete => "the thread was deleted",
            Self::GuildDelete => "the guild was deleted",
            Self::Stopped => "the collector was stopped",
            Self::Shutdown => "the client shut down",
        }
    }
}

impl fmt::Display for EndReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The error of the shortcuts that wait for one item
/// ([`await_component`](crate::ext::MessageExt::await_component),
/// [`await_modal_submit`](crate::ext::InteractionExt::await_modal_submit)): the collector
/// ended before anything was collected (D-268).
///
/// discord.js counterpart: the `InteractionCollectorError` rejection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollectorError {
    reason: EndReason,
}

impl CollectorError {
    /// Why the collector ended.
    #[must_use]
    pub const fn reason(self) -> EndReason {
        self.reason
    }
}

impl fmt::Display for CollectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nothing was collected: {}", self.reason)
    }
}

impl std::error::Error for CollectorError {}

/// What a [reaction collector](crate::ext::MessageExt::reaction_collector) yields.
///
/// Removals are only yielded with [`dispose`](CollectorBuilder::dispose), and only for
/// reactions the collector collected.
///
/// discord.js counterpart: the `collect`, `remove` and `dispose` events of
/// `ReactionCollector`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ReactionEvent {
    /// A user added a reaction (discord.js: `collect`).
    Added(Arc<MessageReactionAdd>),
    /// A user removed a collected reaction (discord.js: `remove`).
    Removed(Arc<MessageReactionRemove>),
    /// All the reactions of the message were removed (discord.js: the collection is
    /// emptied); the collector's counts start again from zero.
    RemovedAll(Arc<MessageReactionRemoveAll>),
}

impl ReactionEvent {
    /// The reacting user, unless every reaction was removed.
    #[must_use]
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Self::Added(reaction) => Some(reaction.user_id),
            Self::Removed(reaction) => Some(reaction.user_id),
            Self::RemovedAll(_) => None,
        }
    }

    /// The emoji, unless every reaction was removed.
    #[must_use]
    pub fn emoji(&self) -> Option<&PartialEmoji> {
        match self {
            Self::Added(reaction) => Some(&reaction.emoji),
            Self::Removed(reaction) => Some(&reaction.emoji),
            Self::RemovedAll(_) => None,
        }
    }
}

/// One emoji's reactions gathered by [`await_reactions`](crate::ext::MessageExt::await_reactions).
///
/// discord.js counterpart: the `MessageReaction` values of `awaitReactions`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CollectedReaction {
    /// The emoji.
    pub emoji: PartialEmoji,
    /// The users who reacted with it, in order.
    pub users: Vec<UserId>,
}

impl CollectedReaction {
    /// The number of users who reacted with the emoji.
    #[must_use]
    pub fn count(&self) -> usize {
        self.users.len()
    }
}

/// The key of an emoji in [`await_reactions`](crate::ext::MessageExt::await_reactions)' result:
/// a custom emoji's ID, or a Unicode emoji itself (`"👍"`), like discord.js' reaction keys.
#[must_use]
pub fn reaction_key(emoji: &PartialEmoji) -> String {
    match emoji {
        PartialEmoji::Custom { id, .. } => id.to_string(),
        PartialEmoji::Unicode { name } => name.to_string(),
    }
}

/// What the collecting shortcuts return (D-268): the [`Collection`] of what was collected
/// (through `Deref`) and why the collector ended. An empty result is not an error.
///
/// discord.js counterpart: the `Collection` of `awaitMessages` / `awaitReactions`.
#[derive(Clone, Debug)]
pub struct Collected<K, V> {
    collection: Collection<K, V>,
    reason: EndReason,
}

impl<K, V> Collected<K, V> {
    /// Why the collector ended.
    #[must_use]
    pub const fn reason(&self) -> EndReason {
        self.reason
    }

    /// Returns the collection.
    #[must_use]
    pub fn into_collection(self) -> Collection<K, V> {
        self.collection
    }
}

impl<K, V> Deref for Collected<K, V> {
    type Target = Collection<K, V>;

    fn deref(&self) -> &Collection<K, V> {
        &self.collection
    }
}

impl<K, V> DerefMut for Collected<K, V> {
    fn deref_mut(&mut self) -> &mut Collection<K, V> {
        &mut self.collection
    }
}

/// The kinds of builders: what a [`CollectorBuilder`] makes.
pub mod mode {
    /// [`build`](super::CollectorBuilder::build) makes a [`Collector`](super::Collector).
    #[derive(Debug)]
    pub struct Stream;

    /// Awaiting the builder waits for the first collected item.
    #[derive(Debug)]
    pub struct First;

    /// Awaiting the builder collects until the collector ends.
    #[derive(Debug)]
    pub struct All;
}

/// What collectors can yield: [`ComponentInteraction`], [`ModalInteraction`],
/// `Arc<MessageCreate>` and [`ReactionEvent`]. This trait is sealed.
pub trait Collectable: sealed::Item {}

impl Collectable for ComponentInteraction {}
impl Collectable for ModalInteraction {}
impl Collectable for Arc<MessageCreate> {}
impl Collectable for ReactionEvent {}

mod sealed {
    use super::{
        Arc, ComponentInteraction, ComponentType, MessageCreate, ModalInteraction, PartialEmoji,
        ReactionEvent, UserId,
    };
    use crate::collector::{Sink, index::Tx};

    /// What an item does to its collector.
    #[derive(Debug)]
    pub enum Action<'a> {
        /// Collect it, if the filter accepts it.
        Collect {
            user: Option<UserId>,
            emoji: Option<&'a PartialEmoji>,
            component: Option<ComponentType>,
        },
        /// A collected reaction was removed.
        Remove {
            user: UserId,
            emoji: &'a PartialEmoji,
        },
        /// Every reaction was removed.
        Clear,
    }

    pub trait Item: Sized + Send + 'static {
        /// The index entry sending these items.
        fn sink(tx: Tx<Self>) -> Sink;

        fn action(&self) -> Action<'_>;
    }

    impl Item for ComponentInteraction {
        fn sink(tx: Tx<Self>) -> Sink {
            Sink::Components(tx)
        }

        fn action(&self) -> Action<'_> {
            Action::Collect {
                user: self.interaction().author().map(|user| user.id),
                emoji: None,
                component: Some(self.component_type),
            }
        }
    }

    impl Item for ModalInteraction {
        fn sink(tx: Tx<Self>) -> Sink {
            Sink::Modals(tx)
        }

        fn action(&self) -> Action<'_> {
            Action::Collect {
                user: self.interaction().author().map(|user| user.id),
                emoji: None,
                component: None,
            }
        }
    }

    impl Item for Arc<MessageCreate> {
        fn sink(tx: Tx<Self>) -> Sink {
            Sink::Messages(tx)
        }

        fn action(&self) -> Action<'_> {
            Action::Collect {
                user: Some(self.author.id),
                emoji: None,
                component: None,
            }
        }
    }

    impl Item for ReactionEvent {
        fn sink(tx: Tx<Self>) -> Sink {
            Sink::Reactions(tx)
        }

        fn action(&self) -> Action<'_> {
            match self {
                Self::Added(reaction) => Action::Collect {
                    user: Some(reaction.user_id),
                    emoji: Some(&reaction.emoji),
                    component: None,
                },
                Self::Removed(reaction) => Action::Remove {
                    user: reaction.user_id,
                    emoji: &reaction.emoji,
                },
                Self::RemovedAll(_) => Action::Clear,
            }
        }
    }
}

use sealed::Action;

type Filter<T> = Box<dyn FnMut(&T) -> bool + Send>;

/// A collector's settings.
struct Options<T> {
    time: Option<Duration>,
    idle: Option<Duration>,
    max: Option<usize>,
    max_processed: Option<usize>,
    max_users: Option<usize>,
    max_emojis: Option<usize>,
    dispose: bool,
    component_type: Option<ComponentType>,
    filter: Option<Filter<T>>,
}

impl<T> Default for Options<T> {
    fn default() -> Self {
        Self {
            time: None,
            idle: None,
            max: None,
            max_processed: None,
            max_users: None,
            max_emojis: None,
            dispose: false,
            component_type: None,
            filter: None,
        }
    }
}

impl<T> fmt::Debug for Options<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Options")
            .field("time", &self.time)
            .field("idle", &self.idle)
            .field("max", &self.max)
            .field("max_processed", &self.max_processed)
            .field("max_users", &self.max_users)
            .field("max_emojis", &self.max_emojis)
            .field("dispose", &self.dispose)
            .field("component_type", &self.component_type)
            .field("filter", &self.filter.is_some())
            .finish()
    }
}

/// Configures a collector; made by the extension methods (`component_collector`,
/// `await_messages`, …) and the [`Context`] methods taking IDs.
///
/// `T` is what the collector yields; `M` is what the builder makes
/// ([`mode::Stream`]: [`build`](Self::build) a [`Collector`]; [`mode::First`] and
/// [`mode::All`]: `.await` the builder). The collector is registered when it is built or
/// awaited, not before.
///
/// discord.js counterpart: the collector options (`CollectorOptions`,
/// `MessageCollectorOptions`, `ReactionCollectorOptions`, `InteractionCollectorOptions`).
#[must_use = "a collector builder collects nothing until it is built or awaited"]
pub struct CollectorBuilder<T: Collectable, M = mode::Stream> {
    collectors: Arc<Collectors>,
    sequential: bool,
    intents: Intents,
    key: Key,
    guild: Option<GuildId>,
    channel: Option<ChannelId>,
    options: Options<T>,
    mode: PhantomData<fn() -> M>,
}

impl<T: Collectable, M> fmt::Debug for CollectorBuilder<T, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CollectorBuilder")
            .field("key", &self.key)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl<T: Collectable, M> CollectorBuilder<T, M> {
    pub(crate) fn new<B: Backend>(
        ctx: &Context<B>,
        key: Key,
        guild: Option<GuildId>,
        channel: Option<ChannelId>,
    ) -> Self {
        Self {
            collectors: Arc::clone(ctx.collectors()),
            sequential: ctx.is_sequential(),
            intents: ctx.intents(),
            key,
            guild,
            channel,
            options: Options::default(),
            mode: PhantomData,
        }
    }

    /// Ends the collector after `time` (discord.js: `time`).
    pub const fn time(mut self, time: Duration) -> Self {
        self.options.time = Some(time);
        self
    }

    /// Ends the collector when nothing was collected for `idle` (discord.js: `idle`).
    pub const fn idle(mut self, idle: Duration) -> Self {
        self.options.idle = Some(idle);
        self
    }

    /// Ends the collector after collecting `max` items (discord.js: `max`). For reactions
    /// with [`dispose`](Self::dispose), a removal gives its place back, as in discord.js.
    pub const fn max(mut self, max: usize) -> Self {
        self.options.max = Some(max);
        self
    }

    /// Ends the collector after `max` items arrived, whether the
    /// [`filter`](Self::filter) accepted them or not (discord.js: `maxProcessed`).
    pub const fn max_processed(mut self, max: usize) -> Self {
        self.options.max_processed = Some(max);
        self
    }

    /// Collects only the items `filter` accepts (discord.js: `filter`). Rejected items still
    /// count for [`max_processed`](Self::max_processed). The filter runs where the collector
    /// is polled, never on the shard.
    pub fn filter(mut self, filter: impl FnMut(&T) -> bool + Send + 'static) -> Self {
        self.options.filter = Some(Box::new(filter));
        self
    }

    /// Registers the collector and starts its timers.
    fn start(self) -> Collector<T> {
        if self.sequential {
            tracing::warn!(
                "a collector was created in the Sequential handler mode: a handler waiting for \
                 it blocks its shard, so it receives nothing from that shard (D-267)"
            );
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let sink = T::sink(tx);
        // The event it collects (the first of its kinds) needs one of its intents (D-271).
        crate::client::warn_missing_intents(self.intents, sink.kinds().first().copied());
        let registration = self
            .collectors
            .register(self.key, self.guild, self.channel, sink);
        let now = Instant::now();
        let timer = |duration: Option<Duration>| {
            duration
                .and_then(|duration| now.checked_add(duration))
                .map(|deadline| Box::pin(tokio::time::sleep_until(deadline)))
        };
        Collector {
            time: timer(self.options.time),
            idle: timer(self.options.idle),
            rx,
            registration: Some(registration),
            tally: Tally::new(&self.options),
            options: self.options,
            end: None,
        }
    }
}

impl<M> CollectorBuilder<ComponentInteraction, M> {
    /// Collects only components of this type (discord.js: `componentType`); others are
    /// ignored, not even processed.
    pub const fn component_type(mut self, kind: ComponentType) -> Self {
        self.options.component_type = Some(kind);
        self
    }

    /// Ends the collector when `max` different users were collected (discord.js:
    /// `maxUsers`).
    pub const fn max_users(mut self, max: usize) -> Self {
        self.options.max_users = Some(max);
        self
    }
}

impl<M> CollectorBuilder<ReactionEvent, M> {
    /// Also yields the removal of collected reactions ([`ReactionEvent::Removed`],
    /// [`ReactionEvent::RemovedAll`]; discord.js: `dispose`).
    pub const fn dispose(mut self, dispose: bool) -> Self {
        self.options.dispose = dispose;
        self
    }

    /// Ends the collector when `max` different users were collected (discord.js:
    /// `maxUsers`).
    pub const fn max_users(mut self, max: usize) -> Self {
        self.options.max_users = Some(max);
        self
    }

    /// Ends the collector when `max` different emojis were collected (discord.js:
    /// `maxEmojis`).
    pub const fn max_emojis(mut self, max: usize) -> Self {
        self.options.max_emojis = Some(max);
        self
    }
}

impl<T: Collectable> CollectorBuilder<T, mode::Stream> {
    /// Registers the collector: it collects from now until it ends.
    ///
    /// discord.js counterpart: `createMessageCollector`, `createReactionCollector`,
    /// `createMessageComponentCollector`.
    pub fn build(self) -> Collector<T> {
        self.start()
    }
}

impl<T: Collectable> IntoFuture for CollectorBuilder<T, mode::First> {
    type Output = Result<T, CollectorError>;
    type IntoFuture = AwaitFirst<T>;

    fn into_future(mut self) -> AwaitFirst<T> {
        self.options.max = Some(1);
        AwaitFirst {
            collector: self.start(),
        }
    }
}

macro_rules! await_all {
    ($item:ty => Collected<$key:ty, $value:ty>, $gather:expr) => {
        impl IntoFuture for CollectorBuilder<$item, mode::All> {
            type Output = Collected<$key, $value>;
            type IntoFuture = AwaitAll<$item, $key, $value>;

            fn into_future(self) -> Self::IntoFuture {
                AwaitAll {
                    collector: self.start(),
                    collection: Some(Collection::new()),
                    gather: $gather,
                }
            }
        }
    };
}

await_all!(
    Arc<MessageCreate> => Collected<MessageId, Arc<MessageCreate>>,
    |collection, message| {
        collection.insert(message.id, message);
    }
);

await_all!(
    ReactionEvent => Collected<String, CollectedReaction>,
    gather_reaction
);

/// Adds a reaction event to the reactions gathered by `await_reactions`.
fn gather_reaction(collection: &mut Collection<String, CollectedReaction>, event: ReactionEvent) {
    match event {
        ReactionEvent::Added(reaction) => {
            collection
                .as_index_map_mut()
                .entry(reaction_key(&reaction.emoji))
                .or_insert_with(|| CollectedReaction {
                    emoji: reaction.emoji.clone(),
                    users: Vec::new(),
                })
                .users
                .push(reaction.user_id);
        }
        ReactionEvent::Removed(reaction) => {
            let key = reaction_key(&reaction.emoji);
            if let Some(gathered) = collection.as_index_map_mut().get_mut(&key) {
                gathered.users.retain(|&user| user != reaction.user_id);
                if gathered.users.is_empty() {
                    collection.as_index_map_mut().shift_remove(&key);
                }
            }
        }
        ReactionEvent::RemovedAll(_) => collection.clear(),
    }
}

/// The counts behind a collector's limits.
#[derive(Debug)]
struct Tally {
    collected: usize,
    processed: usize,
    /// Collected items per user (with `max_users`, or reactions with `dispose`).
    users: Option<HashMap<UserId, usize>>,
    /// Collected reactions per emoji (with `max_emojis`, or `dispose`).
    emojis: Option<HashMap<String, usize>>,
    /// The collected reactions (with `dispose`): a removal of another one is not yielded.
    pairs: Option<HashSet<(UserId, String)>>,
}

impl Tally {
    fn new<T>(options: &Options<T>) -> Self {
        Self {
            collected: 0,
            processed: 0,
            users: (options.max_users.is_some() || options.dispose).then(HashMap::new),
            emojis: (options.max_emojis.is_some() || options.dispose).then(HashMap::new),
            pairs: options.dispose.then(HashSet::new),
        }
    }

    fn collect(&mut self, user: Option<UserId>, emoji: Option<&PartialEmoji>) {
        self.collected += 1;
        if let (Some(users), Some(user)) = (&mut self.users, user) {
            *users.entry(user).or_default() += 1;
        }
        if let (Some(emojis), Some(emoji)) = (&mut self.emojis, emoji) {
            *emojis.entry(reaction_key(emoji)).or_default() += 1;
        }
        if let (Some(pairs), Some(user), Some(emoji)) = (&mut self.pairs, user, emoji) {
            pairs.insert((user, reaction_key(emoji)));
        }
    }

    /// Takes a removed reaction out of the counts; `false` if it was not collected.
    fn remove(&mut self, user: UserId, emoji: &PartialEmoji) -> bool {
        let key = reaction_key(emoji);
        let Some(pairs) = &mut self.pairs else {
            return false;
        };
        let Some((_, key)) = pairs.take(&(user, key)) else {
            return false;
        };
        self.collected = self.collected.saturating_sub(1);
        if let Some(users) = &mut self.users {
            decrement(users, &user);
        }
        if let Some(emojis) = &mut self.emojis {
            decrement(emojis, &key);
        }
        true
    }

    fn clear(&mut self) {
        self.collected = 0;
        if let Some(users) = &mut self.users {
            users.clear();
        }
        if let Some(emojis) = &mut self.emojis {
            emojis.clear();
        }
        if let Some(pairs) = &mut self.pairs {
            pairs.clear();
        }
    }

    /// The limit this tally reached, if any (in discord.js' order).
    fn limit<T>(&self, options: &Options<T>) -> Option<EndReason> {
        let reached = |max: Option<usize>, count: usize| max.is_some_and(|max| count >= max);
        if reached(options.max, self.collected) {
            Some(EndReason::Limit)
        } else if reached(options.max_processed, self.processed) {
            Some(EndReason::ProcessedLimit)
        } else if reached(
            options.max_emojis,
            self.emojis.as_ref().map_or(0, HashMap::len),
        ) {
            Some(EndReason::EmojiLimit)
        } else if reached(
            options.max_users,
            self.users.as_ref().map_or(0, HashMap::len),
        ) {
            Some(EndReason::UserLimit)
        } else {
            None
        }
    }
}

fn decrement<K: core::hash::Hash + Eq>(counts: &mut HashMap<K, usize>, key: &K) {
    if let Some(count) = counts.get_mut(key) {
        *count -= 1;
        if *count == 0 {
            counts.remove(key);
        }
    }
}

/// A running collector: a [`Stream`] of what it collects, ending (`None`) when the collector
/// ends; [`end_reason`](Self::end_reason) then tells why.
///
/// Dropping the collector unregisters it. What arrives while it is not polled waits in its
/// queue; a collector kept but never polled keeps collecting until it ends.
///
/// discord.js counterpart: `MessageCollector`, `ReactionCollector`, `InteractionCollector`
/// (the `collect` event and the `end` event's reason).
#[must_use = "a collector collects nothing unless it is polled"]
pub struct Collector<T: Collectable> {
    rx: mpsc::UnboundedReceiver<Signal<T>>,
    registration: Option<Registration>,
    options: Options<T>,
    tally: Tally,
    time: Option<Pin<Box<Sleep>>>,
    idle: Option<Pin<Box<Sleep>>>,
    end: Option<EndReason>,
}

impl<T: Collectable> fmt::Debug for Collector<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Collector")
            .field("options", &self.options)
            .field("collected", &self.tally.collected)
            .field("processed", &self.tally.processed)
            .field("end", &self.end)
            .finish_non_exhaustive()
    }
}

impl<T: Collectable> Collector<T> {
    /// Waits for the next collected item; `None` once the collector ended.
    pub async fn next(&mut self) -> Option<T> {
        core::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// Ends the collector now (discord.js: `Collector#stop`); its reason is
    /// [`EndReason::Stopped`]. Items that arrived but were not read are dropped.
    pub fn stop(&mut self) {
        self.finish(EndReason::Stopped);
    }

    /// Restarts the timers: `time` and `idle` count again from now, with new durations where
    /// given (discord.js: `Collector#resetTimer`). Does nothing once the collector ended.
    pub fn reset_timer(&mut self, time: Option<Duration>, idle: Option<Duration>) {
        if self.end.is_some() {
            return;
        }
        if time.is_some() {
            self.options.time = time;
        }
        if idle.is_some() {
            self.options.idle = idle;
        }
        let now = Instant::now();
        let restart = |timer: &mut Option<Pin<Box<Sleep>>>, duration: Option<Duration>| {
            *timer = duration
                .and_then(|duration| now.checked_add(duration))
                .map(|deadline| Box::pin(tokio::time::sleep_until(deadline)));
        };
        restart(&mut self.time, self.options.time);
        restart(&mut self.idle, self.options.idle);
    }

    /// Why the collector ended, or `None` while it runs (discord.js: `endReason`).
    #[must_use]
    pub const fn end_reason(&self) -> Option<EndReason> {
        self.end
    }

    /// Whether the collector ended (discord.js: `ended`).
    #[must_use]
    pub const fn is_ended(&self) -> bool {
        self.end.is_some()
    }

    /// The number of items collected so far (for reactions with `dispose`, without the
    /// removed ones).
    #[must_use]
    pub const fn collected(&self) -> usize {
        self.tally.collected
    }

    /// The number of items that arrived so far, whether the filter accepted them or not.
    #[must_use]
    pub const fn processed(&self) -> usize {
        self.tally.processed
    }

    /// Ends the collector: unregisters it and stops its timers.
    fn finish(&mut self, reason: EndReason) {
        if self.end.is_none() {
            self.end = Some(reason);
            self.registration = None;
            self.time = None;
            self.idle = None;
            self.rx.close();
        }
    }

    /// Runs one arrived item through the type, the filter and the limits; returns it if it
    /// is collected (or a yielded removal).
    fn process(&mut self, item: T) -> Option<T> {
        let accepted = match item.action() {
            Action::Collect {
                user,
                emoji,
                component,
            } => {
                if self
                    .options
                    .component_type
                    .is_some_and(|wanted| component != Some(wanted))
                {
                    return None;
                }
                self.tally.processed += 1;
                let accepted = self
                    .options
                    .filter
                    .as_mut()
                    .is_none_or(|filter| filter(&item));
                if accepted {
                    self.tally.collect(user, emoji);
                    if let (Some(idle), Some(timer)) = (self.options.idle, &mut self.idle)
                        && let Some(deadline) = Instant::now().checked_add(idle)
                    {
                        timer.as_mut().reset(deadline);
                    }
                }
                accepted
            }
            Action::Remove { user, emoji } => {
                self.options.dispose && self.tally.remove(user, emoji)
            }
            Action::Clear => {
                self.tally.clear();
                self.options.dispose
            }
        };
        if let Some(reason) = self.tally.limit(&self.options) {
            // The item that reached the limit is still yielded; the next poll ends.
            self.finish(reason);
        }
        accepted.then_some(item)
    }
}

impl<T: Collectable> Stream for Collector<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<T>> {
        let this = self.get_mut();
        loop {
            if this.end.is_some() {
                return Poll::Ready(None);
            }
            if this
                .time
                .as_mut()
                .is_some_and(|timer| timer.as_mut().poll(cx).is_ready())
            {
                this.finish(EndReason::Time);
                continue;
            }
            if this
                .idle
                .as_mut()
                .is_some_and(|timer| timer.as_mut().poll(cx).is_ready())
            {
                this.finish(EndReason::Idle);
                continue;
            }
            match this.rx.poll_recv(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Signal::Item(item))) => {
                    if let Some(item) = this.process(item) {
                        return Poll::Ready(Some(item));
                    }
                }
                Poll::Ready(Some(Signal::End(reason))) => this.finish(reason),
                Poll::Ready(None) => this.finish(EndReason::Shutdown),
            }
        }
    }
}

/// The future of a shortcut waiting for one item (`await_component`, `await_modal_submit`).
#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub struct AwaitFirst<T: Collectable> {
    collector: Collector<T>,
}

impl<T: Collectable> Future for AwaitFirst<T> {
    type Output = Result<T, CollectorError>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let collector = &mut self.get_mut().collector;
        match Pin::new(&mut *collector).poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Ok(item)),
            Poll::Ready(None) => Poll::Ready(Err(CollectorError {
                reason: collector.end.unwrap_or(EndReason::Shutdown),
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The future of a shortcut collecting until the end (`await_messages`, `await_reactions`).
#[must_use = "futures do nothing unless polled"]
pub struct AwaitAll<T: Collectable, K, V> {
    collector: Collector<T>,
    collection: Option<Collection<K, V>>,
    gather: fn(&mut Collection<K, V>, T),
}

impl<T: Collectable, K, V> fmt::Debug for AwaitAll<T, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwaitAll")
            .field("collector", &self.collector)
            .finish_non_exhaustive()
    }
}

// Nothing in it is pinned structurally: the collector is moved around freely.
impl<T: Collectable, K, V> Unpin for AwaitAll<T, K, V> {}

impl<T: Collectable, K, V> Future for AwaitAll<T, K, V> {
    type Output = Collected<K, V>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.collector).poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    if let Some(collection) = &mut this.collection {
                        (this.gather)(collection, item);
                    }
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Collected {
                        collection: this.collection.take().unwrap_or_default(),
                        reason: this.collector.end.unwrap_or(EndReason::Shutdown),
                    });
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Collectors by ID, for code that holds IDs rather than the channel or the message.
impl<B: Backend> Context<B> {
    /// The guild of `channel`, if the cache knows it (so that the guild's deletion ends the
    /// collector).
    fn channel_guild(&self, channel: ChannelId) -> Option<GuildId> {
        self.cache()?.channels().get(channel)?.guild_id()
    }

    /// A collector of the messages sent in `channel` (D-263, D-269).
    ///
    /// It ends at its limits, or when the channel, thread or guild is deleted. Needs the
    /// `GUILD_MESSAGES` or `DIRECT_MESSAGES` intent (and `MESSAGE_CONTENT` to read guild
    /// messages' content).
    ///
    /// discord.js counterpart: `TextChannel#createMessageCollector`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use djs_rs::prelude::*;
    /// # async fn example(ctx: Context, msg: djs_rs::model::message::Message) {
    /// let mut messages = ctx
    ///     .message_collector(msg.channel_id)
    ///     .time(Duration::from_secs(60))
    ///     .build();
    /// while let Some(message) = messages.next().await {
    ///     println!("{}: {}", message.author.username, message.content);
    /// }
    /// # }
    /// ```
    pub fn message_collector(&self, channel: ChannelId) -> CollectorBuilder<Arc<MessageCreate>> {
        let guild = self.channel_guild(channel);
        CollectorBuilder::new(self, Key::Channel(channel), guild, Some(channel))
    }

    /// Collects the messages sent in `channel` until the collector ends, like
    /// [`message_collector`](Self::message_collector); set a limit (`time`, `max`, …), or it
    /// only ends with the channel.
    ///
    /// discord.js counterpart: `TextChannel#awaitMessages`.
    pub fn await_messages(
        &self,
        channel: ChannelId,
    ) -> CollectorBuilder<Arc<MessageCreate>, mode::All> {
        let guild = self.channel_guild(channel);
        CollectorBuilder::new(self, Key::Channel(channel), guild, Some(channel))
    }

    /// A collector of the component interactions (buttons, select menus) on every message of
    /// `channel`.
    ///
    /// discord.js counterpart: `TextChannel#createMessageComponentCollector`.
    pub fn component_collector(
        &self,
        channel: ChannelId,
    ) -> CollectorBuilder<ComponentInteraction> {
        let guild = self.channel_guild(channel);
        CollectorBuilder::new(self, Key::Channel(channel), guild, Some(channel))
    }

    /// Waits for one component interaction on any message of `channel`; set a
    /// [`time`](CollectorBuilder::time), or it only ends with the channel.
    ///
    /// # Errors
    ///
    /// [`CollectorError`] with the reason if the collector ended first.
    ///
    /// discord.js counterpart: `TextChannel#awaitMessageComponent`.
    pub fn await_component(
        &self,
        channel: ChannelId,
    ) -> CollectorBuilder<ComponentInteraction, mode::First> {
        let guild = self.channel_guild(channel);
        CollectorBuilder::new(self, Key::Channel(channel), guild, Some(channel))
    }
}
