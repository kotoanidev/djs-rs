//! [OAuth2](https://docs.discord.com/developers/topics/oauth2): scopes an application can be
//! authorized for.

use crate::macros::str_enum;

str_enum! {
    /// An [OAuth2 scope](https://docs.discord.com/developers/topics/oauth2#shared-resources-oauth2-scopes):
    /// what an application may do on a user's behalf.
    ///
    /// discord.js counterpart: `OAuth2Scopes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::oauth2::Scope;
    ///
    /// let scope: Scope = serde_json::from_str(r#""applications.commands""#).unwrap();
    /// assert_eq!(scope, Scope::ApplicationsCommands);
    /// ```
    pub enum Scope {
        /// Read the user's "Now Playing/Recently Played" list (not available for apps).
        ActivitiesRead = "activities.read",
        /// Update the user's activity (not available for apps).
        ActivitiesWrite = "activities.write",
        /// Read build data of the user's applications.
        ApplicationsBuildsRead = "applications.builds.read",
        /// Upload builds for the user's applications (approved partners only).
        ApplicationsBuildsUpload = "applications.builds.upload",
        /// Add application commands to a guild (included with `bot`).
        ApplicationsCommands = "applications.commands",
        /// Update the app's commands with a client-credentials bearer token.
        ApplicationsCommandsUpdate = "applications.commands.update",
        /// Update the permissions of the app's commands in a guild the user can manage.
        ApplicationsCommandsPermissionsUpdate = "applications.commands.permissions.update",
        /// Read entitlements of the user's applications.
        ApplicationsEntitlements = "applications.entitlements",
        /// Read and update store data of the user's applications.
        ApplicationsStoreUpdate = "applications.store.update",
        /// Add the app's bot to the guild the user selects.
        Bot = "bot",
        /// List the user's linked third-party accounts.
        Connections = "connections",
        /// See the user's DMs and group DMs (approved partners only).
        DmChannelsRead = "dm_channels.read",
        /// Read the user's email address.
        Email = "email",
        /// Add the user to group DMs.
        GdmJoin = "gdm.join",
        /// List the user's guilds.
        Guilds = "guilds",
        /// Add the user to guilds.
        GuildsJoin = "guilds.join",
        /// Read the user's member data in their guilds.
        GuildsMembersRead = "guilds.members.read",
        /// Read the user's account (without email).
        Identify = "identify",
        /// Read the user's Nitro subscription type.
        IdentifyPremium = "identify.premium",
        /// Read messages from all client channels (local RPC server).
        MessagesRead = "messages.read",
        /// Read the user's friends, pending requests and blocked users (Social SDK).
        RelationshipsRead = "relationships.read",
        /// Update the user's connection and metadata for the app (linked roles).
        RoleConnectionsWrite = "role_connections.write",
        /// Control the user's local Discord client (approved partners only).
        Rpc = "rpc",
        /// Update the user's activity over RPC (approved partners only).
        RpcActivitiesWrite = "rpc.activities.write",
        /// Receive notifications pushed to the user over RPC (approved partners only).
        RpcNotificationsRead = "rpc.notifications.read",
        /// Read the user's voice settings over RPC (approved partners only).
        RpcVoiceRead = "rpc.voice.read",
        /// Update the user's voice settings over RPC (approved partners only).
        RpcVoiceWrite = "rpc.voice.write",
        /// Connect to voice on the user's behalf (approved partners only).
        Voice = "voice",
        /// Create a webhook returned in the `OAuth2` token response.
        WebhookIncoming = "webhook.incoming",
    }
}
