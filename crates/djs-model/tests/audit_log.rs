//! Audit logs: a synthetic audit log built from the docs' tables (event types, optional entry
//! info, change exceptions) around the docs' example objects (thread, webhook, auto moderation
//! rule, command, and the audit log page's partial integration).

use djs_model::{
    application_command::{CommandPermission, CommandPermissionType},
    audit_log::{AuditLog, AuditLogChange, AuditLogEntry, AuditLogEvent, AuditLogValue},
    auto_moderation::TriggerType,
    channel::{ChannelType, Overwrite, OverwriteType},
    guild::IntegrationType,
    id::{ApplicationId, ChannelId, GenericId, RoleId},
    permissions::Permissions,
};
use serde::Deserialize;
use serde_json::{Value, json};

mod common;

use common::round_trip;

fn fixture(json: &str) -> Value {
    serde_json::from_str(json).unwrap()
}

const USER: &str = "1300000000000000006";
const ROLE: &str = "1300000000000000030";
const CHANNEL: &str = "1300000000000000004";
const APPLICATION: &str = "1300000000000000002";

fn entries() -> Value {
    json!([
        { "id": "1300000000000000101", "action_type": 25, "target_id": "1300000000000000005",
          "user_id": USER, "reason": "Promotion",
          "changes": [{ "key": "$add", "new_value": [{ "id": ROLE, "name": "Tester" }] }],
          "options": { "integration_type": "discord" } },
        { "id": "1300000000000000102", "action_type": 13, "target_id": CHANNEL, "user_id": USER,
          "changes": [
              { "key": "id", "new_value": ROLE },
              { "key": "type", "new_value": 0 },
              { "key": "allow", "new_value": "1024" },
              { "key": "deny", "new_value": "0" }
          ],
          "options": { "id": ROLE, "type": "0", "role_name": "Tester" } },
        { "id": "1300000000000000103", "action_type": 72, "target_id": "1300000000000000005",
          "user_id": USER, "options": { "channel_id": CHANNEL, "count": "3" } },
        { "id": "1300000000000000104", "action_type": 21, "target_id": null, "user_id": USER,
          "options": { "delete_member_days": "7", "members_removed": "12" } },
        { "id": "1300000000000000105", "action_type": 11, "target_id": CHANNEL, "user_id": USER,
          "changes": [
              { "key": "nsfw", "old_value": false, "new_value": true },
              { "key": "rate_limit_per_user", "old_value": 0, "new_value": 10 },
              { "key": "topic", "old_value": "Old topic" },
              { "key": "type", "new_value": 5 },
              { "key": "position", "old_value": -1, "new_value": 2.5 },
              { "key": "permission_overwrites", "new_value": [
                  { "id": ROLE, "type": 0, "allow": "1024", "deny": "0" }
              ] }
          ] },
        { "id": "1300000000000000106", "action_type": 121, "target_id": APPLICATION,
          "user_id": USER,
          "changes": [{ "key": ROLE,
                        "old_value": { "id": ROLE, "type": 1, "permission": false },
                        "new_value": { "id": ROLE, "type": 1, "permission": true } }],
          "options": { "application_id": APPLICATION } },
        { "id": "1300000000000000107", "action_type": 143, "target_id": "1300000000000000005",
          "user_id": "1300000000000000005",
          "options": { "auto_moderation_rule_name": "No spam",
                       "auto_moderation_rule_trigger_type": "3", "channel_id": CHANNEL } },
        { "id": "1300000000000000108", "action_type": 192, "target_id": "1300000000000000007",
          "user_id": USER,
          "options": { "channel_id": "1300000000000000007", "status": "Coding" } },
        { "id": "1300000000000000109", "action_type": 50, "target_id": "1300000000000000008",
          "user_id": USER,
          "changes": [{ "key": "avatar_hash", "new_value": "a_0123456789abcdef" },
                      { "key": "name", "new_value": "Captain Hook" }] },
        { "id": "1300000000000000110", "action_type": 250, "target_id": null, "user_id": null }
    ])
}

