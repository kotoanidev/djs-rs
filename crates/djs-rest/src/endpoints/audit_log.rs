//! The [audit log endpoint](https://docs.discord.com/developers/resources/audit-log): the
//! record of administrative actions in a guild.

use djs_model::{audit_log::AuditLog, id::GuildId, rest::GetGuildAuditLog};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Guild Audit Log](https://docs.discord.com/developers/resources/audit-log#get-guild-audit-log):
    /// returns a page of a guild's audit log entries (50 by default) with the users,
    /// webhooks, threads and other objects they refer to. Needs `VIEW_AUDIT_LOG`.
    ///
    /// The reasons that methods taking a `reason` send (D-086) appear here.
    ///
    /// discord.js counterpart: `Guild#fetchAuditLogs`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{audit_log::AuditLogEvent, rest::GetGuildAuditLog};
    ///
    /// let mut query = GetGuildAuditLog::default();
    /// query.action_type = Some(AuditLogEvent::MemberBanAdd);
    /// query.limit = Some(10);
    /// let log = rest.get_guild_audit_log(guild_id, &query).await?;
    /// for entry in &log.audit_log_entries {
    ///     println!("{:?} banned {:?}: {:?}", entry.user_id, entry.target_id, entry.reason);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50013` Missing Permissions without `VIEW_AUDIT_LOG`.
    pub async fn get_guild_audit_log(
        &self,
        guild_id: GuildId,
        query: &GetGuildAuditLog,
    ) -> Result<AuditLog, Error> {
        self.request(Request::new(Route::GetGuildAuditLog { guild_id }).query(query))
            .await
    }
}
