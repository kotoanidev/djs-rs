//! [Users](https://docs.discord.com/developers/resources/user).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{GuildId, SkuId, UserId},
    macros::{int_enum, int_flags, str_enum},
    util::{ImageHash, Locale},
};

/// A Discord user — a person or a bot account.
///
/// Users are global: they are not tied to a guild. Guild-specific data (nickname, roles, …)
/// lives on [guild members](crate::guild). This type holds the fields Discord sends for any
/// user; the logged-in user's own account is a [`CurrentUser`] (D-052).
///
/// Several fields are only present in some payloads: `banner` and `accent_color` only when a
/// user is fetched directly (`GET /users/{user.id}`), and message authors of webhooks carry
/// only the basic fields.
///
/// See the [user object](https://docs.discord.com/developers/resources/user#user-object).
///
/// discord.js counterpart: `User`.
///
/// # Examples
///
/// ```
/// use djs_model::user::User;
///
/// let user: User = serde_json::from_str(
///     r#"{"id":"80351110224678912","username":"nelly","discriminator":"0",
///         "global_name":"Nelly","avatar":null,"public_flags":64}"#,
/// )?;
/// assert_eq!(user.display_name(), "Nelly");
/// assert_eq!(user.tag(), "nelly");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct User {
    /// The user's ID.
    pub id: UserId,
    /// The user's username. Not unique across the platform for bots; unique for migrated users.
    pub username: CompactString,
    /// The legacy four-digit Discord tag, or `0` for users migrated to unique usernames.
    ///
    /// Discord sends this as a string (`"0"`, `"0420"`); it is stored as an integer.
    #[serde(with = "discriminator")]
    pub discriminator: u16,
    /// The user's display name, if set. For bots this is the application name.
    pub global_name: Option<CompactString>,
    /// The user's [avatar hash](https://docs.discord.com/developers/reference#image-formatting).
    pub avatar: Option<ImageHash>,
    /// Whether the user belongs to an `OAuth2` application (is a bot).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub bot: bool,
    /// Whether the user is an official Discord system user (part of the urgent message system).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub system: bool,
    /// The user's [banner hash](https://docs.discord.com/developers/reference#image-formatting).
    /// Only present when the user is fetched directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The banner color as an RGB integer (`0xRRGGBB`), used when there is no banner image.
    /// Only present when the user is fetched directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    /// The user's public [flags](UserFlags) (badges).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_flags: Option<UserFlags>,
    /// The user's avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The user's collectibles (nameplate, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
    /// The guild whose server tag the user displays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_guild: Option<Box<PrimaryGuild>>,
}

impl User {
    /// Returns the name to display for this user: the global display name if set, otherwise
    /// the username.
    ///
    /// discord.js counterpart: `User#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        display_name(&self.username, self.global_name.as_deref())
    }

    /// Returns the user's tag: `username#1234` for legacy users, or just `username` for users
    /// migrated to unique usernames (discriminator `0`).
    ///
    /// discord.js counterpart: `User#tag`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::user::User;
    ///
    /// let legacy: User = serde_json::from_str(
    ///     r#"{"id":"1","username":"nelly","discriminator":"0042","global_name":null,"avatar":null}"#,
    /// )?;
    /// assert_eq!(legacy.tag(), "nelly#0042");
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    #[must_use]
    pub fn tag(&self) -> CompactString {
        tag(&self.username, self.discriminator)
    }
}

/// The user account the client is logged in as, with the fields Discord only sends for it
/// (D-052).
///
/// Received in [`Ready`](crate::gateway::payload::Ready) and from `GET /users/@me`. With an
/// `OAuth2` bearer token, `email` and `verified` require the `email` scope. Bots receive these
/// fields too (`email` is then `None`).
///
/// See the [user object](https://docs.discord.com/developers/resources/user#user-object).
///
/// discord.js counterpart: `ClientUser`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentUser {
    /// The user's ID.
    pub id: UserId,
    /// The user's username.
    pub username: CompactString,
    /// The legacy four-digit Discord tag, or `0` for migrated users. See
    /// [`User::discriminator`].
    #[serde(with = "discriminator")]
    pub discriminator: u16,
    /// The user's display name, if set.
    pub global_name: Option<CompactString>,
    /// The user's [avatar hash](https://docs.discord.com/developers/reference#image-formatting).
    pub avatar: Option<ImageHash>,
    /// Whether the user is a bot.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub bot: bool,
    /// Whether the user is an official Discord system user.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub system: bool,
    /// Whether the account has two-factor authentication enabled.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub mfa_enabled: bool,
    /// The user's [banner hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The banner color as an RGB integer (`0xRRGGBB`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    /// The language the user chose in the client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<Locale>,
    /// Whether the account's email address is verified.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub verified: bool,
    /// The account's email address (`OAuth2` `email` scope; always `None` for bots).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<CompactString>,
    /// All [flags](UserFlags) on the account, including non-public ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<UserFlags>,
    /// The account's Nitro subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_type: Option<PremiumType>,
    /// The public [flags](UserFlags) (badges).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_flags: Option<UserFlags>,
    /// The user's avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The user's collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
    /// The guild whose server tag the user displays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_guild: Option<Box<PrimaryGuild>>,
}

