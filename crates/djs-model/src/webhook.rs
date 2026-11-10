//! [Webhooks](https://docs.discord.com/developers/resources/webhook): post messages to a
//! channel without a bot user.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{ApplicationId, ChannelId, GuildId, WebhookId},
    macros::int_enum,
    user::User,
    util::ImageHash,
};

/// A [webhook](https://docs.discord.com/developers/resources/webhook#webhook-object).
///
/// The `Debug` output hides the [`token`](Self::token) (C-011).
///
/// discord.js counterpart: `Webhook`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Webhook {
    /// The webhook's ID.
    pub id: WebhookId,
    /// What kind of webhook this is.
    #[serde(rename = "type")]
    pub kind: WebhookType,
    /// The guild the webhook belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel the webhook posts to.
    pub channel_id: Option<ChannelId>,
    /// Who created the webhook (absent when the webhook is fetched with its token).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,
    /// The default name of the webhook's messages.
    pub name: Option<CompactString>,
    /// The default [avatar hash](https://docs.discord.com/developers/reference#image-formatting).
    pub avatar: Option<ImageHash>,
    /// The secret token used to execute the webhook (incoming webhooks only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<CompactString>,
    /// The application that created the webhook.
    pub application_id: Option<ApplicationId>,
    /// The guild of the followed channel (channel follower webhooks only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_guild: Option<SourceGuild>,
    /// The followed channel (channel follower webhooks only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_channel: Option<SourceChannel>,
    /// The URL to execute the webhook (from the `webhook.incoming` `OAuth2` flow).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<CompactString>,
}

impl core::fmt::Debug for Webhook {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Webhook")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .field("user", &self.user)
            .field("name", &self.name)
            .field("avatar", &self.avatar)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("application_id", &self.application_id)
            .field("source_guild", &self.source_guild)
            .field("source_channel", &self.source_channel)
            .field("url", &self.url.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

int_enum! {
    /// The [kind of a webhook](https://docs.discord.com/developers/resources/webhook#webhook-object-webhook-types).
    ///
    /// discord.js counterpart: `WebhookType`.
    pub enum WebhookType: u8 {
        /// Posts messages with a generated token.
        Incoming = 1,
        /// Internal webhook that posts messages of a followed announcement channel.
        ChannelFollower = 2,
        /// Used by interactions.
        Application = 3,
    }
}

/// The guild of the channel a channel follower webhook follows.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SourceGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's name.
    pub name: CompactString,
    /// The guild's [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
}

/// The channel a channel follower webhook follows.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SourceChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The channel's name.
    pub name: CompactString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_redacted_in_debug() {
        let webhook: Webhook = serde_json::from_str(
            r#"{"id":"1","type":1,"channel_id":"2","name":"hook","avatar":null,
                "token":"secret-token","application_id":null}"#,
        )
        .unwrap();
        let debug = format!("{webhook:?}");
        assert!(!debug.contains("secret-token"));
        assert!(debug.contains("<redacted>"));
        assert_eq!(webhook.kind, WebhookType::Incoming);
    }
}
