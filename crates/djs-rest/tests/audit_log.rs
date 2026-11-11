//! The audit log endpoint against the mock API: path, query and the decoded entries.

use djs_model::{
    audit_log::AuditLogEvent,
    id::{AuditLogEntryId, GuildId, UserId},
    rest::GetGuildAuditLog,
};

mod common;

use common::{Canned, mock, seen};

const LOG: &str = r#"{"application_commands": [], "auto_moderation_rules": [],
    "guild_scheduled_events": [], "integrations": [], "threads": [], "webhooks": [],
    "users": [{"id": "7", "username": "ferris", "discriminator": "0", "global_name": null,
               "avatar": null}],
    "audit_log_entries": [{"id": "1290000000000000001", "action_type": 22,
        "target_id": "8", "user_id": "7", "reason": "spam", "changes": []}]}"#;

const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);

#[tokio::test]
async fn get_guild_audit_log() {
    let (rest, state) = mock(vec![Canned::json(200, LOG)]).await;
    let mut query = GetGuildAuditLog::default();
    query.user_id = Some(UserId::new(7));
    query.action_type = Some(AuditLogEvent::MemberBanAdd);
    query.after = Some(AuditLogEntryId::new(1));
    query.limit = Some(100);
    let log = rest.get_guild_audit_log(GUILD, &query).await.unwrap();
    let entry = &log.audit_log_entries[0];
    assert_eq!(entry.action_type, AuditLogEvent::MemberBanAdd);
    assert_eq!(entry.reason.as_deref(), Some("spam"));
    assert_eq!(log.users[0].username, "ferris");

    let request = &seen(&state)[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, format!("/api/v10/guilds/{GUILD}/audit-logs"));
    assert_eq!(request.query, "user_id=7&action_type=22&after=1&limit=100");
}
