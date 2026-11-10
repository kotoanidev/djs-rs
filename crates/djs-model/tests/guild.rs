//! Deserializes the guild examples of the Discord API documentation (`tests/fixtures/docs/`)
//! and checks that serializing them again loses no field.

use djs_model::{
    gateway::Status,
    guild::{
        AgeRestrictionLevel, Ban, DefaultMessageNotificationLevel, ExplicitContentFilterLevel,
        Feature, Guild, IncidentsData, MfaLevel, Onboarding, OnboardingMode, PremiumTier, Preview,
        PromptType, VerificationLevel, WelcomeScreen, Widget, WidgetSettings,
    },
    util::Locale,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Parses a fixture into `T` and checks that every field of the JSON survives a round trip
/// (ignoring fields that `T` does not model, which are listed in `unmodelled`).
fn round_trip<T: Serialize + DeserializeOwned>(json: &str, unmodelled: &[&str]) -> T {
    let value: T = serde_json::from_str(json).unwrap();
    let mut expected: Value = serde_json::from_str(json).unwrap();
    if let Some(object) = expected.as_object_mut() {
        for field in unmodelled {
            object.remove(*field);
        }
    }
    let actual = serde_json::to_value(&value).unwrap();
    assert_subset(&expected, &actual, "$");
    value
}

/// Asserts that everything in `expected` is present and equal in `actual`.
fn assert_subset(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                let Some(actual) = actual.get(key) else {
                    // Absent and null/false/empty collapse into the same value (D-049), so an
                    // optional field may be skipped when serializing.
                    let collapsible = value.is_null()
                        || *value == Value::Bool(false)
                        || value.as_array().is_some_and(Vec::is_empty);
                    assert!(collapsible, "{path}.{key} was lost");
                    continue;
                };
                assert_subset(value, actual, &format!("{path}.{key}"));
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            assert_eq!(expected.len(), actual.len(), "{path} length");
            for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
                assert_subset(expected, actual, &format!("{path}[{index}]"));
            }
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}

#[test]
fn guild() {
    let guild: Guild = round_trip(include_str!("fixtures/docs/guild.json"), &[]);
    assert_eq!(guild.id.get(), 197_038_439_483_310_086);
    assert_eq!(guild.name, "Discord Testers");
    assert!(guild.features.contains(&Feature::Community));
    // `MORE_EMOJI` is not a documented feature any more; it is kept as unknown.
    assert!(
        guild
            .features
            .iter()
            .any(|feature| feature.as_str() == "MORE_EMOJI" && feature.is_unknown())
    );
    assert_eq!(guild.afk_timeout, 300);
    assert_eq!(guild.verification_level, VerificationLevel::High);
    assert_eq!(
        guild.default_message_notifications,
        DefaultMessageNotificationLevel::OnlyMentions
    );
    assert_eq!(
        guild.explicit_content_filter,
        ExplicitContentFilterLevel::AllMembers
    );
    assert_eq!(guild.mfa_level, MfaLevel::Elevated);
    assert_eq!(guild.premium_tier, PremiumTier::Tier3);
    assert_eq!(guild.preferred_locale, Locale::EnglishUs);
    assert_eq!(guild.nsfw_level, AgeRestrictionLevel::Default);
}

#[test]
fn preview() {
    let preview: Preview = round_trip(include_str!("fixtures/docs/guild_preview.json"), &[]);
    assert_eq!(preview.name, "Discord Testers");
    assert!(preview.approximate_member_count > 0);
}

#[test]
fn widget() {
    let settings: WidgetSettings = round_trip(
        include_str!("fixtures/docs/guild_widget_settings.json"),
        &[],
    );
    assert!(settings.enabled);
    let widget: Widget = round_trip(include_str!("fixtures/docs/guild_widget.json"), &[]);
    assert_eq!(widget.channels.len(), 2);
    assert_eq!(widget.members[0].status, Status::Online);
}

#[test]
fn ban() {
    let ban: Ban = round_trip(include_str!("fixtures/docs/ban.json"), &[]);
    assert_eq!(ban.reason.as_deref(), Some("mentioning b1nzy"));
    assert_eq!(ban.user.discriminator, 9999);
}

#[test]
fn welcome_screen() {
    let screen: WelcomeScreen = round_trip(include_str!("fixtures/docs/welcome_screen.json"), &[]);
    assert_eq!(screen.welcome_channels.len(), 5);
}

#[test]
fn onboarding() {
    let onboarding: Onboarding =
        round_trip(include_str!("fixtures/docs/guild_onboarding.json"), &[]);
    assert_eq!(onboarding.mode, OnboardingMode::Default);
    let prompt = &onboarding.prompts[0];
    assert_eq!(prompt.kind, PromptType::MultipleChoice);
    assert_eq!(prompt.options.len(), 2);
}

#[test]
fn incidents_data() {
    let incidents: IncidentsData =
        round_trip(include_str!("fixtures/docs/incidents_data.json"), &[]);
    assert!(incidents.invites_disabled_until.is_some());
}
