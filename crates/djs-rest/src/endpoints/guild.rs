//! [Guild endpoints](https://docs.discord.com/developers/resources/guild): the guild, its
//! channels, members, bans, roles, prune, voice regions, invites, integrations, widget,
//! vanity URL, welcome screen, onboarding and incident actions.
//!
//! Deprecated: Modify Current User Nick is left out (D-105; use
//! [`modify_current_member`](Rest::modify_current_member)). Get Guild Widget Image is a public
//! URL, not a request (D-116).

use std::collections::HashMap;

use djs_model::{
    channel::Channel,
    guild::{
        Ban, Guild, IncidentsData, Integration, Member, Onboarding, Preview, Role, WelcomeScreen,
        Widget, WidgetSettings,
    },
    id::{GuildId, IntegrationId, RoleId, UserId},
    invite::InviteWithMetadata,
    permissions::Permissions,
    rest::{
        ActiveThreads, AddGuildMember, BeginGuildPrune, BulkBanResponse, BulkGuildBan,
        CreateGuildBan, CreateGuildChannel, CreateGuildRole, GetGuild, GetGuildBans,
        GetGuildPruneCount, ListGuildMembers, ModifyCurrentMember, ModifyGuild,
        ModifyGuildChannelPosition, ModifyGuildIncidentActions, ModifyGuildMember,
        ModifyGuildOnboarding, ModifyGuildRole, ModifyGuildRolePosition, ModifyGuildWelcomeScreen,
        ModifyGuildWidget, SearchGuildMembers, VanityUrl,
    },
    voice::VoiceRegion,
};
use serde::Deserialize;

use super::PermissionsResponse;
use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

/// The response of the prune endpoints, unwrapped by the methods.
#[derive(Deserialize)]
struct PruneResponse {
    pruned: Option<u32>,
}

/// The guild itself, its channels and active threads.
impl Rest {
    /// [Get Guild](https://docs.discord.com/developers/resources/guild#get-guild): returns a
    /// guild the bot is in; with [`with_counts`](GetGuild::with_counts) also the approximate
    /// member and presence counts.
    ///
    /// discord.js counterpart: `GuildManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10004` Unknown Guild.
    pub fn get_guild<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a GetGuild,
    ) -> impl Future<Output = Result<Guild, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetGuild { guild_id }).query(query))
                .await
        })
    }

    /// [Get Guild Preview](https://docs.discord.com/developers/resources/guild#get-guild-preview):
    /// returns the public preview of a guild (one the bot is not in must be discoverable).
    ///
    /// discord.js counterpart: `Client#fetchGuildPreview`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10004` Unknown Guild.
    pub fn get_guild_preview(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Preview, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildPreview { guild_id }))
                .await
        })
    }

    /// [Modify Guild](https://docs.discord.com/developers/resources/guild#modify-guild):
    /// changes the settings set in `body`. Needs `MANAGE_GUILD`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Guild#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50035` for an invalid field (see
    /// [`ApiError::field_errors`](crate::ApiError::field_errors)).
    pub fn modify_guild<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyGuild,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Guild, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuild { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Get Guild Channels](https://docs.discord.com/developers/resources/guild#get-guild-channels):
    /// returns the guild's channels, without threads. From 2026-11-16 Discord leaves out the
    /// channels the bot cannot view.
    ///
    /// discord.js counterpart: `GuildChannelManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_channels(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Channel>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildChannels { guild_id }))
                .await
        })
    }

    /// [Create Guild Channel](https://docs.discord.com/developers/resources/guild#create-guild-channel):
    /// creates a channel. Needs `MANAGE_CHANNELS`; only permissions the bot has can be set in
    /// its overwrites.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildChannelManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{
    ///     channel::Overwrite, id::RoleId, permissions::Permissions, rest::CreateGuildChannel,
    /// };
    ///
    /// let mut announcements = CreateGuildChannel::new("announcements");
    /// // `@everyone` (the guild's ID) can read but not write.
    /// announcements.permission_overwrites.push(Overwrite::role(
    ///     guild_id.cast(),
    ///     Permissions::VIEW_CHANNEL,
    ///     Permissions::SEND_MESSAGES,
    /// ));
    /// rest.create_guild_channel(guild_id, &announcements, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30013` at the maximum number of channels (500).
    pub fn create_guild_channel<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a CreateGuildChannel,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Channel, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateGuildChannel { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Guild Channel Positions](https://docs.discord.com/developers/resources/guild#modify-guild-channel-positions):
    /// moves channels within the list or between categories (only the listed channels).
    /// Needs `MANAGE_CHANNELS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildChannelManager#setPositions`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40009` when more than one entry changes its parent.
    pub fn modify_guild_channel_positions<'a>(
        &'a self,
        guild_id: GuildId,
        positions: &'a [ModifyGuildChannelPosition],
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildChannelPositions { guild_id })
                    .json(positions)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [List Active Guild Threads](https://docs.discord.com/developers/resources/guild#list-active-guild-threads):
    /// returns the active public and private threads of the guild, highest ID first.
    ///
    /// discord.js counterpart: `GuildChannelManager#fetchActiveThreads`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_active_guild_threads(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<ActiveThreads, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::ListActiveGuildThreads { guild_id }))
                .await
        })
    }
}

