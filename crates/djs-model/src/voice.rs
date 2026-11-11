//! [Voice](https://docs.discord.com/developers/resources/voice): users' voice connection states
//! and the available voice regions.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    guild::Member,
    id::{ChannelId, GuildId, UserId},
    util::Timestamp,
};

/// A user's [voice state](https://docs.discord.com/developers/resources/voice#voice-state-object):
/// which voice channel they are in and whether they are muted, deafened, streaming, ….
///
/// Received in `GUILD_CREATE` (without `guild_id` and `member`), `VOICE_STATE_UPDATE` and from
/// the voice state endpoints. Absent booleans are `false` (C-023; the docs' example omits
/// `self_video`).
///
/// discord.js counterpart: `VoiceState`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoiceState {
    /// The guild (absent in `GUILD_CREATE`, where the guild is known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The connected channel (`None` once the user left).
    pub channel_id: Option<ChannelId>,
    /// The user.
    pub user_id: UserId,
    /// The user's member object (absent in `GUILD_CREATE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<Member>>,
    /// The voice session's ID.
    pub session_id: CompactString,
    /// Whether the server deafened the user.
    #[serde(default)]
    pub deaf: bool,
    /// Whether the server muted the user.
    #[serde(default)]
    pub mute: bool,
    /// Whether the user deafened themselves.
    #[serde(default)]
    pub self_deaf: bool,
    /// Whether the user muted themselves.
    #[serde(default)]
    pub self_mute: bool,
    /// Whether the user is streaming with "Go Live".
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub self_stream: bool,
    /// Whether the user's camera is on.
    #[serde(default)]
    pub self_video: bool,
    /// Whether the user's permission to speak is denied (stage audience members).
    #[serde(default)]
    pub suppress: bool,
    /// When the user asked to speak in a stage.
    pub request_to_speak_timestamp: Option<Timestamp>,
}

/// A [voice region](https://docs.discord.com/developers/resources/voice#voice-region-object):
/// a value for a voice channel's `rtc_region`.
///
/// discord.js counterpart: `VoiceRegion`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoiceRegion {
    /// The region's ID (the `rtc_region` value).
    pub id: CompactString,
    /// The region's name.
    pub name: CompactString,
    /// Whether this is the closest region to the current user's client.
    pub optimal: bool,
    /// Whether the region is deprecated (avoid switching to it).
    pub deprecated: bool,
    /// Whether this is a custom region (used for events).
    pub custom: bool,
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use super::*;

    #[test]
    fn sizes() {
        assert!(size_of::<VoiceState>() <= 96, "{}", size_of::<VoiceState>());
    }
}
