//! Applying gateway events to the cache (D-153, D-161, D-163).

use std::sync::Arc;

use djs_model::{
    channel::Channel,
    emoji::Emoji,
    gateway::{
        DispatchEvent,
        payload::{
            ChannelCreate, ChannelDelete, ChannelPinsUpdate, ChannelUpdate, GuildCreate,
            GuildDelete, GuildEmojisUpdate, GuildMemberAdd, GuildMemberRemove, GuildMemberUpdate,
            GuildMembersChunk, GuildRoleCreate, GuildRoleDelete, GuildRoleUpdate,
            GuildScheduledEventCreate, GuildScheduledEventDelete, GuildScheduledEventUpdate,
            GuildSoundboardSoundCreate, GuildSoundboardSoundDelete, GuildSoundboardSoundUpdate,
            GuildSoundboardSoundsUpdate, GuildStickersUpdate, GuildUpdate, MessageCreate,
            MessageDelete, MessageDeleteBulk, MessageReactionAdd, MessageReactionRemove,
            MessageReactionRemoveAll, MessageReactionRemoveEmoji, MessageUpdate, PresenceUpdate,
            Ready, SoundboardSounds, StageInstanceCreate, StageInstanceDelete, StageInstanceUpdate,
            ThreadCreate, ThreadDelete, ThreadListSync, ThreadUpdate, UserUpdate, VoiceStateUpdate,
        },
    },
    guild::Role,
    guild_scheduled_event::GuildScheduledEvent,
    soundboard::SoundboardSound,
    stage_instance::StageInstance,
    sticker::Sticker,
    user::CurrentUser,
    voice::VoiceState,
};

use crate::{
    Backend, Cache, CachedGuild, CachedMember, CachedMessage, CachedPresence, cache::ReactionChange,
};

mod sealed {
    pub trait Sealed {}
}

/// A gateway event payload the cache can apply, with the type of the previous state it
/// returns (D-163).
///
/// Use it through [`Cache::update`]; for a whole [`DispatchEvent`] use
/// [`Cache::update_dispatch`]. Every update borrows the event and copies what the cache keeps
/// (D-161). The previous state is what the event replaced or removed, or nothing when the
/// resource is disabled or the entry was not cached; for a guild it is the guild itself, not
/// its roles, channels or members.
///
/// | Payload | What it does | `Previous` |
/// |---|---|---|
/// | [`Ready`] | sets the current user; lists the guilds as unavailable | `Option<Arc<CurrentUser>>` |
/// | [`UserUpdate`] | replaces the current user | `Option<Arc<CurrentUser>>` |
/// | [`GuildCreate`] | caches the guild with its channels, roles, emojis, stickers and members | `Option<Arc<CachedGuild>>` |
/// | [`GuildUpdate`] | updates the guild (and its roles, emojis, stickers if sent) | `Option<Arc<CachedGuild>>` |
/// | [`GuildDelete`] | removes the guild and everything in it, or marks it unavailable during an outage | `Option<Arc<CachedGuild>>` |
/// | [`ChannelCreate`], [`ChannelUpdate`] | caches the channel (threads are skipped) | `Option<Arc<Channel>>` |
/// | [`ChannelDelete`] | removes the channel | `Option<Arc<Channel>>` |
/// | [`ChannelPinsUpdate`] | sets the channel's last pin time | `Option<Arc<Channel>>` |
/// | [`GuildRoleCreate`], [`GuildRoleUpdate`] | caches the role | `Option<Arc<Role>>` |
/// | [`GuildRoleDelete`] | removes the role | `Option<Arc<Role>>` |
/// | [`GuildEmojisUpdate`] | replaces the guild's emojis | `Vec<Arc<Emoji>>` |
/// | [`GuildStickersUpdate`] | replaces the guild's stickers | `Vec<Arc<Sticker>>` |
/// | [`GuildMemberAdd`] | caches the member, counts it in the guild | `Option<Arc<CachedMember>>` |
/// | [`GuildMemberUpdate`] | updates (or adds) the member | `Option<Arc<CachedMember>>` |
/// | [`GuildMemberRemove`] | removes the member, uncounts it | `Option<Arc<CachedMember>>` |
/// | [`GuildMembersChunk`] | caches the members (and their presences) | `Vec<Arc<CachedMember>>` (those replaced) |
/// | [`MessageCreate`] | caches the message (author as a user, its member as a member) | `Option<Arc<CachedMessage>>` |
/// | [`MessageUpdate`] | replaces a cached message | `Option<Arc<CachedMessage>>` |
/// | [`MessageDelete`] | removes the message | `Option<Arc<CachedMessage>>` |
/// | [`MessageDeleteBulk`] | removes the messages | `Vec<Arc<CachedMessage>>` |
/// | [`MessageReactionAdd`], [`MessageReactionRemove`], [`MessageReactionRemoveAll`], [`MessageReactionRemoveEmoji`] | updates a cached message's reactions | `Option<Arc<CachedMessage>>` |
/// | [`PresenceUpdate`] | caches the presence, or removes it when offline | `Option<Arc<CachedPresence>>` |
/// | [`VoiceStateUpdate`] | caches the voice state, or removes it when the user left voice | `Option<Arc<VoiceState>>` |
/// | [`ThreadCreate`], [`ThreadUpdate`] | caches the thread | `Option<Arc<Channel>>` |
/// | [`ThreadDelete`] | removes the thread and its messages | `Option<Arc<Channel>>` |
/// | [`ThreadListSync`] | replaces the synced channels' active threads | `Vec<Arc<Channel>>` |
/// | [`GuildScheduledEventCreate`], [`GuildScheduledEventUpdate`], [`GuildScheduledEventDelete`] | caches / removes the event | `Option<Arc<GuildScheduledEvent>>` |
/// | [`StageInstanceCreate`], [`StageInstanceUpdate`], [`StageInstanceDelete`] | caches / removes the stage instance | `Option<Arc<StageInstance>>` |
/// | [`GuildSoundboardSoundCreate`], [`GuildSoundboardSoundUpdate`], [`GuildSoundboardSoundDelete`] | caches / removes the sound | `Option<Arc<SoundboardSound>>` |
/// | [`GuildSoundboardSoundsUpdate`] | caches the sounds | `Vec<Arc<SoundboardSound>>` (those replaced) |
/// | [`SoundboardSounds`] | replaces the guild's sounds | `Vec<Arc<SoundboardSound>>` |
///
/// Users seen in member events are cached too when [`Resource::User`](crate::Resource::User)
/// is enabled.
///
/// This trait is sealed: the cache implements it for the payloads it understands.
///
/// discord.js counterpart: the client's internal actions (`client.actions.GuildMemberUpdate`,
/// …), which also return the old and new state.
pub trait UpdateCache: sealed::Sealed {
    /// The previous state this event returns.
    type Previous;

