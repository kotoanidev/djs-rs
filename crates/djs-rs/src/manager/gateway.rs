//! Requests answered through the gateway rather than REST (D-265, D-270): guild members
//! ([`MemberManager::fetch_all`](super::MemberManager::fetch_all) and its siblings) and
//! soundboard sounds ([`GuildManager::fetch_soundboard_sounds`](super::GuildManager::fetch_soundboard_sounds)).
//!
//! Each request is sent on the shard of its guild with a nonce of the client; the answers
//! reach it through the collectors' index (D-264), and what they carry is cached.

use core::{
    fmt,
    future::{Future, IntoFuture},
    pin::Pin,
    time::Duration,
};
use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};

use djs_cache::{Backend, CachedMember};
use djs_collection::Collection;
use djs_gateway::SendError;
use djs_model::{
    gateway::{
        ShardId,
        payload::{GuildMembersChunk, RequestGuildMembers, RequestSoundboardSounds},
    },
    id::{GuildId, SoundboardSoundId, UserId},
    soundboard::SoundboardSound,
};
use tokio::{sync::mpsc, time::Instant};

use crate::{
    Context,
    collector::{EndReason, Key, MemberReply, NONCE_PREFIX, Signal, Sink},
};

/// Why a request answered through the gateway failed.
///
/// discord.js counterpart: the `GuildMembersTimeout` and `GuildSoundboardSoundsTimeout`
/// errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GatewayRequestError {
    /// The guild's shard does not run in this client (with
    /// [`Shards::Range`](crate::Shards::Range), another process runs it).
    #[error("the shard of guild {0} does not run in this client")]
    ShardNotRunning(GuildId),
    /// The request could not be queued on the shard (it stopped).
    #[error("the request could not be sent")]
    Send(#[from] SendError),
    /// Discord did not answer completely in time.
    #[error("the gateway did not answer in time")]
    Timeout,
    /// Discord rate limited the request (`RATE_LIMITED`); it may be sent again after
    /// `retry_after`.
    #[error("the request was rate limited; retry after {retry_after:?}")]
    RateLimited {
        /// How long to wait before asking again.
        retry_after: Duration,
    },
    /// The wait ended before the answer: the guild was deleted or the client shut down.
    #[error("the request ended before its answer: {0}")]
    Ended(EndReason),
}

/// What the members request asks for.
#[derive(Clone, Debug)]
enum Members {
    /// Every member, or those whose name starts with the query.
    Query { query: String, limit: u32 },
    /// The members with these IDs.
    Users(Vec<UserId>),
}

/// A request of guild members through the gateway, not sent yet: configure it, then
/// `.await` it. Made by [`MemberManager::fetch_all`](super::MemberManager::fetch_all),
/// [`fetch_by_query`](super::MemberManager::fetch_by_query) and
/// [`fetch_many`](super::MemberManager::fetch_many) (D-270).
///
/// discord.js counterpart: the options of `GuildMemberManager#fetch`.
#[must_use = "the request is only sent when awaited"]
pub struct FetchMembers<'a, B: Backend> {
    ctx: &'a Context<B>,
    guild_id: GuildId,
    members: Members,
    presences: bool,
    time: Duration,
}

impl<B: Backend> fmt::Debug for FetchMembers<'_, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FetchMembers")
            .field("guild_id", &self.guild_id)
            .field("members", &self.members)
            .field("presences", &self.presences)
            .field("time", &self.time)
            .finish_non_exhaustive()
    }
}

/// The time discord.js waits for the members' chunks.
const MEMBERS_TIME: Duration = Duration::from_secs(120);

/// The most user IDs Discord accepts in one request.
const MAX_USER_IDS: usize = 100;

impl<'a, B: Backend> FetchMembers<'a, B> {
    pub(crate) fn all(ctx: &'a Context<B>, guild_id: GuildId) -> Self {
        Self::new(
            ctx,
            guild_id,
            Members::Query {
                query: String::new(),
                limit: 0,
            },
        )
    }

    pub(crate) fn query(ctx: &'a Context<B>, guild_id: GuildId, query: String) -> Self {
        Self::new(ctx, guild_id, Members::Query { query, limit: 100 })
    }

    pub(crate) fn users(ctx: &'a Context<B>, guild_id: GuildId, users: Vec<UserId>) -> Self {
        Self::new(ctx, guild_id, Members::Users(users))
    }

    const fn new(ctx: &'a Context<B>, guild_id: GuildId, members: Members) -> Self {
        Self {
            ctx,
            guild_id,
            members,
            presences: false,
            time: MEMBERS_TIME,
        }
    }

    /// The most members to return (for [`fetch_by_query`](super::MemberManager::fetch_by_query):
    /// 1–100, 100 by default; for [`fetch_all`](super::MemberManager::fetch_all): 0, every
    /// member, by default). No effect on [`fetch_many`](super::MemberManager::fetch_many).
    ///
    /// discord.js counterpart: the `limit` option.
    pub const fn limit(mut self, limit: u32) -> Self {
        if let Members::Query { limit: current, .. } = &mut self.members {
            *current = limit;
        }
        self
    }

    /// Also asks for the members' presences, which are cached (needs the privileged
    /// `GUILD_PRESENCES` intent).
    ///
    /// discord.js counterpart: the `withPresences` option.
    pub const fn presences(mut self, presences: bool) -> Self {
        self.presences = presences;
        self
    }

    /// How long to wait for the whole answer (120 seconds by default, as in discord.js).
    ///
    /// discord.js counterpart: the `time` option.
    pub const fn time(mut self, time: Duration) -> Self {
        self.time = time;
        self
    }

