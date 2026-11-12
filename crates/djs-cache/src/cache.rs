//! The cache itself.

use core::{fmt, hash::Hash, time::Duration};
use std::sync::{Arc, PoisonError, RwLock};

use dashmap::DashMap;
use djs_model::{
    channel::Channel,
    emoji::{Emoji, PartialEmoji},
    gateway::{
        DispatchEvent, Presence, Status,
        payload::{AvailableGuild, GuildMemberUpdate},
    },
    guild::{Guild, Member, PartialMember, Role},
    guild_scheduled_event::GuildScheduledEvent,
    id::{
        ChannelId, EmojiId, GuildId, MessageId, RoleId, ScheduledEventId, SoundboardSoundId,
        StageInstanceId, StickerId, UserId,
    },
    message::{Message, Reaction},
    soundboard::SoundboardSound,
    stage_instance::StageInstance,
    sticker::Sticker,
    user::{CurrentUser, User},
    util::Timestamp,
    voice::VoiceState,
};

use crate::{
    Backend, CacheConfig, CacheView, CachedGuild, CachedMember, CachedMessage, CachedPresence,
    ChannelView, DashMapBackend, GuildView, Previous, Resource, Store, UpdateCache,
    config::Scope,
    dashmap_store::Hasher,
    limits::{ChannelScopedOf, Global, GlobalOf, Index, Scoped, ScopedOf},
};

/// An in-memory view of Discord state, kept up to date from gateway events.
///
/// What it keeps is set by its [`CacheConfig`] (by default guilds, channels, roles and the
/// current user, D-152); where it keeps it by its [`Backend`] (by default
/// [`DashMapBackend`]). Feed it every dispatch event with [`update`](Self::update) or
/// [`update_dispatch`](Self::update_dispatch), and read it through the per-resource views
/// ([`guilds`](Self::guilds), [`members`](Self::members), …).
///
/// Reads return [`Arc`]s, which hold no lock and can be kept across `.await` (D-151). The
/// cache is `Send + Sync`: share one between all shards, for example in an `Arc`.
///
/// discord.js counterpart: the `cache` collections of the client's managers
/// (`client.guilds.cache`, `guild.members.cache`, …), filled by the client's event handlers.
///
/// # Examples
///
/// ```
/// use djs_cache::{Cache, CacheConfig, Limit};
/// use djs_model::{gateway::payload::GuildRoleCreate, id::GuildId};
/// # use djs_model::guild::Role;
/// # let role: Role = serde_json::from_str(r#"{"id":"2","name":"mods","color":0,"hoist":false,
/// #   "position":1,"permissions":"0","managed":false,"mentionable":false,"flags":0,
/// #   "colors":{"primary_color":0,"secondary_color":null,"tertiary_color":null}}"#).unwrap();
/// # let event: GuildRoleCreate = serde_json::from_value(serde_json::json!({
/// #     "guild_id": "1", "role": role })).unwrap();
///
/// let cache = Cache::with_config(CacheConfig::builder().members(Limit::Max(1_000)).build());
///
/// // `event` is a `GUILD_ROLE_CREATE` received from the gateway.
/// let previous = cache.update(&event);
/// assert!(previous.is_none());
///
/// let role = cache.roles(GuildId::new(1)).get(event.role.id).unwrap();
/// assert_eq!(role.name, "mods");
/// ```
pub struct Cache<B: Backend = DashMapBackend> {
    config: CacheConfig,
    backend: B,
    current_user: RwLock<Option<Arc<CurrentUser>>>,
    unavailable_guilds: DashMap<GuildId, (), Hasher>,
    guilds: Option<GlobalOf<GuildId, CachedGuild, B>>,
    channels: Option<GlobalOf<ChannelId, Channel, B>>,
    /// The channels of each guild, for listing and for dropping a deleted guild's channels.
    guild_channels: Index<GuildId, ChannelId>,
    roles: Option<ScopedOf<RoleId, Role, B>>,
    emojis: Option<ScopedOf<EmojiId, Emoji, B>>,
    stickers: Option<ScopedOf<StickerId, Sticker, B>>,
    members: Option<ScopedOf<UserId, CachedMember, B>>,
    users: Option<GlobalOf<UserId, User, B>>,
    threads: Option<ScopedOf<ChannelId, Channel, B>>,
    messages: Option<ChannelScopedOf<MessageId, CachedMessage, B>>,
    /// The channels of each guild that have cached messages, for dropping a deleted guild's
    /// messages even when channels are not cached.
    message_channels: Index<GuildId, ChannelId>,
    presences: Option<ScopedOf<UserId, CachedPresence, B>>,
    voice_states: Option<ScopedOf<UserId, VoiceState, B>>,
    scheduled_events: Option<ScopedOf<ScheduledEventId, GuildScheduledEvent, B>>,
    stage_instances: Option<ScopedOf<StageInstanceId, StageInstance, B>>,
    soundboard_sounds: Option<ScopedOf<SoundboardSoundId, SoundboardSound, B>>,
}

// The fields above match the scopes the resources declare (D-159).
const _: () = {
    assert!(matches!(Resource::Guild.scope(), Scope::Cache));
    assert!(matches!(Resource::Channel.scope(), Scope::Cache));
    assert!(matches!(Resource::User.scope(), Scope::Cache));
    assert!(matches!(Resource::Role.scope(), Scope::Guild));
    assert!(matches!(Resource::Emoji.scope(), Scope::Guild));
    assert!(matches!(Resource::Sticker.scope(), Scope::Guild));
    assert!(matches!(Resource::Member.scope(), Scope::Guild));
    assert!(matches!(Resource::Thread.scope(), Scope::Guild));
    assert!(matches!(Resource::Message.scope(), Scope::Channel));
    assert!(matches!(Resource::Presence.scope(), Scope::Guild));
    assert!(matches!(Resource::VoiceState.scope(), Scope::Guild));
    assert!(matches!(Resource::ScheduledEvent.scope(), Scope::Guild));
    assert!(matches!(Resource::StageInstance.scope(), Scope::Guild));
    assert!(matches!(Resource::SoundboardSound.scope(), Scope::Guild));
};