impl CurrentUser {
    /// Returns the global display name if set, otherwise the username.
    ///
    /// discord.js counterpart: `User#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        display_name(&self.username, self.global_name.as_deref())
    }

    /// Returns the tag: `username#1234` for legacy users, `username` for migrated users.
    ///
    /// discord.js counterpart: `User#tag`.
    #[must_use]
    pub fn tag(&self) -> CompactString {
        tag(&self.username, self.discriminator)
    }
}

impl From<CurrentUser> for User {
    /// Keeps the fields every user has and drops the account-only ones.
    fn from(user: CurrentUser) -> Self {
        Self {
            id: user.id,
            username: user.username,
            discriminator: user.discriminator,
            global_name: user.global_name,
            avatar: user.avatar,
            bot: user.bot,
            system: user.system,
            banner: user.banner,
            accent_color: user.accent_color,
            public_flags: user.public_flags,
            avatar_decoration_data: user.avatar_decoration_data,
            collectibles: user.collectibles,
            primary_guild: user.primary_guild,
        }
    }
}

fn display_name<'a>(username: &'a str, global_name: Option<&'a str>) -> &'a str {
    global_name.unwrap_or(username)
}

fn tag(username: &str, discriminator: u16) -> CompactString {
    if discriminator == 0 {
        username.into()
    } else {
        compact_str::format_compact!("{username}#{discriminator:04}")
    }
}

/// An [avatar decoration](https://docs.discord.com/developers/resources/user#avatar-decoration-data-object):
/// the animated frame shown around a user's avatar.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AvatarDecorationData {
    /// The [decoration's image hash](https://docs.discord.com/developers/reference#image-formatting).
    pub asset: ImageHash,
    /// The SKU the decoration belongs to.
    pub sku_id: SkuId,
}

/// The [collectibles](https://docs.discord.com/developers/resources/user#collectibles) a user
/// has equipped.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Collectibles {
    /// The nameplate shown behind the user's name in member lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nameplate: Option<Nameplate>,
}

/// A [nameplate](https://docs.discord.com/developers/resources/user#nameplate): the background
/// shown behind a user's name in member lists.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Nameplate {
    /// The nameplate's SKU.
    pub sku_id: SkuId,
    /// The path of the [nameplate asset](https://docs.discord.com/developers/reference#image-formatting).
    pub asset: CompactString,
    /// The nameplate's label (currently unused by Discord).
    pub label: CompactString,
    /// The background color.
    pub palette: NameplatePalette,
}

str_enum! {
    /// The background color of a [`Nameplate`].
    pub enum NameplatePalette {
        /// Crimson.
        Crimson = "crimson",
        /// Berry.
        Berry = "berry",
        /// Sky.
        Sky = "sky",
        /// Teal.
        Teal = "teal",
        /// Forest.
        Forest = "forest",
        /// Bubble gum.
        BubbleGum = "bubble_gum",
        /// Violet.
        Violet = "violet",
        /// Cobalt (seen in the docs' example user; not in the documented list).
        Cobalt = "cobalt",
        /// Clover.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        Clover = "clover",
        /// Lemon.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        Lemon = "lemon",
        /// White.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        White = "white",
    }
}

/// The [primary guild](https://docs.discord.com/developers/resources/user#user-object-user-primary-guild)
/// of a user: the guild whose server tag is shown next to their name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PrimaryGuild {
    /// The guild's ID.
    pub identity_guild_id: Option<GuildId>,
    /// Whether the user displays the server tag. `None` when Discord cleared the identity (for
    /// example because the guild no longer supports tags).
    pub identity_enabled: Option<bool>,
    /// The server tag (up to 4 characters).
    pub tag: Option<CompactString>,
    /// The [server tag badge hash](https://docs.discord.com/developers/reference#image-formatting).
    pub badge: Option<ImageHash>,
}

int_enum! {
    /// The [Nitro subscription](https://docs.discord.com/developers/resources/user#user-object-premium-types)
    /// of a user.
    ///
    /// discord.js counterpart: `UserPremiumType`.
    pub enum PremiumType: u8 {
        /// No Nitro.
        None = 0,
        /// Nitro Classic.
        NitroClassic = 1,
        /// Nitro.
        Nitro = 2,
        /// Nitro Basic.
        NitroBasic = 3,
    }
}