/// Members.
impl Rest {
    /// [Get Guild Member](https://docs.discord.com/developers/resources/guild#get-guild-member):
    /// returns a member of the guild.
    ///
    /// discord.js counterpart: `GuildMemberManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10007` Unknown Member.
    pub fn get_guild_member(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> impl Future<Output = Result<Member, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildMember { guild_id, user_id }))
                .await
        })
    }

    /// [List Guild Members](https://docs.discord.com/developers/resources/guild#list-guild-members):
    /// returns a page of members, sorted by user ID. Needs the `GUILD_MEMBERS` privileged
    /// intent.
    ///
    /// For all members of a large guild, the gateway's Request Guild Members is usually
    /// better.
    ///
    /// discord.js counterpart: `GuildMemberManager#list`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50001` Missing Access without the intent.
    pub fn list_guild_members<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a ListGuildMembers,
    ) -> impl Future<Output = Result<Vec<Member>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListGuildMembers { guild_id }).query(query))
                .await
        })
    }

    /// [Search Guild Members](https://docs.discord.com/developers/resources/guild#search-guild-members):
    /// returns members whose username or nickname starts with the query.
    ///
    /// discord.js counterpart: `GuildMemberManager#search`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn search_guild_members<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a SearchGuildMembers,
    ) -> impl Future<Output = Result<Vec<Member>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::SearchGuildMembers { guild_id }).query(query))
                .await
        })
    }

    /// [Get Guild Member Permissions](https://docs.discord.com/developers/resources/guild#get-guild-member-permissions):
    /// returns a member's guild-wide permissions (their roles with `@everyone`; D-110).
    ///
    /// discord.js counterpart: `GuildMember#permissions` (computed locally).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10007` Unknown Member.
    pub fn get_guild_member_permissions(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> impl Future<Output = Result<Permissions, Error>> + Send + '_ {
        boxed(async move {
            let response: PermissionsResponse = self
                .request(Request::new(Route::GetGuildMemberPermissions {
                    guild_id,
                    user_id,
                }))
                .await?;
            Ok(response.permissions)
        })
    }

    /// [Add Guild Member](https://docs.discord.com/developers/resources/guild#add-guild-member):
    /// adds a user to the guild with an `OAuth2` access token they granted the bot's
    /// application (`guilds.join` scope). Returns the new member, or `None` when the user
    /// already is one. The bot must be in the guild with `CREATE_INSTANT_INVITE`.
    ///
    /// discord.js counterpart: `GuildMemberManager#add`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40007` when the user is banned, `30001` when they are in 100
    /// guilds.
    pub fn add_guild_member<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        body: &'a AddGuildMember,
    ) -> impl Future<Output = Result<Option<Member>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::AddGuildMember { guild_id, user_id }).json(body)?)
                .await
        })
    }

    /// [Modify Guild Member](https://docs.discord.com/developers/resources/guild#modify-guild-member):
    /// changes a member's nickname, roles, voice state or timeout (each needs its own
    /// permission; see [`ModifyGuildMember`]).
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildMember#edit`, `#timeout`, `#setNickname`, …
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, user_id: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{rest::ModifyGuildMember, util::{Patch, Timestamp}};
    ///
    /// // Time the member out for ten minutes.
    /// let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    /// let until = Timestamp::from_unix_secs(now.as_secs() as i64 + 600).unwrap();
    /// let mut body = ModifyGuildMember::default();
    /// body.communication_disabled_until = Patch::Set(until);
    /// rest.modify_guild_member(guild_id, user_id, &body, Some("Spam"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50013` Missing Permissions (also for members above the bot's
    /// highest role).
    pub fn modify_guild_member<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        body: &'a ModifyGuildMember,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Member, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildMember { guild_id, user_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Current Member](https://docs.discord.com/developers/resources/guild#modify-current-member):
    /// changes the bot's own nickname, guild avatar, banner or bio.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildMemberManager#editMe`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_current_member<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyCurrentMember,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Member, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyCurrentMember { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Add Guild Member Role](https://docs.discord.com/developers/resources/guild#add-guild-member-role):
    /// gives a member a role. Needs `MANAGE_ROLES` and a higher role than the one given.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildMemberRoleManager#add`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10011` Unknown Role.
    pub fn add_guild_member_role<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::AddGuildMemberRole {
                    guild_id,
                    user_id,
                    role_id,
                })
                .reason(reason),
            )
            .await
        })
    }

    /// [Remove Guild Member Role](https://docs.discord.com/developers/resources/guild#remove-guild-member-role):
    /// takes a role from a member. Needs `MANAGE_ROLES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildMemberRoleManager#remove`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn remove_guild_member_role<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        role_id: RoleId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::RemoveGuildMemberRole {
                    guild_id,
                    user_id,
                    role_id,
                })
                .reason(reason),
            )
            .await
        })
    }

    /// [Remove Guild Member](https://docs.discord.com/developers/resources/guild#remove-guild-member):
    /// kicks a member. Needs `KICK_MEMBERS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildMember#kick`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50013` Missing Permissions.
    pub fn remove_guild_member<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::RemoveGuildMember { guild_id, user_id }).reason(reason))
                .await
        })
    }
}

