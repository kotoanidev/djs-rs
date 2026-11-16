//! The voice manager: the bot's calls, one per guild (D-319, D-322, D-324, D-340–D-343).

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};
use std::{collections::HashMap, sync::Arc};

use djs_model::{
    gateway::{DispatchEvent, payload::UpdateVoiceState},
    id::{ChannelId, GuildId, UserId},
};
use tokio::sync::oneshot;

use crate::{
    Call, JoinError, VoiceGateway,
    call::{CallCommand, Calls, Spawn, lock},
    codec::DEFAULT_BITRATE,
    connection::Config,
};

/// How a [`VoiceManager`] joins a channel: the bot's own mute and deafen flags (D-341,
/// D-343), and the bitrate of the audio it encodes (D-357). The default is deafened and not
/// muted, as discord.js's `joinVoiceChannel` does (a bot that only plays gets nobody's
/// audio), at 128 kb/s.
///
/// discord.js counterpart: the `selfDeaf` and `selfMute` options of `joinVoiceChannel`.
///
/// # Examples
///
/// ```
/// use djs_voice::JoinOptions;
///
/// // Hear the channel (to receive audio, later), and start muted.
/// let options = JoinOptions::new().self_deaf(false).self_mute(true);
/// assert_ne!(options, JoinOptions::default());
/// // Encode at 96 kb/s, a common voice-channel bitrate.
/// let options = JoinOptions::new().bitrate(96_000);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct JoinOptions {
    self_mute: bool,
    self_deaf: bool,
    bitrate: u32,
}

impl Default for JoinOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl JoinOptions {
    /// The default options: deafened, not muted, 128 kb/s.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            self_mute: false,
            self_deaf: true,
            bitrate: DEFAULT_BITRATE,
        }
    }

    /// Whether the bot mutes itself (others see it muted; it can still send audio, which
    /// Discord drops).
    #[must_use]
    pub const fn self_mute(mut self, self_mute: bool) -> Self {
        self.self_mute = self_mute;
        self
    }

    /// Whether the bot deafens itself: Discord sends it nobody's audio.
    #[must_use]
    pub const fn self_deaf(mut self, self_deaf: bool) -> Self {
        self.self_deaf = self_deaf;
        self
    }

    /// The bitrate, in bits per second, of the audio the call encodes (D-357): PCM, FFmpeg,
    /// and Ogg/Opus at a volume other than 1.0 (Ogg/Opus at 1.0 is sent as it is). 128 kb/s
    /// by default, as songbird; clamped to what libopus accepts, 500 b/s to 512 kb/s. A
    /// channel's own bitrate (8 to 96 kb/s, more with boosts) does not limit what the bot
    /// sends.
    ///
    /// Joining again with other options changes the bitrate of the call's next frames.
    ///
    /// songbird counterpart: `Call::set_bitrate`.
    #[must_use]
    pub const fn bitrate(mut self, bits_per_second: u32) -> Self {
        self.bitrate = bits_per_second;
        self
    }
}

/// The bot's voice calls, one per guild: joins voice channels and keeps the calls.
///
/// The manager sends Opcode 4 through a [`VoiceGateway`] and learns Discord's answers from
/// the main gateway's events, which the bot passes to [`process`](Self::process) (D-324).
/// djs-rs's `voice` feature does both and gives the manager as `client.voice()`. A cheap-clone
/// handle: clones share the calls.
///
/// discord.js counterpart: `joinVoiceChannel` and `getVoiceConnection` of `@discordjs/voice`;
/// songbird: `Songbird`.
///
/// # Examples
///
/// ```no_run
/// # use std::error::Error;
/// # use djs_model::{gateway::{DispatchEvent, payload::UpdateVoiceState}, id::{ChannelId, GuildId, UserId}};
/// # struct Shards;
/// # impl djs_voice::VoiceGateway for Shards {
/// #     fn send(&self, _: u32, _: &UpdateVoiceState) -> Result<(), Box<dyn Error + Send + Sync>> { Ok(()) }
/// # }
/// # async fn next_event() -> DispatchEvent { unimplemented!() }
/// # async fn run(bot: UserId, guild: GuildId, channel: ChannelId) -> Result<(), Box<dyn Error + Send + Sync>> {
/// use djs_voice::{Input, VoiceManager};
///
/// let manager = VoiceManager::new(bot, 1, Shards);
/// // Every event of the main gateway goes through `process`.
/// let events = manager.clone();
/// tokio::spawn(async move {
///     loop {
///         events.process(&next_event().await);
///     }
/// });
/// let call = manager.join(guild, channel).await?;
/// call.play(Input::ogg_file("hello.opus").await?);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct VoiceManager {
    inner: Arc<Inner>,
}

