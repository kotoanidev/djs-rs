//! Bodies, queries and responses of the [channel endpoints](https://docs.discord.com/developers/resources/channel):
//! channels, permission overwrites, invites, group DMs and threads.

use core::fmt;

use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{Attachments, FileUpload};
use crate::{
    channel::{
        AutoArchiveDuration, Channel, ChannelFlags, ChannelType, DefaultReaction, ForumLayoutType,
        ForumTag, Overwrite, OverwriteType, SortOrderType, ThreadMember, VideoQualityMode,
    },
    component::Component,
    id::{ApplicationId, ChannelId, EmojiId, RoleId, StickerId, TagId, UserId},
    invite::InviteTargetType,
    message::{AllowedMentions, AttachmentRequest, Embed, Message, MessageFlags},
    permissions::Permissions,
    util::{ImageData, Patch, Timestamp},
};

/// The body of [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel)
/// for a guild channel (D-109).
///
/// Every field is optional; [`Patch`] fields can also be cleared. Which fields apply depends
/// on the channel's type (e.g. `bitrate` only for voice and stage channels).
///
/// discord.js counterpart: `GuildChannelEditOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::ModifyGuildChannel, util::Patch};
///
/// let mut body = ModifyGuildChannel::default();
/// body.name = Some("general".into());
/// body.topic = Patch::Clear;
/// assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"name":"general","topic":null}"#);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildChannel {
    /// The name (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The new type; only text ↔ announcement, in guilds with the `NEWS` feature.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,
    /// The position in the channel list (channels with the same position are sorted by ID).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub position: Patch<i32>,
    /// The topic (0-1024 characters, 0-4096 for forum and media channels).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub topic: Patch<CompactString>,
    /// Whether the channel is age-restricted.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub nsfw: Patch<bool>,
    /// Slowmode: seconds a member must wait between messages (0-21600).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub rate_limit_per_user: Patch<u16>,
    /// The bitrate of a voice or stage channel, in bits per second (min 8000; the maximum
    /// depends on the guild's boost level).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub bitrate: Patch<u32>,
    /// The user limit of a voice (max 99) or stage (max 10 000) channel; 0 is no limit.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub user_limit: Patch<u32>,
    /// The permission overwrites (all of them; needs `MANAGE_ROLES`).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub permission_overwrites: Patch<Vec<Overwrite>>,
    /// The parent category.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub parent_id: Patch<ChannelId>,
    /// The voice region ID; [`Patch::Clear`] selects it automatically.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub rtc_region: Patch<CompactString>,
    /// The camera video quality of a voice or stage channel.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub video_quality_mode: Patch<VideoQualityMode>,
    /// The archive duration that clients preselect for new threads.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub default_auto_archive_duration: Patch<AutoArchiveDuration>,
    /// The channel's flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
    /// The tags of a forum or media channel (max 20); tags without an ID are created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_tags: Option<Vec<ForumTagData>>,
    /// The emoji of the add-reaction button on posts of a forum or media channel.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub default_reaction_emoji: Patch<DefaultReaction>,
    /// The slowmode copied to new threads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u16>,
    /// The default order of posts in a forum or media channel.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub default_sort_order: Patch<SortOrderType>,
    /// The default layout of a forum channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<ForumLayoutType>,
}

/// The body of [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel)
/// for a thread (D-109).
///
/// Needs `MANAGE_THREADS`, except for unarchiving an unlocked thread (`SEND_MESSAGES`).
///
/// discord.js counterpart: `ThreadEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyThread {
    /// The name (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// Whether the thread is archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    /// The inactivity after which the thread is archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<AutoArchiveDuration>,
    /// Whether the thread is locked (only `MANAGE_THREADS` can unarchive it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    /// Whether non-moderators can add other non-moderators (private threads only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    /// Slowmode: seconds a member must wait between messages (0-21600).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub rate_limit_per_user: Patch<u16>,
    /// The thread's flags (`PINNED` only for posts of forum and media channels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
    /// The tags of a forum or media post (max 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<Vec<TagId>>,
}

/// The body of [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel)
/// for a group DM (D-109).
///
/// discord.js counterpart: `PartialGroupDMChannel#edit` data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGroupDm {
    /// The name (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The icon.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub icon: Patch<ImageData>,
}

