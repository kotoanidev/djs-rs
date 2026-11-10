//! [Guilds](https://docs.discord.com/developers/resources/guild) (servers).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{GuildId, IntegrationId, RoleId, SkuId, UserId},
    macros::int_flags,
    permissions::Permissions,
    user::{AvatarDecorationData, Collectibles, User},
    util::{ImageHash, Timestamp},
};

/// A guild that is not (or not yet) available to the client.
///
/// The [`Ready`](crate::gateway::payload::Ready) event lists every guild the bot is in as
/// unavailable; full guild data then arrives in `GUILD_CREATE` events. A guild also becomes
/// unavailable during a Discord outage.
///
/// See the [unavailable guild object](https://docs.discord.com/developers/resources/guild#unavailable-guild-object).
///
/// discord.js counterpart: guilds with `available === false`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnavailableGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// Whether the guild is unavailable due to an outage.
    ///
    /// In `GUILD_DELETE` events this is `false` (absent) when the bot was removed from the guild
    /// rather than the guild becoming unavailable.
    #[serde(default)]
    pub unavailable: bool,
}

/// A [guild member](https://docs.discord.com/developers/resources/guild#guild-member-object): a
/// user's membership in one guild, with guild-specific data such as nickname and roles.
///
/// This is the complete shape (it includes the [`user`](Self::user)), as returned by
/// `GET /guilds/{guild.id}/members/{user.id}` and member gateway events. Messages carry a
/// [`PartialMember`] instead (D-051).
///
/// discord.js counterpart: `GuildMember`.
///
/// # Examples
///
/// ```
/// use djs_model::guild::Member;
///
/// let member: Member = serde_json::from_str(r#"{
///     "user": {"id": "80351110224678912", "username": "nelly", "discriminator": "0",
///              "global_name": "Nelly", "avatar": null},
///     "nick": "NOT API SUPPORT", "roles": [], "joined_at": "2015-04-26T06:26:56.936000+00:00",
///     "deaf": false, "mute": false, "flags": 0
/// }"#)?;
/// assert_eq!(member.display_name(), "NOT API SUPPORT");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Member {
    /// The user this membership belongs to.
    pub user: User,
    /// The member's nickname in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
    /// The member's [guild avatar hash](https://docs.discord.com/developers/reference#image-formatting),
    /// overriding the user's avatar in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<ImageHash>,
    /// The member's [guild banner hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The IDs of the member's roles (the `@everyone` role is implied and not listed).
    pub roles: Vec<RoleId>,
    /// When the user joined the guild. `None` for guest members.
    pub joined_at: Option<Timestamp>,
    /// When the user started boosting the guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<Timestamp>,
    /// Whether the member is deafened in voice channels.
    #[serde(default)]
    pub deaf: bool,
    /// Whether the member is muted in voice channels.
    #[serde(default)]
    pub mute: bool,
    /// The member's [flags](MemberFlags).
    #[serde(default)]
    pub flags: MemberFlags,
    /// Whether the member has not yet passed the guild's membership screening. Such members
    /// cannot use most of the guild until they do.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub pending: bool,
    /// When the member's [timeout](https://support.discord.com/hc/en-us/articles/4413305239191)
    /// ends. A time in the past means the member is no longer timed out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Timestamp>,
    /// The member's guild-specific avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The member's guild-specific collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
}

impl Member {
    /// Returns the name shown for the member in this guild: the nickname, else the user's
    /// display name, else the username.
    ///
    /// discord.js counterpart: `GuildMember#displayName`.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }

    /// Returns whether the member is timed out at `now`.
    ///
    /// discord.js counterpart: `GuildMember#isCommunicationDisabled`.
    #[must_use]
    pub fn is_timed_out(&self, now: Timestamp) -> bool {
        self.communication_disabled_until
            .is_some_and(|until| until > now)
    }
}

