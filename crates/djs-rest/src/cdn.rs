//! [CDN](https://docs.discord.com/developers/reference#image-formatting) URLs of avatars,
//! icons, banners, emojis, stickers and other images (D-137).
//!
//! A [`Cdn`] knows the base URLs; [`Rest::cdn`](crate::Rest::cdn) returns the client's, and
//! [`Cdn::default`] works without a client. Every method returns the URL as a `String`; none
//! can fail, since formats and sizes are enums (D-139).
//!
//! Without an explicit [`format`](ImageOptions::format), images are WebP, and animated ones
//! (hashes starting with `a_`) are animated WebP (`.webp?animated=true`, D-138).
//!
//! discord.js counterpart: `CDN` of @discordjs/rest (`REST#cdn`).
//!
//! # Examples
//!
//! ```
//! use djs_model::{id::UserId, util::ImageHash};
//! use djs_rest::cdn::{Cdn, ImageFormat, ImageOptions, ImageSize};
//!
//! let cdn = Cdn::default();
//! let hash: ImageHash = "a_1269e74af4df7417b13759eae50c83dc".parse()?;
//! assert_eq!(
//!     cdn.user_avatar(UserId::new(7), hash, ImageOptions::new().size(ImageSize::S256)),
//!     "https://cdn.discordapp.com/avatars/7/a_1269e74af4df7417b13759eae50c83dc.webp?animated=true&size=256"
//! );
//! assert_eq!(
//!     cdn.user_avatar(UserId::new(7), hash, ImageOptions::new().format(ImageFormat::Png)),
//!     "https://cdn.discordapp.com/avatars/7/a_1269e74af4df7417b13759eae50c83dc.png"
//! );
//! # Ok::<(), djs_model::util::ParseImageHashError>(())
//! ```

use core::fmt::{self, Write as _};

use djs_model::{
    id::{
        ApplicationId, ChannelId, EmojiId, GenericId, GuildId, RoleId, ScheduledEventId,
        SoundboardSoundId, StickerId, TeamId, UserId,
    },
    sticker::StickerFormatType,
    util::ImageHash,
};

/// The default CDN base URL.
pub const DEFAULT_CDN: &str = "https://cdn.discordapp.com";

/// The default media proxy base URL (GIF stickers are served from there).
pub const DEFAULT_MEDIA_PROXY: &str = "https://media.discordapp.net";

/// The application that owns the sticker pack banners (from Discord's docs).
const STICKER_PACK_APPLICATION: u64 = 710_982_414_301_790_216;

/// An [image format](https://docs.discord.com/developers/reference#image-formatting-image-formats)
/// (the file extension of the URL, D-139). Which formats an endpoint supports is listed on its
/// method; GIF is only animated for animated hashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// `.webp`, the default (D-138).
    Webp,
    /// `.png`.
    Png,
    /// `.jpg`.
    Jpeg,
    /// `.gif`.
    Gif,
}

impl ImageFormat {
    /// The file extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Webp => "webp",
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
        }
    }
}

/// The size of an image: a power of two from 16 to 4096 pixels (D-139).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ImageSize {
    /// 16 pixels.
    S16,
    /// 32 pixels.
    S32,
    /// 64 pixels.
    S64,
    /// 128 pixels.
    S128,
    /// 256 pixels.
    S256,
    /// 512 pixels.
    S512,
    /// 1024 pixels.
    S1024,
    /// 2048 pixels.
    S2048,
    /// 4096 pixels.
    S4096,
}

impl ImageSize {
    /// The size in pixels.
    #[must_use]
    pub const fn pixels(self) -> u16 {
        match self {
            Self::S16 => 16,
            Self::S32 => 32,
            Self::S64 => 64,
            Self::S128 => 128,
            Self::S256 => 256,
            Self::S512 => 512,
            Self::S1024 => 1024,
            Self::S2048 => 2048,
            Self::S4096 => 4096,
        }
    }

