//! [OAuth2](https://docs.discord.com/developers/topics/oauth2): scopes an application can be
//! authorized for, access tokens and the information of an authorization.

use core::fmt;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    application::PartialApplication, guild::Guild, macros::str_enum, user::User, util::Timestamp,
    webhook::Webhook,
};

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

/// The response of the [token endpoint](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant-access-token-response):
/// an access token for the user who authorized the application.
///
/// Send it with a client built with `RestBuilder::bearer_token` to act for the user. The
/// tokens are redacted in the `Debug` output (C-011).
///
/// discord.js counterpart: none (discord-api-types: `RESTPostOAuth2AccessTokenResult`).
///
/// # Examples
///
/// ```
/// use djs_model::oauth2::{AccessTokenResponse, Scope};
///
/// let response: AccessTokenResponse = serde_json::from_str(
///     r#"{"access_token": "user-access-token", "token_type": "Bearer",
///         "expires_in": 604800, "refresh_token": "user-refresh-token",
///         "scope": "identify guilds"}"#,
/// )?;
/// assert_eq!(
///     response.scopes().collect::<Vec<_>>(),
///     [Scope::Identify, Scope::Guilds]
/// );
/// assert!(!format!("{response:?}").contains("user-access-token"));
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccessTokenResponse {
    /// The access token.
    pub access_token: String,
    /// How the token is sent (`Bearer`).
    pub token_type: CompactString,
    /// How many seconds the access token is valid for.
    pub expires_in: u64,
    /// The token to get a new access token with once this one expires. Client credentials
    /// grants do not get one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// The authorized scopes, separated by spaces; see [`scopes`](Self::scopes).
    pub scope: CompactString,
    /// For the `bot` scope with the authorization code grant: the guild the bot was added
    /// to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild: Option<Box<Guild>>,
    /// For the `webhook.incoming` scope: the webhook created in the channel the user picked,
    /// with its token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook: Option<Box<Webhook>>,
}

impl AccessTokenResponse {
    /// The authorized scopes, parsed from [`scope`](Self::scope).
    pub fn scopes(&self) -> impl Iterator<Item = Scope> + '_ {
        self.scope.split_ascii_whitespace().map(Scope::from)
    }
}

impl fmt::Debug for AccessTokenResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessTokenResponse")
            .field("access_token", &"<redacted>")
            .field("token_type", &self.token_type)
            .field("expires_in", &self.expires_in)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("scope", &self.scope)
            .field("guild", &self.guild)
            .field("webhook", &self.webhook)
            .finish()
    }
}

/// The response of [Get Current Authorization Information](https://docs.discord.com/developers/topics/oauth2#get-current-authorization-information):
/// what an access token is authorized for.
///
/// discord.js counterpart: none (discord-api-types:
/// `RESTGetAPIOAuth2CurrentAuthorizationResult`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentAuthorizationInformation {
    /// The application the token belongs to.
    pub application: PartialApplication,
    /// The scopes the user authorized.
    pub scopes: Vec<Scope>,
    /// When the access token expires.
    pub expires: Timestamp,
    /// The user who authorized, with the `identify` scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,
}
