//! [`StageInstanceManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, GuildView};
use djs_model::{
    id::{ChannelId, GuildId, StageInstanceId},
    rest::{CreateStageInstance, ModifyStageInstance},
    stage_instance::StageInstance,
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The live stages of one guild, by their stage channel. Returned by
/// [`Context::stage_instances`].
///
/// discord.js counterpart: `StageInstanceManager` (`guild.stageInstances`).
#[derive(Debug)]
pub struct StageInstanceManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for StageInstanceManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for StageInstanceManager<'_, B> {}

impl<'a, B: Backend> StageInstanceManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these stages belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached stage instances of the guild (by their own ID), if the client has a cache.
    ///
    /// discord.js counterpart: `StageInstanceManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, StageInstanceId, StageInstance, B>> {
        self.ctx
            .cache()
            .map(|cache| cache.stage_instances(self.guild_id))
    }

    /// The stage instance of a stage channel: the cached one, or [Get Stage
    /// Instance](djs_rest::Rest::get_stage_instance), whose result is cached.
    ///
    /// discord.js counterpart: `StageInstanceManager#fetch(channel)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed (404 when the
    /// stage is not live).
    pub fn fetch(
        self,
        channel_id: ChannelId,
    ) -> Fetch<
        Arc<StageInstance>,
        impl Future<Output = Result<Arc<StageInstance>, Error>> + Send + 'a,
    > {
        let cached = self.ctx.ops().stage_instance_of(self.guild_id, channel_id);
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let stage = ctx.rest().get_stage_instance(channel_id).await?;
            Ok(ctx.store_stage_instance(guild_id, &stage))
        })
    }

    /// Opens a stage in a stage channel ([Create Stage
    /// Instance](djs_rest::Rest::create_stage_instance)); it is cached.
    ///
    /// discord.js counterpart: `StageInstanceManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (the bot must be a stage moderator).
    pub async fn create(
        self,
        body: &CreateStageInstance,
        reason: Option<&str>,
    ) -> Result<Arc<StageInstance>, Error> {
        let stage = self.ctx.rest().create_stage_instance(body, reason).await?;
        Ok(self.ctx.store_stage_instance(self.guild_id, &stage))
    }

    /// Changes a live stage's topic or privacy ([Modify Stage
    /// Instance](djs_rest::Rest::modify_stage_instance)); the result is cached.
    ///
    /// discord.js counterpart: `StageInstanceManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        channel_id: ChannelId,
        body: &ModifyStageInstance,
        reason: Option<&str>,
    ) -> Result<Arc<StageInstance>, Error> {
        let stage = self
            .ctx
            .rest()
            .modify_stage_instance(channel_id, body, reason)
            .await?;
        Ok(self.ctx.store_stage_instance(self.guild_id, &stage))
    }

    /// Ends a stage ([Delete Stage Instance](djs_rest::Rest::delete_stage_instance)). The
    /// cache is updated by the `STAGE_INSTANCE_DELETE` event that follows (D-236).
    ///
    /// discord.js counterpart: `StageInstanceManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, channel_id: ChannelId, reason: Option<&str>) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_stage_instance(channel_id, reason)
            .await
    }
}
