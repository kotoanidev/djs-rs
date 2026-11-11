//! [Sticker endpoints](https://docs.discord.com/developers/resources/sticker): any sticker,
//! Discord's sticker packs, and a guild's custom stickers.

use djs_model::{
    id::{GuildId, StickerId, StickerPackId},
    rest::{CreateGuildSticker, ModifyGuildSticker},
    sticker::{Sticker, StickerPack},
};
use serde::Deserialize;

use crate::{Error, Request, Rest, Route};

/// The response of List Sticker Packs, unwrapped by the method (C-053).
#[derive(Deserialize)]
struct StickerPacks {
    sticker_packs: Vec<StickerPack>,
}

/// Stickers and sticker packs.
impl Rest {
    /// [Get Sticker](https://docs.discord.com/developers/resources/sticker#get-sticker):
    /// returns any sticker by ID — a standard sticker of a pack or a guild sticker.
    ///
    /// discord.js counterpart: `Client#fetchSticker`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10060` Unknown Sticker.
    pub async fn get_sticker(&self, sticker_id: StickerId) -> Result<Sticker, Error> {
        self.request(Request::new(Route::GetSticker { sticker_id }))
            .await
    }

    /// [List Sticker Packs](https://docs.discord.com/developers/resources/sticker#list-sticker-packs):
    /// returns Discord's standard sticker packs. Discord wraps the list in an object; it is
    /// unwrapped.
    ///
    /// discord.js counterpart: `Client#fetchStickerPacks()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_sticker_packs(&self) -> Result<Vec<StickerPack>, Error> {
        let list: StickerPacks = self.request(Request::new(Route::ListStickerPacks)).await?;
        Ok(list.sticker_packs)
    }

    /// [Get Sticker Pack](https://docs.discord.com/developers/resources/sticker#get-sticker-pack):
    /// returns one of Discord's standard sticker packs.
    ///
    /// discord.js counterpart: `Client#fetchStickerPacks({ packId })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10061` Unknown Sticker Pack.
    pub async fn get_sticker_pack(&self, pack_id: StickerPackId) -> Result<StickerPack, Error> {
        self.request(Request::new(Route::GetStickerPack { pack_id }))
            .await
    }
}

/// Guild stickers.
impl Rest {
    /// [List Guild Stickers](https://docs.discord.com/developers/resources/sticker#list-guild-stickers):
    /// returns every custom sticker of a guild. [`Sticker::user`] is included with the
    /// `CREATE_GUILD_EXPRESSIONS` or `MANAGE_GUILD_EXPRESSIONS` permission.
    ///
    /// discord.js counterpart: `GuildStickerManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_guild_stickers(&self, guild_id: GuildId) -> Result<Vec<Sticker>, Error> {
        self.request(Request::new(Route::ListGuildStickers { guild_id }))
            .await
    }

    /// [Get Guild Sticker](https://docs.discord.com/developers/resources/sticker#get-guild-sticker):
    /// returns one custom sticker of a guild.
    ///
    /// discord.js counterpart: `GuildStickerManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10060` Unknown Sticker.
    pub async fn get_guild_sticker(
        &self,
        guild_id: GuildId,
        sticker_id: StickerId,
    ) -> Result<Sticker, Error> {
        self.request(Request::new(Route::GetGuildSticker {
            guild_id,
            sticker_id,
        }))
        .await
    }

    /// [Create Guild Sticker](https://docs.discord.com/developers/resources/sticker#create-guild-sticker):
    /// uploads a custom sticker. Needs `CREATE_GUILD_EXPRESSIONS`; a guild has 5 sticker
    /// slots, and more with each boost level.
    ///
    /// The body is sent as `multipart/form-data` fields (`name`, `description`, `tags`, `file`).
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildStickerManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), Box<dyn std::error::Error>> {
    /// use djs_model::rest::{CreateGuildSticker, FileUpload};
    ///
    /// let file = FileUpload::new("wave.png", std::fs::read("wave.png")?);
    /// let mut body = CreateGuildSticker::new("Wave", "wave", file);
    /// body.description = "Say hello".into();
    /// let sticker = rest.create_guild_sticker(guild_id, &body, None).await?;
    /// println!("Created sticker {}", sticker.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30039` when the guild has no free sticker slot; `50045` when
    /// the file is too large; `50081` for an invalid file.
    pub async fn create_guild_sticker(
        &self,
        guild_id: GuildId,
        body: &CreateGuildSticker,
        reason: Option<&str>,
    ) -> Result<Sticker, Error> {
        let fields = [
            ("name", body.name.as_str()),
            ("description", body.description.as_str()),
            ("tags", body.tags.as_str()),
        ];
        self.request(
            Request::new(Route::CreateGuildSticker { guild_id })
                .form_file("file", &body.file)
                .form_text(&fields)
                .reason(reason),
        )
        .await
    }

    /// [Modify Guild Sticker](https://docs.discord.com/developers/resources/sticker#modify-guild-sticker):
    /// changes a sticker's name, description or tags. Needs `MANAGE_GUILD_EXPRESSIONS`, or
    /// `CREATE_GUILD_EXPRESSIONS` for stickers the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Sticker#edit`, `GuildStickerManager#edit`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10060` Unknown Sticker.
    pub async fn modify_guild_sticker(
        &self,
        guild_id: GuildId,
        sticker_id: StickerId,
        body: &ModifyGuildSticker,
        reason: Option<&str>,
    ) -> Result<Sticker, Error> {
        self.request(
            Request::new(Route::ModifyGuildSticker {
                guild_id,
                sticker_id,
            })
            .json(body)?
            .reason(reason),
        )
        .await
    }

    /// [Delete Guild Sticker](https://docs.discord.com/developers/resources/sticker#delete-guild-sticker):
    /// deletes a custom sticker. Needs `MANAGE_GUILD_EXPRESSIONS`, or
    /// `CREATE_GUILD_EXPRESSIONS` for stickers the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Sticker#delete`, `GuildStickerManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10060` Unknown Sticker.
    pub async fn delete_guild_sticker(
        &self,
        guild_id: GuildId,
        sticker_id: StickerId,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        self.request(
            Request::new(Route::DeleteGuildSticker {
                guild_id,
                sticker_id,
            })
            .reason(reason),
        )
        .await
    }
}