    async fn run(self) -> Result<Collection<UserId, Arc<CachedMember>>, GatewayRequestError> {
        let Self {
            ctx,
            guild_id,
            members,
            presences,
            time,
        } = self;
        let sender = ctx
            .guild_shard(guild_id)
            .ok_or(GatewayRequestError::ShardNotRunning(guild_id))?;
        let deadline = Instant::now().checked_add(time);
        let requests: Vec<RequestGuildMembers> = match members {
            Members::Query { query, limit } => {
                vec![RequestGuildMembers::query(guild_id, query, limit)]
            }
            Members::Users(users) => users
                .chunks(MAX_USER_IDS)
                .map(|users| RequestGuildMembers::users(guild_id, users.iter().copied()))
                .collect(),
        };
        let mut fetched = Collection::new();
        for mut request in requests {
            let nonce = ctx.collectors().nonce();
            let (tx, mut rx) = mpsc::unbounded_channel();
            let _registration = ctx.collectors().register(
                Key::Nonce(nonce),
                Some(guild_id),
                None,
                Sink::MemberChunks(tx),
            );
            request.presences = presences;
            request.nonce = Some(format!("{NONCE_PREFIX}{nonce}").into());
            sender.command(&request)?;
            let mut received = 0;
            loop {
                let reply = match deadline {
                    Some(deadline) => tokio::time::timeout_at(deadline, rx.recv())
                        .await
                        .map_err(|_| GatewayRequestError::Timeout)?,
                    None => rx.recv().await,
                };
                match reply {
                    Some(Signal::Item(MemberReply::Chunk(chunk))) => {
                        received += 1;
                        add_members(ctx, &chunk, &mut fetched);
                        if received >= chunk.chunk_count {
                            break;
                        }
                    }
                    Some(Signal::Item(MemberReply::RateLimited(retry_after))) => {
                        return Err(GatewayRequestError::RateLimited { retry_after });
                    }
                    Some(Signal::End(reason)) => return Err(GatewayRequestError::Ended(reason)),
                    None => return Err(GatewayRequestError::Ended(EndReason::Shutdown)),
                }
            }
        }
        Ok(fetched)
    }
}

/// Adds a chunk's members to `fetched`, as the cache stored them while handling the chunk
/// (converted here when the cache keeps no members).
fn add_members<B: Backend>(
    ctx: &Context<B>,
    chunk: &GuildMembersChunk,
    fetched: &mut Collection<UserId, Arc<CachedMember>>,
) {
    let cached = ctx.cache().map(|cache| cache.members(chunk.guild_id));
    for member in &chunk.members {
        let stored = cached
            .as_ref()
            .and_then(|members| members.get(member.user.id))
            .unwrap_or_else(|| ctx.store_member(chunk.guild_id, member));
        fetched.insert(member.user.id, stored);
    }
}

impl<'a, B: Backend> IntoFuture for FetchMembers<'a, B> {
    type Output = Result<Collection<UserId, Arc<CachedMember>>, GatewayRequestError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.run())
    }
}

/// The soundboard sounds of several guilds, by guild and sound.
pub type GuildSoundboardSounds =
    Collection<GuildId, Collection<SoundboardSoundId, Arc<SoundboardSound>>>;

/// Requests the soundboard sounds of `guild_ids` on their shards and waits up to `time` for
/// every answer.
pub(crate) async fn fetch_soundboard_sounds<B: Backend>(
    ctx: &Context<B>,
    guild_ids: Vec<GuildId>,
    time: Duration,
) -> Result<GuildSoundboardSounds, GatewayRequestError> {
    let deadline = Instant::now().checked_add(time);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut registrations = Vec::with_capacity(guild_ids.len());
    let mut by_shard: HashMap<u32, (djs_gateway::MessageSender, Vec<GuildId>)> = HashMap::new();
    let total = ctx.shard_id().total();
    for &guild_id in &guild_ids {
        let number = ShardId::for_guild(guild_id, total).number();
        let (_, guilds) = match by_shard.entry(number) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let sender = ctx
                    .guild_shard(guild_id)
                    .ok_or(GatewayRequestError::ShardNotRunning(guild_id))?;
                entry.insert((sender, Vec::new()))
            }
        };
        guilds.push(guild_id);
        registrations.push(ctx.collectors().register(
            Key::Sounds(guild_id),
            Some(guild_id),
            None,
            Sink::SoundboardSounds(tx.clone()),
        ));
    }
    drop(tx);
    for (sender, guilds) in by_shard.into_values() {
        sender.command(&RequestSoundboardSounds::new(guilds))?;
    }
    let mut fetched = GuildSoundboardSounds::new();
    while fetched.len() < guild_ids.len() {
        let reply = match deadline {
            Some(deadline) => tokio::time::timeout_at(deadline, rx.recv())
                .await
                .map_err(|_| GatewayRequestError::Timeout)?,
            None => rx.recv().await,
        };
        match reply {
            Some(Signal::Item(sounds)) => {
                let guild_id = sounds.guild_id;
                let stored = sounds
                    .soundboard_sounds
                    .iter()
                    .map(|sound| (sound.sound_id, ctx.store_soundboard_sound(guild_id, sound)))
                    .collect();
                fetched.insert(guild_id, stored);
            }
            Some(Signal::End(reason)) => return Err(GatewayRequestError::Ended(reason)),
            None => return Err(GatewayRequestError::Ended(EndReason::Shutdown)),
        }
    }
    drop(registrations);
    Ok(fetched)
}
