//! Webhooks without a bot: [`WebhookClient`] (D-238), and the errors of the webhook methods.
//!
//! ```no_run
//! use djs_rs::WebhookClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let webhook = WebhookClient::from_url(&std::env::var("WEBHOOK_URL")?)?;
//! let message = webhook.send("Deployed!").await?;
//! println!("sent {}", message.id);
//! # Ok(())
//! # }
//! ```

use core::fmt;
use std::future::Future;

use djs_model::{
    id::{ChannelId, MessageId, WebhookId},
    message::Message,
    rest::{EditWebhookMessage, ExecuteWebhook, ModifyWebhook},
    webhook::Webhook,
};
use djs_rest::{Error, Rest};

/// A webhook used through its ID and token, without a bot token or a gateway connection:
/// sending messages to a channel from a script, a CI job or a server.
///
/// The token is the webhook's password: keep it out of logs and repositories. `Debug`
/// redacts it.
///
/// discord.js counterpart: `WebhookClient`.
#[derive(Clone)]
pub struct WebhookClient {
    rest: Rest,
    id: WebhookId,
    token: String,
}

impl WebhookClient {
    /// A client for the webhook `id` with its `token`.
    #[must_use]
    pub fn new(id: WebhookId, token: impl Into<String>) -> Self {
        Self {
            rest: Rest::builder().build(),
            id,
            token: token.into(),
        }
    }

    /// A client for a webhook URL as Discord shows it,
    /// `https://discord.com/api/webhooks/{id}/{token}` (D-241). The `discordapp.com`, `canary.`
    /// and `ptb.` hosts and a versioned path (`/api/v10/webhooks/…`) are accepted too; a query
    /// string is ignored.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWebhookUrl`] if `url` is not a Discord webhook URL.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rs::{WebhookClient, model::id::WebhookId};
    ///
    /// let webhook = WebhookClient::from_url("https://discord.com/api/webhooks/123/abc-DEF_9").unwrap();
    /// assert_eq!(webhook.id(), WebhookId::new(123));
    /// assert!(WebhookClient::from_url("https://example.com/api/webhooks/123/abc").is_err());
    /// ```
    pub fn from_url(url: &str) -> Result<Self, InvalidWebhookUrl> {
        let (id, token) = parse_url(url).ok_or(InvalidWebhookUrl(()))?;
        Ok(Self::new(id, token))
    }

    /// Sends the requests through `rest` instead of a default client, for example one with a
    /// proxy or another API base. A bot token it holds is not sent: webhook routes are
    /// authorized by the webhook's token.
    #[must_use]
    pub fn rest(mut self, rest: Rest) -> Self {
        self.rest = rest;
        self
    }

    /// The webhook's ID.
    #[must_use]
    pub const fn id(&self) -> WebhookId {
        self.id
    }

    /// Sends a message ([Execute Webhook](djs_rest::Rest::execute_webhook_and_wait), waiting
    /// for the message); accepts anything that converts into an [`ExecuteWebhook`], such as
    /// `&str`.
    ///
    /// discord.js counterpart: `WebhookClient#send`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn send(
        &self,
        body: impl Into<ExecuteWebhook>,
    ) -> impl Future<Output = Result<Message, Error>> + Send + '_ {
        let body = body.into();
        async move {
            self.rest
                .execute_webhook_and_wait(self.id, &self.token, &body)
                .await
        }
    }

    /// A message the webhook sent (`thread` for one in a thread).
    ///
    /// discord.js counterpart: `WebhookClient#fetchMessage`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn fetch_message(
        &self,
        message_id: MessageId,
        thread: Option<ChannelId>,
    ) -> Result<Message, Error> {
        self.rest
            .get_webhook_message(self.id, &self.token, message_id, thread)
            .await
    }

    /// Edits a message the webhook sent.
    ///
    /// discord.js counterpart: `WebhookClient#editMessage`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit_message(
        &self,
        message_id: MessageId,
        body: &EditWebhookMessage,
    ) -> Result<Message, Error> {
        self.rest
            .edit_webhook_message(self.id, &self.token, message_id, body)
            .await
    }

    /// Deletes a message the webhook sent (`thread` for one in a thread).
    ///
    /// discord.js counterpart: `WebhookClient#deleteMessage`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete_message(
        &self,
        message_id: MessageId,
        thread: Option<ChannelId>,
    ) -> Result<(), Error> {
        self.rest
            .delete_webhook_message(self.id, &self.token, message_id, thread)
            .await
    }

    /// The webhook itself ([Get Webhook with Token](djs_rest::Rest::get_webhook_with_token)).
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn fetch(&self) -> Result<Webhook, Error> {
        self.rest.get_webhook_with_token(self.id, &self.token).await
    }

    /// Changes the webhook's name or avatar (not its channel, which needs a bot).
    ///
    /// discord.js counterpart: `WebhookClient#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(&self, body: &ModifyWebhook) -> Result<Webhook, Error> {
        self.rest
            .modify_webhook_with_token(self.id, &self.token, body)
            .await
    }

    /// Deletes the webhook; the client is unusable afterwards.
    ///
    /// discord.js counterpart: `WebhookClient#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(&self) -> Result<(), Error> {
        self.rest
            .delete_webhook_with_token(self.id, &self.token)
            .await
    }
}

