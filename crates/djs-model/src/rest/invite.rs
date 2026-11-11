//! Bodies and queries of the [invite endpoints](https://docs.discord.com/developers/resources/invite).

use serde::Serialize;

use crate::id::{ScheduledEventId, UserId};

/// The query of [Get Invite](https://docs.discord.com/developers/resources/invite#get-invite).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetInvite {
    /// Whether to include the approximate member and presence counts.
    pub with_counts: Option<bool>,
    /// A scheduled event of the guild to include with the invite.
    pub guild_scheduled_event_id: Option<ScheduledEventId>,
}

/// The body of [Bulk-Add Target Users](https://docs.discord.com/developers/resources/invite#bulk-add-target-users)
/// and [Bulk-Delete Target Users](https://docs.discord.com/developers/resources/invite#bulk-delete-target-users).
///
/// # Examples
///
/// ```
/// use djs_model::{id::UserId, rest::BulkTargetUsers};
///
/// let body = BulkTargetUsers::new([UserId::new(7), UserId::new(8)]);
/// assert_eq!(serde_json::to_string(&body)?, r#"{"user_ids":["7","8"]}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct BulkTargetUsers {
    /// The users (max 1000).
    pub user_ids: Vec<UserId>,
}

impl BulkTargetUsers {
    /// Adds or removes `user_ids` (max 1000).
    #[must_use]
    pub fn new(user_ids: impl IntoIterator<Item = UserId>) -> Self {
        Self {
            user_ids: user_ids.into_iter().collect(),
        }
    }
}
