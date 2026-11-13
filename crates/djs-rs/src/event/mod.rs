//! Typed client events (D-222).
//!
//! Every event is a zero-sized value on [`Events`] — `Events::MessageCreate`,
//! `Events::GuildMemberUpdate`, … — whose type determines the payload a handler receives:
//!
//! ```no_run
//! # use djs_rs::prelude::*;
//! # let client = Client::builder("token").build();
//! client.on(Events::MessageCreate, |_ctx, message| async move {
//!     // `message` is an `Arc<MessageCreate>`, which dereferences to `Message`.
//!     println!("{}", message.content);
//!     Ok(())
//! });
//!
//! client.on(Events::MessageUpdate, |_ctx, update| async move {
//!     // `update.old` is the cached message before the edit, if the cache held it (D-221);
//!     // the payload dereferences to the new message.
//!     let before = update.old.as_ref().map(|old| old.content.as_str());
//!     println!("{before:?} -> {}", update.content);
//!     Ok(())
//! });
//! ```
//!
//! # What a handler receives
//!
//! - Most events hand their [`djs-model`](djs_model) payload unchanged, behind an `Arc`
//!   (D-225).
//! - Events whose discord.js counterpart hands an old or removed object (D-227) hand a type
//!   from [`payload`] with the state the cache held before the event in `old` and Discord's
//!   data in `new`, and dereference to `new` (D-228). `old` is `None` without a cache, with
//!   the resource disabled in [`CacheConfig`](djs_cache::CacheConfig), or for something the
//!   cache never received.
//! - Some events are derived by the client (D-223): [`GuildAvailable`] /
//!   [`GuildUnavailable`] / [`GuildCreate`] / [`GuildDelete`] are told apart with or without
//!   a cache; the per-emoji and per-sticker events need that resource in the cache.
//! - [`Raw`] hands every gateway dispatch as its [`DispatchEvent`] before anything else.
//!
//! # Order
//!
//! For each gateway dispatch the client first applies it to the cache (D-171), then emits
//! [`Raw`], then the event itself, then the events derived from it (for example
//! [`GuildEmojisUpdate`], then one [`GuildEmojiCreate`] per new emoji).
//!
//! # Intents
//!
//! Each event lists the [`Intents`] Discord sends it with, also
//! as [`Event::INTENTS`]; events without an intent are always sent. At login the client warns
//! about handlers whose events none of its intents covers (D-271), and
//! [`Client::required_intents`](crate::Client::required_intents) tells which intents the
//! registered handlers need (D-272). Message content in guilds also needs the privileged
//! `MESSAGE_CONTENT` intent, which no event lists: without it the content, embeds,
//! attachments and components of other users' messages arrive empty.
//!
//! # Partials (discord.js)
//!
//! discord.js drops events about objects it has not cached unless the matching `partials`
//! are enabled. djs-rs has no such option (D-273): every event arrives, and what the cache
//! did not hold is simply missing — `old` is `None`, and payloads carry IDs where Discord
//! sends IDs. Complete them with the managers when needed:
//!
//! | discord.js partial | What djs-rs hands instead | To complete it |
//! |---|---|---|
//! | `Partials.Message` | [`MessageUpdate`] / [`MessageDelete`] with `old: None` (`MessageUpdate` still has the new message) | `ctx.messages(channel).fetch(id)` (not after a deletion) |
//! | `Partials.Channel` | DM events arrive without a cached channel; the payloads carry `channel_id` | `ctx.channels().fetch(id)` |
//! | `Partials.Reaction` | [`MessageReactionAdd`] / [`MessageReactionRemove`] carry the message, channel and user IDs | `ctx.messages(channel).fetch(message)` |
//! | `Partials.User` | user IDs (`user_id`) in reaction, typing and poll events | `ctx.users().fetch(id)` |
//! | `Partials.GuildMember` | [`GuildMemberUpdate`] / [`GuildMemberRemove`] with `old: None` (the new data or the user) | `ctx.members(guild).fetch(user)` |
//! | `Partials.ThreadMember` | [`ThreadMemberUpdate`] / [`ThreadMembersUpdate`] payloads as Discord sends them | `ctx.thread_members(thread).fetch(user)` |
//! | `Partials.GuildScheduledEvent` | [`GuildScheduledEventUserAdd`] / [`GuildScheduledEventUserRemove`] with IDs; update / delete with `old: None` | `ctx.scheduled_events(guild).fetch(id)` |
//! | `Partials.SoundboardSound` | soundboard update / delete with `old: None` | `ctx.soundboard_sounds(guild).fetch(id)` |
//! | `Partials.Poll`, `Partials.PollAnswer` | [`MessagePollVoteAdd`] / [`MessagePollVoteRemove`] with the message ID and answer ID | `ctx.messages(channel).fetch(message)` (its `poll`) |
//!
//! discord.js counterpart: the `Events` enum.

use std::sync::Arc;

use djs_gateway::ReceiveError;
use djs_model::gateway::{CloseCode, DispatchEvent, Intents, ShardId, payload as gateway};

pub mod payload;

