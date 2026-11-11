//! [Channels](https://docs.discord.com/developers/resources/channel): guild text, voice, forum
//! and media channels, threads and direct messages.
//!
//! A [`Channel`] is an enum with one variant per [`ChannelType`], each holding a struct with
//! the fields that kind of channel has (D-057).

use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

use crate::{
    guild::Member,
    id::{
        ApplicationId, ChannelId, EmojiId, GenericId, GuildId, MessageId, RoleId, TagId, UserId,
        WebhookId,
    },
    macros::{int_enum, int_flags},
    permissions::Permissions,
    user::User,
    util::{ImageHash, Timestamp},
};

/// A [channel](https://docs.discord.com/developers/resources/channel#channel-object) of any
/// kind (D-057).
///
/// Match on the variant to get the fields of a specific kind of channel, or use the methods for
/// the fields most kinds share.
///
/// discord.js counterpart: `Channel` and its subclasses (`TextChannel`, `VoiceChannel`,
/// `ThreadChannel`, …).
///
/// # Examples
///
/// ```
/// use djs_model::channel::{Channel, ChannelType};
///
/// let channel: Channel = serde_json::from_str(
///     r#"{"id":"41771983423143937","guild_id":"41771983423143937","name":"general",
///         "type":0,"position":6,"permission_overwrites":[],"rate_limit_per_user":2,
///         "nsfw":true,"topic":"24/7 chat about how to gank Mike #2",
///         "last_message_id":"155117677105512449","parent_id":"399942396007890945"}"#,
/// )?;
/// assert_eq!(channel.kind(), ChannelType::GuildText);
/// assert_eq!(channel.name(), Some("general"));
/// if let Channel::Text(text) = &channel {
///     assert_eq!(text.rate_limit_per_user, 2);
/// }
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Channel {
    /// A text channel in a guild (`GUILD_TEXT`).
    Text(TextChannel),
    /// A direct message between two users (`DM`).
    Dm(DmChannel),
    /// A voice channel in a guild (`GUILD_VOICE`).
    Voice(VoiceChannel),
    /// A direct message between several users (`GROUP_DM`).
    GroupDm(GroupDmChannel),
    /// A category that contains up to 50 channels (`GUILD_CATEGORY`).
    Category(CategoryChannel),
    /// An announcement channel whose messages can be followed and crossposted
    /// (`GUILD_ANNOUNCEMENT`).
    Announcement(TextChannel),
    /// A thread in an announcement channel (`ANNOUNCEMENT_THREAD`).
    AnnouncementThread(ThreadChannel),
    /// A public thread in a text, forum or media channel (`PUBLIC_THREAD`).
    PublicThread(ThreadChannel),
    /// A private thread in a text channel (`PRIVATE_THREAD`).
    PrivateThread(ThreadChannel),
    /// A stage channel for events with an audience (`GUILD_STAGE_VOICE`).
    Stage(VoiceChannel),
    /// The channel of a Student Hub listing its servers (`GUILD_DIRECTORY`).
    Directory(DirectoryChannel),
    /// A channel that only contains threads (posts) (`GUILD_FORUM`).
    Forum(ForumChannel),
    /// A forum-like channel for media posts (`GUILD_MEDIA`).
    Media(ForumChannel),
    /// A kind of channel this version of the library does not know about (C-010).
    Unknown(UnknownChannel),
}

impl Channel {
    /// The channel's ID.
    #[must_use]
    pub const fn id(&self) -> ChannelId {
        match self {
            Self::Text(channel) | Self::Announcement(channel) => channel.id,
            Self::Dm(channel) => channel.id,
            Self::Voice(channel) | Self::Stage(channel) => channel.id,
            Self::GroupDm(channel) => channel.id,
            Self::Category(channel) => channel.id,
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => channel.id,
            Self::Directory(channel) => channel.id,
            Self::Forum(channel) | Self::Media(channel) => channel.id,
            Self::Unknown(channel) => channel.id,
        }
    }

    /// The kind of channel.
    #[must_use]
    pub const fn kind(&self) -> ChannelType {
        match self {
            Self::Text(_) => ChannelType::GuildText,
            Self::Dm(_) => ChannelType::Dm,
            Self::Voice(_) => ChannelType::GuildVoice,
            Self::GroupDm(_) => ChannelType::GroupDm,
            Self::Category(_) => ChannelType::GuildCategory,
            Self::Announcement(_) => ChannelType::GuildAnnouncement,
            Self::AnnouncementThread(_) => ChannelType::AnnouncementThread,
            Self::PublicThread(_) => ChannelType::PublicThread,
            Self::PrivateThread(_) => ChannelType::PrivateThread,
            Self::Stage(_) => ChannelType::GuildStageVoice,
            Self::Directory(_) => ChannelType::GuildDirectory,
            Self::Forum(_) => ChannelType::GuildForum,
            Self::Media(_) => ChannelType::GuildMedia,
            Self::Unknown(channel) => channel.kind,
        }
    }

