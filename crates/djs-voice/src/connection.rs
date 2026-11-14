//! A voice connection (step 1c): the voice gateway WebSocket and the UDP socket of one call,
//! from the handshake to the end.
//!
//! [`connect`] runs the handshake (the WebSocket, Identify, Hello and Ready, the UDP socket
//! and IP discovery, Select Protocol, Session Description), then a task that keeps the
//! connection: it heartbeats with the `seq_ack` of version 8, sends the UDP keepalive,
//! passes the messages it does not handle itself to the call as [`ConnectionEvent`]s, and
//! after a lost connection resumes or identifies anew as [`payload::after_close`] says,
//! retrying as D-331 says. Audio goes out through a [`Sender`], straight to the UDP socket
//! of the current session.

#![allow(dead_code, reason = "used by `Call` from step 1e")]

use core::{fmt, time::Duration};
use std::{io, net::SocketAddr, sync::Arc};

use djs_model::id::{GuildId, UserId};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpStream, UdpSocket},
    sync::{mpsc, watch},
    time::{Instant, sleep_until, timeout, timeout_at},
};
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

use crate::{
    crypto::{Cipher, Mode, OVERHEAD, RTP_HEADER},
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
    /// The longest a handshake (or a resume) may take (D-322).
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
    /// leaving, and (from step 1d) DAVE.
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
    /// Send a JSON message (DAVE replies).
    Text(String),
    /// Send a binary message (DAVE key packages and commits).
    Binary(Vec<u8>),
    /// Close the connection.
    Close,
}

/// A running connection, as its call holds it.
#[derive(Debug)]
pub(crate) struct Handle {
    commands: mpsc::UnboundedSender<Command>,
    udp: watch::Receiver<Arc<Udp>>,
    dave_protocol_version: u16,
}

impl Handle {
    /// Sends a command to the connection; `false` once it has ended.
    pub(crate) fn send(&self, command: Command) -> bool {
        self.commands.send(command).is_ok()
    }

    /// A sender of audio packets.
    pub(crate) fn sender(&self) -> Sender {
        Sender::new(self.udp.clone())
    }

    /// The DAVE protocol version the call started with (0: no end-to-end encryption).
    pub(crate) const fn dave_protocol_version(&self) -> u16 {
        self.dave_protocol_version
    }
}

/// Connects to the voice server of `info` and keeps the connection in a task.
///
/// # Errors
///
/// When the handshake fails or takes longer than [`Config::timeout`].
pub(crate) async fn connect(
    info: Info,
    config: Config,
) -> Result<(Handle, mpsc::UnboundedReceiver<ConnectionEvent>), ConnectError> {
    let (session, dave_protocol_version) = timeout(config.timeout, identify(&info, &config))
        .await
        .map_err(|_| ConnectError::Timeout)??;
    let (commands, commands_rx) = mpsc::unbounded_channel();
    let (events, events_rx) = mpsc::unbounded_channel();
    let (udp, udp_rx) = watch::channel(Arc::clone(&session.udp));
    let handle = Handle {
        commands,
        udp: udp_rx,
        dave_protocol_version,
    };
    tokio::spawn(run(info, config, session, commands_rx, events, udp));
    Ok((handle, events_rx))
}

/// An established session.
struct Session {
    socket: Socket,
    udp: Arc<Udp>,
    heartbeat: Heartbeat,
    /// The last sequence number received.
    seq: Option<u64>,
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

/// A new session: Identify, then UDP, IP discovery and Select Protocol. Returns the DAVE
/// version the call starts with too.
async fn identify(info: &Info, config: &Config) -> Result<(Session, u16), ConnectError> {
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
    let session = Session {
        socket,
        udp: Arc::new(udp),
        heartbeat,
        seq,
        pending,
    };
    Ok((session, description.dave_protocol_version))
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
    events: mpsc::UnboundedSender<ConnectionEvent>,
    udp: watch::Sender<Arc<Udp>>,
) {
    let mut speaking = None;
    loop {
        for event in session.pending.drain(..) {
            let _ = events.send(ConnectionEvent::Gateway(event));
        }
        let code = match drive(&mut session, &mut commands, &events, &mut speaking, &config).await {
            Drive::Closed => {
                let _ = events.send(ConnectionEvent::Ended(End::Closed));
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
                    udp.send_replace(Arc::clone(&new.udp));
                    if let Some(flags) = speaking {
                        let message = Message::text(payload::speaking(flags, new.udp.ssrc));
                        // A failure shows as a lost connection on the next turn.
                        let _ = send(&mut new.socket, message).await;
                    }
                }
                session = new;
                let _ = events.send(if resumed {
                    ConnectionEvent::Resumed
                } else {
                    ConnectionEvent::Reconnected
                });
            }
            Err(end) => {
                let _ = events.send(ConnectionEvent::Ended(end));
                return;
            }
        }
    }
}

/// Keeps an established connection until it is lost or the call closes it.
async fn drive(
    session: &mut Session,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    events: &mpsc::UnboundedSender<ConnectionEvent>,
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
                                let _ = events.send(ConnectionEvent::Gateway(event));
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
                    Some(Command::Text(text)) => Message::text(text),
                    Some(Command::Binary(bytes)) => Message::binary(bytes),
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
        .map(|(session, _)| session)
        .map_err(End::Lost)
}

/// Half to all of `delay`, so that calls lost together do not retry together.
fn jitter(delay: Duration) -> Duration {
    delay.mul_f64(0.5 + fastrand::f64() / 2.0)
}

/// Waits `delay` between attempts, still taking commands: a close ends the wait (and the
/// connection), speaking flags are kept for the next session, the rest is dropped (it
/// belonged to the lost session).
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
                Some(Command::Text(_) | Command::Binary(_)) => {}
            },
        }
    }
}

/// Sends audio: Opus packets as encrypted RTP packets of the current session.
#[derive(Debug)]
pub(crate) struct Sender {
    udp: watch::Receiver<Arc<Udp>>,
    sequence: u16,
    timestamp: u32,
    nonce: u32,
    /// The packet being built, reused.
    buffer: Vec<u8>,
}

impl Sender {
    fn new(udp: watch::Receiver<Arc<Udp>>) -> Self {
        Self {
            udp,
            // Random starts, as RTP recommends (RFC 3550, 5.1).
            sequence: fastrand::u16(..),
            timestamp: fastrand::u32(..),
            nonce: 0,
            buffer: Vec::with_capacity(RTP_HEADER + 1275 + OVERHEAD),
        }
    }

    /// Sends one Opus packet that lasts `samples` samples (at 48 kHz).
    ///
    /// # Errors
    ///
    /// When the UDP socket fails.
    pub(crate) async fn send(&mut self, opus: &[u8], samples: u32) -> io::Result<()> {
        let udp = Arc::clone(&*self.udp.borrow());
        let len = RTP_HEADER + opus.len() + OVERHEAD;
        self.buffer.clear();
        self.buffer
            .extend_from_slice(&udp::rtp_header(self.sequence, self.timestamp, udp.ssrc));
        self.buffer.extend_from_slice(opus);
        self.buffer.resize(len, 0);
        let sealed = udp
            .cipher
            .seal(&mut self.buffer, RTP_HEADER, opus.len(), self.nonce)
            .ok_or_else(|| io::Error::other("the packet could not be encrypted"))?;
        self.nonce = self.nonce.wrapping_add(1);
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(samples);
        udp.socket.send(&self.buffer[..sealed]).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests;
