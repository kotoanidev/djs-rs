//! Bodies of the [application endpoints](https://docs.discord.com/developers/resources/application).

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    application::{ApplicationFlags, EventWebhookStatus, InstallParams, IntegrationTypesConfig},
    util::{ImageData, Patch},
    webhook_event::WebhookEventType,
};

/// The body of [Edit Current Application](https://docs.discord.com/developers/resources/application#edit-current-application).
///
/// Every field is optional; only the fields that are set change.
///
/// discord.js counterpart: the argument of `ClientApplication#edit`.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     application::InstallParams,
///     oauth2::Scope,
///     permissions::Permissions,
///     rest::EditCurrentApplication,
/// };
///
/// let mut body = EditCurrentApplication::default();
/// body.description = Some("A bot written in Rust".into());
/// body.install_params = Some(InstallParams::new(
///     [Scope::Bot, Scope::ApplicationsCommands],
///     Permissions::SEND_MESSAGES,
/// ));
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"description":"A bot written in Rust","install_params":{"scopes":["bot","applications.commands"],"permissions":"2048"}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct EditCurrentApplication {
    /// The custom authorization URL, used when the app's install link is a custom URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_install_url: Option<CompactString>,
    /// The description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// The URL that users are sent to to verify their linked roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_connections_verification_url: Option<CompactString>,
    /// The settings of the in-app authorization link, used when the install link is the one
    /// Discord provides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_params: Option<InstallParams>,
    /// The default scopes and permissions of each installation context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types_config: Option<IntegrationTypesConfig>,
    /// The public flags. Only `GATEWAY_PRESENCE_LIMITED`, `GATEWAY_GUILD_MEMBERS_LIMITED` and
    /// `GATEWAY_MESSAGE_CONTENT_LIMITED` can be changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ApplicationFlags>,
    /// The new icon; [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub icon: Patch<ImageData>,
    /// The new default rich presence invite cover image; [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub cover_image: Patch<ImageData>,
    /// The URL that receives interactions over HTTP instead of the gateway. Discord checks
    /// that it answers its validation requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions_endpoint_url: Option<CompactString>,
    /// Tags describing the app (up to 5, 20 characters each).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<CompactString>>,
    /// The URL that receives [webhook events](crate::webhook_event).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_url: Option<CompactString>,
    /// Enables ([`Enabled`](EventWebhookStatus::Enabled)) or disables
    /// ([`Disabled`](EventWebhookStatus::Disabled)) webhook events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_status: Option<EventWebhookStatus>,
    /// The webhook events to receive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_types: Option<Vec<WebhookEventType>>,
}
