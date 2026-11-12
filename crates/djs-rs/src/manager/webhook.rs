//! [`WebhookManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, CachedMessage};
use djs_collection::Collection;
use djs_model::{
    id::{ChannelId, GuildId, MessageId, WebhookId},
    rest::{CreateWebhook, EditWebhookMessage, ExecuteWebhook, ModifyWebhook},
    webhook::Webhook,
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// Webhooks, by ID (D-237). Returned by [`Context::webhooks`].
///
/// The cache does not keep webhooks, so the [`djs-model`](djs_model) [`Webhook`] is returned;
/// the messages a webhook sends are cached like the bot's own. The methods taking a
/// `token` use the webhook's token instead of the bot's (and work for webhooks of other
/// applications); the others need `MANAGE_WEBHOOKS`. For a webhook without a bot, see
/// [`WebhookClient`](crate::webhook::WebhookClient).
///
/// discord.js counterpart: `TextChannel#createWebhook` / `#fetchWebhooks`,
/// `Guild#fetchWebhooks`, `Client#fetchWebhook` and the `Webhook` methods.
#[derive(Debug)]
pub struct WebhookManager<'a, B: Backend> {
    ctx: &'a Context<B>,
}

impl<B: Backend> Clone for WebhookManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for WebhookManager<'_, B> {}

impl<'a, B: Backend> WebhookManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>) -> Self {
        Self { ctx }
    }

    /// A webhook ([Get Webhook](djs_rest::Rest::get_webhook)).
    ///
    /// discord.js counterpart: `Client#fetchWebhook(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn fetch(
        self,
        webhook_id: WebhookId,
    ) -> Fetch<Webhook, impl Future<Output = Result<Webhook, Error>> + Send + 'a> {
        let ctx = self.ctx;
        Fetch::new(
            None,
            async move { ctx.rest().get_webhook(webhook_id).await },
        )
    }

    /// A webhook, authorized by its token ([Get Webhook with
    /// Token](djs_rest::Rest::get_webhook_with_token)).
    ///
    /// discord.js counterpart: `Client#fetchWebhook(id, token)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn fetch_with_token(
        self,
        webhook_id: WebhookId,
        token: &str,
    ) -> Result<Webhook, Error> {
        self.ctx
            .rest()
            .get_webhook_with_token(webhook_id, token)
            .await
    }

    /// Creates a webhook in a channel ([Create Webhook](djs_rest::Rest::create_webhook)).
    ///
    /// discord.js counterpart: `TextChannel#createWebhook`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn create(
        self,
        channel_id: ChannelId,
        body: &CreateWebhook,
        reason: Option<&str>,
    ) -> Result<Webhook, Error> {
        self.ctx
            .rest()
            .create_webhook(channel_id, body, reason)
            .await
    }

    /// The webhooks of a channel ([Get Channel
    /// Webhooks](djs_rest::Rest::get_channel_webhooks)).
    ///
    /// discord.js counterpart: `TextChannel#fetchWebhooks`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list_channel(
        self,
        channel_id: ChannelId,
    ) -> Result<Collection<WebhookId, Webhook>, Error> {
        let webhooks = self.ctx.rest().get_channel_webhooks(channel_id).await?;
        Ok(by_id(webhooks))
    }

    /// The webhooks of a guild ([Get Guild Webhooks](djs_rest::Rest::get_guild_webhooks)).
    ///
    /// discord.js counterpart: `Guild#fetchWebhooks`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list_guild(
        self,
        guild_id: GuildId,
    ) -> Result<Collection<WebhookId, Webhook>, Error> {
        let webhooks = self.ctx.rest().get_guild_webhooks(guild_id).await?;
        Ok(by_id(webhooks))
    }

    /// Edits a webhook: its name, avatar or channel ([Modify
    /// Webhook](djs_rest::Rest::modify_webhook)).
    ///
    /// discord.js counterpart: `Webhook#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        webhook_id: WebhookId,
        body: &ModifyWebhook,
        reason: Option<&str>,
    ) -> Result<Webhook, Error> {
        self.ctx
            .rest()
            .modify_webhook(webhook_id, body, reason)
            .await
    }

    /// Deletes a webhook ([Delete Webhook](djs_rest::Rest::delete_webhook)).
    ///
    /// discord.js counterpart: `Webhook#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, webhook_id: WebhookId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx.rest().delete_webhook(webhook_id, reason).await
    }

    /// Sends a message through a webhook ([Execute Webhook](djs_rest::Rest::execute_webhook_and_wait),
    /// waiting for the message); accepts anything that converts into an [`ExecuteWebhook`],
    /// such as `&str`. The message is cached.
    ///
    /// discord.js counterpart: `Webhook#send`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn send<'t>(
        self,
        webhook_id: WebhookId,
        token: &'t str,
        body: impl Into<ExecuteWebhook>,
    ) -> impl Future<Output = Result<Arc<CachedMessage>, Error>> + Send + 't
    where
        'a: 't,
    {
        let body = body.into();
        let ctx = self.ctx;
        async move {
            let message = ctx
                .rest()
                .execute_webhook_and_wait(webhook_id, token, &body)
                .await?;
            Ok(ctx.store_message(&message))
        }
    }

    /// A message the webhook sent ([Get Webhook Message](djs_rest::Rest::get_webhook_message);
    /// `thread` for one in a thread); it is cached.
    ///
    /// discord.js counterpart: `Webhook#fetchMessage`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn fetch_message(
        self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        thread: Option<ChannelId>,
    ) -> Result<Arc<CachedMessage>, Error> {
        let message = self
            .ctx
            .rest()
            .get_webhook_message(webhook_id, token, message_id, thread)
            .await?;
        Ok(self.ctx.store_message(&message))
    }

    /// Edits a message the webhook sent ([Edit Webhook
    /// Message](djs_rest::Rest::edit_webhook_message)); the result is cached.
    ///
    /// discord.js counterpart: `Webhook#editMessage`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit_message(
        self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        body: &EditWebhookMessage,
    ) -> Result<Arc<CachedMessage>, Error> {
        let message = self
            .ctx
            .rest()
            .edit_webhook_message(webhook_id, token, message_id, body)
            .await?;
        Ok(self.ctx.store_message(&message))
    }

    /// Deletes a message the webhook sent ([Delete Webhook
    /// Message](djs_rest::Rest::delete_webhook_message); `thread` for one in a thread).
    ///
    /// discord.js counterpart: `Webhook#deleteMessage`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete_message(
        self,
        webhook_id: WebhookId,
        token: &str,
        message_id: MessageId,
        thread: Option<ChannelId>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_webhook_message(webhook_id, token, message_id, thread)
            .await
    }
}

fn by_id(webhooks: Vec<Webhook>) -> Collection<WebhookId, Webhook> {
    webhooks
        .into_iter()
        .map(|webhook| (webhook.id, webhook))
        .collect()
}
