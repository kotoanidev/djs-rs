//! Managers: the operations of one resource, reached from the [`Context`] (D-229).
//!
//! ```no_run
//! # use djs_rs::prelude::*;
//! # use djs_rs::model::id::{GuildId, UserId};
//! # async fn run(ctx: Context, guild: GuildId, user: UserId) -> Result<(), djs_rs::rest::Error> {
//! // The cache first, then the REST API; the REST result is cached (D-230).
//! let member = ctx.members(guild).fetch(user).await?;
//! println!("{} joined at {:?}", member.user.username, member.joined_at);
//!
//! // Always ask Discord.
//! let fresh = ctx.members(guild).fetch(user).force().await?;
//!
//! // Lists are `Collection`s in Discord's order (D-231).
//! let roles = ctx.roles(guild).list().await?;
//! let hoisted = roles.filter(|role, _| role.hoist);
//! # let _ = (fresh, hoisted);
//! # Ok(())
//! # }
//! ```
//!
//! # What managers return
//!
//! The type the cache stores (D-233): `Arc<CachedMember>`, `Arc<CachedMessage>`,
//! `Arc<CachedGuild>`, `Arc<CachedUser>` for the compact cached types, `Arc<Channel>` and
//! `Arc<Role>` where the cache keeps the model type — whether the value came from the cache or
//! from Discord. A REST result is converted to that type and stored when its resource is
//! enabled in the [`CacheConfig`](djs_cache::CacheConfig); without a cache it is only
//! converted. Objects the cache never keeps (bans) are returned as their
//! [`djs-model`](djs_model) type.
//!
//! Managers are small `Copy` values borrowing the context; their methods consume them, so
//! `ctx.members(guild).kick(user, None).await` needs no binding.
//!
//! discord.js counterpart: the `*Manager` classes (`GuildMemberManager`, `MessageManager`, …).

mod application_emoji;
mod auto_moderation;
mod ban;
mod channel;
mod command;
mod emoji;
mod gateway;
mod guild;
mod guild_channel;
mod invite;
mod member;
mod message;
mod role;
mod scheduled_event;
mod soundboard;
mod stage_instance;
mod sticker;
mod thread;
mod user;
mod webhook;

use core::{
    fmt,
    future::{Future, IntoFuture},
    pin::Pin,
    task::{Context as TaskContext, Poll},
};
use std::sync::Arc;

use djs_cache::{Backend, CachedGuild, CachedMember, CachedMessage, CachedUser};
use djs_collection::Collection;
use djs_model::{
    channel::Channel,
    emoji::Emoji,
    guild::{Guild, Member, Role},
    guild_scheduled_event::GuildScheduledEvent,
    id::{ChannelId, GuildId, SoundboardSoundId, StickerId, StickerPackId},
    message::Message,
    soundboard::SoundboardSound,
    stage_instance::StageInstance,
    sticker::{Sticker, StickerPack},
    user::User,
};
use djs_rest::Error;

pub use self::{
    application_emoji::ApplicationEmojiManager,
    auto_moderation::AutoModerationRuleManager,
    ban::BanManager,
    channel::{ChannelManager, OverwriteTarget},
    command::{CommandManager, GuildCommandManager},
    emoji::EmojiManager,
    gateway::{FetchMembers, GatewayRequestError, GuildSoundboardSounds},
    guild::GuildManager,
    guild_channel::GuildChannelManager,
    invite::InviteManager,
    member::MemberManager,
    message::MessageManager,
    role::RoleManager,
    scheduled_event::ScheduledEventManager,
    soundboard::SoundboardSoundManager,
    stage_instance::StageInstanceManager,
    sticker::StickerManager,
    thread::{FetchedThreads, ThreadManager, ThreadMemberManager},
    user::UserManager,
    webhook::WebhookManager,
};
use crate::Context;

/// A `fetch` that is not sent yet: await it to read the cache first and ask Discord when the
/// cache does not hold the value (D-230), or call [`force`](Self::force) first to always ask
/// Discord.
///
/// A cache hit allocates nothing; the REST request is only made (and boxed) when needed.
///
/// discord.js counterpart: the `fetch(id, { force })` methods of the managers.
#[must_use = "a fetch does nothing until it is awaited"]
pub struct Fetch<T, F> {
    cached: Option<T>,
    rest: F,
    force: bool,
}

