//! [Permissions](https://docs.discord.com/developers/topics/permissions): what members and
//! roles may do in a guild or channel.

use core::fmt;

use bitflags::bitflags;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Unexpected, Visitor},
};

use crate::{
    channel::Overwrite,
    id::{GuildId, RoleId, UserId},
};

bitflags! {
    /// A set of [permission flags](https://docs.discord.com/developers/topics/permissions#permissions-bitwise-permission-flags).
    ///
    /// Permissions appear on roles, members (computed), channel permission overwrites,
    /// interactions (`app_permissions`) and OAuth2 bot invites. Discord transmits them as a
    /// **decimal string** because the value does not fit into a JavaScript number; this type
    /// (de)serializes that representation and also accepts plain integers when deserializing.
    ///
    /// Unknown bits (permissions added by Discord after this release) are preserved, so values
    /// can be round-tripped without losing information.
    ///
    /// Permissions marked **2FA** below require the guild owner to use two-factor
    /// authentication when the guild has server-wide 2FA enabled.
    ///
    /// discord.js counterpart: `PermissionFlagsBits` / `PermissionsBitField`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::permissions::Permissions;
    ///
    /// let permissions = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
    /// assert!(permissions.contains(Permissions::SEND_MESSAGES));
    /// assert!(!permissions.contains(Permissions::ADMINISTRATOR));
    ///
    /// // Discord sends permissions as decimal strings.
    /// assert_eq!(serde_json::to_string(&permissions).unwrap(), r#""3072""#);
    /// let parsed: Permissions = serde_json::from_str(r#""3072""#).unwrap();
    /// assert_eq!(parsed, permissions);
    /// ```
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Permissions: u64 {
        /// Create instant invites. Text, voice and stage channels.
        const CREATE_INSTANT_INVITE = 1 << 0;
        /// Kick members. **2FA.**
        const KICK_MEMBERS = 1 << 1;
        /// Ban members. **2FA.**
        const BAN_MEMBERS = 1 << 2;
        /// Every permission, bypassing channel permission overwrites. **2FA.**
        const ADMINISTRATOR = 1 << 3;
        /// Create, edit and delete channels. Text, voice and stage channels. **2FA.**
        const MANAGE_CHANNELS = 1 << 4;
        /// Edit the guild's settings. **2FA.**
        const MANAGE_GUILD = 1 << 5;
        /// Add new reactions to messages (reacting with an existing reaction needs no
        /// permission). Text, voice and stage channels.
        const ADD_REACTIONS = 1 << 6;
        /// View the guild's audit log.
        const VIEW_AUDIT_LOG = 1 << 7;
        /// Use priority speaker in voice channels.
        const PRIORITY_SPEAKER = 1 << 8;
        /// Go live (stream) in voice and stage channels.
        const STREAM = 1 << 9;
        /// See a channel: read messages in text channels, join voice channels. Text, voice and
        /// stage channels.
        const VIEW_CHANNEL = 1 << 10;
        /// Send messages in a channel and create forum posts (not messages in threads, see
        /// [`SEND_MESSAGES_IN_THREADS`](Self::SEND_MESSAGES_IN_THREADS)). Text, voice and stage
        /// channels.
        const SEND_MESSAGES = 1 << 11;
        /// Send text-to-speech (`/tts`) messages. Text, voice and stage channels.
        const SEND_TTS_MESSAGES = 1 << 12;
        /// Delete other users' messages. Text, voice and stage channels. **2FA.**
        const MANAGE_MESSAGES = 1 << 13;
        /// Links sent by the member are embedded automatically. Text, voice and stage channels.
        const EMBED_LINKS = 1 << 14;
        /// Upload images and files. Text, voice and stage channels.
        const ATTACH_FILES = 1 << 15;
        /// Read the message history of a channel. Text, voice and stage channels.
        const READ_MESSAGE_HISTORY = 1 << 16;
        /// Mention `@everyone` (all members of the channel) and `@here` (online members). Text,
        /// voice and stage channels.
        const MENTION_EVERYONE = 1 << 17;
        /// Use custom emojis from other guilds. Text, voice and stage channels.
        const USE_EXTERNAL_EMOJIS = 1 << 18;
        /// View guild insights.
        const VIEW_GUILD_INSIGHTS = 1 << 19;
        /// Join voice and stage channels.
        const CONNECT = 1 << 20;
        /// Speak in voice channels.
        const SPEAK = 1 << 21;
        /// Mute members in voice and stage channels.
        const MUTE_MEMBERS = 1 << 22;
        /// Deafen members in voice channels.
        const DEAFEN_MEMBERS = 1 << 23;
        /// Move members between voice and stage channels.
        const MOVE_MEMBERS = 1 << 24;
        /// Use voice activity detection ("Use Voice Activity" in the client) in voice channels.
        const USE_VAD = 1 << 25;
        /// Change one's own nickname.
        const CHANGE_NICKNAME = 1 << 26;
        /// Change other members' nicknames.
        const MANAGE_NICKNAMES = 1 << 27;
        /// Create, edit and delete roles and channel permission overwrites ("Manage
        /// Permissions" in the client). Text, voice and stage channels. **2FA.**
        const MANAGE_ROLES = 1 << 28;
        /// Create, edit and delete webhooks. Text, voice and stage channels. **2FA.**
        const MANAGE_WEBHOOKS = 1 << 29;
        /// Edit and delete emojis, stickers and soundboard sounds created by anyone. **2FA.**
        const MANAGE_GUILD_EXPRESSIONS = 1 << 30;
        /// Use application commands (slash and context-menu commands). Text, voice and stage
        /// channels.
        const USE_APPLICATION_COMMANDS = 1 << 31;
        /// Request to speak in stage channels.
        const REQUEST_TO_SPEAK = 1 << 32;
        /// Edit and delete scheduled events created by anyone. Voice and stage channels.
        const MANAGE_EVENTS = 1 << 33;
        /// Delete and archive threads and view all private threads. Text channels. **2FA.**
        const MANAGE_THREADS = 1 << 34;
        /// Create public and announcement threads. Text channels.
        const CREATE_PUBLIC_THREADS = 1 << 35;
        /// Create private threads. Text channels.
        const CREATE_PRIVATE_THREADS = 1 << 36;
        /// Use custom stickers from other guilds. Text, voice and stage channels.
        const USE_EXTERNAL_STICKERS = 1 << 37;
        /// Send messages in threads. Text channels.
        const SEND_MESSAGES_IN_THREADS = 1 << 38;
        /// Use Activities (applications with the `EMBEDDED` flag). Text and voice channels.
        const USE_EMBEDDED_ACTIVITIES = 1 << 39;
        /// Time out members ("Timeout Members" in the client), which stops them from chatting,
        /// reacting and speaking. Timed-out members temporarily lose most permissions; see
        /// [Permissions for Timed Out Members](https://docs.discord.com/developers/topics/permissions#permissions-for-timed-out-members).
        const MODERATE_MEMBERS = 1 << 40;
        /// View role subscription insights. **2FA.**
        const VIEW_CREATOR_MONETIZATION_ANALYTICS = 1 << 41;
        /// Use the soundboard in voice channels.
        const USE_SOUNDBOARD = 1 << 42;
        /// Create emojis, stickers and soundboard sounds, and edit or delete the ones created by
        /// oneself.
        const CREATE_GUILD_EXPRESSIONS = 1 << 43;
        /// Create scheduled events, and edit or delete the ones created by oneself. Voice and
        /// stage channels.
        const CREATE_EVENTS = 1 << 44;
        /// Use custom soundboard sounds from other guilds in voice channels.
        const USE_EXTERNAL_SOUNDS = 1 << 45;
        /// Send voice messages. Text, voice and stage channels.
        const SEND_VOICE_MESSAGES = 1 << 46;
        /// Set the status of voice channels.
        const SET_VOICE_CHANNEL_STATUS = 1 << 48;
        /// Send polls. Text, voice and stage channels.
        const SEND_POLLS = 1 << 49;
        /// Let user-installed apps send public responses; without it their responses are
        /// ephemeral. Only applies to apps that are not also installed in the guild. Text,
        /// voice and stage channels.
        const USE_EXTERNAL_APPS = 1 << 50;
        /// Pin and unpin messages. Text channels.
        const PIN_MESSAGES = 1 << 51;
        /// Bypass slowmode. Text, voice and stage channels.
        const BYPASS_SLOWMODE = 1 << 52;
    }
}

