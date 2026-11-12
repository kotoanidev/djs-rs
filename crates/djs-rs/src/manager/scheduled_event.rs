//! [`ScheduledEventManager`].

use std::{future::Future, sync::Arc};

use djs_cache::{Backend, GuildView};
use djs_collection::Collection;
use djs_model::{
    guild_scheduled_event::{GuildScheduledEvent, GuildScheduledEventUser},
    id::{GuildId, ScheduledEventId, UserId},
    rest::{
        CreateGuildScheduledEvent, GetGuildScheduledEvent, GetGuildScheduledEventUsers,
        ListScheduledEventsForGuild, ModifyGuildScheduledEvent,
    },
};
use djs_rest::Error;

use super::Fetch;
use crate::Context;

/// The scheduled events of one guild. Returned by [`Context::scheduled_events`].
///
/// Events fetched from Discord carry their interested-user count (`user_count`).
///
/// discord.js counterpart: `GuildScheduledEventManager` (`guild.scheduledEvents`).
#[derive(Debug)]
pub struct ScheduledEventManager<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
}

impl<B: Backend> Clone for ScheduledEventManager<'_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for ScheduledEventManager<'_, B> {}

impl<'a, B: Backend> ScheduledEventManager<'a, B> {
    pub(crate) const fn new(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self { ctx, guild_id }
    }

    /// The guild these events belong to.
    #[must_use]
    pub const fn guild_id(self) -> GuildId {
        self.guild_id
    }

    /// The cached events of the guild, if the client has a cache.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#cache`.
    #[must_use]
    pub fn cache(self) -> Option<GuildView<'a, ScheduledEventId, GuildScheduledEvent, B>> {
        self.ctx
            .cache()
            .map(|cache| cache.scheduled_events(self.guild_id))
    }

    /// An event: the cached one, or [Get Guild Scheduled
    /// Event](djs_rest::Rest::get_guild_scheduled_event), whose result is cached.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#fetch(id)`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if Discord was asked and the request failed.
    pub fn fetch(
        self,
        event_id: ScheduledEventId,
    ) -> Fetch<
        Arc<GuildScheduledEvent>,
        impl Future<Output = Result<Arc<GuildScheduledEvent>, Error>> + Send + 'a,
    > {
        let cached = self.cache().and_then(|events| events.get(event_id));
        let Self { ctx, guild_id } = self;
        Fetch::new(cached, async move {
            let mut query = GetGuildScheduledEvent::default();
            query.with_user_count = Some(true);
            let event = ctx
                .rest()
                .get_guild_scheduled_event(guild_id, event_id, &query)
                .await?;
            Ok(ctx.store_scheduled_event(guild_id, &event))
        })
    }

    /// Every scheduled event of the guild ([List Scheduled Events for
    /// Guild](djs_rest::Rest::list_scheduled_events_for_guild)); the events are cached.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#fetch()`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn list(
        self,
    ) -> Result<Collection<ScheduledEventId, Arc<GuildScheduledEvent>>, Error> {
        let mut query = ListScheduledEventsForGuild::default();
        query.with_user_count = Some(true);
        let events = self
            .ctx
            .rest()
            .list_scheduled_events_for_guild(self.guild_id, &query)
            .await?;
        Ok(events
            .iter()
            .map(|event| {
                (
                    event.id,
                    self.ctx.store_scheduled_event(self.guild_id, event),
                )
            })
            .collect())
    }

    /// Creates an event ([Create Guild Scheduled
    /// Event](djs_rest::Rest::create_guild_scheduled_event)); it is cached.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#create`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `CREATE_EVENTS`).
    pub async fn create(
        self,
        body: &CreateGuildScheduledEvent,
        reason: Option<&str>,
    ) -> Result<Arc<GuildScheduledEvent>, Error> {
        let event = self
            .ctx
            .rest()
            .create_guild_scheduled_event(self.guild_id, body, reason)
            .await?;
        Ok(self.ctx.store_scheduled_event(self.guild_id, &event))
    }

    /// Edits an event, also to start, end or cancel it ([Modify Guild Scheduled
    /// Event](djs_rest::Rest::modify_guild_scheduled_event)); the result is cached.
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#edit`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed (it needs `MANAGE_EVENTS`).
    pub async fn edit(
        self,
        event_id: ScheduledEventId,
        body: &ModifyGuildScheduledEvent,
        reason: Option<&str>,
    ) -> Result<Arc<GuildScheduledEvent>, Error> {
        let event = self
            .ctx
            .rest()
            .modify_guild_scheduled_event(self.guild_id, event_id, body, reason)
            .await?;
        Ok(self.ctx.store_scheduled_event(self.guild_id, &event))
    }

    /// Deletes an event ([Delete Guild Scheduled
    /// Event](djs_rest::Rest::delete_guild_scheduled_event); Discord takes no audit-log
    /// reason). The cache is updated by the `GUILD_SCHEDULED_EVENT_DELETE` event that
    /// follows (D-236).
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#delete`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn delete(self, event_id: ScheduledEventId) -> Result<(), Error> {
        self.ctx
            .rest()
            .delete_guild_scheduled_event(self.guild_id, event_id)
            .await
    }

    /// A page of the users interested in an event, by user ID ([Get Guild Scheduled Event
    /// Users](djs_rest::Rest::get_guild_scheduled_event_users)).
    ///
    /// discord.js counterpart: `GuildScheduledEventManager#fetchSubscribers`.
    ///
    /// # Errors
    ///
    /// Returns the REST error if the request failed.
    pub async fn users(
        self,
        event_id: ScheduledEventId,
        query: &GetGuildScheduledEventUsers,
    ) -> Result<Collection<UserId, GuildScheduledEventUser>, Error> {
        let users = self
            .ctx
            .rest()
            .get_guild_scheduled_event_users(self.guild_id, event_id, query)
            .await?;
        Ok(users.into_iter().map(|user| (user.user.id, user)).collect())
    }
}
