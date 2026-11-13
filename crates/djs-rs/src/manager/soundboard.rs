//! [`SoundboardSoundManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, GuildView};
use djs_collection::Collection;
use djs_model::{
    id::{GuildId, SoundboardSoundId},
    rest::{CreateGuildSoundboardSound, ModifyGuildSoundboardSound},
    soundboard::SoundboardSound,
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The soundboard sounds of one guild. Returned by [`Context::soundboard_sounds`]; Discord's
/// default sounds are listed by [`Context::default_soundboard_sounds`] (D-243).
///
/// discord.js counterpart: `GuildSoundboardSoundManager` (`guild.soundboardSounds`).
#[derive(Debug)]
pub struct SoundboardSoundManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for SoundboardSoundManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for SoundboardSoundManager<'_, B> {}

impl<'a, B: Backend> SoundboardSoundManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these sounds belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached sounds of the guild, if the client has a cache.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, SoundboardSoundId, SoundboardSound, B>> {
        self.ctx
            .cache()
            .map(|cache| cache.soundboard_sounds(self.guild_id))
    }

    /// A sound: the cached one, or [Get Guild Soundboard
    /// Sound](djs_rest::Rest::get_guild_soundboard_sound),
    /// whose result is cached.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        sound_id: SoundboardSoundId,
    ) -> Fetch<
        Arc<SoundboardSound>,
        impl Future<Output = Result<Arc<SoundboardSound>, Error>> + Send + 'a,
    > {
        let cached = self.ctx.ops().soundboard_sound(self.guild_id, sound_id);
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let sound = ctx
                .rest()
                .get_guild_soundboard_sound(guild_id, sound_id)
                .await?;
            Ok(ctx.store_soundboard_sound(guild_id, &sound))
        })
    }

    /// Every soundboard sound of the guild ([List Guild Soundboard
    /// Sounds](djs_rest::Rest::list_guild_soundboard_sounds)); the sounds are cached.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<SoundboardSoundId, Arc<SoundboardSound>>, Error> {
        let sounds = self
            .ctx
            .rest()
            .list_guild_soundboard_sounds(self.guild_id)
            .await?;
        Ok(sounds
            .iter()
            .map(|sound| {
                (
                    sound.sound_id,
                    self.ctx.store_soundboard_sound(self.guild_id, sound),
                )
            })
            .collect())
    }

    /// Uploads a sound ([Create Guild Soundboard
    /// Sound](djs_rest::Rest::create_guild_soundboard_sound)); it is cached.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CREATE_EXPRESSIONS`; the
    /// sound must be an MP3 or OGG of at most 512 KiB and 5.2 seconds).
    pub async fn create(
        self,
        body: &CreateGuildSoundboardSound,
        reason: Option<&str>,
    ) -> Result<Arc<SoundboardSound>, Error> {
        let sound = self
            .ctx
            .rest()
            .create_guild_soundboard_sound(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_soundboard_sound(self.guild_id, &sound))
    }

    /// Changes a sound's name, volume or emoji ([Modify Guild Soundboard
    /// Sound](djs_rest::Rest::modify_guild_soundboard_sound)); the result is cached.
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#edit` / `SoundboardSound#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        sound_id: SoundboardSoundId,
        body: &ModifyGuildSoundboardSound,
        reason: Option<&str>,
    ) -> Result<Arc<SoundboardSound>, Error> {
        let sound = self
            .ctx
            .rest()
            .modify_guild_soundboard_sound(self.guild_id, sound_id, body, reason)
            .await?;
        Ok(self.ctx.store_soundboard_sound(self.guild_id, &sound))
    }

    /// Deletes a sound ([Delete Guild Soundboard
    /// Sound](djs_rest::Rest::delete_guild_soundboard_sound)). The
    /// cache is updated by the `GUILD_SOUNDBOARD_SOUND_DELETE` event that follows (D-236).
    ///
    /// discord.js counterpart: `GuildSoundboardSoundManager#delete` / `SoundboardSound#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(
        self,
        sound_id: SoundboardSoundId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_soundboard_sound(self.guild_id, sound_id, reason)
            .await
    }
}
