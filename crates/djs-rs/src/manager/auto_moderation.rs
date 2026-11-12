//! [`AutoModerationRuleManager`].

use std::future::Future;

use djs_cache::Backend;
use djs_collection::Collection;
use djs_model::{
    auto_moderation::Rule,
    id::{AutoModerationRuleId, GuildId},
    rest::{CreateAutoModerationRule, ModifyAutoModerationRule},
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The auto moderation rules of one guild. Returned by [`Context::auto_moderation_rules`].
///
/// The cache does not keep rules, so the [`djs-model`](djs_model) [`Rule`] is returned. Every
/// method needs `MANAGE_GUILD`.
///
/// discord.js counterpart: `AutoModerationRuleManager` (`guild.autoModerationRules`).
#[derive(Debug)]
pub struct AutoModerationRuleManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for AutoModerationRuleManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for AutoModerationRuleManager<'_, B> {}

impl<'a, B: Backend> AutoModerationRuleManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these rules belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// A rule ([Get Auto Moderation Rule](djs_rest::Rest::get_auto_moderation_rule)).
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub fn fetch(
        self,
        rule_id: AutoModerationRuleId,
    ) -> Fetch<Rule, impl Future<Output = Result<Rule, Error>> + Send + 'a> {
        let Self { ctx, guild_id } = self;
        Fetch::new(None, async move {
            ctx.rest().get_auto_moderation_rule(guild_id, rule_id).await
        })
    }

    /// Every rule of the guild ([List Auto Moderation Rules for
    /// Guild](djs_rest::Rest::list_auto_moderation_rules_for_guild)).
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(self) -> Result<Collection<AutoModerationRuleId, Rule>, Error> {
        let rules = self
            .ctx
            .rest()
            .list_auto_moderation_rules_for_guild(self.guild_id)
            .await?;
        Ok(rules.into_iter().map(|rule| (rule.id, rule)).collect())
    }

    /// Creates a rule ([Create Auto Moderation
    /// Rule](djs_rest::Rest::create_auto_moderation_rule)).
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn create(
        self,
        body: &CreateAutoModerationRule,
        reason: Option<&str>,
    ) -> Result<Rule, Error> {
        self.ctx
            .rest()
            .create_auto_moderation_rule(self.guild_id, body, reason)
            .await
    }

    /// Edits a rule ([Modify Auto Moderation
    /// Rule](djs_rest::Rest::modify_auto_moderation_rule)).
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn edit(
        self,
        rule_id: AutoModerationRuleId,
        body: &ModifyAutoModerationRule,
        reason: Option<&str>,
    ) -> Result<Rule, Error> {
        self.ctx
            .rest()
            .modify_auto_moderation_rule(self.guild_id, rule_id, body, reason)
            .await
    }

    /// Deletes a rule ([Delete Auto Moderation
    /// Rule](djs_rest::Rest::delete_auto_moderation_rule)).
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(
        self,
        rule_id: AutoModerationRuleId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_auto_moderation_rule(self.guild_id, rule_id, reason)
            .await
    }
}
