//! [Soundboard](https://docs.discord.com/developers/resources/soundboard): short sounds users
//! play in voice channels — Discord's default sounds and each guild's own.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{EmojiId, GuildId, SoundboardSoundId},
    user::User,
};

/// A [soundboard sound](https://docs.discord.com/developers/resources/soundboard#soundboard-sound-object).
///
/// The sound file is at `https://cdn.discordapp.com/soundboard-sounds/{sound_id}`. Default
/// sounds have no [`guild_id`](Self::guild_id).
///
/// Contains a float ([`volume`](Self::volume)), so it is `PartialEq` but not `Eq` (D-064).
///
/// discord.js counterpart: `SoundboardSound`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SoundboardSound {
    /// The sound's name.
    pub name: CompactString,
    /// The sound's ID.
    pub sound_id: SoundboardSoundId,
    /// The volume, from 0 to 1.
    pub volume: f64,
    /// The sound's custom emoji.
    pub emoji_id: Option<EmojiId>,
    /// The sound's standard (Unicode) emoji.
    pub emoji_name: Option<CompactString>,
    /// The guild the sound belongs to (absent for default sounds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// Whether the sound can be used (`false` e.g. after the guild lost Server Boosts).
    #[serde(default = "crate::serde_util::default_true")]
    pub available: bool,
    /// The user who uploaded the sound (with the `CREATE_GUILD_EXPRESSIONS` or
    /// `MANAGE_GUILD_EXPRESSIONS` permission).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<Box<User>>,
}
