//! [Emoji endpoints](https://docs.discord.com/developers/resources/emoji): a guild's custom
//! emojis and the application's own emojis.
//!
//! Discord limits the guild emoji routes per guild, separately from the usual buckets, so the
//! rate-limit headers may be inaccurate and an unexpected `429` is retried as usual.

use djs_model::{
    emoji::Emoji,
    id::{ApplicationId, EmojiId, GuildId},
    rest::{CreateApplicationEmoji, CreateGuildEmoji, ModifyApplicationEmoji, ModifyGuildEmoji},
};

use super::Items;
use crate::{Error, Request, Rest, Route};

/// Guild emojis.
impl Rest {
    /// [List Guild Emojis](https://docs.discord.com/developers/resources/emoji#list-guild-emojis):
    /// returns every custom emoji of a guild. [`Emoji::user`] is included with the
    /// `CREATE_GUILD_EXPRESSIONS` or `MANAGE_GUILD_EXPRESSIONS` permission.
    ///
    /// discord.js counterpart: `GuildEmojiManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_guild_emojis(&self, guild_id: GuildId) -> Result<Vec<Emoji>, Error> {
        self.request(Request::new(Route::ListGuildEmojis { guild_id }))
            .await
    }

    /// [Get Guild Emoji](https://docs.discord.com/developers/resources/emoji#get-guild-emoji):
    /// returns one custom emoji of a guild.
    ///
    /// discord.js counterpart: `GuildEmojiManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji.
    pub async fn get_guild_emoji(
        &self,
        guild_id: GuildId,
        emoji_id: EmojiId,
    ) -> Result<Emoji, Error> {
        self.request(Request::new(Route::GetGuildEmoji { guild_id, emoji_id }))
            .await
    }

    /// [Create Guild Emoji](https://docs.discord.com/developers/resources/emoji#create-guild-emoji):
    /// uploads a custom emoji. Needs `CREATE_GUILD_EXPRESSIONS`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildEmojiManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), Box<dyn std::error::Error>> {
    /// use djs_model::{rest::CreateGuildEmoji, util::ImageData};
    ///
    /// let image = ImageData::new(std::fs::read("ferris.png")?)?;
    /// let emoji = rest
    ///     .create_guild_emoji(guild_id, &CreateGuildEmoji::new("ferris", image), Some("New mascot"))
    ///     .await?;
    /// println!("Use it as <:{}:{}>", emoji.name, emoji.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30008` or `30018` when the guild has no free (animated) emoji
    /// slot; a `400` without a JSON error code when the image is larger than 256 KiB.
    pub async fn create_guild_emoji(
        &self,
        guild_id: GuildId,
        body: &CreateGuildEmoji,
        reason: Option<&str>,
    ) -> Result<Emoji, Error> {
        self.request(
            Request::new(Route::CreateGuildEmoji { guild_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Modify Guild Emoji](https://docs.discord.com/developers/resources/emoji#modify-guild-emoji):
    /// renames an emoji or changes which roles may use it. Needs `MANAGE_GUILD_EXPRESSIONS`,
    /// or `CREATE_GUILD_EXPRESSIONS` for emojis the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildEmoji#edit`, `GuildEmojiManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji.
    pub async fn modify_guild_emoji(
        &self,
        guild_id: GuildId,
        emoji_id: EmojiId,
        body: &ModifyGuildEmoji,
        reason: Option<&str>,
    ) -> Result<Emoji, Error> {
        self.request(
            Request::new(Route::ModifyGuildEmoji { guild_id, emoji_id })
                .json(body)?
                .reason(reason),
        )
        .await
    }

    /// [Delete Guild Emoji](https://docs.discord.com/developers/resources/emoji#delete-guild-emoji):
    /// deletes a custom emoji. Needs `MANAGE_GUILD_EXPRESSIONS`, or `CREATE_GUILD_EXPRESSIONS`
    /// for emojis the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildEmoji#delete`, `GuildEmojiManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji.
    pub async fn delete_guild_emoji(
        &self,
        guild_id: GuildId,
        emoji_id: EmojiId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.request(Request::new(Route::DeleteGuildEmoji { guild_id, emoji_id }).reason(reason))
            .await
    }
}

/// Application emojis: emojis owned by the bot's application, usable by the bot everywhere.
impl Rest {
    /// [List Application Emojis](https://docs.discord.com/developers/resources/emoji#list-application-emojis):
    /// returns every emoji of the application, with [`Emoji::user`] (the team member who
    /// uploaded it, or the bot). Discord wraps the list in an object; it is unwrapped.
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_application_emojis(
        &self,
        application_id: ApplicationId,
    ) -> Result<Vec<Emoji>, Error> {
        let list: Items<Emoji> = self
            .request(Request::new(Route::ListApplicationEmojis {
                application_id,
            }))
            .await?;
        Ok(list.items)
    }

    /// [Get Application Emoji](https://docs.discord.com/developers/resources/emoji#get-application-emoji):
    /// returns one emoji of the application, with [`Emoji::user`].
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji.
    pub async fn get_application_emoji(
        &self,
        application_id: ApplicationId,
        emoji_id: EmojiId,
    ) -> Result<Emoji, Error> {
        self.request(Request::new(Route::GetApplicationEmoji {
            application_id,
            emoji_id,
        }))
        .await
    }

    /// [Create Application Emoji](https://docs.discord.com/developers/resources/emoji#create-application-emoji):
    /// uploads an emoji the bot can use in every guild and DM, without needing the
    /// `USE_EXTERNAL_EMOJIS` permission there.
    ///
    /// discord.js counterpart: `ApplicationEmojiManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId) -> Result<(), Box<dyn std::error::Error>> {
    /// use djs_model::{rest::CreateApplicationEmoji, util::ImageData};
    ///
    /// let image = ImageData::new(std::fs::read("loading.gif")?)?;
    /// let body = CreateApplicationEmoji::new("loading", image);
    /// let emoji = rest.create_application_emoji(application_id, &body).await?;
    /// assert!(emoji.animated);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30008` when the application has no free emoji slot; a `400`
    /// without a JSON error code when the image is larger than 256 KiB.
    pub async fn create_application_emoji(
        &self,
        application_id: ApplicationId,
        body: &CreateApplicationEmoji,
    ) -> Result<Emoji, Error> {
        self.request(Request::new(Route::CreateApplicationEmoji { application_id }).json(body)?)
            .await
    }

    /// [Modify Application Emoji](https://docs.discord.com/developers/resources/emoji#modify-application-emoji):
    /// renames an emoji of the application.
    ///
    /// discord.js counterpart: `ApplicationEmoji#edit`, `ApplicationEmojiManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji.
    pub async fn modify_application_emoji(
        &self,
        application_id: ApplicationId,
        emoji_id: EmojiId,
        body: &ModifyApplicationEmoji,
    ) -> Result<Emoji, Error> {
        self.request(
            Request::new(Route::ModifyApplicationEmoji {
                application_id,
                emoji_id,
            })
            .json(body)?,
        )
        .await
    }

    /// [Delete Application Emoji](https://docs.discord.com/developers/resources/emoji#delete-application-emoji):
    /// deletes an emoji of the application.
    ///
    /// discord.js counterpart: `ApplicationEmoji#delete`, `ApplicationEmojiManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10014` Unknown Emoji.
    pub async fn delete_application_emoji(
        &self,
        application_id: ApplicationId,
        emoji_id: EmojiId,
    ) -> Result<(), Error> {
        self.request(Request::new(Route::DeleteApplicationEmoji {
            application_id,
            emoji_id,
        }))
        .await
    }
}
