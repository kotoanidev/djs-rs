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
            GuildStickersUpdate, GuildUpdate, Ready, UserUpdate,
        },
    },
    guild::Role,
    sticker::Sticker,
    user::CurrentUser,
};

use crate::{Backend, Cache, CachedGuild, CachedMember};

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
/// | [`GuildMembersChunk`] | caches the members | `Vec<Arc<CachedMember>>` (those replaced) |
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
        event
            .members
            .iter()
            .filter_map(|member| cache.upsert_member(event.guild_id, member))
            .collect()
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
        _ => Previous::None,
    }
}

impl From<Option<Self>> for Previous {
    fn from(previous: Option<Self>) -> Self {
        previous.unwrap_or(Self::None)
    }
}
