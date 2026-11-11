//! Typed methods for REST endpoints, one module per resource (D-082, D-091).

mod application_command;
mod channel;
mod guild;
mod interaction;
mod message;
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
