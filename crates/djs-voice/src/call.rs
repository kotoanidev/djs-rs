//! A call: the bot's voice connection in one guild, and what it plays (D-316, D-319–D-323,
//! D-335, D-337, D-340, D-344, D-351–D-361).
//!
//! A [`Call`] is a handle on a task that owns everything: the voice state the main gateway
//! reported, the voice connection (or the one being made), and the player. The handle's
//! methods send it commands (D-320). The task joins when both halves of the voice state
//! arrived (`VOICE_STATE_UPDATE` with the channel, then a `VOICE_SERVER_UPDATE`), connects
//! again to a new voice server when the bot is moved, and ends the call when the connection
//! ends for good. Its player plays the current track of the call's [queue](crate::queue)
//! (C-126).

use core::{
    fmt,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
    time::Duration,
};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    sync::{Arc, Mutex, MutexGuard, PoisonError, Weak},
};

use djs_model::{
    gateway::payload::UpdateVoiceState,
    id::{ChannelId, GuildId, UserId},
};
use futures_util::Stream;
use tokio::{
    sync::{
        mpsc::{self, error::TryRecvError},
        oneshot, watch,
    },
    task::JoinHandle,
    time::{Instant, sleep_until},
};

use crate::{
    Input, InputError, JoinError, TrackError, TrackHandle, TrackQueue, VoiceGateway,
    codec::{self, Decoder, Encoder, MAX_PACKET, MAX_PACKET_VALUES},
    connection::{
        self, Command, Config, ConnectError, ConnectionEvent, End, Handle, Info, Receiver, Sender,
    },
    input::{self as reading, Again, Chunk, Frame, History, Source},
    payload::{Event, MICROPHONE},
    queue::{Queue, Shared, Slot},
    receive::{self, ReceiveOptions, Receiving, VoiceStream},
    track::{Track, TrackEnd, TrackEvent, duration},
    udp::Rtp,
};

/// The close code of a bot disconnected from its channel (kicked, the channel deleted) or
/// moved to another: in the second case a new voice server follows.
const DISCONNECTED: u16 = 4014;
/// How long the call waits after a close with [`DISCONNECTED`] for the voice server of a
/// new channel, before it ends (as `@discordjs/voice`'s guide does).
const MOVE_GRACE: Duration = Duration::from_secs(5);
/// The frames of silence sent when audio stops, so that receivers do not interpolate
/// (Discord's "Voice Data Interpolation").
const SILENCE_FRAMES: u8 = 5;
/// The Opus packet of a 20 ms frame of silence.
const SILENCE: [u8; 3] = [0xF8, 0xFF, 0xFE];
/// How late a frame may be before the pace starts over from now, rather than catching up
/// with a burst.
const LATE: Duration = Duration::from_millis(100);
/// How many packets a track's input is read ahead: about a second of 20 ms packets.
const AHEAD: usize = 50;
/// How many Opus packets are still re-encoded at gain 1.0 after a change of gain, before
/// they go out as they are again: the listeners' decoders overlap each frame with the one
/// before, which must then hold the same audio (C-127).
const SETTLE: u8 = 2;

/// A chunk read from a track's input.
type Read = Option<Result<Chunk, InputError>>;

/// An Ogg/Opus track's packets being re-encoded (a gain other than 1.0: a volume, a fade),
/// in step with the packets as they are (C-127). The encoder's output comes its lookahead
/// behind its input, so it is given the audio that far ahead: `fifo` holds the decoded audio
/// from the start of the current frame plus the lookahead on.
struct Reencode {
    decoder: Decoder,
    fifo: VecDeque<f32>,
    /// Packets re-encoded at gain 1.0 since the gain last changed ([`SETTLE`]).
    steady: u8,
}

/// The calls of a manager, by guild.
pub(crate) type Calls = Mutex<HashMap<GuildId, Call>>;

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The bot's voice connection in one guild, from
/// [`VoiceManager::join`](crate::VoiceManager::join): it plays tracks, reports its events,
/// and leaves.
///
/// A cheap-clone handle (D-320): clones are the same call, and every method takes `&self`.
/// A lost connection is resumed or made again in the background (D-331), and a move to
/// another channel (by [`join`](crate::VoiceManager::join) or by a moderator) connects it
/// again; [`events`](Self::events) reports both. The call ends when the bot
/// [leaves](Self::leave), is disconnected, or its connection cannot be repaired
/// ([`CallEvent::Disconnected`]).
///
/// Discord documents voice connections at
/// <https://discord.com/developers/docs/topics/voice-connections>.
///
/// discord.js counterpart: `VoiceConnection` with a subscribed `AudioPlayer` of
/// `@discordjs/voice`; songbird: `Call`.
///
/// # Examples
///
/// ```no_run
/// # async fn run(
/// #     manager: djs_voice::VoiceManager,
/// #     guild: djs_model::id::GuildId,
/// #     channel: djs_model::id::ChannelId,
/// # ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// use djs_voice::{CallEvent, Input};
///
/// let call = manager.join(guild, channel).await?;
/// let mut events = call.events();
/// call.play(Input::ogg_file("intro.opus").await?);
/// while let Some(event) = events.next().await {
///     if let CallEvent::Disconnected(reason) = event {
///         println!("the call ended: {reason}");
///     }
/// }
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Call {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    /// Tells this call from a later one in the same guild.
    pub(crate) id: u64,
    guild_id: GuildId,
    shard_id: u32,
    gateway: Arc<dyn VoiceGateway>,
    /// The commands to the task, and the queue.
    shared: Arc<Shared>,
    subscribers: Arc<Mutex<Subscribers>>,
    channel: watch::Receiver<Option<ChannelId>>,
    tracks: AtomicU64,
}

#[derive(Default)]
struct Subscribers {
    senders: Vec<mpsc::UnboundedSender<CallEvent>>,
    /// The call ended: new streams end at once.
    closed: bool,
}

impl fmt::Debug for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call")
            .field("guild_id", &self.inner.guild_id)
            .field("channel_id", &self.channel_id())
            .finish_non_exhaustive()
    }
}

/// What a call's handles ask of its task.
pub(crate) enum CallCommand {
    /// Join or move to a channel: the manager sends Opcode 4 right after.
    Join {
        channel_id: ChannelId,
        /// The bitrate of encoded audio (D-357).
        bitrate: u32,
        done: oneshot::Sender<Result<(), JoinError>>,
    },
    /// A join gave up (Opcode 4 could not be sent): forget its closed waiter.
    Abandoned,
    /// The bot's `VOICE_STATE_UPDATE` in this guild.
    State {
        session_id: String,
        channel_id: Option<ChannelId>,
    },
    /// A `VOICE_SERVER_UPDATE` of this guild.
    Server {
        token: String,
        endpoint: Option<String>,
    },
    /// The queue or a track changed: play the current track as it is now.
    Wake,
    /// Play the track `id` from `samples` (D-359).
    Seek {
        id: u64,
        samples: u64,
        done: oneshot::Sender<Result<Duration, TrackError>>,
    },
    Leave {
        done: oneshot::Sender<()>,
    },
    /// Receive a user's audio (D-363).
    Subscribe {
        user_id: UserId,
        options: ReceiveOptions,
        tx: mpsc::Sender<receive::Item>,
    },
}

/// Where the guild's shard is, and what the call is to the manager.
pub(crate) struct Spawn {
    pub(crate) id: u64,
    pub(crate) guild_id: GuildId,
    pub(crate) user_id: UserId,
    pub(crate) shard_id: u32,
    pub(crate) gateway: Arc<dyn VoiceGateway>,
    pub(crate) config: Config,
    pub(crate) calls: Weak<Calls>,
}