fn audit_log() -> Value {
    let mut command = fixture(include_str!(
        "fixtures/docs/application_command_examples.json"
    ))[0]["command"]
        .clone();
    command["id"] = "1300000000000000010".into();
    command["application_id"] = APPLICATION.into();
    command["version"] = "1300000000000000011".into();
    json!({
        "application_commands": [command],
        "audit_log_entries": entries(),
        "auto_moderation_rules": [fixture(include_str!("fixtures/docs/auto_moderation_rule.json"))],
        "guild_scheduled_events": [],
        "integrations": [fixture(include_str!("fixtures/docs/audit_log_partial_integration.json"))],
        "threads": [fixture(include_str!("fixtures/docs/channel_thread_channel.json"))],
        "users": [{ "id": USER, "username": "mixo", "discriminator": "0", "global_name": null,
                    "avatar": null }],
        "webhooks": [fixture(include_str!("fixtures/docs/webhook_incoming_webhook.json"))]
    })
}

fn entry(log: &AuditLog, event: AuditLogEvent) -> &AuditLogEntry {
    log.audit_log_entries
        .iter()
        .find(|entry| entry.action_type == event)
        .unwrap_or_else(|| panic!("no {event:?} entry"))
}

fn change<'a>(entry: &'a AuditLogEntry, key: &str) -> &'a AuditLogChange {
    entry
        .changes
        .iter()
        .find(|change| change.key == key)
        .unwrap_or_else(|| panic!("no change {key}"))
}

#[test]
fn audit_log_round_trips() {
    let log: AuditLog = round_trip(&audit_log().to_string(), &[]);
    assert_eq!(log.audit_log_entries.len(), 10);
    assert_eq!(log.integrations[0].kind, Some(IntegrationType::Twitch));
    assert_eq!(log.threads.len(), 1);
    assert_eq!(log.webhooks[0].name.as_deref(), Some("test webhook"));
    assert_eq!(log.auto_moderation_rules.len(), 1);
    assert_eq!(log.application_commands.len(), 1);
    let unknown = log.audit_log_entries.last().unwrap();
    assert_eq!(unknown.action_type, AuditLogEvent::Unknown(250));
    assert!(unknown.target_id.is_none() && unknown.user_id.is_none());
}

#[test]
fn entry_options() {
    let log: AuditLog = serde_json::from_value(audit_log()).unwrap();

    let roles = entry(&log, AuditLogEvent::MemberRoleUpdate);
    assert_eq!(roles.reason.as_deref(), Some("Promotion"));
    let options = roles.options.as_deref().unwrap();
    assert_eq!(options.integration_type, Some(IntegrationType::Discord));

    let overwrite = entry(&log, AuditLogEvent::ChannelOverwriteCreate);
    let options = overwrite.options.as_deref().unwrap();
    assert_eq!(options.kind, Some(OverwriteType::Role));
    assert_eq!(options.id, Some(ROLE.parse::<GenericId>().unwrap()));
    assert_eq!(options.role_name.as_deref(), Some("Tester"));

    let delete = entry(&log, AuditLogEvent::MessageDelete);
    let options = delete.options.as_deref().unwrap();
    assert_eq!(
        (options.count, options.channel_id),
        (Some(3), CHANNEL.parse().ok())
    );

    let prune = entry(&log, AuditLogEvent::MemberPrune);
    let options = prune.options.as_deref().unwrap();
    assert_eq!(
        (options.delete_member_days, options.members_removed),
        (Some(7), Some(12))
    );

    let automod = entry(&log, AuditLogEvent::AutoModerationBlockMessage);
    let options = automod.options.as_deref().unwrap();
    assert_eq!(
        options.auto_moderation_rule_trigger_type,
        Some(TriggerType::Spam)
    );
    assert_eq!(
        options.auto_moderation_rule_name.as_deref(),
        Some("No spam")
    );

    let status = entry(&log, AuditLogEvent::VoiceChannelStatusCreate);
    assert_eq!(
        status.options.as_deref().unwrap().status.as_deref(),
        Some("Coding")
    );

    let permissions = entry(&log, AuditLogEvent::ApplicationCommandPermissionUpdate);
    assert_eq!(
        permissions.options.as_deref().unwrap().application_id,
        Some(ApplicationId::new(1_300_000_000_000_000_002))
    );

    // Counts and types are written back as strings.
    let written = serde_json::to_value(&prune.options).unwrap();
    assert_eq!(
        written,
        json!({ "delete_member_days": "7", "members_removed": "12" })
    );
    let written = serde_json::to_value(&overwrite.options).unwrap();
    assert_eq!(written["type"], "0");
}