impl Cache {
    /// Creates a cache with the [default configuration](CacheConfig::default) and the
    /// [`DashMapBackend`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Creates a cache with `config` and the [`DashMapBackend`].
    #[must_use]
    pub fn with_config(config: CacheConfig) -> Self {
        Self::with_backend(config, DashMapBackend::new())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: Backend> Cache<B> {
    /// Creates a cache with `config`, keeping its resources in stores created by `backend`.
    ///
    /// [`Backend::store`] is called once for each enabled resource.
    #[must_use]
    pub fn with_backend(config: CacheConfig, backend: B) -> Self {
        Self {
            current_user: RwLock::new(None),
            unavailable_guilds: DashMap::with_hasher(Hasher::default()),
            guilds: global(&backend, &config, Resource::Guild),
            channels: global(&backend, &config, Resource::Channel),
            guild_channels: Index::new(),
            roles: scoped(&backend, &config, Resource::Role),
            emojis: scoped(&backend, &config, Resource::Emoji),
            stickers: scoped(&backend, &config, Resource::Sticker),
            members: scoped(&backend, &config, Resource::Member),
            users: global(&backend, &config, Resource::User),
            threads: scoped(&backend, &config, Resource::Thread),
            messages: channel_scoped(&backend, &config),
            message_channels: Index::new(),
            presences: scoped(&backend, &config, Resource::Presence),
            voice_states: scoped(&backend, &config, Resource::VoiceState),
            scheduled_events: scoped(&backend, &config, Resource::ScheduledEvent),
            stage_instances: scoped(&backend, &config, Resource::StageInstance),
            soundboard_sounds: scoped(&backend, &config, Resource::SoundboardSound),
            config,
            backend,
        }
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Returns the backend.
    #[must_use]
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Applies a dispatch event's payload to the cache, returning the previous state of what
    /// it changed (D-153, D-163). See [`UpdateCache`] for the events and what each returns.
    ///
    /// discord.js counterpart: the client's internal event handlers (`client.actions`).
    pub fn update<E: UpdateCache>(&self, event: &E) -> E::Previous {
        event.update_cache(self)
    }

    /// Applies any dispatch event to the cache, returning the previous state as a
    /// [`Previous`]. Events the cache does not use return [`Previous::None`].
    pub fn update_dispatch(&self, event: &DispatchEvent) -> Previous {
        crate::update::dispatch(self, event)
    }

    /// Returns the bot's own user, from `READY` and `USER_UPDATE`. Always kept.
    ///
    /// discord.js counterpart: `client.user`.
    #[must_use]
    pub fn current_user(&self) -> Option<Arc<CurrentUser>> {
        self.current_user
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Returns the guilds that are unavailable: listed in `READY` but not loaded yet, or
    /// unreachable because of a Discord outage. Their cached data, if any, is kept.
    ///
    /// discord.js counterpart: guilds with `Guild#available === false`.
    #[must_use]
    pub fn unavailable_guilds(&self) -> Vec<GuildId> {
        self.unavailable_guilds
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Returns the guilds ([`Resource::Guild`]).
    ///
    /// discord.js counterpart: `client.guilds.cache`.
    #[must_use]
    pub fn guilds(&self) -> CacheView<'_, GuildId, CachedGuild, B> {
        CacheView::new(self.guilds.as_ref(), None)
    }

    /// Returns the channels ([`Resource::Channel`]); [`CacheView::in_guild`] lists a guild's.
    ///
    /// discord.js counterpart: `client.channels.cache`.
    #[must_use]
    pub fn channels(&self) -> CacheView<'_, ChannelId, Channel, B> {
        CacheView::new(self.channels.as_ref(), Some(&self.guild_channels))
    }

    /// Returns the users ([`Resource::User`]).
    ///
    /// discord.js counterpart: `client.users.cache`.
    #[must_use]
    pub fn users(&self) -> CacheView<'_, UserId, User, B> {
        CacheView::new(self.users.as_ref(), None)
    }

    /// Returns a guild's roles ([`Resource::Role`]).
    ///
    /// discord.js counterpart: `guild.roles.cache`.
    #[must_use]
    pub fn roles(&self, guild_id: GuildId) -> GuildView<'_, RoleId, Role, B> {
        GuildView::new(self.roles.as_ref(), guild_id)
    }

    /// Returns a guild's custom emojis ([`Resource::Emoji`]).
    ///
    /// discord.js counterpart: `guild.emojis.cache`.
    #[must_use]
    pub fn emojis(&self, guild_id: GuildId) -> GuildView<'_, EmojiId, Emoji, B> {
        GuildView::new(self.emojis.as_ref(), guild_id)
    }

