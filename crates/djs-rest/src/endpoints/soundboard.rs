//! [Soundboard endpoints](https://docs.discord.com/developers/resources/soundboard): playing
//! sounds in voice channels, Discord's default sounds and a guild's custom sounds.

use djs_model::{
    id::{ChannelId, GuildId, SoundboardSoundId},
    rest::{CreateGuildSoundboardSound, ModifyGuildSoundboardSound, SendSoundboardSound},
    soundboard::SoundboardSound,
};

use super::Items;
use crate::{Error, Request, Rest, Route};

/// Playing sounds and the default sounds.
impl Rest {
    /// [Send Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#send-soundboard-sound):
    /// plays a sound in a voice channel.
    ///
    /// The bot must be connected to the channel, neither muted, deafened nor suppressed,
    /// and needs `SPEAK` and `USE_SOUNDBOARD` (plus `USE_EXTERNAL_SOUNDS` for a sound of
    /// another guild).
    ///
    /// discord.js counterpart: `VoiceChannel#sendSoundboardSound`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::SendSoundboardSound;
    ///
    /// let sounds = rest.list_default_soundboard_sounds().await?;
    /// if let Some(sound) = sounds.iter().find(|sound| sound.name == "quack") {
    ///     rest.send_soundboard_sound(channel_id, &SendSoundboardSound::new(sound.sound_id))
    ///         .await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10097` Unknown Sound; `50013` Missing Permissions.
    pub async fn send_soundboard_sound(
        &self,
        channel_id: ChannelId,
        body: &SendSoundboardSound,
    ) -> Result<(), Error> {
        self.request_ignoring_body(
            Request::new(Route::SendSoundboardSound { channel_id }).json(body)?,
        )
        .await
    }

    /// [List Default Soundboard Sounds](https://docs.discord.com/developers/resources/soundboard#list-default-soundboard-sounds):
    /// returns the sounds every user can play.
    ///
    /// discord.js counterpart: `Client#fetchDefaultSoundboardSounds`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_default_soundboard_sounds(&self) -> Result<Vec<SoundboardSound>, Error> {
        self.request(Request::new(Route::ListDefaultSoundboardSounds))
            .await
    }
}

/// Guild soundboard sounds.
impl Rest {
    /// [List Guild Soundboard Sounds](https://docs.discord.com/developers/resources/soundboard#list-guild-soundboard-sounds):
    /// returns every custom sound of a guild. [`SoundboardSound::user`] is included with the
    /// `CREATE_GUILD_EXPRESSIONS` or `MANAGE_GUILD_EXPRESSIONS` permission. Discord wraps the
    /// list in an object; it is unwrapped.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_guild_soundboard_sounds(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<SoundboardSound>, Error> {
        let list: Items<SoundboardSound> = self
            .request(Request::new(Route::ListGuildSoundboardSounds { guild_id }))
            .await?;
        Ok(list.items)
    }

    /// [Get Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#get-guild-soundboard-sound):
    /// returns one custom sound of a guild.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10097` Unknown Sound.
    pub async fn get_guild_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound_id: SoundboardSoundId,
    ) -> Result<SoundboardSound, Error> {
        self.request(Request::new(Route::GetGuildSoundboardSound {
            guild_id,
            sound_id,
        }))
        .await
    }

    /// [Create Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#create-guild-soundboard-sound):
    /// uploads a custom sound (MP3 or Ogg, at most 512 KB and 5.2 seconds). Needs
    /// `CREATE_GUILD_EXPRESSIONS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), Box<dyn std::error::Error>> {
    /// use djs_model::{rest::CreateGuildSoundboardSound, util::SoundData};
    ///
    /// let sound = SoundData::new(std::fs::read("tada.mp3")?)?;
    /// let mut body = CreateGuildSoundboardSound::new("tada", sound);
    /// body.emoji_name = Some("🎉".into());
    /// rest.create_guild_soundboard_sound(guild_id, &body, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30045` when the guild has no free sound slot.
    pub async fn create_guild_soundboard_sound(
        &self,
        guild_id: GuildId,
        body: &CreateGuildSoundboardSound,
        reason: Option<&str>,
    ) -> Result<SoundboardSound, Error> {
        self.request(
            Request::new(Route::CreateGuildSoundboardSound { guild_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Modify Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#modify-guild-soundboard-sound):
    /// changes a sound's name, volume or emoji. Needs `MANAGE_GUILD_EXPRESSIONS`, or
    /// `CREATE_GUILD_EXPRESSIONS` for sounds the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `SoundboardSound#edit`, `GuildSoundboardSoundManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10097` Unknown Sound.
    pub async fn modify_guild_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound_id: SoundboardSoundId,
        body: &ModifyGuildSoundboardSound,
        reason: Option<&str>,
    ) -> Result<SoundboardSound, Error> {
        self.request(
            Request::new(Route::ModifyGuildSoundboardSound { guild_id, sound_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Delete Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#delete-guild-soundboard-sound):
    /// deletes a custom sound. Needs `MANAGE_GUILD_EXPRESSIONS`, or
    /// `CREATE_GUILD_EXPRESSIONS` for sounds the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `SoundboardSound#delete`, `GuildSoundboardSoundManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10097` Unknown Sound.
    pub async fn delete_guild_soundboard_sound(
        &self,
        guild_id: GuildId,
        sound_id: SoundboardSoundId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.request(
            Request::new(Route::DeleteGuildSoundboardSound { guild_id, sound_id }).reason(reason),
        )
        .await
    }
}