/// Computing a member's permissions, following [Discord's
/// algorithm](https://docs.discord.com/developers/topics/permissions#permission-overwrites)
/// (D-292). Three steps, each usable alone:
///
/// 1. [`guild_level`](Self::guild_level): the owner, `@everyone`, the member's roles and
///    `ADMINISTRATOR`;
/// 2. [`in_channel`](Self::in_channel): a channel's overwrites, then the implicit denials
///    (D-295);
/// 3. [`during_timeout`](Self::during_timeout): what a timed-out member keeps (D-294).
///
/// `djs_cache::Cache::member_permissions` and `member_permissions_in` run them on cached data.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     channel::Overwrite,
///     id::{GuildId, RoleId, UserId},
///     permissions::Permissions,
/// };
///
/// let (guild, moderator, user) = (GuildId::new(1), RoleId::new(2), UserId::new(3));
/// let everyone = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
/// let base = Permissions::guild_level(false, everyone, [Permissions::BAN_MEMBERS]);
/// assert!(base.contains(Permissions::BAN_MEMBERS));
///
/// // A read-only channel for `@everyone` (its overwrite has the guild's ID).
/// let overwrites = [Overwrite::role(guild.cast(), Permissions::empty(), Permissions::SEND_MESSAGES)];
/// let in_channel = base.in_channel(guild, user, &[moderator], &overwrites);
/// assert!(!in_channel.contains(Permissions::SEND_MESSAGES));
///
/// // Timed out: only viewing and reading the history remain.
/// assert_eq!(in_channel.during_timeout(), Permissions::VIEW_CHANNEL);
/// ```
impl Permissions {
    /// The permissions that implicitly depend on [`SEND_MESSAGES`](Self::SEND_MESSAGES): a
    /// member without it cannot use them in that channel.
    pub const NEEDS_SEND_MESSAGES: Self = Self::MENTION_EVERYONE
        .union(Self::SEND_TTS_MESSAGES)
        .union(Self::ATTACH_FILES)
        .union(Self::EMBED_LINKS);