    /// Returns a guild's stickers ([`Resource::Sticker`]).
    ///
    /// discord.js counterpart: `guild.stickers.cache`.
    #[must_use]
    pub fn stickers(&self, guild_id: GuildId) -> GuildView<'_, StickerId, Sticker, B> {
        GuildView::new(self.stickers.as_ref(), guild_id)
    }

    /// Returns a guild's members ([`Resource::Member`]), by user ID.
    ///
    /// discord.js counterpart: `guild.members.cache`.
    #[must_use]
    pub fn members(&self, guild_id: GuildId) -> GuildView<'_, UserId, CachedMember, B> {
        GuildView::new(self.members.as_ref(), guild_id)
    }

    /// Returns a guild's threads ([`Resource::Thread`]).
    ///
    /// discord.js counterpart: `guild.channels.cache` (threads are channels there).
    #[must_use]
    pub fn threads(&self, guild_id: GuildId) -> GuildView<'_, ChannelId, Channel, B> {
        GuildView::new(self.threads.as_ref(), guild_id)
    }

    /// Returns a channel's (or thread's) messages ([`Resource::Message`]).
    ///
    /// discord.js counterpart: `channel.messages.cache`.
    #[must_use]
    pub fn messages(&self, channel_id: ChannelId) -> ChannelView<'_, MessageId, CachedMessage, B> {
        ChannelView::new(self.messages.as_ref(), channel_id)
    }

    /// Returns a guild's presences ([`Resource::Presence`]), by user ID.
    ///
    /// discord.js counterpart: `guild.presences.cache`.
    #[must_use]
    pub fn presences(&self, guild_id: GuildId) -> GuildView<'_, UserId, CachedPresence, B> {
        GuildView::new(self.presences.as_ref(), guild_id)
    }

    /// Returns a guild's voice states ([`Resource::VoiceState`]), by user ID. Their `member`
    /// is always `None`: members are a resource of their own.
    ///
    /// discord.js counterpart: `guild.voiceStates.cache`.
    #[must_use]
    pub fn voice_states(&self, guild_id: GuildId) -> GuildView<'_, UserId, VoiceState, B> {
        GuildView::new(self.voice_states.as_ref(), guild_id)
    }

    /// Returns a guild's scheduled events ([`Resource::ScheduledEvent`]).
    ///
    /// discord.js counterpart: `guild.scheduledEvents.cache`.
    #[must_use]
    pub fn scheduled_events(
        &self,
        guild_id: GuildId,
    ) -> GuildView<'_, ScheduledEventId, GuildScheduledEvent, B> {
        GuildView::new(self.scheduled_events.as_ref(), guild_id)
    }

    /// Returns a guild's stage instances ([`Resource::StageInstance`]).
    ///
    /// discord.js counterpart: `guild.stageInstances.cache`.
    #[must_use]
    pub fn stage_instances(
        &self,
        guild_id: GuildId,
    ) -> GuildView<'_, StageInstanceId, StageInstance, B> {
        GuildView::new(self.stage_instances.as_ref(), guild_id)
    }

    /// Returns a guild's soundboard sounds ([`Resource::SoundboardSound`]).
    ///
    /// discord.js counterpart: `guild.soundboardSounds.cache`.
    #[must_use]
    pub fn soundboard_sounds(
        &self,
        guild_id: GuildId,
    ) -> GuildView<'_, SoundboardSoundId, SoundboardSound, B> {
        GuildView::new(self.soundboard_sounds.as_ref(), guild_id)
    }
}

/// Sweeping (D-167, D-168): each method removes the entries its filter returns `true` for and
/// returns how many it removed. Resources kept per guild or channel pass the guild or channel
/// to the filter. Removing a guild, channel or thread this way also drops what belongs to it,
/// as deleting it does.
///
/// discord.js counterpart: `client.sweepers.sweepGuilds(filter)`, `sweepUsers(filter)`, ….
///
/// # Examples
///
/// ```
/// use djs_cache::{Cache, CacheConfig, Limit};
///
/// let cache = Cache::with_config(CacheConfig::builder().users(Limit::Unlimited).build());
/// // Forget every bot user.
/// let swept = cache.sweep_users(|user| user.bot);
/// assert_eq!(swept, 0);
/// ```
impl<B: Backend> Cache<B> {
    /// Sweeps guilds, with everything in them.
    pub fn sweep_guilds(&self, mut filter: impl FnMut(&CachedGuild) -> bool) -> usize {
        let Some(guilds) = &self.guilds else { return 0 };
        let mut swept = Vec::new();
        let count = guilds.sweep(&mut |id, guild| {
            let sweep = filter(guild);
            if sweep {
                swept.push(*id);
            }
            sweep
        });
        for id in swept {
            self.unavailable_guilds.remove(&id);
            self.remove_guild_children(id);
        }
        count
    }

    /// Sweeps channels, with their messages.
    pub fn sweep_channels(&self, mut filter: impl FnMut(&Channel) -> bool) -> usize {
        let Some(channels) = &self.channels else {
            return 0;
        };
        let mut swept = Vec::new();
        let count = channels.sweep(&mut |id, channel| {
            let sweep = filter(channel);
            if sweep {
                swept.push((*id, channel.guild_id()));
            }
            sweep
        });
        for (id, guild_id) in swept {
            if let Some(guild_id) = guild_id {
                self.guild_channels.remove(guild_id, id);
            }
            self.remove_messages_of(guild_id, id);
        }
        count
    }

    /// Sweeps users. Members keep their own reference to their user.
    pub fn sweep_users(&self, mut filter: impl FnMut(&User) -> bool) -> usize {
        self.users
            .as_ref()
            .map_or(0, |users| users.sweep(&mut |_, user| filter(user)))
    }

    /// Sweeps threads, with their messages.
    pub fn sweep_threads(&self, mut filter: impl FnMut(GuildId, &Channel) -> bool) -> usize {
        let Some(threads) = &self.threads else {
            return 0;
        };
        let mut swept = Vec::new();
        let count = threads.sweep(&mut |guild_id, thread| {
            let sweep = filter(guild_id, thread);
            if sweep {
                swept.push((guild_id, thread.id()));
            }
            sweep
        });
        for (guild_id, id) in swept {
            self.remove_messages_of(Some(guild_id), id);
        }
        count
    }

    /// Sweeps threads archived more than `lifetime` ago.
    ///
    /// discord.js counterpart: the default `threads` sweeper
    /// (`Sweepers.archivedThreadSweepFilter(lifetime)`).
    pub fn sweep_archived_threads(&self, lifetime: Duration) -> usize {
        let cutoff = cutoff(lifetime);
        self.sweep_threads(|_, thread| {
            thread.as_thread().is_some_and(|thread| {
                thread.thread_metadata.archived
                    && thread.thread_metadata.archive_timestamp.as_unix_micros() < cutoff
            })
        })
    }