/// (De)serializes a discriminator: a string of digits on the wire, `u16` in memory.
mod discriminator {
    use core::fmt;

    use serde::{
        Deserializer, Serializer,
        de::{Error, Unexpected, Visitor},
    };

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "signature required by serde(with)"
    )]
    pub(super) fn serialize<S: Serializer>(value: &u16, serializer: S) -> Result<S::Ok, S::Error> {
        if *value == 0 {
            serializer.serialize_str("0")
        } else {
            serializer.collect_str(&format_args!("{value:04}"))
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u16, D::Error> {
        struct DiscriminatorVisitor;

        impl Visitor<'_> for DiscriminatorVisitor {
            type Value = u16;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a discriminator of up to four digits")
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<u16, E> {
                if value.is_empty() || value.len() > 4 || !value.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(E::invalid_value(Unexpected::Str(value), &self));
                }
                value
                    .parse()
                    .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            }

            fn visit_u64<E: Error>(self, value: u64) -> Result<u16, E> {
                u16::try_from(value)
                    .ok()
                    .filter(|value| *value <= 9999)
                    .ok_or_else(|| E::invalid_value(Unexpected::Unsigned(value), &self))
            }
        }

        deserializer.deserialize_any(DiscriminatorVisitor)
    }
}

int_flags! {
    /// [Flags on a user](https://docs.discord.com/developers/resources/user#user-object-user-flags),
    /// most of them shown as profile badges.
    ///
    /// Users carry these in `public_flags` (visible to everyone) and, for the current user
    /// with the right OAuth2 scope, in `flags`.
    ///
    /// discord.js counterpart: `UserFlags` / `UserFlagsBitField`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::user::UserFlags;
    ///
    /// let flags: UserFlags = serde_json::from_str("65536").unwrap();
    /// assert_eq!(flags, UserFlags::VERIFIED_BOT);
    /// ```
    pub struct UserFlags: u64 {
        /// Discord employee.
        const STAFF = 1 << 0;
        /// Owner of a partnered server.
        const PARTNER = 1 << 1;
        /// Member of HypeSquad Events.
        const HYPESQUAD = 1 << 2;
        /// Bug Hunter, level 1.
        const BUG_HUNTER_LEVEL_1 = 1 << 3;
        /// The account has SMS two-factor authentication enabled.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const MFA_SMS = 1 << 4;
        /// The user dismissed a Nitro promotion.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const PREMIUM_PROMO_DISMISSED = 1 << 5;
        /// Member of HypeSquad house Bravery.
        const HYPESQUAD_ONLINE_HOUSE_1 = 1 << 6;
        /// Member of HypeSquad house Brilliance.
        const HYPESQUAD_ONLINE_HOUSE_2 = 1 << 7;
        /// Member of HypeSquad house Balance.
        const HYPESQUAD_ONLINE_HOUSE_3 = 1 << 8;
        /// Early Nitro supporter.
        const PREMIUM_EARLY_SUPPORTER = 1 << 9;
        /// The "user" is a [team](https://docs.discord.com/developers/topics/teams) account.
        const TEAM_PSEUDO_USER = 1 << 10;
        /// The user has unread urgent messages from Discord.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const HAS_UNREAD_URGENT_MESSAGES = 1 << 13;
        /// Bug Hunter, level 2.
        const BUG_HUNTER_LEVEL_2 = 1 << 14;
        /// A verified bot.
        const VERIFIED_BOT = 1 << 16;
        /// Early verified bot developer.
        const VERIFIED_DEVELOPER = 1 << 17;
        /// Alumnus of the Discord Moderator Programs.
        const CERTIFIED_MODERATOR = 1 << 18;
        /// A bot that only uses HTTP interactions; it is shown in the online member list.
        const BOT_HTTP_INTERACTIONS = 1 << 19;
        /// The account is flagged as a spammer.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const SPAMMER = 1 << 20;
        /// Nitro features are disabled for the account.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const DISABLE_PREMIUM = 1 << 21;
        /// Holder of the Active Developer badge.
        ///
        /// Discord no longer grants this flag.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const ACTIVE_DEVELOPER = 1 << 22;
        /// The account is quarantined.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const QUARANTINED = 1 << 44;
        /// The user is a collaborator on an application team.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const COLLABORATOR = 1 << 50;
        /// The user is a restricted collaborator on an application team.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const RESTRICTED_COLLABORATOR = 1 << 51;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example user object from the Discord API documentation.
    const DOCS_EXAMPLE: &str = r#"{
        "id": "80351110224678912",
        "username": "Nelly",
        "global_name": null,
        "discriminator": "1337",
        "avatar": "8342729096ea3675442027381ff50dfe",
        "verified": true,
        "email": "nelly@discord.com",
        "flags": 64,
        "banner": "06c16474723fe537c283b8efa61a30c8",
        "accent_color": 16711680,
        "premium_type": 0,
        "public_flags": 64,
        "avatar_decoration_data": {
            "sku_id": "1144058844004233369",
            "asset": "a_fed43ab12698df65902ba06727e20c0e"
        },
        "collectibles": {
            "nameplate": {
                "sku_id": "2247558840304243311",
                "asset": "nameplates/nameplates/twilight/",
                "label": "",
                "palette": "cobalt"
            }
        },
        "primary_guild": {
            "identity_guild_id": "1234647491267808778",
            "identity_enabled": true,
            "tag": "DISC",
            "badge": "7d1734ae5a615e82bc7a4033b98fade8"
        }
    }"#;

    #[test]
    fn deserialize_documented_example_as_user() {
        let user: User = serde_json::from_str(DOCS_EXAMPLE).unwrap();
        assert_eq!(user.id.get(), 80_351_110_224_678_912);
        assert_eq!(user.username, "Nelly");
        assert_eq!(user.discriminator, 1337);
        assert_eq!(user.global_name, None);
        assert_eq!(
            user.avatar.unwrap().to_string(),
            "8342729096ea3675442027381ff50dfe"
        );
        assert!(!user.bot);
        assert_eq!(user.accent_color, Some(0xFF_0000));
        assert_eq!(user.public_flags, Some(UserFlags::HYPESQUAD_ONLINE_HOUSE_1));
        let decoration = user.avatar_decoration_data.as_ref().unwrap();
        assert_eq!(decoration.sku_id.get(), 1_144_058_844_004_233_369);
        assert!(decoration.asset.is_animated());
        let nameplate = user
            .collectibles
            .as_ref()
            .unwrap()
            .nameplate
            .as_ref()
            .unwrap();
        assert_eq!(nameplate.palette, NameplatePalette::Cobalt);
        assert_eq!(nameplate.asset, "nameplates/nameplates/twilight/");
        let guild = user.primary_guild.as_ref().unwrap();
        assert_eq!(guild.tag.as_deref(), Some("DISC"));
        assert_eq!(guild.identity_enabled, Some(true));
        assert_eq!(user.tag(), "Nelly#1337");
        assert_eq!(user.display_name(), "Nelly");
    }

    #[test]
    fn deserialize_documented_example_as_current_user() {
        let user: CurrentUser = serde_json::from_str(DOCS_EXAMPLE).unwrap();
        assert!(user.verified);
        assert_eq!(user.email.as_deref(), Some("nelly@discord.com"));
        assert_eq!(user.flags, Some(UserFlags::HYPESQUAD_ONLINE_HOUSE_1));
        assert_eq!(user.premium_type, Some(PremiumType::None));
        assert!(!user.mfa_enabled);
        assert_eq!(user.locale, None);
        let plain = User::from(user.clone());
        assert_eq!(plain, serde_json::from_str::<User>(DOCS_EXAMPLE).unwrap());
    }

    #[test]
    fn serialization_keeps_discord_shape() {
        let json = r#"{"id":"1","username":"bot","discriminator":"0007","global_name":"Bot","avatar":null,"bot":true}"#;
        let user: User = serde_json::from_str(json).unwrap();
        // `?type` fields are written as null, absent `field?` fields are skipped (D-049).
        assert_eq!(serde_json::to_string(&user).unwrap(), json);
    }

    #[test]
    fn missing_nullable_fields_are_accepted() {
        // Some partial user payloads omit `global_name`/`avatar` entirely.
        let user: User =
            serde_json::from_str(r#"{"id":"1","username":"a","discriminator":"0"}"#).unwrap();
        assert_eq!(user.global_name, None);
        assert_eq!(user.avatar, None);
    }

    #[test]
    fn migrated_user_discriminator() {
        let json =
            r#"{"id":"1","username":"a","discriminator":"0","global_name":null,"avatar":null}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.discriminator, 0);
        assert!(
            serde_json::to_string(&user)
                .unwrap()
                .contains(r#""discriminator":"0""#)
        );
    }

    #[test]
    fn rejects_bad_discriminator() {
        for bad in [r#""12345""#, r#""""#, r#""12a""#] {
            let json =
                format!(r#"{{"id":"1","username":"a","discriminator":{bad},"avatar":null}}"#);
            assert!(
                serde_json::from_str::<User>(&json).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn hot_type_sizes() {
        // Guard against accidental growth of the message author type.
        assert!(
            size_of::<User>() <= 256,
            "User is {} bytes",
            size_of::<User>()
        );
    }
}