impl<T, F> Fetch<T, F> {
    pub(crate) const fn new(cached: Option<T>, rest: F) -> Self {
        Self {
            cached,
            rest,
            force: false,
        }
    }

    /// Skips the cache: the value is always requested from Discord (and the cache updated).
    ///
    /// discord.js counterpart: the `force: true` fetch option.
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Whether the cache holds the value, so awaiting this fetch (without
    /// [`force`](Self::force)) makes no request.
    #[must_use]
    pub const fn is_cached(&self) -> bool {
        self.cached.is_some()
    }
}

impl<T, F> fmt::Debug for Fetch<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fetch")
            .field("cached", &self.cached.is_some())
            .field("force", &self.force)
            .finish_non_exhaustive()
    }
}

impl<T: Unpin, F: Future<Output = Result<T, Error>>> IntoFuture for Fetch<T, F> {
    type Output = Result<T, Error>;
    type IntoFuture = FetchFuture<T, F>;

    fn into_future(self) -> FetchFuture<T, F> {
        match self.cached {
            Some(cached) if !self.force => FetchFuture(State::Cached(Some(cached))),
            _ => FetchFuture(State::Rest(Box::pin(self.rest))),
        }
    }
}

/// The future of an awaited [`Fetch`].
#[must_use = "futures do nothing unless polled"]
pub struct FetchFuture<T, F>(State<T, F>);

enum State<T, F> {
    Cached(Option<T>),
    Rest(Pin<Box<F>>),
}

impl<T, F> fmt::Debug for FetchFuture<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.0 {
            State::Cached(_) => "cached",
            State::Rest(_) => "requesting",
        };
        f.debug_tuple("FetchFuture").field(&state).finish()
    }
}

impl<T: Unpin, F: Future<Output = Result<T, Error>>> Future for FetchFuture<T, F> {
    type Output = Result<T, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        match &mut self.get_mut().0 {
            State::Cached(cached) => Poll::Ready(Ok(cached
                .take()
                .expect("`FetchFuture` polled after completion"))),
            State::Rest(rest) => rest.as_mut().poll(cx),
        }
    }
}

/// Converts REST objects to the cached types through the context's cache (D-230, D-233),
/// with the code compiled in djs-rs (D-289).
impl<B: Backend> Context<B> {
    pub(crate) fn store_guild(&self, guild: &Guild) -> Arc<CachedGuild> {
        self.ops().store_guild(guild)
    }

    pub(crate) fn store_channel(&self, channel: &Channel) -> Arc<Channel> {
        self.ops().store_channel(channel)
    }

    pub(crate) fn store_user(&self, user: &User) -> Arc<CachedUser> {
        self.ops().store_user(user)
    }

    pub(crate) fn store_member(&self, guild_id: GuildId, member: &Member) -> Arc<CachedMember> {
        self.ops().store_member(guild_id, member)
    }

    pub(crate) fn store_message(&self, message: &Message) -> Arc<CachedMessage> {
        self.ops().store_message(message)
    }

    pub(crate) fn store_role(&self, guild_id: GuildId, role: &Role) -> Arc<Role> {
        self.ops().store_role(guild_id, role)
    }

    pub(crate) fn store_emoji(&self, guild_id: GuildId, emoji: &Emoji) -> Arc<Emoji> {
        self.ops().store_emoji(guild_id, emoji)
    }

    pub(crate) fn store_sticker(&self, guild_id: GuildId, sticker: &Sticker) -> Arc<Sticker> {
        self.ops().store_sticker(guild_id, sticker)
    }

    pub(crate) fn store_scheduled_event(
        &self,
        guild_id: GuildId,
        event: &GuildScheduledEvent,
    ) -> Arc<GuildScheduledEvent> {
        self.ops().store_scheduled_event(guild_id, event)
    }

    pub(crate) fn store_stage_instance(
        &self,
        guild_id: GuildId,
        stage: &StageInstance,
    ) -> Arc<StageInstance> {
        self.ops().store_stage_instance(guild_id, stage)
    }

