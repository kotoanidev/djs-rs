//! The payloads the client builds itself (D-225): events with the state before them (D-221,
//! D-227), events derived from a list (D-223) and shard lifecycle events (D-226).
//!
//! Events with a previous state have two fields (D-228):
//!
//! - `old`: what the cache held before the event — `None` without a cache, with the resource
//!   disabled in [`CacheConfig`](djs_cache::CacheConfig), or for something it never received
//!   (the counterpart of a discord.js partial);
//! - `new`: Discord's data for the event, as its [`djs-model`](djs_model) type. For a removal,
//!   this is Discord's notice (IDs) and `old` the removed object.
//!
//! Each of them dereferences to `new`, so its fields read as on an event without a wrapper:
//!
//! ```no_run
//! # use djs_rs::prelude::*;
//! # let client = Client::builder("token").build();
//! client.on(Events::GuildMemberUpdate, |_ctx, update| async move {
//!     let old_nick = update.old.as_ref().and_then(|old| old.nick.clone());
//!     println!("{} was {old_nick:?}, is now {:?}", update.user.id, update.nick);
//!     Ok(())
//! });
//! ```

use core::ops::Deref;
use std::sync::Arc;

use djs_cache::{CachedGuild, CachedMember, CachedMessage, CachedPresence};
use djs_model::{
    channel::Channel, emoji::Emoji, gateway::payload as gateway, guild::Role,
    guild_scheduled_event::GuildScheduledEvent, id::GuildId, soundboard::SoundboardSound,
    stage_instance::StageInstance, sticker::Sticker, user::CurrentUser, voice::VoiceState,
};

/// Defines a payload with the state before the event: `Name(Old => New)`, where `old` is an
/// `Option<Arc<Old>>` and `new` a `New`.
macro_rules! with_old {
    ($(
        $(#[$doc:meta])*
        $name:ident($old:ty => $new:ident);
    )*) => {$(
        $(#[$doc])*
        ///
        #[doc = concat!(
            "Dereferences to `new`, a [`", stringify!($new), "`](djs_model::gateway::payload::",
            stringify!($new), ")."
        )]
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct $name {
            /// What the cache held before the event, if it held it.
            pub old: Option<Arc<$old>>,
            /// Discord's data for the event.
            pub new: gateway::$new,
        }

        impl Deref for $name {
            type Target = gateway::$new;

            fn deref(&self) -> &gateway::$new {
                &self.new
            }
        }
    )*};
}

with_old! {
    /// The payload of [`Events::ChannelUpdate`](crate::Events::ChannelUpdate).
    ChannelUpdate(Channel => ChannelUpdate);
    /// The payload of [`Events::ChannelDelete`](crate::Events::ChannelDelete).
    ChannelDelete(Channel => ChannelDelete);
    /// The payload of [`Events::ThreadUpdate`](crate::Events::ThreadUpdate).
    ThreadUpdate(Channel => ThreadUpdate);
    /// The payload of [`Events::ThreadDelete`](crate::Events::ThreadDelete): `new` only holds
    /// the thread's IDs, `old` the cached thread.
    ThreadDelete(Channel => ThreadDelete);
    /// The payload of [`Events::GuildUpdate`](crate::Events::GuildUpdate).
    GuildUpdate(CachedGuild => GuildUpdate);
    /// The payload of [`Events::GuildDelete`](crate::Events::GuildDelete): `new` only holds the
    /// guild's ID, `old` the cached guild.
    GuildDelete(CachedGuild => GuildDelete);
    /// The payload of [`Events::GuildUnavailable`](crate::Events::GuildUnavailable): `new`
    /// only holds the guild's ID, `old` the cached guild (which the cache keeps while the guild
    /// is unavailable).
    GuildUnavailable(CachedGuild => GuildDelete);
    /// The payload of [`Events::GuildMemberUpdate`](crate::Events::GuildMemberUpdate).
    GuildMemberUpdate(CachedMember => GuildMemberUpdate);
    /// The payload of [`Events::GuildMemberRemove`](crate::Events::GuildMemberRemove): `new`
    /// holds the guild ID and the user, `old` the cached member.
    GuildMemberRemove(CachedMember => GuildMemberRemove);
    /// The payload of [`Events::GuildRoleUpdate`](crate::Events::GuildRoleUpdate).
    GuildRoleUpdate(Role => GuildRoleUpdate);
    /// The payload of [`Events::GuildRoleDelete`](crate::Events::GuildRoleDelete): `new` only
    /// holds the IDs, `old` the cached role.
    GuildRoleDelete(Role => GuildRoleDelete);
    /// The payload of
    /// [`Events::GuildScheduledEventUpdate`](crate::Events::GuildScheduledEventUpdate).
    GuildScheduledEventUpdate(GuildScheduledEvent => GuildScheduledEventUpdate);
    /// The payload of
    /// [`Events::GuildScheduledEventDelete`](crate::Events::GuildScheduledEventDelete).
    GuildScheduledEventDelete(GuildScheduledEvent => GuildScheduledEventDelete);
    /// The payload of
    /// [`Events::GuildSoundboardSoundUpdate`](crate::Events::GuildSoundboardSoundUpdate).
    GuildSoundboardSoundUpdate(SoundboardSound => GuildSoundboardSoundUpdate);
    /// The payload of
    /// [`Events::GuildSoundboardSoundDelete`](crate::Events::GuildSoundboardSoundDelete): `new`
    /// only holds the IDs, `old` the cached sound.
    GuildSoundboardSoundDelete(SoundboardSound => GuildSoundboardSoundDelete);
    /// The payload of [`Events::StageInstanceUpdate`](crate::Events::StageInstanceUpdate).
    StageInstanceUpdate(StageInstance => StageInstanceUpdate);
    /// The payload of [`Events::StageInstanceDelete`](crate::Events::StageInstanceDelete).
    StageInstanceDelete(StageInstance => StageInstanceDelete);
    /// The payload of [`Events::MessageUpdate`](crate::Events::MessageUpdate).
    MessageUpdate(CachedMessage => MessageUpdate);
    /// The payload of [`Events::MessageDelete`](crate::Events::MessageDelete): `new` only holds
    /// the IDs, `old` the cached message.
    MessageDelete(CachedMessage => MessageDelete);
    /// The payload of [`Events::PresenceUpdate`](crate::Events::PresenceUpdate).
    PresenceUpdate(CachedPresence => PresenceUpdate);
    /// The payload of [`Events::UserUpdate`](crate::Events::UserUpdate).
    UserUpdate(CurrentUser => UserUpdate);
    /// The payload of [`Events::VoiceStateUpdate`](crate::Events::VoiceStateUpdate).
    VoiceStateUpdate(VoiceState => VoiceStateUpdate);
}

