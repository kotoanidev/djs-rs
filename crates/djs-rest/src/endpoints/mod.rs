//! Typed methods for REST endpoints, one module per resource (D-082, D-091).

mod application_command;
mod audit_log;
mod auto_moderation;
mod channel;
mod emoji;
mod entitlement;
mod guild;
mod guild_scheduled_event;
mod guild_template;
mod interaction;
mod invite;
mod message;
mod poll;
mod sku;
mod soundboard;
mod stage_instance;
mod sticker;
mod subscription;
mod user;
mod webhook;

use djs_model::{permissions::Permissions, rest::GatewayBot};
use serde::Deserialize;

use crate::{Error, Request, Rest, Route};

/// The response of the computed permission endpoints (channel and guild), unwrapped by the
/// methods (D-110).
#[derive(Deserialize)]
struct PermissionsResponse {
    permissions: Permissions,
}

/// A list that Discord wraps in an object (`{"items": [...]}`), unwrapped by the methods.
#[derive(Deserialize)]
struct Items<T> {
    items: Vec<T>,
}

impl Rest {
    /// [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot):
    /// returns the gateway URL, the recommended shard count and the session start limit.
    ///
    /// discord.js counterpart: `REST#get(Routes.gatewayBot())`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_gateway_bot(&self) -> Result<GatewayBot, Error> {
        self.request(Request::new(Route::GetGatewayBot)).await
    }

    /// Sends `request` and discards the response body, whatever it is: for endpoints whose
    /// docs do not describe their response.
    pub(crate) async fn request_ignoring_body(&self, request: Request<'_>) -> Result<(), Error> {
        let _: serde::de::IgnoredAny = self.request(request).await?;
        Ok(())
    }
}