/// The partial roles of `$add` / `$remove` changes.
#[derive(Deserialize)]
struct PartialRole<'a> {
    id: RoleId,
    name: &'a str,
}

#[test]
fn change_values_convert_to_field_types() {
    let log: AuditLog = serde_json::from_value(audit_log()).unwrap();

    let update = entry(&log, AuditLogEvent::ChannelUpdate);
    let nsfw = change(update, "nsfw");
    assert_eq!(nsfw.old_value_as::<bool>(), Ok(Some(false)));
    assert_eq!(nsfw.new_value_as::<bool>(), Ok(Some(true)));
    assert_eq!(
        change(update, "rate_limit_per_user").new_value_as::<u16>(),
        Ok(Some(10))
    );
    // A missing new value means the field was reset.
    let topic = change(update, "topic");
    assert_eq!(topic.old_value_as::<&str>(), Ok(Some("Old topic")));
    assert_eq!(topic.new_value_as::<String>(), Ok(None));
    assert_eq!(
        change(update, "type").new_value_as::<ChannelType>(),
        Ok(Some(ChannelType::GuildAnnouncement))
    );
    let position = change(update, "position");
    assert_eq!(position.old_value, Some(AuditLogValue::Signed(-1)));
    assert_eq!(position.new_value, Some(AuditLogValue::Float(2.5)));
    let overwrites = change(update, "permission_overwrites")
        .new_value_as::<Vec<Overwrite>>()
        .unwrap()
        .unwrap();
    assert_eq!(overwrites[0].allow, Permissions::VIEW_CHANNEL);
    assert_eq!(
        overwrites[0].role_id(),
        Some(RoleId::new(1_300_000_000_000_000_030))
    );

    // `$add` holds partial roles.
    let roles = entry(&log, AuditLogEvent::MemberRoleUpdate);
    let added = change(roles, "$add")
        .new_value_as::<Vec<PartialRole<'_>>>()
        .unwrap()
        .unwrap();
    assert_eq!(
        (added[0].id, added[0].name),
        (RoleId::new(1_300_000_000_000_000_030), "Tester")
    );

    // Command permission changes are keyed by the target ID.
    let permissions = entry(&log, AuditLogEvent::ApplicationCommandPermissionUpdate);
    let permission = change(permissions, ROLE)
        .new_value_as::<CommandPermission>()
        .unwrap()
        .unwrap();
    assert_eq!(permission.kind, CommandPermissionType::Role);
    assert!(permission.permission);

    // Webhooks use `avatar_hash`.
    let webhook = entry(&log, AuditLogEvent::WebhookCreate);
    assert_eq!(
        change(webhook, "avatar_hash")
            .new_value
            .as_ref()
            .and_then(AuditLogValue::as_str),
        Some("a_0123456789abcdef")
    );

    // Converting to the wrong type is an error, not a panic.
    let error = nsfw.new_value_as::<ChannelId>().unwrap_err();
    assert!(error.to_string().contains("boolean"), "{error}");
    let error = change(update, "permission_overwrites")
        .new_value_as::<Vec<u8>>()
        .unwrap_err();
    assert!(error.to_string().contains("map"), "{error}");
}

#[test]
fn values_round_trip_every_json_shape() {
    let json = r#"[null,true,0,18446744073709551615,-9223372036854775808,0.5,"text",[1,[2]],{"a":{"b":null}}]"#;
    let values: Vec<AuditLogValue> = serde_json::from_str(json).unwrap();
    assert_eq!(values[3], AuditLogValue::Unsigned(u64::MAX));
    assert_eq!(values[4], AuditLogValue::Signed(i64::MIN));
    assert_eq!(serde_json::to_string(&values).unwrap(), json);
    // Optional fields see `null` as `None`.
    assert_eq!(values[0].to_type::<Option<u8>>(), Ok(None));
    assert_eq!(values[2].to_type::<Option<u8>>(), Ok(Some(0)));
}