/// Bans.
impl Rest {
    /// [Get Guild Bans](https://docs.discord.com/developers/resources/guild#get-guild-bans):
    /// returns a page of bans, sorted by user ID. Needs `BAN_MEMBERS` or `VIEW_AUDIT_LOG`.
    ///
    /// discord.js counterpart: `GuildBanManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_bans<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a GetGuildBans,
    ) -> impl Future<Output = Result<Vec<Ban>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetGuildBans { guild_id }).query(query))
                .await
        })
    }

    /// [Get Guild Ban](https://docs.discord.com/developers/resources/guild#get-guild-ban):
    /// returns the ban of a user. Needs `BAN_MEMBERS` or `VIEW_AUDIT_LOG`.
    ///
    /// discord.js counterpart: `GuildBanManager#fetch(user)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10026` Unknown Ban.
    pub fn get_guild_ban(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> impl Future<Output = Result<Ban, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildBan { guild_id, user_id }))
                .await
        })
    }

    /// [Create Guild Ban](https://docs.discord.com/developers/resources/guild#create-guild-ban):
    /// bans a user (member or not), optionally deleting their recent messages. Needs
    /// `BAN_MEMBERS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildBanManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, user_id: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateGuildBan;
    ///
    /// let mut body = CreateGuildBan::default();
    /// body.delete_message_seconds = Some(24 * 60 * 60);
    /// rest.create_guild_ban(guild_id, user_id, &body, Some("Raid"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50013` Missing Permissions.
    pub fn create_guild_ban<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        body: &'a CreateGuildBan,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateGuildBan { guild_id, user_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Remove Guild Ban](https://docs.discord.com/developers/resources/guild#remove-guild-ban):
    /// unbans a user. Needs `BAN_MEMBERS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildBanManager#remove`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10026` Unknown Ban.
    pub fn remove_guild_ban<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::RemoveGuildBan { guild_id, user_id }).reason(reason))
                .await
        })
    }

    /// [Bulk Guild Ban](https://docs.discord.com/developers/resources/guild#bulk-guild-ban):
    /// bans up to 200 users at once, returning who was and was not banned. Needs
    /// `BAN_MEMBERS` and `MANAGE_GUILD`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildBanManager#bulkCreate`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `500000` when none of the users could be banned.
    pub fn bulk_guild_ban<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a BulkGuildBan,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<BulkBanResponse, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::BulkGuildBan { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }
}

