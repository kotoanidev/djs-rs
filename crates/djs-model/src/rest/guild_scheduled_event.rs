//! Bodies and queries of the [guild scheduled event endpoints](https://docs.discord.com/developers/resources/guild-scheduled-event).

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    guild_scheduled_event::{
        EntityMetadata, EntityType, PrivacyLevel, RecurrenceFrequency, RecurrenceMonth,
        RecurrenceNWeekday, RecurrenceRule, RecurrenceWeekday, Status,
    },
    id::{ChannelId, UserId},
    util::{ImageData, Patch, Timestamp},
};

/// The query of [List Scheduled Events for Guild](https://docs.discord.com/developers/resources/guild-scheduled-event#list-scheduled-events-for-guild).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListScheduledEventsForGuild {
    /// Whether to include each event's [`user_count`](crate::guild_scheduled_event::GuildScheduledEvent::user_count).
    pub with_user_count: Option<bool>,
}

/// The query of [Get Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#get-guild-scheduled-event).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetGuildScheduledEvent {
    /// Whether to include the event's [`user_count`](crate::guild_scheduled_event::GuildScheduledEvent::user_count).
    pub with_user_count: Option<bool>,
}

/// The query of [Get Guild Scheduled Event Users](https://docs.discord.com/developers/resources/guild-scheduled-event#get-guild-scheduled-event-users).
///
/// Users are sorted by ID; with both `before` and `after`, only `before` is used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GetGuildScheduledEventUsers {
    /// The page size (max 100, the default).
    pub limit: Option<u8>,
    /// Whether to include each user's member data in the guild.
    pub with_member: Option<bool>,
    /// Users with a lower ID.
    pub before: Option<UserId>,
    /// Users with a higher ID.
    pub after: Option<UserId>,
}

/// How a scheduled event repeats, as sent to Discord (D-124): the settable fields of a
/// [`RecurrenceRule`].
///
/// Discord accepts only some combinations; the constructors build exactly those shapes:
/// [`daily`](Self::daily), [`daily_on`](Self::daily_on) (a known set of weekdays),
/// [`weekly`](Self::weekly), [`every_other_week`](Self::every_other_week),
/// [`monthly`](Self::monthly) (the n-th weekday) and [`yearly`](Self::yearly).
/// [`From<RecurrenceRule>`](#impl-From<RecurrenceRule>-for-RecurrenceRuleData) keeps the rule of
/// an existing event.
///
/// discord.js counterpart: `GuildScheduledEventRecurrenceRuleOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     guild_scheduled_event::RecurrenceWeekday, rest::RecurrenceRuleData, util::Timestamp,
/// };
///
/// let start = Timestamp::parse("2026-11-17T18:00:00Z")?;
/// let rule = RecurrenceRuleData::monthly(start, 3, RecurrenceWeekday::Tuesday);
/// assert_eq!(
///     serde_json::to_string(&rule)?,
///     r#"{"start":"2026-11-17T18:00:00.000000+00:00","frequency":1,"interval":1,"by_n_weekday":[{"n":3,"day":1}]}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RecurrenceRuleData {
    /// When the recurrence starts (the first occurrence).
    pub start: Timestamp,
    /// How often the event repeats.
    pub frequency: RecurrenceFrequency,
    /// The spacing between occurrences in units of `frequency`; only weekly events may use 2.
    pub interval: u16,
    /// The weekdays to repeat on (daily: a known set; weekly: one day).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_weekday: Vec<RecurrenceWeekday>,
    /// The n-th weekday of the month to repeat on (monthly: one entry).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_n_weekday: Vec<RecurrenceNWeekday>,
    /// The month to repeat in (yearly: one entry, with [`by_month_day`](Self::by_month_day)).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_month: Vec<RecurrenceMonth>,
    /// The day of the month to repeat on (yearly: one entry, with [`by_month`](Self::by_month)).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_month_day: Vec<u8>,
}