impl Call {
    /// A new call in a guild, and its task.
    pub(crate) fn spawn(spawn: Spawn) -> Self {
        let (commands, commands_rx) = mpsc::unbounded_channel();
        let (channel, channel_rx) = watch::channel(None);
        let subscribers = Arc::new(Mutex::new(Subscribers::default()));
        let shared = Arc::new(Shared {
            commands,
            queue: Mutex::new(Queue::default()),
        });
        let task = Task {
            call_id: spawn.id,
            guild_id: spawn.guild_id,
            user_id: spawn.user_id,
            config: spawn.config,
            calls: spawn.calls,
            subscribers: Arc::clone(&subscribers),
            channel,
            commands: commands_rx,
            voice: VoiceState::default(),
            fresh_server: false,
            live: None,
            connecting: None,
            join: None,
            grace: None,
            removed: false,
            connected_once: false,
            leave: None,
            player: Player::new(Arc::clone(&shared)),
            receiving: Receiving::default(),
        };
        tokio::spawn(task.run());
        Self {
            inner: Arc::new(Inner {
                id: spawn.id,
                guild_id: spawn.guild_id,
                shard_id: spawn.shard_id,
                gateway: spawn.gateway,
                shared,
                subscribers,
                channel: channel_rx,
                tracks: AtomicU64::new(0),
            }),
        }
    }

    /// Sends the task a command; `false` once it has ended.
    pub(crate) fn command(&self, command: CallCommand) -> bool {
        self.inner.shared.commands.send(command).is_ok()
    }

    /// The guild of the call.
    ///
    /// discord.js counterpart: `VoiceConnection#joinConfig.guildId`.
    #[must_use]
    pub fn guild_id(&self) -> GuildId {
        self.inner.guild_id
    }

    /// The voice channel the call is connected to; `None` while it is not (connecting again
    /// after a move, or ended).
    ///
    /// discord.js counterpart: `VoiceConnection#joinConfig.channelId`.
    #[must_use]
    pub fn channel_id(&self) -> Option<ChannelId> {
        *self.inner.channel.borrow()
    }

    /// Plays `input` now, replacing the current track (D-335, D-352): it ends
    /// ([`TrackEnd::Replaced`]), and the tracks waiting in the [queue](Self::queue) play
    /// after this one. Returns at once; the input is read and sent in the background, paced
    /// in real time.
    ///
    /// While the call is connecting again (after a move or a lost connection) the track
    /// keeps its pace and its audio is not sent; with end-to-end encryption (DAVE), audio is
    /// sent once the bot is in the call's group (D-349).
    ///
    /// discord.js counterpart: `AudioPlayer#play` of a player subscribed to the connection;
    /// songbird: `Call::play_only_input`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(call: djs_voice::Call) -> Result<(), djs_voice::InputError> {
    /// let track = call.play(djs_voice::Input::ogg_file("song.opus").await?);
    /// # Ok(()) }
    /// ```
    #[allow(
        clippy::must_use_candidate,
        reason = "playing without keeping the handle is fine"
    )]
    pub fn play(&self, input: Input) -> TrackHandle {
        self.add(input, true)
    }

    /// Adds `input` to the end of the call's [queue](Self::queue) (D-352): it plays when the
    /// tracks before it have ended, at once when nothing plays. Returns its handle at once;
    /// until its turn it is [`PlayState::Queued`](crate::PlayState::Queued), and FFmpeg is
    /// not run for it yet.
    ///
    /// discord.js counterpart: none (a bot keeps its own queue); songbird:
    /// `Call::enqueue_input`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn run(call: djs_voice::Call) {
    /// use djs_voice::Input;
    ///
    /// for song in ["one.mp3", "two.mp3", "three.mp3"] {
    ///     call.enqueue(Input::ffmpeg(song));
    /// }
    /// # }
    /// ```
    #[allow(
        clippy::must_use_candidate,
        reason = "queueing without keeping the handle is fine"
    )]
    pub fn enqueue(&self, input: Input) -> TrackHandle {
        self.add(input, false)
    }

    fn add(&self, input: Input, now: bool) -> TrackHandle {
        let id = self.inner.tracks.fetch_add(1, Ordering::Relaxed);
        let track = Track::new(id, input.seekable(), Arc::downgrade(&self.inner.shared));
        let handle = TrackHandle::new(Arc::clone(&track));
        let slot = Slot {
            track,
            input: Some(input),
        };
        let mut queue = lock(&self.inner.shared.queue);
        if queue.closed {
            slot.track.end(TrackEnd::CallEnded);
            return handle;
        }
        if now {
            if let Some(old) = queue.current.replace(slot) {
                old.track.end(TrackEnd::Replaced);
            }
        } else if queue.current.is_none() {
            queue.current = Some(slot);
        } else {
            queue.waiting.push_back(slot);
        }
        drop(queue);
        self.inner.shared.wake();
        handle
    }

    /// The call's queue of tracks (D-352): skip the current one, clear the waiting ones,
    /// see what plays.
    ///
    /// songbird counterpart: `Call::queue`.
    ///
    /// discord.js counterpart: none (`@discordjs/voice` has no queue).
    #[must_use]
    pub fn queue(&self) -> TrackQueue {
        TrackQueue {
            shared: Arc::clone(&self.inner.shared),
        }
    }

    /// Receives the audio of the user `user_id` (D-363, D-367): a stream of their packets,
    /// as Opus, in order, ended manually ([`subscribe_with`](Self::subscribe_with) chooses).
    ///
    /// Discord sends a bot nobody's audio while it is deafened, which is how
    /// [`VoiceManager::join`](crate::VoiceManager::join) joins: join with
    /// [`JoinOptions::self_deaf(false)`](crate::JoinOptions::self_deaf) to receive. The
    /// stream may be made before the user speaks or even joins; packets come once the voice
    /// gateway has said which source is theirs (the first time they speak). Each stream is
    /// its own, with a copy of the packets (D-373); dropping it unsubscribes. It ends with
    /// the call.
    ///
    /// discord.js counterpart: `VoiceReceiver#subscribe`; songbird: the `VoiceTick` event.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(manager: djs_voice::VoiceManager, guild_id: djs_model::id::GuildId,
    /// #     channel_id: djs_model::id::ChannelId, user_id: djs_model::id::UserId)
    /// #     -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// use djs_voice::{JoinOptions, VoiceData};
    ///
    /// let options = JoinOptions::new().self_deaf(false);
    /// let call = manager.join_with(guild_id, channel_id, options).await?;
    /// let mut voice = call.subscribe(user_id);
    /// let mut bytes = 0;
    /// while let Some(packet) = voice.next().await {
    ///     if let VoiceData::Opus(opus) = packet.audio {
    ///         bytes += opus.len();
    ///     }
    /// }
    /// println!("{bytes} bytes of Opus received");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn subscribe(&self, user_id: UserId) -> VoiceStream {
        self.subscribe_with(user_id, ReceiveOptions::new())
    }

    /// Receives the audio of the user `user_id` with `options` (D-367, D-372): decoded to
    /// PCM or not, ended by itself or not. Otherwise as [`subscribe`](Self::subscribe).
    ///
    /// discord.js counterpart: `VoiceReceiver#subscribe` with its options.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(call: djs_voice::Call, user_id: djs_model::id::UserId) {
    /// use std::time::Duration;
    ///
    /// use djs_voice::{EndBehavior, ReceiveOptions, VoiceData};
    ///
    /// // One utterance, decoded: it ends after a second of silence.
    /// let options = ReceiveOptions::new()
    ///     .decode(true)
    ///     .end(EndBehavior::AfterSilence(Duration::from_secs(1)));
    /// let mut voice = call.subscribe_with(user_id, options);
    /// let mut samples: Vec<i16> = Vec::new();
    /// while let Some(packet) = voice.next().await {
    ///     if let VoiceData::Pcm(pcm) = packet.audio {
    ///         samples.extend(pcm);
    ///     }
    /// }
    /// println!("{} s of audio", samples.len() / 2 / 48_000);
    /// # }
    /// ```
    #[must_use]
    pub fn subscribe_with(&self, user_id: UserId, options: ReceiveOptions) -> VoiceStream {
        let (tx, rx) = mpsc::channel(receive::QUEUE);
        // A call that has ended drops `tx`: the stream ends at once.
        self.command(CallCommand::Subscribe {
            user_id,
            options,
            tx,
        });
        VoiceStream::new(rx, options)
    }

    /// Returns a stream of the call's events: connections resumed and made again, users
    /// joining, leaving and speaking, and the end of the call. Each stream gets every event
    /// from its creation on, and ends with the call.
    ///
    /// discord.js counterpart: `VoiceConnection#on('stateChange')`; songbird:
    /// `Call::add_global_event`.
    #[must_use]
    pub fn events(&self) -> CallEvents {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut subscribers = lock(&self.inner.subscribers);
        if !subscribers.closed {
            subscribers.senders.push(tx);
        }
        CallEvents { rx }
    }

    /// Leaves the voice channel: sends Opcode 4 without a channel, closes the connection
    /// and ends the call ([`DisconnectReason::Left`]); returns once it has ended.
    ///
    /// discord.js counterpart: `VoiceConnection#destroy`; songbird: `Call::leave`.
    ///
    /// # Errors
    ///
    /// When the [`VoiceGateway`] could not send Opcode 4: the call still ends, but Discord
    /// may show the bot in the channel until its gateway session ends.
    pub async fn leave(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let command = UpdateVoiceState::new(self.inner.guild_id, None, false, false);
        let sent = self.inner.gateway.send(self.inner.shard_id, &command);
        let (done, ended) = oneshot::channel();
        if self.command(CallCommand::Leave { done }) {
            // An error only means the call already ended.
            let _ = ended.await;
        }
        sent
    }
}