    /// The guild the channel belongs to. `None` for DMs, and for guild channels inside
    /// `GUILD_CREATE` (where Discord omits it).
    #[must_use]
    pub const fn guild_id(&self) -> Option<GuildId> {
        match self {
            Self::Text(channel) | Self::Announcement(channel) => channel.guild_id,
            Self::Voice(channel) | Self::Stage(channel) => channel.guild_id,
            Self::Category(channel) => channel.guild_id,
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => channel.guild_id,
            Self::Directory(channel) => channel.guild_id,
            Self::Forum(channel) | Self::Media(channel) => channel.guild_id,
            Self::Unknown(channel) => channel.guild_id,
            Self::Dm(_) | Self::GroupDm(_) => None,
        }
    }

    /// The channel's name. `None` for DMs and unnamed group DMs.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Text(channel) | Self::Announcement(channel) => Some(&channel.name),
            Self::Voice(channel) | Self::Stage(channel) => Some(&channel.name),
            Self::Category(channel) => Some(&channel.name),
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => Some(&channel.name),
            Self::Directory(channel) => Some(&channel.name),
            Self::Forum(channel) | Self::Media(channel) => Some(&channel.name),
            Self::GroupDm(channel) => channel.name.as_deref(),
            Self::Unknown(channel) => channel.name.as_deref(),
            Self::Dm(_) => None,
        }
    }

    /// The parent: the category of a guild channel, or the channel a thread was created in.
    #[must_use]
    pub const fn parent_id(&self) -> Option<ChannelId> {
        match self {
            Self::Text(channel) | Self::Announcement(channel) => channel.parent_id,
            Self::Voice(channel) | Self::Stage(channel) => channel.parent_id,
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => channel.parent_id,
            Self::Directory(channel) => channel.parent_id,
            Self::Forum(channel) | Self::Media(channel) => channel.parent_id,
            Self::Unknown(channel) => channel.parent_id,
            Self::Dm(_) | Self::GroupDm(_) | Self::Category(_) => None,
        }
    }

    /// The channel's [flags](ChannelFlags).
    #[must_use]
    pub const fn flags(&self) -> ChannelFlags {
        match self {
            Self::Text(channel) | Self::Announcement(channel) => channel.flags,
            Self::Dm(channel) => channel.flags,
            Self::Voice(channel) | Self::Stage(channel) => channel.flags,
            Self::GroupDm(channel) => channel.flags,
            Self::Category(channel) => channel.flags,
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => channel.flags,
            Self::Directory(channel) => channel.flags,
            Self::Forum(channel) | Self::Media(channel) => channel.flags,
            Self::Unknown(channel) => channel.flags,
        }
    }

    /// The permission overwrites of a guild channel; empty for DMs and threads.
    #[must_use]
    pub fn permission_overwrites(&self) -> &[Overwrite] {
        match self {
            Self::Text(channel) | Self::Announcement(channel) => &channel.permission_overwrites,
            Self::Voice(channel) | Self::Stage(channel) => &channel.permission_overwrites,
            Self::Category(channel) => &channel.permission_overwrites,
            Self::Directory(channel) => &channel.permission_overwrites,
            Self::Forum(channel) | Self::Media(channel) => &channel.permission_overwrites,
            Self::Dm(_)
            | Self::GroupDm(_)
            | Self::AnnouncementThread(_)
            | Self::PublicThread(_)
            | Self::PrivateThread(_)
            | Self::Unknown(_) => &[],
        }
    }

    /// Returns the thread data if this channel is a thread.
    #[must_use]
    pub const fn as_thread(&self) -> Option<&ThreadChannel> {
        match self {
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => Some(channel),
            _ => None,
        }
    }
}

int_enum! {
    /// The [type of a channel](https://docs.discord.com/developers/resources/channel#channel-object-channel-types).
    ///
    /// discord.js counterpart: `ChannelType`.
    pub enum ChannelType: u8 {
        /// A text channel in a guild.
        GuildText = 0,
        /// A direct message between two users.
        Dm = 1,
        /// A voice channel in a guild.
        GuildVoice = 2,
        /// A direct message between several users.
        GroupDm = 3,
        /// A category that organizes up to 50 channels.
        GuildCategory = 4,
        /// A channel that users can follow and crosspost into their own guild.
        GuildAnnouncement = 5,
        /// A thread in an announcement channel.
        AnnouncementThread = 10,
        /// A public thread in a text, forum or media channel.
        PublicThread = 11,
        /// A private thread in a text channel, visible to invited members and moderators.
        PrivateThread = 12,
        /// A voice channel for hosting events with an audience.
        GuildStageVoice = 13,
        /// The channel of a Student Hub listing its servers.
        GuildDirectory = 14,
        /// A channel that can only contain threads.
        GuildForum = 15,
        /// A channel that can only contain threads, for media posts.
        GuildMedia = 16,
    }
}

