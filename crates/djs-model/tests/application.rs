//! Applications and teams against the docs' examples.
//!
//! The docs' application example predates the team member `role` field (and still shows the
//! deprecated `permissions: ["*"]`), so the fixture adds `"role": "admin"`. The Activity
//! instance example has a trailing comma, which the fixture drops.

use djs_model::{
    application::{
        ActivityInstance, ActivityLocationKind, Application, ApplicationFlags, EventWebhookStatus,
        IntegrationTypesConfig, PartialApplication,
    },
    oauth2::Scope,
    permissions::Permissions,
    team::{MemberRole, MembershipState},
};

mod common;

use common::round_trip;

#[test]
fn application() {
    let application: Application = round_trip(
        include_str!("fixtures/docs/application_application.json"),
        // `User` has no account `flags` (only `CurrentUser`, D-052); the team member
        // `permissions` field is deprecated in favour of `role`.
        &["owner.flags", "team.members[].permissions"],
    );
    assert_eq!(application.name, "Baba O-Riley");
    assert!(application.bot_public);
    assert!(!application.bot_require_code_grant);
    assert_eq!(
        application.flags,
        Some(ApplicationFlags::GATEWAY_PRESENCE_LIMITED)
    );
    assert_eq!(application.flags_new, application.flags);
    assert_eq!(
        application.event_webhooks_status,
        Some(EventWebhookStatus::Disabled)
    );
    assert_eq!(application.owner.as_ref().unwrap().discriminator, 1738);

    let config = application.integration_types_config.as_ref().unwrap();
    let guild = config.guild_install.as_ref().unwrap();
    let guild = guild.oauth2_install_params.as_ref().unwrap();
    assert_eq!(guild.scopes, [Scope::ApplicationsCommands, Scope::Bot]);
    assert_eq!(guild.permissions, Permissions::SEND_MESSAGES);
    let user = config.user_install.as_ref().unwrap();
    let user = user.oauth2_install_params.as_ref().unwrap();
    assert_eq!(user.permissions, Permissions::empty());

    let team = application.team.as_ref().unwrap();
    assert_eq!(team.name, "A team");
    assert_eq!(team.members.len(), 1);
    let member = &team.members[0];
    assert_eq!(member.membership_state, MembershipState::Accepted);
    assert_eq!(member.role, MemberRole::Admin);
    assert_eq!(member.user.id, team.owner_user_id);
}

#[test]
fn flags_new_is_a_decimal_string() {
    let json = r#"{"id":"1","flags":8192,"flags_new":"8392704"}"#;
    let application: PartialApplication = serde_json::from_str(json).unwrap();
    assert_eq!(
        application.flags,
        Some(ApplicationFlags::GATEWAY_PRESENCE_LIMITED)
    );

    let mut full: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/docs/application_application.json")).unwrap();
    // Unknown bits are kept and written back as a string.
    full["flags_new"] = "9223372036854784000".into();
    let application: Application = serde_json::from_value(full.clone()).unwrap();
    assert_eq!(
        application.flags_new.unwrap().bits(),
        9_223_372_036_854_784_000
    );
    let written = serde_json::to_value(&application).unwrap();
    assert_eq!(written["flags_new"], "9223372036854784000");

    full["flags_new"] = 8192.into();
    assert!(serde_json::from_value::<Application>(full.clone()).is_err());
    full["flags_new"] = "x".into();
    assert!(serde_json::from_value::<Application>(full).is_err());
}

#[test]
fn integration_types_config_keys() {
    let config: IntegrationTypesConfig = serde_json::from_str(r#"{"1":{}}"#).unwrap();
    assert!(config.guild_install.is_none());
    assert!(config.user_install.unwrap().oauth2_install_params.is_none());
}

#[test]
fn activity_instance() {
    let instance: ActivityInstance = round_trip(
        include_str!("fixtures/docs/application_activity_instance.json"),
        &[],
    );
    assert_eq!(instance.location.kind, ActivityLocationKind::GuildChannel);
    assert_eq!(instance.users.len(), 1);
    assert_eq!(
        instance.location.guild_id.map(djs_model::id::Id::get),
        Some(912_952_092_627_435_520)
    );
}

#[test]
fn invite_target_application() {
    let mut invite: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/docs/invite_invite_object.json")).unwrap();
    invite["target_type"] = 2.into();
    invite["target_application"] = serde_json::json!({
        "id": "880218394199220334",
        "name": "Watch Together",
        "icon": "fe2b7fa334817b0346d57416ad75e93b",
        "description": "Create and watch a playlist of YouTube videos with your friends.",
        "flags": 0
    });
    let json = invite.to_string();
    let invite: djs_model::invite::Invite = round_trip(&json, &[]);
    let target = invite.target_application.unwrap();
    assert_eq!(target.name.as_deref(), Some("Watch Together"));
    assert_eq!(target.flags, Some(ApplicationFlags::empty()));
}