/// An event of a [`Call`], from [`Call::events`] (D-321, D-337).
///
/// discord.js counterpart: the state changes of `VoiceConnection` and the events of its
/// receiver's `SpeakingMap`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum CallEvent {
    /// The connection was lost and resumed: nothing was missed.
    Resumed,
    /// The connection was made again: after a lost session, or to the voice server of a
    /// channel a moderator moved the bot to. Audio goes on.
    Reconnected,
    /// The call ended, for this reason; the stream ends after it.
    Disconnected(DisconnectReason),
    /// A user joined the voice channel (or was in it when the bot joined).
    UserJoined(UserId),
    /// A user left the voice channel.
    UserLeft(UserId),
    /// A user started speaking with this SSRC: the RTP source of their audio.
    #[non_exhaustive]
    Speaking {
        /// The user.
        user_id: UserId,
        /// Their audio's synchronization source.
        ssrc: u32,
    },
    /// A user's audio started coming (D-369): their first packet after none for 100 ms.
    /// Discord sends none to a deafened bot (see [`Call::subscribe`]).
    ///
    /// discord.js counterpart: the `start` event of `VoiceReceiver#speaking`.
    SpeakingStarted(UserId),
    /// 100 ms have passed since a user's last packet (D-369).
    ///
    /// discord.js counterpart: the `end` event of `VoiceReceiver#speaking`.
    SpeakingStopped(UserId),
}

/// Why a call ended (D-344), in [`CallEvent::Disconnected`].
///
/// discord.js counterpart: `VoiceConnectionDisconnectReason` of `@discordjs/voice`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DisconnectReason {
    /// The bot left with [`Call::leave`].
    Left,
    /// The voice gateway closed the connection with a code after which resuming cannot
    /// help: 4014 when the bot was disconnected from the channel (kicked, the channel
    /// deleted), 4022 when the call was terminated, …
    #[non_exhaustive]
    Closed {
        /// The close code; Discord lists them at
        /// <https://discord.com/developers/docs/topics/opcodes-and-status-codes#voice-voice-close-event-codes>.
        code: u16,
    },
    /// Every attempt to resume or connect again failed (D-331); the last failure is the
    /// [`source`](Error::source).
    Lost(Arc<dyn Error + Send + Sync>),
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => f.write_str("the bot left the call"),
            Self::Closed { code } => {
                write!(f, "the voice gateway closed the call with code {code}")
            }
            Self::Lost(_) => f.write_str("the voice connection was lost"),
        }
    }
}

impl Error for DisconnectReason {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Lost(error) => Some(&**error),
            Self::Left | Self::Closed { .. } => None,
        }
    }
}

/// The stream of a [`Call`]'s events. Created by [`Call::events`].
///
/// Implements [`Stream`]; [`next`](Self::next) is provided so no extension trait is needed.
///
/// discord.js counterpart: `VoiceConnection#on` (`@discordjs/voice`).
#[derive(Debug)]
pub struct CallEvents {
    rx: mpsc::UnboundedReceiver<CallEvent>,
}

impl CallEvents {
    /// Waits for the next event. Returns `None` once the call has ended (after
    /// [`CallEvent::Disconnected`]).
    ///
    /// discord.js counterpart: `VoiceConnection#on` (`@discordjs/voice`).
    pub async fn next(&mut self) -> Option<CallEvent> {
        self.rx.recv().await
    }
}

impl Stream for CallEvents {
    type Item = CallEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<CallEvent>> {
        self.rx.poll_recv(cx)
    }
}

/// The voice state the main gateway reported for the bot in this guild.
#[derive(Default)]
struct VoiceState {
    session_id: Option<String>,
    channel_id: Option<ChannelId>,
    /// The token and the endpoint of the voice server.
    server: Option<(String, String)>,
}

/// A connected voice connection.
struct Live {
    channel_id: ChannelId,
    handle: Handle,
    sender: Sender,
    receiver: Receiver,
    events: mpsc::UnboundedReceiver<ConnectionEvent>,
}

type Connected = (Handle, Sender, mpsc::UnboundedReceiver<ConnectionEvent>);

/// A voice connection being made.
struct Connecting {
    channel_id: ChannelId,
    task: JoinHandle<Result<Connected, ConnectError>>,
}

/// A join waiting for the voice state of its channel.
struct PendingJoin {
    channel_id: ChannelId,
    waiters: Vec<oneshot::Sender<Result<(), JoinError>>>,
    deadline: Instant,
    /// The `VOICE_STATE_UPDATE` of the channel arrived.
    state: bool,
    /// A `VOICE_SERVER_UPDATE` arrived.
    server: bool,
}

/// What woke the task.
enum Step {
    Command(Option<CallCommand>),
    Connection(Option<ConnectionEvent>),
    Connected(ChannelId, Result<Connected, ConnectError>),
    JoinTimeout,
    GraceOver,
    Chunk(Option<Result<Chunk, InputError>>),
    Frame,
    /// An RTP packet of others' audio.
    Packet(Rtp),
    /// Something of [`Receiving`] is due.
    Receive,
}

/// How the call ends: the event reported (none for a call that never connected), and what
/// a join still waiting gets.
struct Ending {
    reason: Option<DisconnectReason>,
    join: JoinError,
}

