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
//! enabled in the [`CacheConfig`]; without a cache it is only
//! converted. Objects the cache never keeps (bans) are returned as their
//! [`djs-model`](djs_model) type.
//!
//! Managers are small `Copy` values borrowing the context; their methods consume them, so
//! `ctx.members(guild).kick(user, None).await` needs no binding.
//!
//! discord.js counterpart: the `*Manager` classes (`GuildMemberManager`, `MessageManager`, …).

mod ban;
mod channel;
mod guild;
mod guild_channel;
mod member;
mod message;
mod role;
mod user;

use core::{
    fmt,
    future::{Future, IntoFuture},
    pin::Pin,
    task::{Context as TaskContext, Poll},
};
use std::sync::{Arc, LazyLock};

use djs_cache::{
    Backend, Cache, CacheConfig, CachedGuild, CachedMember, CachedMessage, CachedUser, Limit,
};
use djs_model::{
    channel::Channel,
    guild::{Guild, Member, Role},
    id::GuildId,
    message::Message,
    user::User,
};
use djs_rest::Error;

pub use self::{
    ban::BanManager,
    channel::{ChannelManager, OverwriteTarget},
    guild::GuildManager,
    guild_channel::GuildChannelManager,
    member::MemberManager,
    message::MessageManager,
    role::RoleManager,
    user::UserManager,
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

/// A cache that keeps nothing, converting REST objects for clients without a cache.
static CONVERT: LazyLock<Cache> = LazyLock::new(|| {
    Cache::with_config(
        CacheConfig::builder()
            .guilds(Limit::Disabled)
            .channels(Limit::Disabled)
            .roles(Limit::Disabled)
            .build(),
    )
});

/// Converts REST objects to the cached types through the context's cache (D-230, D-233).
impl<B: Backend> Context<B> {
    pub(crate) fn store_guild(&self, guild: &Guild) -> Arc<CachedGuild> {
        self.cache().map_or_else(
            || CONVERT.store_guild(guild),
            |cache| cache.store_guild(guild),
        )
    }

    pub(crate) fn store_channel(&self, channel: &Channel) -> Arc<Channel> {
        self.cache().map_or_else(
            || CONVERT.store_channel(channel),
            |cache| cache.store_channel(channel),
        )
    }

    pub(crate) fn store_user(&self, user: &User) -> Arc<CachedUser> {
        self.cache()
            .map_or_else(|| CONVERT.store_user(user), |cache| cache.store_user(user))
    }

    pub(crate) fn store_member(&self, guild_id: GuildId, member: &Member) -> Arc<CachedMember> {
        self.cache().map_or_else(
            || CONVERT.store_member(guild_id, member),
            |cache| cache.store_member(guild_id, member),
        )
    }

    pub(crate) fn store_message(&self, message: &Message) -> Arc<CachedMessage> {
        self.cache().map_or_else(
            || CONVERT.store_message(message),
            |cache| cache.store_message(message),
        )
    }

    pub(crate) fn store_role(&self, guild_id: GuildId, role: &Role) -> Arc<Role> {
        self.cache().map_or_else(
            || CONVERT.store_role(guild_id, role),
            |cache| cache.store_role(guild_id, role),
        )
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
    pub const fn messages(&self, channel_id: djs_model::id::ChannelId) -> MessageManager<'_, B> {
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
}
