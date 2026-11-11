//! Typed methods for REST endpoints, one module per resource (D-082, D-091).

mod message;

use djs_model::rest::GatewayBot;

use crate::{Error, Request, Rest, Route};

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
}
