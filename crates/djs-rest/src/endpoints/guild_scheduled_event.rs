//! [Guild scheduled event endpoints](https://docs.discord.com/developers/resources/guild-scheduled-event):
//! events planned in a stage channel, a voice channel or elsewhere, and their subscribers.

use djs_model::{
    guild_scheduled_event::{GuildScheduledEvent, GuildScheduledEventUser},
    id::{GuildId, ScheduledEventId},
    rest::{
        CreateGuildScheduledEvent, GetGuildScheduledEvent, GetGuildScheduledEventUsers,
        ListScheduledEventsForGuild, ModifyGuildScheduledEvent,
    },
};

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [List Scheduled Events for Guild](https://docs.discord.com/developers/resources/guild-scheduled-event#list-scheduled-events-for-guild):
    /// returns the guild's scheduled and active events.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_scheduled_events_for_guild<'a>(
        &'a self,
        guild_id: GuildId,
        query: &'a ListScheduledEventsForGuild,
    ) -> impl Future<Output = Result<Vec<GuildScheduledEvent>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListScheduledEventsForGuild { guild_id }).query(query))
                .await
        })
    }

    /// [Create Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#create-guild-scheduled-event):
    /// plans an event. Needs `CREATE_EVENTS`, plus the permissions to manage the stage or join
    /// the voice channel for events there.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#create`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, stage: djs_model::id::ChannelId) -> Result<(), Box<dyn std::error::Error>> {
    /// use djs_model::{
    ///     guild_scheduled_event::RecurrenceWeekday,
    ///     rest::{CreateGuildScheduledEvent, RecurrenceRuleData},
    ///     util::Timestamp,
    /// };
    ///
    /// let start = Timestamp::parse("2026-11-19T18:00:00Z")?;
    /// let mut body = CreateGuildScheduledEvent::stage(stage, "Office hours", start);
    /// body.recurrence_rule = Some(RecurrenceRuleData::weekly(start, RecurrenceWeekday::Thursday));
    /// let event = rest.create_guild_scheduled_event(guild_id, &body, None).await?;
    /// println!("Planned event {}", event.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30038` at 100 events that have not ended; `50035` for a
    /// combination of fields Discord does not accept.
    pub fn create_guild_scheduled_event<'a>(
        &'a self,
        guild_id: GuildId,
        body: &'a CreateGuildScheduledEvent,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<GuildScheduledEvent, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::CreateGuildScheduledEvent { guild_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Get Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#get-guild-scheduled-event):
    /// returns one event.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10070` Unknown Guild Scheduled Event.
    pub fn get_guild_scheduled_event<'a>(
        &'a self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
        query: &'a GetGuildScheduledEvent,
    ) -> impl Future<Output = Result<GuildScheduledEvent, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::GetGuildScheduledEvent { guild_id, event_id }).query(query),
            )
            .await
        })
    }

    /// [Modify Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#modify-guild-scheduled-event):
    /// changes an event, or starts, ends or cancels it through its
    /// [`status`](ModifyGuildScheduledEvent::status). Needs `MANAGE_EVENTS`, or `CREATE_EVENTS`
    /// for events the bot created.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `GuildScheduledEvent#edit`, `#setStatus`,
    /// `GuildScheduledEventManager#edit`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, guild_id: djs_model::id::GuildId, event_id: djs_model::id::ScheduledEventId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{guild_scheduled_event::Status, rest::ModifyGuildScheduledEvent};
    ///
    /// let mut body = ModifyGuildScheduledEvent::default();
    /// body.status = Some(Status::Completed);
    /// rest.modify_guild_scheduled_event(guild_id, event_id, &body, Some("Meetup is over"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10070` Unknown Guild Scheduled Event; `180000` for an event that
    /// has already ended.
    pub fn modify_guild_scheduled_event<'a>(
        &'a self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
        body: &'a ModifyGuildScheduledEvent,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<GuildScheduledEvent, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::ModifyGuildScheduledEvent { guild_id, event_id })
                    .json(body)?
                    .reason(reason),
            )
            .await
        })
    }

    /// [Delete Guild Scheduled Event](https://docs.discord.com/developers/resources/guild-scheduled-event#delete-guild-scheduled-event):
    /// deletes an event. Needs `MANAGE_EVENTS`, or `CREATE_EVENTS` for events the bot
    /// created.
    ///
    /// discord.js counterpart: `GuildScheduledEvent#delete`,
    /// `GuildScheduledEventManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10070` Unknown Guild Scheduled Event.
    pub fn delete_guild_scheduled_event(
        &self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::DeleteGuildScheduledEvent {
                guild_id,
                event_id,
            }))
            .await
        })
    }

    /// [Get Guild Scheduled Event Users](https://docs.discord.com/developers/resources/guild-scheduled-event#get-guild-scheduled-event-users):
    /// returns a page of the users interested in an event, sorted by user ID (100 by
    /// default); with [`with_member`](GetGuildScheduledEventUsers::with_member), their member
    /// data too.
    ///
    /// discord.js counterpart: `GuildScheduledEvent#fetchSubscribers`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10070` Unknown Guild Scheduled Event.
    pub fn get_guild_scheduled_event_users<'a>(
        &'a self,
        guild_id: GuildId,
        event_id: ScheduledEventId,
        query: &'a GetGuildScheduledEventUsers,
    ) -> impl Future<Output = Result<Vec<GuildScheduledEventUser>, Error>> + Send + 'a {
        boxed(async move {
            self.call(
                Request::new(Route::GetGuildScheduledEventUsers { guild_id, event_id })
                    .query(query),
            )
            .await
        })
    }
}