struct Inner {
    user_id: UserId,
    shard_count: u32,
    gateway: Arc<dyn VoiceGateway>,
    calls: Arc<Calls>,
    config: Config,
    next_call: AtomicU64,
}

impl fmt::Debug for VoiceManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceManager")
            .field("user_id", &self.inner.user_id)
            .field("shard_count", &self.inner.shard_count)
            .field("calls", &lock(&self.inner.calls).len())
            .finish_non_exhaustive()
    }
}

impl VoiceManager {
    /// A manager of the bot `bot_user_id`, whose main gateway has `shard_count` shards (0 is
    /// taken as 1), sending Opcode 4 through `gateway`.
    ///
    /// discord.js counterpart: none (the adapter of each `joinVoiceChannel` call).
    #[must_use]
    pub fn new(bot_user_id: UserId, shard_count: u32, gateway: impl VoiceGateway) -> Self {
        Self::with_config(bot_user_id, shard_count, gateway, Config::default())
    }

    pub(crate) fn with_config(
        bot_user_id: UserId,
        shard_count: u32,
        gateway: impl VoiceGateway,
        config: Config,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                user_id: bot_user_id,
                shard_count: shard_count.max(1),
                gateway: Arc::new(gateway),
                calls: Arc::new(Calls::new(HashMap::new())),
                config,
                next_call: AtomicU64::new(0),
            }),
        }
    }

    /// Takes in an event of the main gateway. Only the bot's own `VOICE_STATE_UPDATE`s and
    /// the `VOICE_SERVER_UPDATE`s of guilds with a call matter; everything else is ignored,
    /// so passing every event is fine.
    ///
    /// discord.js counterpart: `DiscordGatewayAdapterImplementerMethods#onVoiceStateUpdate`
    /// and `onVoiceServerUpdate`.
    pub fn process(&self, event: &DispatchEvent) {
        match event {
            DispatchEvent::VoiceStateUpdate(state) => {
                if state.user_id != self.inner.user_id {
                    return;
                }
                if let Some(call) = state.guild_id.and_then(|guild_id| self.get(guild_id)) {
                    call.command(CallCommand::State {
                        session_id: state.session_id.to_string(),
                        channel_id: state.channel_id,
                    });
                }
            }
            DispatchEvent::VoiceServerUpdate(server) => {
                if let Some(call) = self.get(server.guild_id) {
                    call.command(CallCommand::Server {
                        token: server.token.to_string(),
                        endpoint: server.endpoint.as_ref().map(ToString::to_string),
                    });
                }
            }
            _ => {}
        }
    }

    /// Joins the voice channel `channel_id` of `guild_id`, deafened and not muted
    /// ([`JoinOptions::default`]); returns the [`Call`] once it is connected (D-322, D-349).
    ///
    /// With end-to-end encryption (DAVE, which Discord requires), audio goes out once the bot
    /// is in the call's MLS group: about a second after someone else is in the channel. A
    /// bot alone in a channel has no group, so what it plays is not sent (nobody would hear
    /// it); nothing is ever sent without end-to-end encryption.
    ///
    /// When the bot already has a call in the guild, it moves to the channel, and the same
    /// call is returned (D-340).
    ///
    /// discord.js counterpart: `joinVoiceChannel` (which returns at once); songbird:
    /// `Songbird::join`.
    ///
    /// # Errors
    ///
    /// [`JoinError::Gateway`] when Opcode 4 cannot be sent, [`JoinError::Timeout`] when the
    /// call is not ready within 10 seconds (also when the bot lacks the Connect permission:
    /// Discord does not answer), [`JoinError::Connection`] when the voice connection fails,
    /// [`JoinError::Cancelled`] when the call is left or joined elsewhere meanwhile.
    pub async fn join(&self, guild_id: GuildId, channel_id: ChannelId) -> Result<Call, JoinError> {
        self.join_with(guild_id, channel_id, JoinOptions::default())
            .await
    }

    /// [`join`](Self::join) with `options`.
    ///
    /// # Errors
    ///
    /// As [`join`](Self::join).
    pub async fn join_with(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        options: JoinOptions,
    ) -> Result<Call, JoinError> {
        // A call that is just ending may still be in the map: its join is not answered,
        // and the next attempt makes a new call.
        let mut attempts = 0;
        loop {
            attempts += 1;
            let call = self.call(guild_id);
            let (done, result) = oneshot::channel();
            let bitrate = options.bitrate;
            if !call.command(CallCommand::Join {
                channel_id,
                bitrate,
                done,
            }) {
                self.forget(&call);
                continue;
            }
            let command = UpdateVoiceState::new(
                guild_id,
                Some(channel_id),
                options.self_mute,
                options.self_deaf,
            );
            if let Err(error) = self.inner.gateway.send(self.shard_id(guild_id), &command) {
                drop(result);
                call.command(CallCommand::Abandoned);
                return Err(JoinError::Gateway(error));
            }
            match result.await {
                Ok(Ok(())) => return Ok(call),
                Ok(Err(error)) => return Err(error),
                Err(_) if attempts < 2 => self.forget(&call),
                Err(_) => return Err(JoinError::Cancelled),
            }
        }
    }

    /// The bot's call in `guild_id`, if it has one.
    ///
    /// discord.js counterpart: `getVoiceConnection`; songbird: `Songbird::get`.
    #[must_use]
    pub fn get(&self, guild_id: GuildId) -> Option<Call> {
        lock(&self.inner.calls).get(&guild_id).cloned()
    }

    /// The bot's current calls, in no particular order.
    ///
    /// discord.js counterpart: `getVoiceConnections`; songbird: `Songbird::iter`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(manager: djs_voice::VoiceManager) {
    /// for call in manager.calls() {
    ///     println!("in {:?} of {}", call.channel_id(), call.guild_id());
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn calls(&self) -> Vec<Call> {
        lock(&self.inner.calls).values().cloned().collect()
    }

    /// The guild's call, made if there is none.
    fn call(&self, guild_id: GuildId) -> Call {
        lock(&self.inner.calls)
            .entry(guild_id)
            .or_insert_with(|| {
                Call::spawn(Spawn {
                    id: self.inner.next_call.fetch_add(1, Ordering::Relaxed),
                    guild_id,
                    user_id: self.inner.user_id,
                    shard_id: self.shard_id(guild_id),
                    gateway: Arc::clone(&self.inner.gateway),
                    config: self.inner.config.clone(),
                    calls: Arc::downgrade(&self.inner.calls),
                })
            })
            .clone()
    }

    /// Removes a call that ended from the map, if it is still there.
    fn forget(&self, call: &Call) {
        let mut calls = lock(&self.inner.calls);
        if calls
            .get(&call.guild_id())
            .is_some_and(|current| current.inner.id == call.inner.id)
        {
            calls.remove(&call.guild_id());
        }
    }

    /// The shard of a guild: `(guild_id >> 22) % shard_count`, as Discord documents.
    fn shard_id(&self, guild_id: GuildId) -> u32 {
        let shard = (guild_id.get() >> 22) % u64::from(self.inner.shard_count);
        // Below `shard_count`, a `u32`.
        u32::try_from(shard).unwrap_or_default()
    }
}

#[cfg(test)]
mod player_tests;
#[cfg(test)]
mod receive_tests;
#[cfg(test)]
mod tests;
