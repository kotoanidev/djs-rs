//! [Applications](https://docs.discord.com/developers/resources/application): the apps (and
//! their bot users) registered in the Developer Portal.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    guild::PartialGuild,
    id::{ApplicationId, ChannelId, GenericId, GuildId, SkuId, UserId},
    macros::{int_enum, int_flags, str_enum},
    oauth2::Scope,
    permissions::Permissions,
    team::Team,
    user::User,
    util::ImageHash,
    webhook_event::WebhookEventType,
};

/// An [application](https://docs.discord.com/developers/resources/application#application-object).
///
/// discord.js counterpart: `ClientApplication` / `Application`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Application {
    /// The application's ID.
    pub id: ApplicationId,
    /// The application's name.
    pub name: CompactString,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The application's description.
    pub description: CompactString,
    /// RPC origin URLs, if RPC is enabled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rpc_origins: Vec<CompactString>,
    /// Whether anyone (not only the owner) can add the bot to guilds.
    pub bot_public: bool,
    /// Whether the bot only joins after the full `OAuth2` code grant flow.
    pub bot_require_code_grant: bool,
    /// The application's bot user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<Box<User>>,
    /// The URL of the terms of service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terms_of_service_url: Option<CompactString>,
    /// The URL of the privacy policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_policy_url: Option<CompactString>,
    /// The application's owner (for team-owned apps, a team pseudo-user).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<Box<User>>,
    /// The hex-encoded public key used to verify interaction requests.
    pub verify_key: CompactString,
    /// The team that owns the application.
    pub team: Option<Box<Team>>,
    /// The guild associated with the application (e.g. its support server).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The associated guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild: Option<Box<PartialGuild>>,
    /// For games sold on Discord, the "Game SKU".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_sku_id: Option<SkuId>,
    /// For games sold on Discord, the URL slug of the store page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<CompactString>,
    /// The default rich presence invite [cover image hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<ImageHash>,
    /// The application's public flags (legacy integer field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<ApplicationFlags>,
    /// The application's public flags, sent as a decimal string.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_flags_string"
    )]
    pub flags_new: Option<ApplicationFlags>,
    /// The approximate number of guilds the app is in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_guild_count: Option<u32>,
    /// The approximate number of users who installed the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_user_install_count: Option<u32>,
    /// The approximate number of users with an `OAuth2` authorization for the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_user_authorization_count: Option<u32>,
    /// The `OAuth2` redirect URIs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub redirect_uris: Vec<CompactString>,
    /// The [interactions endpoint URL](https://docs.discord.com/developers/interactions/receiving-and-responding#receiving-an-interaction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interactions_endpoint_url: Option<CompactString>,
    /// The role connection (linked roles) verification URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_connections_verification_url: Option<CompactString>,
    /// The URL that receives [webhook events](https://docs.discord.com/developers/events/webhook-events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_webhooks_url: Option<CompactString>,
    /// Whether webhook events are enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_webhooks_status: Option<EventWebhookStatus>,
    /// The webhook events the app subscribes to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_webhooks_types: Vec<WebhookEventType>,
    /// Up to 5 tags describing the app.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<CompactString>,
    /// The settings of the app's default in-app authorization link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_params: Option<InstallParams>,
    /// The default scopes and permissions per installation context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_types_config: Option<IntegrationTypesConfig>,
    /// The custom authorization URL, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_install_url: Option<CompactString>,
}

/// A partial [`Application`], as attached to invites (`target_application`) (D-051).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PartialApplication {
    /// The application's ID.
    pub id: ApplicationId,
    /// The application's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ImageHash>,
    /// The application's description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The [cover image hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<ImageHash>,
    /// The application's public flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<ApplicationFlags>,
}

int_enum! {
    /// [Where an app can be installed](https://docs.discord.com/developers/resources/application#application-object-application-integration-types).
    ///
    /// discord.js counterpart: `ApplicationIntegrationType`.
    #[doc(alias = "ApplicationIntegrationType")]
    pub enum IntegrationType: u8 {
        /// In guilds (servers).
        GuildInstall = 0,
        /// For users (the app follows the user everywhere).
        UserInstall = 1,
    }
}

/// The default install settings per installation context; the keys of
/// [`integration_types_config`](Application::integration_types_config).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrationTypesConfig {
    /// For guild installs (key `"0"`).
    #[serde(rename = "0", default, skip_serializing_if = "Option::is_none")]
    pub guild_install: Option<IntegrationTypeConfig>,
    /// For user installs (key `"1"`).
    #[serde(rename = "1", default, skip_serializing_if = "Option::is_none")]
    pub user_install: Option<IntegrationTypeConfig>,
}

/// The [install settings of one installation context](https://docs.discord.com/developers/resources/application#application-object-application-integration-type-configuration-object).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrationTypeConfig {
    /// The scopes and permissions of the context's default in-app authorization link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_install_params: Option<InstallParams>,
}

