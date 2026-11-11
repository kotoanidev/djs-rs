//! Message, reaction, poll, interaction, presence, user and monetization events.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use super::newtype_event;
use crate::{
    emoji::PartialEmoji,
    entitlement::Entitlement,
    gateway::Presence,
    guild::Member,
    id::{ChannelId, GuildId, MessageId, UserId},
    interaction::Interaction,
    message::{Message, ReactionType},
    subscription::Subscription,
    user::CurrentUser,
    util::Timestamp,
};

newtype_event! {
    /// The data of the [Message Create](https://docs.discord.com/developers/events/gateway-events#message-create)
    /// dispatch event: a new message was sent.
    ///
    /// discord.js counterpart: the `MessageCreate` event.
    MessageCreate(Message)
}

newtype_event! {
    /// The data of the [Message Update](https://docs.discord.com/developers/events/gateway-events#message-update)
    /// dispatch event: a message was edited (the full new message; `tts` is always `false`).
    ///
    /// discord.js counterpart: the `MessageUpdate` event.
    MessageUpdate(Message)
}

/// The data of the [Message Delete](https://docs.discord.com/developers/events/gateway-events#message-delete)
/// dispatch event.
///
/// discord.js counterpart: the `MessageDelete` event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageDelete {
    /// The deleted message.
    pub id: MessageId,
    /// The message's channel.
    pub channel_id: ChannelId,
    /// The message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
}

/// The data of the [Message Delete Bulk](https://docs.discord.com/developers/events/gateway-events#message-delete-bulk)
/// dispatch event.
///
/// discord.js counterpart: the `MessageBulkDelete` event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageDeleteBulk {
    /// The deleted messages.
    pub ids: Vec<MessageId>,
    /// The messages' channel.
    pub channel_id: ChannelId,
    /// The messages' guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
}

/// The data of the [Message Reaction Add](https://docs.discord.com/developers/events/gateway-events#message-reaction-add)
/// dispatch event.
///
/// discord.js counterpart: the `MessageReactionAdd` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageReactionAdd {
    /// The user who reacted.
    pub user_id: UserId,
    /// The message's channel.
    pub channel_id: ChannelId,
    /// The message.
    pub message_id: MessageId,
    /// The message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The member who reacted (guild reactions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<Member>>,
    /// The emoji.
    pub emoji: PartialEmoji,
    /// The author of the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_author_id: Option<UserId>,
    /// Whether this is a super reaction.
    #[serde(default)]
    pub burst: bool,
    /// The super reaction's animation colors (`#rrggbb`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub burst_colors: Vec<CompactString>,
    /// The kind of reaction.
    #[serde(default, rename = "type")]
    pub kind: ReactionType,
}

/// The data of the [Message Reaction Remove](https://docs.discord.com/developers/events/gateway-events#message-reaction-remove)
/// dispatch event.
///
/// discord.js counterpart: the `MessageReactionRemove` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageReactionRemove {
    /// The user whose reaction was removed.
    pub user_id: UserId,
    /// The message's channel.
    pub channel_id: ChannelId,
    /// The message.
    pub message_id: MessageId,
    /// The message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The emoji.
    pub emoji: PartialEmoji,
    /// Whether this was a super reaction.
    #[serde(default)]
    pub burst: bool,
    /// The kind of reaction.
    #[serde(default, rename = "type")]
    pub kind: ReactionType,
}

/// The data of the [Message Reaction Remove All](https://docs.discord.com/developers/events/gateway-events#message-reaction-remove-all)
/// dispatch event.
///
/// discord.js counterpart: the `MessageReactionRemoveAll` event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageReactionRemoveAll {
    /// The message's channel.
    pub channel_id: ChannelId,
    /// The message.
    pub message_id: MessageId,
    /// The message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
}

/// The data of the [Message Reaction Remove Emoji](https://docs.discord.com/developers/events/gateway-events#message-reaction-remove-emoji)
/// dispatch event: all reactions with one emoji were removed.
///
/// discord.js counterpart: the `MessageReactionRemoveEmoji` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageReactionRemoveEmoji {
    /// The message's channel.
    pub channel_id: ChannelId,
    /// The message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The message.
    pub message_id: MessageId,
    /// The removed emoji.
    pub emoji: PartialEmoji,
}

