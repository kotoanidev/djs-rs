//! [Stage instance endpoints](https://docs.discord.com/developers/resources/stage-instance):
//! opening, changing and closing the live stage of a stage channel.
//!
//! Every endpoint needs the bot to be a stage moderator (`MANAGE_CHANNELS`, `MUTE_MEMBERS`
//! and `MOVE_MEMBERS` in the channel).

use djs_model::{
    id::ChannelId,
    rest::{CreateStageInstance, ModifyStageInstance},
    stage_instance::StageInstance,
};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Create Stage Instance](https://docs.discord.com/developers/resources/stage-instance#create-stage-instance):
    /// opens a stage in a stage channel.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `StageInstanceManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, stage: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateStageInstance;
    ///
    /// rest.create_stage_instance(&CreateStageInstance::new(stage, "Weekly Q&A"), None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `150006` when the stage is already open.
    pub async fn create_stage_instance(
        &self,
        body: &CreateStageInstance,
        reason: Option<&str>,
    ) -> Result<StageInstance, Error> {
        self.request(
            Request::new(Route::CreateStageInstance)
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Get Stage Instance](https://docs.discord.com/developers/resources/stage-instance#get-stage-instance):
    /// returns the open stage of a stage channel.
    ///
    /// discord.js counterpart: `StageInstanceManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10067` Unknown Stage Instance when no stage is open.
    pub async fn get_stage_instance(&self, channel_id: ChannelId) -> Result<StageInstance, Error> {
        self.request(Request::new(Route::GetStageInstance { channel_id }))
            .await
    }

    /// [Modify Stage Instance](https://docs.discord.com/developers/resources/stage-instance#modify-stage-instance):
    /// changes the topic or privacy of an open stage.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `StageInstance#edit`, `#setTopic`, `StageInstanceManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10067` Unknown Stage Instance.
    pub async fn modify_stage_instance(
        &self,
        channel_id: ChannelId,
        body: &ModifyStageInstance,
        reason: Option<&str>,
    ) -> Result<StageInstance, Error> {
        self.request(
            Request::new(Route::ModifyStageInstance { channel_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Delete Stage Instance](https://docs.discord.com/developers/resources/stage-instance#delete-stage-instance):
    /// closes a stage.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `StageInstance#delete`, `StageInstanceManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10067` Unknown Stage Instance.
    pub async fn delete_stage_instance(
        &self,
        channel_id: ChannelId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.request(Request::new(Route::DeleteStageInstance { channel_id }).reason(reason))
            .await
    }
}
