//! [Users](https://docs.discord.com/developers/resources/user).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{id::UserId, macros::int_flags, util::ImageHash};

/// A Discord user — a person or a bot account.
///
/// Users are global: they are not tied to a guild. Guild-specific data (nickname, roles, …)
/// lives on guild members.
///
/// **Coverage:** this is the Phase 1 subset of the
/// [user object](https://docs.discord.com/developers/resources/user#user-object). Unknown
/// fields are ignored during deserialization, so newer payloads always parse.
///
/// discord.js counterpart: `User`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl User {
    /// Returns the name to display for this user: the global display name if set, otherwise
    /// the username.
    ///
    /// discord.js counterpart: `User#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }

    /// Returns the user's tag: `username#1234` for legacy users, or just `username` for users
    /// migrated to unique usernames (discriminator `0`).
    ///
    /// discord.js counterpart: `User#tag`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use djs_model::{id::UserId, user::User};
    /// # let mut user = User {
    /// #     id: UserId::new(1), username: "nelly".into(), discriminator: 0,
    /// #     global_name: None, avatar: None, bot: false, system: false,
    /// # };
    /// assert_eq!(user.tag(), "nelly");
    /// user.discriminator = 42;
    /// assert_eq!(user.tag(), "nelly#0042");
    /// ```
    #[must_use]
    pub fn tag(&self) -> CompactString {
        if self.discriminator == 0 {
            self.username.clone()
        } else {
            compact_str::format_compact!("{}#{:04}", self.username, self.discriminator)
        }
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

    #[test]
    fn deserialize_documented_example() {
        // Example user object from the Discord API documentation.
        let json = r#"{
            "id": "80351110224678912",
            "username": "Nelly",
            "discriminator": "1337",
            "avatar": "8342729096ea3675442027381ff50dfe",
            "verified": true,
            "email": "nelly@discord.com",
            "flags": 64,
            "banner": "06c16474723fe537c283b8efa61a30c8",
            "accent_color": 16711680,
            "premium_type": 1,
            "public_flags": 64,
            "avatar_decoration_data": {
                "sku_id": "1144058844004233369",
                "asset": "a_fed43ab12698df65902ba06727e20c0e"
            }
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id.get(), 80_351_110_224_678_912);
        assert_eq!(user.username, "Nelly");
        assert_eq!(user.discriminator, 1337);
        assert_eq!(user.global_name, None);
        assert_eq!(
            user.avatar.unwrap().to_string(),
            "8342729096ea3675442027381ff50dfe"
        );
        assert!(!user.bot);
        assert_eq!(user.tag(), "Nelly#1337");
        assert_eq!(user.display_name(), "Nelly");
    }

    #[test]
    fn serialize_round_trip() {
        let user = User {
            id: UserId::new(1),
            username: "bot".into(),
            discriminator: 7,
            global_name: Some("Bot".into()),
            avatar: None,
            bot: true,
            system: false,
        };
        let json = serde_json::to_string(&user).unwrap();
        assert_eq!(
            json,
            r#"{"id":"1","username":"bot","discriminator":"0007","global_name":"Bot","avatar":null,"bot":true}"#
        );
        assert_eq!(serde_json::from_str::<User>(&json).unwrap(), user);
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
}
