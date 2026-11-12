//! The [`Shard`]: one gateway connection.

use core::{fmt::Write as _, future::Future, pin::Pin, time::Duration};
use std::sync::{Arc, Mutex, PoisonError};

use djs_model::gateway::{
    API_VERSION, CloseCode, DispatchEvent, GatewayEvent, GatewayEventDeserializer, OpCode, ShardId,
    payload::{
        CommandPayload, GatewayCapabilities, GatewayCommand, Heartbeat, Identify, Resume,
        UpdatePresence,
    },
};
use futures_util::{SinkExt, StreamExt};
use serde::de::DeserializeSeed;
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time::{Instant, Sleep, sleep, sleep_until},
};
use tokio_websockets::{ClientBuilder, Limits, MaybeTlsStream, Message, WebSocketStream};

use crate::{
    Config,
    compression::Decompressor,
    error::{ReceiveError, ReceiveErrorKind, SendError},
    send_limit::SendLimit,
};

type Connection = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A shard's turn in the IDENTIFY queue.
///
/// Kept in a [`Mutex`] only so that [`Shard`] stays `Sync` (the future is only `Send`); it is
/// never locked — polling goes through [`Mutex::get_mut`], which is free.
type IdentifyTurn = Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>;

/// Upper bound of the reconnect backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// How long to wait for our close frame to be written before dropping the connection.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
/// The close code used when *we* close a connection we want to resume later. Codes 1000 and
/// 1001 would invalidate the session.
const RESUMABLE_CLOSE_CODE: u16 = 4000;

/// The connection state of a [`Shard`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardState {
    /// Not connected. The next call to [`Shard::next_event`] connects (after a backoff delay if
    /// the previous attempt failed).
    Disconnected,
    /// Connected and identifying; waiting for `READY`.
    Identifying,
    /// Connected and resuming; waiting for `RESUMED`.
    Resuming,
    /// The session is established and events are flowing.
    Active,
    /// Discord closed the connection with a code that forbids reconnecting.
    FatallyClosed(CloseCode),
    /// Closed by [`Shard::close`].
    Closed,
}

/// A single connection to the Discord gateway.
///
/// A shard is **poll-driven** (D-037): calling [`next_event`](Self::next_event) is what drives
/// the connection — it connects, identifies or resumes, sends heartbeats, detects zombied
/// connections and reconnects, all while waiting for the next event. Nothing happens while no
/// one is polling, and there are no background tasks or internal event channels.
///
/// Before identifying, a shard waits for its turn in the [`IdentifyQueue`] of its
/// [`Config`]; shards created from clones of one configuration (as [`create_shards`] does)
/// share it. Resuming does not identify and does not wait.
///
/// Commands (presence updates, member requests, …) can be sent from other tasks through a
/// [`MessageSender`]; they are delivered once the session is active, paced to stay under
/// Discord's send limit.
///
/// [`IdentifyQueue`]: crate::IdentifyQueue
/// [`create_shards`]: crate::create_shards
///
/// discord.js counterpart: `WebSocketShard` from `@discordjs/ws`.
///
/// # Examples
///
/// ```no_run
/// use djs_gateway::Shard;
/// use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, ShardId};
///
/// # async fn run() {
/// let token = std::env::var("DISCORD_TOKEN").unwrap();
/// let mut shard = Shard::new(ShardId::ONE, token, Intents::GUILDS | Intents::GUILD_MESSAGES);
///
/// while let Some(event) = shard.next_event().await {
///     match event {
///         Ok(GatewayEvent::Dispatch(_, DispatchEvent::Ready(ready))) => {
///             println!("logged in as {}", ready.user.username);
///         }
///         Ok(_) => {}
///         Err(error) if error.is_fatal() => {
///             eprintln!("fatal: {error}");
///             break;
///         }
///         Err(error) => eprintln!("recoverable: {error}"),
///     }
/// }
/// # }
/// ```
pub struct Shard {
    id: ShardId,
    config: Config,
    state: ShardState,
    connection: Option<Connection>,
    decompressor: Option<Decompressor>,
    heartbeat: Option<Heartbeater>,
    hello_deadline: Option<Pin<Box<Sleep>>>,
    /// The shard's turn in the IDENTIFY queue, while it waits for it.
    identify_turn: Option<IdentifyTurn>,
    session: Option<Session>,
    sequence: Option<u64>,
    latency: Option<Duration>,
    reconnect_attempts: u32,
    connect_after: Option<Instant>,
    commands_tx: mpsc::UnboundedSender<String>,
    commands_rx: mpsc::UnboundedReceiver<String>,
    send_limit: SendLimit,
    held: Option<HeldCommand>,
    /// The presence to identify with, shared with every [`MessageSender`].
    presence: SharedPresence,
}

