//! [Webhook endpoints](https://docs.discord.com/developers/resources/webhook): creating and
//! managing webhooks, executing them, and their messages.
//!
//! Routes with the webhook's token in the path need no bot token and are sent without it
//! (D-121), so a client built without a token can use them. Execute Slack-Compatible and
//! GitHub-Compatible Webhook are not modelled (D-120).

use djs_model::{
    id::{ChannelId, GuildId, MessageId, WebhookId},
    message::Message,
    rest::{CreateWebhook, EditWebhookMessage, ExecuteWebhook, ModifyWebhook},
    webhook::Webhook,
};

use crate::client::boxed;
use crate::{
    Error, Request, Rest, Route,
    query::{Query, QueryWriter},
};

/// The query parameters of the webhook message endpoints (D-119), also used by the
/// interaction endpoints that edit webhook messages.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WebhookQuery {
    /// Execute Webhook only: whether to wait for and return the message.
    pub(crate) wait: Option<bool>,
    pub(crate) thread_id: Option<ChannelId>,
    pub(crate) with_components: Option<bool>,
}

impl WebhookQuery {
    /// The query fields of an edit body.
    pub(crate) const fn of_edit(body: &EditWebhookMessage) -> Self {
        Self {
            wait: None,
            thread_id: body.thread_id,
            with_components: body.with_components,
        }
    }

    /// The query fields of an execute body.
    const fn of_execute(body: &ExecuteWebhook, wait: bool) -> Self {
        Self {
            wait: if wait { Some(true) } else { None },
            thread_id: body.thread_id,
            with_components: body.with_components,
        }
    }

    /// Only a thread.
    const fn thread(thread_id: Option<ChannelId>) -> Self {
        Self {
            wait: None,
            thread_id,
            with_components: None,
        }
    }
}

impl Query for WebhookQuery {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("wait", self.wait.as_ref());
        query.optional("thread_id", self.thread_id.as_ref());
        query.optional("with_components", self.with_components.as_ref());
    }
}

/// Managing webhooks.
impl Rest {
    /// [Create Webhook](https://docs.discord.com/developers/resources/webhook#create-webhook):
    /// creates a webhook in a channel; its [`token`](Webhook::token) is in the response.
    /// Needs `MANAGE_WEBHOOKS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `TextChannel#createWebhook`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30007` at the maximum number of webhooks (15), `50035` for an
    /// invalid name.
    pub fn create_webhook<'a>(
        &'a self,
        channel_id: ChannelId,
        body: &'a CreateWebhook,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateWebhook { channel_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Get Channel Webhooks](https://docs.discord.com/developers/resources/webhook#get-channel-webhooks):
    /// returns the webhooks of a channel. Needs `MANAGE_WEBHOOKS`.
    ///
    /// discord.js counterpart: `TextChannel#fetchWebhooks`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_channel_webhooks(
        &self,
        channel_id: ChannelId,
    ) -> impl Future<Output = Result<Vec<Webhook>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetChannelWebhooks { channel_id }))
                .await
        })
    }

    /// [Get Guild Webhooks](https://docs.discord.com/developers/resources/webhook#get-guild-webhooks):
    /// returns the webhooks of a guild. Needs `MANAGE_WEBHOOKS`.
    ///
    /// discord.js counterpart: `Guild#fetchWebhooks`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_guild_webhooks(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Webhook>, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetGuildWebhooks { guild_id }))
                .await
        })
    }

    /// [Get Webhook](https://docs.discord.com/developers/resources/webhook#get-webhook):
    /// returns a webhook. Needs `MANAGE_WEBHOOKS`, unless the application owns the webhook.
    ///
    /// discord.js counterpart: `Client#fetchWebhook(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10015` Unknown Webhook.
    pub fn get_webhook(
        &self,
        webhook_id: WebhookId,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetWebhook { webhook_id }))
                .await
        })
    }

    /// [Get Webhook with Token](https://docs.discord.com/developers/resources/webhook#get-webhook-with-token):
    /// returns a webhook (without its [`user`](Webhook::user)), authenticated by its token.
    ///
    /// discord.js counterpart: `Client#fetchWebhook(id, token)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50027` Invalid Webhook Token.
    pub fn get_webhook_with_token<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetWebhookWithToken {
                webhook_id,
                webhook_token,
            }))
            .await
        })
    }

    /// [Modify Webhook](https://docs.discord.com/developers/resources/webhook#modify-webhook):
    /// changes a webhook's default name or avatar, or moves it to another channel. Needs
    /// `MANAGE_WEBHOOKS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Webhook#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_webhook<'a>(
        &'a self,
        webhook_id: WebhookId,
        body: &'a ModifyWebhook,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyWebhook { webhook_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Modify Webhook with Token](https://docs.discord.com/developers/resources/webhook#modify-webhook-with-token):
    /// like [`modify_webhook`](Self::modify_webhook), authenticated by the token; the
    /// [`channel_id`](ModifyWebhook::channel_id) cannot be changed.
    ///
    /// discord.js counterpart: `WebhookClient#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_webhook_with_token<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
        body: &'a ModifyWebhook,
    ) -> impl Future<Output = Result<Webhook, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyWebhookWithToken {
                    webhook_id,
                    webhook_token,
                })
                .json(body)?,
            )
            .await
        })
    }

    /// [Delete Webhook](https://docs.discord.com/developers/resources/webhook#delete-webhook):
    /// deletes a webhook for good. Needs `MANAGE_WEBHOOKS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Webhook#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10015` Unknown Webhook.
    pub fn delete_webhook<'a>(
        &'a self,
        webhook_id: WebhookId,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteWebhook { webhook_id }).reason(reason))
                .await
        })
    }

    /// [Delete Webhook with Token](https://docs.discord.com/developers/resources/webhook#delete-webhook-with-token):
    /// deletes a webhook, authenticated by its token.
    ///
    /// discord.js counterpart: `WebhookClient#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn delete_webhook_with_token<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteWebhookWithToken {
                webhook_id,
                webhook_token,
            }))
            .await
        })
    }
}