    /// Sweeps messages.
    pub fn sweep_messages(
        &self,
        mut filter: impl FnMut(ChannelId, &CachedMessage) -> bool,
    ) -> usize {
        self.messages.as_ref().map_or(0, |messages| {
            messages.sweep(&mut |channel_id, message| filter(channel_id, message))
        })
    }

    /// Sweeps messages last edited (or, if never edited, sent) more than `lifetime` ago.
    ///
    /// discord.js counterpart: `Sweepers.filterByLifetime` for messages, which also compares
    /// the edit time first.
    pub fn sweep_messages_older_than(&self, lifetime: Duration) -> usize {
        let cutoff = cutoff(lifetime);
        self.sweep_messages(|_, message| {
            message
                .edited_timestamp
                .unwrap_or(message.timestamp)
                .as_unix_micros()
                < cutoff
        })
    }
}

/// Sweeps of the resources kept per guild.
macro_rules! sweep_per_guild {
    ($($(#[$doc:meta])* $method:ident: $field:ident => $value:ty;)*) => {
        impl<B: Backend> Cache<B> {$(
            $(#[$doc])*
            pub fn $method(&self, mut filter: impl FnMut(GuildId, &$value) -> bool) -> usize {
                self.$field.as_ref().map_or(0, |entries| {
                    entries.sweep(&mut |guild_id, value| filter(guild_id, value))
                })
            }
        )*}
    };
}

sweep_per_guild! {
    /// Sweeps roles.
    sweep_roles: roles => Role;
    /// Sweeps custom emojis.
    sweep_emojis: emojis => Emoji;
    /// Sweeps stickers.
    sweep_stickers: stickers => Sticker;
    /// Sweeps members.
    sweep_members: members => CachedMember;
    /// Sweeps presences.
    sweep_presences: presences => CachedPresence;
    /// Sweeps voice states.
    sweep_voice_states: voice_states => VoiceState;
    /// Sweeps scheduled events.
    sweep_scheduled_events: scheduled_events => GuildScheduledEvent;
    /// Sweeps stage instances.
    sweep_stage_instances: stage_instances => StageInstance;
    /// Sweeps soundboard sounds.
    sweep_soundboard_sounds: soundboard_sounds => SoundboardSound;
}

/// The Unix time, in microseconds, `lifetime` before now.
fn cutoff(lifetime: Duration) -> i64 {
    let lifetime = i64::try_from(lifetime.as_micros()).unwrap_or(i64::MAX);
    Timestamp::now().as_unix_micros().saturating_sub(lifetime)
}

/// The changes the event handlers in `update.rs` make.
impl<B: Backend> Cache<B> {
    pub(crate) fn set_current_user(&self, user: &CurrentUser) -> Option<Arc<CurrentUser>> {
        self.current_user
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(Arc::new(user.clone()))
    }

    pub(crate) fn mark_unavailable(&self, guild_id: GuildId) {
        self.unavailable_guilds.insert(guild_id, ());
    }

    /// `GUILD_CREATE` of an available guild.
    pub(crate) fn insert_guild(&self, guild: &AvailableGuild) -> Option<Arc<CachedGuild>> {
        let id = guild.guild.id;
        self.unavailable_guilds.remove(&id);
        for channel in &guild.channels {
            self.insert_channel(channel, Some(id));
        }
        replace(self.roles.as_ref(), id, &guild.guild.roles, |role| role.id);
        replace(self.emojis.as_ref(), id, &guild.guild.emojis, |emoji| {
            emoji.id
        });
        replace(
            self.stickers.as_ref(),
            id,
            &guild.guild.stickers,
            |sticker| sticker.id,
        );
        for member in &guild.members {
            self.upsert_member(id, member);
        }
        if let Some(threads) = &self.threads {
            for thread in &guild.threads {
                let mut thread = thread.clone();
                set_guild_id(&mut thread, id);
                threads.insert(id, thread.id(), Arc::new(thread));
            }
        }
        for presence in &guild.presences {
            self.update_presence(id, presence);
        }
        if let Some(voice_states) = &self.voice_states {
            for state in guild
                .voice_states
                .iter()
                .filter(|state| state.channel_id.is_some())
            {
                voice_states.insert(id, state.user_id, Arc::new(stripped_voice_state(state, id)));
            }
        }
        replace(
            self.stage_instances.as_ref(),
            id,
            &guild.stage_instances,
            |stage| stage.id,
        );
        replace(
            self.scheduled_events.as_ref(),
            id,
            &guild.guild_scheduled_events,
            |event| event.id,
        );
        replace(
            self.soundboard_sounds.as_ref(),
            id,
            &guild.soundboard_sounds,
            |sound| sound.sound_id,
        );
        self.put_guild(CachedGuild::from_available(guild))
    }

    /// `GUILD_UPDATE`. The guild's roles, emojis and stickers are replaced by the update's,
    /// unless a list is empty (Discord may omit them; their own events cover removals).
    pub(crate) fn update_guild(&self, guild: &Guild) -> Option<Arc<CachedGuild>> {
        if !guild.roles.is_empty() {
            replace(self.roles.as_ref(), guild.id, &guild.roles, |role| role.id);
        }
        if !guild.emojis.is_empty() {
            replace(self.emojis.as_ref(), guild.id, &guild.emojis, |emoji| {
                emoji.id
            });
        }
        if !guild.stickers.is_empty() {
            replace(
                self.stickers.as_ref(),
                guild.id,
                &guild.stickers,
                |sticker| sticker.id,
            );
        }
        let guilds = self.guilds.as_ref()?;
        let previous = guilds.get(&guild.id)?;
        let updated = CachedGuild::updated(guild, &previous);
        guilds.insert(guild.id, Arc::new(updated)).previous
    }

    /// `GUILD_DELETE`: the bot left the guild (`unavailable == false`), or the guild is in an
    /// outage, in which case its data stays.
    pub(crate) fn remove_guild(&self, id: GuildId, unavailable: bool) -> Option<Arc<CachedGuild>> {
        if unavailable {
            self.mark_unavailable(id);
            return None;
        }
        self.unavailable_guilds.remove(&id);
        self.remove_guild_children(id);
        self.guilds.as_ref()?.remove(&id)
    }

    fn put_guild(&self, guild: CachedGuild) -> Option<Arc<CachedGuild>> {
        let guilds = self.guilds.as_ref()?;
        let inserted = guilds.insert(guild.id, Arc::new(guild));
        if let Some((evicted, _)) = inserted.evicted {
            self.remove_guild_children(evicted);
        }
        inserted.previous
    }

    /// Drops everything cached for a guild (it was deleted, or evicted by the guild limit).
    fn remove_guild_children(&self, id: GuildId) {
        if let Some(channels) = &self.channels {
            for channel in self.guild_channels.remove_parent(id) {
                channels.remove(&channel);
            }
        }
        remove_all(self.roles.as_ref(), id);
        remove_all(self.emojis.as_ref(), id);
        remove_all(self.stickers.as_ref(), id);
        remove_all(self.members.as_ref(), id);
        remove_all(self.threads.as_ref(), id);
        remove_all(self.presences.as_ref(), id);
        remove_all(self.voice_states.as_ref(), id);
        remove_all(self.scheduled_events.as_ref(), id);
        remove_all(self.stage_instances.as_ref(), id);
        remove_all(self.soundboard_sounds.as_ref(), id);
        if let Some(messages) = &self.messages {
            for channel in self.message_channels.remove_parent(id) {
                messages.remove_parent(channel);
            }
        }
    }

    /// Adds a guild member to the guild's member count.
    pub(crate) fn count_member(&self, guild_id: GuildId, joined: bool) {
        let Some(guilds) = &self.guilds else { return };
        let Some(guild) = guilds.get(&guild_id) else {
            return;
        };
        let mut updated = CachedGuild::clone(&guild);
        updated.member_count = if joined {
            updated.member_count.saturating_add(1)
        } else {
            updated.member_count.saturating_sub(1)
        };
        guilds.insert(guild_id, Arc::new(updated));
    }

    /// `CHANNEL_CREATE` / `CHANNEL_UPDATE`, and the channels of a `GUILD_CREATE` (which lack
    /// their `guild_id`; the cached copy gets it). Threads are a resource of their own and are
    /// skipped here.
    pub(crate) fn insert_channel(
        &self,
        channel: &Channel,
        guild_id: Option<GuildId>,
    ) -> Option<Arc<Channel>> {
        if channel.as_thread().is_some() {
            return None;
        }
        let channels = self.channels.as_ref()?;
        let mut channel = channel.clone();
        if let Some(guild_id) = guild_id.or_else(|| channel.guild_id()) {
            set_guild_id(&mut channel, guild_id);
            self.guild_channels.insert(guild_id, channel.id());
        }
        let inserted = channels.insert(channel.id(), Arc::new(channel));
        if let Some((id, evicted)) = &inserted.evicted
            && let Some(guild_id) = evicted.guild_id()
        {
            self.guild_channels.remove(guild_id, *id);
        }
        inserted.previous
    }

    /// `CHANNEL_DELETE`. The channel's messages go too.
    pub(crate) fn remove_channel(&self, channel: &Channel) -> Option<Arc<Channel>> {
        self.remove_messages_of(channel.guild_id(), channel.id());
        let removed = self.channels.as_ref()?.remove(&channel.id());
        let guild_id = channel
            .guild_id()
            .or_else(|| removed.as_ref().and_then(|removed| removed.guild_id()));
        if let Some(guild_id) = guild_id {
            self.guild_channels.remove(guild_id, channel.id());
        }
        removed
    }

    /// `CHANNEL_PINS_UPDATE`.
    pub(crate) fn update_pins(
        &self,
        channel_id: ChannelId,
        last_pin_timestamp: Option<Timestamp>,
    ) -> Option<Arc<Channel>> {
        let channels = self.channels.as_ref()?;
        let mut channel = Channel::clone(&*channels.get(&channel_id)?);
        if !set_last_pin_timestamp(&mut channel, last_pin_timestamp) {
            return None;
        }
        channels.insert(channel_id, Arc::new(channel)).previous
    }

    pub(crate) fn insert_role(&self, guild_id: GuildId, role: &Role) -> Option<Arc<Role>> {
        self.roles
            .as_ref()?
            .insert(guild_id, role.id, Arc::new(role.clone()))
            .previous
    }

    pub(crate) fn remove_role(&self, guild_id: GuildId, role_id: RoleId) -> Option<Arc<Role>> {
        self.roles.as_ref()?.remove(guild_id, role_id)
    }

    pub(crate) fn replace_emojis(&self, guild_id: GuildId, emojis: &[Emoji]) -> Vec<Arc<Emoji>> {
        replace(self.emojis.as_ref(), guild_id, emojis, |emoji| emoji.id)
    }

    pub(crate) fn replace_stickers(
        &self,
        guild_id: GuildId,
        stickers: &[Sticker],
    ) -> Vec<Arc<Sticker>> {
        replace(self.stickers.as_ref(), guild_id, stickers, |sticker| {
            sticker.id
        })
    }

    /// `GUILD_MEMBER_ADD`, the members of `GUILD_CREATE` and `GUILD_MEMBERS_CHUNK`.
    pub(crate) fn upsert_member(
        &self,
        guild_id: GuildId,
        member: &Member,
    ) -> Option<Arc<CachedMember>> {
        let Some(members) = &self.members else {
            self.cache_user(&member.user);
            return None;
        };
        let user = self.share_user(&member.user);
        let id = user.id;
        members
            .insert(
                guild_id,
                id,
                Arc::new(CachedMember::new(guild_id, member, user)),
            )
            .previous
    }

    /// `GUILD_MEMBER_UPDATE`. A member that was not cached is added, like discord.js does.
    pub(crate) fn update_member(&self, update: &GuildMemberUpdate) -> Option<Arc<CachedMember>> {
        let Some(members) = &self.members else {
            self.cache_user(&update.user);
            return None;
        };
        let user = self.share_user(&update.user);
        let id = user.id;
        let previous = members.get(update.guild_id, id);
        let updated = CachedMember::updated(update, user, previous.as_deref());
        members
            .insert(update.guild_id, id, Arc::new(updated))
            .previous
    }

    /// `GUILD_MEMBER_REMOVE`. The user stays cached, like in discord.js.
    pub(crate) fn remove_member(
        &self,
        guild_id: GuildId,
        user: &User,
    ) -> Option<Arc<CachedMember>> {
        self.cache_user(user);
        self.members.as_ref()?.remove(guild_id, user.id)
    }

    /// `THREAD_CREATE` / `THREAD_UPDATE`.
    pub(crate) fn insert_thread(&self, thread: &Channel) -> Option<Arc<Channel>> {
        let threads = self.threads.as_ref()?;
        let guild_id = thread.guild_id()?;
        threads
            .insert(guild_id, thread.id(), Arc::new(thread.clone()))
            .previous
    }

    /// `THREAD_DELETE`. The thread's messages go too.
    pub(crate) fn remove_thread(&self, guild_id: GuildId, id: ChannelId) -> Option<Arc<Channel>> {
        self.remove_messages_of(Some(guild_id), id);
        self.threads.as_ref()?.remove(guild_id, id)
    }

    /// `THREAD_LIST_SYNC`: the active threads of the synced parent channels (all of the
    /// guild's when `parents` is `None`) are replaced. Returns the threads it replaced.
    pub(crate) fn sync_threads(
        &self,
        guild_id: GuildId,
        parents: Option<&[ChannelId]>,
        synced: &[Channel],
    ) -> Vec<Arc<Channel>> {
        let Some(threads) = &self.threads else {
            return Vec::new();
        };
        let mut previous = Vec::new();
        for thread in threads.values_of(guild_id) {
            let in_sync = parents.is_none_or(|parents| {
                thread
                    .parent_id()
                    .is_some_and(|parent| parents.contains(&parent))
            });
            let active = thread
                .as_thread()
                .is_some_and(|thread| !thread.thread_metadata.archived);
            if in_sync && active {
                previous.extend(threads.remove(guild_id, thread.id()));
            }
        }
        for thread in synced {
            threads.insert(guild_id, thread.id(), Arc::new(thread.clone()));
        }
        previous
    }

    /// `MESSAGE_CREATE`. The author is cached as a user (unless it is a webhook) and, with
    /// the message's member data, as a member.
    pub(crate) fn insert_message(&self, message: &Message) -> Option<Arc<CachedMessage>> {
        let author = if message.webhook_id.is_some() {
            Arc::new(message.author.clone())
        } else if self.messages.is_some() || self.members.is_some() {
            self.share_user(&message.author)
        } else {
            self.cache_user(&message.author);
            return None;
        };
        if let (Some(guild_id), Some(member), Some(members)) =
            (message.guild_id, &message.member, &self.members)
        {
            put_partial_member(members, guild_id, member, Arc::clone(&author));
        }
        let messages = self.messages.as_ref()?;
        if let Some(guild_id) = message.guild_id {
            self.message_channels.insert(guild_id, message.channel_id);
        }
        let cached = CachedMessage::new(message, author);
        messages
            .insert(message.channel_id, message.id, Arc::new(cached))
            .previous
    }

    /// `MESSAGE_UPDATE` (the full new message). Only a cached message is updated, like in
    /// discord.js; the author and member are cached either way.
    pub(crate) fn update_message(&self, message: &Message) -> Option<Arc<CachedMessage>> {
        let cached = self
            .messages
            .as_ref()
            .is_some_and(|messages| messages.get(message.channel_id, message.id).is_some());
        if cached {
            self.insert_message(message)
        } else {
            if message.webhook_id.is_none() {
                self.cache_user(&message.author);
            }
            None
        }
    }

    /// `MESSAGE_DELETE`.
    pub(crate) fn remove_message(
        &self,
        channel_id: ChannelId,
        id: MessageId,
    ) -> Option<Arc<CachedMessage>> {
        self.messages.as_ref()?.remove(channel_id, id)
    }

    fn remove_messages_of(&self, guild_id: Option<GuildId>, channel_id: ChannelId) {
        if let Some(messages) = &self.messages {
            messages.remove_parent(channel_id);
            if let Some(guild_id) = guild_id {
                self.message_channels.remove(guild_id, channel_id);
            }
        }
    }

    /// The reaction events (D-166): changes a cached message's reactions, returning the
    /// message before.
    pub(crate) fn react(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        change: ReactionChange<'_>,
    ) -> Option<Arc<CachedMessage>> {
        let messages = self.messages.as_ref()?;
        let mut message = CachedMessage::clone(&*messages.get(channel_id, message_id)?);
        let me = self.current_user().map(|user| user.id);
        change.apply(&mut message.reactions, me);
        messages
            .insert(channel_id, message_id, Arc::new(message))
            .previous
    }

    /// `PRESENCE_UPDATE` and the presences of `GUILD_CREATE` / `GUILD_MEMBERS_CHUNK`. An
    /// offline member's presence is removed.
    pub(crate) fn update_presence(
        &self,
        guild_id: GuildId,
        presence: &Presence,
    ) -> Option<Arc<CachedPresence>> {
        let presences = self.presences.as_ref()?;
        if presence.status == Status::Offline {
            return presences.remove(guild_id, presence.user.id);
        }
        presences
            .insert(
                guild_id,
                presence.user.id,
                Arc::new(CachedPresence::new(guild_id, presence)),
            )
            .previous
    }

    /// `VOICE_STATE_UPDATE`: a state without a channel means the user left voice. The
    /// state's member is cached as a member.
    pub(crate) fn update_voice_state(&self, state: &VoiceState) -> Option<Arc<VoiceState>> {
        let guild_id = state.guild_id?;
        if let Some(member) = &state.member {
            self.upsert_member(guild_id, member);
        }
        let voice_states = self.voice_states.as_ref()?;
        if state.channel_id.is_none() {
            return voice_states.remove(guild_id, state.user_id);
        }
        voice_states
            .insert(
                guild_id,
                state.user_id,
                Arc::new(stripped_voice_state(state, guild_id)),
            )
            .previous
    }

    pub(crate) fn put_scheduled_event(
        &self,
        event: &GuildScheduledEvent,
    ) -> Option<Arc<GuildScheduledEvent>> {
        put(
            self.scheduled_events.as_ref(),
            event.guild_id,
            event.id,
            event,
        )
    }

    pub(crate) fn remove_scheduled_event(
        &self,
        event: &GuildScheduledEvent,
    ) -> Option<Arc<GuildScheduledEvent>> {
        self.scheduled_events
            .as_ref()?
            .remove(event.guild_id, event.id)
    }

    pub(crate) fn put_stage_instance(&self, stage: &StageInstance) -> Option<Arc<StageInstance>> {
        put(
            self.stage_instances.as_ref(),
            stage.guild_id,
            stage.id,
            stage,
        )
    }

    pub(crate) fn remove_stage_instance(
        &self,
        stage: &StageInstance,
    ) -> Option<Arc<StageInstance>> {
        self.stage_instances
            .as_ref()?
            .remove(stage.guild_id, stage.id)
    }

    pub(crate) fn put_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound: &SoundboardSound,
    ) -> Option<Arc<SoundboardSound>> {
        put(
            self.soundboard_sounds.as_ref(),
            guild_id,
            sound.sound_id,
            sound,
        )
    }

    pub(crate) fn remove_soundboard_sound(
        &self,
        guild_id: GuildId,
        id: SoundboardSoundId,
    ) -> Option<Arc<SoundboardSound>> {
        self.soundboard_sounds.as_ref()?.remove(guild_id, id)
    }

    pub(crate) fn replace_soundboard_sounds(
        &self,
        guild_id: GuildId,
        sounds: &[SoundboardSound],
    ) -> Vec<Arc<SoundboardSound>> {
        replace(self.soundboard_sounds.as_ref(), guild_id, sounds, |sound| {
            sound.sound_id
        })
    }

    /// Caches a user seen in an event, if users are cached.
    fn cache_user(&self, user: &User) {
        if self.users.is_some() {
            self.share_user(user);
        }
    }

    /// Returns an `Arc` of `user`: the user resource's own when it holds an equal user (so a
    /// user in many guilds is stored once), else a new one, which the user resource then
    /// keeps.
    fn share_user(&self, user: &User) -> Arc<User> {
        let Some(users) = &self.users else {
            return Arc::new(user.clone());
        };
        if let Some(cached) = users.get(&user.id)
            && *cached == *user
        {
            return cached;
        }
        let shared = Arc::new(user.clone());
        users.insert(user.id, Arc::clone(&shared));
        shared
    }
}

impl<B: Backend> fmt::Debug for Cache<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cache")
            .field("config", &self.config)
            .field("backend", &self.backend)
            .field("guilds", &self.guilds().len())
            .field("channels", &self.channels().len())
            .field("users", &self.users().len())
            .field("members", &self.members.as_ref().map_or(0, Scoped::len))
            .field("messages", &self.messages.as_ref().map_or(0, Scoped::len))
            .finish_non_exhaustive()
    }
}