/// A [forum tag](https://docs.discord.com/developers/resources/channel#forum-tag-object) in a
/// request (D-114): tags without an [`id`](Self::id) are created.
///
/// discord.js counterpart: `GuildForumTagData`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::ForumTagData;
///
/// let mut tag = ForumTagData::new("solved");
/// tag.emoji_name = Some("✅".into());
/// assert_eq!(
///     serde_json::to_string(&tag).unwrap(),
///     r#"{"name":"solved","moderated":false,"emoji_name":"✅"}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ForumTagData {
    /// The ID of an existing tag; `None` for a new tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<TagId>,
    /// The name (0-20 characters).
    pub name: CompactString,
    /// Whether only members with `MANAGE_THREADS` can add or remove the tag.
    pub moderated: bool,
    /// A custom emoji of the guild.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<EmojiId>,
    /// A Unicode emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<CompactString>,
}

impl ForumTagData {
    /// A new tag named `name`, without an emoji.
    #[must_use]
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            id: None,
            name: name.into(),
            moderated: false,
            emoji_id: None,
            emoji_name: None,
        }
    }
}

/// An existing tag, sent back unchanged.
impl From<ForumTag> for ForumTagData {
    fn from(tag: ForumTag) -> Self {
        let ForumTag {
            id,
            name,
            moderated,
            emoji_id,
            emoji_name,
        } = tag;
        Self {
            id: Some(id),
            name,
            moderated,
            emoji_id,
            emoji_name,
        }
    }
}

impl From<&ForumTag> for ForumTagData {
    fn from(tag: &ForumTag) -> Self {
        tag.clone().into()
    }
}

/// The query of [Get Channel Member Permissions](https://docs.discord.com/developers/resources/channel#get-channel-member-permissions)
/// and [Get Channel Role Permissions](https://docs.discord.com/developers/resources/channel#get-channel-role-permissions).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetChannelPermissions {
    /// Whether to apply [implicit permissions](https://docs.discord.com/developers/topics/permissions#implicit-permissions)
    /// (no permissions without `VIEW_CHANNEL`, or `CONNECT` in voice and stage channels;
    /// Discord's default: yes). Never applied to bots.
    pub implicit: Option<bool>,
}

/// The body of [Set Voice Channel Status](https://docs.discord.com/developers/resources/channel#set-voice-channel-status).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct SetVoiceChannelStatus {
    /// The status (up to 500 characters); `None` removes it.
    pub status: Option<CompactString>,
}

impl SetVoiceChannelStatus {
    /// Sets the status to `status`, or removes it.
    #[must_use]
    pub fn new(status: Option<impl Into<CompactString>>) -> Self {
        Self {
            status: status.map(Into::into),
        }
    }
}

/// The body of [Edit Channel Permissions](https://docs.discord.com/developers/resources/channel#edit-channel-permissions):
/// the permission overwrite of one role or member.
///
/// discord.js counterpart: `PermissionOverwriteOptions` (as bitfields).
///
/// # Examples
///
/// ```
/// use djs_model::{channel::OverwriteType, permissions::Permissions, rest::EditChannelPermissions};
///
/// let mut muted = EditChannelPermissions::new(OverwriteType::Role);
/// muted.deny = Some(Permissions::SEND_MESSAGES);
/// assert_eq!(serde_json::to_string(&muted).unwrap(), r#"{"deny":"2048","type":0}"#);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct EditChannelPermissions {
    /// The permissions to allow (Discord's default: none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Permissions>,
    /// The permissions to deny (Discord's default: none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Permissions>,
    /// Whether the overwrite is for a role or a member.
    #[serde(rename = "type")]
    pub kind: OverwriteType,
}

impl EditChannelPermissions {
    /// An overwrite for a role or member that allows and denies nothing yet.
    #[must_use]
    pub const fn new(kind: OverwriteType) -> Self {
        Self {
            allow: None,
            deny: None,
            kind,
        }
    }
}

/// The body of [Create Channel Invite](https://docs.discord.com/developers/resources/channel#create-channel-invite).
///
/// Every field is optional. The users who can accept the invite are given either as
/// [`target_user_ids`](Self::target_user_ids) or, for more than 1000 users, as a CSV
/// [`target_users_file`](Self::target_users_file) with one user ID per line (D-111).
///
/// discord.js counterpart: `InviteCreateOptions`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CreateChannelInvite {
    /// Seconds until the invite expires (0-604800, 0 = never; Discord's default: 86400).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u32>,
    /// How many times the invite can be used (0-100, 0 = unlimited, the default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u8>,
    /// Whether the invite only grants temporary membership.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub temporary: bool,
    /// Whether to always create a new invite instead of reusing a similar one.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub unique: bool,
    /// What a voice channel invite opens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<InviteTargetType>,
    /// The streaming user whose stream the invite shows (for
    /// [`InviteTargetType::Stream`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_user_id: Option<UserId>,
    /// The embedded application the invite opens (for
    /// [`InviteTargetType::EmbeddedApplication`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_application_id: Option<ApplicationId>,
    /// The only users who can see and accept the invite (max 1000).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub target_user_ids: Vec<UserId>,
    /// The roles given to users who accept the invite (needs `MANAGE_ROLES`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<RoleId>,
    /// A CSV file of the only users who can accept the invite, instead of
    /// [`target_user_ids`](Self::target_user_ids); sent as `multipart/form-data`.
    #[serde(skip)]
    pub target_users_file: Option<FileUpload>,
}

