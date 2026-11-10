//! Payloads (commands) sent to the gateway.

use core::fmt;

use compact_str::CompactString;
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::gateway::{Intents, OpCode, ShardId};

/// A payload that can be sent to the gateway.
///
/// Each command knows its [opcode](Self::OPCODE). Wrap a command in [`CommandPayload`] to get
/// the `{"op": …, "d": …}` envelope the gateway expects.
pub trait GatewayCommand: Serialize {
    /// The opcode of the command.
    const OPCODE: OpCode;
}

/// The `{"op": …, "d": …}` envelope around a [`GatewayCommand`], ready to be serialized.
///
/// # Examples
///
/// ```
/// use djs_model::gateway::payload::{CommandPayload, Heartbeat};
///
/// let json = serde_json::to_string(&CommandPayload::new(&Heartbeat(Some(251)))).unwrap();
/// assert_eq!(json, r#"{"op":1,"d":251}"#);
/// ```
#[derive(Debug)]
pub struct CommandPayload<'a, T> {
    command: &'a T,
}

impl<'a, T: GatewayCommand> CommandPayload<'a, T> {
    /// Wraps a command.
    #[must_use]
    pub const fn new(command: &'a T) -> Self {
        Self { command }
    }
}

impl<T: GatewayCommand> Serialize for CommandPayload<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut payload = serializer.serialize_struct("CommandPayload", 2)?;
        payload.serialize_field("op", &T::OPCODE)?;
        payload.serialize_field("d", self.command)?;
        payload.end()
    }
}

/// A [Heartbeat](https://docs.discord.com/developers/events/gateway-events#heartbeat)
/// (opcode 1): the last sequence number received, or `None` if no dispatch has been received
/// yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Heartbeat(pub Option<u64>);

impl GatewayCommand for Heartbeat {
    const OPCODE: OpCode = OpCode::Heartbeat;
}

/// An [Identify](https://docs.discord.com/developers/events/gateway-events#identify)
/// (opcode 2): starts a new session.
///
/// Identifying is limited to `max_concurrency` sessions per 5 seconds and 1000 per day; exceeding
/// the daily limit resets the bot token.
///
/// The [`Debug`] implementation redacts the token.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Identify {
    /// The bot token, **without** the `Bot ` prefix.
    pub token: CompactString,
    /// Connection properties describing the client.
    pub properties: IdentifyProperties,
    /// The member count above which offline members are not sent in a guild's member list
    /// (50–250, Discord's default is 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<u8>,
    /// The shard this session handles. Omit for unsharded bots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard: Option<ShardId>,
    /// The gateway intents to subscribe to.
    pub intents: Intents,
}

impl GatewayCommand for Identify {
    const OPCODE: OpCode = OpCode::Identify;
}

impl fmt::Debug for Identify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identify")
            .field("token", &"<redacted>")
            .field("properties", &self.properties)
            .field("large_threshold", &self.large_threshold)
            .field("shard", &self.shard)
            .field("intents", &self.intents)
            .finish()
    }
}

/// The [connection properties](https://docs.discord.com/developers/events/gateway-events#identify-identify-connection-properties)
/// sent in [`Identify`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IdentifyProperties {
    /// The operating system, e.g. `linux`.
    pub os: CompactString,
    /// The library name.
    pub browser: CompactString,
    /// The library name.
    pub device: CompactString,
}

impl IdentifyProperties {
    /// Creates properties with the given library name and the current operating system
    /// ([`std::env::consts::OS`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::payload::IdentifyProperties;
    ///
    /// let properties = IdentifyProperties::new("djs-rs");
    /// assert_eq!(properties.browser, "djs-rs");
    /// assert_eq!(properties.os, std::env::consts::OS);
    /// ```
    #[must_use]
    pub fn new(library: &str) -> Self {
        Self {
            os: CompactString::const_new(std::env::consts::OS),
            browser: library.into(),
            device: library.into(),
        }
    }
}

/// A [Resume](https://docs.discord.com/developers/events/gateway-events#resume) (opcode 6):
/// continues a disconnected session and replays missed events.
///
/// The [`Debug`] implementation redacts the token.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Resume {
    /// The bot token, **without** the `Bot ` prefix.
    pub token: CompactString,
    /// The session ID from [`Ready`](super::Ready).
    pub session_id: CompactString,
    /// The last sequence number received.
    pub seq: u64,
}

impl GatewayCommand for Resume {
    const OPCODE: OpCode = OpCode::Resume;
}

impl fmt::Debug for Resume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resume")
            .field("token", &"<redacted>")
            .field("session_id", &self.session_id)
            .field("seq", &self.seq)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_json() {
        let identify = Identify {
            token: "secret".into(),
            properties: IdentifyProperties {
                os: "linux".into(),
                browser: "djs-rs".into(),
                device: "djs-rs".into(),
            },
            large_threshold: None,
            shard: Some(ShardId::ONE),
            intents: Intents::GUILDS | Intents::GUILD_MESSAGES,
        };
        let json = serde_json::to_string(&CommandPayload::new(&identify)).unwrap();
        assert_eq!(
            json,
            r#"{"op":2,"d":{"token":"secret","properties":{"os":"linux","browser":"djs-rs","device":"djs-rs"},"shard":[0,1],"intents":513}}"#
        );
        assert!(!format!("{identify:?}").contains("secret"));
    }

    #[test]
    fn resume_json() {
        let resume = Resume {
            token: "secret".into(),
            session_id: "abc".into(),
            seq: 1337,
        };
        let json = serde_json::to_string(&CommandPayload::new(&resume)).unwrap();
        assert_eq!(
            json,
            r#"{"op":6,"d":{"token":"secret","session_id":"abc","seq":1337}}"#
        );
        assert!(!format!("{resume:?}").contains("secret"));
    }

    #[test]
    fn first_heartbeat_is_null() {
        let json = serde_json::to_string(&CommandPayload::new(&Heartbeat(None))).unwrap();
        assert_eq!(json, r#"{"op":1,"d":null}"#);
    }
}
