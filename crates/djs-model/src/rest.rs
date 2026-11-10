//! Request bodies and response types of the [REST API](https://docs.discord.com/developers/reference#http-api).
//!
//! These are plain data types; sending requests is the job of the `djs-rest` crate.
//!
//! discord.js counterpart: the `RESTPost…JSONBody` / `RESTGet…Result` types of
//! discord-api-types.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::message::MessageReference;

/// The response of [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot)
/// (`GET /gateway/bot`): where and how to connect to the gateway.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GatewayBot {
    /// The WSS URL to connect to, e.g. `wss://gateway.discord.gg`.
    pub url: CompactString,
    /// The recommended number of shards.
    pub shards: u32,
    /// Information on the current session start limit.
    pub session_start_limit: SessionStartLimit,
}

/// How many gateway sessions the bot may still start
/// ([session start limit object](https://docs.discord.com/developers/events/gateway#session-start-limit-object)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SessionStartLimit {
    /// The total number of session starts allowed per reset period (usually 1000 per day).
    pub total: u32,
    /// The remaining number of session starts.
    pub remaining: u32,
    /// Milliseconds until the limit resets.
    pub reset_after: u64,
    /// How many identify requests are allowed per 5 seconds (per rate-limit key).
    pub max_concurrency: u32,
}

/// The JSON body of [Create Message](https://docs.discord.com/developers/resources/message#create-message)
/// (`POST /channels/{channel.id}/messages`).
///
/// **Coverage:** Phase 1 supports text content, TTS and replies. Embeds, components, files,
/// allowed mentions, polls and the rest arrive in Phase 2/3.
///
/// discord.js counterpart: `MessageCreateOptions` / `RESTPostAPIChannelMessageJSONBody`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::CreateMessage;
///
/// let body = CreateMessage::from("Pong!");
/// assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"content":"Pong!"}"#);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CreateMessage {
    /// The message text (up to 2000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<CompactString>,
    /// Whether this is a text-to-speech message.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub tts: bool,
    /// The message to reply to or forward.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,
}

impl From<&str> for CreateMessage {
    fn from(content: &str) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<String> for CreateMessage {
    fn from(content: String) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<CompactString> for CreateMessage {
    fn from(content: CompactString) -> Self {
        Self {
            content: Some(content),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_bot_documented_example() {
        let json = r#"{
            "url": "wss://gateway.discord.gg/",
            "shards": 9,
            "session_start_limit": {
                "total": 1000,
                "remaining": 999,
                "reset_after": 14400000,
                "max_concurrency": 1
            }
        }"#;
        let info: GatewayBot = serde_json::from_str(json).unwrap();
        assert_eq!(info.url, "wss://gateway.discord.gg/");
        assert_eq!(info.shards, 9);
        assert_eq!(info.session_start_limit.max_concurrency, 1);
    }
}
