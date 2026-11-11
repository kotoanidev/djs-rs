//! Bodies, queries and responses of the [user endpoints](https://docs.discord.com/developers/resources/user).
//!
//! Update Current User Application Role Connection takes the model's
//! [`ApplicationRoleConnection`](crate::user::ApplicationRoleConnection) as its body.

use core::fmt;
use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    guild::Feature,
    id::{GuildId, UserId},
    permissions::Permissions,
    util::{ImageData, ImageHash, Patch},
};

/// The body of [Modify Current User](https://docs.discord.com/developers/resources/user#modify-current-user).
///
/// discord.js counterpart: `ClientUser#edit`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyCurrentUser {
    /// The username.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<CompactString>,
    /// The avatar.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub avatar: Patch<ImageData>,
    /// The profile banner.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub banner: Patch<ImageData>,
}

/// The query of [Get Current User Guilds](https://docs.discord.com/developers/resources/user#get-current-user-guilds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetCurrentUserGuilds {
    /// Guilds with a lower ID.
    pub before: Option<GuildId>,
    /// Guilds with a higher ID.
    pub after: Option<GuildId>,
    /// The page size (1-200, the default).
    pub limit: Option<u8>,
    /// Only the guilds of this bucket (`0` to `max_concurrency - 1`); required for bots
    /// with [large bot sharding](https://docs.discord.com/developers/events/gateway#sharding-for-large-bots).
    pub shard: Option<u16>,
    /// Whether to include the approximate member and presence counts.
    pub with_counts: Option<bool>,
}

/// A guild of the current user, as returned by
/// [Get Current User Guilds](https://docs.discord.com/developers/resources/user#get-current-user-guilds).
///
/// discord.js counterpart: `OAuth2Guild`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentUserGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's name.
    pub name: CompactString,
    /// The [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The [banner hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default)]
    pub banner: Option<ImageHash>,
    /// Whether the current user owns the guild.
    pub owner: bool,
    /// The current user's permissions in the guild.
    pub permissions: Permissions,
    /// The guild's features.
    pub features: Vec<Feature>,
    /// The approximate number of members (with `with_counts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_member_count: Option<u32>,
    /// The approximate number of online members (with `with_counts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_presence_count: Option<u32>,
}

/// The body of [Create DM](https://docs.discord.com/developers/resources/user#create-dm).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateDm {
    /// The user to open a DM with.
    pub recipient_id: UserId,
}

impl CreateDm {
    /// Opens a DM with `recipient_id`.
    #[must_use]
    pub const fn new(recipient_id: UserId) -> Self {
        Self { recipient_id }
    }
}

/// The body of [Create Group DM](https://docs.discord.com/developers/resources/user#create-group-dm).
///
/// The access tokens are redacted in the `Debug` output.
#[derive(Clone, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateGroupDm {
    /// `OAuth2` access tokens of the users, who granted the `gdm.join` scope.
    pub access_tokens: Vec<String>,
    /// The users' nicknames in the group DM.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub nicks: BTreeMap<UserId, CompactString>,
}

impl CreateGroupDm {
    /// A group DM with the users whose access tokens are `access_tokens`.
    #[must_use]
    pub fn new(access_tokens: Vec<String>) -> Self {
        Self {
            access_tokens,
            nicks: BTreeMap::new(),
        }
    }
}

impl fmt::Debug for CreateGroupDm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateGroupDm")
            .field(
                "access_tokens",
                &format_args!("<{} redacted>", self.access_tokens.len()),
            )
            .field("nicks", &self.nicks)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bodies() {
        let modify = ModifyCurrentUser {
            username: Some("ferris".into()),
            avatar: Patch::Clear,
            ..ModifyCurrentUser::default()
        };
        assert_eq!(
            serde_json::to_string(&modify).unwrap(),
            r#"{"username":"ferris","avatar":null}"#
        );

        let mut group = CreateGroupDm::new(vec!["super-secret".into()]);
        group.nicks.insert(UserId::new(7), "Ferris".into());
        assert_eq!(
            serde_json::to_string(&group).unwrap(),
            r#"{"access_tokens":["super-secret"],"nicks":{"7":"Ferris"}}"#
        );
        let debug = format!("{group:?}");
        assert!(!debug.contains("super-secret"), "{debug}");
        assert!(debug.contains("<1 redacted>"), "{debug}");
    }

    #[test]
    fn current_user_guild() {
        let guild: CurrentUserGuild = serde_json::from_str(
            r#"{
                "id": "80351110224678912", "name": "1337 Krew",
                "icon": "8342729096ea3675442027381ff50dfe",
                "banner": "bb42bdc37653b7cf58c4c8cc622e76cb", "owner": true,
                "permissions": "36953089", "features": ["COMMUNITY", "NEWS"],
                "approximate_member_count": 3268, "approximate_presence_count": 784
            }"#,
        )
        .unwrap();
        assert!(guild.owner);
        assert!(
            guild
                .permissions
                .contains(Permissions::CREATE_INSTANT_INVITE)
        );
        assert_eq!(guild.approximate_member_count, Some(3268));
    }
}