/// Creates the store of a resource kept over the whole cache, if it is enabled.
fn global<K, V, B>(
    backend: &B,
    config: &CacheConfig,
    resource: Resource,
) -> Option<GlobalOf<K, V, B>>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    let limit = config.limit(resource);
    limit
        .is_enabled()
        .then(|| Global::new(backend.store(resource, limit), limit))
}

/// Creates the message store (kept per channel), if messages are enabled.
fn channel_scoped<B: Backend>(
    backend: &B,
    config: &CacheConfig,
) -> Option<ChannelScopedOf<MessageId, CachedMessage, B>> {
    let limit = config.limit(Resource::Message);
    limit
        .is_enabled()
        .then(|| Scoped::new(backend.store(Resource::Message, limit), limit))
}

/// Inserts a copy of `value` into a guild resource, returning the previous one.
fn put<K, V, S>(
    entries: Option<&Scoped<GuildId, K, V, S>>,
    guild_id: GuildId,
    id: K,
    value: &V,
) -> Option<Arc<V>>
where
    K: Eq + Hash + Copy,
    V: Clone,
    S: Store<(GuildId, K), V>,
{
    entries?
        .insert(guild_id, id, Arc::new(value.clone()))
        .previous
}

/// Caches the partial member attached to a message, with the message's author as its user.
fn put_partial_member<S>(
    members: &Scoped<GuildId, UserId, CachedMember, S>,
    guild_id: GuildId,
    member: &PartialMember,
    user: Arc<User>,
) where
    S: Store<(GuildId, UserId), CachedMember>,
{
    let id = user.id;
    let cached = CachedMember::from_partial(guild_id, member, user);
    members.insert(guild_id, id, Arc::new(cached));
}

