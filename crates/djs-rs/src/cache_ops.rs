//! The cache operations of the managers and structure methods, behind a trait object
//! (D-289).
//!
//! Managers and [`Context`](crate::Context) are generic over the cache backend, so code
//! calling [`Cache`] methods directly is compiled in the crate that uses them (the bot's).
//! Instead they go through `&dyn CacheOps`, created once per context where the client loop
//! builds it: for the default backend that is inside djs-rs (D-287), so the cache code is
//! compiled once, in the library. A call costs one indirect call.

use std::sync::{Arc, LazyLock};

use djs_cache::{
    Backend, Cache, CacheConfig, CachedGuild, CachedMember, CachedMessage, CachedUser, Limit,
};
use djs_model::{
    channel::Channel,
    emoji::Emoji,
    guild::{Guild, Member, Role},
    guild_scheduled_event::GuildScheduledEvent,
    id::{
        ChannelId, EmojiId, GuildId, MessageId, RoleId, ScheduledEventId, SoundboardSoundId,
        StickerId, UserId,
    },
    message::Message,
    permissions::Permissions,
    soundboard::SoundboardSound,
    stage_instance::StageInstance,
    sticker::Sticker,
    user::User,
};

/// What the managers do with the cache: convert (and, where the resource is enabled, store)
/// REST objects, and read cached values (D-230, D-233).
///
/// The reads return `None` when the resource is not cached, like the [`Cache`] views.
pub(crate) trait CacheOps: Send + Sync {
    fn store_guild(&self, guild: &Guild) -> Arc<CachedGuild>;
    fn store_channel(&self, channel: &Channel) -> Arc<Channel>;
    fn store_user(&self, user: &User) -> Arc<CachedUser>;
    fn store_member(&self, guild_id: GuildId, member: &Member) -> Arc<CachedMember>;
    fn store_message(&self, message: &Message) -> Arc<CachedMessage>;
    fn store_role(&self, guild_id: GuildId, role: &Role) -> Arc<Role>;
    fn store_emoji(&self, guild_id: GuildId, emoji: &Emoji) -> Arc<Emoji>;
    fn store_sticker(&self, guild_id: GuildId, sticker: &Sticker) -> Arc<Sticker>;
    fn store_scheduled_event(
        &self,
        guild_id: GuildId,
        event: &GuildScheduledEvent,
    ) -> Arc<GuildScheduledEvent>;
    fn store_stage_instance(&self, guild_id: GuildId, stage: &StageInstance) -> Arc<StageInstance>;
    fn store_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound: &SoundboardSound,
    ) -> Arc<SoundboardSound>;

    fn guild(&self, guild_id: GuildId) -> Option<Arc<CachedGuild>>;
    fn channel(&self, channel_id: ChannelId) -> Option<Arc<Channel>>;
    fn user(&self, user_id: UserId) -> Option<Arc<CachedUser>>;
    fn member(&self, guild_id: GuildId, user_id: UserId) -> Option<Arc<CachedMember>>;
    fn message(&self, channel_id: ChannelId, message_id: MessageId) -> Option<Arc<CachedMessage>>;
    fn role(&self, guild_id: GuildId, role_id: RoleId) -> Option<Arc<Role>>;
    fn emoji(&self, guild_id: GuildId, emoji_id: EmojiId) -> Option<Arc<Emoji>>;
    fn sticker(&self, guild_id: GuildId, sticker_id: StickerId) -> Option<Arc<Sticker>>;
    fn scheduled_event(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
    ) -> Option<Arc<GuildScheduledEvent>>;
    /// The live stage of a stage channel (the cache keys them by their own ID).
    fn stage_instance_of(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<Arc<StageInstance>>;
    fn soundboard_sound(
        &self,
        guild_id: GuildId,
        sound_id: SoundboardSoundId,
    ) -> Option<Arc<SoundboardSound>>;

    /// [`Cache::member_permissions`].
    fn member_permissions(&self, guild_id: GuildId, user_id: UserId) -> Option<Permissions>;
    /// [`Cache::member_permissions_in`] for a cached channel or thread of `guild_id`.
    fn member_permissions_in(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Option<Permissions>;
}

impl<B: Backend> CacheOps for Cache<B> {
    fn store_guild(&self, guild: &Guild) -> Arc<CachedGuild> {
        Cache::store_guild(self, guild)
    }

    fn store_channel(&self, channel: &Channel) -> Arc<Channel> {
        Cache::store_channel(self, channel)
    }

    fn store_user(&self, user: &User) -> Arc<CachedUser> {
        Cache::store_user(self, user)
    }

    fn store_member(&self, guild_id: GuildId, member: &Member) -> Arc<CachedMember> {
        Cache::store_member(self, guild_id, member)
    }

    fn store_message(&self, message: &Message) -> Arc<CachedMessage> {
        Cache::store_message(self, message)
    }

    fn store_role(&self, guild_id: GuildId, role: &Role) -> Arc<Role> {
        Cache::store_role(self, guild_id, role)
    }

    fn store_emoji(&self, guild_id: GuildId, emoji: &Emoji) -> Arc<Emoji> {
        Cache::store_emoji(self, guild_id, emoji)
    }

    fn store_sticker(&self, guild_id: GuildId, sticker: &Sticker) -> Arc<Sticker> {
        Cache::store_sticker(self, guild_id, sticker)
    }

    fn store_scheduled_event(
        &self,
        guild_id: GuildId,
        event: &GuildScheduledEvent,
    ) -> Arc<GuildScheduledEvent> {
        Cache::store_scheduled_event(self, guild_id, event)
    }

    fn store_stage_instance(&self, guild_id: GuildId, stage: &StageInstance) -> Arc<StageInstance> {
        Cache::store_stage_instance(self, guild_id, stage)
    }

    fn store_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound: &SoundboardSound,
    ) -> Arc<SoundboardSound> {
        Cache::store_soundboard_sound(self, guild_id, sound)
    }

    fn guild(&self, guild_id: GuildId) -> Option<Arc<CachedGuild>> {
        self.guilds().get(guild_id)
    }

    fn channel(&self, channel_id: ChannelId) -> Option<Arc<Channel>> {
        self.channels().get(channel_id)
    }

    fn user(&self, user_id: UserId) -> Option<Arc<CachedUser>> {
        self.users().get(user_id)
    }

    fn member(&self, guild_id: GuildId, user_id: UserId) -> Option<Arc<CachedMember>> {
        self.members(guild_id).get(user_id)
    }

    fn message(&self, channel_id: ChannelId, message_id: MessageId) -> Option<Arc<CachedMessage>> {
        self.messages(channel_id).get(message_id)
    }

    fn role(&self, guild_id: GuildId, role_id: RoleId) -> Option<Arc<Role>> {
        self.roles(guild_id).get(role_id)
    }

    fn emoji(&self, guild_id: GuildId, emoji_id: EmojiId) -> Option<Arc<Emoji>> {
        self.emojis(guild_id).get(emoji_id)
    }

    fn sticker(&self, guild_id: GuildId, sticker_id: StickerId) -> Option<Arc<Sticker>> {
        self.stickers(guild_id).get(sticker_id)
    }

    fn scheduled_event(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
    ) -> Option<Arc<GuildScheduledEvent>> {
        self.scheduled_events(guild_id).get(event_id)
    }

    fn stage_instance_of(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<Arc<StageInstance>> {
        self.stage_instances(guild_id)
            .values()
            .into_iter()
            .find(|stage| stage.channel_id == channel_id)
    }

    fn soundboard_sound(
        &self,
        guild_id: GuildId,
        sound_id: SoundboardSoundId,
    ) -> Option<Arc<SoundboardSound>> {
        self.soundboard_sounds(guild_id).get(sound_id)
    }

    fn member_permissions(&self, guild_id: GuildId, user_id: UserId) -> Option<Permissions> {
        Cache::member_permissions(self, guild_id, user_id)
    }

    fn member_permissions_in(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Option<Permissions> {
        let channel = self
            .channels()
            .get(channel_id)
            .or_else(|| self.threads(guild_id).get(channel_id))?;
        // A channel of another guild: the member has no permissions there.
        if channel.guild_id() != Some(guild_id) {
            return None;
        }
        Cache::member_permissions_in(self, &channel, user_id)
    }
}

/// A cache that keeps nothing: it converts REST objects for clients without a cache, and
/// its reads find nothing.
static CONVERT: LazyLock<Arc<Cache>> = LazyLock::new(|| {
    Arc::new(Cache::with_config(
        CacheConfig::builder()
            .guilds(Limit::Disabled)
            .channels(Limit::Disabled)
            .roles(Limit::Disabled)
            .build(),
    ))
});

/// The cache operations of a context: its cache's, or the converter's without one.
///
/// Not inlined, so that the converter's operations are compiled here, not where a context
/// is created with a custom backend.
#[inline(never)]
pub(crate) fn converter() -> Arc<dyn CacheOps> {
    Arc::<Cache>::clone(&CONVERT)
}