/// The body of [Follow Announcement Channel](https://docs.discord.com/developers/resources/channel#follow-announcement-channel).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct FollowAnnouncementChannel {
    /// The channel that receives the announcements (needs `MANAGE_WEBHOOKS` there).
    pub webhook_channel_id: ChannelId,
}

impl FollowAnnouncementChannel {
    /// Follows into `webhook_channel_id`.
    #[must_use]
    pub const fn new(webhook_channel_id: ChannelId) -> Self {
        Self { webhook_channel_id }
    }
}

/// The body of [Group DM Add Recipient](https://docs.discord.com/developers/resources/channel#group-dm-add-recipient).
///
/// The access token is redacted in the `Debug` output.
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct GroupDmAddRecipient {
    /// The `OAuth2` access token of the user, who granted the `gdm.join` scope.
    pub access_token: String,
    /// The user's nickname in the group DM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<CompactString>,
}

impl GroupDmAddRecipient {
    /// Adds the user whose access token is `access_token`.
    #[must_use]
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            nick: None,
        }
    }
}

impl fmt::Debug for GroupDmAddRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GroupDmAddRecipient")
            .field("access_token", &"<redacted>")
            .field("nick", &self.nick)
            .finish()
    }
}

/// The body of [Start Thread from Message](https://docs.discord.com/developers/resources/channel#start-thread-from-message).
///
/// discord.js counterpart: `StartThreadOptions`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct StartThreadFromMessage {
    /// The name (1-100 characters).
    pub name: CompactString,
    /// The inactivity after which the thread is archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<AutoArchiveDuration>,
    /// Slowmode: seconds a member must wait between messages (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
}

impl StartThreadFromMessage {
    /// A thread named `name`.
    #[must_use]
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            name: name.into(),
            auto_archive_duration: None,
            rate_limit_per_user: None,
        }
    }
}

/// The body of [Start Thread without Message](https://docs.discord.com/developers/resources/channel#start-thread-without-message).
///
/// The thread's type is always sent (Discord's default, a private thread, is going away).
///
/// discord.js counterpart: `GuildTextThreadCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{channel::ChannelType, rest::StartThreadWithoutMessage};
///
/// let body = StartThreadWithoutMessage::new("Support", ChannelType::PrivateThread);
/// assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"name":"Support","type":12}"#);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct StartThreadWithoutMessage {
    /// The name (1-100 characters).
    pub name: CompactString,
    /// The inactivity after which the thread is archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<AutoArchiveDuration>,
    /// The type of thread: public, private or (in announcement channels) announcement.
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// Whether non-moderators can add other non-moderators (private threads only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    /// Slowmode: seconds a member must wait between messages (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
}

impl StartThreadWithoutMessage {
    /// A thread of type `kind` named `name`.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, kind: ChannelType) -> Self {
        Self {
            name: name.into(),
            auto_archive_duration: None,
            kind,
            invitable: None,
            rate_limit_per_user: None,
        }
    }
}

/// The body of [Start Thread in Forum or Media Channel](https://docs.discord.com/developers/resources/channel#start-thread-in-forum-or-media-channel):
/// a post with its first message.
///
/// The files of the [`message`](Self::message) are uploaded with it.
///
/// discord.js counterpart: `GuildForumThreadCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::{ForumThreadMessage, StartThreadInForumOrMediaChannel};
///
/// let body = StartThreadInForumOrMediaChannel::new("Bug: crash", ForumThreadMessage::from("It crashes"));
/// assert_eq!(
///     serde_json::to_string(&body).unwrap(),
///     r#"{"name":"Bug: crash","message":{"content":"It crashes"}}"#
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct StartThreadInForumOrMediaChannel {
    /// The name (1-100 characters).
    pub name: CompactString,
    /// The inactivity after which the thread is archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<AutoArchiveDuration>,
    /// Slowmode: seconds a member must wait between messages (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
    /// The first message.
    pub message: ForumThreadMessage,
    /// The tags of the post (max 5).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied_tags: Vec<TagId>,
}

