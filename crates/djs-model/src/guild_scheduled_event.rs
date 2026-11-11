//! [Guild scheduled events](https://docs.discord.com/developers/resources/guild-scheduled-event):
//! events planned in a stage channel, a voice channel or somewhere else.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    guild::PartialMember,
    id::{ChannelId, GenericId, GuildId, ScheduledEventId, UserId},
    macros::int_enum,
    user::User,
    util::{ImageHash, Timestamp},
};

/// A [scheduled event](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-object)
/// of a guild.
///
/// Which location fields are set depends on [`entity_type`](Self::entity_type): stage and
/// voice events have a [`channel_id`](Self::channel_id); external events have
/// [`entity_metadata`](Self::entity_metadata) with a location and a
/// [`scheduled_end_time`](Self::scheduled_end_time).
///
/// discord.js counterpart: `GuildScheduledEvent`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildScheduledEvent {
    /// The event's ID.
    pub id: ScheduledEventId,
    /// The guild the event belongs to.
    pub guild_id: GuildId,
    /// The stage or voice channel the event takes place in; `None` for external events.
    pub channel_id: Option<ChannelId>,
    /// Who created the event; `None` for events created before October 25th, 2021.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator_id: Option<UserId>,
    /// The event's name (1-100 characters).
    pub name: CompactString,
    /// The event's description (1-1000 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
    /// When the event starts.
    pub scheduled_start_time: Timestamp,
    /// When the event ends; required for external events.
    pub scheduled_end_time: Option<Timestamp>,
    /// Who can see the event.
    pub privacy_level: PrivacyLevel,
    /// Whether the event is scheduled, running, finished or canceled.
    pub status: Status,
    /// Where the event takes place.
    pub entity_type: EntityType,
    /// The ID of the entity associated with the event (e.g. the stage instance).
    pub entity_id: Option<GenericId>,
    /// Extra data for external events.
    pub entity_metadata: Option<EntityMetadata>,
    /// The user who created the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<Box<User>>,
    /// How many users are interested in the event (with `with_user_count=true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_count: Option<u32>,
    /// The [cover image hash](https://docs.discord.com/developers/reference#image-formatting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageHash>,
    /// How the event repeats; `None` for one-time events.
    pub recurrence_rule: Option<Box<RecurrenceRule>>,
}

int_enum! {
    /// [Who can see](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-privacy-level)
    /// a scheduled event.
    ///
    /// discord.js counterpart: `GuildScheduledEventPrivacyLevel`.
    #[doc(alias = "GuildScheduledEventPrivacyLevel")]
    pub enum PrivacyLevel: u8 {
        /// Only guild members.
        GuildOnly = 2,
    }
}

int_enum! {
    /// [Where](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-entity-types)
    /// a scheduled event takes place.
    ///
    /// discord.js counterpart: `GuildScheduledEventEntityType`.
    #[doc(alias = "GuildScheduledEventEntityType")]
    pub enum EntityType: u8 {
        /// In a stage channel.
        StageInstance = 1,
        /// In a voice channel.
        Voice = 2,
        /// Outside Discord (or in a text channel); has a location.
        External = 3,
    }
}

int_enum! {
    /// The [status](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-status)
    /// of a scheduled event.
    ///
    /// A scheduled event can become active or canceled; an active event can only become
    /// completed. Completed and canceled events cannot change any more.
    ///
    /// discord.js counterpart: `GuildScheduledEventStatus`.
    #[doc(alias = "GuildScheduledEventStatus")]
    pub enum Status: u8 {
        /// Not started yet.
        Scheduled = 1,
        /// Running.
        Active = 2,
        /// Finished.
        Completed = 3,
        /// Canceled.
        Canceled = 4,
    }
}

/// [Extra data](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-entity-metadata)
/// of an external scheduled event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EntityMetadata {
    /// Where the event takes place (1-100 characters); required for external events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<CompactString>,
}

impl EntityMetadata {
    /// The metadata of an external event taking place at `location` (1-100 characters).
    #[must_use]
    pub fn new(location: impl Into<CompactString>) -> Self {
        Self {
            location: Some(location.into()),
        }
    }
}

