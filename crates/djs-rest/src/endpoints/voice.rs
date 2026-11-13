//! [Voice endpoints](https://docs.discord.com/developers/resources/voice): voice regions and
//! voice states. Joining voice channels goes through the gateway, not these endpoints.

use djs_model::{
    id::{GuildId, UserId},
    rest::{ModifyCurrentUserVoiceState, ModifyUserVoiceState},
    voice::{VoiceRegion, VoiceState},
};

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [List Voice Regions](https://docs.discord.com/developers/resources/voice#list-voice-regions):
    /// returns the regions a voice or stage channel's `rtc_region` can be set to.
    ///
    /// discord.js counterpart: `Client#fetchVoiceRegions`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_voice_regions(
        &self,
    ) -> impl Future<Output = Result<Vec<VoiceRegion>, Error>> + Send + '_ {
        boxed(async move { self.call(Request::new(Route::ListVoiceRegions)).await })
    }

    /// [Get Current User Voice State](https://docs.discord.com/developers/resources/voice#get-current-user-voice-state):
    /// returns the bot's voice state in a guild.
    ///
    /// discord.js counterpart: `VoiceStateManager#fetch('@me')`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10065` Unknown Voice State when the bot is not in a voice
    /// channel of the guild.
    pub fn get_current_user_voice_state(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<VoiceState, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetCurrentUserVoiceState { guild_id }))
                .await
        })
    }

    /// [Get User Voice State](https://docs.discord.com/developers/resources/voice#get-user-voice-state):
    /// returns a member's voice state. If they are in a voice channel, the bot needs the
    /// permission to connect to it.
    ///
    /// discord.js counterpart: `VoiceStateManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10065` Unknown Voice State when the user is not in a voice
    /// channel of the guild.
    pub fn get_user_voice_state(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> impl Future<Output = Result<VoiceState, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetUserVoiceState { guild_id, user_id }))
                .await
        })
    }

    /// [Modify Current User Voice State](https://docs.discord.com/developers/resources/voice#modify-current-user-voice-state):
    /// makes the bot a speaker or audience member of the stage it is in, or requests to
    /// speak.
    ///
    /// discord.js counterpart: `VoiceState#edit`, `#setSuppressed`, `#setRequestToSpeak`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, stage: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::ModifyCurrentUserVoiceState;
    ///
    /// // Become a speaker (needs MUTE_MEMBERS).
    /// let mut body = ModifyCurrentUserVoiceState::default();
    /// body.channel_id = Some(stage);
    /// body.suppress = Some(false);
    /// rest.modify_current_user_voice_state(guild_id, &body).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_current_user_voice_state<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a ModifyCurrentUserVoiceState,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::ModifyCurrentUserVoiceState { guild_id }).json(body)?,
            )
            .await
        })
    }

    /// [Modify User Voice State](https://docs.discord.com/developers/resources/voice#modify-user-voice-state):
    /// makes a member a speaker or audience member of the stage they are in. Needs
    /// `MUTE_MEMBERS`.
    ///
    /// discord.js counterpart: `VoiceState#edit`, `#setSuppressed`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn modify_user_voice_state<'a>(
        &'a self,
        guild_id: GuildId,
        user_id: UserId,
        body: &'a ModifyUserVoiceState,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::ModifyUserVoiceState { guild_id, user_id }).json(body)?,
            )
            .await
        })
    }
}