/// The presence a shard identifies with: the configured one until
/// [`MessageSender::update_presence`] replaces it.
type SharedPresence = Arc<Mutex<Option<UpdatePresence>>>;

struct Session {
    id: Box<str>,
    resume_url: Box<str>,
}

struct Heartbeater {
    interval: Duration,
    sleep: Pin<Box<Sleep>>,
    last_sent: Option<Instant>,
    acked: bool,
}

/// A command taken from the queue but held back by the send limit.
struct HeldCommand {
    json: String,
    /// Fires once the send limit has room for it.
    until: Pin<Box<Sleep>>,
}

/// What woke the shard up.
enum Wake {
    Heartbeat,
    HelloTimeout,
    Identify,
    Command(String),
    HeldCommandAllowed,
    Message(Option<Result<Message, tokio_websockets::Error>>),
}

impl core::fmt::Debug for Shard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Shard")
            .field("id", &self.id)
            .field("state", &self.state)
            .field(
                "session_id",
                &self.session.as_ref().map(|session| &session.id),
            )
            .field("sequence", &self.sequence)
            .field("latency", &self.latency)
            .finish_non_exhaustive()
    }
}

impl Shard {
    /// Creates a shard with the default [`Config`]. It does not connect until
    /// [`next_event`](Self::next_event) is called.
    ///
    /// Pass the raw token, without the `Bot ` prefix.
    #[must_use]
    pub fn new(
        id: ShardId,
        token: impl Into<String>,
        intents: djs_model::gateway::Intents,
    ) -> Self {
        Self::with_config(id, Config::new(token, intents))
    }

    /// Creates a shard with a custom [`Config`].
    #[must_use]
    pub fn with_config(id: ShardId, config: Config) -> Self {
        let (commands_tx, commands_rx) = mpsc::unbounded_channel();
        let presence = Arc::new(Mutex::new(config.presence.clone()));
        Self {
            id,
            config,
            state: ShardState::Disconnected,
            connection: None,
            decompressor: None,
            heartbeat: None,
            hello_deadline: None,
            identify_turn: None,
            session: None,
            sequence: None,
            latency: None,
            reconnect_attempts: 0,
            connect_after: None,
            commands_tx,
            commands_rx,
            send_limit: SendLimit::default(),
            held: None,
            presence,
        }
    }

    /// Returns the shard's ID.
    #[must_use]
    pub const fn id(&self) -> ShardId {
        self.id
    }

    /// Returns the shard's configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Returns the current connection state.
    #[must_use]
    pub const fn state(&self) -> ShardState {
        self.state
    }

    /// Returns the round-trip time of the last acknowledged heartbeat.
    ///
    /// discord.js counterpart: `WebSocketShard#ping` / `client.ws.ping`.
    #[must_use]
    pub const fn latency(&self) -> Option<Duration> {
        self.latency
    }

    /// Returns the current session ID, if a session is established.
    #[must_use]
    pub fn session_id(&self) -> Option<&str> {
        self.session.as_ref().map(|session| &*session.id)
    }

