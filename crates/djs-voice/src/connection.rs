//! A voice connection (step 1c): the voice gateway WebSocket and the UDP socket of one call,
//! from the handshake to the end.
//!
//! [`connect`] runs the handshake (the WebSocket, Identify, Hello and Ready, the UDP socket
//! and IP discovery, Select Protocol, Session Description), then a task that keeps the
//! connection: it heartbeats with the `seq_ack` of version 8, sends the UDP keepalive,
//! handles DAVE (step 1d) through a [`Dave`] it shares with the senders, passes the other
//! messages to the call as [`ConnectionEvent`]s, and after a lost connection resumes or
//! identifies anew as [`payload::after_close`] says, retrying as D-331 says. [`connect`]
//! returns once media can be sent: right after the handshake without DAVE, once the bot has
//! the group's key with it (D-322). Audio goes out through a [`Sender`], end-to-end
//! encrypted with DAVE, then for the transport, straight to the UDP socket of the current
//! session.

#![allow(dead_code, reason = "used by `Call` from step 1e")]

use core::{fmt, time::Duration};
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use djs_model::id::{ChannelId, GuildId, UserId};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpStream, UdpSocket},
    sync::{mpsc, watch},
    time::{Instant, sleep_until, timeout, timeout_at},
};
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

use crate::{
    crypto::{Cipher, Mode, OVERHEAD, RTP_HEADER},
    dave::{Dave, Handled, Reply},
    payload::{self, AfterClose, DecodeError, Event, Incoming},
    tls, udp,
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How long closing the WebSocket waits for the voice gateway's answer.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(1);

/// What the main gateway tells about a call: the bot's voice session (`VOICE_STATE_UPDATE`)
/// and the voice server (`VOICE_SERVER_UPDATE`).
#[derive(Clone)]
pub(crate) struct Info {
    pub(crate) guild_id: GuildId,
    /// The voice channel: its ID is the DAVE group's.
    pub(crate) channel_id: ChannelId,
    pub(crate) user_id: UserId,
    pub(crate) session_id: String,
    /// The voice server's `host:port`, without a scheme (`wss://` is added); one that starts
    /// with `ws://` or `wss://` is used as it is (tests, proxies).
    pub(crate) endpoint: String,
    /// The voice token: a secret.
    pub(crate) token: String,
}

impl fmt::Debug for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Info")
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .field("user_id", &self.user_id)
            .field("session_id", &self.session_id)
            .field("endpoint", &self.endpoint)
            .field("token", &"<redacted>")
            .finish()
    }
}

/// How connections behave.
#[derive(Clone)]
pub(crate) struct Config {
    /// The longest a handshake (or a resume) may take, until DAVE is ready too (D-322).
    pub(crate) timeout: Duration,
    /// The delays of the attempts to resume after the immediate one (D-331).
    pub(crate) retry_delays: Vec<Duration>,
    /// The newest DAVE protocol version offered in Identify.
    pub(crate) max_dave_protocol_version: u16,
    /// How often the UDP keepalive is sent.
    pub(crate) keepalive: Duration,
    /// How long IP discovery waits for an answer before asking again (UDP may lose it).
    pub(crate) discovery_retry: Duration,
    /// The TLS connector of `wss://` connections.
    pub(crate) tls: Arc<Connector>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            retry_delays: [1, 2, 4, 8, 16].map(Duration::from_secs).to_vec(),
            max_dave_protocol_version: davey::DAVE_PROTOCOL_VERSION,
            keepalive: Duration::from_secs(5),
            discovery_retry: Duration::from_secs(1),
            tls: Arc::new(tls::connector()),
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("timeout", &self.timeout)
            .field("retry_delays", &self.retry_delays)
            .field("max_dave_protocol_version", &self.max_dave_protocol_version)
            .field("keepalive", &self.keepalive)
            .field("discovery_retry", &self.discovery_retry)
            .finish_non_exhaustive()
    }
}

/// The UDP side of a session: what sending audio needs. Kept by a resume, replaced when the
/// connection identifies anew (a new SSRC, socket and key).
#[derive(Debug)]
pub(crate) struct Udp {
    pub(crate) socket: UdpSocket,
    pub(crate) ssrc: u32,
    pub(crate) cipher: Cipher,
}

