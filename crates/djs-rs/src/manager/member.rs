//! [`MemberManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CachedMember, GuildView};
use djs_collection::Collection;
use djs_model::{
    id::{GuildId, RoleId, UserId},
    rest::{CreateGuildBan, ListGuildMembers, ModifyGuildMember},
    util::{Patch, Timestamp},
};
use djs_rest::Error;

use super::Fetch;
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
        let cached = self.cache().and_then(|members| members.get(user_id));
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let member = ctx.rest().get_guild_member(guild_id, user_id).await?;
            Ok(ctx.store_member(guild_id, &member))
        })
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
}