impl ChannelType {
    /// Whether channels of this type are threads.
    #[must_use]
    pub const fn is_thread(self) -> bool {
        matches!(
            self,
            Self::AnnouncementThread | Self::PublicThread | Self::PrivateThread
        )
    }
}

/// A text or announcement channel in a guild.
///
/// discord.js counterpart: `TextChannel` / `NewsChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TextChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The guild; `None` inside `GUILD_CREATE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel's name (1-100 characters).
    pub name: CompactString,
    /// The sorting position; channels with equal positions are ordered by ID.
    pub position: i32,
    /// Explicit permissions for roles and members.
    pub permission_overwrites: Vec<Overwrite>,
    /// The parent category.
    pub parent_id: Option<ChannelId>,
    /// Whether the channel is [age-restricted](https://support.discord.com/hc/en-us/articles/115000084051).
    pub nsfw: bool,
    /// The channel topic (0-1024 characters).
    pub topic: Option<CompactString>,
    /// The last message sent (it may not exist any more).
    pub last_message_id: Option<MessageId>,
    /// Slowmode: seconds a user must wait between messages (0-21600). Bots and members with
    /// `MANAGE_MESSAGES`/`MANAGE_CHANNELS` are exempt.
    pub rate_limit_per_user: u16,
    /// When the last message was pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<Timestamp>,
    /// The auto-archive duration copied onto new threads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<AutoArchiveDuration>,
    /// The slowmode copied onto new threads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u16>,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// A voice or stage channel in a guild.
///
/// Voice channels also have a text chat.
///
/// discord.js counterpart: `VoiceChannel` / `StageChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct VoiceChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The guild; `None` inside `GUILD_CREATE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel's name (1-100 characters).
    pub name: CompactString,
    /// The sorting position.
    pub position: i32,
    /// Explicit permissions for roles and members.
    pub permission_overwrites: Vec<Overwrite>,
    /// The parent category.
    pub parent_id: Option<ChannelId>,
    /// Whether the channel is age-restricted.
    pub nsfw: bool,
    /// The topic of a stage channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<CompactString>,
    /// The last message sent in the channel's text chat.
    pub last_message_id: Option<MessageId>,
    /// Slowmode of the text chat, in seconds.
    pub rate_limit_per_user: u16,
    /// The audio bitrate in bits per second.
    pub bitrate: u32,
    /// The maximum number of users; `0` means no limit.
    pub user_limit: u32,
    /// The [voice region](https://docs.discord.com/developers/resources/voice#voice-region-object);
    /// `None` means automatic.
    pub rtc_region: Option<CompactString>,
    /// The camera video quality.
    pub video_quality_mode: VideoQualityMode,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// A category that organizes channels.
///
/// discord.js counterpart: `CategoryChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CategoryChannel {
    /// The category's ID.
    pub id: ChannelId,
    /// The guild; `None` inside `GUILD_CREATE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The category's name.
    pub name: CompactString,
    /// The sorting position.
    pub position: i32,
    /// Explicit permissions, synced onto channels that follow the category.
    pub permission_overwrites: Vec<Overwrite>,
    /// Whether the category is age-restricted.
    pub nsfw: bool,
    /// The category's flags.
    pub flags: ChannelFlags,
}

/// A forum or media channel: it only contains threads (posts).
///
/// discord.js counterpart: `ForumChannel` / `MediaChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ForumChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The guild; `None` inside `GUILD_CREATE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel's name.
    pub name: CompactString,
    /// The sorting position.
    pub position: i32,
    /// Explicit permissions for roles and members.
    pub permission_overwrites: Vec<Overwrite>,
    /// The parent category.
    pub parent_id: Option<ChannelId>,
    /// Whether the channel is age-restricted.
    pub nsfw: bool,
    /// The guidelines shown to posters (0-4096 characters).
    pub topic: Option<CompactString>,
    /// The last thread created (it may not exist any more).
    pub last_message_id: Option<MessageId>,
    /// Slowmode for creating posts, in seconds.
    pub rate_limit_per_user: u16,
    /// The auto-archive duration copied onto new posts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<AutoArchiveDuration>,
    /// The tags posts can have (up to 20).
    pub available_tags: Vec<ForumTag>,
    /// The slowmode copied onto new posts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u16>,
    /// The emoji shown on the add-reaction button of posts.
    pub default_reaction_emoji: Option<DefaultReaction>,
    /// How posts are sorted by default; `None` when not set.
    pub default_sort_order: Option<SortOrderType>,
    /// How posts are displayed by default (forum channels only).
    pub default_forum_layout: ForumLayoutType,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// A thread: a temporary sub-channel of a text, announcement, forum or media channel.