impl Ending {
    const fn new(reason: Option<DisconnectReason>, join: JoinError) -> Self {
        Self { reason, join }
    }
}

/// A call's task.
struct Task {
    call_id: u64,
    guild_id: GuildId,
    user_id: UserId,
    config: Config,
    calls: Weak<Calls>,
    subscribers: Arc<Mutex<Subscribers>>,
    channel: watch::Sender<Option<ChannelId>>,
    commands: mpsc::UnboundedReceiver<CallCommand>,
    voice: VoiceState,
    /// A `VOICE_SERVER_UPDATE` arrived outside a join: connect to it.
    fresh_server: bool,
    live: Option<Live>,
    connecting: Option<Connecting>,
    join: Option<PendingJoin>,
    /// After a close with [`DISCONNECTED`]: until when a move may still come.
    grace: Option<Instant>,
    /// The bot's voice state has no channel: it was disconnected.
    removed: bool,
    connected_once: bool,
    /// The [`Call::leave`] waiting for the end.
    leave: Option<oneshot::Sender<()>>,
    player: Player,
    receiving: Receiving,
}

impl Task {
    async fn run(mut self) {
        loop {
            let join_deadline = self
                .join
                .as_ref()
                .filter(|_| self.connecting.is_none())
                .map(|join| join.deadline);
            // A connection being made has its own timeout.
            let grace = self.grace.filter(|_| self.connecting.is_none());
            let due = self.player.due();
            let wants_chunk = self.player.wants_chunk();
            let receive = self.receiving.deadline();
            let step = tokio::select! {
                command = self.commands.recv() => Step::Command(command),
                step = live_step(&mut self.live) => step,
                (channel_id, result) = connected(&mut self.connecting) => {
                    Step::Connected(channel_id, result)
                }
                () = wait(join_deadline) => Step::JoinTimeout,
                () = wait(grace) => Step::GraceOver,
                chunk = self.player.recv(), if wants_chunk => Step::Chunk(chunk),
                () = wait(due) => Step::Frame,
                () = wait(receive) => Step::Receive,
            };
            if let Err(ending) = self.step(step).await {
                self.end(ending);
                return;
            }
        }
    }

    async fn step(&mut self, step: Step) -> Result<(), Ending> {
        match step {
            Step::Command(None) => {
                // Every handle and the manager are gone.
                Err(Ending::new(None, JoinError::Cancelled))
            }
            Step::Command(Some(command)) => self.command(command),
            Step::Connection(event) => self.connection_event(event),
            Step::Connected(channel_id, result) => self.connected(channel_id, result),
            Step::JoinTimeout => {
                let join = self.join.take();
                if let Err(ending) = self.idle(|| JoinError::Timeout) {
                    // `end` answers it, once the call is out of the manager.
                    self.join = join;
                    return Err(ending);
                }
                if let Some(join) = join {
                    answer(join.waiters, || Err(JoinError::Timeout));
                }
                Ok(())
            }
            Step::GraceOver => Err(Ending::new(
                Some(DisconnectReason::Closed { code: DISCONNECTED }),
                JoinError::Cancelled,
            )),
            Step::Chunk(chunk) => {
                self.player.received(chunk);
                Ok(())
            }
            Step::Frame => {
                self.player.frame(self.live.as_mut()).await;
                Ok(())
            }
            Step::Packet(rtp) => {
                self.packet(rtp);
                Ok(())
            }
            Step::Receive => {
                for user_id in self.receiving.tick(Instant::now()) {
                    self.emit(&CallEvent::SpeakingStopped(user_id));
                }
                Ok(())
            }
        }
    }

    /// An RTP packet of someone's audio came: they speak; it is opened for those who
    /// receive them.
    fn packet(&mut self, rtp: Rtp) {
        let now = Instant::now();
        let Some(arrival) = self.receiving.arrived(rtp, now) else {
            return;
        };
        if arrival.started {
            self.emit(&CallEvent::SpeakingStarted(arrival.user_id));
        }
        if arrival.wanted
            && let Some(live) = &mut self.live
            && let Some(opus) = live.receiver.open(arrival.user_id.get())
        {
            self.receiving.opened(arrival.user_id, rtp, opus, now);
        }
    }

    fn command(&mut self, command: CallCommand) -> Result<(), Ending> {
        match command {
            CallCommand::Join {
                channel_id,
                bitrate,
                done,
            } => {
                self.player.set_bitrate(bitrate);
                self.start_join(channel_id, done);
            }
            CallCommand::Abandoned => {
                if let Some(join) = &mut self.join {
                    join.waiters.retain(|waiter| !waiter.is_closed());
                    if join.waiters.is_empty() {
                        self.join = None;
                    }
                }
                return self.idle(|| JoinError::Cancelled);
            }
            CallCommand::State {
                session_id,
                channel_id,
            } => {
                self.voice.session_id = Some(session_id);
                self.voice.channel_id = channel_id;
                if let Some(channel_id) = channel_id {
                    self.removed = false;
                    if let Some(join) = &mut self.join
                        && join.channel_id == channel_id
                    {
                        join.state = true;
                    }
                    self.connect();
                } else if self.join.is_none() {
                    // Disconnected from the channel (kicked, the channel deleted): the voice
                    // gateway closes with 4014, or already did.
                    self.removed = true;
                    if self.grace.is_some() {
                        return Err(Ending::new(
                            Some(DisconnectReason::Closed { code: DISCONNECTED }),
                            JoinError::Cancelled,
                        ));
                    }
                }
            }
            CallCommand::Server { token, endpoint } => match endpoint {
                // The voice server is being reallocated: the next update says where.
                None => self.voice.server = None,
                Some(endpoint) => {
                    self.voice.server = Some((token, endpoint));
                    match &mut self.join {
                        Some(join) => join.server = true,
                        None => self.fresh_server = true,
                    }
                    self.connect();
                }
            },
            CallCommand::Wake => self.player.reconcile(),
            CallCommand::Seek { id, samples, done } => self.player.seek(id, samples, done),
            CallCommand::Leave { done } => {
                self.leave = Some(done);
                return Err(Ending::new(
                    Some(DisconnectReason::Left),
                    JoinError::Cancelled,
                ));
            }
            CallCommand::Subscribe {
                user_id,
                options,
                tx,
            } => self.receiving.subscribe(user_id, options, tx),
        }
        Ok(())
    }

    fn start_join(&mut self, channel_id: ChannelId, done: oneshot::Sender<Result<(), JoinError>>) {
        let already_there = self
            .live
            .as_ref()
            .is_some_and(|live| live.channel_id == channel_id);
        if already_there && self.connecting.is_none() && self.join.is_none() {
            let _ = done.send(Ok(()));
            return;
        }
        match &mut self.join {
            Some(join) if join.channel_id == channel_id => join.waiters.push(done),
            _ => {
                if let Some(join) = self.join.take() {
                    answer(join.waiters, || Err(JoinError::Cancelled));
                }
                if let Some(connecting) = self.connecting.take() {
                    connecting.task.abort();
                }
                self.join = Some(PendingJoin {
                    channel_id,
                    waiters: vec![done],
                    deadline: Instant::now() + self.config.timeout,
                    state: false,
                    server: false,
                });
            }
        }
    }

