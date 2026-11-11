//! Channel, thread, invite, stage, webhook and voice events.

use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

use super::{guild_id_event, newtype_event};
use crate::{
    application::PartialApplication,
    channel::{Channel, ChannelType, ThreadMember},
    emoji::PartialEmoji,
    gateway::Presence,
    id::{ChannelId, GuildId, RoleId, SoundboardSoundId, UserId},
    invite::InviteTargetType,
    macros::int_enum,
    serde_util::extras::{self, Extras, Visit},
    stage_instance::StageInstance,
    user::User,
    util::Timestamp,
    voice::VoiceState,
};

newtype_event! {
    /// The data of the [Channel Create](https://docs.discord.com/developers/events/gateway-events#channel-create)
    /// dispatch event: a guild channel relevant to the bot was created.
    ///
    /// discord.js counterpart: the `ChannelCreate` event.
    ChannelCreate(Channel)
}

newtype_event! {
    /// The data of the [Channel Update](https://docs.discord.com/developers/events/gateway-events#channel-update)
    /// dispatch event (not sent for `last_message_id` changes).
    ///
    /// discord.js counterpart: the `ChannelUpdate` event.
    ChannelUpdate(Channel)
}

newtype_event! {
    /// The data of the [Channel Delete](https://docs.discord.com/developers/events/gateway-events#channel-delete)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `ChannelDelete` event.
    ChannelDelete(Channel)
}

/// The data of the [Channel Info](https://docs.discord.com/developers/events/gateway-events#channel-info)
/// dispatch event: the ephemeral data of a guild's channels, sent in response to a
/// [`RequestChannelInfo`](crate::gateway::payload::RequestChannelInfo) command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChannelInfo {
    /// The guild.
    pub guild_id: GuildId,
    /// The channels' ephemeral data.
    #[serde(default)]
    pub channels: Vec<ChannelInfoChannel>,
}

/// The [ephemeral data of a channel](https://docs.discord.com/developers/events/gateway-events#channel-info-channel-info-channel-structure)
/// in a [`ChannelInfo`] event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChannelInfoChannel {
    /// The channel.
    pub id: ChannelId,
    /// The voice channel status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<CompactString>,
    /// When the voice session started (sent as Unix seconds, D-075).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_unix_secs"
    )]
    pub voice_start_time: Option<Timestamp>,
}

/// The data of the [Voice Channel Status Update](https://docs.discord.com/developers/events/gateway-events#voice-channel-status-update)
/// dispatch event.
///
/// discord.js counterpart: the `VoiceChannelStatusUpdate` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoiceChannelStatusUpdate {
    /// The channel.
    pub id: ChannelId,
    /// The guild.
    pub guild_id: GuildId,
    /// The new status (`None` when cleared).
    pub status: Option<CompactString>,
}

/// The data of the [Voice Channel Start Time Update](https://docs.discord.com/developers/events/gateway-events#voice-channel-start-time-update)
/// dispatch event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoiceChannelStartTimeUpdate {
    /// The channel.
    pub id: ChannelId,
    /// The guild.
    pub guild_id: GuildId,
    /// When the voice session started (sent as Unix seconds, D-075).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_unix_secs"
    )]
    pub voice_start_time: Option<Timestamp>,
}

/// The data of the [Thread Create](https://docs.discord.com/developers/events/gateway-events#thread-create)
/// dispatch event: a thread was created, or the bot was added to a private thread (then the
/// thread includes the bot's [`ThreadMember`]).
///
/// Dereferences to the contained [`Channel`].
///
/// discord.js counterpart: the `ThreadCreate` event (`newlyCreated`).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThreadCreate {
    /// The thread.
    pub thread: Channel,
    /// Whether the thread was just created (`false` when the bot was added to an existing
    /// thread).
    pub newly_created: bool,
}

impl core::ops::Deref for ThreadCreate {
    type Target = Channel;

    fn deref(&self) -> &Channel {
        &self.thread
    }
}

#[derive(Default)]
struct ThreadCreateExtra {
    newly_created: bool,
}

impl<'de> Extras<'de> for ThreadCreateExtra {
    fn visit<A: serde::de::MapAccess<'de>>(
        &mut self,
        key: &str,
        map: &mut A,
    ) -> Result<Visit, A::Error> {
        if key == "newly_created" {
            self.newly_created = map.next_value()?;
            Ok(Visit::Consumed)
        } else {
            Ok(Visit::Pass)
        }
    }
}

impl<'de> Deserialize<'de> for ThreadCreate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (thread, extra) = extras::deserialize::<Channel, ThreadCreateExtra, D>(deserializer)?;
        Ok(Self {
            thread: thread.ok_or_else(|| D::Error::custom("missing event data"))?,
            newly_created: extra.newly_created,
        })
    }
}