    /// What a timed-out member keeps (D-294).
    pub const DURING_TIMEOUT: Self = Self::VIEW_CHANNEL.union(Self::READ_MESSAGE_HISTORY);

    /// A member's permissions in its guild: every permission for the guild's owner;
    /// otherwise the permissions of the `@everyone` role and of the member's `roles` together,
    /// and every permission when they include [`ADMINISTRATOR`](Self::ADMINISTRATOR).
    ///
    /// This is Discord's `compute_base_permissions`; a timeout is applied with
    /// [`during_timeout`](Self::during_timeout).
    ///
    /// discord.js counterpart: `GuildMember#permissions` (which leaves `ADMINISTRATOR` to
    /// `has`).
    #[must_use]
    pub fn guild_level(owner: bool, everyone: Self, roles: impl IntoIterator<Item = Self>) -> Self {
        if owner {
            return Self::all();
        }
        let permissions = roles.into_iter().fold(everyone, Self::union);
        if permissions.contains(Self::ADMINISTRATOR) {
            Self::all()
        } else {
            permissions
        }
    }

    /// Applies a channel's `overwrites` to guild-level permissions (from
    /// [`guild_level`](Self::guild_level)) of the member `user_id` of `guild_id`, who has the
    /// roles `role_ids` (without `@everyone`): the `@everyone` overwrite, then the role
    /// overwrites together, then the member's own; `ADMINISTRATOR` keeps every permission.
    ///
    /// Then the implicit denials (D-295): without [`VIEW_CHANNEL`](Self::VIEW_CHANNEL) the
    /// member can do nothing in the channel (the result is empty), and without
    /// [`SEND_MESSAGES`](Self::SEND_MESSAGES) the [`NEEDS_SEND_MESSAGES`](Self::NEEDS_SEND_MESSAGES)
    /// permissions are removed. Discord also ignores some permissions in a voice channel
    /// without [`CONNECT`](Self::CONNECT) (`MANAGE_CHANNELS`, for example) but gives no
    /// complete list, so that rule is not applied.
    ///
    /// A thread has no overwrites of its own: pass its parent channel's.
    ///
    /// discord.js counterpart: `GuildChannel#permissionsFor(member)` (without the implicit
    /// denials).
    #[must_use]
    pub fn in_channel(
        self,
        guild_id: GuildId,
        user_id: UserId,
        role_ids: &[RoleId],
        overwrites: &[Overwrite],
    ) -> Self {
        if self.contains(Self::ADMINISTRATOR) {
            return Self::all();
        }
        let mut everyone = None;
        let mut member = None;
        let (mut allow, mut deny) = (Self::empty(), Self::empty());
        for overwrite in overwrites {
            if let Some(role_id) = overwrite.role_id() {
                if role_id.get() == guild_id.get() {
                    everyone = Some(overwrite);
                } else if role_ids.contains(&role_id) {
                    allow |= overwrite.allow;
                    deny |= overwrite.deny;
                }
            } else if overwrite.user_id() == Some(user_id) {
                member = Some(overwrite);
            }
        }
        let mut permissions = self;
        if let Some(everyone) = everyone {
            permissions = permissions.difference(everyone.deny).union(everyone.allow);
        }
        permissions = permissions.difference(deny).union(allow);
        if let Some(member) = member {
            permissions = permissions.difference(member.deny).union(member.allow);
        }
        // An overwrite cannot grant `ADMINISTRATOR`.
        permissions.remove(Self::ADMINISTRATOR);
        if !permissions.contains(Self::VIEW_CHANNEL) {
            return Self::empty();
        }
        if !permissions.contains(Self::SEND_MESSAGES) {
            permissions.remove(Self::NEEDS_SEND_MESSAGES);
        }
        permissions
    }

