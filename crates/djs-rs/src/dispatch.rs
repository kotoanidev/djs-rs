//! Turning a gateway dispatch into client events (D-221–D-228).
//!
//! For each dispatch: the cache is updated first (D-171), then [`Raw`](crate::event::Raw),
//! the event itself and the events derived from it are pushed to `out`. A payload is only
//! built for an event someone listens to (`wants`).

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};

use djs_cache::{Backend, Cache, Previous, Resource};
use djs_model::{
    gateway::{
        DispatchEvent,
        payload::{GuildCreate, Ready},
    },
    id::GuildId,
};

use crate::event::{Dispatch, EventKind, payload};

/// What a shard remembers between dispatches.
#[derive(Debug, Default)]
pub(crate) struct ShardTracker {
    /// The guilds known to be unavailable: from `READY`, or a `GUILD_DELETE` with
    /// `unavailable` (D-223). Their next `GUILD_CREATE` is `GuildAvailable`.
    unavailable: HashSet<GuildId>,
    /// Set by a `READY`.
    ready: Option<ReadySeen>,
}

/// A `READY` was received, with its payload if `ShardReady` or `ClientReady` was wanted.
#[derive(Debug)]
pub(crate) struct ReadySeen(pub(crate) Option<Arc<Ready>>);

impl ShardTracker {
    /// Returns the `READY` of the last dispatch, if it was one.
    pub(crate) const fn take_ready(&mut self) -> Option<ReadySeen> {
        self.ready.take()
    }
}

/// Takes the previous state of one kind out of `previous`.
macro_rules! take {
    ($previous:ident, $($variant:ident)|+) => {
        match $previous {
            $(Previous::$variant(value))|+ => Some(value),
            _ => None,
        }
    };
}