impl fmt::Debug for WebhookClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookClient")
            .field("id", &self.id)
            .field("token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// `https://{canary.|ptb.}discord{app}.com/api{/vN}/webhooks/{id}/{token}`, ignoring a query
/// string, a fragment and a trailing slash.
fn parse_url(url: &str) -> Option<(WebhookId, &str)> {
    let url = url.strip_prefix("https://")?;
    let url = url.split(['?', '#']).next()?;
    let (host, path) = url.split_once('/')?;
    let host = host
        .strip_prefix("canary.")
        .or_else(|| host.strip_prefix("ptb."))
        .unwrap_or(host);
    if host != "discord.com" && host != "discordapp.com" {
        return None;
    }
    let path = path.strip_prefix("api/")?;
    let path = match path.split_once('/') {
        Some((version, rest))
            if version.len() > 1
                && version.starts_with('v')
                && version[1..].bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            rest
        }
        _ => path,
    };
    let path = path.strip_prefix("webhooks/")?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let (id, token) = path.split_once('/')?;
    let id = WebhookId::new_checked(id.parse().ok()?)?;
    let valid = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_';
    (!token.is_empty() && token.bytes().all(valid)).then_some((id, token))
}

/// The URL given to [`WebhookClient::from_url`] is not a Discord webhook URL (D-241).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a Discord webhook URL (https://discord.com/api/webhooks/{{id}}/{{token}})")]
pub struct InvalidWebhookUrl(());

/// An error of a webhook method that needs the webhook's token (D-240).
///
/// discord.js counterpart: the `WebhookTokenUnavailable` error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WebhookError {
    /// The webhook has no token: channel-follower and application webhooks cannot be used to
    /// send messages.
    #[error("the webhook has no token")]
    MissingToken,
    /// The REST request failed.
    #[error("a REST request failed")]
    Rest(#[from] Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_urls_are_parsed() {
        let id = WebhookId::new(123);
        for url in [
            "https://discord.com/api/webhooks/123/abc-DEF_9",
            "https://discord.com/api/webhooks/123/abc-DEF_9/",
            "https://discordapp.com/api/webhooks/123/abc-DEF_9",
            "https://canary.discord.com/api/webhooks/123/abc-DEF_9",
            "https://ptb.discord.com/api/v10/webhooks/123/abc-DEF_9",
            "https://discord.com/api/webhooks/123/abc-DEF_9?thread_id=5",
        ] {
            assert_eq!(parse_url(url), Some((id, "abc-DEF_9")), "{url}");
        }
        for url in [
            "http://discord.com/api/webhooks/123/abc",
            "https://example.com/api/webhooks/123/abc",
            "https://discord.com.evil.example/api/webhooks/123/abc",
            "https://discord.com/webhooks/123/abc",
            "https://discord.com/api/webhooks/123",
            "https://discord.com/api/webhooks/123/",
            "https://discord.com/api/webhooks/0/abc",
            "https://discord.com/api/webhooks/abc/abc",
            "https://discord.com/api/webhooks/123/ab%2Fc",
            "https://discord.com/api/webhooks/123/abc/extra",
            "https://discord.com/api/vx/webhooks/123/abc",
            "",
        ] {
            assert_eq!(parse_url(url), None, "{url}");
        }
    }

    #[test]
    fn the_token_is_redacted() {
        let client = WebhookClient::new(WebhookId::new(1), "secret-token-value");
        let debug = format!("{client:?}");
        assert!(!debug.contains("secret"), "{debug}");
    }
}