/// Why a connection could not be made (or resumed).
#[derive(Debug)]
pub(crate) enum ConnectError {
    /// The endpoint does not make a valid URL.
    Endpoint,
    /// The WebSocket could not connect, or failed.
    WebSocket(tokio_websockets::Error),
    /// The voice gateway closed the connection, with this code if it gave one.
    Closed(Option<u16>),
    /// The handshake took longer than [`Config::timeout`].
    Timeout,
    /// The UDP socket failed.
    Io(io::Error),
    /// A message could not be read.
    Decode(DecodeError),
    /// The voice server offers no encryption mode djs-voice supports.
    NoMode,
    /// The voice server did something the protocol does not allow.
    Unexpected(&'static str),
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Endpoint => f.write_str("the voice server endpoint is not a valid URL"),
            Self::WebSocket(error) => write!(f, "the voice gateway connection failed: {error}"),
            Self::Closed(Some(code)) => write!(f, "the voice gateway closed with code {code}"),
            Self::Closed(None) => f.write_str("the voice gateway connection was lost"),
            Self::Timeout => f.write_str("the voice connection timed out"),
            Self::Io(error) => write!(f, "the voice UDP socket failed: {error}"),
            Self::Decode(error) => error.fmt(f),
            Self::NoMode => f.write_str("the voice server offers no supported encryption mode"),
            Self::Unexpected(what) => write!(f, "the voice server misbehaved: {what}"),
        }
    }
}