    /// Returns the sequence number of the last dispatch received.
    #[must_use]
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns a handle to send commands to this shard from anywhere.
    ///
    /// Commands are queued and sent once the session is [active](ShardState::Active), in
    /// order, paced to stay under Discord's send limit (see [`MessageSender`]).
    #[must_use]
    pub fn sender(&self) -> MessageSender {
        MessageSender {
            tx: self.commands_tx.clone(),
            presence: Arc::clone(&self.presence),
        }
    }

    /// Waits for the next event, driving the connection meanwhile.
    ///
    /// Returns `None` once the shard has been [closed](Self::close) or fatally closed by Discord.
    /// Every gateway payload is returned, including control payloads such as
    /// [`GatewayEvent::Hello`], which the shard has already acted upon.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe in the sense that cancelling it never corrupts the shard: it
    /// can be used in `tokio::select!`. An event being processed at the moment of cancellation
    /// may be lost.
    pub async fn next_event(&mut self) -> Option<Result<GatewayEvent, ReceiveError>> {
        loop {
            if matches!(
                self.state,
                ShardState::FatallyClosed(_) | ShardState::Closed
            ) {
                return None;
            }
            if self.connection.is_none() {
                if let Some(at) = self.connect_after.take() {
                    sleep_until(at).await;
                }
                if let Err(error) = self.connect().await {
                    self.schedule_reconnect();
                    return Some(Err(error));
                }
            }
            if let Some(result) = self.step().await {
                return Some(result);
            }
        }
    }

    /// Closes the connection and invalidates the session (the bot appears offline). Afterwards
    /// [`next_event`](Self::next_event) returns `None`.
    ///
    /// discord.js counterpart: `WebSocketShard#destroy`.
    pub async fn close(&mut self) {
        if let Some(mut connection) = self.connection.take() {
            let frame = Message::close(Some(tokio_websockets::CloseCode::NORMAL_CLOSURE), "");
            let _ = tokio::time::timeout(CLOSE_TIMEOUT, connection.send(frame)).await;
        }
        self.reset_connection();
        self.session = None;
        self.sequence = None;
        self.state = ShardState::Closed;
        tracing::info!(shard = %self.id, "shard closed");
    }

    /// Waits for one wake-up source and handles it. Returns an item for the caller, if any.
    async fn step(&mut self) -> Option<Result<GatewayEvent, ReceiveError>> {
        let active = self.state == ShardState::Active;
        // A held command goes out first; the queue waits behind it.
        let receive_commands = active && self.held.is_none();
        let connection = self.connection.as_mut()?;
        let heartbeat = self
            .heartbeat
            .as_mut()
            .map(|heartbeat| heartbeat.sleep.as_mut());
        let hello = self.hello_deadline.as_mut().map(Pin::as_mut);
        let identify = self.identify_turn.as_mut().map(|turn| {
            turn.get_mut()
                .unwrap_or_else(PoisonError::into_inner)
                .as_mut()
        });
        let held = self
            .held
            .as_mut()
            .filter(|_| active)
            .map(|held| held.until.as_mut());

        let wake = tokio::select! {
            biased;
            () = optional(heartbeat) => Wake::Heartbeat,
            () = optional(hello) => Wake::HelloTimeout,
            () = optional(identify) => Wake::Identify,
            () = optional(held) => Wake::HeldCommandAllowed,
            Some(command) = self.commands_rx.recv(), if receive_commands => Wake::Command(command),
            message = connection.next() => Wake::Message(message),
        };

        match wake {
            Wake::Heartbeat => self.heartbeat_due().await.err().map(Err),
            Wake::HelloTimeout => {
                tracing::warn!(shard = %self.id, "no Hello received in time, reconnecting");
                self.disconnect(true).await;
                None
            }
            Wake::Identify => {
                self.identify_turn = None;
                self.identify().await.err().map(Err)
            }
            Wake::Command(json) => self.send_limited(json).await.err().map(Err),
            Wake::HeldCommandAllowed => match self.held.take() {
                Some(held) => self.send_limited(held.json).await.err().map(Err),
                None => None,
            },
            Wake::Message(None) => {
                tracing::info!(shard = %self.id, "connection ended without a close frame, reconnecting");
                self.disconnect(true).await;
                None
            }
            Wake::Message(Some(Err(error))) => {
                self.disconnect(true).await;
                Some(Err(ReceiveError::with_source(
                    ReceiveErrorKind::Connection,
                    error,
                )))
            }
            Wake::Message(Some(Ok(message))) => self.handle_message(&message).await,
        }
    }

