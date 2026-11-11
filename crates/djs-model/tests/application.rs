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

#[test]
fn entitlement() {
    use djs_model::{
        entitlement::{Entitlement, EntitlementType},
        util::Timestamp,
    };

    let entitlement: Entitlement = round_trip(
        include_str!("fixtures/docs/entitlement_entitlement.json"),
        // Shown in the example but documented nowhere (nor in discord-api-types).
        &["promotion_id", "gift_code_flags", "subscription_id"],
    );
    assert_eq!(entitlement.kind, EntitlementType::ApplicationSubscription);
    assert!(!entitlement.deleted && !entitlement.consumed);
    let during: Timestamp = "2022-10-01T00:00:00+00:00".parse().unwrap();
    let after: Timestamp = "2022-10-14T17:00:18.704163+00:00".parse().unwrap();
    assert!(entitlement.is_active(during));
    assert!(!entitlement.is_active(after));
}

#[test]
fn sku() {
    use djs_model::sku::{Sku, SkuFlags, SkuType};

    let sku: Sku = round_trip(
        include_str!("fixtures/docs/sku_sku.json"),
        // Shown in the example but not in the SKU table.
        &[
            "dependent_sku_id",
            "manifest_labels",
            "access_type",
            "features",
            "release_date",
            "premium",
            "show_age_gate",
        ],
    );
    assert_eq!(sku.kind, SkuType::Subscription);
    assert_eq!(sku.slug, "test-premium");
    assert_eq!(sku.flags, SkuFlags::GUILD_SUBSCRIPTION);
}

#[test]
fn subscription() {
    use djs_model::subscription::{Subscription, SubscriptionStatus};

    let subscription: Subscription = round_trip(
        include_str!("fixtures/docs/subscription_subscription.json"),
        &[],
    );
    assert_eq!(subscription.status, SubscriptionStatus::Active);
    assert_eq!(subscription.sku_ids.len(), 1);
    assert_eq!(subscription.entitlement_ids.len(), 0);
    assert!(subscription.renewal_sku_ids.is_none() && subscription.canceled_at.is_none());
    assert!(subscription.current_period_start < subscription.current_period_end);
    // Nullable fields are written as `null` (D-049).
    let written = serde_json::to_value(&subscription).unwrap();
    assert!(written["canceled_at"].is_null() && written["renewal_sku_ids"].is_null());
    assert!(written.get("country").is_none());
}

#[test]
fn identity_profile() {
    use djs_model::{
        application_identity_profile::{
            ApplicationIdentity, ApplicationIdentityProfile, DynamicField, DynamicFieldType,
            DynamicFieldValue, ProfileData, ProfileMedia,
        },
        util::JsonValue,
    };

    let profile: ApplicationIdentityProfile = round_trip(
        include_str!("fixtures/docs/application_identity_profile.json"),
        &[],
    );
    assert_eq!(profile.username.as_deref(), Some("johndoe123"));
    assert!(profile.metadata.is_none());
    let data = profile.data.unwrap();
    let primary = data.primary.unwrap();
    assert_eq!(primary.playtime_hours, Some(69.41));
    assert_eq!(primary.total_wins, Some(57));
    assert_eq!(primary.rank_name.as_deref(), Some("Silver"));
    let values: Vec<&DynamicFieldValue> = data.dynamic.iter().map(|field| &field.value).collect();
    assert_eq!(
        values,
        [
            &DynamicFieldValue::String("hello".into()),
            &DynamicFieldValue::Number(123.45),
            &DynamicFieldValue::Media(ProfileMedia::new("https://example.com/some-media.png")),
        ]
    );

    // Metadata is arbitrary JSON (D-072); unknown field types keep their name and type.
    let json = r#"{"username":null,"metadata":{"level":12,"tags":["a"]},"data":{"dynamic":[{"type":9,"name":"x","value":true}]}}"#;
    let profile: ApplicationIdentityProfile = serde_json::from_str(json).unwrap();
    let metadata = profile.metadata.as_ref().unwrap();
    assert_eq!(metadata["level"], JsonValue::Unsigned(12));
    let field = &profile.data.as_ref().unwrap().dynamic[0];
    assert_eq!(field.value.kind(), DynamicFieldType::Unknown(9));
    assert_eq!(
        serde_json::to_string(&profile).unwrap(),
        r#"{"username":null,"metadata":{"level":12,"tags":["a"]},"data":{"dynamic":[{"type":9,"name":"x"}]}}"#
    );

    // Building an update.
    let mut update = ProfileData::default();
    update
        .dynamic
        .push(DynamicField::new("mmr", DynamicFieldValue::Number(1520.0)));
    assert_eq!(
        serde_json::to_string(&update).unwrap(),
        r#"{"dynamic":[{"type":2,"name":"mmr","value":1520.0}]}"#
    );
    // A wrongly typed value is an error.
    let error = serde_json::from_str::<DynamicField>(r#"{"type":2,"name":"n","value":"1"}"#)
        .unwrap_err()
        .to_string();
    assert!(error.contains("value"), "{error}");

    let identity: ApplicationIdentity = round_trip(
        r#"{"user_id":"1","provider_type":"steam","provider_id":"x","provider_issued_user_id":"765"}"#,
        &[],
    );
    assert_eq!(identity.provider_type, "steam");
}