/// Roles.
impl Rest {
    /// [Get Guild Roles](https://docs.discord.com/developers/resources/guild#get-guild-roles):
    /// returns the guild's roles.
    ///
    /// discord.js counterpart: `RoleManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_roles(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Role>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildRoles { guild_id }))
                .await
        })
    }

    /// [Get Guild Role](https://docs.discord.com/developers/resources/guild#get-guild-role):
    /// returns one role.
    ///
    /// discord.js counterpart: `RoleManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10011` Unknown Role.
    pub fn get_guild_role(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
    ) -> impl Future<Output = Result<Role, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildRole { guild_id, role_id }))
                .await
        })
    }

    /// [Get Guild Role Permissions](https://docs.discord.com/developers/resources/guild#get-guild-role-permissions):
    /// returns the guild-wide permissions of a role together with `@everyone` (D-110).
    ///
    /// discord.js counterpart: `Role#permissions` (computed locally).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10011` Unknown Role.
    pub fn get_guild_role_permissions(
        &self,
        guild_id: GuildId,
        role_id: RoleId,
    ) -> impl Future<Output = Result<Permissions, Error>> + Send + '_ {
        boxed(async move {
            let response: PermissionsResponse = self
                .request(Request::new(Route::GetGuildRolePermissions {
                    guild_id,
                    role_id,
                }))
                .await?;
            Ok(response.permissions)
        })
    }

    /// [Get Guild Role Member Counts](https://docs.discord.com/developers/resources/guild#get-guild-role-member-counts):
    /// returns how many members have each role (without `@everyone`).
    ///
    /// discord.js counterpart: `RoleManager#fetchMemberCounts`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_role_member_counts(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<HashMap<RoleId, u32>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildRoleMemberCounts { guild_id }))
                .await
        })
    }

    /// [Create Guild Role](https://docs.discord.com/developers/resources/guild#create-guild-role):
    /// creates a role. Needs `MANAGE_ROLES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `RoleManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{guild::RoleColors, rest::CreateGuildRole};
    ///
    /// let mut body = CreateGuildRole::default();
    /// body.name = Some("Rustacean".into());
    /// body.colors = Some(RoleColors::solid(0xdea5_84));
    /// body.mentionable = true;
    /// let role = rest.create_guild_role(guild_id, &body, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30005` at the maximum number of roles (250).
    pub fn create_guild_role<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a CreateGuildRole,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Role, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateGuildRole { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Guild Role Positions](https://docs.discord.com/developers/resources/guild#modify-guild-role-positions):
    /// reorders roles and returns all of them. Needs `MANAGE_ROLES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `RoleManager#setPositions`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_guild_role_positions<'a>(
        &'a self,
        guild_id: GuildId,
        positions: &'a [ModifyGuildRolePosition],
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Vec<Role>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildRolePositions { guild_id })
                    .json(positions)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Guild Role](https://docs.discord.com/developers/resources/guild#modify-guild-role):
    /// changes the fields set in `body`. Needs `MANAGE_ROLES` and a higher role.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Role#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_guild_role<'a>(
        &'a self,
        guild_id: GuildId,
        role_id: RoleId,
        body: &'a ModifyGuildRole,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Role, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildRole { guild_id, role_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Delete Guild Role](https://docs.discord.com/developers/resources/guild#delete-guild-role):
    /// deletes a role. Needs `MANAGE_ROLES`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Role#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10011` Unknown Role.
    pub fn delete_guild_role<'a>(
        &'a self,
        guild_id: GuildId,
        role_id: RoleId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteGuildRole { guild_id, role_id }).reason(reason))
                .await
        })
    }
}