/// A [user interested in a scheduled event](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-user-object).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GuildScheduledEventUser {
    /// The event the user is interested in.
    pub guild_scheduled_event_id: ScheduledEventId,
    /// The user.
    pub user: User,
    /// The user's member data in the event's guild (with `with_member=true`), without the
    /// user, which is [`user`](Self::user); combine them with [`PartialMember::into_member`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<PartialMember>>,
}

/// [How a scheduled event repeats](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object),
/// modelled on the iCalendar RRULE.
///
/// Discord only accepts a subset of combinations (for example weekly events on one weekday, or
/// monthly events on the n-th weekday); see the documentation for the allowed shapes. `end`,
/// `by_year_day` and `count` are set by Discord and cannot be sent.
///
/// discord.js counterpart: `GuildScheduledEventRecurrenceRule`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecurrenceRule {
    /// When the recurrence interval starts.
    pub start: Timestamp,
    /// When the recurrence interval ends.
    pub end: Option<Timestamp>,
    /// How often the event repeats.
    pub frequency: RecurrenceFrequency,
    /// The spacing between events in units of `frequency` (e.g. 2 with `Weekly` = every other
    /// week).
    pub interval: u16,
    /// The weekdays to repeat on.
    pub by_weekday: Option<Vec<RecurrenceWeekday>>,
    /// The n-th weekdays of the month to repeat on.
    pub by_n_weekday: Option<Vec<RecurrenceNWeekday>>,
    /// The months to repeat in.
    pub by_month: Option<Vec<RecurrenceMonth>>,
    /// The days of the month to repeat on.
    pub by_month_day: Option<Vec<u8>>,
    /// The days of the year to repeat on (1-364).
    pub by_year_day: Option<Vec<u16>>,
    /// How many times the event may repeat before stopping.
    pub count: Option<u32>,
}

int_enum! {
    /// [How often](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object-guild-scheduled-event-recurrence-rule-frequency)
    /// a scheduled event repeats.
    ///
    /// discord.js counterpart: `GuildScheduledEventRecurrenceRuleFrequency`.
    pub enum RecurrenceFrequency: u8 {
        /// Every year.
        Yearly = 0,
        /// Every month.
        Monthly = 1,
        /// Every week.
        Weekly = 2,
        /// Every day.
        Daily = 3,
    }
}

int_enum! {
    /// A [day of the week](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object-guild-scheduled-event-recurrence-rule-weekday)
    /// in a [`RecurrenceRule`].
    ///
    /// discord.js counterpart: `GuildScheduledEventRecurrenceRuleWeekday`.
    pub enum RecurrenceWeekday: u8 {
        /// Monday.
        Monday = 0,
        /// Tuesday.
        Tuesday = 1,
        /// Wednesday.
        Wednesday = 2,
        /// Thursday.
        Thursday = 3,
        /// Friday.
        Friday = 4,
        /// Saturday.
        Saturday = 5,
        /// Sunday.
        Sunday = 6,
    }
}

/// The [n-th weekday of a month](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object-guild-scheduled-event-recurrence-rule-nweekday-structure),
/// e.g. the second Tuesday.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecurrenceNWeekday {
    /// The week of the month (1-5).
    pub n: u8,
    /// The day of that week.
    pub day: RecurrenceWeekday,
}

impl RecurrenceNWeekday {
    /// The `n`-th (1-5) `day` of a month, e.g. `new(2, RecurrenceWeekday::Tuesday)` for the
    /// second Tuesday.
    #[must_use]
    pub const fn new(n: u8, day: RecurrenceWeekday) -> Self {
        Self { n, day }
    }
}

int_enum! {
    /// A [month](https://docs.discord.com/developers/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object-guild-scheduled-event-recurrence-rule-month)
    /// in a [`RecurrenceRule`].
    ///
    /// discord.js counterpart: `GuildScheduledEventRecurrenceRuleMonth`.
    pub enum RecurrenceMonth: u8 {
        /// January.
        January = 1,
        /// February.
        February = 2,
        /// March.
        March = 3,
        /// April.
        April = 4,
        /// May.
        May = 5,
        /// June.
        June = 6,
        /// July.
        July = 7,
        /// August.
        August = 8,
        /// September.
        September = 9,
        /// October.
        October = 10,
        /// November.
        November = 11,
        /// December.
        December = 12,
    }
}