/// A voice state as cached: with its guild, without its member (members are a resource of
/// their own).
fn stripped_voice_state(state: &VoiceState, guild_id: GuildId) -> VoiceState {
    let mut state = state.clone();
    state.guild_id = Some(guild_id);
    state.member = None;
    state
}

/// A change to a message's reactions (D-166).
pub(crate) enum ReactionChange<'a> {
    /// `MESSAGE_REACTION_ADD`.
    Add {
        emoji: &'a PartialEmoji,
        user_id: UserId,
        burst: bool,
        burst_colors: &'a [compact_str::CompactString],
    },
    /// `MESSAGE_REACTION_REMOVE`.
    Remove {
        emoji: &'a PartialEmoji,
        user_id: UserId,
        burst: bool,
    },
    /// `MESSAGE_REACTION_REMOVE_ALL`.
    RemoveAll,
    /// `MESSAGE_REACTION_REMOVE_EMOJI`.
    RemoveEmoji(&'a PartialEmoji),
}

impl ReactionChange<'_> {
    fn apply(self, reactions: &mut Vec<Reaction>, me: Option<UserId>) {
        match self {
            Self::Add {
                emoji,
                user_id,
                burst,
                burst_colors,
            } => {
                let index = if let Some(index) = reactions
                    .iter()
                    .position(|reaction| same_emoji(&reaction.emoji, emoji))
                {
                    index
                } else {
                    reactions.push(Reaction::new(emoji.clone()));
                    reactions.len() - 1
                };
                let reaction = &mut reactions[index];
                let mine = me == Some(user_id);
                reaction.count += 1;
                if burst {
                    reaction.count_details.burst += 1;
                    reaction.me_burst |= mine;
                    if reaction.burst_colors.is_empty() {
                        reaction.burst_colors = burst_colors.to_vec();
                    }
                } else {
                    reaction.count_details.normal += 1;
                    reaction.me |= mine;
                }
            }
            Self::Remove {
                emoji,
                user_id,
                burst,
            } => {
                let Some(index) = reactions
                    .iter()
                    .position(|reaction| same_emoji(&reaction.emoji, emoji))
                else {
                    return;
                };
                let reaction = &mut reactions[index];
                let mine = me == Some(user_id);
                reaction.count = reaction.count.saturating_sub(1);
                if burst {
                    reaction.count_details.burst = reaction.count_details.burst.saturating_sub(1);
                    reaction.me_burst &= !mine;
                } else {
                    reaction.count_details.normal = reaction.count_details.normal.saturating_sub(1);
                    reaction.me &= !mine;
                }
                if reaction.count == 0 {
                    reactions.remove(index);
                }
            }
            Self::RemoveAll => reactions.clear(),
            Self::RemoveEmoji(emoji) => {
                reactions.retain(|reaction| !same_emoji(&reaction.emoji, emoji));
            }
        }
    }
}