/// Defines [`Events`], one marker type and [`Event`] impl per event, [`EventKind`] and
/// [`Dispatch`] from one list, so they cannot drift apart.
macro_rules! events {
    ($(
        $(#[$doc:meta])*
        $name:ident = $js:literal => $payload:ty [$($intent:ident)|*];
    )*) => {
        /// The events a [`Client`](crate::Client) can emit, as typed values (D-222).
        ///
        /// Pass one of these to [`Client::on`](crate::Client::on). The names are those of
        /// discord.js' `Events` (`Events.MessageCreate`, `Events.GuildRoleUpdate`, …); gateway
        /// dispatches discord.js has no event for are named after the dispatch
        /// (`INTEGRATION_CREATE` is [`IntegrationCreate`](Self::IntegrationCreate)).
        ///
        /// See the [module documentation](self) for what handlers receive and in which order.
        #[derive(Clone, Copy, Debug)]
        #[non_exhaustive]
        pub struct Events;

        #[expect(
            non_upper_case_globals,
            reason = "event names mirror discord.js' `Events.MessageCreate` style"
        )]
        impl Events {
            $(
                $(#[$doc])*
                pub const $name: $name = $name;
            )*
        }

        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            #[non_exhaustive]
            pub struct $name;

            impl sealed::Sealed for $name {}

            impl Event for $name {
                type Payload = $payload;
                const NAME: &'static str = $js;
                const KIND: EventKind = EventKind::$name;
                const INTENTS: Intents = Intents::empty()$(.union(Intents::$intent))*;

                fn payload(dispatch: &Dispatch) -> Option<&Arc<$payload>> {
                    match dispatch {
                        Dispatch::$name(payload) => Some(payload),
                        _ => None,
                    }
                }
            }
        )*

        /// Identifies an event; used to index the handler table.
        #[doc(hidden)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum EventKind {
            $($name,)*
        }

        impl EventKind {
            /// The number of events.
            pub(crate) const COUNT: usize = [$(EventKind::$name),*].len();

            /// Every event, in [`EventKind`] order.
            pub(crate) const ALL: [Self; Self::COUNT] = [$(EventKind::$name),*];

            /// The intents Discord sends the event under ([`Event::INTENTS`]).
            pub(crate) const fn intents(self) -> Intents {
                match self {
                    $(Self::$name => <$name as Event>::INTENTS,)*
                }
            }

            /// The event's name as discord.js spells it ([`Event::NAME`]).
            pub(crate) const fn name(self) -> &'static str {
                match self {
                    $(Self::$name => $js,)*
                }
            }
        }

        /// An event as delivered by [`Client::events`](crate::Client::events): one variant per
        /// event of [`Events`], with the payload its handlers receive.
        ///
        /// [`Raw`] is only delivered to handlers, never to event streams (they already receive
        /// every event).
        #[derive(Clone, Debug)]
        #[non_exhaustive]
        pub enum Dispatch {
            $(
                #[doc = concat!("The [`", stringify!($name), "`] event.")]
                $name(Arc<$payload>),
            )*
            /// A gateway dispatch this version of the library does not know, with its name.
            Unknown(Arc<str>),
        }

        impl Dispatch {
            /// Returns the event's name as discord.js spells it (`"messageCreate"`), or the
            /// Discord name of an [`Unknown`](Self::Unknown) dispatch.
            #[must_use]
            pub fn name(&self) -> &str {
                match self {
                    $(Self::$name(_) => $js,)*
                    Self::Unknown(name) => name,
                }
            }

            /// Returns which event this is (`None` for unknown dispatches).
            pub(crate) const fn kind(&self) -> Option<EventKind> {
                match self {
                    $(Self::$name(_) => Some(EventKind::$name),)*
                    Self::Unknown(_) => None,
                }
            }
        }
    };
}

/// A typed event that can be passed to [`Client::on`](crate::Client::on).
///
/// Implemented by the zero-sized event types on [`Events`]; sealed, so the set of events is
/// controlled by this crate.
pub trait Event: Copy + Send + Sync + 'static + sealed::Sealed {
    /// The data a handler of this event receives (behind an `Arc`).
    type Payload: Send + Sync + 'static;

    /// The event's name as discord.js spells it, such as `"messageCreate"`.
    const NAME: &'static str;

    #[doc(hidden)]
    const KIND: EventKind;

    /// The [gateway intents](https://docs.discord.com/developers/events/gateway#list-of-intents)
    /// Discord sends this event under: the client receives it if it requests any of them (for
    /// [`MessageCreate`] guild messages come with `GUILD_MESSAGES`, direct messages with
    /// `DIRECT_MESSAGES`). Empty for events sent without an intent, such as
    /// [`InteractionCreate`] or the client's own lifecycle events. The client warns at login
    /// about handlers whose events none of its intents cover (D-271).
    const INTENTS: Intents;

    #[doc(hidden)]
    fn payload(dispatch: &Dispatch) -> Option<&Arc<Self::Payload>>;
}

mod sealed {
    pub trait Sealed {}
}

