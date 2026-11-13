//! Members' permissions computed from the cache (D-292, D-294, D-295).

use djs_model::{
    channel::Channel,
    id::{GuildId, UserId},
    permissions::Permissions,
    util::Timestamp,
};

use crate::{Backend, Cache, CachedMember};

/// Permissions computed from the cached guild, roles, member and channel (D-292).
impl<B: Backend> Cache<B> {
    /// The permissions of the member `user_id` in the guild `guild_id`, as Discord applies them
    /// now: [`Permissions::guild_level`] from the cached guild owner, `@everyone` role and the
    /// member's roles, then [`Permissions::during_timeout`] if the member is timed out (D-294).
    ///
    /// `None` when the cache lacks something the computation needs: the guild, the member, or
    /// one of the member's roles (so enable [`Resource::Member`](crate::Resource::Member),
    /// which is off by default, and keep [`Resource::Role`](crate::Resource::Role)). A
    /// missing piece never gives a wrong answer.
    ///
    /// discord.js counterpart: `GuildMember#permissions`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_cache::Cache;
    /// use djs_model::{id::{GuildId, UserId}, permissions::Permissions};
    ///
    /// fn can_ban(cache: &Cache, guild: GuildId, user: UserId) -> bool {
    ///     cache
    ///         .member_permissions(guild, user)
    ///         .is_some_and(|permissions| permissions.contains(Permissions::BAN_MEMBERS))
    /// }
    /// # assert!(!can_ban(&Cache::new(), GuildId::new(1), UserId::new(2)));
    /// ```
    #[must_use]
    pub fn member_permissions(&self, guild_id: GuildId, user_id: UserId) -> Option<Permissions> {
        let member = self.members(guild_id).get(user_id)?;
        let base = self.guild_level(guild_id, &member)?;
        Some(apply_timeout(base, &member))
    }

    /// The permissions of the member `user_id` in `channel` (a guild channel or a thread,
    /// cached or not), as Discord applies them now: [`member_permissions`](Self::member_permissions)
    /// before the timeout, then [`Permissions::in_channel`] with the channel's overwrites and
    /// the implicit denials (D-295), then the timeout (D-294). A thread takes the overwrites of
    /// its parent channel, which must be cached.
    ///
    /// `None` for a channel outside a guild, or when the cache lacks something the computation
    /// needs (see [`member_permissions`](Self::member_permissions); for a thread, its parent).
    ///
    /// discord.js counterpart: `GuildMember#permissionsIn(channel)` /
    /// `GuildChannel#permissionsFor(member)`.
    #[must_use]
    pub fn member_permissions_in(&self, channel: &Channel, user_id: UserId) -> Option<Permissions> {
        let guild_id = channel.guild_id()?;
        let member = self.members(guild_id).get(user_id)?;
        let base = self.guild_level(guild_id, &member)?;
        let in_channel = if channel.kind().is_thread() {
            let parent = self.channels().get(channel.parent_id()?)?;
            base.in_channel(
                guild_id,
                user_id,
                &member.roles,
                parent.permission_overwrites(),
            )
        } else {
            base.in_channel(
                guild_id,
                user_id,
                &member.roles,
                channel.permission_overwrites(),
            )
        };
        Some(apply_timeout(in_channel, &member))
    }

    /// [`Permissions::guild_level`] of `member` from the cached guild and roles.
    fn guild_level(&self, guild_id: GuildId, member: &CachedMember) -> Option<Permissions> {
        let guild = self.guilds().get(guild_id)?;
        if guild.owner_id == member.user.id {
            return Some(Permissions::all());
        }
        let roles = self.roles(guild_id);
        // `@everyone` has the guild's ID.
        let everyone = roles.get(guild_id.cast())?.permissions;
        let mut granted = Permissions::empty();
        for &role_id in &*member.roles {
            granted |= roles.get(role_id)?.permissions;
        }
        Some(Permissions::guild_level(false, everyone, [granted]))
    }
}

/// Applies [`Permissions::during_timeout`] if `member`'s timeout has not ended yet.
fn apply_timeout(permissions: Permissions, member: &CachedMember) -> Permissions {
    match member.communication_disabled_until() {
        Some(until) if until > Timestamp::now() => permissions.during_timeout(),
        _ => permissions,
    }
}