    /// Connects to the voice server reported, when the voice state is complete: for a join,
    /// once both updates of its channel arrived; otherwise on each new voice server.
    fn connect(&mut self) {
        let VoiceState {
            session_id: Some(session_id),
            channel_id: Some(channel_id),
            server: Some((token, endpoint)),
        } = &self.voice
        else {
            return;
        };
        let channel_id = *channel_id;
        let timeout = match &self.join {
            Some(join) if join.channel_id == channel_id && join.state && join.server => {
                join.deadline.saturating_duration_since(Instant::now())
            }
            Some(_) => return,
            None => {
                if !core::mem::take(&mut self.fresh_server) {
                    return;
                }
                self.config.timeout
            }
        };
        let info = Info {
            guild_id: self.guild_id,
            channel_id,
            user_id: self.user_id,
            session_id: session_id.clone(),
            endpoint: endpoint.clone(),
            token: token.clone(),
        };
        let config = Config {
            timeout,
            ..self.config.clone()
        };
        if let Some(old) = self.connecting.take() {
            old.task.abort();
        }
        self.connecting = Some(Connecting {
            channel_id,
            task: tokio::spawn(connection::connect(info, config)),
        });
    }

    fn connected(
        &mut self,
        channel_id: ChannelId,
        result: Result<Connected, ConnectError>,
    ) -> Result<(), Ending> {
        match result {
            Ok((handle, sender, events)) => {
                if let Some(old) = self.live.take() {
                    // Its end is not this call's.
                    old.handle.send(Command::Close);
                }
                // Other users, other sources.
                for user_id in self.receiving.reset() {
                    self.emit(&CallEvent::SpeakingStopped(user_id));
                }
                let receiver = sender.receiver();
                self.live = Some(Live {
                    channel_id,
                    handle,
                    sender,
                    receiver,
                    events,
                });
                self.channel.send_replace(Some(channel_id));
                self.grace = None;
                // A new connection has no speaking state yet.
                self.player.speaking = false;
                match self.join.take() {
                    Some(join) => answer(join.waiters, || Ok(())),
                    None if self.connected_once => self.emit(&CallEvent::Reconnected),
                    None => {}
                }
                self.connected_once = true;
                Ok(())
            }
            Err(error) => {
                let error = Arc::new(error);
                let join_error = {
                    let error = Arc::clone(&error);
                    move || match &*error {
                        ConnectError::Timeout => JoinError::Timeout,
                        _ => JoinError::Connection(Box::new(Arc::clone(&error))),
                    }
                };
                // A move that failed ends the call, unless the old connection is still there.
                self.grace = None;
                if self.live.is_some() {
                    if let Some(join) = self.join.take() {
                        answer(join.waiters, || Err(join_error()));
                    }
                    tracing::warn!(guild = %self.guild_id, %error, "connecting the call again failed");
                    return Ok(());
                }
                // `end` answers a join, once the call is out of the manager.
                let reason = self
                    .connected_once
                    .then(|| DisconnectReason::Lost(error as Arc<dyn Error + Send + Sync>));
                Err(Ending::new(reason, join_error()))
            }
        }
    }

    fn connection_event(&mut self, event: Option<ConnectionEvent>) -> Result<(), Ending> {
        let end = match event {
            Some(ConnectionEvent::Gateway(event)) => {
                match event {
                    Event::ClientsConnect(users) => {
                        for user in users {
                            self.emit(&CallEvent::UserJoined(user));
                        }
                    }
                    Event::ClientDisconnect(user) => {
                        self.receiving.left(user);
                        self.emit(&CallEvent::UserLeft(user));
                    }
                    Event::Speaking(speaking) => {
                        self.receiving
                            .map(speaking.user_id, speaking.ssrc, Instant::now());
                        self.emit(&CallEvent::Speaking {
                            user_id: speaking.user_id,
                            ssrc: speaking.ssrc,
                        });
                    }
                    _ => {}
                }
                return Ok(());
            }
            Some(ConnectionEvent::Resumed) => {
                self.emit(&CallEvent::Resumed);
                return Ok(());
            }
            Some(ConnectionEvent::Reconnected) => {
                self.emit(&CallEvent::Reconnected);
                return Ok(());
            }
            Some(ConnectionEvent::Ended(end)) => end,
            None => End::Lost(ConnectError::Closed(None)),
        };
        self.live = None;
        self.channel.send_replace(None);
        if self.join.is_some() || self.connecting.is_some() {
            // A move is under way: the new connection takes over.
            return Ok(());
        }
        match end {
            End::Stopped(DISCONNECTED) if !self.removed => {
                // Moved to another channel, maybe: its voice server would follow.
                self.grace = Some(Instant::now() + MOVE_GRACE);
                Ok(())
            }
            End::Stopped(code) => Err(Ending::new(
                Some(DisconnectReason::Closed { code }),
                JoinError::Cancelled,
            )),
            End::Lost(error) => Err(Ending::new(
                Some(DisconnectReason::Lost(Arc::new(error))),
                JoinError::Cancelled,
            )),
            End::Closed => Err(Ending::new(None, JoinError::Cancelled)),
        }
    }

    /// Ends a call left with nothing to do: no connection, none being made, no join.
    fn idle(&self, join: impl FnOnce() -> JoinError) -> Result<(), Ending> {
        if self.live.is_none()
            && self.connecting.is_none()
            && self.join.is_none()
            && self.grace.is_none()
        {
            let reason = self.connected_once.then(|| {
                DisconnectReason::Lost(
                    Arc::new(ConnectError::Timeout) as Arc<dyn Error + Send + Sync>
                )
            });
            return Err(Ending::new(reason, join()));
        }
        Ok(())
    }

    fn emit(&self, event: &CallEvent) {
        lock(&self.subscribers)
            .senders
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }

    /// Ends the call: out of the manager first (a new join makes a new call), then the
    /// connection closed, the event reported, the waiters answered.
    fn end(mut self, ending: Ending) {
        if let Some(calls) = self.calls.upgrade() {
            let mut calls = lock(&calls);
            if calls
                .get(&self.guild_id)
                .is_some_and(|call| call.inner.id == self.call_id)
            {
                calls.remove(&self.guild_id);
            }
        }
        if let Some(connecting) = self.connecting.take() {
            connecting.task.abort();
        }
        if let Some(live) = self.live.take() {
            live.handle.send(Command::Close);
        }
        lock(&self.player.shared.queue).close();
        self.player.end();
        self.channel.send_replace(None);
        {
            let mut subscribers = lock(&self.subscribers);
            if let Some(reason) = ending.reason {
                let event = CallEvent::Disconnected(reason);
                for subscriber in &subscribers.senders {
                    let _ = subscriber.send(event.clone());
                }
            }
            subscribers.senders.clear();
            subscribers.closed = true;
        }
        if let Some(join) = self.join.take() {
            let mut join_error = Some(ending.join);
            answer(join.waiters, || {
                Err(join_error.take().unwrap_or(JoinError::Cancelled))
            });
        }
        if let Some(leave) = self.leave.take() {
            let _ = leave.send(());
        }
    }
}

/// Answers the waiters of a join.
fn answer<T>(waiters: Vec<oneshot::Sender<T>>, mut result: impl FnMut() -> T) {
    for waiter in waiters {
        let _ = waiter.send(result());
    }
}

/// What the connection gives next: one of its events, or a packet of others' audio.
async fn live_step(live: &mut Option<Live>) -> Step {
    let Some(live) = live else {
        return core::future::pending().await;
    };
    // Both are cancel-safe.
    tokio::select! {
        event = live.events.recv() => Step::Connection(event),
        rtp = live.receiver.recv() => Step::Packet(rtp),
    }
}