/// The data of the [Message Poll Vote Add](https://docs.discord.com/developers/events/gateway-events#message-poll-vote-add)
/// dispatch event (one event per answer for multiselect polls).
///
/// discord.js counterpart: the `MessagePollVoteAdd` event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessagePollVoteAdd {
    /// The voter.
    pub user_id: UserId,
    /// The poll message's channel.
    pub channel_id: ChannelId,
    /// The poll message.
    pub message_id: MessageId,
    /// The poll message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The answer voted for.
    pub answer_id: u32,
}

/// The data of the [Message Poll Vote Remove](https://docs.discord.com/developers/events/gateway-events#message-poll-vote-remove)
/// dispatch event (one event per answer for multiselect polls).
///
/// discord.js counterpart: the `MessagePollVoteRemove` event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessagePollVoteRemove {
    /// The user who removed the vote.
    pub user_id: UserId,
    /// The poll message's channel.
    pub channel_id: ChannelId,
    /// The poll message.
    pub message_id: MessageId,
    /// The poll message's guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The answer the vote was removed from.
    pub answer_id: u32,
}

/// The data of the [Typing Start](https://docs.discord.com/developers/events/gateway-events#typing-start)
/// dispatch event.
///
/// discord.js counterpart: the `TypingStart` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TypingStart {
    /// The channel.
    pub channel_id: ChannelId,
    /// The guild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The typing user.
    pub user_id: UserId,
    /// When the user started typing (sent as Unix seconds, D-075).
    #[serde(with = "crate::serde_util::unix_secs")]
    pub timestamp: Timestamp,
    /// The typing member (guild channels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<Member>>,
}

newtype_event! {
    /// The data of the [Interaction Create](https://docs.discord.com/developers/events/gateway-events#interaction-create)
    /// dispatch event: a user used a command, component or modal.
    ///
    /// discord.js counterpart: the `InteractionCreate` event.
    InteractionCreate(Interaction)
}

newtype_event! {
    /// The data of the [Presence Update](https://docs.discord.com/developers/events/gateway-events#presence-update)
    /// dispatch event (requires the `GUILD_PRESENCES` intent).
    ///
    /// discord.js counterpart: the `PresenceUpdate` event.
    PresenceUpdate(Presence)
}

newtype_event! {
    /// The data of the [User Update](https://docs.discord.com/developers/events/gateway-events#user-update)
    /// dispatch event: the bot's own user changed.
    ///
    /// discord.js counterpart: the `UserUpdate` event (for the client user).
    UserUpdate(CurrentUser)
}

newtype_event! {
    /// The data of the [Entitlement Create](https://docs.discord.com/developers/events/gateway-events#entitlement-create)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `EntitlementCreate` event.
    EntitlementCreate(Entitlement)
}

newtype_event! {
    /// The data of the [Entitlement Update](https://docs.discord.com/developers/events/gateway-events#entitlement-update)
    /// dispatch event (for subscriptions: when they end, with `ends_at`).
    ///
    /// discord.js counterpart: the `EntitlementUpdate` event.
    EntitlementUpdate(Entitlement)
}

newtype_event! {
    /// The data of the [Entitlement Delete](https://docs.discord.com/developers/events/gateway-events#entitlement-delete)
    /// dispatch event (refunds and removals; entitlements are not deleted when they expire).
    ///
    /// discord.js counterpart: the `EntitlementDelete` event.
    EntitlementDelete(Entitlement)
}

newtype_event! {
    /// The data of the [Subscription Create](https://docs.discord.com/developers/events/gateway-events#subscription-create)
    /// dispatch event (use entitlements, not the status, to grant perks).
    ///
    /// discord.js counterpart: the `SubscriptionCreate` event.
    SubscriptionCreate(Subscription)
}

newtype_event! {
    /// The data of the [Subscription Update](https://docs.discord.com/developers/events/gateway-events#subscription-update)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `SubscriptionUpdate` event.
    SubscriptionUpdate(Subscription)
}

newtype_event! {
    /// The data of the [Subscription Delete](https://docs.discord.com/developers/events/gateway-events#subscription-delete)
    /// dispatch event.
    ///
    /// discord.js counterpart: the `SubscriptionDelete` event.
    SubscriptionDelete(Subscription)
}