    async fn handle_message(
        &mut self,
        message: &Message,
    ) -> Option<Result<GatewayEvent, ReceiveError>> {
        if let Some((code, reason)) = message.as_close() {
            let code = u16::from(code);
            let reason = reason.to_owned();
            return self.handle_close(Some(code), &reason);
        }
        if message.is_close() {
            return self.handle_close(None, "");
        }

        let (sequence, parsed) = if let Some(json) = message.as_text() {
            parse(json)
        } else if message.is_binary() {
            let Some(decompressor) = self.decompressor.as_mut() else {
                tracing::warn!(shard = %self.id, "binary message received without compression, ignoring");
                return None;
            };
            match decompressor.decompress(message.as_payload()) {
                Ok(bytes) => match core::str::from_utf8(bytes) {
                    Ok(json) => parse(json),
                    Err(error) => (
                        None,
                        Err(ReceiveError::with_source(
                            ReceiveErrorKind::Deserialize,
                            error,
                        )),
                    ),
                },
                Err(error) => {
                    let error = ReceiveError::with_source(ReceiveErrorKind::Decompress, error);
                    self.disconnect(true).await;
                    return Some(Err(error));
                }
            }
        } else {
            // Ping/pong frames are answered by the WebSocket library.
            return None;
        };

        if let Some(sequence) = sequence {
            self.sequence = Some(sequence);
        }
        match parsed {
            Ok(event) => match self.process(&event).await {
                Ok(()) => Some(Ok(event)),
                Err(error) => Some(Err(error)),
            },
            Err(error) => {
                tracing::warn!(shard = %self.id, %error, "skipping a gateway payload that failed to parse");
                Some(Err(error))
            }
        }
    }