impl std::error::Error for ConnectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WebSocket(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

/// An event of a connection, for its call.
#[derive(Debug)]
pub(crate) enum ConnectionEvent {
    /// A message the connection does not handle itself: speaking states, users joining and
    /// leaving (DAVE's are handled by the connection).
    Gateway(Event),
    /// The connection was lost and resumed: the same session.
    Resumed,
    /// The connection was lost and identified anew: a new session (SSRC and key).
    Reconnected,
    /// The connection ended for good.
    Ended(End),
}

/// Why a connection ended.
#[derive(Debug)]
pub(crate) enum End {
    /// Its call closed it.
    Closed,
    /// The voice gateway closed it with a code after which reconnecting cannot help.
    Stopped(u16),
    /// Reconnecting failed every time; the last failure.
    Lost(ConnectError),
}

/// What a call asks of its connection.
#[derive(Debug)]
pub(crate) enum Command {
    /// Send Opcode 5 Speaking with these flags (and again after identifying anew).
    Speaking(u32),
    /// Close the connection.
    Close,
}

/// A running connection, as its call holds it.
#[derive(Debug)]
pub(crate) struct Handle {
    commands: mpsc::UnboundedSender<Command>,
    dave: Arc<Mutex<Dave>>,
}

impl Handle {
    /// Sends a command to the connection; `false` once it has ended.
    pub(crate) fn send(&self, command: Command) -> bool {
        self.commands.send(command).is_ok()
    }

    /// The DAVE protocol version media is sent with now (0: no end-to-end encryption).
    pub(crate) fn dave_protocol_version(&self) -> u16 {
        lock(&self.dave).version()
    }
}

/// The DAVE state; a sender that panicked while holding it left nothing half-changed that
/// matters more than the call going on.
fn lock(dave: &Mutex<Dave>) -> MutexGuard<'_, Dave> {
    dave.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Connects to the voice server of `info`, keeps the connection in a task, and returns once
/// media can be sent (D-322: DAVE ready too), with the connection's only [`Sender`]: there
/// is one, as its nonce counter must not repeat under one key.
///
/// # Errors
///
/// When the handshake fails, the connection ends before DAVE is ready, or both take longer
/// than [`Config::timeout`] (the connection is closed then).
pub(crate) async fn connect(
    info: Info,
    config: Config,
) -> Result<(Handle, Sender, mpsc::UnboundedReceiver<ConnectionEvent>), ConnectError> {
    let deadline = Instant::now() + config.timeout;
    let session = timeout_at(deadline, identify(&info, &config))
        .await
        .map_err(|_| ConnectError::Timeout)??;
    let (commands, commands_rx) = mpsc::unbounded_channel();
    let (events, mut events_rx) = mpsc::unbounded_channel();
    let (udp, udp_rx) = watch::channel(Arc::clone(&session.udp));
    let (ready, mut ready_rx) = watch::channel(false);
    let dave = Arc::new(Mutex::new(Dave::new(
        info.user_id.get(),
        info.channel_id.get(),
    )));
    let handle = Handle {
        commands,
        dave: Arc::clone(&dave),
    };
    let sender = Sender::new(udp_rx, Arc::clone(&dave));
    let shared = Shared {
        events,
        udp,
        dave,
        ready,
    };
    tokio::spawn(run(info, config, session, commands_rx, shared));
    match timeout_at(deadline, ready_rx.wait_for(|ready| *ready)).await {
        Ok(Ok(_)) => Ok((handle, sender, events_rx)),
        // The task ended: it said why.
        Ok(Err(_)) => Err(ended(&mut events_rx)),
        Err(_) => {
            handle.send(Command::Close);
            Err(ConnectError::Timeout)
        }
    }
}

/// Why a connection that ended before it was ready did: its `Ended` event, as an error.
fn ended(events: &mut mpsc::UnboundedReceiver<ConnectionEvent>) -> ConnectError {
    while let Ok(event) = events.try_recv() {
        if let ConnectionEvent::Ended(end) = event {
            return match end {
                End::Lost(error) => error,
                End::Stopped(code) => ConnectError::Closed(Some(code)),
                End::Closed => ConnectError::Closed(None),
            };
        }
    }
    ConnectError::Closed(None)
}

/// What the connection's task shares with its call.
struct Shared {
    events: mpsc::UnboundedSender<ConnectionEvent>,
    /// The UDP side of the current session, for the senders.
    udp: watch::Sender<Arc<Udp>>,
    dave: Arc<Mutex<Dave>>,
    /// Whether media can be sent ([`Dave::ready`]).
    ready: watch::Sender<bool>,
}

impl Shared {
    fn event(&self, event: ConnectionEvent) {
        // The call may be gone; the connection ends with its handle.
        let _ = self.events.send(event);
    }

    /// Starts DAVE for a new session; returns the messages to send.
    fn start(&self, version: u16) -> Vec<Reply> {
        let mut dave = lock(&self.dave);
        let replies = dave.start(version);
        self.ready
            .send_if_modified(|ready| replace(ready, dave.ready()));
        replies
    }

    /// Handles an event of the voice gateway: DAVE's here (returns the messages to send),
    /// the others for the call.
    fn handle(&self, event: Event) -> Vec<Reply> {
        let mut dave = lock(&self.dave);
        let replies = match dave.handle(event) {
            Handled::Replies(replies) => replies,
            Handled::Forward(event) => {
                self.event(ConnectionEvent::Gateway(event));
                Vec::new()
            }
        };
        self.ready
            .send_if_modified(|ready| replace(ready, dave.ready()));
        replies
    }
}

/// Sets `value` to `new`; whether it changed.
fn replace(value: &mut bool, new: bool) -> bool {
    core::mem::replace(value, new) != new
}

/// An established session.
struct Session {
    socket: Socket,
    udp: Arc<Udp>,
    heartbeat: Heartbeat,
    /// The last sequence number received.
    seq: Option<u64>,
    /// The DAVE protocol version of a new session's Session Description, until DAVE starts
    /// with it; `None` for a resumed session.
    dave: Option<u16>,
    /// Messages received during the handshake, for the call.
    pending: Vec<Event>,
}

/// The heartbeat of a session.
struct Heartbeat {
    interval: Duration,
    next: Instant,
    nonce: u64,
    /// The nonce of the last heartbeat sent, until it is acknowledged.
    unacknowledged: Option<u64>,
}

impl Heartbeat {
    /// The heartbeat of an interval in milliseconds, from Hello; `None` if it is unusable.
    fn new(interval: f64) -> Option<Self> {
        let interval = Duration::try_from_secs_f64(interval / 1000.0).ok()?;
        (!interval.is_zero()).then(|| Self {
            interval,
            next: Instant::now() + interval,
            nonce: 0,
            unacknowledged: None,
        })
    }

    fn acknowledged(&mut self, nonce: u64) {
        if self.unacknowledged == Some(nonce) {
            self.unacknowledged = None;
        }
    }

    /// Sends the next heartbeat.
    async fn beat(&mut self, socket: &mut Socket, seq: Option<u64>) -> Result<(), ConnectError> {
        self.nonce = self.nonce.wrapping_add(1);
        self.unacknowledged = Some(self.nonce);
        self.next += self.interval;
        send(socket, Message::text(payload::heartbeat(self.nonce, seq))).await
    }
}

fn url(endpoint: &str) -> String {
    let base = if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
        endpoint.trim_end_matches('/').to_owned()
    } else {
        format!("wss://{}", endpoint.trim_end_matches('/'))
    };
    format!("{base}/?v={}", payload::VERSION)
}

