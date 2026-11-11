//! Payloads received from the gateway.

use core::ops::{Deref, DerefMut};

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    application::ApplicationFlags, gateway::ShardId, guild::UnavailableGuild, id::ApplicationId,
    message::Message, user::CurrentUser,
};

/// The data of a [Hello](https://docs.discord.com/developers/events/gateway-events#hello) event
/// (opcode 10), sent immediately after the WebSocket connection opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Hello {
    /// How often to send heartbeats, in milliseconds.
    pub heartbeat_interval: u64,
}

/// The data of the [Ready](https://docs.discord.com/developers/events/gateway-events#ready)
/// dispatch event, sent after a successful identify.
///
/// It contains the information needed to resume the session later
/// ([`session_id`](Self::session_id) and [`resume_gateway_url`](Self::resume_gateway_url)).
///
/// discord.js counterpart: the `ClientReady` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Ready {
    /// The [API version](https://docs.discord.com/developers/reference#api-versioning).
    #[serde(rename = "v")]
    pub version: u8,
    /// The bot's own user.
    pub user: CurrentUser,
    /// The guilds the bot is in, all initially unavailable. With sharding, only the guilds of
    /// this shard are listed.
    pub guilds: Vec<UnavailableGuild>,
    /// The session ID, used for resuming.
    pub session_id: CompactString,
    /// The gateway URL to use when resuming this session.
    pub resume_gateway_url: CompactString,
    /// The shard this session belongs to, if one was sent when identifying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard: Option<ShardId>,
    /// The bot's application.
    pub application: ReadyApplication,
}

/// The partial application included in [`Ready`]: its ID and flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReadyApplication {
    /// The application's ID.
    pub id: ApplicationId,
    /// The application's flags, e.g. which privileged intents it may use.
    #[serde(default)]
    pub flags: ApplicationFlags,
}

/// The data of the [Message Create](https://docs.discord.com/developers/events/gateway-events#message-create)
/// dispatch event: a new message was sent.
///
/// Dereferences to the contained [`Message`].
///
/// discord.js counterpart: the `MessageCreate` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageCreate(pub Message);

impl Deref for MessageCreate {
    type Target = Message;

    fn deref(&self) -> &Message {
        &self.0
    }
}

impl DerefMut for MessageCreate {
    fn deref_mut(&mut self) -> &mut Message {
        &mut self.0
    }
}

impl From<MessageCreate> for Message {
    fn from(event: MessageCreate) -> Self {
        event.0
    }
}