/// Executing webhooks and their messages.
impl Rest {
    /// [Execute Webhook](https://docs.discord.com/developers/resources/webhook#execute-webhook):
    /// sends a message through a webhook without waiting for it to be saved. Files of the
    /// body are uploaded with it.
    ///
    /// To get the created message (and errors of messages that could not be saved), use
    /// [`execute_webhook_and_wait`](Self::execute_webhook_and_wait) (D-119, like D-106).
    ///
    /// discord.js counterpart: `WebhookClient#send` / `Webhook#send`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(webhook_id: djs_model::id::WebhookId, token: &str) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::ExecuteWebhook;
    /// use djs_rest::Rest;
    ///
    /// // Webhook routes need no bot token (D-121).
    /// let rest = Rest::builder().build();
    /// let body = ExecuteWebhook::from("Deploy finished").username("CI");
    /// rest.execute_webhook(webhook_id, token, &body).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50027` Invalid Webhook Token.
    pub fn execute_webhook<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
        body: &'a ExecuteWebhook,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::of_execute(body, false);
            self.request_ignoring_body(
                Request::new(Route::ExecuteWebhook {
                    webhook_id,
                    webhook_token,
                })
                .query(&query)
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Execute Webhook](https://docs.discord.com/developers/resources/webhook#execute-webhook)
    /// with `wait=true`: sends a message through a webhook and returns it once saved.
    ///
    /// discord.js counterpart: `WebhookClient#send` (which always waits).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50006` for an empty message, `50027` Invalid Webhook Token.
    pub fn execute_webhook_and_wait<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
        body: &'a ExecuteWebhook,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::of_execute(body, true);
            self.call(
                Request::new(Route::ExecuteWebhook {
                    webhook_id,
                    webhook_token,
                })
                .query(&query)
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Get Webhook Message](https://docs.discord.com/developers/resources/webhook#get-webhook-message):
    /// returns a message the webhook sent (`thread_id`: the thread it is in).
    ///
    /// discord.js counterpart: `WebhookClient#fetchMessage`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10008` Unknown Message.
    pub fn get_webhook_message<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
        message_id: MessageId,
        thread_id: Option<ChannelId>,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::thread(thread_id);
            self.call(
                Request::new(Route::GetWebhookMessage {
                    webhook_id,
                    webhook_token,
                    message_id,
                })
                .query(&query),
            )
            .await
        })
    }

    /// [Edit Webhook Message](https://docs.discord.com/developers/resources/webhook#edit-webhook-message):
    /// edits a message the webhook sent; new files are uploaded and added to the kept
    /// attachments.
    ///
    /// discord.js counterpart: `WebhookClient#editMessage`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10008` Unknown Message.
    pub fn edit_webhook_message<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
        message_id: MessageId,
        body: &'a EditWebhookMessage,
    ) -> impl Future<Output = Result<Message, Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::of_edit(body);
            self.call(
                Request::new(Route::EditWebhookMessage {
                    webhook_id,
                    webhook_token,
                    message_id,
                })
                .query(&query)
                .json(body)?
                .files(&body.files),
            )
            .await
        })
    }

    /// [Delete Webhook Message](https://docs.discord.com/developers/resources/webhook#delete-webhook-message):
    /// deletes a message the webhook sent (`thread_id`: the thread it is in).
    ///
    /// discord.js counterpart: `WebhookClient#deleteMessage`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10008` Unknown Message.
    pub fn delete_webhook_message<'a>(
        &'a self,
        webhook_id: WebhookId,
        webhook_token: &'a str,
        message_id: MessageId,
        thread_id: Option<ChannelId>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            let query = WebhookQuery::thread(thread_id);
            self.call(
                Request::new(Route::DeleteWebhookMessage {
                    webhook_id,
                    webhook_token,
                    message_id,
                })
                .query(&query),
            )
            .await
        })
    }
}