/// Whether two reaction emojis are the same: custom emojis by ID (their names may be missing),
/// Unicode emojis by the emoji itself.
fn same_emoji(a: &PartialEmoji, b: &PartialEmoji) -> bool {
    match (a.id(), b.id()) {
        (Some(a), Some(b)) => a == b,
        (None, None) => a == b,
        _ => false,
    }
}

/// Creates the store of a resource kept per guild, if it is enabled.
fn scoped<K, V, B>(
    backend: &B,
    config: &CacheConfig,
    resource: Resource,
) -> Option<ScopedOf<K, V, B>>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: Backend,
{
    let limit = config.limit(resource);
    limit
        .is_enabled()
        .then(|| Scoped::new(backend.store(resource, limit), limit))
}

/// Replaces a guild's entries of a resource (the roles of a `GUILD_CREATE`, the emojis of a
/// `GUILD_EMOJIS_UPDATE`, …), returning the previous ones.
fn replace<K, V, S>(
    entries: Option<&Scoped<GuildId, K, V, S>>,
    guild_id: GuildId,
    items: &[V],
    id: impl Fn(&V) -> K,
) -> Vec<Arc<V>>
where
    K: Eq + Hash + Copy,
    V: Clone,
    S: Store<(GuildId, K), V>,
{
    let Some(entries) = entries else {
        return Vec::new();
    };
    let previous = entries.remove_parent(guild_id);
    for item in items {
        entries.insert(guild_id, id(item), Arc::new(item.clone()));
    }
    previous
}