/// A [guild member](https://docs.discord.com/developers/resources/guild#guild-member-object)
/// without its [`User`], as attached to messages (`MESSAGE_CREATE`, `MESSAGE_UPDATE`) where the
/// user is the message's author (D-051).
///
/// Combine it with the author through [`PartialMember::into_member`].
///
/// discord.js counterpart: the `member` of a `Message` (a `GuildMember`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PartialMember {
    /// The member's nickname in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
    /// The member's [guild avatar hash](https://docs.discord.com/developers/reference#image-formatting),
    /// overriding the user's avatar in this guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<ImageHash>,
    /// The member's [guild banner hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The IDs of the member's roles (the `@everyone` role is implied and not listed).
    pub roles: Vec<RoleId>,
    /// When the user joined the guild. `None` for guest members.
    pub joined_at: Option<Timestamp>,
    /// When the user started boosting the guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<Timestamp>,
    /// Whether the member is deafened in voice channels.
    #[serde(default)]
    pub deaf: bool,
    /// Whether the member is muted in voice channels.
    #[serde(default)]
    pub mute: bool,
    /// The member's [flags](MemberFlags).
    #[serde(default)]
    pub flags: MemberFlags,
    /// Whether the member has not yet passed the guild's membership screening. Such members
    /// cannot use most of the guild until they do.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub pending: bool,
    /// When the member's [timeout](https://support.discord.com/hc/en-us/articles/4413305239191)
    /// ends. A time in the past means the member is no longer timed out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Timestamp>,
    /// The member's guild-specific avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The member's guild-specific collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
}

impl PartialMember {
    /// Completes this member with its user, typically the author of the message the member
    /// came with.
    #[must_use]
    pub fn into_member(self, user: User) -> Member {
        Member {
            user,
            nick: self.nick,
            avatar: self.avatar,
            banner: self.banner,
            roles: self.roles,
            joined_at: self.joined_at,
            premium_since: self.premium_since,
            deaf: self.deaf,
            mute: self.mute,
            flags: self.flags,
            pending: self.pending,
            communication_disabled_until: self.communication_disabled_until,
            avatar_decoration_data: self.avatar_decoration_data,
            collectibles: self.collectibles,
        }
    }
}

/// A [role](https://docs.discord.com/developers/topics/permissions#role-object): a named set of
/// permissions (and a color, icon and position in the member list) that members can have.
///
/// The `@everyone` role has the guild's ID.
///
/// discord.js counterpart: `Role`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Role {
    /// The role's ID.
    pub id: RoleId,
    /// The role's name.
    pub name: CompactString,
    /// The role's primary color as an RGB integer. **Deprecated** by Discord in favor of
    /// [`colors`](Self::colors), but still sent.
    pub color: u32,
    /// The role's colors (solid, gradient or holographic).
    pub colors: RoleColors,
    /// Whether members with this role are listed separately in the member list.
    pub hoist: bool,
    /// The [role icon hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ImageHash>,
    /// The role's Unicode emoji, shown instead of an icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<CompactString>,
    /// The role's position in the role list; roles with equal positions are ordered by ID.
    pub position: i32,
    /// The permissions the role grants.
    pub permissions: Permissions,
    /// Whether the role is managed by an integration (bot role, booster role, …).
    pub managed: bool,
    /// Whether everyone can mention the role.
    pub mentionable: bool,
    /// What the role is tied to (a bot, an integration, boosting, a subscription, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Box<RoleTags>>,
    /// The role's [flags](RoleFlags).
    #[serde(default)]
    pub flags: RoleFlags,
}

/// The [colors of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-colors-object).
///
/// A role without colors has `primary_color == 0` and does not affect the member's color.
/// Secondary and tertiary colors need the guild feature `ENHANCED_ROLE_COLORS`.
///
/// discord.js counterpart: `RoleColors` (`Role#colors`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleColors {
    /// The primary color as an RGB integer.
    pub primary_color: u32,
    /// The secondary color: makes the role a gradient between the colors.
    pub secondary_color: Option<u32>,
    /// The tertiary color: makes the gradient holographic.
    pub tertiary_color: Option<u32>,
}

impl RoleColors {
    /// The only colors Discord accepts for a holographic role (when a tertiary color is sent).
    ///
    /// discord.js counterpart: `Constants.HolographicStyle`.
    pub const HOLOGRAPHIC: Self = Self {
        primary_color: 11_127_295,
        secondary_color: Some(16_759_788),
        tertiary_color: Some(16_761_760),
    };

    /// A solid color.
    #[must_use]
    pub const fn solid(color: u32) -> Self {
        Self {
            primary_color: color,
            secondary_color: None,
            tertiary_color: None,
        }
    }
}