impl StartThreadInForumOrMediaChannel {
    /// A post named `name` starting with `message`.
    #[must_use]
    pub fn new(name: impl Into<CompactString>, message: ForumThreadMessage) -> Self {
        Self {
            name: name.into(),
            auto_archive_duration: None,
            rate_limit_per_user: None,
            message,
            applied_tags: Vec::new(),
        }
    }
}

/// The [first message](https://docs.discord.com/developers/resources/channel#start-thread-in-forum-or-media-channel-forum-and-media-thread-message-params-object)
/// of a forum or media post (D-112).
///
/// At least one of `content`, `embeds`, `sticker_ids`, `components` or `files` is required.
/// Like [`CreateMessage`](super::CreateMessage), every file gets an `attachments` entry
/// automatically (D-094).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ForumThreadMessage {
    /// The text (up to 2000 characters).
    pub content: Option<CompactString>,
    /// Up to 10 rich embeds (up to 6000 characters in total).
    pub embeds: Vec<Embed>,
    /// Which mentions in the content notify.
    pub allowed_mentions: Option<AllowedMentions>,
    /// The components.
    pub components: Vec<Component>,
    /// Up to 3 stickers of the guild to send.
    pub sticker_ids: Vec<StickerId>,
    /// Explicit attachment entries. Usually left out: every file gets one automatically,
    /// unless an entry with its `files[n]` index is listed here.
    pub attachments: Option<Vec<AttachmentRequest>>,
    /// Only [`SUPPRESS_EMBEDS`](MessageFlags::SUPPRESS_EMBEDS),
    /// [`SUPPRESS_NOTIFICATIONS`](MessageFlags::SUPPRESS_NOTIFICATIONS) and
    /// [`IS_COMPONENTS_V2`](MessageFlags::IS_COMPONENTS_V2) can be set.
    pub flags: Option<MessageFlags>,
    /// Files to upload with the message.
    pub files: Vec<FileUpload>,
}

impl From<&str> for ForumThreadMessage {
    fn from(content: &str) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl From<String> for ForumThreadMessage {
    fn from(content: String) -> Self {
        Self {
            content: Some(content.into()),
            ..Self::default()
        }
    }
}

impl Serialize for ForumThreadMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<&'a CompactString>,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            embeds: &'a [Embed],
            #[serde(skip_serializing_if = "Option::is_none")]
            allowed_mentions: Option<&'a AllowedMentions>,
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            components: &'a [Component],
            #[serde(skip_serializing_if = "<[_]>::is_empty")]
            sticker_ids: &'a [StickerId],
            #[serde(skip_serializing_if = "Attachments::is_absent")]
            attachments: Attachments<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            flags: Option<MessageFlags>,
        }

        // Destructured without `..`, so a new field cannot be forgotten here.
        let Self {
            content,
            embeds,
            allowed_mentions,
            components,
            sticker_ids,
            attachments,
            flags,
            files,
        } = self;
        Wire {
            content: content.as_ref(),
            embeds,
            allowed_mentions: allowed_mentions.as_ref(),
            components,
            sticker_ids,
            attachments: Attachments::new(attachments.as_deref(), files),
            flags: *flags,
        }
        .serialize(serializer)
    }
}

/// The response of [Start Thread in Forum or Media Channel](https://docs.discord.com/developers/resources/channel#start-thread-in-forum-or-media-channel):
/// the created post and its first message (D-115).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ForumThread {
    /// The thread.
    pub channel: Channel,
    /// The thread's first message.
    pub message: Option<Message>,
}

impl<'de> Deserialize<'de> for ForumThread {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // The message is nested inside the channel object.
        #[derive(Deserialize)]
        struct Wire {
            #[serde(flatten)]
            channel: Channel,
            #[serde(default)]
            message: Option<Message>,
        }

        let Wire { channel, message } = Wire::deserialize(deserializer)?;
        Ok(Self { channel, message })
    }
}

/// The query of [Get Thread Member](https://docs.discord.com/developers/resources/channel#get-thread-member).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetThreadMember {
    /// Whether to include the guild [`member`](ThreadMember::member).
    pub with_member: Option<bool>,
}

/// The query of [List Thread Members](https://docs.discord.com/developers/resources/channel#list-thread-members).
///
/// Needs the `GUILD_MEMBERS` privileged intent. With `with_member`, the results are
/// paginated (100 per page by default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListThreadMembers {
    /// Whether to include each guild [`member`](ThreadMember::member) (and paginate).
    pub with_member: Option<bool>,
    /// Members after this user ID.
    pub after: Option<UserId>,
    /// The page size (1-100, default 100).
    pub limit: Option<u8>,
}

