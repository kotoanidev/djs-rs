//! Bodies of the [soundboard endpoints](https://docs.discord.com/developers/resources/soundboard).

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    id::{EmojiId, GuildId, SoundboardSoundId},
    util::{Patch, SoundData},
};

/// The body of [Send Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#send-soundboard-sound).
///
/// discord.js counterpart: the argument of `VoiceChannel#sendSoundboardSound`.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     id::{GuildId, SoundboardSoundId},
///     rest::SendSoundboardSound,
/// };
///
/// // A sound of another guild needs that guild's ID.
/// let mut body = SendSoundboardSound::new(SoundboardSoundId::new(1_270_000_000_000_000_001));
/// body.source_guild_id = Some(GuildId::new(1_100_000_000_000_000_009));
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"sound_id":"1270000000000000001","source_guild_id":"1100000000000000009"}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct SendSoundboardSound {
    /// The sound to play.
    pub sound_id: SoundboardSoundId,
    /// The guild the sound belongs to, required for sounds of another guild (which also
    /// needs the `USE_EXTERNAL_SOUNDS` permission); not needed for default sounds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_guild_id: Option<GuildId>,
}

impl SendSoundboardSound {
    /// Plays `sound_id`, a default sound or a sound of the voice channel's guild.
    #[must_use]
    pub const fn new(sound_id: SoundboardSoundId) -> Self {
        Self {
            sound_id,
            source_guild_id: None,
        }
    }
}

/// The body of [Create Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#create-guild-soundboard-sound).
///
/// The sound (MP3 or Ogg) may be at most 512 KB and 5.2 seconds long.
///
/// discord.js counterpart: `GuildSoundboardSoundCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::CreateGuildSoundboardSound, util::SoundData};
///
/// let ogg = b"OggS\x00\x02...".to_vec();
/// let mut body = CreateGuildSoundboardSound::new("tada", SoundData::new(ogg)?);
/// body.volume = Some(0.5);
/// body.emoji_name = Some("🎉".into());
/// let json = serde_json::to_string(&body)?;
/// assert!(json.starts_with(r#"{"name":"tada","sound":"data:audio/ogg;base64,"#));
/// assert!(json.ends_with(r#","volume":0.5,"emoji_name":"🎉"}"#));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CreateGuildSoundboardSound {
    /// The name (2–32 characters).
    pub name: CompactString,
    /// The sound.
    pub sound: SoundData,
    /// The volume, from 0 to 1 (Discord's default: 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    /// A custom emoji shown with the sound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<EmojiId>,
    /// A Unicode emoji shown with the sound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<CompactString>,
}

impl CreateGuildSoundboardSound {
    /// A sound named `name` playing `sound`, at full volume, without an emoji.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, sound: SoundData) -> Self {
        Self {
            name: name.into(),
            sound,
            volume: None,
            emoji_id: None,
            emoji_name: None,
        }
    }
}

/// The body of [Modify Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#modify-guild-soundboard-sound).
///
/// Every field is optional; the sound itself cannot be changed. [`Patch`] fields can be
/// cleared.
///
/// discord.js counterpart: `GuildSoundboardSoundEditOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::ModifyGuildSoundboardSound, util::Patch};
///
/// let mut body = ModifyGuildSoundboardSound::default();
/// body.volume = Patch::Set(0.25);
/// body.emoji_name = Patch::Clear;
/// assert_eq!(serde_json::to_string(&body)?, r#"{"volume":0.25,"emoji_name":null}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildSoundboardSound {
    /// The new name (2–32 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The volume, from 0 to 1.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub volume: Patch<f64>,
    /// A custom emoji shown with the sound.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub emoji_id: Patch<EmojiId>,
    /// A Unicode emoji shown with the sound.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub emoji_name: Patch<CompactString>,
}