    /// Applies the event. Called by [`Cache::update`].
    #[doc(hidden)]
    fn update_cache<B: Backend>(&self, cache: &Cache<B>) -> Self::Previous;
}

/// The previous state returned by [`Cache::update_dispatch`]: the
/// [`UpdateCache::Previous`] of the event, in a variant per kind.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Previous {
    /// The event changes nothing in the cache, or nothing was cached before it.
    None,
    /// The current user before a [`Ready`] or [`UserUpdate`].
    CurrentUser(Arc<CurrentUser>),
    /// A guild before a [`GuildCreate`], [`GuildUpdate`] or [`GuildDelete`].
    Guild(Arc<CachedGuild>),
    /// A channel before a channel event.
    Channel(Arc<Channel>),
    /// A role before a role event.
    Role(Arc<Role>),
    /// A guild's emojis before a [`GuildEmojisUpdate`].
    Emojis(Vec<Arc<Emoji>>),
    /// A guild's stickers before a [`GuildStickersUpdate`].
    Stickers(Vec<Arc<Sticker>>),
    /// A member before a member event.
    Member(Arc<CachedMember>),
    /// The members a [`GuildMembersChunk`] replaced.
    Members(Vec<Arc<CachedMember>>),
    /// A message before a message or reaction event.
    Message(Arc<CachedMessage>),
    /// The messages a [`MessageDeleteBulk`] removed.
    Messages(Vec<Arc<CachedMessage>>),
    /// A presence before a [`PresenceUpdate`].
    Presence(Arc<CachedPresence>),
    /// A voice state before a [`VoiceStateUpdate`].
    VoiceState(Arc<VoiceState>),
    /// A thread before a thread event.
    Thread(Arc<Channel>),
    /// The threads a [`ThreadListSync`] replaced.
    Threads(Vec<Arc<Channel>>),
    /// A scheduled event before a scheduled event event.
    ScheduledEvent(Arc<GuildScheduledEvent>),
    /// A stage instance before a stage instance event.
    StageInstance(Arc<StageInstance>),
    /// A soundboard sound before a soundboard sound event.
    SoundboardSound(Arc<SoundboardSound>),
    /// The soundboard sounds a [`GuildSoundboardSoundsUpdate`] or [`SoundboardSounds`]
    /// replaced.
    SoundboardSounds(Vec<Arc<SoundboardSound>>),
}