async fn connected(
    connecting: &mut Option<Connecting>,
) -> (ChannelId, Result<Connected, ConnectError>) {
    let Some(pending) = connecting else {
        return core::future::pending().await;
    };
    let result = (&mut pending.task)
        .await
        .unwrap_or(Err(ConnectError::Unexpected("the connection task failed")));
    let channel_id = pending.channel_id;
    *connecting = None;
    (channel_id, result)
}

/// Waits until `deadline`, or forever without one.
async fn wait(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => core::future::pending().await,
    }
}

/// Reads `source` from `start` in a task of its own, about a second ahead.
fn spawn_reader(
    source: Source,
    start: u64,
) -> (mpsc::Receiver<Result<Chunk, InputError>>, JoinHandle<()>) {
    let (tx, chunks) = mpsc::channel(AHEAD);
    (chunks, tokio::spawn(reading::read(source, start, tx)))
}

/// The track the player plays: its input read ahead, and what the player knows of it.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about the track, each changed on its own"
)]
struct Playing {
    track: Arc<Track>,
    /// How to open its input again, for loops and seeking; `None` for a reader.
    again: Option<Again>,
    chunks: mpsc::Receiver<Result<Chunk, InputError>>,
    reader: JoinHandle<()>,
    /// While its Opus packets are re-encoded.
    reencode: Option<Reencode>,
    /// The input's last Opus packets (those before a seek's position, after one): a new
    /// decoder decodes them first.
    history: History,
    /// The input opened again while the listeners got the encoder's packets: its stream
    /// goes on, unprimed (C-127).
    continuing: bool,
    /// Whether the input's last Opus packet was CELT-only ([`codec::celt_only`]).
    celt: bool,
    /// Frames read ahead of `Player::next` for the re-encoding's lookahead, then what came
    /// after them (an error, the end), returned by `Player::recv` in order.
    after: VecDeque<Frame>,
    held: Option<Read>,
    /// How many frames from `Player::next` on are decoded already (into the `fifo`).
    decoded: usize,
    /// Paused, as the player last saw it.
    paused: bool,
    /// [`TrackEvent::Playing`] was sent since its audio last stopped.
    announced: bool,
    /// Seeks waiting for the first frame from the new place.
    seeks: Vec<oneshot::Sender<Result<Duration, TrackError>>>,
    /// The gain at the end of the last frame sent: the next one ramps from it (C-127).
    gain: f32,
    /// The next frame fades out to silence: the track pauses or seeks.
    closing: bool,
    /// A seek waiting for the frame from before it to fade out: the input is opened again
    /// at this position then.
    seek_to: Option<u64>,
    /// Its input is being opened again (a seek, a loop): frames of silence keep the stream
    /// going until its audio comes.
    restarting: bool,
}

impl Playing {
    fn new(track: Arc<Track>, source: Source, again: Option<Again>) -> Self {
        let (chunks, reader) = spawn_reader(source, track.position());
        Self {
            gain: track.volume(),
            paused: track.paused(),
            track,
            again,
            chunks,
            reader,
            reencode: None,
            history: History::default(),
            continuing: false,
            celt: false,
            after: VecDeque::new(),
            held: None,
            decoded: 0,
            announced: false,
            seeks: Vec::new(),
            closing: false,
            seek_to: None,
            restarting: false,
        }
    }

    /// Reads the input again from `start`, opened anew.
    fn restart(&mut self, start: u64) {
        let Some(again) = &self.again else {
            return;
        };
        self.reader.abort();
        (self.chunks, self.reader) = spawn_reader(again.source(), start);
        self.continuing = self.reencode.is_some();
        self.reencode = None;
        self.celt = false;
        self.history.clear();
        self.after.clear();
        self.held = None;
        self.decoded = 0;
        self.track.set_position(start);
        self.restarting = true;
    }

    /// Reads the next frame ahead, if the reader has one ready, into `after`; whether it
    /// did. What is not a frame is kept for `Player::recv`.
    fn pull(&mut self) -> bool {
        if self.held.is_some() {
            return false;
        }
        match self.chunks.try_recv() {
            Ok(Ok(Chunk::Frame(frame))) => {
                self.after.push_back(frame);
                true
            }
            Ok(other) => {
                self.held = Some(Some(other));
                false
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.held = Some(None);
                false
            }
        }
    }

    fn answer(&mut self, result: &Result<Duration, TrackError>) {
        for seek in self.seeks.drain(..) {
            let _ = seek.send(*result);
        }
    }
}

impl Drop for Playing {
    fn drop(&mut self) {
        // Its seeks, dropped, see the track ended.
        self.reader.abort();
    }
}

/// The player of a call (D-351: one track at a time). It plays the current track of the
/// queue, sends each frame at its time, `at`, paced by the frames' durations, and frames of
/// silence when audio stops. Opus packets at volume 1.0 go out as they are; the others are
/// encoded when due (C-126).
struct Player {
    shared: Arc<Shared>,
    track: Option<Playing>,
    /// The next frame, waiting for its time.
    next: Option<Frame>,
    /// When the next frame (audio or silence) is due.
    at: Option<Instant>,
    /// Frames of silence still to send.
    silence: u8,
    /// Opcode 5 Speaking says the bot speaks on the current connection.
    speaking: bool,
    bitrate: u32,
    /// Made when the first frame is encoded.
    encoder: Option<Encoder>,
    /// Room for the PCM of the longest packet.
    pcm: Vec<f32>,
    /// Room for an encoded packet.
    packet: Vec<u8>,
}