impl RecurrenceRuleData {
    const fn new(start: Timestamp, frequency: RecurrenceFrequency, interval: u16) -> Self {
        Self {
            start,
            frequency,
            interval,
            by_weekday: Vec::new(),
            by_n_weekday: Vec::new(),
            by_month: Vec::new(),
            by_month_day: Vec::new(),
        }
    }

    /// Every day.
    #[must_use]
    pub const fn daily(start: Timestamp) -> Self {
        Self::new(start, RecurrenceFrequency::Daily, 1)
    }

    /// Every day of a set of weekdays. Discord accepts these sets only: Monday–Friday,
    /// Tuesday–Saturday, Sunday–Thursday, Friday and Saturday, Saturday and Sunday, Sunday and
    /// Monday.
    #[must_use]
    pub fn daily_on(start: Timestamp, days: impl IntoIterator<Item = RecurrenceWeekday>) -> Self {
        Self {
            by_weekday: days.into_iter().collect(),
            ..Self::daily(start)
        }
    }

    /// Every week on `day`.
    #[must_use]
    pub fn weekly(start: Timestamp, day: RecurrenceWeekday) -> Self {
        Self {
            by_weekday: vec![day],
            ..Self::new(start, RecurrenceFrequency::Weekly, 1)
        }
    }

    /// Every other week on `day`.
    #[must_use]
    pub fn every_other_week(start: Timestamp, day: RecurrenceWeekday) -> Self {
        Self {
            by_weekday: vec![day],
            ..Self::new(start, RecurrenceFrequency::Weekly, 2)
        }
    }

    /// Every month on the `n`-th (1-5) `day`, e.g. the third Tuesday.
    #[must_use]
    pub fn monthly(start: Timestamp, n: u8, day: RecurrenceWeekday) -> Self {
        Self {
            by_n_weekday: vec![RecurrenceNWeekday::new(n, day)],
            ..Self::new(start, RecurrenceFrequency::Monthly, 1)
        }
    }

    /// Every year on `day` of `month`.
    #[must_use]
    pub fn yearly(start: Timestamp, month: RecurrenceMonth, day: u8) -> Self {
        Self {
            by_month: vec![month],
            by_month_day: vec![day],
            ..Self::new(start, RecurrenceFrequency::Yearly, 1)
        }
    }
}

impl From<RecurrenceRule> for RecurrenceRuleData {
    /// Keeps the settable fields of an existing rule (not `end`, `count`, `by_year_day`).
    fn from(rule: RecurrenceRule) -> Self {
        Self {
            start: rule.start,
            frequency: rule.frequency,
            interval: rule.interval,
            by_weekday: rule.by_weekday.unwrap_or_default(),
            by_n_weekday: rule.by_n_weekday.unwrap_or_default(),
            by_month: rule.by_month.unwrap_or_default(),
            by_month_day: rule.by_month_day.unwrap_or_default(),
        }
    }
}

/// The body of [Create Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#create-guild-scheduled-event).
///
/// Build it with the constructor of the event's place (D-125) — [`stage`](Self::stage),
/// [`voice`](Self::voice) or [`external`](Self::external) — which sets the fields that place
/// requires; the rest are optional. A guild can have at most 100 scheduled or active events.
///
/// discord.js counterpart: `GuildScheduledEventCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{rest::CreateGuildScheduledEvent, util::Timestamp};
///
/// let start = Timestamp::parse("2026-11-20T18:00:00Z")?;
/// let end = Timestamp::parse("2026-11-20T20:00:00Z")?;
/// let mut body = CreateGuildScheduledEvent::external("Rust meetup, Kraków", "Meetup", start, end);
/// body.description = Some("Talks and pizza".into());
/// let json = serde_json::to_string(&body)?;
/// assert!(json.starts_with(r#"{"entity_metadata":{"location":"Rust meetup, Kraków"},"name":"Meetup","privacy_level":2,"#));
/// assert!(json.ends_with(r#""description":"Talks and pizza","entity_type":3}"#));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CreateGuildScheduledEvent {
    /// The stage or voice channel (not for external events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// The location of an external event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<EntityMetadata>,
    /// The name (1-100 characters).
    pub name: CompactString,
    /// Who can see the event (only [`PrivacyLevel::GuildOnly`] exists).
    pub privacy_level: PrivacyLevel,
    /// When the event starts.
    pub scheduled_start_time: Timestamp,
    /// When the event ends (required for external events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<Timestamp>,
    /// The description (1-1000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// Where the event takes place.
    pub entity_type: EntityType,
    /// The cover image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageData>,
    /// How the event repeats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<RecurrenceRuleData>,
}

