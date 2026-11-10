//! [Stage instances](https://docs.discord.com/developers/resources/stage-instance): live
//! audio events in a stage channel.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{ChannelId, GuildId, ScheduledEventId, StageInstanceId},
    macros::int_enum,
};

/// A [stage instance](https://docs.discord.com/developers/resources/stage-instance#stage-instance-object):
/// a stage channel that is currently live.
///
/// discord.js counterpart: `StageInstance`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StageInstance {
    /// The stage instance's ID.
    pub id: StageInstanceId,
    /// The guild of the stage channel.
    pub guild_id: GuildId,
    /// The stage channel.
    pub channel_id: ChannelId,
    /// The topic (1-120 characters).
    pub topic: CompactString,
    /// Who can see the stage instance.
    pub privacy_level: PrivacyLevel,
    /// Whether Stage Discovery is disabled. **Deprecated** by Discord.
    #[serde(default)]
    pub discoverable_disabled: bool,
    /// The scheduled event this stage instance belongs to.
    pub guild_scheduled_event_id: Option<ScheduledEventId>,
}

int_enum! {
    /// [Who can see](https://docs.discord.com/developers/resources/stage-instance#stage-instance-object-privacy-level)
    /// a stage instance.
    ///
    /// discord.js counterpart: `StageInstancePrivacyLevel`.
    #[doc(alias = "StageInstancePrivacyLevel")]
    pub enum PrivacyLevel: u8 {
        /// Everyone. **Deprecated** by Discord.
        Public = 1,
        /// Only guild members.
        GuildOnly = 2,
    }
}