impl Serialize for ThreadCreate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(flatten)]
            thread: &'a Channel,
            #[serde(skip_serializing_if = "core::ops::Not::not")]
            newly_created: bool,
        }

        Flat {
            thread: &self.thread,
            newly_created: self.newly_created,
        }
        .serialize(serializer)
    }
}

newtype_event! {
    /// The data of the [Thread Update](https://docs.discord.com/developers/events/gateway-events#thread-update)
    /// dispatch event (not sent for `last_message_id` changes).
    ///
    /// discord.js counterpart: the `ThreadUpdate` event.
    ThreadUpdate(Channel)
}

/// The data of the [Thread Delete](https://docs.discord.com/developers/events/gateway-events#thread-delete)
/// dispatch event: only the thread's IDs and type.
///
/// discord.js counterpart: the `ThreadDelete` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadDelete {
    /// The thread.
    pub id: ChannelId,
    /// The guild.
    pub guild_id: GuildId,
    /// The parent channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ChannelId>,
    /// The thread's type.
    #[serde(rename = "type")]
    pub kind: ChannelType,
}

/// The data of the [Thread List Sync](https://docs.discord.com/developers/events/gateway-events#thread-list-sync)
/// dispatch event: the active threads of channels the bot gained access to.
///
/// discord.js counterpart: the `ThreadListSync` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadListSync {
    /// The guild.
    pub guild_id: GuildId,
    /// The parent channels whose threads are synced (may include channels without active
    /// threads); `None` means the whole guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_ids: Option<Vec<ChannelId>>,
    /// The active threads the bot can access in those channels.
    #[serde(default)]
    pub threads: Vec<Channel>,
    /// The bot's thread members of the synced threads (which threads it was added to).
    #[serde(default)]
    pub members: Vec<ThreadMember>,
}

guild_id_event! {
    /// The data of the [Thread Member Update](https://docs.discord.com/developers/events/gateway-events#thread-member-update)
    /// dispatch event: the bot's thread member changed.
    ///
    /// discord.js counterpart: the `ThreadMemberUpdate` event.
    ThreadMemberUpdate {
        /// The bot's thread member.
        member: ThreadMember
    }
}

/// The data of the [Thread Members Update](https://docs.discord.com/developers/events/gateway-events#thread-members-update)
/// dispatch event: users were added to or removed from a thread (without the `GUILD_MEMBERS`
/// intent: only the bot).
///
/// discord.js counterpart: the `ThreadMembersUpdate` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadMembersUpdate {
    /// The thread.
    pub id: ChannelId,
    /// The guild.
    pub guild_id: GuildId,
    /// The approximate number of members, capped at 50.
    pub member_count: u32,
    /// The added members.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_members: Vec<AddedThreadMember>,
    /// The removed users.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_member_ids: Vec<UserId>,
}

/// A thread member added in a [`ThreadMembersUpdate`]: with its guild member (in
/// [`ThreadMember::member`]) and presence.
///
/// Dereferences to the contained [`ThreadMember`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AddedThreadMember {
    /// The thread member.
    pub member: ThreadMember,
    /// The member's presence.
    pub presence: Option<Presence>,
}

impl core::ops::Deref for AddedThreadMember {
    type Target = ThreadMember;

    fn deref(&self) -> &ThreadMember {
        &self.member
    }
}

#[derive(Default)]
struct PresenceExtra {
    presence: Option<Presence>,
}

impl<'de> Extras<'de> for PresenceExtra {
    fn visit<A: serde::de::MapAccess<'de>>(
        &mut self,
        key: &str,
        map: &mut A,
    ) -> Result<Visit, A::Error> {
        if key == "presence" {
            self.presence = map.next_value()?;
            Ok(Visit::Consumed)
        } else {
            Ok(Visit::Pass)
        }
    }
}

impl<'de> Deserialize<'de> for AddedThreadMember {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (member, extra) = extras::deserialize::<ThreadMember, PresenceExtra, D>(deserializer)?;
        Ok(Self {
            member: member.ok_or_else(|| D::Error::custom("missing event data"))?,
            presence: extra.presence,
        })
    }
}

impl Serialize for AddedThreadMember {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(flatten)]
            member: &'a ThreadMember,
            presence: Option<&'a Presence>,
        }

        Flat {
            member: &self.member,
            presence: self.presence.as_ref(),
        }
        .serialize(serializer)
    }
}

/// The data of the [Channel Pins Update](https://docs.discord.com/developers/events/gateway-events#channel-pins-update)
/// dispatch event (not sent when a pinned message is deleted).
///
/// discord.js counterpart: the `ChannelPinsUpdate` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChannelPinsUpdate {
    /// The guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The channel.
    pub channel_id: ChannelId,
    /// When the most recent pin was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<Timestamp>,
}

