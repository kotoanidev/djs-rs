//! Payloads received from the gateway: the connection events and the data of every dispatch
//! event.
//!
//! Each dispatch event has its own payload type (D-073). An event whose data is exactly one
//! object is a newtype that dereferences to it (e.g. [`ChannelCreate`] →
//! [`Channel`](crate::channel::Channel)); an event with extra fields holds the object and the
//! extras, decoded in one pass.

mod channel;
mod guild;
mod message;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

pub use self::{channel::*, guild::*, message::*};
use crate::{
    application::ApplicationFlags,
    gateway::{OpCode, ShardId},
    guild::UnavailableGuild,
    id::{ApplicationId, GuildId},
    user::CurrentUser,
};

/// Defines a dispatch event whose data is exactly one object: a transparent newtype that
/// dereferences to it.
macro_rules! newtype_event {
    ($(#[$meta:meta])* $name:ident($inner:ty)) => {
        $(#[$meta])*
        ///
        #[doc = concat!("Dereferences to the contained [`", stringify!($inner), "`].")]
        #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $inner);

        impl core::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &$inner {
                &self.0
            }
        }

        impl core::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut $inner {
                &mut self.0
            }
        }

        impl From<$name> for $inner {
            fn from(event: $name) -> Self {
                event.0
            }
        }
    };
}

use newtype_event;

/// Collects the `guild_id` that some events add to an object (D-073).
#[derive(Default)]
struct GuildIdExtra {
    guild_id: Option<GuildId>,
}

impl<'de> crate::serde_util::extras::Extras<'de> for GuildIdExtra {
    fn visit<A: serde::de::MapAccess<'de>>(
        &mut self,
        key: &str,
        map: &mut A,
    ) -> Result<crate::serde_util::extras::Visit, A::Error> {
        if key == "guild_id" {
            self.guild_id = Some(map.next_value()?);
            Ok(crate::serde_util::extras::Visit::Consumed)
        } else {
            Ok(crate::serde_util::extras::Visit::Pass)
        }
    }
}

/// Defines a dispatch event whose data is an object plus a `guild_id` (D-073): a struct with
/// both that dereferences to the object, decoded in one pass.
macro_rules! guild_id_event {
    ($(#[$meta:meta])* $name:ident { $(#[$field_meta:meta])* $field:ident: $inner:ty }) => {
        $(#[$meta])*
        ///
        #[doc = concat!("Dereferences to the contained [`", stringify!($inner), "`].")]
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct $name {
            /// The guild.
            pub guild_id: crate::id::GuildId,
            $(#[$field_meta])*
            pub $field: $inner,
        }

        impl core::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &$inner {
                &self.$field
            }
        }

        impl core::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut $inner {
                &mut self.$field
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                use serde::de::Error as _;

                let (base, extra) = crate::serde_util::extras::deserialize::<
                    $inner,
                    super::GuildIdExtra,
                    D,
                >(deserializer)?;
                Ok(Self {
                    guild_id: extra
                        .guild_id
                        .ok_or_else(|| D::Error::missing_field("guild_id"))?,
                    $field: base.ok_or_else(|| D::Error::custom("missing event data"))?,
                })
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                #[derive(serde::Serialize)]
                struct Flat<'a> {
                    guild_id: crate::id::GuildId,
                    #[serde(flatten)]
                    $field: &'a $inner,
                }

                Flat {
                    guild_id: self.guild_id,
                    $field: &self.$field,
                }
                .serialize(serializer)
            }
        }
    };
}

use guild_id_event;

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
    /// The flags as a decimal string (for flags beyond 32 bits).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_flags_string"
    )]
    pub flags_new: Option<ApplicationFlags>,
}

/// The data of the [Rate Limited](https://docs.discord.com/developers/events/gateway-events#rate-limited)
/// dispatch event: a gateway command was rate limited.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RateLimited {
    /// The opcode of the rate-limited command.
    pub opcode: OpCode,
    /// How long to wait before sending it again, in seconds.
    pub retry_after: f64,
    /// Details for the opcode ([`RequestGuildMembers`](OpCode::RequestGuildMembers): the guild
    /// and nonce).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<RateLimitMetadata>,
}

/// The [metadata of a rate-limited `REQUEST_GUILD_MEMBERS`](https://docs.discord.com/developers/events/gateway-events#rate-limited-request-guild-member-rate-limit-metadata-structure).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RateLimitMetadata {
    /// The guild whose members were requested.
    pub guild_id: GuildId,
    /// The nonce of the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<CompactString>,
}