    /// The size for `pixels`, if it is a power of two from 16 to 4096.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rest::cdn::ImageSize;
    ///
    /// assert_eq!(ImageSize::from_pixels(512), Some(ImageSize::S512));
    /// assert_eq!(ImageSize::from_pixels(500), None);
    /// ```
    #[must_use]
    pub const fn from_pixels(pixels: u16) -> Option<Self> {
        Some(match pixels {
            16 => Self::S16,
            32 => Self::S32,
            64 => Self::S64,
            128 => Self::S128,
            256 => Self::S256,
            512 => Self::S512,
            1024 => Self::S1024,
            2048 => Self::S2048,
            4096 => Self::S4096,
            _ => return None,
        })
    }
}

/// The format and size of an image URL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[must_use]
pub struct ImageOptions {
    /// The format; `None` is WebP, animated for animated images (D-138).
    pub format: Option<ImageFormat>,
    /// The size; `None` lets Discord pick the original size.
    pub size: Option<ImageSize>,
    /// Whether animated images are returned as a still image.
    pub force_static: bool,
}

impl ImageOptions {
    /// The default options: WebP (animated when the image is), original size.
    pub const fn new() -> Self {
        Self {
            format: None,
            size: None,
            force_static: false,
        }
    }

    /// Sets the format.
    pub const fn format(mut self, format: ImageFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Sets the size.
    pub const fn size(mut self, size: ImageSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Returns a still image even for an animated one.
    pub const fn force_static(mut self, force_static: bool) -> Self {
        self.force_static = force_static;
        self
    }
}

/// The index of the [default avatar](https://docs.discord.com/developers/reference#image-formatting-cdn-endpoints)
/// of a user without one: `(user_id >> 22) % 6` for users on the new username system
/// (discriminator `0`), `discriminator % 5` for the others.
///
/// # Examples
///
/// ```
/// use djs_model::id::UserId;
/// use djs_rest::cdn::default_avatar_index;
///
/// assert_eq!(default_avatar_index(UserId::new(80_351_110_224_678_912), 1337), 2);
/// assert_eq!(default_avatar_index(UserId::new(80_351_110_224_678_912), 0), 5);
/// ```
#[must_use]
pub const fn default_avatar_index(user_id: UserId, discriminator: u16) -> u8 {
    if discriminator == 0 {
        ((user_id.get() >> 22) % 6) as u8
    } else {
        (discriminator % 5) as u8
    }
}

/// Builds CDN URLs (D-137). See the [module](self) documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cdn {
    cdn: String,
    media_proxy: String,
}

impl Default for Cdn {
    /// Discord's CDN and media proxy.
    fn default() -> Self {
        Self::new(DEFAULT_CDN, DEFAULT_MEDIA_PROXY)
    }
}

impl Cdn {
    /// A CDN with other base URLs (e.g. a proxy). Trailing slashes are ignored.
    #[must_use]
    pub fn new(cdn: impl Into<String>, media_proxy: impl Into<String>) -> Self {
        let trim = |mut url: String| {
            let len = url.trim_end_matches('/').len();
            url.truncate(len);
            url
        };
        Self {
            cdn: trim(cdn.into()),
            media_proxy: trim(media_proxy.into()),
        }
    }

    /// The CDN base URL.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.cdn
    }

    /// The media proxy base URL.
    #[must_use]
    pub fn media_proxy(&self) -> &str {
        &self.media_proxy
    }

    /// A custom emoji (PNG, JPEG, WebP, GIF). Emojis have no hash, so pass whether the emoji
    /// is [`animated`](djs_model::emoji::Emoji::animated). Discord recommends WebP for emojis.
    ///
    /// discord.js counterpart: `CDN#emoji`.
    #[must_use]
    pub fn emoji(&self, emoji_id: EmojiId, animated: bool, options: ImageOptions) -> String {
        self.image(format_args!("emojis/{emoji_id}"), animated, options)
    }

    /// A guild's icon (PNG, JPEG, WebP, GIF).
    ///
    /// discord.js counterpart: `CDN#icon`, `Guild#iconURL`.
    #[must_use]
    pub fn guild_icon(&self, guild_id: GuildId, hash: ImageHash, options: ImageOptions) -> String {
        self.hashed(format_args!("icons/{guild_id}"), hash, options)
    }