/// Implements [`UpdateCache`] for a payload: `Payload => Previous, |event, cache| body`.
macro_rules! update_cache {
    ($($payload:ty => $previous:ty, |$event:ident, $cache:ident| $body:expr;)*) => {$(
        impl sealed::Sealed for $payload {}

        impl UpdateCache for $payload {
            type Previous = $previous;

            fn update_cache<B: Backend>(&self, $cache: &Cache<B>) -> $previous {
                let $event = self;
                $body
            }
        }
    )*};
}

update_cache! {
    Ready => Option<Arc<CurrentUser>>, |event, cache| {
        for guild in &event.guilds {
            cache.mark_unavailable(guild.id);
        }
        cache.set_current_user(&event.user)
    };
    UserUpdate => Option<Arc<CurrentUser>>, |event, cache| cache.set_current_user(event);
    GuildCreate => Option<Arc<CachedGuild>>, |event, cache| match event {
        GuildCreate::Available(guild) => cache.insert_guild(guild),
        GuildCreate::Unavailable(guild) => {
            cache.mark_unavailable(guild.id);
            None
        }
        _ => None,
    };
    GuildUpdate => Option<Arc<CachedGuild>>, |event, cache| cache.update_guild(event);
    GuildDelete => Option<Arc<CachedGuild>>, |event, cache| {
        cache.remove_guild(event.id, event.unavailable)
    };
    ChannelCreate => Option<Arc<Channel>>, |event, cache| cache.insert_channel(event, None);
    ChannelUpdate => Option<Arc<Channel>>, |event, cache| cache.insert_channel(event, None);
    ChannelDelete => Option<Arc<Channel>>, |event, cache| cache.remove_channel(event);
    ChannelPinsUpdate => Option<Arc<Channel>>, |event, cache| {
        cache.update_pins(event.channel_id, event.last_pin_timestamp)
    };
    GuildRoleCreate => Option<Arc<Role>>, |event, cache| {
        cache.insert_role(event.guild_id, &event.role)
    };
    GuildRoleUpdate => Option<Arc<Role>>, |event, cache| {
        cache.insert_role(event.guild_id, &event.role)
    };
    GuildRoleDelete => Option<Arc<Role>>, |event, cache| {
        cache.remove_role(event.guild_id, event.role_id)
    };
    GuildEmojisUpdate => Vec<Arc<Emoji>>, |event, cache| {
        cache.replace_emojis(event.guild_id, &event.emojis)
    };
    GuildStickersUpdate => Vec<Arc<Sticker>>, |event, cache| {
        cache.replace_stickers(event.guild_id, &event.stickers)
    };
    GuildMemberAdd => Option<Arc<CachedMember>>, |event, cache| {
        let previous = cache.upsert_member(event.guild_id, event);
        if previous.is_none() {
            cache.count_member(event.guild_id, true);
        }
        previous
    };
    GuildMemberUpdate => Option<Arc<CachedMember>>, |event, cache| cache.update_member(event);
    GuildMemberRemove => Option<Arc<CachedMember>>, |event, cache| {
        cache.count_member(event.guild_id, false);
        cache.remove_member(event.guild_id, &event.user)
    };
    GuildMembersChunk => Vec<Arc<CachedMember>>, |event, cache| {
        for presence in &event.presences {
            cache.update_presence(event.guild_id, presence);
        }
        event
            .members
            .iter()
            .filter_map(|member| cache.upsert_member(event.guild_id, member))
            .collect()
    };
    MessageCreate => Option<Arc<CachedMessage>>, |event, cache| cache.insert_message(event);
    MessageUpdate => Option<Arc<CachedMessage>>, |event, cache| cache.update_message(event);
    MessageDelete => Option<Arc<CachedMessage>>, |event, cache| {
        cache.remove_message(event.channel_id, event.id)
    };
    MessageDeleteBulk => Vec<Arc<CachedMessage>>, |event, cache| {
        event
            .ids
            .iter()
            .filter_map(|&id| cache.remove_message(event.channel_id, id))
            .collect()
    };
    MessageReactionAdd => Option<Arc<CachedMessage>>, |event, cache| {
        if let (Some(guild_id), Some(member)) = (event.guild_id, &event.member) {
            cache.upsert_member(guild_id, member);
        }
        cache.react(event.channel_id, event.message_id, ReactionChange::Add {
            emoji: &event.emoji,
            user_id: event.user_id,
            burst: event.burst,
            burst_colors: &event.burst_colors,
        })
    };
    MessageReactionRemove => Option<Arc<CachedMessage>>, |event, cache| {
        cache.react(event.channel_id, event.message_id, ReactionChange::Remove {
            emoji: &event.emoji,
            user_id: event.user_id,
            burst: event.burst,
        })
    };
    MessageReactionRemoveAll => Option<Arc<CachedMessage>>, |event, cache| {
        cache.react(event.channel_id, event.message_id, ReactionChange::RemoveAll)
    };
    MessageReactionRemoveEmoji => Option<Arc<CachedMessage>>, |event, cache| {
        cache.react(event.channel_id, event.message_id, ReactionChange::RemoveEmoji(&event.emoji))
    };
    PresenceUpdate => Option<Arc<CachedPresence>>, |event, cache| {
        cache.update_presence(event.guild_id?, event)
    };
    VoiceStateUpdate => Option<Arc<VoiceState>>, |event, cache| cache.update_voice_state(event);
    ThreadCreate => Option<Arc<Channel>>, |event, cache| cache.insert_thread(&event.thread);
    ThreadUpdate => Option<Arc<Channel>>, |event, cache| cache.insert_thread(event);
    ThreadDelete => Option<Arc<Channel>>, |event, cache| {
        cache.remove_thread(event.guild_id, event.id)
    };
    ThreadListSync => Vec<Arc<Channel>>, |event, cache| {
        cache.sync_threads(event.guild_id, event.channel_ids.as_deref(), &event.threads)
    };
    GuildScheduledEventCreate => Option<Arc<GuildScheduledEvent>>, |event, cache| {
        cache.put_scheduled_event(event)
    };
    GuildScheduledEventUpdate => Option<Arc<GuildScheduledEvent>>, |event, cache| {
        cache.put_scheduled_event(event)
    };
    GuildScheduledEventDelete => Option<Arc<GuildScheduledEvent>>, |event, cache| {
        cache.remove_scheduled_event(event)
    };
    StageInstanceCreate => Option<Arc<StageInstance>>, |event, cache| {
        cache.put_stage_instance(event)
    };
    StageInstanceUpdate => Option<Arc<StageInstance>>, |event, cache| {
        cache.put_stage_instance(event)
    };
    StageInstanceDelete => Option<Arc<StageInstance>>, |event, cache| {
        cache.remove_stage_instance(event)
    };
    GuildSoundboardSoundCreate => Option<Arc<SoundboardSound>>, |event, cache| {
        cache.put_soundboard_sound(event.guild_id?, event)
    };
    GuildSoundboardSoundUpdate => Option<Arc<SoundboardSound>>, |event, cache| {
        cache.put_soundboard_sound(event.guild_id?, event)
    };
    GuildSoundboardSoundDelete => Option<Arc<SoundboardSound>>, |event, cache| {
        cache.remove_soundboard_sound(event.guild_id, event.sound_id)
    };
    GuildSoundboardSoundsUpdate => Vec<Arc<SoundboardSound>>, |event, cache| {
        event
            .soundboard_sounds
            .iter()
            .filter_map(|sound| cache.put_soundboard_sound(event.guild_id, sound))
            .collect()
    };
    SoundboardSounds => Vec<Arc<SoundboardSound>>, |event, cache| {
        cache.replace_soundboard_sounds(event.guild_id, &event.soundboard_sounds)
    };
}

