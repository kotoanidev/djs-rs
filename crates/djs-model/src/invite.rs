//! [Invites](https://docs.discord.com/developers/resources/invite) to guilds, group DMs and
//! voice channels.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    application::PartialApplication,
    channel::ChannelType,
    guild::{AgeRestrictionLevel, Feature, RoleColors, VerificationLevel, WelcomeScreen},
    guild_scheduled_event::GuildScheduledEvent,
    id::{ChannelId, GuildId, RoleId},
    macros::{int_enum, int_flags},
    user::User,
    util::{ImageHash, Timestamp},
};

/// An [invite](https://docs.discord.com/developers/resources/invite#invite-object) to a guild,
/// group DM or friendship.
///
/// discord.js counterpart: `Invite`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Invite {
    /// What the invite is for.
    #[serde(rename = "type")]
    pub kind: InviteType,
    /// The invite code (`discord.gg/<code>`).
    pub code: CompactString,
    /// The guild the invite is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild: Option<Box<InviteGuild>>,
    /// The channel the invite is for.
    pub channel: Option<InviteChannel>,
    /// Who created the invite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inviter: Option<Box<User>>,
    /// What a voice channel invite opens: a stream or an embedded application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<InviteTargetType>,
    /// Whose stream a stream invite shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_user: Option<Box<User>>,
    /// The embedded application an Activity invite opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_application: Option<Box<PartialApplication>>,
    /// The approximate number of online members (`with_counts=true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_presence_count: Option<u32>,
    /// The approximate number of members (`with_counts=true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_member_count: Option<u32>,
    /// When the invite expires; `None` for invites that never expire.
    pub expires_at: Option<Timestamp>,
    /// The scheduled event the invite links to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_scheduled_event: Option<Box<GuildScheduledEvent>>,
    /// The guild invite's flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<InviteFlags>,
    /// Roles given to the user who accepts the invite.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<InviteRole>,
}

/// An invite together with its [metadata](https://docs.discord.com/developers/resources/invite#invite-metadata-object),
/// as returned by the endpoints that create or list invites of a guild or channel.
///
/// The metadata is optional (D-117): Get Guild Invites only includes it when the bot has
/// `MANAGE_GUILD`. Dereferences to the [`Invite`].
///
/// discord.js counterpart: `Invite` (with `uses`, `maxUses`, …).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "ExtendedInvite")]
pub struct InviteWithMetadata {
    /// The invite.
    #[serde(flatten)]
    pub invite: Invite,
    /// The usage statistics, when Discord sends them.
    #[serde(flatten)]
    pub metadata: Option<InviteMetadata>,
}

/// The [metadata of an invite](https://docs.discord.com/developers/resources/invite#invite-metadata-object):
/// how often it was used and for how long it is valid.
///
/// discord.js counterpart: `Invite#uses`, `#maxUses`, `#maxAge`, `#temporary`,
/// `#createdTimestamp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteMetadata {
    /// How many times the invite was used.
    pub uses: u32,
    /// How many times the invite may be used; `0` means unlimited.
    pub max_uses: u32,
    /// How long the invite is valid, in seconds; `0` means forever.
    pub max_age: u32,
    /// Whether the invite only grants temporary membership.
    pub temporary: bool,
    /// When the invite was created.
    pub created_at: Timestamp,
}

impl core::ops::Deref for InviteWithMetadata {
    type Target = Invite;

    fn deref(&self) -> &Invite {
        &self.invite
    }
}

int_enum! {
    /// [What an invite is for](https://docs.discord.com/developers/resources/invite#invite-object-invite-types).
    ///
    /// discord.js counterpart: `InviteType`.
    pub enum InviteType: u8 {
        /// A guild.
        Guild = 0,
        /// A group DM.
        GroupDm = 1,
        /// A friend request.
        Friend = 2,
    }
}

