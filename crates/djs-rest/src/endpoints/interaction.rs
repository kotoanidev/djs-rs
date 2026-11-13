//! [Interaction endpoints](https://docs.discord.com/developers/interactions/receiving-and-responding#endpoints):
//! the initial response, the original response and follow-up messages.
//!
//! None of them count towards the global rate limit, and initial responses are never queued
//! behind one another (they must arrive within 3 seconds). Interaction tokens are valid for
//! 15 minutes.

use djs_model::{
    id::{ApplicationId, InteractionId, MessageId},
    interaction::{InteractionCallbackResponse, InteractionResponse},
    message::Message,
    rest::{CreateFollowupMessage, EditWebhookMessage},
};

use super::webhook::WebhookQuery;
use crate::client::boxed;
use crate::{
    Error, Request, Rest, Route,
    query::{Query, QueryWriter},
};

/// `?with_response=true` of Create Interaction Response (D-106).
#[derive(Debug)]
struct WithResponse;

impl Query for WithResponse {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.pair("with_response", &true);
    }
}

impl Rest {
    /// [Create Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#create-interaction-response):
    /// the initial response to an interaction, within 3 seconds of receiving it. Files of
    /// the response's message are uploaded with it.
    ///
    /// To get the callback (e.g. the created message), use
    /// [`create_interaction_response_with_response`](Self::create_interaction_response_with_response).
    ///
    /// discord.js counterpart: `reply`, `deferReply`, `update`, `deferUpdate`, `showModal`,
    /// `respond` of the interaction classes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, interaction: djs_model::interaction::Interaction) -> Result<(), djs_rest::Error> {
    /// use djs_model::interaction::{InteractionResponse, MessageCallbackData};
    ///
    /// let reply = InteractionResponse::ChannelMessageWithSource(Box::new(
    ///     MessageCallbackData::from("Pong!"),
    /// ));
    /// rest.create_interaction_response(interaction.id, &interaction.token, &reply)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40060` if the interaction was already acknowledged, `10062`
    /// Unknown Interaction once it expired.
    pub fn create_interaction_response<'a>(
        &'a self,
        interaction_id: InteractionId,
        interaction_token: &'a str,
        response: &'a InteractionResponse,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(callback(interaction_id, interaction_token, response)?)
                .await
        })
    }

    /// [Create Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#create-interaction-response)
    /// with `with_response`: like
    /// [`create_interaction_response`](Self::create_interaction_response), but returns the
    /// [callback](InteractionCallbackResponse) (D-106), which holds e.g. the created message.
    ///
    /// discord.js counterpart: the same methods with `withResponse: true`.
    ///
    /// # Errors
    ///
    /// See [`create_interaction_response`](Self::create_interaction_response).
    pub fn create_interaction_response_with_response<'a>(
        &'a self,
        interaction_id: InteractionId,
        interaction_token: &'a str,
        response: &'a InteractionResponse,
    ) -> impl Future<Output = Result<InteractionCallbackResponse, Error>> + Send + 'a {
        boxed(async move {
            self.call(callback(interaction_id, interaction_token, response)?.query(&WithResponse))
                .await
        })
    }

    /// [Get Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#get-original-interaction-response):
    /// returns the message of the initial response.
    ///
    /// discord.js counterpart: `fetchReply`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_original_interaction_response<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetOriginalInteractionResponse {
                application_id,
                interaction_token,
            }))
            .await
        })
    }

    /// [Edit Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#edit-original-interaction-response):
    /// edits the message of the initial response (also the way to answer a deferred
    /// response).
    ///
    /// discord.js counterpart: `editReply`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn edit_original_interaction_response<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
        body: &'a EditWebhookMessage,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::of_edit(body);
            self.call(
                Request::new(Route::EditOriginalInteractionResponse {
                    application_id,
                    interaction_token,
                })
                .query(&query)
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Delete Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#delete-original-interaction-response):
    /// deletes the message of the initial response.
    ///
    /// discord.js counterpart: `deleteReply`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_original_interaction_response<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteOriginalInteractionResponse {
                application_id,
                interaction_token,
            }))
            .await
        })
    }

    /// [Create Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#create-followup-message):
    /// sends another message for the interaction and returns it.
    ///
    /// Apps installed only for the user (not in the guild) may send at most 5 follow-ups per
    /// interaction.
    ///
    /// discord.js counterpart: `followUp`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40094` once the maximum number of follow-ups is reached.
    pub fn create_followup_message<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
        body: &'a CreateFollowupMessage,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateFollowupMessage {
                    application_id,
                    interaction_token,
                })
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Get Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#get-followup-message):
    /// returns a follow-up message.
    ///
    /// discord.js counterpart: `fetchReply` with a message ID.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_followup_message<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
        message_id: MessageId,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetFollowupMessage {
                application_id,
                interaction_token,
                message_id,
            }))
            .await
        })
    }

    /// [Edit Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#edit-followup-message):
    /// edits a follow-up message.
    ///
    /// discord.js counterpart: `editReply` with a message ID / `InteractionWebhook#editMessage`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn edit_followup_message<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
        message_id: MessageId,
        body: &'a EditWebhookMessage,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::of_edit(body);
            self.call(
                Request::new(Route::EditFollowupMessage {
                    application_id,
                    interaction_token,
                    message_id,
                })
                .query(&query)
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Delete Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#delete-followup-message):
    /// deletes a follow-up message.
    ///
    /// discord.js counterpart: `deleteReply` with a message ID.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_followup_message<'a>(
        &'a self,
        application_id: ApplicationId,
        interaction_token: &'a str,
        message_id: MessageId,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteFollowupMessage {
                application_id,
                interaction_token,
                message_id,
            }))
            .await
        })
    }
}

/// The request of Create Interaction Response, with the files of the response's message.
fn callback<'a>(
    interaction_id: InteractionId,
    interaction_token: &'a str,
    response: &'a InteractionResponse,
) -> Result<Request<'a>, Error> {
    Ok(Request::new(Route::CreateInteractionResponse {
        interaction_id,
        interaction_token,
    })
    .json(response)?
    .files(response.files()))
}
