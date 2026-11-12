//! Bodies and queries of the [lobby endpoints](https://docs.discord.com/developers/resources/lobby).
//!
//! Metadata are string key/value pairs of at most 1000 characters in total.

use core::fmt;
use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::Serialize;

use crate::{id::UserId, lobby::LobbyMemberFlags, message::MessageFlags, util::Patch};

/// String key/value pairs set by the app on a lobby, a member or a message.
pub type LobbyMetadata = BTreeMap<CompactString, CompactString>;

/// The body of [Create Lobby](https://docs.discord.com/developers/resources/lobby#create-lobby).
///
/// Social SDK clients cannot join or leave lobbies created this way.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     id::UserId,
///     rest::{CreateLobby, LobbyMemberData},
/// };
///
/// let mut body = CreateLobby::default();
/// body.metadata.insert("map".into(), "harbor".into());
/// body.members = vec![LobbyMemberData::new(UserId::new(7))];
/// body.idle_timeout_seconds = Some(600);
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"metadata":{"map":"harbor"},"members":[{"id":"7"}],"idle_timeout_seconds":600}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CreateLobby {
    /// The lobby's metadata.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: LobbyMetadata,
    /// Users to add (up to 25).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<LobbyMemberData>,
    /// How many seconds an idle lobby lives before it is shut down (5 to 604 800, seven
    /// days).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,
}

/// A member in the body of [Create Lobby](https://docs.discord.com/developers/resources/lobby#create-lobby)
/// or [Modify Lobby](https://docs.discord.com/developers/resources/lobby#modify-lobby), or one
/// to [upsert](BulkLobbyMember::upsert) with Bulk Update Lobby Members (D-134).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LobbyMemberData {
    /// The user.
    pub id: UserId,
    /// The member's metadata; `Some` of an empty map clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LobbyMetadata>,
    /// The member's flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<LobbyMemberFlags>,
    /// An extra display name (1-80 characters), e.g. an in-game character name;
    /// [`Patch::Keep`] keeps a member's current one, [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub additional_name: Patch<CompactString>,
}

impl LobbyMemberData {
    /// The member `id`, without metadata, flags or additional name.
    #[must_use]
    pub const fn new(id: UserId) -> Self {
        Self {
            id,
            metadata: None,
            flags: None,
            additional_name: Patch::Keep,
        }
    }
}

/// The body of [Create or Join Lobby](https://docs.discord.com/developers/resources/lobby#create-or-join-lobby).
///
/// The secret is redacted in the `Debug` output: anyone who knows it can join the lobby.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CreateOrJoinLobby {
    /// Identifies the lobby among the application's lobbies (up to 250 characters).
    pub secret: CompactString,
    /// How many seconds an idle lobby lives before it is shut down (5 to 604 800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,
    /// Replaces the lobby's metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lobby_metadata: Option<LobbyMetadata>,
    /// Sets the calling user's member metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_metadata: Option<LobbyMetadata>,
}

impl CreateOrJoinLobby {
    /// Creates the lobby with `secret`, or joins it if it exists.
    #[must_use]
    pub fn new(secret: impl Into<CompactString>) -> Self {
        Self {
            secret: secret.into(),
            idle_timeout_seconds: None,
            lobby_metadata: None,
            member_metadata: None,
        }
    }
}

impl fmt::Debug for CreateOrJoinLobby {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateOrJoinLobby")
            .field("secret", &"<redacted>")
            .field("idle_timeout_seconds", &self.idle_timeout_seconds)
            .field("lobby_metadata", &self.lobby_metadata)
            .field("member_metadata", &self.member_metadata)
            .finish()
    }
}

/// The body of [Modify Lobby](https://docs.discord.com/developers/resources/lobby#modify-lobby).
///
/// Every field is optional.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ModifyLobby {
    /// Replaces the lobby's metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LobbyMetadata>,
    /// Replaces the members (up to 25): members not in the list are removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<LobbyMemberData>>,
    /// How many seconds an idle lobby lives before it is shut down (5 to 604 800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,
}

/// The body of [Add a Member to a Lobby](https://docs.discord.com/developers/resources/lobby#add-a-member-to-a-lobby)
/// (D-134). The user is in the path; for a member already in the lobby, the fields that are
/// set are updated.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AddLobbyMember {
    /// The member's metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LobbyMetadata>,
    /// The member's flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<LobbyMemberFlags>,
    /// An extra display name (1-80 characters); [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub additional_name: Patch<CompactString>,
}

/// One entry of [Bulk Update Lobby Members](https://docs.discord.com/developers/resources/lobby#bulk-update-lobby-members):
/// a member to add or update, or one to remove (D-134).
///
/// # Examples
///
/// ```
/// use djs_model::{
///     id::UserId,
///     rest::{BulkLobbyMember, LobbyMemberData},
///     util::Patch,
/// };
///
/// let mut ferris = LobbyMemberData::new(UserId::new(7));
/// ferris.additional_name = Patch::Set("Ferris".into());
/// let body = [
///     BulkLobbyMember::upsert(ferris),
///     BulkLobbyMember::remove(UserId::new(8)),
/// ];
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"[{"id":"7","additional_name":"Ferris"},{"id":"8","remove_member":true}]"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct BulkLobbyMember {
    /// The member and the fields to set.
    #[serde(flatten)]
    pub member: LobbyMemberData,
    /// Whether the member is removed instead of added or updated.
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    pub remove_member: bool,
}

impl BulkLobbyMember {
    /// Adds `member`, or updates it if the user is already in the lobby.
    #[must_use]
    pub const fn upsert(member: LobbyMemberData) -> Self {
        Self {
            member,
            remove_member: false,
        }
    }

    /// Removes the user `id` from the lobby.
    #[must_use]
    pub const fn remove(id: UserId) -> Self {
        Self {
            member: LobbyMemberData::new(id),
            remove_member: true,
        }
    }
}

/// The body of [Send Lobby Message](https://docs.discord.com/developers/resources/lobby#send-lobby-message).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SendLobbyMessage {
    /// The text (not empty).
    pub content: CompactString,
    /// Metadata delivered with the message to Social SDK clients (not kept on the linked
    /// channel's copy).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: LobbyMetadata,
    /// Message flags; only those the Social SDK may set are accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,
}

impl SendLobbyMessage {
    /// A message with `content`.
    #[must_use]
    pub fn new(content: impl Into<CompactString>) -> Self {
        Self {
            content: content.into(),
            metadata: BTreeMap::new(),
            flags: None,
        }
    }
}

/// The query of [Get Lobby Messages](https://docs.discord.com/developers/resources/lobby#get-lobby-messages).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetLobbyMessages {
    /// How many of the most recent messages (1-200; Discord's default is 50).
    pub limit: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_bodies() {
        let add = AddLobbyMember {
            flags: Some(LobbyMemberFlags::CAN_LINK_LOBBY),
            additional_name: Patch::Clear,
            ..AddLobbyMember::default()
        };
        assert_eq!(
            serde_json::to_string(&add).unwrap(),
            r#"{"flags":1,"additional_name":null}"#
        );
        let mut join = CreateOrJoinLobby::new("lobby-secret");
        join.member_metadata = Some(LobbyMetadata::new());
        assert_eq!(
            serde_json::to_string(&join).unwrap(),
            r#"{"secret":"lobby-secret","member_metadata":{}}"#
        );
        assert!(!format!("{join:?}").contains("lobby-secret"));
    }
}
