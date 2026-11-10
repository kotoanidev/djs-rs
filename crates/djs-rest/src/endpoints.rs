//! Typed methods for REST endpoints.

use djs_model::{
    id::ChannelId,
    message::Message,
    rest::{CreateMessage, GatewayBot},
};

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

    /// [Create Message](https://docs.discord.com/developers/resources/message#create-message):
    /// sends a message to a channel and returns it.
    ///
    /// Requires the `SEND_MESSAGES` permission in guild channels (and
    /// `SEND_MESSAGES_IN_THREADS` in threads).
    ///
    /// discord.js counterpart: `TextChannel#send`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]. Common API errors: `50001` Missing Access, `50013` Missing
    /// Permissions, `50035` Invalid Form Body.
    pub async fn create_message(
        &self,
        channel_id: ChannelId,
        body: &CreateMessage,
    ) -> Result<Message, Error> {
        self.request(Request::new(Route::CreateMessage { channel_id }).json(body)?)
            .await
    }
}