/// Converts `event` and pushes the resulting events to `out`.
#[expect(clippy::too_many_lines, reason = "one arm per gateway dispatch")]
pub(crate) fn convert<B: Backend>(
    event: DispatchEvent,
    cache: Option<&Cache<B>>,
    tracker: &mut ShardTracker,
    wants: impl Fn(EventKind) -> bool,
    out: &mut Vec<Dispatch>,
) {
    let previous = cache.map_or(Previous::None, |cache| cache.update_dispatch(&event));
    if wants(EventKind::Raw) {
        out.push(Dispatch::Raw(Arc::new(event.clone())));
    }

    /// Pushes an event built by `$value`, which is only evaluated if someone listens.
    macro_rules! emit {
        ($kind:ident, $value:expr) => {
            if wants(EventKind::$kind) {
                out.push(Dispatch::$kind($value))
            }
        };
    }
    /// An event with a previous state: `Event(previous variants, boxed payload)`.
    macro_rules! with_old {
        ($kind:ident($($variant:ident)|+, $payload:expr)) => {
            emit!(
                $kind,
                Arc::new(payload::$kind {
                    old: take!(previous, $($variant)|+),
                    new: $payload,
                })
            )
        };
    }

    match event {
        DispatchEvent::Ready(ready) => {
            tracker.unavailable = ready.guilds.iter().map(|guild| guild.id).collect();
            let built =
                (wants(EventKind::ShardReady) || wants(EventKind::ClientReady)).then(|| {
                    let ready: Arc<Ready> = Arc::from(ready);
                    emit!(ShardReady, Arc::clone(&ready));
                    ready
                });
            tracker.ready = Some(ReadySeen(built));
        }
        // `ShardResume` is emitted by the client, which counts the replayed events.
        DispatchEvent::Resumed => {}
        DispatchEvent::RateLimited(event) => emit!(RateLimited, Arc::from(event)),
        DispatchEvent::ApplicationCommandPermissionsUpdate(event) => {
            emit!(ApplicationCommandPermissionsUpdate, Arc::from(event));
        }
        DispatchEvent::AutoModerationRuleCreate(event) => {
            emit!(AutoModerationRuleCreate, Arc::from(event));
        }
        DispatchEvent::AutoModerationRuleUpdate(event) => {
            emit!(AutoModerationRuleUpdate, Arc::from(event));
        }
        DispatchEvent::AutoModerationRuleDelete(event) => {
            emit!(AutoModerationRuleDelete, Arc::from(event));
        }
        DispatchEvent::AutoModerationActionExecution(event) => {
            emit!(AutoModerationActionExecution, Arc::from(event));
        }
        DispatchEvent::ChannelCreate(event) => emit!(ChannelCreate, Arc::from(event)),
        DispatchEvent::ChannelUpdate(event) => with_old!(ChannelUpdate(Channel, *event)),
        DispatchEvent::ChannelDelete(event) => with_old!(ChannelDelete(Channel, *event)),
        DispatchEvent::ChannelInfo(event) => emit!(ChannelInfo, Arc::from(event)),
        DispatchEvent::ChannelPinsUpdate(event) => emit!(ChannelPinsUpdate, Arc::from(event)),
        DispatchEvent::ThreadCreate(event) => emit!(ThreadCreate, Arc::from(event)),
        DispatchEvent::ThreadUpdate(event) => with_old!(ThreadUpdate(Thread, *event)),
        DispatchEvent::ThreadDelete(event) => with_old!(ThreadDelete(Thread, *event)),
        DispatchEvent::ThreadListSync(event) => emit!(ThreadListSync, Arc::from(event)),
        DispatchEvent::ThreadMemberUpdate(event) => emit!(ThreadMemberUpdate, Arc::from(event)),
        DispatchEvent::ThreadMembersUpdate(event) => {
            emit!(ThreadMembersUpdate, Arc::from(event));
        }
        DispatchEvent::EntitlementCreate(event) => emit!(EntitlementCreate, Arc::from(event)),
        DispatchEvent::EntitlementUpdate(event) => emit!(EntitlementUpdate, Arc::from(event)),
        DispatchEvent::EntitlementDelete(event) => emit!(EntitlementDelete, Arc::from(event)),
        DispatchEvent::GuildCreate(event) => {
            let id = event.id();
            let was_unavailable = tracker.unavailable.remove(&id);
            let available = matches!(*event, GuildCreate::Available(_));
            if !available {
                tracker.unavailable.insert(id);
            }
            if !was_unavailable {
                emit!(GuildCreate, Arc::from(event));
            } else if available {
                emit!(GuildAvailable, Arc::from(event));
            }
        }
        DispatchEvent::GuildUpdate(event) => with_old!(GuildUpdate(Guild, *event)),
        DispatchEvent::GuildDelete(event) => {
            if event.unavailable {
                tracker.unavailable.insert(event.id);
                // The cache keeps an unavailable guild, so `old` is the guild it holds.
                emit!(
                    GuildUnavailable,
                    Arc::new(payload::GuildUnavailable {
                        old: cache.and_then(|cache| cache.guilds().get(event.id)),
                        new: event,
                    })
                );
            } else {
                tracker.unavailable.remove(&event.id);
                with_old!(GuildDelete(Guild, event));
            }
        }
        DispatchEvent::GuildAuditLogEntryCreate(event) => {
            emit!(GuildAuditLogEntryCreate, Arc::from(event));
        }
        DispatchEvent::GuildBanAdd(event) => emit!(GuildBanAdd, Arc::from(event)),
        DispatchEvent::GuildBanRemove(event) => emit!(GuildBanRemove, Arc::from(event)),
        DispatchEvent::GuildEmojisUpdate(event) => {
            let guild_id = event.guild_id;
            let old = take!(previous, Emojis).unwrap_or_default();
            let diff = cache.is_some_and(|cache| cache.config().is_enabled(Resource::Emoji))
                && [
                    EventKind::GuildEmojiCreate,
                    EventKind::GuildEmojiUpdate,
                    EventKind::GuildEmojiDelete,
                ]
                .into_iter()
                .any(&wants);
            let new = diff.then(|| event.emojis.clone());
            emit!(GuildEmojisUpdate, Arc::from(event));
            if let Some(new) = new {
                diff_list(
                    old,
                    new,
                    |emoji| emoji.id,
                    |change| match change {
                        Change::Create(emoji) => emit!(
                            GuildEmojiCreate,
                            Arc::new(payload::GuildEmojiCreate { guild_id, emoji })
                        ),
                        Change::Update(old, new) => emit!(
                            GuildEmojiUpdate,
                            Arc::new(payload::GuildEmojiUpdate { guild_id, old, new })
                        ),
                        Change::Delete(emoji) => emit!(
                            GuildEmojiDelete,
                            Arc::new(payload::GuildEmojiDelete { guild_id, emoji })
                        ),
                    },
                );
            }
        }
        DispatchEvent::GuildStickersUpdate(event) => {
            let guild_id = event.guild_id;
            let old = take!(previous, Stickers).unwrap_or_default();
            let diff = cache.is_some_and(|cache| cache.config().is_enabled(Resource::Sticker))
                && [
                    EventKind::GuildStickerCreate,
                    EventKind::GuildStickerUpdate,
                    EventKind::GuildStickerDelete,
                ]
                .into_iter()
                .any(&wants);
            let new = diff.then(|| event.stickers.clone());
            emit!(GuildStickersUpdate, Arc::from(event));
            if let Some(new) = new {
                diff_list(
                    old,
                    new,
                    |sticker| sticker.id,
                    |change| match change {
                        Change::Create(sticker) => emit!(
                            GuildStickerCreate,
                            Arc::new(payload::GuildStickerCreate { guild_id, sticker })
                        ),
                        Change::Update(old, new) => emit!(
                            GuildStickerUpdate,
                            Arc::new(payload::GuildStickerUpdate { guild_id, old, new })
                        ),
                        Change::Delete(sticker) => emit!(
                            GuildStickerDelete,
                            Arc::new(payload::GuildStickerDelete { guild_id, sticker })
                        ),
                    },
                );
            }
        }
        DispatchEvent::GuildIntegrationsUpdate(event) => {
            emit!(GuildIntegrationsUpdate, Arc::new(event));
        }
        DispatchEvent::GuildMemberAdd(event) => emit!(GuildMemberAdd, Arc::from(event)),
        DispatchEvent::GuildMemberRemove(event) => {
            with_old!(GuildMemberRemove(Member, *event));
        }
        DispatchEvent::GuildMemberUpdate(event) => {
            with_old!(GuildMemberUpdate(Member, *event));
        }
        DispatchEvent::GuildMembersChunk(event) => emit!(GuildMembersChunk, Arc::from(event)),
        DispatchEvent::GuildRoleCreate(event) => emit!(GuildRoleCreate, Arc::from(event)),
        DispatchEvent::GuildRoleUpdate(event) => with_old!(GuildRoleUpdate(Role, *event)),
        DispatchEvent::GuildRoleDelete(event) => with_old!(GuildRoleDelete(Role, event)),
        DispatchEvent::GuildScheduledEventCreate(event) => {
            emit!(GuildScheduledEventCreate, Arc::from(event));
        }
        DispatchEvent::GuildScheduledEventUpdate(event) => {
            with_old!(GuildScheduledEventUpdate(ScheduledEvent, *event));
        }
        DispatchEvent::GuildScheduledEventDelete(event) => {
            with_old!(GuildScheduledEventDelete(ScheduledEvent, *event));
        }
        DispatchEvent::GuildScheduledEventUserAdd(event) => {
            emit!(GuildScheduledEventUserAdd, Arc::new(event));
        }
        DispatchEvent::GuildScheduledEventUserRemove(event) => {
            emit!(GuildScheduledEventUserRemove, Arc::new(event));
        }
        DispatchEvent::GuildSoundboardSoundCreate(event) => {
            emit!(GuildSoundboardSoundCreate, Arc::from(event));
        }
        DispatchEvent::GuildSoundboardSoundUpdate(event) => {
            with_old!(GuildSoundboardSoundUpdate(SoundboardSound, *event));
        }
        DispatchEvent::GuildSoundboardSoundDelete(event) => {
            with_old!(GuildSoundboardSoundDelete(SoundboardSound, event));
        }
        DispatchEvent::GuildSoundboardSoundsUpdate(event) => {
            emit!(GuildSoundboardSoundsUpdate, Arc::from(event));
        }
        DispatchEvent::SoundboardSounds(event) => emit!(SoundboardSounds, Arc::from(event)),
        DispatchEvent::IntegrationCreate(event) => emit!(IntegrationCreate, Arc::from(event)),
        DispatchEvent::IntegrationUpdate(event) => emit!(IntegrationUpdate, Arc::from(event)),
        DispatchEvent::IntegrationDelete(event) => emit!(IntegrationDelete, Arc::new(event)),
        DispatchEvent::InteractionCreate(event) => emit!(InteractionCreate, Arc::from(event)),
        DispatchEvent::InviteCreate(event) => emit!(InviteCreate, Arc::from(event)),
        DispatchEvent::InviteDelete(event) => emit!(InviteDelete, Arc::from(event)),
        DispatchEvent::MessageCreate(event) => emit!(MessageCreate, Arc::from(event)),
        DispatchEvent::MessageUpdate(event) => with_old!(MessageUpdate(Message, *event)),
        DispatchEvent::MessageDelete(event) => with_old!(MessageDelete(Message, *event)),
        DispatchEvent::MessageDeleteBulk(event) => emit!(
            MessageBulkDelete,
            Arc::new(payload::MessageBulkDelete {
                old: take!(previous, Messages).unwrap_or_default(),
                new: *event,
            })
        ),
        DispatchEvent::MessageReactionAdd(event) => emit!(MessageReactionAdd, Arc::from(event)),
        DispatchEvent::MessageReactionRemove(event) => {
            emit!(MessageReactionRemove, Arc::from(event));
        }
        DispatchEvent::MessageReactionRemoveAll(event) => {
            emit!(MessageReactionRemoveAll, Arc::new(event));
        }
        DispatchEvent::MessageReactionRemoveEmoji(event) => {
            emit!(MessageReactionRemoveEmoji, Arc::from(event));
        }
        DispatchEvent::MessagePollVoteAdd(event) => emit!(MessagePollVoteAdd, Arc::from(event)),
        DispatchEvent::MessagePollVoteRemove(event) => {
            emit!(MessagePollVoteRemove, Arc::from(event));
        }
        DispatchEvent::PresenceUpdate(event) => with_old!(PresenceUpdate(Presence, *event)),
        DispatchEvent::StageInstanceCreate(event) => {
            emit!(StageInstanceCreate, Arc::from(event));
        }
        DispatchEvent::StageInstanceUpdate(event) => {
            with_old!(StageInstanceUpdate(StageInstance, *event));
        }
        DispatchEvent::StageInstanceDelete(event) => {
            with_old!(StageInstanceDelete(StageInstance, *event));
        }
        DispatchEvent::SubscriptionCreate(event) => emit!(SubscriptionCreate, Arc::from(event)),
        DispatchEvent::SubscriptionUpdate(event) => emit!(SubscriptionUpdate, Arc::from(event)),
        DispatchEvent::SubscriptionDelete(event) => emit!(SubscriptionDelete, Arc::from(event)),
        DispatchEvent::TypingStart(event) => emit!(TypingStart, Arc::from(event)),
        DispatchEvent::UserUpdate(event) => with_old!(UserUpdate(CurrentUser, *event)),
        DispatchEvent::VoiceChannelEffectSend(event) => {
            emit!(VoiceChannelEffectSend, Arc::from(event));
        }
        DispatchEvent::VoiceChannelStartTimeUpdate(event) => {
            emit!(VoiceChannelStartTimeUpdate, Arc::from(event));
        }
        DispatchEvent::VoiceChannelStatusUpdate(event) => {
            emit!(VoiceChannelStatusUpdate, Arc::from(event));
        }
        DispatchEvent::VoiceStateUpdate(event) => {
            with_old!(VoiceStateUpdate(VoiceState, *event));
        }
        DispatchEvent::VoiceServerUpdate(event) => emit!(VoiceServerUpdate, Arc::from(event)),
        DispatchEvent::WebhooksUpdate(event) => emit!(WebhooksUpdate, Arc::new(event)),
        DispatchEvent::Unknown(name) => out.push(Dispatch::Unknown(Arc::from(name.as_str()))),
        // A dispatch `djs-model` decodes but this version of the client does not route yet.
        other => out.push(Dispatch::Unknown(Arc::from(other.name()))),
    }
}

/// One change between a cached list and its new version.
enum Change<T> {
    Create(T),
    Update(Arc<T>, T),
    Delete(Arc<T>),
}

/// Compares a guild's cached list with the new one, in discord.js' order: each new item in
/// order (created or, if it changed, updated), then the deleted ones.
fn diff_list<T: PartialEq, K: Hash + Eq>(
    old: Vec<Arc<T>>,
    new: Vec<T>,
    key: impl Fn(&T) -> K,
    mut on: impl FnMut(Change<T>),
) {
    let mut old: HashMap<K, Arc<T>> = old.into_iter().map(|item| (key(&item), item)).collect();
    for item in new {
        match old.remove(&key(&item)) {
            Some(cached) if *cached == item => {}
            Some(cached) => on(Change::Update(cached, item)),
            None => on(Change::Create(item)),
        }
    }
    for (_, item) in old {
        on(Change::Delete(item));
    }
}
