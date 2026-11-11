//! The Auto Moderation endpoints against the mock API: method, path, body and audit log reason
//! of every request.

use djs_model::{
    auto_moderation::{Action, ActionType, TriggerType},
    id::{AutoModerationRuleId, ChannelId, GuildId, RoleId},
    rest::{CreateAutoModerationRule, ModifyAutoModerationRule},
};

mod common;

use common::{Canned, mock, seen};

const RULE: &str = include_str!("../../djs-model/tests/fixtures/docs/auto_moderation_rule.json");

const GUILD: GuildId = GuildId::new(613_425_648_685_547_541);
const ID: AutoModerationRuleId = AutoModerationRuleId::new(969_707_018_069_872_670);

/// `(method, path after /api/v10, body, audit log reason)`.
type Expected = (&'static str, String, &'static str, Option<&'static str>);

#[tokio::test]
async fn auto_moderation_endpoints() {
    let rule = || Canned::json(200, RULE);
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{RULE}]")),
        rule(),
        rule(),
        rule(),
        Canned::json(204, ""),
    ])
    .await;

    let rules = rest
        .list_auto_moderation_rules_for_guild(GUILD)
        .await
        .unwrap();
    assert_eq!(rules[0].trigger_type, TriggerType::Keyword);
    let fetched = rest.get_auto_moderation_rule(GUILD, ID).await.unwrap();
    assert_eq!(fetched.actions[2].kind, ActionType::Timeout);

    let mut create = CreateAutoModerationRule::keyword(
        "Keyword Filter 1",
        ["cat*", "*dog"],
        [
            Action::block_message_with("No pets"),
            Action::send_alert_message(ChannelId::new(5)),
            Action::timeout(60),
        ],
    );
    create.trigger_metadata.regex_patterns = vec!["(b|c)at".into()];
    create.enabled = Some(true);
    create.exempt_roles = vec![RoleId::new(3)];
    let created = rest
        .create_auto_moderation_rule(GUILD, &create, Some("pets"))
        .await
        .unwrap();
    assert_eq!(created.id, ID);

    let mut modify = ModifyAutoModerationRule::default();
    modify.name = Some("Pets".into());
    modify.trigger_metadata = Some(created.trigger_metadata);
    modify.exempt_channels = Some(Vec::new());
    rest.modify_auto_moderation_rule(GUILD, ID, &modify, Some("rename"))
        .await
        .unwrap();
    rest.delete_auto_moderation_rule(GUILD, ID, Some("unused"))
        .await
        .unwrap();

    let list = format!("/guilds/{GUILD}/auto-moderation/rules");
    let one = format!("{list}/{ID}");
    let expected: Vec<Expected> = vec![
        ("GET", list.clone(), "", None),
        ("GET", one.clone(), "", None),
        (
            "POST",
            list,
            r#"{"name":"Keyword Filter 1","event_type":1,"trigger_type":1,"trigger_metadata":{"keyword_filter":["cat*","*dog"],"regex_patterns":["(b|c)at"]},"actions":[{"type":1,"metadata":{"custom_message":"No pets"}},{"type":2,"metadata":{"channel_id":"5"}},{"type":3,"metadata":{"duration_seconds":60}}],"enabled":true,"exempt_roles":["3"]}"#,
            Some("pets"),
        ),
        (
            "PATCH",
            one.clone(),
            r#"{"name":"Pets","trigger_metadata":{"keyword_filter":["cat*","*dog","*ana*","i like c++"],"regex_patterns":["(b|c)at","^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$"]},"exempt_channels":[]}"#,
            Some("rename"),
        ),
        ("DELETE", one, "", Some("unused")),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, body, reason)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, "", "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