/// Prune, voice regions, invites and integrations.
impl Rest {
    /// [Get Guild Prune Count](https://docs.discord.com/developers/resources/guild#get-guild-prune-count):
    /// returns how many members a prune would kick. Needs `MANAGE_GUILD` and `KICK_MEMBERS`
    /// (`ADMINISTRATOR` with the `PRUNE_REQUIRES_ADMIN` feature).
    ///
    /// discord.js counterpart: `GuildMemberManager#prune({ dry: true })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_prune_count<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a GetGuildPruneCount,
    ) -> impl Future<Output = Result<u32, Error>> + Send + 'a {
        boxed(async move {
            let response: PruneResponse = self
                .request(Request::new(Route::GetGuildPruneCount { guild_id }).query(query))
                .await?;
            Ok(response.pruned.unwrap_or(0))
        })
    }

    /// [Begin Guild Prune](https://docs.discord.com/developers/resources/guild#begin-guild-prune):
    /// kicks the inactive members and returns how many, unless
    /// [`compute_prune_count`](BeginGuildPrune::compute_prune_count) is `false`. Needs the
    /// permissions of [`get_guild_prune_count`](Self::get_guild_prune_count).
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildMemberManager#prune`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn begin_guild_prune<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a BeginGuildPrune,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Option<u32>, Error>> + Send + 'a {
        boxed(async move {
            let response: PruneResponse = self
                .request(
                    Request::new(Route::BeginGuildPrune { guild_id })
                        .json(body)?
                        .reason(reason),
                )
                .await?;
            Ok(response.pruned)
        })
    }

    /// [Get Guild Voice Regions](https://docs.discord.com/developers/resources/guild#get-guild-voice-regions):
    /// returns the voice regions available to the guild (including VIP ones).
    ///
    /// discord.js counterpart: `Guild#fetchVoiceRegions`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_voice_regions(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<VoiceRegion>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildVoiceRegions { guild_id }))
                .await
        })
    }

    /// [Get Guild Invites](https://docs.discord.com/developers/resources/guild#get-guild-invites):
    /// returns the guild's invites. Needs `MANAGE_GUILD` or `VIEW_AUDIT_LOG`; the
    /// [`metadata`](InviteWithMetadata::metadata) is only included with `MANAGE_GUILD`
    /// (D-117).
    ///
    /// discord.js counterpart: `GuildInviteManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_invites(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<InviteWithMetadata>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildInvites { guild_id }))
                .await
        })
    }

    /// [Get Guild Integrations](https://docs.discord.com/developers/resources/guild#get-guild-integrations):
    /// returns up to 50 integrations of the guild. Needs `MANAGE_GUILD`.
    ///
    /// discord.js counterpart: `Guild#fetchIntegrations`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_integrations(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Integration>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildIntegrations { guild_id }))
                .await
        })
    }

    /// [Delete Guild Integration](https://docs.discord.com/developers/resources/guild#delete-guild-integration):
    /// removes an integration with its webhooks, kicking its bot. Needs `MANAGE_GUILD`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Integration#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_guild_integration<'a>(
        &'a self,
        guild_id: GuildId,
        integration_id: IntegrationId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::DeleteGuildIntegration {
                    guild_id,
                    integration_id,
                })
                .reason(reason),
            )
            .await
        })
    }
}