/// [`Cache::update_dispatch`].
pub(crate) fn dispatch<B: Backend>(cache: &Cache<B>, event: &DispatchEvent) -> Previous {
    match event {
        DispatchEvent::Ready(event) => cache.update(&**event).map(Previous::CurrentUser).into(),
        DispatchEvent::UserUpdate(event) => {
            cache.update(&**event).map(Previous::CurrentUser).into()
        }
        DispatchEvent::GuildCreate(event) => cache.update(&**event).map(Previous::Guild).into(),
        DispatchEvent::GuildUpdate(event) => cache.update(&**event).map(Previous::Guild).into(),
        DispatchEvent::GuildDelete(event) => cache.update(event).map(Previous::Guild).into(),
        DispatchEvent::ChannelCreate(event) => cache.update(&**event).map(Previous::Channel).into(),
        DispatchEvent::ChannelUpdate(event) => cache.update(&**event).map(Previous::Channel).into(),
        DispatchEvent::ChannelDelete(event) => cache.update(&**event).map(Previous::Channel).into(),
        DispatchEvent::ChannelPinsUpdate(event) => {
            cache.update(&**event).map(Previous::Channel).into()
        }
        DispatchEvent::GuildRoleCreate(event) => cache.update(&**event).map(Previous::Role).into(),
        DispatchEvent::GuildRoleUpdate(event) => cache.update(&**event).map(Previous::Role).into(),
        DispatchEvent::GuildRoleDelete(event) => cache.update(event).map(Previous::Role).into(),
        DispatchEvent::GuildEmojisUpdate(event) => Previous::Emojis(cache.update(&**event)),
        DispatchEvent::GuildStickersUpdate(event) => Previous::Stickers(cache.update(&**event)),
        DispatchEvent::GuildMemberAdd(event) => cache.update(&**event).map(Previous::Member).into(),
        DispatchEvent::GuildMemberUpdate(event) => {
            cache.update(&**event).map(Previous::Member).into()
        }
        DispatchEvent::GuildMemberRemove(event) => {
            cache.update(&**event).map(Previous::Member).into()
        }
        DispatchEvent::GuildMembersChunk(event) => Previous::Members(cache.update(&**event)),
        DispatchEvent::MessageCreate(event) => cache.update(&**event).map(Previous::Message).into(),
        DispatchEvent::MessageUpdate(event) => cache.update(&**event).map(Previous::Message).into(),
        DispatchEvent::MessageDelete(event) => cache.update(&**event).map(Previous::Message).into(),
        DispatchEvent::MessageDeleteBulk(event) => Previous::Messages(cache.update(&**event)),
        DispatchEvent::MessageReactionAdd(event) => {
            cache.update(&**event).map(Previous::Message).into()
        }
        DispatchEvent::MessageReactionRemove(event) => {
            cache.update(&**event).map(Previous::Message).into()
        }
        DispatchEvent::MessageReactionRemoveAll(event) => {
            cache.update(event).map(Previous::Message).into()
        }
        DispatchEvent::MessageReactionRemoveEmoji(event) => {
            cache.update(&**event).map(Previous::Message).into()
        }
        DispatchEvent::PresenceUpdate(event) => {
            cache.update(&**event).map(Previous::Presence).into()
        }
        DispatchEvent::VoiceStateUpdate(event) => {
            cache.update(&**event).map(Previous::VoiceState).into()
        }
        DispatchEvent::ThreadCreate(event) => cache.update(&**event).map(Previous::Thread).into(),
        DispatchEvent::ThreadUpdate(event) => cache.update(&**event).map(Previous::Thread).into(),
        DispatchEvent::ThreadDelete(event) => cache.update(&**event).map(Previous::Thread).into(),
        DispatchEvent::ThreadListSync(event) => Previous::Threads(cache.update(&**event)),
        DispatchEvent::GuildScheduledEventCreate(event) => {
            cache.update(&**event).map(Previous::ScheduledEvent).into()
        }
        DispatchEvent::GuildScheduledEventUpdate(event) => {
            cache.update(&**event).map(Previous::ScheduledEvent).into()
        }
        DispatchEvent::GuildScheduledEventDelete(event) => {
            cache.update(&**event).map(Previous::ScheduledEvent).into()
        }
        DispatchEvent::StageInstanceCreate(event) => {
            cache.update(&**event).map(Previous::StageInstance).into()
        }
        DispatchEvent::StageInstanceUpdate(event) => {
            cache.update(&**event).map(Previous::StageInstance).into()
        }
        DispatchEvent::StageInstanceDelete(event) => {
            cache.update(&**event).map(Previous::StageInstance).into()
        }
        DispatchEvent::GuildSoundboardSoundCreate(event) => {
            cache.update(&**event).map(Previous::SoundboardSound).into()
        }
        DispatchEvent::GuildSoundboardSoundUpdate(event) => {
            cache.update(&**event).map(Previous::SoundboardSound).into()
        }
        DispatchEvent::GuildSoundboardSoundDelete(event) => {
            cache.update(event).map(Previous::SoundboardSound).into()
        }
        DispatchEvent::GuildSoundboardSoundsUpdate(event) => {
            Previous::SoundboardSounds(cache.update(&**event))
        }
        DispatchEvent::SoundboardSounds(event) => {
            Previous::SoundboardSounds(cache.update(&**event))
        }
        _ => Previous::None,
    }
}

impl From<Option<Self>> for Previous {
    fn from(previous: Option<Self>) -> Self {
        previous.unwrap_or(Self::None)
    }
}