///
/// discord.js counterpart: `ThreadChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ThreadChannel {
    /// The thread's ID (for threads started from a message, the message's ID).
    pub id: ChannelId,
    /// The guild; `None` inside `GUILD_CREATE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The thread's name.
    pub name: CompactString,
    /// The channel the thread was created in.
    pub parent_id: Option<ChannelId>,
    /// Who created the thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<UserId>,
    /// Whether the thread is age-restricted (inherited from the parent).
    pub nsfw: bool,
    /// The last message sent (it may not exist any more).
    pub last_message_id: Option<MessageId>,
    /// Slowmode, in seconds.
    pub rate_limit_per_user: u16,
    /// When the last message was pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<Timestamp>,
    /// The number of messages, not counting the starter message and deleted messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u32>,
    /// The approximate number of members; stops counting at 50.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    /// The number of messages ever sent; unlike `message_count` it does not go down when
    /// messages are deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_message_sent: Option<u32>,
    /// Thread-specific data.
    pub thread_metadata: ThreadMetadata,
    /// The current user's membership, when they joined the thread (only in some endpoints).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<ThreadMember>>,
    /// The tags applied to a forum or media post.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied_tags: Vec<TagId>,
    /// The thread's flags.
    pub flags: ChannelFlags,
}

/// A direct message between the current user and one other user.
///
/// discord.js counterpart: `DMChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct DmChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The last message sent (it may not exist any more).
    pub last_message_id: Option<MessageId>,
    /// The other user(s) of the DM.
    pub recipients: Vec<User>,
    /// When the last message was pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<Timestamp>,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// A direct message between several users.
///
/// discord.js counterpart: `PartialGroupDMChannel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GroupDmChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The group's name.
    pub name: Option<CompactString>,
    /// The [group icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The users in the group.
    pub recipients: Vec<User>,
    /// Who created the group.
    pub owner_id: UserId,
    /// The application that created the group, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
    /// Whether an application manages the group through the `gdm.join` scope.
    pub managed: bool,
    /// The last message sent (it may not exist any more).
    pub last_message_id: Option<MessageId>,
    /// When the last message was pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<Timestamp>,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// The directory channel of a Student Hub.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct DirectoryChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The guild; `None` inside `GUILD_CREATE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel's name.
    pub name: CompactString,
    /// The sorting position.
    pub position: i32,
    /// Explicit permissions for roles and members.
    pub permission_overwrites: Vec<Overwrite>,
    /// The parent category.
    pub parent_id: Option<ChannelId>,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// A channel of a [`ChannelType`] this version of the library does not know about. Only the
/// fields every channel has are kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct UnknownChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The raw channel type.
    #[serde(skip)]
    pub kind: ChannelType,
    /// The guild, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel's name, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The parent channel, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ChannelId>,
    /// The channel's flags.
    pub flags: ChannelFlags,
}

