//! [`MemberManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CachedMember, GuildView};
use djs_collection::Collection;
use djs_model::{
    id::{GuildId, RoleId, UserId},
    rest::{
        BeginGuildPrune, CreateGuildBan, GetGuildPruneCount, ListGuildMembers, ModifyCurrentMember,
        ModifyGuildMember, SearchGuildMembers,
    },
    util::{Patch, Timestamp},
};
use djs_rest::Error;

use super::{Fetch, FetchMembers};
use crate::Context;

/// The members of one guild. Returned by [`Context::members`].
///
/// discord.js counterpart: `GuildMemberManager` (`guild.members`).
#[derive(Debug)]
pub struct MemberManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for MemberManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for MemberManager<'_, B> {}

impl<'a, B: Backend> MemberManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these members belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached members of the guild, if the client has a cache.
    ///
    /// discord.js counterpart: `GuildMemberManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, UserId, CachedMember, B>> {
        self.ctx.cache().map(|cache| cache.members(self.guild_id))
    }

    /// A member: the cached one, or [Get Guild
    /// Member](djs_rest::Rest::get_guild_member), whose result is cached.
    ///
    /// discord.js counterpart: `GuildMemberManager#fetch(user)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed (for example, the
    /// user is not a member).
    pub fn fetch(
        self,
        user_id: UserId,
    ) -> Fetch<Arc<CachedMember>, impl Future<Output = Result<Arc<CachedMember>, Error>> + Send + 'a>
    {
        let cached = self.ctx.ops().member(self.guild_id, user_id);
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let member = ctx.rest().get_guild_member(guild_id, user_id).await?;
            Ok(ctx.store_member(guild_id, &member))
        })
    }

    /// Every member of the guild, through the gateway (D-270): sends [Request Guild
    /// Members](djs_model::gateway::payload::RequestGuildMembers) on the guild's shard and
    /// gathers every chunk of the answer. Needs the privileged `GUILD_MEMBERS` intent. The
    /// members (and, with [`presences`](FetchMembers::presences), their presences) are cached.
    ///
    /// Await the returned request; it fails with
    /// [`GatewayRequestError`](super::GatewayRequestError) if the guild's shard does not run
    /// here, the answer does not complete in [`time`](FetchMembers::time) (120 seconds by
    /// default) or Discord rate limits it.
    ///
    /// discord.js counterpart: `GuildMemberManager#fetch()` (no options).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::{model::id::GuildId, prelude::*};
    /// # async fn example(ctx: Context, guild_id: GuildId) -> Result<(), HandlerError> {
    /// let members = ctx.members(guild_id).fetch_all().await?;
    /// println!("{} members", members.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn fetch_all(self) -> FetchMembers<'a, B> {
        FetchMembers::all(self.ctx, self.guild_id)
    }

    /// The members whose username or nickname starts with `query`, through the gateway (at
    /// most [`limit`](FetchMembers::limit), 100 by default), like [`fetch_all`](Self::fetch_all).
    ///
    /// discord.js counterpart: `GuildMemberManager#fetch({ query, limit })`.
    pub fn fetch_by_query(self, query: impl Into<String>) -> FetchMembers<'a, B> {
        FetchMembers::query(self.ctx, self.guild_id, query.into())
    }

    /// The members with these user IDs, through the gateway, like
    /// [`fetch_all`](Self::fetch_all); IDs that are not members are left out. Discord takes
    /// 100 IDs per request, so more are asked in several requests, one after another.
    ///
    /// discord.js counterpart: `GuildMemberManager#fetch({ user: [ids] })`.
    pub fn fetch_many(self, user_ids: impl IntoIterator<Item = UserId>) -> FetchMembers<'a, B> {
        FetchMembers::users(self.ctx, self.guild_id, user_ids.into_iter().collect())
    }

    /// A page of members, sorted by user ID ([List Guild
    /// Members](djs_rest::Rest::list_guild_members); needs the `GUILD_MEMBERS` intent). The
    /// members are cached.
    ///
    /// discord.js counterpart: `GuildMemberManager#list`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(
        self,
        query: &ListGuildMembers,
    ) -> Result<Collection<UserId, Arc<CachedMember>>, Error> {
        let members = self
            .ctx
            .rest()
            .list_guild_members(self.guild_id, query)
            .await?;
        Ok(members
            .iter()
            .map(|member| {
                let member = self.ctx.store_member(self.guild_id, member);
                (member.user.id, member)
            })
            .collect())
    }

    /// Edits a member ([Modify Guild Member](djs_rest::Rest::modify_guild_member)); the
    /// result is cached.
    ///
    /// discord.js counterpart: `GuildMemberManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        user_id: UserId,
        body: &ModifyGuildMember,
        reason: Option<&str>,
    ) -> Result<Arc<CachedMember>, Error> {
        let member = self
            .ctx
            .rest()
            .modify_guild_member(self.guild_id, user_id, body, reason)
            .await?;
        Ok(self.ctx.store_member(self.guild_id, &member))
    }

    /// Times a member out until `until` (at most 28 days ahead), or ends their timeout with
    /// `None`.
    ///
    /// discord.js counterpart: `GuildMember#timeout` / `GuildMember#disableCommunicationUntil`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MODERATE_MEMBERS`).
    pub async fn timeout(
        self,
        user_id: UserId,
        until: Option<Timestamp>,
        reason: Option<&str>,
    ) -> Result<Arc<CachedMember>, Error> {
        let mut body = ModifyGuildMember::default();
        body.communication_disabled_until = until.into();
        self.edit(user_id, &body, reason).await
    }

    /// Sets a member's nickname, or removes it with `None`.
    ///
    /// discord.js counterpart: `GuildMember#setNickname`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_NICKNAMES`).
    pub async fn set_nickname(
        self,
        user_id: UserId,
        nickname: Option<&str>,
        reason: Option<&str>,
    ) -> Result<Arc<CachedMember>, Error> {
        let mut body = ModifyGuildMember::default();
        body.nick = nickname.map_or(Patch::Clear, |nick| Patch::Set(nick.into()));
        self.edit(user_id, &body, reason).await
    }

    /// Gives a member a role ([Add Guild Member
    /// Role](djs_rest::Rest::add_guild_member_role)).
    ///
    /// discord.js counterpart: `GuildMemberRoleManager#add`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_ROLES`, and the role
    /// must be below the bot's highest role).
    pub async fn add_role(
        self,
        user_id: UserId,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .add_guild_member_role(self.guild_id, user_id, role_id, reason)
            .await
    }

    /// Takes a role from a member ([Remove Guild Member
    /// Role](djs_rest::Rest::remove_guild_member_role)).
    ///
    /// discord.js counterpart: `GuildMemberRoleManager#remove`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn remove_role(
        self,
        user_id: UserId,
        role_id: RoleId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .remove_guild_member_role(self.guild_id, user_id, role_id, reason)
            .await
    }

    /// Kicks a member ([Remove Guild Member](djs_rest::Rest::remove_guild_member)).
    ///
    /// discord.js counterpart: `GuildMemberManager#kick`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `KICK_MEMBERS`).
    pub async fn kick(self, user_id: UserId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .remove_guild_member(self.guild_id, user_id, reason)
            .await
    }

    /// Bans a user, member or not (the same as [`BanManager::create`](super::BanManager::create)).
    ///
    /// discord.js counterpart: `GuildMemberManager#ban`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `BAN_MEMBERS`).
    pub async fn ban(
        self,
        user_id: UserId,
        body: &CreateGuildBan,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .bans(self.guild_id)
            .create(user_id, body, reason)
            .await
    }

    /// Lifts a user's ban (the same as [`BanManager::remove`](super::BanManager::remove)).
    ///
    /// discord.js counterpart: `GuildMemberManager#unban`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (for example, the user is not banned).
    pub async fn unban(self, user_id: UserId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx.bans(self.guild_id).remove(user_id, reason).await
    }
    /// Members whose username or nickname starts with a query ([Search Guild
    /// Members](djs_rest::Rest::search_guild_members)); the members are cached.
    ///
    /// discord.js counterpart: `GuildMemberManager#search`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn search(
        self,
        query: &SearchGuildMembers,
    ) -> Result<Collection<UserId, Arc<CachedMember>>, Error> {
        let members = self
            .ctx
            .rest()
            .search_guild_members(self.guild_id, query)
            .await?;
        Ok(members
            .iter()
            .map(|member| {
                let member = self.ctx.store_member(self.guild_id, member);
                (member.user.id, member)
            })
            .collect())
    }

    /// Edits the bot's own membership: its nickname and guild profile ([Modify Current
    /// Member](djs_rest::Rest::modify_current_member)); the result is cached.
    ///
    /// discord.js counterpart: `GuildMemberManager#editMe`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CHANGE_NICKNAME` for the
    /// nickname).
    pub async fn edit_me(
        self,
        body: &ModifyCurrentMember,
        reason: Option<&str>,
    ) -> Result<Arc<CachedMember>, Error> {
        let member = self
            .ctx
            .rest()
            .modify_current_member(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_member(self.guild_id, &member))
    }

    /// How many members a prune would kick ([Get Guild Prune
    /// Count](djs_rest::Rest::get_guild_prune_count)).
    ///
    /// discord.js counterpart: `GuildMemberManager#prune` with `dry: true`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_GUILD` and
    /// `KICK_MEMBERS`).
    pub async fn prune_count(self, query: &GetGuildPruneCount) -> Result<u32, Error> {
        self.ctx
            .rest()
            .get_guild_prune_count(self.guild_id, query)
            .await
    }

    /// Kicks inactive members ([Begin Guild Prune](djs_rest::Rest::begin_guild_prune)),
    /// returning how many were kicked unless `compute_prune_count` was `false`.
    ///
    /// discord.js counterpart: `GuildMemberManager#prune`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_GUILD` and
    /// `KICK_MEMBERS`).
    pub async fn prune(
        self,
        body: &BeginGuildPrune,
        reason: Option<&str>,
    ) -> Result<Option<u32>, Error> {
        self.ctx
            .rest()
            .begin_guild_prune(self.guild_id, body, reason)
            .await
    }
}