async fn open(info: &Info, config: &Config) -> Result<Socket, ConnectError> {
    let (socket, _) = ClientBuilder::new()
        .uri(&url(&info.endpoint))
        .map_err(|_| ConnectError::Endpoint)?
        .connector(&config.tls)
        .connect()
        .await
        .map_err(ConnectError::WebSocket)?;
    Ok(socket)
}

async fn send(socket: &mut Socket, message: Message) -> Result<(), ConnectError> {
    socket.send(message).await.map_err(ConnectError::WebSocket)
}

/// Reads a message: `None` for control frames (pings, pongs).
fn decode(message: &Message) -> Option<Result<Incoming, DecodeError>> {
    if let Some(text) = message.as_text() {
        Some(payload::decode_text(text))
    } else if message.is_binary() {
        Some(payload::decode_binary(message.as_payload()))
    } else {
        None
    }
}

/// The next message during a handshake, heartbeating on the way once the interval is known
/// (acknowledgements are taken here).
async fn next(
    socket: &mut Socket,
    heartbeat: &mut Option<Heartbeat>,
    seq: &mut Option<u64>,
) -> Result<Event, ConnectError> {
    loop {
        let deadline = heartbeat.as_ref().map(|heartbeat| heartbeat.next);
        let message = tokio::select! {
            message = socket.next() => message,
            () = wait(deadline) => {
                if let Some(heartbeat) = heartbeat {
                    heartbeat.beat(socket, *seq).await?;
                }
                continue;
            }
        };
        let message = match message {
            None => return Err(ConnectError::Closed(None)),
            Some(Err(error)) => return Err(ConnectError::WebSocket(error)),
            Some(Ok(message)) => message,
        };
        if let Some((code, _)) = message.as_close() {
            return Err(ConnectError::Closed(Some(u16::from(code))));
        }
        let Some(incoming) = decode(&message) else {
            continue;
        };
        let Incoming { seq: number, event } = incoming.map_err(ConnectError::Decode)?;
        if number.is_some() {
            *seq = number;
        }
        match (event, heartbeat.as_mut()) {
            (Event::HeartbeatAck(nonce), Some(heartbeat)) => heartbeat.acknowledged(nonce),
            (Event::HeartbeatAck(_), None) => {}
            (event, _) => return Ok(event),
        }
    }
}

/// The heartbeat of a Hello.
fn hello(interval: f64) -> Result<Heartbeat, ConnectError> {
    Heartbeat::new(interval).ok_or(ConnectError::Unexpected(
        "Hello has no usable heartbeat interval",
    ))
}

/// Waits until `deadline`, or forever without one.
async fn wait(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => core::future::pending().await,
    }
}