/// Every field of every kind of channel; decoded in one pass and then converted into the
/// [`Channel`] variant selected by `type` (D-057).
#[derive(Deserialize)]
struct RawChannel {
    id: ChannelId,
    #[serde(rename = "type")]
    kind: ChannelType,
    #[serde(default)]
    guild_id: Option<GuildId>,
    #[serde(default)]
    position: Option<i32>,
    #[serde(default)]
    permission_overwrites: Vec<Overwrite>,
    #[serde(default)]
    name: Option<CompactString>,
    #[serde(default)]
    topic: Option<CompactString>,
    #[serde(default)]
    nsfw: bool,
    #[serde(default)]
    last_message_id: Option<MessageId>,
    #[serde(default)]
    bitrate: Option<u32>,
    #[serde(default)]
    user_limit: Option<u32>,
    #[serde(default)]
    rate_limit_per_user: Option<u16>,
    #[serde(default)]
    recipients: Vec<User>,
    #[serde(default)]
    icon: Option<ImageHash>,
    #[serde(default)]
    owner_id: Option<UserId>,
    #[serde(default)]
    application_id: Option<ApplicationId>,
    #[serde(default)]
    managed: bool,
    #[serde(default)]
    parent_id: Option<ChannelId>,
    #[serde(default)]
    last_pin_timestamp: Option<Timestamp>,
    #[serde(default)]
    rtc_region: Option<CompactString>,
    #[serde(default)]
    video_quality_mode: Option<VideoQualityMode>,
    #[serde(default)]
    message_count: Option<u32>,
    #[serde(default)]
    member_count: Option<u32>,
    #[serde(default)]
    thread_metadata: Option<ThreadMetadata>,
    #[serde(default)]
    member: Option<Box<ThreadMember>>,
    #[serde(default)]
    default_auto_archive_duration: Option<AutoArchiveDuration>,
    #[serde(default)]
    flags: ChannelFlags,
    #[serde(default)]
    total_message_sent: Option<u32>,
    #[serde(default)]
    available_tags: Vec<ForumTag>,
    #[serde(default)]
    applied_tags: Vec<TagId>,
    #[serde(default)]
    default_reaction_emoji: Option<DefaultReaction>,
    #[serde(default)]
    default_thread_rate_limit_per_user: Option<u16>,
    #[serde(default)]
    default_sort_order: Option<SortOrderType>,
    #[serde(default)]
    default_forum_layout: Option<ForumLayoutType>,
}

impl RawChannel {
    fn name<E: DeError>(&mut self) -> Result<CompactString, E> {
        self.name.take().ok_or_else(|| E::missing_field("name"))
    }

    fn into_channel<E: DeError>(mut self) -> Result<Channel, E> {
        Ok(match self.kind {
            ChannelType::GuildText => Channel::Text(self.text()?),
            ChannelType::GuildAnnouncement => Channel::Announcement(self.text()?),
            ChannelType::GuildVoice => Channel::Voice(self.voice()?),
            ChannelType::GuildStageVoice => Channel::Stage(self.voice()?),
            ChannelType::GuildForum => Channel::Forum(self.forum()?),
            ChannelType::GuildMedia => Channel::Media(self.forum()?),
            ChannelType::AnnouncementThread => Channel::AnnouncementThread(self.thread()?),
            ChannelType::PublicThread => Channel::PublicThread(self.thread()?),
            ChannelType::PrivateThread => Channel::PrivateThread(self.thread()?),
            ChannelType::GuildCategory => Channel::Category(CategoryChannel {
                id: self.id,
                guild_id: self.guild_id,
                name: self.name()?,
                position: self.position.unwrap_or_default(),
                permission_overwrites: self.permission_overwrites,
                nsfw: self.nsfw,
                flags: self.flags,
            }),
            ChannelType::Dm => Channel::Dm(DmChannel {
                id: self.id,
                last_message_id: self.last_message_id,
                recipients: self.recipients,
                last_pin_timestamp: self.last_pin_timestamp,
                flags: self.flags,
            }),
            ChannelType::GroupDm => Channel::GroupDm(GroupDmChannel {
                id: self.id,
                name: self.name,
                icon: self.icon,
                recipients: self.recipients,
                owner_id: self.owner_id.ok_or_else(|| E::missing_field("owner_id"))?,
                application_id: self.application_id,
                managed: self.managed,
                last_message_id: self.last_message_id,
                last_pin_timestamp: self.last_pin_timestamp,
                flags: self.flags,
            }),
            ChannelType::GuildDirectory => Channel::Directory(DirectoryChannel {
                id: self.id,
                guild_id: self.guild_id,
                name: self.name()?,
                position: self.position.unwrap_or_default(),
                permission_overwrites: self.permission_overwrites,
                parent_id: self.parent_id,
                flags: self.flags,
            }),
            ChannelType::Unknown(_) => Channel::Unknown(UnknownChannel {
                id: self.id,
                kind: self.kind,
                guild_id: self.guild_id,
                name: self.name,
                parent_id: self.parent_id,
                flags: self.flags,
            }),
        })
    }

    fn text<E: DeError>(mut self) -> Result<TextChannel, E> {
        Ok(TextChannel {
            id: self.id,
            guild_id: self.guild_id,
            name: self.name()?,
            position: self.position.unwrap_or_default(),
            permission_overwrites: self.permission_overwrites,
            parent_id: self.parent_id,
            nsfw: self.nsfw,
            topic: self.topic,
            last_message_id: self.last_message_id,
            rate_limit_per_user: self.rate_limit_per_user.unwrap_or_default(),
            last_pin_timestamp: self.last_pin_timestamp,
            default_auto_archive_duration: self.default_auto_archive_duration,
            default_thread_rate_limit_per_user: self.default_thread_rate_limit_per_user,
            flags: self.flags,
        })
    }