int_enum! {
    /// [What a voice channel invite opens](https://docs.discord.com/developers/resources/invite#invite-object-invite-target-types).
    ///
    /// discord.js counterpart: `InviteTargetType`.
    pub enum InviteTargetType: u8 {
        /// A user's stream.
        Stream = 1,
        /// An embedded application (Activity).
        EmbeddedApplication = 2,
    }
}

/// The partial guild of an [`Invite`] (D-051).
///
/// discord.js counterpart: `InviteGuild`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's name.
    pub name: CompactString,
    /// The [invite splash hash](https://docs.discord.com/developers/reference#image-formatting).
    pub splash: Option<ImageHash>,
    /// The [banner hash](https://docs.discord.com/developers/reference#image-formatting).
    pub banner: Option<ImageHash>,
    /// The guild's description.
    pub description: Option<CompactString>,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The guild's features.
    pub features: Vec<Feature>,
    /// The guild's verification level.
    pub verification_level: VerificationLevel,
    /// The vanity invite code.
    pub vanity_url_code: Option<CompactString>,
    /// The guild's age-restriction level.
    #[serde(default)]
    pub nsfw_level: AgeRestrictionLevel,
    /// The number of boosts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_subscription_count: Option<u32>,
    /// The welcome screen of a Community guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome_screen: Option<WelcomeScreen>,
}

/// The partial channel of an [`Invite`] (D-051).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The kind of channel.
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// The channel's name.
    pub name: Option<CompactString>,
    /// The [group DM icon hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ImageHash>,
    /// The usernames of a group DM's members.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipients: Vec<InviteRecipient>,
}

/// A member of the group DM an [`InviteChannel`] belongs to; only the username is known.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteRecipient {
    /// The user's username.
    pub username: CompactString,
}

/// A role given to users who accept an [`Invite`] (D-051).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteRole {
    /// The role's ID.
    pub id: RoleId,
    /// The role's name.
    pub name: CompactString,
    /// The role's position.
    pub position: i32,
    /// The role's primary color. **Deprecated** in favor of [`colors`](Self::colors).
    pub color: u32,
    /// The role's colors.
    pub colors: RoleColors,
    /// The [role icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The role's Unicode emoji.
    pub unicode_emoji: Option<CompactString>,
}

int_flags! {
    /// [Flags of a guild invite](https://docs.discord.com/developers/resources/invite#invite-object-guild-invite-flags).
    ///
    /// discord.js counterpart: `InviteFlags` / `InviteFlagsBitField`.
    pub struct InviteFlags: u64 {
        /// A guest invite: it only grants access to one voice channel.
        const IS_GUEST_INVITE = 1 << 0;
    }
}

/// The [state of the job](https://docs.discord.com/developers/resources/invite#get-target-users-job-status)
/// that processes an invite's target users file, which Discord does asynchronously.
///
/// discord.js counterpart: `TargetUsersJobStatusForInvite`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteTargetUsersJobStatus {
    /// Where the job is.
    pub status: InviteTargetUsersJobStatusCode,
    /// How many users the file lists.
    pub total_users: u32,
    /// How many of them were processed so far.
    pub processed_users: u32,
    /// When the job was created.
    pub created_at: Timestamp,
    /// When the job finished.
    #[serde(default)]
    pub completed_at: Option<Timestamp>,
    /// Why the job failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<CompactString>,
}

int_enum! {
    /// The [status](https://docs.discord.com/developers/resources/invite#get-target-users-job-status-status-codes)
    /// of an [`InviteTargetUsersJobStatus`].
    ///
    /// discord.js counterpart: `InviteTargetUsersJobStatus`.
    pub enum InviteTargetUsersJobStatusCode: u8 {
        /// No status given.
        Unspecified = 0,
        /// The file is still being processed.
        Processing = 1,
        /// The file was processed.
        Completed = 2,
        /// Processing failed; see [`InviteTargetUsersJobStatus::error_message`].
        Failed = 3,
    }
}