events! {
    // --- The client and its shards -------------------------------------------------------

    /// The client is ready: fired once per [`login`](crate::Client::login), at the `READY` of
    /// the last of its shards to become ready (D-224).
    ///
    /// It does **not** wait for the guilds' `GUILD_CREATE`s, so the cache may still be filling
    /// (discord.js waits for them). The payload is that shard's `READY`: its `user` and
    /// `application` are the same on every shard, its `guilds` are that shard's.
    ///
    /// discord.js: `Events.ClientReady` (`clientReady`).
    ClientReady = "clientReady" => gateway::Ready [];

    /// A shard's session is ready (`READY`): after connecting, and again after every
    /// re-identify (not after a resume, see [`ShardResume`]).
    ///
    /// The shard is [`Context::shard_id`](crate::Context::shard_id); the guilds in the payload
    /// arrive as [`GuildAvailable`] afterwards.
    ///
    /// discord.js: `Events.ShardReady` (`shardReady`).
    ShardReady = "shardReady" => gateway::Ready [];

    /// A shard resumed its session (`RESUMED`) after a reconnect; Discord replayed the events
    /// it missed, counted in [`ShardResume::replayed`](payload::ShardResume::replayed).
    ///
    /// discord.js: `Events.ShardResume` (`shardResume`).
    ShardResume = "shardResume" => payload::ShardResume [];

    /// A shard lost its connection and is reconnecting (it resumes when it can, otherwise
    /// it identifies again).
    ///
    /// Fired at the first event after the connection was lost, at the latest at the new
    /// connection's hello.
    ///
    /// discord.js: `Events.ShardReconnecting` (`shardReconnecting`).
    ShardReconnecting = "shardReconnecting" => payload::ShardReconnecting [];

    /// Discord invalidated a shard's session (`INVALID_SESSION` that cannot be resumed): the
    /// shard identifies again, and the events in between are lost.
    ///
    /// discord.js lists `Events.Invalidated` (`invalidated`) but v14 no longer emits it; here
    /// it marks a lost session.
    Invalidated = "invalidated" => payload::Invalidated [];

    /// A shard hit an error it recovers from (a failed connection, a payload that could not
    /// be decoded, …); it reconnects or skips the payload by itself.
    ///
    /// discord.js: `Events.ShardError` (`shardError`).
    ShardError = "shardError" => ReceiveError [];

    /// A shard was closed by Discord with a code that forbids reconnecting (an invalid token,
    /// disallowed intents, …); the [`Client`](crate::Client) stops and
    /// [`login`](crate::Client::login) returns the error.
    ///
    /// discord.js: `Events.ShardDisconnect` (`shardDisconnect`).
    ShardDisconnect = "shardDisconnect" => CloseCode [];

    /// Every gateway dispatch, decoded, before the cache's events and the derived ones.
    ///
    /// Only handlers receive it; registering one costs a clone of every dispatch.
    ///
    /// discord.js: `Events.Raw` (`raw`), which hands the undecoded packet.
    Raw = "raw" => DispatchEvent [];

    // --- Applications, auto moderation, monetization ------------------------------------

    /// An application command's permissions were updated (`APPLICATION_COMMAND_PERMISSIONS_UPDATE`).
    ///
    /// discord.js: `Events.ApplicationCommandPermissionsUpdate`.
    ApplicationCommandPermissionsUpdate = "applicationCommandPermissionsUpdate"
        => gateway::ApplicationCommandPermissionsUpdate [];

    /// An auto moderation rule acted on a message or member
    /// (`AUTO_MODERATION_ACTION_EXECUTION`). Intent: `AUTO_MODERATION_EXECUTION`.
    ///
    /// discord.js: `Events.AutoModerationActionExecution`.
    AutoModerationActionExecution = "autoModerationActionExecution"
        => gateway::AutoModerationActionExecution [AUTO_MODERATION_EXECUTION];

    /// An auto moderation rule was created (`AUTO_MODERATION_RULE_CREATE`). Intent:
    /// `AUTO_MODERATION_CONFIGURATION`.
    ///
    /// discord.js: `Events.AutoModerationRuleCreate`.
    AutoModerationRuleCreate = "autoModerationRuleCreate" => gateway::AutoModerationRuleCreate [AUTO_MODERATION_CONFIGURATION];

    /// An auto moderation rule was updated (`AUTO_MODERATION_RULE_UPDATE`). Intent:
    /// `AUTO_MODERATION_CONFIGURATION`.
    ///
    /// The cache does not keep rules, so there is no old rule (C-091).
    ///
    /// discord.js: `Events.AutoModerationRuleUpdate`.
    AutoModerationRuleUpdate = "autoModerationRuleUpdate" => gateway::AutoModerationRuleUpdate [AUTO_MODERATION_CONFIGURATION];

    /// An auto moderation rule was deleted (`AUTO_MODERATION_RULE_DELETE`). Intent:
    /// `AUTO_MODERATION_CONFIGURATION`.
    ///
    /// discord.js: `Events.AutoModerationRuleDelete`.
    AutoModerationRuleDelete = "autoModerationRuleDelete" => gateway::AutoModerationRuleDelete [AUTO_MODERATION_CONFIGURATION];

    /// A user subscribed to a SKU (`ENTITLEMENT_CREATE`).
    ///
    /// discord.js: `Events.EntitlementCreate`.
    EntitlementCreate = "entitlementCreate" => gateway::EntitlementCreate [];

    /// An entitlement was updated, for example renewed (`ENTITLEMENT_UPDATE`).
    ///
    /// discord.js: `Events.EntitlementUpdate`.
    EntitlementUpdate = "entitlementUpdate" => gateway::EntitlementUpdate [];

    /// An entitlement was deleted (`ENTITLEMENT_DELETE`).
    ///
    /// discord.js: `Events.EntitlementDelete`.
    EntitlementDelete = "entitlementDelete" => gateway::EntitlementDelete [];

    /// A premium subscription was created (`SUBSCRIPTION_CREATE`).
    ///
    /// discord.js: `Events.SubscriptionCreate`.
    SubscriptionCreate = "subscriptionCreate" => gateway::SubscriptionCreate [];

    /// A premium subscription was updated (`SUBSCRIPTION_UPDATE`).
    ///
    /// discord.js: `Events.SubscriptionUpdate`.
    SubscriptionUpdate = "subscriptionUpdate" => gateway::SubscriptionUpdate [];

    /// A premium subscription was deleted (`SUBSCRIPTION_DELETE`).
    ///
    /// discord.js: `Events.SubscriptionDelete`.
    SubscriptionDelete = "subscriptionDelete" => gateway::SubscriptionDelete [];

    // --- Channels and threads -----------------------------------------------------------

    /// A guild channel was created (`CHANNEL_CREATE`). Intent: `GUILDS`.
    ///
    /// discord.js: `Events.ChannelCreate`.
    ChannelCreate = "channelCreate" => gateway::ChannelCreate [GUILDS];

    /// A channel was updated (`CHANNEL_UPDATE`), with the cached channel before the update.
    /// Intent: `GUILDS`.
    ///
    /// discord.js: `Events.ChannelUpdate` (`oldChannel`, `newChannel`).
    ChannelUpdate = "channelUpdate" => payload::ChannelUpdate [GUILDS];

    /// A channel was deleted (`CHANNEL_DELETE`), with the cached channel. Intent: `GUILDS`.
    ///
    /// discord.js: `Events.ChannelDelete`.
    ChannelDelete = "channelDelete" => payload::ChannelDelete [GUILDS];

    /// A message was pinned or unpinned (`CHANNEL_PINS_UPDATE`). Intents: `GUILDS`,
    /// `DIRECT_MESSAGES`.
    ///
    /// discord.js: `Events.ChannelPinsUpdate`.
    ChannelPinsUpdate = "channelPinsUpdate" => gateway::ChannelPinsUpdate [GUILDS | DIRECT_MESSAGES];

    /// The ephemeral data of a guild's channels (`CHANNEL_INFO`), the answer to a
    /// [`RequestChannelInfo`](djs_model::gateway::payload::RequestChannelInfo) command.
    ///
    /// discord.js has no such event (C-091).
    ChannelInfo = "channelInfo" => gateway::ChannelInfo [];

    /// A thread was created, or the bot was added to one (`THREAD_CREATE`). Intent: `GUILDS`.
    ///
    /// discord.js: `Events.ThreadCreate` (`newly_created` is a field of the payload).
    ThreadCreate = "threadCreate" => gateway::ThreadCreate [GUILDS];

    /// A thread was updated (`THREAD_UPDATE`), with the cached thread before the update.
    /// Intent: `GUILDS`.
    ///
    /// discord.js: `Events.ThreadUpdate` (`oldThread`, `newThread`).
    ThreadUpdate = "threadUpdate" => payload::ThreadUpdate [GUILDS];

    /// A thread was deleted (`THREAD_DELETE`), with the cached thread. Intent: `GUILDS`.
    ///
    /// discord.js: `Events.ThreadDelete`.
    ThreadDelete = "threadDelete" => payload::ThreadDelete [GUILDS];

    /// The bot gained access to channels: their active threads (`THREAD_LIST_SYNC`). Intent:
    /// `GUILDS`.
    ///
    /// discord.js: `Events.ThreadListSync`.
    ThreadListSync = "threadListSync" => gateway::ThreadListSync [GUILDS];

    /// The bot's own thread member was updated (`THREAD_MEMBER_UPDATE`). Intent: `GUILDS`.
    ///
    /// The cache does not keep the bot's thread member, so there is no old one (C-091).
    ///
    /// discord.js: `Events.ThreadMemberUpdate`.
    ThreadMemberUpdate = "threadMemberUpdate" => gateway::ThreadMemberUpdate [GUILDS];

    /// Members were added to or removed from a thread (`THREAD_MEMBERS_UPDATE`). Intents:
    /// `GUILDS`, and `GUILD_MEMBERS` for other users than the bot.
    ///
    /// discord.js: `Events.ThreadMembersUpdate`.
    ThreadMembersUpdate = "threadMembersUpdate" => gateway::ThreadMembersUpdate [GUILDS];

    // --- Guilds --------------------------------------------------------------------------

    /// The bot joined a guild (`GUILD_CREATE` of a guild it was not waiting for). Intent:
    /// `GUILDS`.
    ///
    /// Guilds from `READY`, and guilds back from an outage, are [`GuildAvailable`] instead
    /// (D-223).
    ///
    /// discord.js: `Events.GuildCreate`.
    GuildCreate = "guildCreate" => gateway::GuildCreate [GUILDS];

    /// A guild became available (`GUILD_CREATE` of a guild from `READY` or back from an
    /// outage). Intent: `GUILDS`.
    ///
    /// discord.js: `Events.GuildAvailable`.
    GuildAvailable = "guildAvailable" => gateway::GuildCreate [GUILDS];

    /// A guild was updated (`GUILD_UPDATE`), with the cached guild before the update. Intent:
    /// `GUILDS`.
    ///
    /// discord.js: `Events.GuildUpdate` (`oldGuild`, `newGuild`).
    GuildUpdate = "guildUpdate" => payload::GuildUpdate [GUILDS];

    /// The bot left a guild, was removed, or the guild was deleted (`GUILD_DELETE` without
    /// `unavailable`), with the cached guild. Intent: `GUILDS`.
    ///
    /// discord.js: `Events.GuildDelete`.
    GuildDelete = "guildDelete" => payload::GuildDelete [GUILDS];

    /// A guild became unavailable because of an outage (`GUILD_DELETE` with `unavailable`),
    /// with the cached guild; [`GuildAvailable`] follows when it is back. Intent: `GUILDS`.
    ///
    /// discord.js: `Events.GuildUnavailable`.
    GuildUnavailable = "guildUnavailable" => payload::GuildUnavailable [GUILDS];

    /// An audit log entry was created (`GUILD_AUDIT_LOG_ENTRY_CREATE`). Intent:
    /// `GUILD_MODERATION`, and the bot needs the `VIEW_AUDIT_LOG` permission.
    ///
    /// discord.js: `Events.GuildAuditLogEntryCreate`.
    GuildAuditLogEntryCreate = "guildAuditLogEntryCreate" => gateway::GuildAuditLogEntryCreate [GUILD_MODERATION];

    /// A user was banned (`GUILD_BAN_ADD`). Intent: `GUILD_MODERATION`.
    ///
    /// discord.js: `Events.GuildBanAdd`.
    GuildBanAdd = "guildBanAdd" => gateway::GuildBanAdd [GUILD_MODERATION];

    /// A user was unbanned (`GUILD_BAN_REMOVE`). Intent: `GUILD_MODERATION`.
    ///
    /// discord.js: `Events.GuildBanRemove`.
    GuildBanRemove = "guildBanRemove" => gateway::GuildBanRemove [GUILD_MODERATION];

    /// A guild's emojis changed (`GUILD_EMOJIS_UPDATE`): the full new list. Intent:
    /// `GUILD_EXPRESSIONS`.
    ///
    /// With emojis in the cache, [`GuildEmojiCreate`], [`GuildEmojiUpdate`] and
    /// [`GuildEmojiDelete`] follow for each change (D-223).
    ///
    /// discord.js has no such event; it only emits the per-emoji ones (C-091).
    GuildEmojisUpdate = "guildEmojisUpdate" => gateway::GuildEmojisUpdate [GUILD_EXPRESSIONS];

    /// An emoji was created, derived from [`GuildEmojisUpdate`] with emojis in the cache.
    ///
    /// discord.js: `Events.GuildEmojiCreate` (`emojiCreate`).
    GuildEmojiCreate = "emojiCreate" => payload::GuildEmojiCreate [GUILD_EXPRESSIONS];

    /// An emoji was updated, derived from [`GuildEmojisUpdate`] with emojis in the cache.
    ///
    /// discord.js: `Events.GuildEmojiUpdate` (`emojiUpdate`).
    GuildEmojiUpdate = "emojiUpdate" => payload::GuildEmojiUpdate [GUILD_EXPRESSIONS];

    /// An emoji was deleted, derived from [`GuildEmojisUpdate`] with emojis in the cache.
    ///
    /// discord.js: `Events.GuildEmojiDelete` (`emojiDelete`).
    GuildEmojiDelete = "emojiDelete" => payload::GuildEmojiDelete [GUILD_EXPRESSIONS];

    /// A guild's stickers changed (`GUILD_STICKERS_UPDATE`): the full new list. Intent:
    /// `GUILD_EXPRESSIONS`.
    ///
    /// With stickers in the cache, [`GuildStickerCreate`], [`GuildStickerUpdate`] and
    /// [`GuildStickerDelete`] follow for each change (D-223).
    ///
    /// discord.js has no such event; it only emits the per-sticker ones (C-091).
    GuildStickersUpdate = "guildStickersUpdate" => gateway::GuildStickersUpdate [GUILD_EXPRESSIONS];

    /// A sticker was created, derived from [`GuildStickersUpdate`] with stickers in the cache.
    ///
    /// discord.js: `Events.GuildStickerCreate` (`stickerCreate`).
    GuildStickerCreate = "stickerCreate" => payload::GuildStickerCreate [GUILD_EXPRESSIONS];

    /// A sticker was updated, derived from [`GuildStickersUpdate`] with stickers in the cache.
    ///
    /// discord.js: `Events.GuildStickerUpdate` (`stickerUpdate`).
    GuildStickerUpdate = "stickerUpdate" => payload::GuildStickerUpdate [GUILD_EXPRESSIONS];

    /// A sticker was deleted, derived from [`GuildStickersUpdate`] with stickers in the cache.
    ///
    /// discord.js: `Events.GuildStickerDelete` (`stickerDelete`).
    GuildStickerDelete = "stickerDelete" => payload::GuildStickerDelete [GUILD_EXPRESSIONS];

    /// A guild's integrations changed (`GUILD_INTEGRATIONS_UPDATE`). Intent:
    /// `GUILD_INTEGRATIONS`.
    ///
    /// discord.js: `Events.GuildIntegrationsUpdate`.
    GuildIntegrationsUpdate = "guildIntegrationsUpdate" => gateway::GuildIntegrationsUpdate [GUILD_INTEGRATIONS];

    /// An integration was created (`INTEGRATION_CREATE`). Intent: `GUILD_INTEGRATIONS`.
    ///
    /// discord.js has no such event (C-091).
    IntegrationCreate = "integrationCreate" => gateway::IntegrationCreate [GUILD_INTEGRATIONS];

    /// An integration was updated (`INTEGRATION_UPDATE`). Intent: `GUILD_INTEGRATIONS`.
    ///
    /// discord.js has no such event (C-091).
    IntegrationUpdate = "integrationUpdate" => gateway::IntegrationUpdate [GUILD_INTEGRATIONS];

    /// An integration was deleted (`INTEGRATION_DELETE`). Intent: `GUILD_INTEGRATIONS`.
    ///
    /// discord.js has no such event (C-091).
    IntegrationDelete = "integrationDelete" => gateway::IntegrationDelete [GUILD_INTEGRATIONS];

    /// A user joined a guild (`GUILD_MEMBER_ADD`). Intent: `GUILD_MEMBERS` (privileged).
    ///
    /// discord.js: `Events.GuildMemberAdd`.
    GuildMemberAdd = "guildMemberAdd" => gateway::GuildMemberAdd [GUILD_MEMBERS];

    /// A member was updated (`GUILD_MEMBER_UPDATE`), with the cached member before the update.
    /// Intent: `GUILD_MEMBERS` (privileged).
    ///
    /// discord.js: `Events.GuildMemberUpdate` (`oldMember`, `newMember`).
    GuildMemberUpdate = "guildMemberUpdate" => payload::GuildMemberUpdate [GUILD_MEMBERS];

    /// A user left or was removed from a guild (`GUILD_MEMBER_REMOVE`), with the cached
    /// member. Intent: `GUILD_MEMBERS` (privileged).
    ///
    /// discord.js: `Events.GuildMemberRemove`.
    GuildMemberRemove = "guildMemberRemove" => payload::GuildMemberRemove [GUILD_MEMBERS];

    /// A chunk of a guild's members (`GUILD_MEMBERS_CHUNK`), the answer to a
    /// [`RequestGuildMembers`](djs_model::gateway::payload::RequestGuildMembers) command.
    ///
    /// discord.js: `Events.GuildMembersChunk`.
    GuildMembersChunk = "guildMembersChunk" => gateway::GuildMembersChunk [];

    /// A role was created (`GUILD_ROLE_CREATE`). Intent: `GUILDS`.
    ///
    /// discord.js: `Events.GuildRoleCreate` (`roleCreate`).
    GuildRoleCreate = "roleCreate" => gateway::GuildRoleCreate [GUILDS];

    /// A role was updated (`GUILD_ROLE_UPDATE`), with the cached role before the update.
    /// Intent: `GUILDS`.
    ///
    /// discord.js: `Events.GuildRoleUpdate` (`roleUpdate`; `oldRole`, `newRole`).
    GuildRoleUpdate = "roleUpdate" => payload::GuildRoleUpdate [GUILDS];

    /// A role was deleted (`GUILD_ROLE_DELETE`), with the cached role. Intent: `GUILDS`.
    ///
    /// discord.js: `Events.GuildRoleDelete` (`roleDelete`).
    GuildRoleDelete = "roleDelete" => payload::GuildRoleDelete [GUILDS];

    /// A scheduled event was created (`GUILD_SCHEDULED_EVENT_CREATE`). Intent:
    /// `GUILD_SCHEDULED_EVENTS`.
    ///
    /// discord.js: `Events.GuildScheduledEventCreate`.
    GuildScheduledEventCreate = "guildScheduledEventCreate"
        => gateway::GuildScheduledEventCreate [GUILD_SCHEDULED_EVENTS];

    /// A scheduled event was updated (`GUILD_SCHEDULED_EVENT_UPDATE`), with the cached event
    /// before the update. Intent: `GUILD_SCHEDULED_EVENTS`.
    ///
    /// discord.js: `Events.GuildScheduledEventUpdate`.
    GuildScheduledEventUpdate = "guildScheduledEventUpdate"
        => payload::GuildScheduledEventUpdate [GUILD_SCHEDULED_EVENTS];

    /// A scheduled event was deleted (`GUILD_SCHEDULED_EVENT_DELETE`), with the cached event.
    /// Intent: `GUILD_SCHEDULED_EVENTS`.
    ///
    /// discord.js: `Events.GuildScheduledEventDelete`.
    GuildScheduledEventDelete = "guildScheduledEventDelete"
        => payload::GuildScheduledEventDelete [GUILD_SCHEDULED_EVENTS];

    /// A user subscribed to a scheduled event (`GUILD_SCHEDULED_EVENT_USER_ADD`). Intent:
    /// `GUILD_SCHEDULED_EVENTS`.
    ///
    /// discord.js: `Events.GuildScheduledEventUserAdd`.
    GuildScheduledEventUserAdd = "guildScheduledEventUserAdd"
        => gateway::GuildScheduledEventUserAdd [GUILD_SCHEDULED_EVENTS];

    /// A user unsubscribed from a scheduled event (`GUILD_SCHEDULED_EVENT_USER_REMOVE`).
    /// Intent: `GUILD_SCHEDULED_EVENTS`.
    ///
    /// discord.js: `Events.GuildScheduledEventUserRemove`.
    GuildScheduledEventUserRemove = "guildScheduledEventUserRemove"
        => gateway::GuildScheduledEventUserRemove [GUILD_SCHEDULED_EVENTS];

    /// A soundboard sound was created (`GUILD_SOUNDBOARD_SOUND_CREATE`). Intent:
    /// `GUILD_EXPRESSIONS`.
    ///
    /// discord.js: `Events.GuildSoundboardSoundCreate`.
    GuildSoundboardSoundCreate = "guildSoundboardSoundCreate"
        => gateway::GuildSoundboardSoundCreate [GUILD_EXPRESSIONS];

    /// A soundboard sound was updated (`GUILD_SOUNDBOARD_SOUND_UPDATE`), with the cached sound
    /// before the update. Intent: `GUILD_EXPRESSIONS`.
    ///
    /// discord.js: `Events.GuildSoundboardSoundUpdate`.
    GuildSoundboardSoundUpdate = "guildSoundboardSoundUpdate"
        => payload::GuildSoundboardSoundUpdate [GUILD_EXPRESSIONS];

    /// A soundboard sound was deleted (`GUILD_SOUNDBOARD_SOUND_DELETE`), with the cached
    /// sound. Intent: `GUILD_EXPRESSIONS`.
    ///
    /// discord.js: `Events.GuildSoundboardSoundDelete`.
    GuildSoundboardSoundDelete = "guildSoundboardSoundDelete"
        => payload::GuildSoundboardSoundDelete [GUILD_EXPRESSIONS];

    /// Several soundboard sounds were updated at once (`GUILD_SOUNDBOARD_SOUNDS_UPDATE`).
    /// Intent: `GUILD_EXPRESSIONS`.
    ///
    /// discord.js: `Events.GuildSoundboardSoundsUpdate`.
    GuildSoundboardSoundsUpdate = "guildSoundboardSoundsUpdate"
        => gateway::GuildSoundboardSoundsUpdate [GUILD_EXPRESSIONS];

    /// A guild's soundboard sounds (`SOUNDBOARD_SOUNDS`), the answer to a
    /// [`RequestSoundboardSounds`](djs_model::gateway::payload::RequestSoundboardSounds)
    /// command.
    ///
    /// discord.js: `Events.SoundboardSounds`.
    SoundboardSounds = "soundboardSounds" => gateway::SoundboardSounds [];

    /// A stage instance was created (`STAGE_INSTANCE_CREATE`). Intent: `GUILDS`.
    ///
    /// discord.js: `Events.StageInstanceCreate`.
    StageInstanceCreate = "stageInstanceCreate" => gateway::StageInstanceCreate [GUILDS];

    /// A stage instance was updated (`STAGE_INSTANCE_UPDATE`), with the cached instance before
    /// the update. Intent: `GUILDS`.
    ///
    /// discord.js: `Events.StageInstanceUpdate`.
    StageInstanceUpdate = "stageInstanceUpdate" => payload::StageInstanceUpdate [GUILDS];

    /// A stage instance was deleted (`STAGE_INSTANCE_DELETE`), with the cached instance.
    /// Intent: `GUILDS`.
    ///
    /// discord.js: `Events.StageInstanceDelete`.
    StageInstanceDelete = "stageInstanceDelete" => payload::StageInstanceDelete [GUILDS];

    // --- Interactions, invites, messages -----------------------------------------------

    /// A user used a command, a component or a modal (`INTERACTION_CREATE`).
    ///
    /// discord.js: `Events.InteractionCreate`.
    InteractionCreate = "interactionCreate" => gateway::InteractionCreate [];

    /// An invite was created (`INVITE_CREATE`). Intent: `GUILD_INVITES`.
    ///
    /// discord.js: `Events.InviteCreate`.
    InviteCreate = "inviteCreate" => gateway::InviteCreate [GUILD_INVITES];

    /// An invite was deleted (`INVITE_DELETE`). Intent: `GUILD_INVITES`.
    ///
    /// discord.js: `Events.InviteDelete`.
    InviteDelete = "inviteDelete" => gateway::InviteDelete [GUILD_INVITES];

    /// A message was sent (`MESSAGE_CREATE`). Intents: `GUILD_MESSAGES`, `DIRECT_MESSAGES`.
    ///
    /// Guild message content needs the privileged `MESSAGE_CONTENT` intent.
    ///
    /// discord.js: `Events.MessageCreate`.
    MessageCreate = "messageCreate" => gateway::MessageCreate [GUILD_MESSAGES | DIRECT_MESSAGES];

    /// A message was edited (`MESSAGE_UPDATE`), with the cached message before the edit.
    /// Intents: `GUILD_MESSAGES`, `DIRECT_MESSAGES`.
    ///
    /// discord.js: `Events.MessageUpdate` (`oldMessage`, `newMessage`).
    MessageUpdate = "messageUpdate" => payload::MessageUpdate [GUILD_MESSAGES | DIRECT_MESSAGES];

    /// A message was deleted (`MESSAGE_DELETE`), with the cached message. Intents:
    /// `GUILD_MESSAGES`, `DIRECT_MESSAGES`.
    ///
    /// discord.js: `Events.MessageDelete`.
    MessageDelete = "messageDelete" => payload::MessageDelete [GUILD_MESSAGES | DIRECT_MESSAGES];

    /// Messages were deleted at once (`MESSAGE_DELETE_BULK`), with the cached ones. Intent:
    /// `GUILD_MESSAGES`.
    ///
    /// discord.js: `Events.MessageBulkDelete` (`messageDeleteBulk`).
    MessageBulkDelete = "messageDeleteBulk" => payload::MessageBulkDelete [GUILD_MESSAGES];

    /// A reaction was added (`MESSAGE_REACTION_ADD`). Intents: `GUILD_MESSAGE_REACTIONS`,
    /// `DIRECT_MESSAGE_REACTIONS`.
    ///
    /// discord.js: `Events.MessageReactionAdd`.
    MessageReactionAdd = "messageReactionAdd" => gateway::MessageReactionAdd [GUILD_MESSAGE_REACTIONS | DIRECT_MESSAGE_REACTIONS];

    /// A reaction was removed (`MESSAGE_REACTION_REMOVE`). Intents: `GUILD_MESSAGE_REACTIONS`,
    /// `DIRECT_MESSAGE_REACTIONS`.
    ///
    /// discord.js: `Events.MessageReactionRemove`.
    MessageReactionRemove = "messageReactionRemove" => gateway::MessageReactionRemove [GUILD_MESSAGE_REACTIONS | DIRECT_MESSAGE_REACTIONS];

    /// All reactions were removed from a message (`MESSAGE_REACTION_REMOVE_ALL`). Intents:
    /// `GUILD_MESSAGE_REACTIONS`, `DIRECT_MESSAGE_REACTIONS`.
    ///
    /// discord.js: `Events.MessageReactionRemoveAll`.
    MessageReactionRemoveAll = "messageReactionRemoveAll" => gateway::MessageReactionRemoveAll [GUILD_MESSAGE_REACTIONS | DIRECT_MESSAGE_REACTIONS];

    /// All reactions with one emoji were removed (`MESSAGE_REACTION_REMOVE_EMOJI`). Intents:
    /// `GUILD_MESSAGE_REACTIONS`, `DIRECT_MESSAGE_REACTIONS`.
    ///
    /// discord.js: `Events.MessageReactionRemoveEmoji`.
    MessageReactionRemoveEmoji = "messageReactionRemoveEmoji"
        => gateway::MessageReactionRemoveEmoji [GUILD_MESSAGE_REACTIONS | DIRECT_MESSAGE_REACTIONS];

    /// A user voted in a poll (`MESSAGE_POLL_VOTE_ADD`). Intents: `GUILD_MESSAGE_POLLS`,
    /// `DIRECT_MESSAGE_POLLS`.
    ///
    /// discord.js: `Events.MessagePollVoteAdd`.
    MessagePollVoteAdd = "messagePollVoteAdd" => gateway::MessagePollVoteAdd [GUILD_MESSAGE_POLLS | DIRECT_MESSAGE_POLLS];

    /// A user removed a poll vote (`MESSAGE_POLL_VOTE_REMOVE`). Intents: `GUILD_MESSAGE_POLLS`,
    /// `DIRECT_MESSAGE_POLLS`.
    ///
    /// discord.js: `Events.MessagePollVoteRemove`.
    MessagePollVoteRemove = "messagePollVoteRemove" => gateway::MessagePollVoteRemove [GUILD_MESSAGE_POLLS | DIRECT_MESSAGE_POLLS];

    /// A user started typing (`TYPING_START`). Intents: `GUILD_MESSAGE_TYPING`,
    /// `DIRECT_MESSAGE_TYPING`.
    ///
    /// discord.js: `Events.TypingStart`.
    TypingStart = "typingStart" => gateway::TypingStart [GUILD_MESSAGE_TYPING | DIRECT_MESSAGE_TYPING];

    // --- Users, presences, voice, webhooks, rate limits -------------------------------

    /// A user's presence changed (`PRESENCE_UPDATE`), with the cached presence before the
    /// change. Intent: `GUILD_PRESENCES` (privileged).
    ///
    /// discord.js: `Events.PresenceUpdate` (`oldPresence`, `newPresence`).
    PresenceUpdate = "presenceUpdate" => payload::PresenceUpdate [GUILD_PRESENCES];

    /// The bot's user was updated (`USER_UPDATE`), with the cached user before the update.
    ///
    /// discord.js: `Events.UserUpdate` (`oldUser`, `newUser`).
    UserUpdate = "userUpdate" => payload::UserUpdate [];

    /// A user joined, left or moved between voice channels, or changed their voice state
    /// (`VOICE_STATE_UPDATE`), with the cached state before the change. Intent:
    /// `GUILD_VOICE_STATES`.
    ///
    /// discord.js: `Events.VoiceStateUpdate` (`oldState`, `newState`).
    VoiceStateUpdate = "voiceStateUpdate" => payload::VoiceStateUpdate [GUILD_VOICE_STATES];

    /// The voice server of a guild the bot is connecting to (`VOICE_SERVER_UPDATE`).
    ///
    /// discord.js: `Events.VoiceServerUpdate`.
    VoiceServerUpdate = "voiceServerUpdate" => gateway::VoiceServerUpdate [];

    /// A user sent an effect, such as an emoji, in a voice channel
    /// (`VOICE_CHANNEL_EFFECT_SEND`). Intent: `GUILD_VOICE_STATES`.
    ///
    /// discord.js: `Events.VoiceChannelEffectSend`.
    VoiceChannelEffectSend = "voiceChannelEffectSend" => gateway::VoiceChannelEffectSend [GUILD_VOICE_STATES];

    /// A voice channel's status was set or cleared (`VOICE_CHANNEL_STATUS_UPDATE`). Intent:
    /// `GUILDS`.
    ///
    /// discord.js has no such event (C-091).
    VoiceChannelStatusUpdate = "voiceChannelStatusUpdate" => gateway::VoiceChannelStatusUpdate [GUILDS];

    /// A voice channel's start time changed (`VOICE_CHANNEL_START_TIME_UPDATE`). Intent:
    /// `GUILDS`.
    ///
    /// discord.js has no such event (C-091).
    VoiceChannelStartTimeUpdate = "voiceChannelStartTimeUpdate"
        => gateway::VoiceChannelStartTimeUpdate [GUILDS];

    /// A channel's webhooks changed (`WEBHOOKS_UPDATE`). Intent: `GUILD_WEBHOOKS`.
    ///
    /// discord.js: `Events.WebhooksUpdate`.
    WebhooksUpdate = "webhooksUpdate" => gateway::WebhooksUpdate [GUILD_WEBHOOKS];

    /// A gateway command sent by the bot was rate limited (`RATE_LIMITED`), for example a
    /// [`RequestGuildMembers`](djs_model::gateway::payload::RequestGuildMembers).
    ///
    /// discord.js has no such event (C-091).
    RateLimited = "rateLimited" => gateway::RateLimited [];
}

/// An event delivered by [`Client::events`](crate::Client::events): the shard it came from
/// and the event.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ShardEvent {
    /// The shard that received the event.
    pub shard: ShardId,
    /// The event.
    pub event: Dispatch,
}