    fn voice<E: DeError>(mut self) -> Result<VoiceChannel, E> {
        Ok(VoiceChannel {
            id: self.id,
            guild_id: self.guild_id,
            name: self.name()?,
            position: self.position.unwrap_or_default(),
            permission_overwrites: self.permission_overwrites,
            parent_id: self.parent_id,
            nsfw: self.nsfw,
            topic: self.topic,
            last_message_id: self.last_message_id,
            rate_limit_per_user: self.rate_limit_per_user.unwrap_or_default(),
            bitrate: self.bitrate.unwrap_or_default(),
            user_limit: self.user_limit.unwrap_or_default(),
            rtc_region: self.rtc_region,
            video_quality_mode: self.video_quality_mode.unwrap_or_default(),
            flags: self.flags,
        })
    }

    fn forum<E: DeError>(mut self) -> Result<ForumChannel, E> {
        Ok(ForumChannel {
            id: self.id,
            guild_id: self.guild_id,
            name: self.name()?,
            position: self.position.unwrap_or_default(),
            permission_overwrites: self.permission_overwrites,
            parent_id: self.parent_id,
            nsfw: self.nsfw,
            topic: self.topic,
            last_message_id: self.last_message_id,
            rate_limit_per_user: self.rate_limit_per_user.unwrap_or_default(),
            default_auto_archive_duration: self.default_auto_archive_duration,
            available_tags: self.available_tags,
            default_thread_rate_limit_per_user: self.default_thread_rate_limit_per_user,
            default_reaction_emoji: self.default_reaction_emoji,
            default_sort_order: self.default_sort_order,
            default_forum_layout: self.default_forum_layout.unwrap_or_default(),
            flags: self.flags,
        })
    }

    fn thread<E: DeError>(mut self) -> Result<ThreadChannel, E> {
        Ok(ThreadChannel {
            id: self.id,
            guild_id: self.guild_id,
            name: self.name()?,
            parent_id: self.parent_id,
            owner_id: self.owner_id,
            nsfw: self.nsfw,
            last_message_id: self.last_message_id,
            rate_limit_per_user: self.rate_limit_per_user.unwrap_or_default(),
            last_pin_timestamp: self.last_pin_timestamp,
            message_count: self.message_count,
            member_count: self.member_count,
            total_message_sent: self.total_message_sent,
            thread_metadata: self
                .thread_metadata
                .ok_or_else(|| E::missing_field("thread_metadata"))?,
            member: self.member,
            applied_tags: self.applied_tags,
            flags: self.flags,
        })
    }
}

impl<'de> Deserialize<'de> for Channel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        RawChannel::deserialize(deserializer)?.into_channel()
    }
}

/// Serializes a variant's struct together with its `type`.
#[derive(Serialize)]
struct Tagged<'a, T> {
    #[serde(rename = "type")]
    kind: ChannelType,
    #[serde(flatten)]
    channel: &'a T,
}

impl Serialize for Channel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = self.kind();
        match self {
            Self::Text(channel) | Self::Announcement(channel) => {
                Tagged { kind, channel }.serialize(serializer)
            }
            Self::Dm(channel) => Tagged { kind, channel }.serialize(serializer),
            Self::Voice(channel) | Self::Stage(channel) => {
                Tagged { kind, channel }.serialize(serializer)
            }
            Self::GroupDm(channel) => Tagged { kind, channel }.serialize(serializer),
            Self::Category(channel) => Tagged { kind, channel }.serialize(serializer),
            Self::AnnouncementThread(channel)
            | Self::PublicThread(channel)
            | Self::PrivateThread(channel) => Tagged { kind, channel }.serialize(serializer),
            Self::Directory(channel) => Tagged { kind, channel }.serialize(serializer),
            Self::Forum(channel) | Self::Media(channel) => {
                Tagged { kind, channel }.serialize(serializer)
            }
            Self::Unknown(channel) => Tagged { kind, channel }.serialize(serializer),
        }
    }
}

/// A [permission overwrite](https://docs.discord.com/developers/resources/channel#overwrite-object):
/// permissions explicitly allowed or denied for a role or member in a channel.
///
/// discord.js counterpart: `PermissionOverwrites`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "PermissionOverwrite")]
pub struct Overwrite {
    /// The role or user ID, depending on [`kind`](Self::kind).
    pub id: GenericId,
    /// Whether the overwrite is for a role or a member.
    #[serde(rename = "type")]
    pub kind: OverwriteType,
    /// The permissions explicitly allowed.
    pub allow: Permissions,
    /// The permissions explicitly denied.
    pub deny: Permissions,
}