    /// A guild's invite splash (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#splash`, `Guild#splashURL`.
    #[must_use]
    pub fn guild_splash(
        &self,
        guild_id: GuildId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("splashes/{guild_id}"), hash, options)
    }

    /// A guild's discovery splash (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#discoverySplash`, `Guild#discoverySplashURL`.
    #[must_use]
    pub fn guild_discovery_splash(
        &self,
        guild_id: GuildId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("discovery-splashes/{guild_id}"), hash, options)
    }

    /// A guild's banner (PNG, JPEG, WebP, GIF).
    ///
    /// discord.js counterpart: `CDN#banner`, `Guild#bannerURL`.
    #[must_use]
    pub fn guild_banner(
        &self,
        guild_id: GuildId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("banners/{guild_id}"), hash, options)
    }

    /// A user's profile banner (PNG, JPEG, WebP, GIF).
    ///
    /// discord.js counterpart: `CDN#banner`, `User#bannerURL`.
    #[must_use]
    pub fn user_banner(&self, user_id: UserId, hash: ImageHash, options: ImageOptions) -> String {
        self.hashed(format_args!("banners/{user_id}"), hash, options)
    }

    /// The default avatar `index` (0-5, see [`default_avatar_index`]) of users without one
    /// (always PNG of a fixed size).
    ///
    /// discord.js counterpart: `CDN#defaultAvatar`, `User#defaultAvatarURL`.
    #[must_use]
    pub fn default_user_avatar(&self, index: u8) -> String {
        Self::fixed(&self.cdn, format_args!("embed/avatars/{index}"), "png")
    }

    /// A user's avatar (PNG, JPEG, WebP, GIF).
    ///
    /// discord.js counterpart: `CDN#avatar`, `User#avatarURL`.
    #[must_use]
    pub fn user_avatar(&self, user_id: UserId, hash: ImageHash, options: ImageOptions) -> String {
        self.hashed(format_args!("avatars/{user_id}"), hash, options)
    }

    /// A member's guild-specific avatar (PNG, JPEG, WebP, GIF).
    ///
    /// discord.js counterpart: `CDN#guildMemberAvatar`, `GuildMember#avatarURL`.
    #[must_use]
    pub fn guild_member_avatar(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(
            format_args!("guilds/{guild_id}/users/{user_id}/avatars"),
            hash,
            options,
        )
    }

    /// A member's guild-specific banner (PNG, JPEG, WebP, GIF).
    ///
    /// discord.js counterpart: `CDN#guildMemberBanner`, `GuildMember#bannerURL`.
    #[must_use]
    pub fn guild_member_banner(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(
            format_args!("guilds/{guild_id}/users/{user_id}/banners"),
            hash,
            options,
        )
    }

    /// An avatar decoration, from its [`asset`](djs_model::user::AvatarDecorationData::asset)
    /// (always PNG).
    ///
    /// discord.js counterpart: `CDN#avatarDecoration`, `User#avatarDecorationURL`.
    #[must_use]
    pub fn avatar_decoration(&self, asset: ImageHash) -> String {
        Self::fixed(
            &self.cdn,
            format_args!("avatar-decoration-presets/{asset}"),
            "png",
        )
    }

    /// An application's icon (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#appIcon`, `Application#iconURL`.
    #[must_use]
    pub fn application_icon(
        &self,
        application_id: ApplicationId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("app-icons/{application_id}"), hash, options)
    }

    /// An application's default rich presence invite cover (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#appIcon`, `Application#coverURL`.
    #[must_use]
    pub fn application_cover(
        &self,
        application_id: ApplicationId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("app-icons/{application_id}"), hash, options)
    }

    /// An application's rich presence asset, e.g. an activity's `large_image` (PNG, JPEG,
    /// WebP).
    ///
    /// discord.js counterpart: `CDN#appAsset`, `RichPresenceAssets#largeImageURL`.
    #[must_use]
    pub fn application_asset(
        &self,
        application_id: ApplicationId,
        asset_id: &str,
        options: ImageOptions,
    ) -> String {
        self.image(
            format_args!("app-assets/{application_id}/{asset_id}"),
            false,
            options,
        )
    }

    /// An achievement's icon (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: none.
    #[must_use]
    pub fn achievement_icon(
        &self,
        application_id: ApplicationId,
        achievement_id: GenericId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(
            format_args!("app-assets/{application_id}/achievements/{achievement_id}/icons"),
            hash,
            options,
        )
    }

    /// An asset of an application's store page (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: none.
    #[must_use]
    pub fn store_page_asset(
        &self,
        application_id: ApplicationId,
        asset_id: GenericId,
        options: ImageOptions,
    ) -> String {
        self.image(
            format_args!("app-assets/{application_id}/store/{asset_id}"),
            false,
            options,
        )
    }

    /// A sticker pack's banner, from its
    /// [`banner_asset_id`](djs_model::sticker::StickerPack::banner_asset_id) (PNG, JPEG,
    /// WebP).
    ///
    /// discord.js counterpart: `CDN#stickerPackBanner`, `StickerPack#bannerURL`.
    #[must_use]
    pub fn sticker_pack_banner(&self, banner_asset_id: GenericId, options: ImageOptions) -> String {
        self.image(
            format_args!("app-assets/{STICKER_PACK_APPLICATION}/store/{banner_asset_id}"),
            false,
            options,
        )
    }

    /// A team's icon (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#teamIcon`, `Team#iconURL`.
    #[must_use]
    pub fn team_icon(&self, team_id: TeamId, hash: ImageHash, options: ImageOptions) -> String {
        self.hashed(format_args!("team-icons/{team_id}"), hash, options)
    }

    /// A sticker, in the format its `format_type` is served in (fixed size): PNG for PNG and
    /// APNG stickers, Lottie JSON for Lottie ones, and GIF — from the media proxy — for GIF
    /// ones.
    ///
    /// discord.js counterpart: `CDN#sticker`, `Sticker#url`.
    #[must_use]
    pub fn sticker(&self, sticker_id: StickerId, format_type: StickerFormatType) -> String {
        let path = format_args!("stickers/{sticker_id}");
        match format_type {
            StickerFormatType::Gif => Self::fixed(&self.media_proxy, path, "gif"),
            StickerFormatType::Lottie => Self::fixed(&self.cdn, path, "json"),
            _ => Self::fixed(&self.cdn, path, "png"),
        }
    }

    /// A role's icon (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#roleIcon`, `Role#iconURL`.
    #[must_use]
    pub fn role_icon(&self, role_id: RoleId, hash: ImageHash, options: ImageOptions) -> String {
        self.hashed(format_args!("role-icons/{role_id}"), hash, options)
    }

    /// A scheduled event's cover image (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#guildScheduledEventCover`, `GuildScheduledEvent#coverImageURL`.
    #[must_use]
    pub fn guild_scheduled_event_cover(
        &self,
        event_id: ScheduledEventId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("guild-events/{event_id}"), hash, options)
    }

    /// A guild's server tag badge, from a user's
    /// [`primary_guild`](djs_model::user::PrimaryGuild::badge) (PNG, JPEG, WebP).
    ///
    /// discord.js counterpart: `CDN#guildTagBadge`.
    #[must_use]
    pub fn guild_tag_badge(
        &self,
        guild_id: GuildId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("guild-tag-badges/{guild_id}"), hash, options)
    }

    /// A group DM's icon (PNG, JPEG, WebP). Not in Discord's table; the path discord.js uses.
    ///
    /// discord.js counterpart: `CDN#channelIcon`, `GroupDMChannel#iconURL`.
    #[must_use]
    pub fn channel_icon(
        &self,
        channel_id: ChannelId,
        hash: ImageHash,
        options: ImageOptions,
    ) -> String {
        self.hashed(format_args!("channel-icons/{channel_id}"), hash, options)
    }

    /// A soundboard sound's audio file (no extension; Discord serves MP3 or Ogg).
    ///
    /// discord.js counterpart: `CDN#soundboardSound`, `SoundboardSound#url`.
    #[must_use]
    pub fn soundboard_sound(&self, sound_id: SoundboardSoundId) -> String {
        format!("{}/soundboard-sounds/{sound_id}", self.cdn)
    }

    /// An image at `path` + `/hash`, animated when the hash is.
    fn hashed(&self, path: fmt::Arguments<'_>, hash: ImageHash, options: ImageOptions) -> String {
        self.image(format_args!("{path}/{hash}"), hash.is_animated(), options)
    }

    /// An image at `path` with the format and size of `options` (D-138).
    fn image(&self, path: fmt::Arguments<'_>, animated: bool, options: ImageOptions) -> String {
        let animated = animated && !options.force_static;
        let format = options.format.unwrap_or(ImageFormat::Webp);
        let mut url = String::with_capacity(self.cdn.len() + 96);
        // Writing to a `String` cannot fail.
        let _ = write!(url, "{}/{path}.{}", self.cdn, format.extension());
        let mut separator = '?';
        if animated && format == ImageFormat::Webp {
            url.push_str("?animated=true");
            separator = '&';
        }
        if let Some(size) = options.size {
            let _ = write!(url, "{separator}size={}", size.pixels());
        }
        url
    }

    /// An image of a fixed format and size.
    fn fixed(base: &str, path: fmt::Arguments<'_>, extension: &str) -> String {
        format!("{base}/{path}.{extension}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATIC: &str = "1269e74af4df7417b13759eae50c83dc";
    const ANIMATED: &str = "a_1269e74af4df7417b13759eae50c83dc";

    fn hash(value: &str) -> ImageHash {
        value.parse().unwrap()
    }

    #[test]
    fn formats_and_animation() {
        let cdn = Cdn::default();
        let guild = GuildId::new(1);
        let icon = |value, options| cdn.guild_icon(guild, hash(value), options);
        let base = "https://cdn.discordapp.com/icons/1";
        assert_eq!(
            icon(STATIC, ImageOptions::new()),
            format!("{base}/{STATIC}.webp")
        );
        assert_eq!(
            icon(ANIMATED, ImageOptions::new()),
            format!("{base}/{ANIMATED}.webp?animated=true")
        );
        assert_eq!(
            icon(
                ANIMATED,
                ImageOptions::new().force_static(true).size(ImageSize::S64)
            ),
            format!("{base}/{ANIMATED}.webp?size=64")
        );
        assert_eq!(
            icon(ANIMATED, ImageOptions::new().format(ImageFormat::Gif)),
            format!("{base}/{ANIMATED}.gif")
        );
        assert_eq!(
            icon(STATIC, ImageOptions::new().format(ImageFormat::Jpeg)),
            format!("{base}/{STATIC}.jpg")
        );
        assert_eq!(
            cdn.emoji(
                EmojiId::new(9),
                true,
                ImageOptions::new().size(ImageSize::S4096)
            ),
            "https://cdn.discordapp.com/emojis/9.webp?animated=true&size=4096"
        );
    }

    #[test]
    fn fixed_urls_and_bases() {
        let cdn = Cdn::new("https://cdn.example.com/", "https://media.example.com");
        assert_eq!(
            cdn.default_user_avatar(3),
            "https://cdn.example.com/embed/avatars/3.png"
        );
        assert_eq!(
            cdn.sticker(StickerId::new(5), StickerFormatType::Gif),
            "https://media.example.com/stickers/5.gif"
        );
        assert_eq!(
            cdn.sticker(StickerId::new(5), StickerFormatType::Lottie),
            "https://cdn.example.com/stickers/5.json"
        );
        assert_eq!(
            cdn.sticker(StickerId::new(5), StickerFormatType::Apng),
            "https://cdn.example.com/stickers/5.png"
        );
        assert_eq!(
            cdn.sticker_pack_banner(GenericId::new(8), ImageOptions::new()),
            "https://cdn.example.com/app-assets/710982414301790216/store/8.webp"
        );
        assert_eq!(
            cdn.soundboard_sound(SoundboardSoundId::new(4)),
            "https://cdn.example.com/soundboard-sounds/4"
        );
        assert_eq!(
            cdn.guild_member_avatar(
                GuildId::new(1),
                UserId::new(2),
                hash(STATIC),
                ImageOptions::new().format(ImageFormat::Png)
            ),
            format!("https://cdn.example.com/guilds/1/users/2/avatars/{STATIC}.png")
        );
    }

    #[test]
    fn sizes_round_trip() {
        for pixels in [16, 32, 64, 128, 256, 512, 1024, 2048, 4096] {
            assert_eq!(ImageSize::from_pixels(pixels).unwrap().pixels(), pixels);
        }
        assert_eq!(ImageSize::from_pixels(0), None);
        assert_eq!(ImageSize::from_pixels(8192), None);
    }
}
