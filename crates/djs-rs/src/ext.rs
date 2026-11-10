//! Methods on Discord structures (discord.js-style "structures" behaviour).
//!
//! djs-rs keeps the data types of `djs-model` plain and adds behaviour through extension
//! traits that take the handler [`Context`] explicitly — `msg.reply(&ctx, "Pong!")` — instead of
//! storing a client reference inside every structure as discord.js does. This keeps structures
//! small, `Clone` cheap and free of reference cycles.
//!
//! Import the traits with [`prelude`](crate::prelude).

use std::future::Future;

use djs_model::{
    message::{Message, MessageReference},
    rest::CreateMessage,
};

use crate::Context;

/// Actions on a [`Message`].
///
/// This trait is sealed; it is implemented for [`Message`] (and is therefore usable on
/// anything that dereferences to one, such as the `MessageCreate` event payload).
pub trait MessageExt: sealed::Sealed {
    /// Replies to this message in the same channel.
    ///
    /// The reply references this message (Discord shows it as a reply). Accepts anything that
    /// converts into a [`CreateMessage`], such as `&str` or `String`.
    ///
    /// discord.js counterpart: `Message#reply`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the message could not be sent (see
    /// [`Rest::create_message`](djs_rest::Rest::create_message)).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// client.on(Events::MessageCreate, |ctx, msg| async move {
    ///     if msg.content == "!ping" {
    ///         msg.reply(&ctx, "Pong!").await?;
    ///     }
    ///     Ok(())
    /// });
    /// ```
    fn reply(
        &self,
        ctx: &Context,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Message, djs_rest::Error>> + Send;
}

impl MessageExt for Message {
    fn reply(
        &self,
        ctx: &Context,
        body: impl Into<CreateMessage>,
    ) -> impl Future<Output = Result<Message, djs_rest::Error>> + Send {
        let mut body = body.into();
        body.message_reference = Some(MessageReference::reply(self.id));
        let rest = ctx.rest().clone();
        let channel_id = self.channel_id;
        async move { rest.create_message(channel_id, &body).await }
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for djs_model::message::Message {}
}