    /// Updates the shard state for an event and performs the required actions.
    async fn process(&mut self, event: &GatewayEvent) -> Result<(), ReceiveError> {
        match event {
            GatewayEvent::Hello(hello) => {
                self.hello_deadline = None;
                let interval = Duration::from_millis(hello.heartbeat_interval);
                // The first heartbeat is sent after `interval * jitter` (jitter in [0, 1)).
                let first = interval.mul_f64(fastrand::f64());
                self.heartbeat = Some(Heartbeater {
                    interval,
                    sleep: Box::pin(sleep(first)),
                    last_sent: None,
                    acked: true,
                });
                if self.session.is_some() && self.sequence.is_some() {
                    self.resume().await?;
                } else {
                    self.schedule_identify();
                }
            }
            GatewayEvent::HeartbeatAck => {
                if let Some(heartbeat) = self.heartbeat.as_mut() {
                    heartbeat.acked = true;
                    if let Some(sent) = heartbeat.last_sent {
                        self.latency = Some(sent.elapsed());
                    }
                }
            }
            GatewayEvent::Heartbeat => self.send_heartbeat().await?,
            GatewayEvent::Reconnect => {
                tracing::info!(shard = %self.id, "Discord requested a reconnect");
                self.disconnect(true).await;
            }
            GatewayEvent::InvalidSession(resumable) => {
                tracing::info!(shard = %self.id, resumable, "session invalidated");
                self.disconnect(*resumable).await;
                // Discord recommends waiting a random 1–5 seconds before reconnecting.
                self.connect_after =
                    Some(Instant::now() + Duration::from_millis(fastrand::u64(1_000..=5_000)));
            }
            GatewayEvent::Dispatch(_, DispatchEvent::Ready(ready)) => {
                self.session = Some(Session {
                    id: ready.session_id.as_str().into(),
                    resume_url: ready.resume_gateway_url.as_str().into(),
                });
                self.state = ShardState::Active;
                self.reconnect_attempts = 0;
                tracing::info!(shard = %self.id, user = %ready.user.username, guilds = ready.guilds.len(), "session ready");
            }
            GatewayEvent::Dispatch(_, DispatchEvent::Resumed) => {
                self.state = ShardState::Active;
                self.reconnect_attempts = 0;
                tracing::info!(shard = %self.id, "session resumed");
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_close(
        &mut self,
        code: Option<u16>,
        reason: &str,
    ) -> Option<Result<GatewayEvent, ReceiveError>> {
        tracing::info!(shard = %self.id, code, reason, "gateway closed the connection");
        self.connection = None;
        self.reset_connection();

        if let Some(code) = code.filter(|code| (4000..5000).contains(code)) {
            let code = CloseCode::from(code);
            if !code.can_reconnect() {
                tracing::error!(shard = %self.id, ?code, "fatal close code, the shard will stop");
                self.state = ShardState::FatallyClosed(code);
                return Some(Err(ReceiveError::new(ReceiveErrorKind::FatallyClosed {
                    code,
                })));
            }
            if matches!(
                code,
                CloseCode::InvalidSequence | CloseCode::SessionTimedOut
            ) {
                self.session = None;
                self.sequence = None;
            }
        }
        None
    }

    async fn connect(&mut self) -> Result<(), ReceiveError> {
        let base = match (&self.session, self.sequence) {
            (Some(session), Some(_)) => &*session.resume_url,
            _ => &*self.config.gateway_url,
        };
        let mut url = String::with_capacity(base.len() + 48);
        url.push_str(base.trim_end_matches('/'));
        let _ = write!(url, "/?v={API_VERSION}&encoding=json");
        if let Some(compression) = self.config.compression.query_value() {
            let _ = write!(url, "&compress={compression}");
        }

        tracing::debug!(shard = %self.id, %url, "connecting to the gateway");
        let builder = ClientBuilder::new()
            .uri(&url)
            .map_err(|error| ReceiveError::with_source(ReceiveErrorKind::Connect, error))?
            .connector(&self.config.tls)
            .limits(Limits::unlimited());
        let (connection, _) = tokio::time::timeout(self.config.connect_timeout, builder.connect())
            .await
            .map_err(|_| {
                ReceiveError::with_source(ReceiveErrorKind::Connect, "connection timed out")
            })?
            .map_err(|error| ReceiveError::with_source(ReceiveErrorKind::Connect, error))?;

        self.decompressor = Decompressor::new(self.config.compression)
            .map_err(|error| ReceiveError::with_source(ReceiveErrorKind::Connect, error))?;
        self.connection = Some(connection);
        self.hello_deadline = Some(Box::pin(sleep(self.config.hello_timeout)));
        tracing::info!(shard = %self.id, "connected to the gateway");
        Ok(())
    }

    /// Schedules the next connection attempt with exponential backoff and jitter.
    fn schedule_reconnect(&mut self) {
        self.reconnect_attempts = self.reconnect_attempts.saturating_add(1);
        let exponent = self.reconnect_attempts.saturating_sub(1).min(6);
        let delay = Duration::from_secs(1 << exponent).min(MAX_BACKOFF);
        let delay = delay.mul_f64(0.5 + fastrand::f64() / 2.0);
        tracing::warn!(
            shard = %self.id,
            attempt = self.reconnect_attempts,
            delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
            "connecting failed, retrying after a delay"
        );
        self.connect_after = Some(Instant::now() + delay);
    }

    /// Queues for an IDENTIFY turn; the shard identifies when the queue lets it.
    fn schedule_identify(&mut self) {
        self.identify_turn = Some(Mutex::new(self.config.identify_queue.wait_turn(self.id)));
        self.state = ShardState::Identifying;
    }

    async fn identify(&mut self) -> Result<(), ReceiveError> {
        let identify = Identify {
            token: (*self.config.token).into(),
            properties: self.config.properties.clone(),
            large_threshold: self.config.large_threshold,
            shard: Some(self.id),
            intents: self.config.intents,
            presence: self
                .presence
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone(),
            capabilities: GatewayCapabilities::empty(),
        };
        self.state = ShardState::Identifying;
        tracing::debug!(shard = %self.id, "identifying");
        self.send_command(&identify).await
    }

    async fn resume(&mut self) -> Result<(), ReceiveError> {
        let (Some(session), Some(seq)) = (&self.session, self.sequence) else {
            return Ok(());
        };
        let resume = Resume {
            token: (*self.config.token).into(),
            session_id: (*session.id).into(),
            seq,
        };
        self.state = ShardState::Resuming;
        tracing::debug!(shard = %self.id, seq, "resuming");
        self.send_command(&resume).await
    }

    async fn heartbeat_due(&mut self) -> Result<(), ReceiveError> {
        if self
            .heartbeat
            .as_ref()
            .is_some_and(|heartbeat| !heartbeat.acked)
        {
            tracing::warn!(shard = %self.id, "no heartbeat ACK received (zombied connection), reconnecting");
            self.disconnect(true).await;
            return Ok(());
        }
        self.send_heartbeat().await
    }

    async fn send_heartbeat(&mut self) -> Result<(), ReceiveError> {
        let sequence = self.sequence;
        if let Some(heartbeat) = self.heartbeat.as_mut() {
            let now = Instant::now();
            heartbeat.last_sent = Some(now);
            heartbeat.acked = false;
            heartbeat.sleep.as_mut().reset(now + heartbeat.interval);
        }
        self.send_command(&Heartbeat(sequence)).await
    }

    async fn send_command<T: GatewayCommand>(&mut self, command: &T) -> Result<(), ReceiveError> {
        let json = serde_json::to_string(&CommandPayload::new(command))
            .expect("gateway commands always serialize");
        self.send(json).await
    }

    /// Sends a queued command if the send limit allows it, or holds it until it does.
    async fn send_limited(&mut self, json: String) -> Result<(), ReceiveError> {
        match self.send_limit.try_acquire(Instant::now()) {
            Ok(()) => self.send(json).await,
            Err(at) => {
                tracing::debug!(shard = %self.id, "gateway send limit reached, holding queued commands");
                self.held = Some(HeldCommand {
                    json,
                    until: Box::pin(sleep_until(at)),
                });
                Ok(())
            }
        }
    }

    /// Writes a payload, bypassing the send limit (used directly only for heartbeats, identify
    /// and resume, which the limit leaves room for).
    async fn send(&mut self, json: String) -> Result<(), ReceiveError> {
        let Some(connection) = self.connection.as_mut() else {
            return Ok(());
        };
        if let Err(error) = connection.send(Message::text(json)).await {
            self.disconnect(true).await;
            return Err(ReceiveError::with_source(
                ReceiveErrorKind::Connection,
                error,
            ));
        }
        Ok(())
    }

    /// Drops the connection. With `resume`, the session is kept so the next connection resumes
    /// it; otherwise the next connection identifies from scratch.
    async fn disconnect(&mut self, resume: bool) {
        if let Some(mut connection) = self.connection.take() {
            let code = tokio_websockets::CloseCode::try_from(RESUMABLE_CLOSE_CODE).ok();
            let _ = tokio::time::timeout(CLOSE_TIMEOUT, connection.send(Message::close(code, "")))
                .await;
        }
        self.reset_connection();
        if !resume {
            self.session = None;
            self.sequence = None;
        }
    }

    /// Clears all per-connection state.
    fn reset_connection(&mut self) {
        self.connection = None;
        self.decompressor = None;
        self.heartbeat = None;
        self.hello_deadline = None;
        // Leaves the IDENTIFY queue; the next connection queues again.
        self.identify_turn = None;
        self.state = ShardState::Disconnected;
        // The send limit is per connection: a held command may go out as soon as the next
        // session is active.
        self.send_limit.clear();
        if let Some(held) = self.held.as_mut() {
            held.until.as_mut().reset(Instant::now());
        }
    }
}

/// Waits on an optional timer or turn; never completes if there is none.
async fn optional<F: Future<Output = ()> + ?Sized>(future: Option<Pin<&mut F>>) {
    match future {
        Some(future) => future.await,
        None => core::future::pending().await,
    }
}

/// Decodes a JSON gateway payload. Also returns the dispatch sequence number, which must be
/// tracked even if the payload itself fails to deserialize.
fn parse(json: &str) -> (Option<u64>, Result<GatewayEvent, ReceiveError>) {
    let Some(seed) = GatewayEventDeserializer::from_json(json) else {
        return (
            None,
            Err(ReceiveError::with_source(
                ReceiveErrorKind::Deserialize,
                "payload has no valid `op` field",
            )),
        );
    };
    let sequence = (seed.op() == OpCode::Dispatch)
        .then(|| seed.sequence())
        .flatten();
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = seed
        .deserialize(&mut deserializer)
        .map_err(|error| ReceiveError::with_source(ReceiveErrorKind::Deserialize, error));
    (sequence, result)
}

/// Sends commands to a [`Shard`] from any task. Cheap to clone.
///
/// Commands are queued and written by the shard, in order, once its session is active.
///
/// # Send limit
///
/// Discord accepts [120 payloads per connection every 60 seconds][limit] and disconnects a
/// client that sends more. The shard therefore sends at most **115** queued commands in any
/// 61-second span (one second of margin for network jitter); the rest wait in the queue and
/// go out as soon as the limit allows, so [`command`](Self::command) never blocks or fails
/// because of it. The remaining room is kept for heartbeats, identify and resume, which are
/// never held back. A new connection starts with a fresh limit.
///
/// discord.js counterpart: `WebSocketShard#send` from `@discordjs/ws`, which paces payloads
/// the same way.
///
/// [limit]: https://docs.discord.com/developers/events/gateway#rate-limiting
#[derive(Clone, Debug)]
pub struct MessageSender {
    tx: mpsc::UnboundedSender<String>,
    presence: SharedPresence,
}

impl MessageSender {
    /// Queues a command.
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Closed`] if the shard has been dropped, or
    /// [`SendError::Serialize`] if the command cannot be serialized.
    pub fn command<T: GatewayCommand>(&self, command: &T) -> Result<(), SendError> {
        let json =
            serde_json::to_string(&CommandPayload::new(command)).map_err(SendError::Serialize)?;
        self.tx.send(json).map_err(|_| SendError::Closed)
    }

    /// Updates the bot's presence on this shard, and makes it the presence the shard identifies
    /// with from now on, so a new session keeps it (D-148).
    ///
    /// Prefer this over [`command`](Self::command) with an [`UpdatePresence`], which only sends
    /// the update: after a new session the shard would show its configured
    /// [presence](crate::ConfigBuilder::presence) again.
    ///
    /// discord.js counterpart: `ClientUser#setPresence` (which, unlike this method, is
    /// forgotten when a new session starts).
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Closed`] if the shard has been dropped, or
    /// [`SendError::Serialize`] if the presence cannot be serialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_gateway::Shard;
    /// use djs_model::gateway::{Activity, Intents, ShardId, Status, payload::UpdatePresence};
    ///
    /// let shard = Shard::new(ShardId::ONE, "my-token", Intents::GUILDS);
    /// shard
    ///     .sender()
    ///     .update_presence(&UpdatePresence::new(Status::Idle, [Activity::custom("Resting")]))
    ///     .unwrap();
    /// ```
    pub fn update_presence(&self, presence: &UpdatePresence) -> Result<(), SendError> {
        self.command(presence)?;
        *self.presence.lock().unwrap_or_else(PoisonError::into_inner) = Some(presence.clone());
        Ok(())
    }

    /// Returns `true` if the shard has been dropped.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

// Shards and senders move between tasks and threads; keep them `Send + Sync`.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Shard>();
    assert_send_sync::<MessageSender>();
    assert_send_sync::<crate::Config>();
};