impl Player {
    fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            track: None,
            next: None,
            at: None,
            silence: 0,
            speaking: false,
            bitrate: codec::DEFAULT_BITRATE,
            encoder: None,
            pcm: vec![0.0; MAX_PACKET_VALUES],
            packet: vec![0; MAX_PACKET],
        }
    }

    fn set_bitrate(&mut self, bitrate: u32) {
        self.bitrate = bitrate;
        if let Some(encoder) = &mut self.encoder {
            encoder.set_bitrate(bitrate);
        }
    }

    /// Plays the queue's current track: starts it if it is new (the one playing ended
    /// already: replaced, skipped or stopped), and follows its pause.
    fn reconcile(&mut self) {
        let mut queue = lock(&self.shared.queue);
        let current = queue.current.as_ref().map(|slot| slot.track.id);
        if current.is_some() && current == self.track.as_ref().map(|playing| playing.track.id) {
            drop(queue);
            self.refresh();
            return;
        }
        let started = queue.current.as_mut().and_then(|slot| {
            let input = slot.input.take()?;
            slot.track.start();
            Some((Arc::clone(&slot.track), input))
        });
        drop(queue);
        let had = self.track.take().is_some();
        self.next = None;
        if let Some((track, input)) = started {
            let (source, again) = input.into_parts();
            self.track = Some(Playing::new(track, source, again));
        } else if had {
            self.quiet();
        }
    }

    /// Follows the current track's pause.
    fn refresh(&mut self) {
        let Some(playing) = &mut self.track else {
            return;
        };
        let paused = playing.track.paused();
        if paused == playing.paused {
            return;
        }
        playing.paused = paused;
        if !paused {
            // Not paused after all, if its last frame has not gone yet (a seek still fades
            // it out).
            playing.closing = playing.seek_to.is_some();
            return;
        }
        if core::mem::take(&mut playing.announced) {
            playing.track.emit(&TrackEvent::Paused);
        }
        // The next frame (read now if need be) fades out, then silence follows.
        playing.closing = true;
    }

    fn seek(&mut self, id: u64, samples: u64, done: oneshot::Sender<Result<Duration, TrackError>>) {
        match &mut self.track {
            Some(playing)
                if playing.track.id == id && !playing.track.ended() && playing.again.is_some() =>
            {
                playing.seeks.push(done);
                // Nothing audible to fade out: paused, or already opening the input again.
                let silent = (playing.paused && !playing.closing)
                    || (playing.restarting && self.next.is_none());
                if silent {
                    playing.restart(samples);
                    playing.seek_to = None;
                    playing.gain = 0.0;
                    self.next = None;
                } else {
                    // The frame from before fades out, then the input opens again.
                    playing.seek_to = Some(samples);
                    playing.closing = true;
                }
            }
            _ => {
                let _ = done.send(Err(TrackError::Ended));
            }
        }
    }

    /// Frames of silence follow, if audio was going out.
    fn quiet(&mut self) {
        if self.speaking {
            self.silence = SILENCE_FRAMES;
        }
    }

    /// The call ends: so does the track (its call already ended it).
    fn end(&mut self) {
        self.track = None;
        self.next = None;
    }

    fn playing(&self) -> bool {
        self.track.as_ref().is_some_and(|playing| !playing.paused)
    }

    fn wants_chunk(&self) -> bool {
        // A seek is answered by the first frame from the new place, even paused, and a
        // frame fading out is read even paused.
        let reading = self
            .track
            .as_ref()
            .is_some_and(|playing| !playing.seeks.is_empty() || playing.closing);
        (self.next.is_none() || self.ahead()) && self.silence == 0 && (self.playing() || reading)
    }

    /// Whether the Opus frame that is next waits for 20 ms more of the input to be read:
    /// re-encoding it takes the start of what follows too (the encoder's lookahead, C-127).
    fn ahead(&self) -> bool {
        matches!(self.next, Some(Frame::Opus { .. }))
            && self.track.as_ref().is_some_and(|playing| {
                playing.held.is_none()
                    && playing.after.iter().map(Frame::samples).sum::<u32>() < 960
            })
    }

    async fn recv(&mut self) -> Read {
        let Some(playing) = &mut self.track else {
            return core::future::pending().await;
        };
        // What was read ahead first, in order.
        if let Some(frame) = playing.after.pop_front() {
            return Some(Ok(Chunk::Frame(frame)));
        }
        if let Some(read) = playing.held.take() {
            return read;
        }
        playing.chunks.recv().await
    }

    fn received(&mut self, chunk: Option<Result<Chunk, InputError>>) {
        let Some(playing) = &mut self.track else {
            return;
        };
        match chunk {
            // Read ahead of the next frame: kept in order for `recv`.
            Some(Ok(Chunk::Frame(frame))) if self.next.is_some() => playing.after.push_back(frame),
            chunk if self.next.is_some() => playing.held = Some(chunk),
            Some(Ok(Chunk::Preroll(packets))) => playing.history = packets,
            Some(Ok(Chunk::Start(at))) => {
                playing.track.set_position(at);
                playing.answer(&Ok(duration(at)));
            }
            Some(Ok(Chunk::Frame(frame))) => {
                // Not the frame fading out before a seek: one from the new place.
                if playing.seek_to.is_none() {
                    if !playing.seeks.is_empty() {
                        playing.answer(&Ok(duration(playing.track.position())));
                    }
                    playing.restarting = false;
                }
                self.next = Some(frame);
            }
            Some(Err(error)) => self.finished(Some(error)),
            None => self.finished(None),
        }
    }

    /// The track's input ended: it plays again if it loops, or ends, and the next one plays.
    fn finished(&mut self, error: Option<InputError>) {
        let Some(playing) = &mut self.track else {
            return;
        };
        if let Some(samples) = playing.seek_to.take() {
            // The old place ended before a frame could fade out: the seek goes on.
            playing.closing = false;
            playing.restart(samples);
            playing.gain = 0.0;
            return;
        }
        if error.is_none()
            && playing.again.is_some()
            && !playing.track.ended()
            && playing.track.repeat()
        {
            playing.restart(0);
            playing.track.emit(&TrackEvent::Looped);
            return;
        }
        let reason = match error {
            None => TrackEnd::Finished,
            Some(error) => {
                tracing::warn!(%error, "a track's input failed");
                TrackEnd::Failed(Arc::new(error))
            }
        };
        {
            let mut queue = lock(&self.shared.queue);
            playing.track.end(reason);
            if queue
                .current
                .as_ref()
                .is_some_and(|slot| slot.track.id == playing.track.id)
            {
                queue.advance();
            }
        }
        // The next track, or silence.
        self.reconcile();
    }

    /// Whether a frame of silence fills in while the track's input is opened again.
    fn filling(&self) -> bool {
        self.speaking
            && self.next.is_none()
            && self
                .track
                .as_ref()
                .is_some_and(|playing| playing.restarting && !playing.paused)
    }

    /// When the next frame is due: now for the first after a pause.
    fn due(&self) -> Option<Instant> {
        let closing = self.track.as_ref().is_some_and(|playing| playing.closing);
        let ready = self.silence > 0
            || (self.next.is_some() && !self.ahead() && (self.playing() || closing))
            || self.filling();
        ready.then(|| self.at.unwrap_or_else(Instant::now))
    }

    /// Sends the frame that is due, if the call has a connection.
    async fn frame(&mut self, live: Option<&mut Live>) {
        let now = Instant::now();
        let at = match self.at {
            Some(at) if at + LATE >= now => at,
            // A first frame, or one far too late: the pace starts over.
            _ => now,
        };
        // Silence filling in while the input opens again keeps the bot speaking.
        let mut filler = false;
        let frame = if self.silence > 0 {
            self.silence -= 1;
            None
        } else if let Some(frame) = self.next.take() {
            Some(frame)
        } else if self.filling() {
            filler = true;
            None
        } else {
            return;
        };
        let samples = frame.as_ref().map_or(960, Frame::samples);
        self.at = (frame.is_some() || filler || self.silence > 0)
            .then(|| at + duration(u64::from(samples)));
        // The gain the frame ramps from and to, and where the input opens again after it.
        let mut gains = (1.0, 1.0);
        let mut restart = None;
        if frame.is_some() {
            let Some(playing) = &mut self.track else {
                return;
            };
            if playing.track.ended() {
                // Read before the track was stopped or replaced.
                return;
            }
            let to = if playing.closing {
                0.0
            } else {
                playing.track.volume()
            };
            gains = (playing.gain, to);
            playing.gain = to;
            playing.track.advance(samples);
            let closing = core::mem::take(&mut playing.closing);
            // Once it has faded out, the input opens again at the new place.
            restart = playing.seek_to.take().filter(|_| closing);
            let pausing = closing && playing.paused;
            if !pausing && !core::mem::replace(&mut playing.announced, true) {
                playing.track.emit(&TrackEvent::Playing);
            }
            if pausing {
                // Its last frame fades out; silence follows.
                self.quiet();
            }
        }
        self.send(live, frame.as_ref(), gains, filler, samples)
            .await;
        if let Some(playing) = &mut self.track {
            if let Some(Frame::Opus { data, samples }) = frame {
                playing.history.push(data, samples);
            }
            if let Some(samples) = restart {
                playing.restart(samples);
            }
        }
    }

    /// Sends `frame` (silence when `None`: a filler, or the frames after audio), if the call
    /// has a connection.
    async fn send(
        &mut self,
        live: Option<&mut Live>,
        frame: Option<&Frame>,
        gains: (f32, f32),
        filler: bool,
        samples: u32,
    ) {
        let Some(live) = live else {
            if self.silence == 0 && frame.is_none() {
                self.speaking = false;
            }
            if let Some(encoder) = &mut self.encoder {
                // Nobody decodes this one: the next starts independently.
                encoder.independent_next();
            }
            return;
        };
        // The packet, and whether the encoder made it.
        let (payload, encoded) = match frame {
            Some(frame) => {
                if !self.speaking {
                    live.handle.send(Command::Speaking(MICROPHONE));
                    self.speaking = true;
                }
                match (self.encode(frame, gains), frame) {
                    (Some(len), _) => (&self.packet[..len], true),
                    (None, Frame::Opus { data, .. }) => (&data[..], false),
                    // PCM that could not be encoded: nothing to send.
                    (None, Frame::Pcm(_)) => return,
                }
            }
            // While the input opens again: silence from the encoder, so that the listeners'
            // decoders stay in its stereo stream (Discord's silence frame is mono).
            None if filler => match self.silence_packet() {
                Some(len) => (&self.packet[..len], true),
                None => (&SILENCE[..], false),
            },
            None => (&SILENCE[..], false),
        };
        if let Err(error) = live.sender.send(payload, samples).await {
            tracing::debug!(%error, "a voice packet could not be sent");
        }
        if !encoded && let Some(encoder) = &mut self.encoder {
            // The listeners' decoders now hold what the encoder did not make (C-127).
            encoder.independent_next();
        }
        if frame.is_none() && !filler && self.silence == 0 {
            live.handle.send(Command::Speaking(0));
            self.speaking = false;
        }
    }

    /// A 20 ms packet of silence from the encoder, in `packet`; its length.
    fn silence_packet(&mut self) -> Option<usize> {
        let encoder = match &mut self.encoder {
            Some(encoder) => encoder,
            None => self.encoder.insert(Encoder::new(self.bitrate)?),
        };
        let pcm = &mut self.pcm[..codec::FRAME_VALUES];
        pcm.fill(0.0);
        encoder.encode(pcm, 1.0, 1.0, &mut self.packet)
    }

    /// Encodes `frame` into `packet`, its gain ramping from `gains.0` to `gains.1`; its
    /// length, or `None` for an Opus packet sent as it is (at 1.0 throughout once
    /// re-encoding has settled, or one libopus cannot re-encode).
    fn encode(&mut self, frame: &Frame, (from, to): (f32, f32)) -> Option<usize> {
        let encoder = match &mut self.encoder {
            Some(encoder) => encoder,
            None => self.encoder.insert(Encoder::new(self.bitrate)?),
        };
        let playing = self.track.as_mut()?;
        let (data, samples) = match frame {
            Frame::Pcm(bytes) => {
                let pcm = &mut self.pcm[..codec::FRAME_VALUES];
                codec::from_s16le(bytes, pcm);
                return encoder.encode(pcm, from, to, &mut self.packet);
            }
            Frame::Opus { data, samples } => (data, usize::try_from(*samples).ok()?),
        };
        let unity = (from - 1.0).abs() <= f32::EPSILON && (to - 1.0).abs() <= f32::EPSILON;
        let celt = codec::celt_only(data);
        let celt_before = core::mem::replace(&mut playing.celt, celt);
        let as_they_are = match &playing.reencode {
            // Not right after the input opened again under the encoder's packets.
            None => !playing.continuing,
            // Back to the packets as they are once the listeners' decoders hold the same
            // audio, and only between CELT-only ones: after the encoder's (CELT) packets,
            // a SILK or hybrid one is decoded from a reset SILK state, far too quiet.
            Some(reencode) => reencode.steady >= SETTLE && celt && celt_before,
        };
        if unity && as_they_are {
            playing.reencode = None;
            playing.decoded = 0;
            return None;
        }
        let lookahead = encoder.lookahead();
        if playing.reencode.is_none() {
            let mut decoder = Decoder::new()?;
            let mut fifo = VecDeque::with_capacity(MAX_PACKET_VALUES * 2);
            // The encoder first takes the 20 ms before this frame shifted by its lookahead:
            // the end of the packet before (silence if there is none), then the start of
            // this one. Its next output is then this frame, in step with the packets.
            let before = 960 - lookahead;
            // The input's packets before this one bring the decoder where the listeners'
            // decoders are: from one packet alone, CELT and SILK come out far too quiet, and
            // the encoder would start from the wrong state.
            let mut previous = 0;
            for packet in playing.history.packets() {
                previous = decoder.decode(packet, &mut self.pcm).unwrap_or(0);
            }
            let continuing = core::mem::take(&mut playing.continuing);
            if !continuing {
                let kept = previous.min(before);
                fifo.extend(core::iter::repeat_n(0.0, (before - kept) * 2));
                fifo.extend(&self.pcm[(previous - kept) * 2..previous * 2]);
            }
            let current = decoder.decode(data, &mut self.pcm)?;
            fifo.extend(&self.pcm[..current * 2]);
            playing.decoded = 1;
            let mut reencode = Reencode {
                decoder,
                fifo,
                steady: 0,
            };
            if continuing {
                // The encoder's stream goes on: the listeners decoded its packets. The new
                // place starts its lookahead in, so that it stays in step.
                fill(playing, &mut reencode, &mut self.pcm, lookahead * 2);
                reencode.fifo.drain(..lookahead * 2);
            } else {
                fill(playing, &mut reencode, &mut self.pcm, codec::FRAME_VALUES);
                // At the gain this frame starts at, as the listeners heard it: the packets
                // as they are (1.0), or nothing (0.0).
                let mut prime: Vec<f32> = reencode
                    .fifo
                    .drain(..codec::FRAME_VALUES)
                    .map(|value| value * from)
                    .collect();
                let _ = encoder.encode(&mut prime, 1.0, 1.0, &mut self.packet);
                // The listeners did not decode that one: the next starts independently.
                encoder.independent_next();
            }
            playing.reencode = Some(reencode);
        } else if playing.decoded == 0 {
            let reencode = playing.reencode.as_mut()?;
            let decoded = reencode.decoder.decode(data, &mut self.pcm)?;
            reencode.fifo.extend(&self.pcm[..decoded * 2]);
            playing.decoded = 1;
        }
        let mut reencode = playing.reencode.take()?;
        reencode.steady = if unity {
            reencode.steady.saturating_add(1)
        } else {
            0
        };
        let values = samples * 2;
        fill(playing, &mut reencode, &mut self.pcm, values);
        for (out, value) in self.pcm[..values]
            .iter_mut()
            .zip(reencode.fifo.drain(..values.min(reencode.fifo.len())))
        {
            *out = value;
        }
        playing.decoded = playing.decoded.saturating_sub(1);
        playing.reencode = Some(reencode);
        encoder.encode(&mut self.pcm[..values], from, to, &mut self.packet)
    }
}

/// Decodes frames read ahead until `reencode.fifo` holds `values` (silence completes it at
/// the input's end, or when the reader is behind).
fn fill(playing: &mut Playing, reencode: &mut Reencode, pcm: &mut [f32], values: usize) {
    while reencode.fifo.len() < values {
        // The frames decoded so far: the current one and `decoded - 1` read ahead.
        let index = playing.decoded.saturating_sub(1);
        if index >= playing.after.len() && !playing.pull() {
            break;
        }
        let Some(Frame::Opus { data, .. }) = playing.after.get(index) else {
            break;
        };
        let Some(decoded) = reencode.decoder.decode(data, pcm) else {
            break;
        };
        reencode.fifo.extend(&pcm[..decoded * 2]);
        playing.decoded += 1;
    }
    let missing = values.saturating_sub(reencode.fifo.len());
    reencode.fifo.extend(core::iter::repeat_n(0.0, missing));
}
