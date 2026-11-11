//! [Guild template endpoints](https://docs.discord.com/developers/resources/guild-template):
//! snapshots of a guild's channels, roles and settings that others can create guilds from.
//!
//! Every endpoint except [`get_guild_template`](Rest::get_guild_template) needs
//! `MANAGE_GUILD`.

use djs_model::{
    guild_template::GuildTemplate,
    id::GuildId,
    rest::{CreateGuildTemplate, ModifyGuildTemplate},
};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Guild Template](https://docs.discord.com/developers/resources/guild-template#get-guild-template):
    /// resolves a template code (the part after `discord.new/`).
    ///
    /// discord.js counterpart: `Client#fetchGuildTemplate`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10057` Unknown Guild Template.
    pub async fn get_guild_template(&self, template_code: &str) -> Result<GuildTemplate, Error> {
        self.request(Request::new(Route::GetGuildTemplate { template_code }))
            .await
    }

    /// [Get Guild Templates](https://docs.discord.com/developers/resources/guild-template#get-guild-templates):
    /// returns the templates of a guild (at most one).
    ///
    /// discord.js counterpart: `Guild#fetchTemplates`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_guild_templates(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<GuildTemplate>, Error> {
        self.request(Request::new(Route::GetGuildTemplates { guild_id }))
            .await
    }

    /// [Create Guild Template](https://docs.discord.com/developers/resources/guild-template#create-guild-template):
    /// snapshots the guild into a new template.
    ///
    /// discord.js counterpart: `Guild#createTemplate`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateGuildTemplate;
    ///
    /// let template = rest
    ///     .create_guild_template(guild_id, &CreateGuildTemplate::new("Study group"))
    ///     .await?;
    /// println!("https://discord.new/{}", template.code);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30031` when the guild already has a template.
    pub async fn create_guild_template(
        &self,
        guild_id: GuildId,
        body: &CreateGuildTemplate,
    ) -> Result<GuildTemplate, Error> {
        self.request(Request::new(Route::CreateGuildTemplate { guild_id }).json(body)?)
            .await
    }

    /// [Sync Guild Template](https://docs.discord.com/developers/resources/guild-template#sync-guild-template):
    /// updates the template's snapshot to the guild's current state.
    ///
    /// discord.js counterpart: `GuildTemplate#sync`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10057` Unknown Guild Template.
    pub async fn sync_guild_template(
        &self,
        guild_id: GuildId,
        template_code: &str,
    ) -> Result<GuildTemplate, Error> {
        self.request(Request::new(Route::SyncGuildTemplate {
            guild_id,
            template_code,
        }))
        .await
    }

    /// [Modify Guild Template](https://docs.discord.com/developers/resources/guild-template#modify-guild-template):
    /// changes the template's name or description.
    ///
    /// discord.js counterpart: `GuildTemplate#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10057` Unknown Guild Template.
    pub async fn modify_guild_template(
        &self,
        guild_id: GuildId,
        template_code: &str,
        body: &ModifyGuildTemplate,
    ) -> Result<GuildTemplate, Error> {
        self.request(
            Request::new(Route::ModifyGuildTemplate {
                guild_id,
                template_code,
            })
            .json(body)?,
        )
        .await
    }

    /// [Delete Guild Template](https://docs.discord.com/developers/resources/guild-template#delete-guild-template):
    /// deletes the template and returns it.
    ///
    /// discord.js counterpart: `GuildTemplate#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10057` Unknown Guild Template.
    pub async fn delete_guild_template(
        &self,
        guild_id: GuildId,
        template_code: &str,
    ) -> Result<GuildTemplate, Error> {
        self.request(Request::new(Route::DeleteGuildTemplate {
            guild_id,
            template_code,
        }))
        .await
    }
}
