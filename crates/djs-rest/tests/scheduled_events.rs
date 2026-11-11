//! The guild scheduled event endpoints against the mock API: method, path, query, body and
//! audit log reason of every request.

use djs_model::{
    guild_scheduled_event::{EntityMetadata, RecurrenceWeekday, Status},
    id::{ChannelId, GuildId, ScheduledEventId, UserId},
    rest::{
        CreateGuildScheduledEvent, GetGuildScheduledEvent, GetGuildScheduledEventUsers,
        ListScheduledEventsForGuild, ModifyGuildScheduledEvent, RecurrenceRuleData,
    },
    util::{Patch, Timestamp},
};

mod common;

use common::{Canned, mock, seen};

const EVENT: &str = r#"{"id": "1280000000000000001", "guild_id": "1100000000000000000",
    "channel_id": "1200000000000000005", "creator_id": "7", "name": "Office hours",
    "description": null, "scheduled_start_time": "2026-11-19T18:00:00+00:00",
    "scheduled_end_time": null, "privacy_level": 2, "status": 1, "entity_type": 1,
    "entity_id": null, "entity_metadata": null, "user_count": 12, "image": null,
    "recurrence_rule": {"start": "2026-11-19T18:00:00+00:00", "end": null, "frequency": 2,
        "interval": 1, "by_weekday": [3], "by_n_weekday": null, "by_month": null,
        "by_month_day": null, "by_year_day": null, "count": null}}"#;

const USERS: &str = r#"[{"guild_scheduled_event_id": "1280000000000000001",
    "user": {"id": "7", "username": "ferris", "discriminator": "0", "global_name": null,
             "avatar": null},
    "member": {"roles": [], "joined_at": "2026-01-01T00:00:00+00:00", "deaf": false,
               "mute": false, "flags": 0}}]"#;

const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);
const STAGE: ChannelId = ChannelId::new(1_200_000_000_000_000_005);
const ID: ScheduledEventId = ScheduledEventId::new(1_280_000_000_000_000_001);

/// `(method, path after /api/v10, query, body, audit log reason)`.
type Expected = (
    &'static str,
    String,
    &'static str,
    &'static str,
    Option<&'static str>,
);

#[tokio::test]
async fn scheduled_event_endpoints() {
    let event = || Canned::json(200, EVENT);
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{EVENT}]")),
        event(),
        event(),
        event(),
        Canned::json(204, ""),
        Canned::json(200, USERS),
    ])
    .await;

    let mut list = ListScheduledEventsForGuild::default();
    list.with_user_count = Some(true);
    let events = rest
        .list_scheduled_events_for_guild(GUILD, &list)
        .await
        .unwrap();
    assert_eq!(events[0].user_count, Some(12));

    let start = Timestamp::parse("2026-11-19T18:00:00Z").unwrap();
    let mut create = CreateGuildScheduledEvent::stage(STAGE, "Office hours", start);
    create.recurrence_rule = Some(RecurrenceRuleData::weekly(
        start,
        RecurrenceWeekday::Thursday,
    ));
    let created = rest
        .create_guild_scheduled_event(GUILD, &create, Some("weekly"))
        .await
        .unwrap();
    // The rule of an existing event converts back into a body.
    assert_eq!(
        RecurrenceRuleData::from(*created.recurrence_rule.unwrap()),
        RecurrenceRuleData::weekly(start, RecurrenceWeekday::Thursday)
    );

    rest.get_guild_scheduled_event(GUILD, ID, &GetGuildScheduledEvent::default())
        .await
        .unwrap();

    let mut modify = ModifyGuildScheduledEvent::default();
    modify.status = Some(Status::Active);
    modify.description = Patch::Clear;
    modify.entity_metadata = Patch::Set(EntityMetadata::new("Kraków"));
    rest.modify_guild_scheduled_event(GUILD, ID, &modify, Some("start"))
        .await
        .unwrap();
    rest.delete_guild_scheduled_event(GUILD, ID).await.unwrap();

    let mut users = GetGuildScheduledEventUsers::default();
    users.limit = Some(50);
    users.with_member = Some(true);
    users.after = Some(UserId::new(5));
    let subscribers = rest
        .get_guild_scheduled_event_users(GUILD, ID, &users)
        .await
        .unwrap();
    assert!(subscribers[0].member.is_some());

    let list_path = format!("/guilds/{GUILD}/scheduled-events");
    let one = format!("{list_path}/{ID}");
    let expected: Vec<Expected> = vec![
        ("GET", list_path.clone(), "with_user_count=true", "", None),
        (
            "POST",
            list_path,
            "",
            r#"{"channel_id":"1200000000000000005","name":"Office hours","privacy_level":2,"scheduled_start_time":"2026-11-19T18:00:00.000000+00:00","entity_type":1,"recurrence_rule":{"start":"2026-11-19T18:00:00.000000+00:00","frequency":2,"interval":1,"by_weekday":[3]}}"#,
            Some("weekly"),
        ),
        ("GET", one.clone(), "", "", None),
        (
            "PATCH",
            one.clone(),
            "",
            r#"{"entity_metadata":{"location":"Kraków"},"description":null,"status":2}"#,
            Some("start"),
        ),
        ("DELETE", one.clone(), "", "", None),
        (
            "GET",
            format!("{one}/users"),
            "limit=50&with_member=true&after=5",
            "",
            None,
        ),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body, reason)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