impl CreateGuildScheduledEvent {
    fn new(name: CompactString, start: Timestamp, entity_type: EntityType) -> Self {
        Self {
            channel_id: None,
            entity_metadata: None,
            name,
            privacy_level: PrivacyLevel::GuildOnly,
            scheduled_start_time: start,
            scheduled_end_time: None,
            description: None,
            entity_type,
            image: None,
            recurrence_rule: None,
        }
    }

    /// An event in the stage channel `channel_id`.
    #[must_use]
    pub fn stage(channel_id: ChannelId, name: impl Into<CompactString>, start: Timestamp) -> Self {
        Self {
            channel_id: Some(channel_id),
            ..Self::new(name.into(), start, EntityType::StageInstance)
        }
    }

    /// An event in the voice channel `channel_id`.
    #[must_use]
    pub fn voice(channel_id: ChannelId, name: impl Into<CompactString>, start: Timestamp) -> Self {
        Self {
            channel_id: Some(channel_id),
            ..Self::new(name.into(), start, EntityType::Voice)
        }
    }

    /// An event outside Discord's voice channels, at `location` (1-100 characters), from
    /// `start` to `end`.
    #[must_use]
    pub fn external(
        location: impl Into<CompactString>,
        name: impl Into<CompactString>,
        start: Timestamp,
        end: Timestamp,
    ) -> Self {
        Self {
            entity_metadata: Some(EntityMetadata::new(location)),
            scheduled_end_time: Some(end),
            ..Self::new(name.into(), start, EntityType::External)
        }
    }
}

/// The body of [Modify Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#modify-guild-scheduled-event).
///
/// Every field is optional; [`Patch`] fields can also be cleared. Starting, ending or
/// canceling an event is a change of [`status`](Self::status). Moving an event to
/// [`EntityType::External`] needs [`channel_id`](Self::channel_id) set to [`Patch::Clear`],
/// a location in [`entity_metadata`](Self::entity_metadata) and a
/// [`scheduled_end_time`](Self::scheduled_end_time).
///
/// discord.js counterpart: `GuildScheduledEventEditOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{guild_scheduled_event::Status, rest::ModifyGuildScheduledEvent};
///
/// let mut body = ModifyGuildScheduledEvent::default();
/// body.status = Some(Status::Active); // start the event
/// assert_eq!(serde_json::to_string(&body)?, r#"{"status":2}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildScheduledEvent {
    /// The stage or voice channel; [`Patch::Clear`] when moving the event outside.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub channel_id: Patch<ChannelId>,
    /// The location of an external event (ignored for other events).
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub entity_metadata: Patch<EntityMetadata>,
    /// The name (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// Who can see the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<PrivacyLevel>,
    /// When the event starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start_time: Option<Timestamp>,
    /// When the event ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<Timestamp>,
    /// The description; [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub description: Patch<CompactString>,
    /// Where the event takes place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<EntityType>,
    /// The status: scheduled events can become active or canceled, active events completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    /// The cover image; [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub image: Patch<ImageData>,
    /// How the event repeats; [`Patch::Clear`] makes it a one-time event.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub recurrence_rule: Patch<RecurrenceRuleData>,
}