fn remove_all<K, V, S>(entries: Option<&Scoped<GuildId, K, V, S>>, guild_id: GuildId)
where
    K: Eq + Hash + Copy,
    S: Store<(GuildId, K), V>,
{
    if let Some(entries) = entries {
        entries.remove_parent(guild_id);
    }
}

/// Sets the guild of a guild channel (Discord omits it in `GUILD_CREATE`).
const fn set_guild_id(channel: &mut Channel, guild_id: GuildId) {
    let slot = match channel {
        Channel::Text(channel) | Channel::Announcement(channel) => &mut channel.guild_id,
        Channel::Voice(channel) | Channel::Stage(channel) => &mut channel.guild_id,
        Channel::Category(channel) => &mut channel.guild_id,
        Channel::AnnouncementThread(channel)
        | Channel::PublicThread(channel)
        | Channel::PrivateThread(channel) => &mut channel.guild_id,
        Channel::Directory(channel) => &mut channel.guild_id,
        Channel::Forum(channel) | Channel::Media(channel) => &mut channel.guild_id,
        Channel::Unknown(channel) => &mut channel.guild_id,
        _ => return,
    };
    *slot = Some(guild_id);
}

/// Sets the last pin time of a channel that has one; returns whether it has.
const fn set_last_pin_timestamp(channel: &mut Channel, timestamp: Option<Timestamp>) -> bool {
    let slot = match channel {
        Channel::Text(channel) | Channel::Announcement(channel) => &mut channel.last_pin_timestamp,
        Channel::Dm(channel) => &mut channel.last_pin_timestamp,
        Channel::GroupDm(channel) => &mut channel.last_pin_timestamp,
        Channel::AnnouncementThread(channel)
        | Channel::PublicThread(channel)
        | Channel::PrivateThread(channel) => &mut channel.last_pin_timestamp,
        _ => return false,
    };
    *slot = timestamp;
    true
}

/// The cache is shared between shards.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Cache>();
};