    /// What a timed-out member keeps of these permissions: only
    /// [`VIEW_CHANNEL`](Self::VIEW_CHANNEL) and
    /// [`READ_MESSAGE_HISTORY`](Self::READ_MESSAGE_HISTORY) (D-294). The owner and members
    /// with [`ADMINISTRATOR`](Self::ADMINISTRATOR) are exempt (their permissions are
    /// returned unchanged).
    ///
    /// See [Permissions for Timed Out
    /// Members](https://docs.discord.com/developers/topics/permissions#permissions-for-timed-out-members).
    #[must_use]
    pub const fn during_timeout(self) -> Self {
        if self.contains(Self::ADMINISTRATOR) {
            self
        } else {
            self.intersection(Self::DURING_TIMEOUT)
        }
    }
}

impl Serialize for Permissions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `collect_str` lets serde_json write the digits directly, without an allocation.
        serializer.collect_str(&self.bits())
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PermissionsVisitor;

        impl Visitor<'_> for PermissionsVisitor {
            type Value = Permissions;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a permissions bitfield as a decimal string or integer")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<Permissions, E> {
                value
                    .parse()
                    .map(Permissions::from_bits_retain)
                    .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            }

            fn visit_u64<E: DeError>(self, value: u64) -> Result<Permissions, E> {
                Ok(Permissions::from_bits_retain(value))
            }

            fn visit_i64<E: DeError>(self, value: i64) -> Result<Permissions, E> {
                u64::try_from(value)
                    .map(Permissions::from_bits_retain)
                    .map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
            }
        }

        deserializer.deserialize_any(PermissionsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_bits_match_the_docs() {
        // Bits 0..=46 and 48..=52 (bit 47 is unassigned); computed independently.
        assert_eq!(Permissions::all().bits(), 8_866_461_766_385_663);
        assert_eq!(Permissions::ADMINISTRATOR.bits(), 0x8);
        assert_eq!(Permissions::BYPASS_SLOWMODE.bits(), 0x0010_0000_0000_0000);
    }

    #[test]
    fn serializes_as_decimal_string() {
        let check = |permissions: Permissions, json: &str| {
            assert_eq!(serde_json::to_string(&permissions).unwrap(), json);
        };
        check(Permissions::empty(), r#""0""#);
        check(
            Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            r#""3072""#,
        );
        check(
            Permissions::from_bits_retain(u64::MAX),
            r#""18446744073709551615""#,
        );
    }

    #[test]
    fn deserializes_strings_and_integers_keeping_unknown_bits() {
        let unknown = (1 << 47) | (1 << 63) | (1 << 3);
        let permissions: Permissions = serde_json::from_str(r#""9223512774343131144""#).unwrap();
        assert_eq!(permissions.bits(), unknown);
        assert!(permissions.contains(Permissions::ADMINISTRATOR));
        assert_eq!(
            serde_json::to_string(&permissions).unwrap(),
            r#""9223512774343131144""#
        );

        let permissions: Permissions = serde_json::from_str("3072").unwrap();
        assert_eq!(
            permissions,
            Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES
        );
    }

    #[test]
    fn rejects_invalid_values() {
        for json in [
            r#""""#,
            r#""-1""#,
            r#""12a""#,
            "-1",
            r#""18446744073709551616""#,
            "1.5",
        ] {
            assert!(
                serde_json::from_str::<Permissions>(json).is_err(),
                "{json} must be rejected"
            );
        }
    }
}