/// The payload of [`Events::MessageBulkDelete`](crate::Events::MessageBulkDelete): `new` holds
/// the IDs, `old` the messages the cache held (empty without a message cache).
///
/// Dereferences to `new`, a
/// [`MessageDeleteBulk`](djs_model::gateway::payload::MessageDeleteBulk).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MessageBulkDelete {
    /// The deleted messages the cache held, in no particular order.
    pub old: Vec<Arc<CachedMessage>>,
    /// Discord's data for the event.
    pub new: gateway::MessageDeleteBulk,
}

impl Deref for MessageBulkDelete {
    type Target = gateway::MessageDeleteBulk;

    fn deref(&self) -> &gateway::MessageDeleteBulk {
        &self.new
    }
}

/// Defines the three per-item payloads derived from a guild's list of `$item`s (D-223).
macro_rules! derived {
    ($item:ident: $field:ident, $create:ident, $update:ident, $delete:ident, $list:literal) => {
        #[doc = concat!(
                    "The payload of [`Events::", stringify!($create), "`](crate::Events::",
                    stringify!($create), "): a new ", $list, ".\n\nDereferences to the [`",
                    stringify!($item), "`]."
                )]
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct $create {
            /// The guild.
            pub guild_id: GuildId,
            #[doc = concat!("The new ", $list, ".")]
            pub $field: $item,
        }

        impl Deref for $create {
            type Target = $item;

            fn deref(&self) -> &$item {
                &self.$field
            }
        }

        #[doc = concat!(
                    "The payload of [`Events::", stringify!($update), "`](crate::Events::",
                    stringify!($update), "): a changed ", $list, ", before and after.\n\n",
                    "Dereferences to `new`."
                )]
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct $update {
            /// The guild.
            pub guild_id: GuildId,
            #[doc = concat!("The ", $list, " as the cache held it.")]
            pub old: Arc<$item>,
            #[doc = concat!("The ", $list, " now.")]
            pub new: $item,
        }

        impl Deref for $update {
            type Target = $item;

            fn deref(&self) -> &$item {
                &self.new
            }
        }

        #[doc = concat!(
                    "The payload of [`Events::", stringify!($delete), "`](crate::Events::",
                    stringify!($delete), "): a deleted ", $list, ", as the cache held it.\n\n",
                    "Dereferences to the [`", stringify!($item), "`]."
                )]
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct $delete {
            /// The guild.
            pub guild_id: GuildId,
            #[doc = concat!("The deleted ", $list, ".")]
            pub $field: Arc<$item>,
        }

        impl Deref for $delete {
            type Target = $item;

            fn deref(&self) -> &$item {
                &self.$field
            }
        }
    };
}

derived!(Emoji: emoji, GuildEmojiCreate, GuildEmojiUpdate, GuildEmojiDelete, "emoji");
derived!(Sticker: sticker, GuildStickerCreate, GuildStickerUpdate, GuildStickerDelete, "sticker");

/// The payload of [`Events::ShardResume`](crate::Events::ShardResume).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShardResume {
    /// How many events Discord replayed between the reconnect and `RESUMED`. They were
    /// emitted as usual before this event.
    pub replayed: u64,
}

/// The payload of [`Events::ShardReconnecting`](crate::Events::ShardReconnecting). The shard
/// is [`Context::shard_id`](crate::Context::shard_id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShardReconnecting;

/// The payload of [`Events::Invalidated`](crate::Events::Invalidated). The shard is
/// [`Context::shard_id`](crate::Context::shard_id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Invalidated;
