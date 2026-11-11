//! [Lobbies](https://docs.discord.com/developers/resources/lobby): groups of users of an app's
//! game (Discord Social SDK), managed by the app over HTTP, optionally linked to a guild
//! channel.

use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    channel::Channel,
    id::{ApplicationId, ChannelId, LobbyId, MessageId, UserId},
    macros::int_flags,
    message::{MessageFlags, MessageType},
    user::User,
};

/// A [lobby](https://docs.discord.com/developers/resources/lobby#lobby-object).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Lobby {
    /// The lobby's ID.
    pub id: LobbyId,
    /// The application that created the lobby.
    pub application_id: ApplicationId,
    /// String key/value pairs set by the app (max 1000 characters in total; `null` and absent
    /// are empty, D-058).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub metadata: BTreeMap<CompactString, CompactString>,
    /// The lobby's members.
    #[serde(default)]
    pub members: Vec<LobbyMember>,
    /// The guild channel linked to the lobby.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_channel: Option<Box<Channel>>,
}

/// A [lobby member](https://docs.discord.com/developers/resources/lobby#lobby-member-object).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LobbyMember {
    /// The user's ID.
    pub id: UserId,
    /// String key/value pairs set by the app (max 1000 characters in total).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub metadata: BTreeMap<CompactString, CompactString>,
    /// The member's flags.
    #[serde(default, skip_serializing_if = "LobbyMemberFlags::is_empty")]
    pub flags: LobbyMemberFlags,
    /// An extra display name (1-80 characters), e.g. an in-game character name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_name: Option<CompactString>,
}

int_flags! {
    /// [Flags of a lobby member](https://docs.discord.com/developers/resources/lobby#lobby-member-object-lobby-member-flags).
    pub struct LobbyMemberFlags: u64 {
        /// The member can link a text channel to the lobby.
        const CAN_LINK_LOBBY = 1 << 0;
    }
}

/// A [message sent in a lobby](https://docs.discord.com/developers/resources/lobby#lobby-message-object).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LobbyMessage {
    /// The message's ID.
    pub id: MessageId,
    /// The message's type.
    #[serde(rename = "type")]
    pub kind: MessageType,
    /// The message's text.
    pub content: CompactString,
    /// The lobby it was sent to.
    pub lobby_id: LobbyId,
    /// Equal to [`lobby_id`](Self::lobby_id), for compatibility with messages.
    pub channel_id: ChannelId,
    /// The author.
    pub author: User,
    /// The author's lobby member data when the message was sent (if they had an additional
    /// name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lobby_member: Option<LobbyMessageMember>,
    /// Key/value pairs delivered with the message to active Social SDK clients.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub metadata: BTreeMap<CompactString, CompactString>,
    /// Moderation key/value pairs set by the app.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "crate::serde_util::null_as_default"
    )]
    pub moderation_metadata: BTreeMap<CompactString, CompactString>,
    /// The message's flags.
    #[serde(default)]
    pub flags: MessageFlags,
    /// The application that sent the message.
    pub application_id: ApplicationId,
}

/// The author's lobby member data in a [`LobbyMessage`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LobbyMessageMember {
    /// The author's additional display name.
    pub additional_name: CompactString,
}

/// A [lobby invite](https://docs.discord.com/developers/resources/lobby#create-lobby-channel-invite-for-self):
/// an invite to the lobby's linked channel.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LobbyInvite {
    /// The invite code.
    pub code: CompactString,
}
