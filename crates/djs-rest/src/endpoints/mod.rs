//! Typed methods for REST endpoints, one module per resource (D-082, D-091).

mod application;
mod application_command;
mod application_identity_profile;
mod application_role_connection_metadata;
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
mod lobby;
mod message;
mod oauth2;
mod poll;
mod sku;
mod soundboard;
mod stage_instance;
mod sticker;
mod subscription;
mod user;
mod voice;
mod webhook;

use djs_model::{
    permissions::Permissions,
    rest::{Gateway, GatewayBot},
};
use serde::Deserialize;

use crate::client::boxed;
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
    /// [Get Gateway](https://docs.discord.com/developers/events/gateway#get-gateway): returns
    /// the gateway URL. Needs no token; bots should use
    /// [`get_gateway_bot`](Self::get_gateway_bot), which also returns the shard count and the
    /// session start limit.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_gateway(&self) -> impl Future<Output = Result<Gateway, Error>> + Send + '_ {
        boxed(async move { self.call(Request::new(Route::GetGateway)).await })
    }

    /// [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot):
    /// returns the gateway URL, the recommended shard count and the session start limit.
    ///
    /// discord.js counterpart: `REST#get(Routes.gatewayBot())`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_gateway_bot(&self) -> impl Future<Output = Result<GatewayBot, Error>> + Send + '_ {
        boxed(async move { self.call(Request::new(Route::GetGatewayBot)).await })
    }

    /// Sends `request` and discards the response body, whatever it is: for endpoints whose
    /// docs do not describe their response.
    pub(crate) async fn request_ignoring_body(&self, request: Request<'_>) -> Result<(), Error> {
        let _: serde::de::IgnoredAny = self.call(request).await?;
        Ok(())
    }
}