/// The [tags of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-tags-structure):
/// what the role belongs to.
///
/// On the wire, the boolean tags are keys whose value is `null` when true and that are absent
/// when false; here they are plain `bool`s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoleTags {
    /// The bot this role belongs to (the bot's managed role).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<UserId>,
    /// The integration this role belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<IntegrationId>,
    /// Whether this is the guild's Booster role.
    #[serde(
        default,
        with = "null_flag",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub premium_subscriber: bool,
    /// The SKU and listing of the role's subscription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_listing_id: Option<SkuId>,
    /// Whether the role can be purchased.
    #[serde(
        default,
        with = "null_flag",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub available_for_purchase: bool,
    /// Whether the role is a guild's linked role.
    #[serde(
        default,
        with = "null_flag",
        skip_serializing_if = "core::ops::Not::not"
    )]
    pub guild_connections: bool,
}

/// (De)serializes a "null means true" flag: the key is present with a `null` value when the
/// flag is set and absent otherwise (absence is handled by `#[serde(default)]` and
/// `skip_serializing_if`).
mod null_flag {
    use serde::{Deserialize, Deserializer, Serializer};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "signature required by serde(with)"
    )]
    pub(super) fn serialize<S: Serializer>(_: &bool, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<bool, D::Error> {
        Option::<()>::deserialize(deserializer).map(|_| true)
    }
}

int_flags! {
    /// [Flags of a guild's system channel](https://docs.discord.com/developers/resources/guild#guild-object-system-channel-flags):
    /// which automatic messages Discord does **not** post there.
    ///
    /// discord.js counterpart: `GuildSystemChannelFlags` / `SystemChannelFlagsBitField`.
    pub struct SystemChannelFlags: u64 {
        /// No message when a member joins.
        const SUPPRESS_JOIN_NOTIFICATIONS = 1 << 0;
        /// No message when the guild is boosted.
        const SUPPRESS_PREMIUM_SUBSCRIPTIONS = 1 << 1;
        /// No server setup tips.
        const SUPPRESS_GUILD_REMINDER_NOTIFICATIONS = 1 << 2;
        /// No sticker reply button on join messages.
        const SUPPRESS_JOIN_NOTIFICATION_REPLIES = 1 << 3;
        /// No message when a role subscription is purchased or renewed.
        const SUPPRESS_ROLE_SUBSCRIPTION_PURCHASE_NOTIFICATIONS = 1 << 4;
        /// No sticker reply button on role subscription messages.
        const SUPPRESS_ROLE_SUBSCRIPTION_PURCHASE_NOTIFICATION_REPLIES = 1 << 5;
    }
}

int_flags! {
    /// [Flags of a guild member](https://docs.discord.com/developers/resources/guild#guild-member-object-guild-member-flags).
    ///
    /// Only [`BYPASSES_VERIFICATION`](Self::BYPASSES_VERIFICATION) can be changed by bots
    /// (when modifying a member); the others are set by Discord.
    ///
    /// discord.js counterpart: `GuildMemberFlags` / `GuildMemberFlagsBitField`.
    pub struct MemberFlags: u64 {
        /// The member left the guild and joined again.
        const DID_REJOIN = 1 << 0;
        /// The member completed onboarding.
        const COMPLETED_ONBOARDING = 1 << 1;
        /// The member is exempt from the guild's verification requirements. Editable.
        const BYPASSES_VERIFICATION = 1 << 2;
        /// The member started onboarding.
        const STARTED_ONBOARDING = 1 << 3;
        /// The member is a guest who can only access the voice channel they were invited to.
        const IS_GUEST = 1 << 4;
        /// The member started the Server Guide new-member actions.
        const STARTED_HOME_ACTIONS = 1 << 5;
        /// The member completed the Server Guide new-member actions.
        const COMPLETED_HOME_ACTIONS = 1 << 6;
        /// AutoMod blocks the member's username, display name or nickname.
        const AUTOMOD_QUARANTINED_USERNAME = 1 << 7;
        /// AutoMod blocks the member's bio.
        ///
        /// **Deprecated** by Discord.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const AUTOMOD_QUARANTINED_BIO = 1 << 8;
        /// The member dismissed the DM settings upsell.
        const DM_SETTINGS_UPSELL_ACKNOWLEDGED = 1 << 9;
        /// AutoMod blocks the member's guild tag.
        const AUTOMOD_QUARANTINED_GUILD_TAG = 1 << 10;
    }
}