/// The query of [List Public Archived Threads](https://docs.discord.com/developers/resources/channel#list-public-archived-threads)
/// and [List Private Archived Threads](https://docs.discord.com/developers/resources/channel#list-private-archived-threads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListArchivedThreads {
    /// Threads archived before this time.
    pub before: Option<Timestamp>,
    /// The maximum number of threads.
    pub limit: Option<u16>,
}

/// The query of [List Joined Private Archived Threads](https://docs.discord.com/developers/resources/channel#list-joined-private-archived-threads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListJoinedPrivateArchivedThreads {
    /// Threads with an ID below this one.
    pub before: Option<ChannelId>,
    /// The maximum number of threads.
    pub limit: Option<u16>,
}

/// The response of the [archived thread lists](https://docs.discord.com/developers/resources/channel#list-public-archived-threads).
///
/// discord.js counterpart: `FetchedThreadsMore`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArchivedThreads {
    /// The threads, newest archived (or highest ID) first.
    pub threads: Vec<Channel>,
    /// The current user's membership of each returned thread they joined.
    pub members: Vec<ThreadMember>,
    /// Whether more threads may follow.
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::GenericId;

    #[test]
    fn modify_bodies() {
        let mut channel = ModifyGuildChannel {
            parent_id: Patch::Clear,
            user_limit: Patch::Set(10),
            ..ModifyGuildChannel::default()
        };
        channel.permission_overwrites = Patch::Set(vec![Overwrite {
            id: GenericId::new(5),
            kind: OverwriteType::Member,
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::empty(),
        }]);
        assert_eq!(
            serde_json::to_string(&channel).unwrap(),
            concat!(
                r#"{"user_limit":10,"permission_overwrites":[{"id":"5","type":1,"allow":"1024","#,
                r#""deny":"0"}],"parent_id":null}"#
            )
        );

        let thread = ModifyThread {
            archived: Some(false),
            rate_limit_per_user: Patch::Clear,
            applied_tags: Some(vec![TagId::new(3)]),
            ..ModifyThread::default()
        };
        assert_eq!(
            serde_json::to_string(&thread).unwrap(),
            r#"{"archived":false,"rate_limit_per_user":null,"applied_tags":["3"]}"#
        );

        let group = ModifyGroupDm {
            icon: Patch::Set(ImageData::with_content_type(&b"GIF89a"[..], "image/gif")),
            ..ModifyGroupDm::default()
        };
        assert_eq!(
            serde_json::to_string(&group).unwrap(),
            r#"{"icon":"data:image/gif;base64,R0lGODlh"}"#
        );
    }

    #[test]
    fn invite_and_recipient_bodies() {
        let invite = CreateChannelInvite {
            max_age: Some(0),
            unique: true,
            target_users_file: Some(FileUpload::new("users.csv", "1\n")),
            ..CreateChannelInvite::default()
        };
        assert_eq!(
            serde_json::to_string(&invite).unwrap(),
            r#"{"max_age":0,"unique":true}"#
        );
        assert_eq!(
            serde_json::to_string(&CreateChannelInvite::default()).unwrap(),
            "{}"
        );

        let recipient = GroupDmAddRecipient::new("super-secret");
        assert!(!format!("{recipient:?}").contains("super-secret"));
        assert_eq!(
            serde_json::to_string(&recipient).unwrap(),
            r#"{"access_token":"super-secret"}"#
        );
    }

    #[test]
    fn forum_thread_with_files() {
        let mut message = ForumThreadMessage::from("see log");
        message.files.push(FileUpload::new("log.txt", "ok"));
        let mut body = StartThreadInForumOrMediaChannel::new("Report", message);
        body.applied_tags.push(TagId::new(9));
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            concat!(
                r#"{"name":"Report","message":{"content":"see log","attachments":"#,
                r#"[{"id":0,"filename":"log.txt"}]},"applied_tags":["9"]}"#
            )
        );
    }

    #[test]
    fn forum_tags_round_trip() {
        let tag: ForumTag = serde_json::from_str(
            r#"{"id":"7","name":"bug","moderated":true,"emoji_id":null,"emoji_name":"🐛"}"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&ForumTagData::from(&tag)).unwrap(),
            r#"{"id":"7","name":"bug","moderated":true,"emoji_name":"🐛"}"#
        );
    }
}
