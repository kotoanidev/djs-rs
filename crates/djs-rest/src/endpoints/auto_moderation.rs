//! [Auto Moderation endpoints](https://docs.discord.com/developers/resources/auto-moderation):
//! rules that block, log or time out content automatically. Every endpoint needs
//! `MANAGE_GUILD`.

use djs_model::{
    auto_moderation::Rule,
    id::{AutoModerationRuleId, GuildId},
    rest::{CreateAutoModerationRule, ModifyAutoModerationRule},
};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [List Auto Moderation Rules for Guild](https://docs.discord.com/developers/resources/auto-moderation#list-auto-moderation-rules-for-guild):
    /// returns every rule of the guild.
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_auto_moderation_rules_for_guild(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<Rule>, Error> {
        self.request(Request::new(Route::ListAutoModerationRulesForGuild {
            guild_id,
        }))
        .await
    }

    /// [Get Auto Moderation Rule](https://docs.discord.com/developers/resources/auto-moderation#get-auto-moderation-rule):
    /// returns one rule.
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule_id: AutoModerationRuleId,
    ) -> Result<Rule, Error> {
        self.request(Request::new(Route::GetAutoModerationRule {
            guild_id,
            rule_id,
        }))
        .await
    }

    /// [Create Auto Moderation Rule](https://docs.discord.com/developers/resources/auto-moderation#create-auto-moderation-rule):
    /// adds a rule. A `Timeout` action also needs `MODERATE_MEMBERS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `AutoModerationRuleManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, log: djs_model::id::ChannelId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{auto_moderation::Action, rest::CreateAutoModerationRule};
    ///
    /// let mut body = CreateAutoModerationRule::mention_spam(
    ///     "Mention spam",
    ///     10,
    ///     [Action::block_message(), Action::send_alert_message(log)],
    /// );
    /// body.enabled = Some(true);
    /// let rule = rest
    ///     .create_auto_moderation_rule(guild_id, &body, Some("Raid protection"))
    ///     .await?;
    /// println!("Created rule {}", rule.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `400` when the guild already has the most rules of that trigger
    /// type, or for metadata or actions the trigger type does not accept.
    pub async fn create_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        body: &CreateAutoModerationRule,
        reason: Option<&str>,
    ) -> Result<Rule, Error> {
        self.request(
            Request::new(Route::CreateAutoModerationRule { guild_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Modify Auto Moderation Rule](https://docs.discord.com/developers/resources/auto-moderation#modify-auto-moderation-rule):
    /// changes a rule and returns it.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `AutoModerationRule#edit` and its setters,
    /// `AutoModerationRuleManager#edit`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, rule_id: djs_model::id::AutoModerationRuleId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::ModifyAutoModerationRule;
    ///
    /// let mut body = ModifyAutoModerationRule::default();
    /// body.enabled = Some(false);
    /// rest.modify_auto_moderation_rule(guild_id, rule_id, &body, None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn modify_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule_id: AutoModerationRuleId,
        body: &ModifyAutoModerationRule,
        reason: Option<&str>,
    ) -> Result<Rule, Error> {
        self.request(
            Request::new(Route::ModifyAutoModerationRule { guild_id, rule_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Delete Auto Moderation Rule](https://docs.discord.com/developers/resources/auto-moderation#delete-auto-moderation-rule):
    /// deletes a rule.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `AutoModerationRule#delete`,
    /// `AutoModerationRuleManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn delete_auto_moderation_rule(
        &self,
        guild_id: GuildId,
        rule_id: AutoModerationRuleId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.request(
            Request::new(Route::DeleteAutoModerationRule { guild_id, rule_id }).reason(reason),
        )
        .await
    }
}