impl Overwrite {
    /// An overwrite of `kind` for the role or user `id`.
    #[must_use]
    pub const fn new(
        id: GenericId,
        kind: OverwriteType,
        allow: Permissions,
        deny: Permissions,
    ) -> Self {
        Self {
            id,
            kind,
            allow,
            deny,
        }
    }

    /// An overwrite for a role (`@everyone` has the guild's ID).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::{channel::Overwrite, id::RoleId, permissions::Permissions};
    ///
    /// let read_only = Overwrite::role(RoleId::new(1), Permissions::VIEW_CHANNEL, Permissions::SEND_MESSAGES);
    /// assert_eq!(read_only.role_id(), Some(RoleId::new(1)));
    /// ```
    #[must_use]
    pub const fn role(role_id: RoleId, allow: Permissions, deny: Permissions) -> Self {
        Self::new(role_id.cast(), OverwriteType::Role, allow, deny)
    }

    /// An overwrite for a member.
    #[must_use]
    pub const fn member(user_id: UserId, allow: Permissions, deny: Permissions) -> Self {
        Self::new(user_id.cast(), OverwriteType::Member, allow, deny)
    }

    /// The role this overwrite applies to, if it is a role overwrite.
    #[must_use]
    pub fn role_id(&self) -> Option<RoleId> {
        (self.kind == OverwriteType::Role).then(|| self.id.cast())
    }

    /// The member this overwrite applies to, if it is a member overwrite.
    #[must_use]
    pub fn user_id(&self) -> Option<UserId> {
        (self.kind == OverwriteType::Member).then(|| self.id.cast())
    }
}

int_enum! {
    /// Whether an [`Overwrite`] is for a role or a member.
    ///
    /// discord.js counterpart: `OverwriteType`.
    pub enum OverwriteType: u8 {
        /// A role overwrite.
        Role = 0,
        /// A member overwrite.
        Member = 1,
    }
}

int_enum! {
    /// The [camera video quality](https://docs.discord.com/developers/resources/channel#channel-object-video-quality-modes)
    /// of a voice channel.
    ///
    /// discord.js counterpart: `VideoQualityMode`.
    #[derive(Default)]
    pub enum VideoQualityMode: u8 {
        /// Discord picks the best quality (the default when absent).
        #[default]
        Auto = 1,
        /// 720p.
        Full = 2,
    }
}

int_enum! {
    /// How the posts of a forum or media channel are [sorted](https://docs.discord.com/developers/resources/channel#channel-object-sort-order-types).
    ///
    /// discord.js counterpart: `SortOrderType`.
    pub enum SortOrderType: u8 {
        /// By recent activity.
        LatestActivity = 0,
        /// By creation time, newest first.
        CreationDate = 1,
    }
}

int_enum! {
    /// How the posts of a forum channel are [displayed](https://docs.discord.com/developers/resources/channel#channel-object-forum-layout-types).
    ///
    /// discord.js counterpart: `ForumLayoutType`.
    #[derive(Default)]
    pub enum ForumLayoutType: u8 {
        /// No default set (the default when absent).
        #[default]
        NotSet = 0,
        /// A list.
        ListView = 1,
        /// A gallery of tiles.
        GalleryView = 2,
    }
}

int_enum! {
    /// After how many minutes of inactivity a thread is archived (hidden from the channel
    /// list). Discord only accepts these values.
    ///
    /// discord.js counterpart: `ThreadAutoArchiveDuration`.
    #[doc(alias = "ThreadAutoArchiveDuration")]
    pub enum AutoArchiveDuration: u16 {
        /// One hour.
        OneHour = 60,
        /// One day.
        OneDay = 1440,
        /// Three days.
        ThreeDays = 4320,
        /// One week.
        OneWeek = 10080,
    }
}

/// [Thread-specific data](https://docs.discord.com/developers/resources/channel#thread-metadata-object).
///
/// discord.js counterpart: the thread fields of `ThreadChannel` (`archived`, `locked`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadMetadata {
    /// Whether the thread is archived.
    pub archived: bool,
    /// After how long without activity the thread is archived.
    pub auto_archive_duration: AutoArchiveDuration,
    /// When the archive status last changed (used to compute recent activity).
    pub archive_timestamp: Timestamp,
    /// Whether the thread is locked: only members with `MANAGE_THREADS` can unarchive it.
    pub locked: bool,
    /// Whether non-moderators can add other non-moderators (private threads only).
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub invitable: bool,
    /// When the thread was created; only set for threads created after 2022-01-09.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_timestamp: Option<Timestamp>,
}