    pub(crate) fn store_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound: &SoundboardSound,
    ) -> Arc<SoundboardSound> {
        self.ops().store_soundboard_sound(guild_id, sound)
    }

    /// Returns the guild manager (D-229).
    ///
    /// discord.js counterpart: `Client#guilds`.
    #[must_use]
    pub const fn guilds(&self) -> GuildManager<'_, B> {
        GuildManager::new(self)
    }

    /// Returns the channel manager.
    ///
    /// discord.js counterpart: `Client#channels`.
    #[must_use]
    pub const fn channels(&self) -> ChannelManager<'_, B> {
        ChannelManager::new(self)
    }

    /// Returns the manager of a guild's channels as a whole: create, list, positions (D-234).
    ///
    /// discord.js counterpart: `Guild#channels`.
    #[must_use]
    pub const fn guild_channels(&self, guild_id: GuildId) -> GuildChannelManager<'_, B> {
        GuildChannelManager::new(self, guild_id)
    }

    /// Returns the user manager.
    ///
    /// discord.js counterpart: `Client#users`.
    #[must_use]
    pub const fn users(&self) -> UserManager<'_, B> {
        UserManager::new(self)
    }

    /// Returns the manager of a guild's members.
    ///
    /// discord.js counterpart: `Guild#members`.
    #[must_use]
    pub const fn members(&self, guild_id: GuildId) -> MemberManager<'_, B> {
        MemberManager::new(self, guild_id)
    }

    /// Returns the manager of a channel's messages.
    ///
    /// discord.js counterpart: `TextChannel#messages` (and `TextChannel#send`).
    #[must_use]
    pub const fn messages(&self, channel_id: ChannelId) -> MessageManager<'_, B> {
        MessageManager::new(self, channel_id)
    }

    /// Returns the manager of a guild's roles.
    ///
    /// discord.js counterpart: `Guild#roles`.
    #[must_use]
    pub const fn roles(&self, guild_id: GuildId) -> RoleManager<'_, B> {
        RoleManager::new(self, guild_id)
    }

    /// Returns the manager of a guild's bans.
    ///
    /// discord.js counterpart: `Guild#bans`.
    #[must_use]
    pub const fn bans(&self, guild_id: GuildId) -> BanManager<'_, B> {
        BanManager::new(self, guild_id)
    }

    /// Returns the manager of a guild's custom emojis.
    ///
    /// discord.js counterpart: `Guild#emojis`.
    #[must_use]
    pub const fn emojis(&self, guild_id: GuildId) -> EmojiManager<'_, B> {
        EmojiManager::new(self, guild_id)
    }

    /// Returns the manager of a guild's stickers.
    ///
    /// discord.js counterpart: `Guild#stickers`.
    #[must_use]
    pub const fn stickers(&self, guild_id: GuildId) -> StickerManager<'_, B> {
        StickerManager::new(self, guild_id)
    }

    /// Returns the manager of a guild's scheduled events.
    ///
    /// discord.js counterpart: `Guild#scheduledEvents`.
    #[must_use]
    pub const fn scheduled_events(&self, guild_id: GuildId) -> ScheduledEventManager<'_, B> {
        ScheduledEventManager::new(self, guild_id)
    }

    /// Returns the manager of a guild's live stages.
    ///
    /// discord.js counterpart: `Guild#stageInstances`.
    #[must_use]
    pub const fn stage_instances(&self, guild_id: GuildId) -> StageInstanceManager<'_, B> {
        StageInstanceManager::new(self, guild_id)
    }

    /// Returns the manager of a guild's soundboard sounds.
    ///
    /// discord.js counterpart: `Guild#soundboardSounds`.
    #[must_use]
    pub const fn soundboard_sounds(&self, guild_id: GuildId) -> SoundboardSoundManager<'_, B> {
        SoundboardSoundManager::new(self, guild_id)
    }

    /// Returns the manager of the threads of a parent channel (D-242).
    ///
    /// discord.js counterpart: `TextChannel#threads` / `ForumChannel#threads`.
    #[must_use]
    pub const fn threads(&self, channel_id: ChannelId) -> ThreadManager<'_, B> {
        ThreadManager::new(self, channel_id)
    }

    /// Returns the manager of a thread's members (D-242).
    ///
    /// discord.js counterpart: `ThreadChannel#members`.
    #[must_use]
    pub const fn thread_members(&self, thread_id: ChannelId) -> ThreadMemberManager<'_, B> {
        ThreadMemberManager::new(self, thread_id)
    }

    /// Discord's default soundboard sounds, available in every guild ([List Default
    /// Soundboard Sounds](djs_rest::Rest::list_default_soundboard_sounds); D-243). They are
    /// not cached.
    ///
    /// discord.js counterpart: `Client#fetchDefaultSoundboardSounds`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn default_soundboard_sounds(
        &self,
    ) -> Result<Collection<SoundboardSoundId, SoundboardSound>, Error> {
        let sounds = self.rest().list_default_soundboard_sounds().await?;
        Ok(sounds
            .into_iter()
            .map(|sound| (sound.sound_id, sound))
            .collect())
    }

    /// Returns the manager of the bot's global application commands (D-244).
    ///
    /// discord.js counterpart: `client.application.commands`.
    ///
    /// # Panics
    ///
    /// Panics before the first `READY`, while the application ID is unknown (D-247): only in
    /// `ShardError`, `Invalidated` or `ShardDisconnect` handlers running while logging in.
    #[must_use]
    #[track_caller]
    pub fn commands(&self) -> CommandManager<'_, B> {
        CommandManager::new(self, self.expect_application_id())
    }

    /// Returns the manager of the bot's application commands in a guild, and their
    /// permissions there (D-244).
    ///
    /// discord.js counterpart: `Guild#commands`.
    ///
    /// # Panics
    ///
    /// Panics before the first `READY` (D-247), like [`commands`](Self::commands).
    #[must_use]
    #[track_caller]
    pub fn guild_commands(&self, guild_id: GuildId) -> GuildCommandManager<'_, B> {
        GuildCommandManager::new(self, self.expect_application_id(), guild_id)
    }

    /// Returns the manager of the application's own emojis.
    ///
    /// discord.js counterpart: `client.application.emojis`.
    ///
    /// # Panics
    ///
    /// Panics before the first `READY` (D-247), like [`commands`](Self::commands).
    #[must_use]
    #[track_caller]
    pub fn application_emojis(&self) -> ApplicationEmojiManager<'_, B> {
        ApplicationEmojiManager::new(self, self.expect_application_id())
    }

    /// Returns the manager of a guild's auto moderation rules.
    ///
    /// discord.js counterpart: `Guild#autoModerationRules`.
    #[must_use]
    pub const fn auto_moderation_rules(
        &self,
        guild_id: GuildId,
    ) -> AutoModerationRuleManager<'_, B> {
        AutoModerationRuleManager::new(self, guild_id)
    }

    /// A sticker by its ID, standard or from a guild ([Get Sticker](djs_rest::Rest::get_sticker)).
    /// Not cached.
    ///
    /// discord.js counterpart: `Client#fetchSticker`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn sticker(&self, sticker_id: StickerId) -> Result<Sticker, Error> {
        self.rest().get_sticker(sticker_id).await
    }

    /// Discord's standard sticker packs ([List Sticker
    /// Packs](djs_rest::Rest::list_sticker_packs)). Not cached.
    ///
    /// discord.js counterpart: `Client#fetchStickerPacks`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn sticker_packs(&self) -> Result<Collection<StickerPackId, StickerPack>, Error> {
        let packs = self.rest().list_sticker_packs().await?;
        Ok(packs.into_iter().map(|pack| (pack.id, pack)).collect())
    }

    /// One standard sticker pack ([Get Sticker Pack](djs_rest::Rest::get_sticker_pack)). Not
    /// cached.
    ///
    /// discord.js counterpart: `Client#fetchStickerPacks({ packId })`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn sticker_pack(&self, pack_id: StickerPackId) -> Result<StickerPack, Error> {
        self.rest().get_sticker_pack(pack_id).await
    }

    /// Returns the invite manager (D-239).
    ///
    /// discord.js counterpart: `Guild#invites` and `Client#fetchInvite`.
    #[must_use]
    pub const fn invites(&self) -> InviteManager<'_, B> {
        InviteManager::new(self)
    }

    /// Returns the webhook manager (D-237).
    ///
    /// discord.js counterpart: the webhook methods of `TextChannel`, `Guild` and `Client`.
    #[must_use]
    pub const fn webhooks(&self) -> WebhookManager<'_, B> {
        WebhookManager::new(self)
    }
}