int_flags! {
    /// [Flags of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-flags).
    ///
    /// discord.js counterpart: `RoleFlags` / `RoleFlagsBitField`.
    pub struct RoleFlags: u64 {
        /// Members can pick the role in an onboarding prompt.
        const IN_PROMPT = 1 << 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = r#"{"id":"80351110224678912","username":"nelly","discriminator":"0","global_name":"Nelly","avatar":null}"#;

    #[test]
    fn member_from_documented_example() {
        // The docs example with its empty `user` filled in.
        let json = format!(
            r#"{{"user":{USER},"nick":"NOT API SUPPORT","avatar":null,"banner":null,"roles":[],
                "joined_at":"2015-04-26T06:26:56.936000+00:00","deaf":false,"mute":false}}"#
        );
        let member: Member = serde_json::from_str(&json).unwrap();
        assert_eq!(member.user.id.get(), 80_351_110_224_678_912);
        assert_eq!(member.nick.as_deref(), Some("NOT API SUPPORT"));
        assert_eq!(member.avatar, None);
        assert_eq!(
            member.joined_at.unwrap().to_string(),
            "2015-04-26T06:26:56.936000+00:00"
        );
        assert_eq!(member.flags, MemberFlags::empty());
        assert!(!member.pending);
        assert_eq!(member.display_name(), "NOT API SUPPORT");
    }

    #[test]
    fn partial_member_from_message_create() {
        // The `member` of a real MESSAGE_CREATE payload: no `user`.
        let json = r#"{"roles":["1100000000000000001"],"premium_since":null,"pending":false,
            "nick":null,"mute":false,"joined_at":"2024-05-01T12:00:00.000000+00:00","flags":2,
            "deaf":false,"communication_disabled_until":"2026-11-10T20:00:00.000000+00:00",
            "banner":null,"avatar":null}"#;
        let partial: PartialMember = serde_json::from_str(json).unwrap();
        assert_eq!(partial.roles.len(), 1);
        assert_eq!(partial.flags, MemberFlags::COMPLETED_ONBOARDING);
        let user: User = serde_json::from_str(USER).unwrap();
        let member = partial.into_member(user);
        assert_eq!(member.display_name(), "Nelly");
        let before: Timestamp = "2026-11-10T19:00:00.000000+00:00".parse().unwrap();
        let after: Timestamp = "2026-11-10T21:00:00.000000+00:00".parse().unwrap();
        assert!(member.is_timed_out(before));
        assert!(!member.is_timed_out(after));
    }

    #[test]
    fn role_from_documented_example() {
        let json = r#"{"id":"41771983423143936","name":"WE DEM BOYZZ!!!!!!","color":3447003,
            "colors":{"primary_color":3447003,"secondary_color":null,"tertiary_color":null},
            "hoist":true,"icon":"cf3ced8600b777c9486c6d8d84fb4327","unicode_emoji":null,
            "position":1,"permissions":"66321471","managed":false,"mentionable":false,"flags":0}"#;
        let role: Role = serde_json::from_str(json).unwrap();
        assert_eq!(role.colors, RoleColors::solid(3_447_003));
        assert_eq!(role.permissions.bits(), 66_321_471);
        assert!(role.permissions.contains(Permissions::SEND_MESSAGES));
        assert_eq!(role.position, 1);
        assert!(role.tags.is_none());
        assert_eq!(
            role.icon.unwrap().to_string(),
            "cf3ced8600b777c9486c6d8d84fb4327"
        );
    }

    #[test]
    fn role_tags_null_flags() {
        let json = r#"{"bot_id":"1","premium_subscriber":null,"guild_connections":null}"#;
        let tags: RoleTags = serde_json::from_str(json).unwrap();
        assert_eq!(tags.bot_id.map(crate::id::Id::get), Some(1));
        assert!(tags.premium_subscriber);
        assert!(tags.guild_connections);
        assert!(!tags.available_for_purchase);
        // Set flags are written back as `null`, unset flags are omitted.
        assert_eq!(serde_json::to_string(&tags).unwrap(), json);
    }

    #[test]
    fn holographic_colors_serialize_like_the_docs() {
        assert_eq!(
            serde_json::to_string(&RoleColors::HOLOGRAPHIC).unwrap(),
            r#"{"primary_color":11127295,"secondary_color":16759788,"tertiary_color":16761760}"#
        );
    }

    #[test]
    fn hot_type_sizes() {
        assert!(
            size_of::<Member>() <= 304,
            "Member is {} bytes",
            size_of::<Member>()
        );
        assert!(
            size_of::<PartialMember>() <= 160,
            "PartialMember is {} bytes",
            size_of::<PartialMember>()
        );
        assert!(
            size_of::<Role>() <= 128,
            "Role is {} bytes",
            size_of::<Role>()
        );
    }
}