/// A new session: Identify, then UDP, IP discovery and Select Protocol.
async fn identify(info: &Info, config: &Config) -> Result<Session, ConnectError> {
    let mut socket = open(info, config).await?;
    let identify = payload::identify(
        info.guild_id,
        info.user_id,
        &info.session_id,
        &info.token,
        config.max_dave_protocol_version,
    );
    send(&mut socket, Message::text(identify)).await?;
    let mut heartbeat = None;
    let mut ready = None;
    let mut seq = None;
    let mut pending = Vec::new();
    while heartbeat.is_none() || ready.is_none() {
        match next(&mut socket, &mut heartbeat, &mut seq).await? {
            Event::Hello(interval) => heartbeat = Some(hello(interval)?),
            Event::Ready(event) => ready = Some(event),
            event => pending.push(event),
        }
    }
    let ready = ready.ok_or(ConnectError::Unexpected("no Ready"))?;
    let mode = Mode::choose(ready.modes.iter().map(String::as_str)).ok_or(ConnectError::NoMode)?;
    let bind: SocketAddr = if ready.ip.is_ipv4() {
        (core::net::Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (core::net::Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket_udp = UdpSocket::bind(bind).await.map_err(ConnectError::Io)?;
    socket_udp
        .connect((ready.ip, ready.port))
        .await
        .map_err(ConnectError::Io)?;
    let address = discover(&socket_udp, ready.ssrc, config.discovery_retry).await?;
    let select = payload::select_protocol(address.ip(), address.port(), mode);
    send(&mut socket, Message::text(select)).await?;
    let description = loop {
        match next(&mut socket, &mut heartbeat, &mut seq).await? {
            Event::SessionDescription(description) => break description,
            event => pending.push(event),
        }
    };
    if Mode::from_name(&description.mode) != Some(mode) {
        return Err(ConnectError::Unexpected(
            "the voice server described another encryption mode",
        ));
    }
    let heartbeat = heartbeat.ok_or(ConnectError::Unexpected("no Hello"))?;
    let udp = Udp {
        socket: socket_udp,
        ssrc: ready.ssrc,
        cipher: Cipher::new(mode, &description.secret_key),
    };
    Ok(Session {
        socket,
        udp: Arc::new(udp),
        heartbeat,
        seq,
        dave: Some(description.dave_protocol_version),
        pending,
    })
}

/// IP discovery: asks the voice server for the address it sees this socket at, again every
/// `retry` until it answers.
async fn discover(
    socket: &UdpSocket,
    ssrc: u32,
    retry: Duration,
) -> Result<SocketAddr, ConnectError> {
    let request = udp::discovery_request(ssrc);
    let mut buffer = [0; 1500];
    loop {
        socket.send(&request).await.map_err(ConnectError::Io)?;
        let deadline = Instant::now() + retry;
        while let Ok(received) = timeout_at(deadline, socket.recv(&mut buffer)).await {
            let len = received.map_err(ConnectError::Io)?;
            if let Some(address) = udp::discovery_response(&buffer[..len], ssrc) {
                return Ok(address);
            }
        }
    }
}

/// Resumes a session on a new connection: Resume, then Hello and Resumed.
async fn resume(
    info: &Info,
    config: &Config,
    udp: Arc<Udp>,
    seq: Option<u64>,
) -> Result<Session, ConnectError> {
    let mut socket = open(info, config).await?;
    let resume = payload::resume(info.guild_id, &info.session_id, &info.token, seq);
    send(&mut socket, Message::text(resume)).await?;
    let mut heartbeat = None;
    let mut seq = seq;
    let mut pending = Vec::new();
    let mut resumed = false;
    while heartbeat.is_none() || !resumed {
        match next(&mut socket, &mut heartbeat, &mut seq).await? {
            Event::Hello(interval) => heartbeat = Some(hello(interval)?),
            Event::Resumed => resumed = true,
            event => pending.push(event),
        }
    }
    let heartbeat = heartbeat.ok_or(ConnectError::Unexpected("no Hello"))?;
    Ok(Session {
        socket,
        udp,
        heartbeat,
        seq,
        dave: None,
        pending,
    })
}

/// How [`drive`] ended.
enum Drive {
    /// The call asked to close.
    Closed,
    /// The connection was lost, with the close code if there was one.
    Lost(Option<u16>),
}

/// The connection's task.
async fn run(
    info: Info,
    config: Config,
    mut session: Session,
    mut commands: mpsc::UnboundedReceiver<Command>,
    shared: Shared,
) {
    let mut speaking = None;
    loop {
        // Failures to send show as a lost connection in `drive`.
        if let Some(version) = session.dave.take() {
            let replies = shared.start(version);
            let _ = send_replies(&mut session.socket, replies).await;
        }
        for event in core::mem::take(&mut session.pending) {
            let replies = shared.handle(event);
            let _ = send_replies(&mut session.socket, replies).await;
        }
        let code = match drive(&mut session, &mut commands, &shared, &mut speaking, &config).await {
            Drive::Closed => {
                shared.event(ConnectionEvent::Ended(End::Closed));
                // The closing handshake, bounded: the call does not wait for it.
                let _ = timeout(CLOSE_TIMEOUT, session.socket.close()).await;
                return;
            }
            Drive::Lost(code) => code,
        };
        tracing::debug!(guild = %info.guild_id, ?code, "voice connection lost");
        match reconnect(&info, &config, &session, code, &mut commands, &mut speaking).await {
            Ok((mut new, resumed)) => {
                if !resumed {
                    shared.udp.send_replace(Arc::clone(&new.udp));
                    if let Some(flags) = speaking {
                        let message = Message::text(payload::speaking(flags, new.udp.ssrc));
                        // A failure shows as a lost connection on the next turn.
                        let _ = send(&mut new.socket, message).await;
                    }
                }
                session = new;
                shared.event(if resumed {
                    ConnectionEvent::Resumed
                } else {
                    ConnectionEvent::Reconnected
                });
            }
            Err(end) => {
                shared.event(ConnectionEvent::Ended(end));
                return;
            }
        }
    }
}

/// Sends DAVE's messages.
async fn send_replies(socket: &mut Socket, replies: Vec<Reply>) -> Result<(), ConnectError> {
    for reply in replies {
        let message = match reply {
            Reply::Text(text) => Message::text(text),
            Reply::Binary(bytes) => Message::binary(bytes),
        };
        send(socket, message).await?;
    }
    Ok(())
}

/// Keeps an established connection until it is lost or the call closes it.
async fn drive(
    session: &mut Session,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    shared: &Shared,
    speaking: &mut Option<u32>,
    config: &Config,
) -> Drive {
    let mut keepalive = Instant::now() + config.keepalive;
    loop {
        tokio::select! {
            message = session.socket.next() => {
                let Some(Ok(message)) = message else {
                    return Drive::Lost(None);
                };
                if let Some((code, _)) = message.as_close() {
                    return Drive::Lost(Some(u16::from(code)));
                }
                match decode(&message) {
                    None => {}
                    Some(Err(error)) => {
                        tracing::warn!(%error, "ignoring a voice gateway message");
                    }
                    Some(Ok(Incoming { seq, event })) => {
                        if seq.is_some() {
                            session.seq = seq;
                        }
                        match event {
                            Event::HeartbeatAck(nonce) => session.heartbeat.acknowledged(nonce),
                            Event::Hello(interval) => {
                                if let Some(heartbeat) = Heartbeat::new(interval) {
                                    session.heartbeat = heartbeat;
                                }
                            }
                            event => {
                                let replies = shared.handle(event);
                                if send_replies(&mut session.socket, replies).await.is_err() {
                                    return Drive::Lost(None);
                                }
                            }
                        }
                    }
                }
            }
            () = sleep_until(session.heartbeat.next) => {
                if session.heartbeat.unacknowledged.is_some() {
                    tracing::warn!("a voice heartbeat was not acknowledged; reconnecting");
                    return Drive::Lost(None);
                }
                if session.heartbeat.beat(&mut session.socket, session.seq).await.is_err() {
                    return Drive::Lost(None);
                }
            }
            () = sleep_until(keepalive) => {
                // Best effort: a full socket buffer only skips one.
                let _ = session.udp.socket.try_send(&udp::keepalive(session.udp.ssrc));
                keepalive += config.keepalive;
            }
            command = commands.recv() => {
                let message = match command {
                    None | Some(Command::Close) => return Drive::Closed,
                    Some(Command::Speaking(flags)) => {
                        *speaking = Some(flags);
                        Message::text(payload::speaking(flags, session.udp.ssrc))
                    }
                };
                if send(&mut session.socket, message).await.is_err() {
                    return Drive::Lost(None);
                }
            }
        }
    }
}

/// Reconnects after a lost connection as its close code says: resumes (D-331: at once,
/// then after each delay), identifies anew, or stops. Returns the new session and whether
/// it was resumed.
async fn reconnect(
    info: &Info,
    config: &Config,
    session: &Session,
    code: Option<u16>,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    speaking: &mut Option<u32>,
) -> Result<(Session, bool), End> {
    match payload::after_close(code) {
        AfterClose::Stop => return Err(End::Stopped(code.unwrap_or_default())),
        AfterClose::Identify => return renew(info, config).await.map(|session| (session, false)),
        AfterClose::Resume => {}
    }
    let mut last = ConnectError::Closed(code);
    for delay in core::iter::once(Duration::ZERO).chain(config.retry_delays.iter().copied()) {
        pause(jitter(delay), commands, speaking).await?;
        let attempt = timeout(
            config.timeout,
            resume(info, config, Arc::clone(&session.udp), session.seq),
        )
        .await
        .unwrap_or(Err(ConnectError::Timeout));
        match attempt {
            Ok(new) => return Ok((new, true)),
            Err(ConnectError::Closed(code)) => match payload::after_close(code) {
                AfterClose::Resume => last = ConnectError::Closed(code),
                AfterClose::Identify => {
                    return renew(info, config).await.map(|session| (session, false));
                }
                AfterClose::Stop => return Err(End::Stopped(code.unwrap_or_default())),
            },
            Err(error) => last = error,
        }
        tracing::debug!(guild = %info.guild_id, error = %last, "resuming a voice connection failed");
    }
    Err(End::Lost(last))
}

/// A new session after the old one became invalid (D-331: once).
async fn renew(info: &Info, config: &Config) -> Result<Session, End> {
    timeout(config.timeout, identify(info, config))
        .await
        .unwrap_or(Err(ConnectError::Timeout))
        .map_err(End::Lost)
}

/// Half to all of `delay`, so that calls lost together do not retry together.
fn jitter(delay: Duration) -> Duration {
    delay.mul_f64(0.5 + fastrand::f64() / 2.0)
}

/// Waits `delay` between attempts, still taking commands: a close ends the wait (and the
/// connection), speaking flags are kept for the next session.
async fn pause(
    delay: Duration,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    speaking: &mut Option<u32>,
) -> Result<(), End> {
    let deadline = Instant::now() + delay;
    loop {
        tokio::select! {
            () = sleep_until(deadline) => return Ok(()),
            command = commands.recv() => match command {
                None | Some(Command::Close) => return Err(End::Closed),
                Some(Command::Speaking(flags)) => *speaking = Some(flags),
            },
        }
    }
}

/// Sends audio: Opus packets as RTP packets of the current session, end-to-end encrypted
/// with DAVE when the call uses it, then encrypted for the transport.
#[derive(Debug)]
pub(crate) struct Sender {
    udp: watch::Receiver<Arc<Udp>>,
    dave: Arc<Mutex<Dave>>,
    sequence: u16,
    timestamp: u32,
    nonce: u32,
    /// The packet being built, reused.
    buffer: Vec<u8>,
}

impl Sender {
    fn new(udp: watch::Receiver<Arc<Udp>>, dave: Arc<Mutex<Dave>>) -> Self {
        Self {
            udp,
            dave,
            // Random starts, as RTP recommends (RFC 3550, 5.1).
            sequence: fastrand::u16(..),
            timestamp: fastrand::u32(..),
            nonce: 0,
            buffer: Vec::with_capacity(RTP_HEADER + 1275 + OVERHEAD),
        }
    }

    /// Sends one Opus packet that lasts `samples` samples (at 48 kHz). Returns `false`, and
    /// sends nothing, while DAVE has no key to encrypt with (after identifying anew, until
    /// the bot is in the new group); the RTP timestamp moves on all the same.
    ///
    /// # Errors
    ///
    /// When the UDP socket fails.
    pub(crate) async fn send(&mut self, opus: &[u8], samples: u32) -> io::Result<bool> {
        let udp = Arc::clone(&*self.udp.borrow());
        let timestamp = self.timestamp;
        self.timestamp = timestamp.wrapping_add(samples);
        let len = {
            let mut dave = lock(&self.dave);
            let Some(frame) = dave.encrypt(opus) else {
                return Ok(false);
            };
            self.buffer.clear();
            self.buffer
                .extend_from_slice(&udp::rtp_header(self.sequence, timestamp, udp.ssrc));
            self.buffer.extend_from_slice(frame);
            frame.len()
        };
        self.buffer.resize(RTP_HEADER + len + OVERHEAD, 0);
        let sealed = udp
            .cipher
            .seal(&mut self.buffer, RTP_HEADER, len, self.nonce)
            .ok_or_else(|| io::Error::other("the packet could not be encrypted"))?;
        self.nonce = self.nonce.wrapping_add(1);
        self.sequence = self.sequence.wrapping_add(1);
        udp.socket.send(&self.buffer[..sealed]).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
