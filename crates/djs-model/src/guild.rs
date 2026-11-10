//! [Guilds](https://docs.discord.com/developers/resources/guild) (servers).

use serde::{Deserialize, Serialize};

use crate::id::GuildId;

/// A guild that is not (or not yet) available to the client.
///
/// The [`Ready`](crate::gateway::payload::Ready) event lists every guild the bot is in as
/// unavailable; full guild data then arrives in `GUILD_CREATE` events. A guild also becomes
/// unavailable during a Discord outage.
///
/// See the [unavailable guild object](https://docs.discord.com/developers/resources/guild#unavailable-guild-object).
///
/// discord.js counterpart: guilds with `available === false`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnavailableGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// Whether the guild is unavailable due to an outage.
    ///
    /// In `GUILD_DELETE` events this is `false` (absent) when the bot was removed from the guild
    /// rather than the guild becoming unavailable.
    #[serde(default)]
    pub unavailable: bool,
}
