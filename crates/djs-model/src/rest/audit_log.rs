//! The query of the [audit log endpoint](https://docs.discord.com/developers/resources/audit-log).

use crate::{
    audit_log::AuditLogEvent,
    id::{AuditLogEntryId, UserId},
};

/// The query of [Get Guild Audit Log](https://docs.discord.com/developers/resources/audit-log#get-guild-audit-log).
///
/// Entries come newest first, or oldest first with [`after`](Self::after) (`after` the
/// smallest ID, e.g. `AuditLogEntryId::new(1)`, starts from the oldest).
///
/// discord.js counterpart: `GuildAuditLogsFetchOptions`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetGuildAuditLog {
    /// Only entries of actions by this user.
    pub user_id: Option<UserId>,
    /// Only entries of this kind of action.
    pub action_type: Option<AuditLogEvent>,
    /// Entries older than this one.
    pub before: Option<AuditLogEntryId>,
    /// Entries newer than this one.
    pub after: Option<AuditLogEntryId>,
    /// The page size (1-100, default 50).
    pub limit: Option<u8>,
}