/// The data of the [Invite Create](https://docs.discord.com/developers/events/gateway-events#invite-create)
/// dispatch event (requires `MANAGE_CHANNELS` on the channel).
///
/// discord.js counterpart: the `InviteCreate` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteCreate {
    /// The channel the invite is for.
    pub channel_id: ChannelId,
    /// The invite code.
    pub code: CompactString,
    /// When the invite was created.
    pub created_at: Timestamp,
    /// The invite's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// Who created the invite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inviter: Option<Box<User>>,
    /// How long the invite is valid, in seconds (`0` = forever).
    pub max_age: u32,
    /// How often it can be used (`0` = unlimited).
    pub max_uses: u32,
    /// What a voice channel invite targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<InviteTargetType>,
    /// The user whose stream a stream invite shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_user: Option<Box<User>>,
    /// The embedded application an application invite opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_application: Option<Box<PartialApplication>>,
    /// Whether users who join through it are kicked on disconnect unless given a role.
    #[serde(default)]
    pub temporary: bool,
    /// How often it was used (always `0` here).
    #[serde(default)]
    pub uses: u32,
    /// When the invite expires.
    #[serde(default)]
    pub expires_at: Option<Timestamp>,
    /// The roles given to users who accept the invite.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<RoleId>,
}

/// The data of the [Invite Delete](https://docs.discord.com/developers/events/gateway-events#invite-delete)
/// dispatch event.
///
/// discord.js counterpart: the `InviteDelete` event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InviteDelete {
    /// The invite's channel.
    pub channel_id: ChannelId,
    /// The invite's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The invite code.
    pub code: CompactString,
}

newtype_event! {
    /// The data of the [Stage Instance Create](https://docs.discord.com/developers/events/gateway-events#stage-instance-create)
    /// dispatch event: a stage went live.
    ///
    /// discord.js counterpart: the `StageInstanceCreate` event.
    StageInstanceCreate(StageInstance)
}

newtype_event! {
    /// The data of the [Stage Instance Update](https://docs.discord.com/developers/events/gateway-events#stage-instance-update)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `StageInstanceUpdate` event.
    StageInstanceUpdate(StageInstance)
}

newtype_event! {
    /// The data of the [Stage Instance Delete](https://docs.discord.com/developers/events/gateway-events#stage-instance-delete)
    /// dispatch event: a stage ended.
    ///
    /// discord.js counterpart: the `StageInstanceDelete` event.
    StageInstanceDelete(StageInstance)
}

/// The data of the [Webhooks Update](https://docs.discord.com/developers/events/gateway-events#webhooks-update)
/// dispatch event: a channel's webhooks changed.
///
/// discord.js counterpart: the `WebhooksUpdate` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhooksUpdate {
    /// The guild.
    pub guild_id: GuildId,
    /// The channel.
    pub channel_id: ChannelId,
}

newtype_event! {
    /// The data of the [Voice State Update](https://docs.discord.com/developers/events/gateway-events#voice-state-update)
    /// dispatch event: someone joined, left or moved between voice channels.
    ///
    /// discord.js counterpart: the `VoiceStateUpdate` event.
    VoiceStateUpdate(VoiceState)
}

/// The data of the [Voice Server Update](https://docs.discord.com/developers/events/gateway-events#voice-server-update)
/// dispatch event: where to connect for voice. `Debug` does not print the token.
///
/// discord.js counterpart: the voice server data used by `@discordjs/voice`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoiceServerUpdate {
    /// The voice connection token (secret).
    pub token: CompactString,
    /// The guild.
    pub guild_id: GuildId,
    /// The voice server host; `None` while the server is reallocated (disconnect and wait for
    /// the next update).
    pub endpoint: Option<CompactString>,
}

impl core::fmt::Debug for VoiceServerUpdate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VoiceServerUpdate")
            .field("token", &"<redacted>")
            .field("guild_id", &self.guild_id)
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

/// The data of the [Voice Channel Effect Send](https://docs.discord.com/developers/events/gateway-events#voice-channel-effect-send)
/// dispatch event: someone sent an emoji reaction or soundboard sound in the bot's voice
/// channel.
///
/// discord.js counterpart: the `VoiceChannelEffectSend` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoiceChannelEffectSend {
    /// The channel.
    pub channel_id: ChannelId,
    /// The guild.
    pub guild_id: GuildId,
    /// The user who sent the effect.
    pub user_id: UserId,
    /// The emoji (emoji reactions and soundboard sounds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
    /// The kind of animation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_type: Option<VoiceChannelEffectAnimationType>,
    /// The animation's ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_id: Option<u64>,
    /// The soundboard sound (soundboard effects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound_id: Option<SoundboardSoundId>,
    /// The sound's volume, from 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound_volume: Option<f64>,
}

int_enum! {
    /// The [animation of a voice channel effect](https://docs.discord.com/developers/events/gateway-events#voice-channel-effect-send-animation-types).
    ///
    /// discord.js counterpart: `VoiceChannelEffectSendAnimationType`.
    pub enum VoiceChannelEffectAnimationType: u8 {
        /// A fun animation, sent by a Nitro subscriber.
        Premium = 0,
        /// The standard animation.
        Basic = 1,
    }
}