/// Widget, vanity URL, welcome screen, onboarding and incident actions.
impl Rest {
    /// [Get Guild Widget Settings](https://docs.discord.com/developers/resources/guild#get-guild-widget-settings):
    /// returns whether the widget is enabled and its invite channel. Needs `MANAGE_GUILD`.
    ///
    /// discord.js counterpart: `Guild#fetchWidgetSettings`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_widget_settings(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<WidgetSettings, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildWidgetSettings { guild_id }))
                .await
        })
    }

    /// [Modify Guild Widget](https://docs.discord.com/developers/resources/guild#modify-guild-widget):
    /// changes the widget settings. Needs `MANAGE_GUILD`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Guild#setWidgetSettings`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_guild_widget<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyGuildWidget,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<WidgetSettings, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildWidget { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Get Guild Widget](https://docs.discord.com/developers/resources/guild#get-guild-widget):
    /// returns the public widget data (online members, voice channels, invite).
    ///
    /// discord.js counterpart: `Client#fetchGuildWidget`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50004` when the widget is disabled.
    pub fn get_guild_widget(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Widget, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildWidget { guild_id }))
                .await
        })
    }

    /// [Get Guild Vanity URL](https://docs.discord.com/developers/resources/guild#get-guild-vanity-url):
    /// returns the vanity invite code and its uses. Needs `MANAGE_GUILD`.
    ///
    /// discord.js counterpart: `Guild#fetchVanityData`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_vanity_url(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<VanityUrl, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildVanityUrl { guild_id }))
                .await
        })
    }

    /// [Get Guild Welcome Screen](https://docs.discord.com/developers/resources/guild#get-guild-welcome-screen):
    /// returns the welcome screen (needs `MANAGE_GUILD` when it is disabled).
    ///
    /// discord.js counterpart: `Guild#fetchWelcomeScreen`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_welcome_screen(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<WelcomeScreen, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildWelcomeScreen { guild_id }))
                .await
        })
    }

    /// [Modify Guild Welcome Screen](https://docs.discord.com/developers/resources/guild#modify-guild-welcome-screen):
    /// changes the welcome screen. Needs `MANAGE_GUILD`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Guild#editWelcomeScreen`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_guild_welcome_screen<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyGuildWelcomeScreen,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<WelcomeScreen, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildWelcomeScreen { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Get Guild Onboarding](https://docs.discord.com/developers/resources/guild#get-guild-onboarding):
    /// returns the onboarding flow.
    ///
    /// discord.js counterpart: `Guild#fetchOnboarding`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_onboarding(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Onboarding, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildOnboarding { guild_id }))
                .await
        })
    }

    /// [Modify Guild Onboarding](https://docs.discord.com/developers/resources/guild#modify-guild-onboarding):
    /// replaces the onboarding configuration. Needs `MANAGE_GUILD` and `MANAGE_ROLES`. New
    /// prompts get generated IDs (D-118).
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Guild#editOnboarding`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50035` when the enabled flow lacks default channels.
    pub fn modify_guild_onboarding<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyGuildOnboarding,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Onboarding, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildOnboarding { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Guild Incident Actions](https://docs.discord.com/developers/resources/guild#modify-guild-incident-actions):
    /// pauses invites or direct messages for up to 24 hours (e.g. during a raid), or ends a
    /// pause. Needs `MANAGE_GUILD`.
    ///
    /// discord.js counterpart: `Guild#setIncidentActions`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_guild_incident_actions<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyGuildIncidentActions,
    ) -> impl Future<Output = Result<IncidentsData, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ModifyGuildIncidentActions { guild_id }).json(body)?)
                .await
        })
    }
}