/// The [settings of an in-app authorization link](https://docs.discord.com/developers/resources/application#install-params-object).
///
/// discord.js counterpart: `ClientApplicationInstallParams`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InstallParams {
    /// The scopes to authorize.
    pub scopes: Vec<Scope>,
    /// The permissions requested for the bot role.
    pub permissions: Permissions,
}

int_enum! {
    /// [Whether webhook events are enabled](https://docs.discord.com/developers/resources/application#application-object-application-event-webhook-status).
    ///
    /// discord.js counterpart: `ApplicationWebhookEventStatus`.
    pub enum EventWebhookStatus: u8 {
        /// Disabled by the developer (the default).
        Disabled = 1,
        /// Enabled by the developer.
        Enabled = 2,
        /// Disabled by Discord, usually because of inactivity.
        DisabledByDiscord = 3,
    }
}

/// A running [Activity instance](https://docs.discord.com/developers/resources/application#get-application-activity-instance-activity-instance-object).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityInstance {
    /// The application.
    pub application_id: ApplicationId,
    /// The instance's ID.
    pub instance_id: CompactString,
    /// The unique ID of the launch.
    pub launch_id: GenericId,
    /// Where the instance runs.
    pub location: ActivityLocation,
    /// The users connected to the instance.
    pub users: Vec<UserId>,
}

/// [Where an Activity instance runs](https://docs.discord.com/developers/resources/application#get-application-activity-instance-activity-location-object).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityLocation {
    /// The location's ID.
    pub id: CompactString,
    /// Whether the location is a guild channel or a private channel.
    pub kind: ActivityLocationKind,
    /// The channel.
    pub channel_id: ChannelId,
    /// The guild, for guild channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
}

str_enum! {
    /// The [kind of an Activity location](https://docs.discord.com/developers/resources/application#get-application-activity-instance-activity-location-kind-enum).
    pub enum ActivityLocationKind {
        /// A guild channel.
        GuildChannel = "gc",
        /// A private channel (DM or group DM).
        PrivateChannel = "pc",
    }
}

int_flags! {
    /// [Flags of an application](https://docs.discord.com/developers/resources/application#application-object-application-flags).
    ///
    /// The `GATEWAY_*` flags tell whether the app may use the privileged
    /// [intents](crate::gateway::Intents): the plain flag is granted after approval (bots in 100
    /// or more servers), the `*_LIMITED` flag when it is enabled in the Developer Portal for a
    /// bot in fewer than 100 servers.
    ///
    /// discord.js counterpart: `ApplicationFlags` / `ApplicationFlagsBitField`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::application::ApplicationFlags;
    ///
    /// let flags: ApplicationFlags = serde_json::from_str("532480").unwrap();
    /// assert!(flags.contains(ApplicationFlags::GATEWAY_MESSAGE_CONTENT_LIMITED));
    /// ```
    pub struct ApplicationFlags: u64 {
        /// The app's embedded activity has been released.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const EMBEDDED_RELEASED = 1 << 1;
        /// The app can manage emojis.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const MANAGED_EMOJI = 1 << 2;
        /// The app's embedded activity supports in-app purchases.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const EMBEDDED_IAP = 1 << 3;
        /// The app can create group DMs.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const GROUP_DM_CREATE = 1 << 4;
        /// The app uses the [Auto Moderation API](https://docs.discord.com/developers/resources/auto-moderation)
        /// (shown as a badge on its profile).
        const APPLICATION_AUTO_MODERATION_RULE_CREATE_BADGE = 1 << 6;
        /// The app has connected to RPC.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const RPC_HAS_CONNECTED = 1 << 11;
        /// Approved for the privileged presence intent (bots in 100 or more servers).
        const GATEWAY_PRESENCE = 1 << 12;
        /// Presence intent enabled in the Developer Portal (bots in fewer than 100 servers).
        const GATEWAY_PRESENCE_LIMITED = 1 << 13;
        /// Approved for the privileged guild members intent (bots in 100 or more servers).
        const GATEWAY_GUILD_MEMBERS = 1 << 14;
        /// Guild members intent enabled in the Developer Portal (bots in fewer than 100
        /// servers).
        const GATEWAY_GUILD_MEMBERS_LIMITED = 1 << 15;
        /// Unusual growth of the app prevents its verification.
        const VERIFICATION_PENDING_GUILD_LIMIT = 1 << 16;
        /// The app is embedded in the Discord client (not publicly available).
        const EMBEDDED = 1 << 17;
        /// Approved for the privileged message content intent (bots in 100 or more servers).
        const GATEWAY_MESSAGE_CONTENT = 1 << 18;
        /// Message content intent enabled in the Developer Portal (bots in fewer than 100
        /// servers).
        const GATEWAY_MESSAGE_CONTENT_LIMITED = 1 << 19;
        /// The embedded app is made by Discord.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const EMBEDDED_FIRST_PARTY = 1 << 20;
        /// The app has registered global application commands (shown as a badge on its
        /// profile).
        const APPLICATION_COMMAND_BADGE = 1 << 23;
    }
}
