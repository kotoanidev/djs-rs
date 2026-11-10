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
mod common;

use common::round_trip;

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

#[test]
fn auto_moderation_rule() {
    use djs_model::auto_moderation::{ActionType, EventType, Rule, TriggerType};

    let rule: Rule = round_trip(include_str!("fixtures/docs/auto_moderation_rule.json"), &[]);
    assert_eq!(rule.trigger_type, TriggerType::Keyword);
    assert_eq!(rule.event_type, EventType::MessageSend);
    assert_eq!(rule.trigger_metadata.keyword_filter.len(), 4);
    assert_eq!(rule.trigger_metadata.regex_patterns[0], "(b|c)at");
    let kinds: Vec<_> = rule.actions.iter().map(|action| action.kind).collect();
    assert_eq!(
        kinds,
        [
            ActionType::BlockMessage,
            ActionType::SendAlertMessage,
            ActionType::Timeout
        ]
    );
    assert_eq!(
        rule.actions[2].metadata.as_ref().unwrap().duration_seconds,
        Some(60)
    );
}

#[test]
fn scheduled_event_with_recurrence() {
    use djs_model::guild_scheduled_event::{
        EntityType, GuildScheduledEvent, PrivacyLevel, RecurrenceFrequency, RecurrenceWeekday,
        Status as EventStatus,
    };

    // Monthly on the fourth Wednesday, as in the docs' recurrence examples.
    let json = r#"{
        "id": "1300000000000000001", "guild_id": "1300000000000000002",
        "channel_id": null, "creator_id": "1300000000000000003",
        "name": "Book club", "description": "Chapter 4",
        "scheduled_start_time": "2026-11-25T18:00:00.000000+00:00",
        "scheduled_end_time": "2026-11-25T19:00:00.000000+00:00",
        "privacy_level": 2, "status": 1, "entity_type": 3, "entity_id": null,
        "entity_metadata": {"location": "Library"}, "user_count": 7, "image": null,
        "recurrence_rule": {
            "start": "2026-11-25T18:00:00.000000+00:00", "end": null, "frequency": 1,
            "interval": 1, "by_weekday": null, "by_n_weekday": [{"n": 4, "day": 2}],
            "by_month": null, "by_month_day": null, "by_year_day": null, "count": null
        }
    }"#;
    let event: GuildScheduledEvent = round_trip(json, &[]);
    assert_eq!(event.entity_type, EntityType::External);
    assert_eq!(event.privacy_level, PrivacyLevel::GuildOnly);
    assert_eq!(event.status, EventStatus::Scheduled);
    assert_eq!(
        event.entity_metadata.unwrap().location.as_deref(),
        Some("Library")
    );
    let rule = event.recurrence_rule.unwrap();
    assert_eq!(rule.frequency, RecurrenceFrequency::Monthly);
    let n_weekday = rule.by_n_weekday.unwrap()[0];
    assert_eq!(
        (n_weekday.n, n_weekday.day),
        (4, RecurrenceWeekday::Wednesday)
    );
}