/// A [user's membership in a thread](https://docs.discord.com/developers/resources/channel#thread-member-object).
///
/// discord.js counterpart: `ThreadMember`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadMember {
    /// The thread's ID (omitted inside `GUILD_CREATE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ChannelId>,
    /// The user's ID (omitted inside `GUILD_CREATE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<UserId>,
    /// When the user last joined the thread.
    pub join_timestamp: Timestamp,
    /// The user's thread settings (notifications).
    pub flags: ThreadMemberFlags,
    /// The user's guild member data (with `with_member=true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<Member>>,
}

int_flags! {
    /// The settings of a [`ThreadMember`], currently about notifications.
    ///
    /// discord.js counterpart: `ThreadMemberFlags`.
    pub struct ThreadMemberFlags: u64 {
        /// The user has interacted with the thread.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const HAS_INTERACTED = 1 << 0;
        /// The user is notified of all messages.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const ALL_MESSAGES = 1 << 1;
        /// The user is only notified of mentions.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const ONLY_MENTIONS = 1 << 2;
        /// The user is not notified.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const NO_MESSAGES = 1 << 3;
    }
}

/// The [emoji of the add-reaction button](https://docs.discord.com/developers/resources/channel#default-reaction-object)
/// on posts of a forum or media channel. Exactly one of the fields is set.
///
/// discord.js counterpart: `DefaultReactionEmoji`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DefaultReaction {
    /// A custom emoji of the guild.
    pub emoji_id: Option<EmojiId>,
    /// A Unicode emoji.
    pub emoji_name: Option<CompactString>,
}

impl DefaultReaction {
    /// A custom emoji of the guild.
    #[must_use]
    pub const fn custom(emoji_id: EmojiId) -> Self {
        Self {
            emoji_id: Some(emoji_id),
            emoji_name: None,
        }
    }

    /// A Unicode emoji (e.g. `"👍"`).
    #[must_use]
    pub fn unicode(emoji: impl Into<CompactString>) -> Self {
        Self {
            emoji_id: None,
            emoji_name: Some(emoji.into()),
        }
    }
}

/// A [tag](https://docs.discord.com/developers/resources/channel#forum-tag-object) that can be
/// applied to posts of a forum or media channel.
///
/// discord.js counterpart: `GuildForumTag`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ForumTag {
    /// The tag's ID.
    pub id: TagId,
    /// The tag's name (0-20 characters).
    pub name: CompactString,
    /// Whether only members with `MANAGE_THREADS` can add or remove the tag.
    pub moderated: bool,
    /// The tag's custom emoji.
    pub emoji_id: Option<EmojiId>,
    /// The tag's Unicode emoji.
    pub emoji_name: Option<CompactString>,
}

/// The result of [following an announcement channel](https://docs.discord.com/developers/resources/channel#followed-channel-object).
///
/// discord.js counterpart: the return value of `NewsChannel#addFollower`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FollowedChannel {
    /// The followed (source) channel.
    pub channel_id: ChannelId,
    /// The webhook created in the target channel.
    pub webhook_id: WebhookId,
}

int_flags! {
    /// [Flags of a channel](https://docs.discord.com/developers/resources/channel#channel-object-channel-flags).
    ///
    /// discord.js counterpart: `ChannelFlags` / `ChannelFlagsBitField`.
    pub struct ChannelFlags: u64 {
        /// The channel was removed from the guild's feed.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const GUILD_FEED_REMOVED = 1 << 0;
        /// The thread is pinned to the top of its forum or media channel.
        const PINNED = 1 << 1;
        /// The channel was removed from the guild's active channels.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const ACTIVE_CHANNELS_REMOVED = 1 << 2;
        /// New threads in this forum or media channel must have a tag (`applied_tags`).
        const REQUIRE_TAG = 1 << 4;
        /// The channel is marked as spam.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const IS_SPAM = 1 << 5;
        /// The channel is a guild resource channel (Server Guide).
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const IS_GUILD_RESOURCE_CHANNEL = 1 << 7;
        /// Clyde AI is enabled in the channel (feature retired by Discord).
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const CLYDE_AI = 1 << 8;
        /// The channel is scheduled for deletion.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const IS_SCHEDULED_FOR_DELETION = 1 << 9;
        /// Media download options are hidden. Media channels only.
        const HIDE_MEDIA_DOWNLOAD_OPTIONS = 1 << 15;
        /// The channel's metadata is hidden because the current user cannot view the channel.
        /// Only set on channels received over the gateway.
        const CHANNEL_OBFUSCATED = 1 << 17;
        /// A spoiler channel: users must opt in to see its contents. Textual guild channels and
        /// voice channels.
        const IS_SPOILER_CHANNEL = 1 << 21;
    }
}
