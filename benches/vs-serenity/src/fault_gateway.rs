//! A mock gateway that fails on purpose (D-279, reliability).
//!
//! Like Discord, it produces events for the *session*, not the connection: once a client has
//! identified, a producer appends numbered `MESSAGE_CREATE`s to the current session at a
//! steady pace whether a connection is open or not, a connection sends what its session has
//! not sent yet, `RESUME` replays the rest, and a new `IDENTIFY` starts a new session — the
//! old session's unsent events are then lost, as on Discord. At one message the scenario's
//! fault happens:
//!
//! | `--scenario` | The fault | A correct client |
//! |---|---|---|
//! | `drop` | the TCP connection is closed, without a close frame | reconnects and resumes: nothing lost |
//! | `reconnect` | opcode 7 `RECONNECT` | reconnects and resumes: nothing lost |
//! | `invalid` | the connection drops and the session cannot be resumed (`INVALID_SESSION`, `d: false`) | identifies again; what was produced meanwhile is lost, by the protocol |
//! | `zombie` | no more heartbeat ACKs and nothing sent, the connection stays open | notices the missing ACK, reconnects and resumes |
//! | `close4000` | close code 4000 (unknown error, resumable) | reconnects and resumes |
//! | `close4004` | close code 4004 (authentication failed) | stops for good, without reconnecting |
//!
//! Message `k` (1-based, across sessions) has the ID and the text `message number k` of
//! [`message_create`]; its `s` is its sequence in its session. What happened (connections,
//! identifies, resumes, replayed events, the fault's time, and when the first `READY` or
//! `RESUMED` after it ended the outage) is written to `--report` as JSON after every change.

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use djs_vs_node::{
    Options,
    payloads::{message_create, ready},
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{CloseCode, Message, ServerBuilder, WebSocketStream};

/// The fault of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Drop,
    Reconnect,
    Invalid,
    Zombie,
    Close4000,
    Close4004,
}

impl Scenario {
    fn parse(name: &str) -> Self {
        match name {
            "drop" => Self::Drop,
            "reconnect" => Self::Reconnect,
            "invalid" => Self::Invalid,
            "zombie" => Self::Zombie,
            "close4000" => Self::Close4000,
            "close4004" => Self::Close4004,
            other => panic!(
                "unknown --scenario {other} (drop, reconnect, invalid, zombie, close4000, close4004)"
            ),
        }
    }
}

/// One session's events (`READY` is sequence 1), kept for `RESUME`.
#[derive(Debug)]
struct Session {
    sequence: u64,
    events: Vec<(u64, String)>,
    resumable: bool,
}

/// What the mock counted, written to the report.
#[derive(Debug, Default)]
struct Counts {
    produced: u64,
    fault_ms: Option<f64>,
    connections: u64,
    connections_after_fault: u64,
    first_connection_after_fault_ms: Option<f64>,
    /// When the first `READY` or `RESUMED` after the fault was sent.
    recovered_ms: Option<f64>,
    identifies: u64,
    resumes: u64,
    resumed: u64,
    invalid_sessions: u64,
    replayed: u64,
    ignored_heartbeats: u64,
}

#[derive(Debug)]
struct State {
    scenario: Scenario,
    messages: u64,
    fault_at: u64,
    interval: Duration,
    heartbeat_ms: u64,
    url: String,
    report: PathBuf,
    started: Instant,
    counts: Mutex<Counts>,
    sessions: Mutex<HashMap<String, Session>>,
    /// The session the producer appends to: the last one identified.
    current: Mutex<Option<String>>,
    producer_started: AtomicBool,
    /// Set by the producer at the fault's message, taken by the connection that sends it.
    fault_pending: AtomicBool,
}

impl State {
    fn ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }

    fn counts(&self) -> MutexGuard<'_, Counts> {
        self.counts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<String, Session>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn current(&self) -> MutexGuard<'_, Option<String>> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Records a `READY` or `RESUMED`: the first one after the fault ends the outage.
    fn recovered(&self) {
        let now = self.ms();
        let mut counts = self.counts();
        if counts.fault_ms.is_some() {
            counts.recovered_ms.get_or_insert(now);
        }
    }

    /// Rewrites the report file with the current counts.
    fn write_report(&self) {
        let report = {
            let c = self.counts();
            json!({
                "scenario": format!("{:?}", self.scenario).to_lowercase(),
                "messages": self.messages,
                "fault_at": self.fault_at,
                "heartbeat_ms": self.heartbeat_ms,
                "produced": c.produced,
                "fault_ms": c.fault_ms,
                "connections": c.connections,
                "connections_after_fault": c.connections_after_fault,
                "reconnect_ms": c
                    .first_connection_after_fault_ms
                    .zip(c.fault_ms)
                    .map(|(connected, fault)| connected - fault),
                "recovered_ms": c
                    .recovered_ms
                    .zip(c.fault_ms)
                    .map(|(recovered, fault)| recovered - fault),
                "identifies": c.identifies,
                "resumes": c.resumes,
                "resumed": c.resumed,
                "invalid_sessions": c.invalid_sessions,
                "replayed": c.replayed,
                "ignored_heartbeats": c.ignored_heartbeats,
            })
        };
        let _ = fs::write(&self.report, report.to_string());
    }
}

/// `fault-gateway --scenario S --messages N --fault-at K --report FILE [--interval-us U]
/// [--heartbeat-ms H]`: serves the gateway on a free port after printing
/// `{"gateway": "ws://127.0.0.1:<port>"}`.
pub(crate) async fn run(options: &Options) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let url = format!("ws://{}", listener.local_addr().expect("bound"));
    let messages = options.get("messages", 3000);
    let state = Arc::new(State {
        scenario: Scenario::parse(options.required("scenario")),
        messages,
        fault_at: options.get("fault-at", messages / 2),
        interval: Duration::from_micros(options.get("interval-us", 1000)),
        heartbeat_ms: options.get("heartbeat-ms", 1000),
        url: url.clone(),
        report: PathBuf::from(options.required("report")),
        started: Instant::now(),
        counts: Mutex::default(),
        sessions: Mutex::default(),
        current: Mutex::default(),
        producer_started: AtomicBool::new(false),
        fault_pending: AtomicBool::new(false),
    });
    state.write_report();
    println!("{}", json!({ "gateway": url }));
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(error) = serve(stream, &state).await {
                eprintln!("[fault-gateway] connection ended: {error}");
            }
        });
    }
}

/// Appends the numbered messages to the current session at a steady pace, until all are
/// produced; at the fault's message, raises the fault for the connection that sends it.
async fn produce(state: Arc<State>) {
    let mut ticker = tokio::time::interval(state.interval);
    loop {
        ticker.tick().await;
        let index = {
            let mut counts = state.counts();
            if counts.produced >= state.messages {
                return;
            }
            counts.produced += 1;
            counts.produced
        };
        let Some(current) = state.current().clone() else {
            continue;
        };
        if let Some(session) = state.sessions().get_mut(&current) {
            session.sequence += 1;
            let payload = numbered_message(index, session.sequence);
            session.events.push((session.sequence, payload));
        }
        if index == state.fault_at {
            state.counts().fault_ms = Some(state.ms());
            state.fault_pending.store(true, Ordering::Release);
            state.write_report();
        }
    }
}

/// The opcode and data of a client payload.
fn parse(message: &Message) -> Option<(u64, Value)> {
    let value: Value = serde_json::from_str(message.as_text()?).ok()?;
    Some((value["op"].as_u64()?, value["d"].clone()))
}

/// `MESSAGE_CREATE` number `index`, sent as event `sequence` of its session.
fn numbered_message(index: u64, sequence: u64) -> String {
    let payload = message_create(index, index % 1000);
    let prefix = format!(r#"{{"t":"MESSAGE_CREATE","s":{index},"#);
    let rest = payload
        .strip_prefix(&prefix)
        .expect("message_create starts with its sequence");
    format!(r#"{{"t":"MESSAGE_CREATE","s":{sequence},{rest}"#)
}

const ACK: &str = r#"{"op":11,"d":null}"#;

/// How a connection's session began.
struct Handshake {
    session: String,
    /// The last sequence the client has.
    sequence: u64,
    resumed: bool,
}

/// Answers heartbeats and waits for `IDENTIFY` or `RESUME`.
async fn handshake(
    ws: &mut WebSocketStream<TcpStream>,
    state: &Arc<State>,
) -> Result<Option<Handshake>, tokio_websockets::Error> {
    while let Some(message) = ws.next().await {
        let Some((op, d)) = parse(&message?) else {
            continue;
        };
        match op {
            1 => ws.send(Message::text(ACK.to_owned())).await?,
            2 => {
                let id = {
                    let mut counts = state.counts();
                    counts.identifies += 1;
                    format!("session-{}", counts.identifies)
                };
                let ready = ready(0)
                    .replace("0123456789abcdef0123456789abcdef", &id)
                    .replace("ws://127.0.0.1:9", &state.url);
                state.sessions().insert(
                    id.clone(),
                    Session {
                        sequence: 1,
                        events: Vec::new(),
                        resumable: true,
                    },
                );
                *state.current() = Some(id.clone());
                ws.send(Message::text(ready)).await?;
                state.recovered();
                if !state.producer_started.swap(true, Ordering::AcqRel) {
                    tokio::spawn(produce(Arc::clone(state)));
                }
                state.write_report();
                return Ok(Some(Handshake {
                    session: id,
                    sequence: 1,
                    resumed: false,
                }));
            }
            6 => {
                state.counts().resumes += 1;
                let id = d["session_id"].as_str().unwrap_or_default().to_owned();
                let seq = d["seq"].as_u64().unwrap_or(0);
                let resumable = state
                    .sessions()
                    .get(&id)
                    .is_some_and(|session| session.resumable);
                if resumable {
                    state.counts().resumed += 1;
                    state.write_report();
                    // The events after `seq` are sent by the loop, counted as replayed.
                    return Ok(Some(Handshake {
                        session: id,
                        sequence: seq,
                        resumed: true,
                    }));
                }
                state.counts().invalid_sessions += 1;
                ws.send(Message::text(r#"{"op":9,"d":false}"#.to_owned()))
                    .await?;
                state.write_report();
                // The client identifies again, on this connection or a new one.
            }
            _ => {}
        }
    }
    Ok(None)
}

/// What a connection does after its fault.
enum After {
    /// It is closed.
    Close,
    /// It stays open, sending nothing more.
    Quiet,
    /// It stays open, sending nothing and acknowledging no heartbeat.
    Zombie,
}

/// Performs the scenario's fault on the connection of `session`.
async fn fault(
    ws: &mut WebSocketStream<TcpStream>,
    state: &State,
    session: &str,
) -> Result<After, tokio_websockets::Error> {
    Ok(match state.scenario {
        // Dropping the stream closes the TCP connection without a close frame.
        Scenario::Drop => After::Close,
        Scenario::Invalid => {
            if let Some(s) = state.sessions().get_mut(session) {
                s.resumable = false;
            }
            After::Close
        }
        Scenario::Reconnect => {
            ws.send(Message::text(r#"{"op":7,"d":null}"#.to_owned()))
                .await?;
            After::Quiet
        }
        Scenario::Zombie => After::Zombie,
        Scenario::Close4000 | Scenario::Close4004 => {
            let code = if state.scenario == Scenario::Close4000 {
                4000
            } else {
                4004
            };
            let code = CloseCode::try_from(code).expect("a valid close code");
            ws.send(Message::close(Some(code), "fault")).await?;
            After::Close
        }
    })
}

async fn serve(stream: TcpStream, state: &Arc<State>) -> Result<(), tokio_websockets::Error> {
    stream.set_nodelay(true)?;
    let (_, mut ws) = ServerBuilder::new().accept(stream).await?;
    {
        let now = state.ms();
        let mut counts = state.counts();
        counts.connections += 1;
        if counts.fault_ms.is_some() {
            counts.connections_after_fault += 1;
            counts.first_connection_after_fault_ms.get_or_insert(now);
        }
    }
    state.write_report();
    ws.send(Message::text(format!(
        r#"{{"t":null,"s":null,"op":10,"d":{{"heartbeat_interval":{}}}}}"#,
        state.heartbeat_ms
    )))
    .await?;
    let Some(Handshake {
        session,
        sequence: mut sent,
        resumed: mut replaying,
    }) = handshake(&mut ws, state).await?
    else {
        return Ok(());
    };

    let mut ticker = tokio::time::interval(state.interval);
    let (mut sending, mut acking) = (true, true);
    loop {
        tokio::select! {
            message = ws.next() => {
                let Some(Ok(message)) = message else { return Ok(()) };
                if let Some((1, _)) = parse(&message) {
                    if acking {
                        ws.send(Message::text(ACK.to_owned())).await?;
                    } else {
                        state.counts().ignored_heartbeats += 1;
                        state.write_report();
                    }
                }
            }
            _ = ticker.tick(), if sending => {
                let batch: Vec<(u64, String)> = state
                    .sessions()
                    .get(&session)
                    .map(|s| {
                        let first = s.events.partition_point(|(sequence, _)| *sequence <= sent);
                        s.events[first..].to_vec()
                    })
                    .unwrap_or_default();
                if let Some(&(last, _)) = batch.last() {
                    for (_, payload) in &batch {
                        ws.feed(Message::text(payload.clone())).await?;
                    }
                    if replaying {
                        // A resumed session first gets what it missed, then `RESUMED`.
                        state.counts().replayed += batch.len() as u64;
                        ws.feed(Message::text(format!(
                            r#"{{"t":"RESUMED","s":{last},"op":0,"d":{{}}}}"#
                        )))
                        .await?;
                        replaying = false;
                        state.recovered();
                        state.write_report();
                    }
                    ws.flush().await?;
                    sent = last;
                } else if replaying {
                    ws.send(Message::text(format!(
                        r#"{{"t":"RESUMED","s":{sent},"op":0,"d":{{}}}}"#
                    )))
                    .await?;
                    replaying = false;
                    state.recovered();
                    state.write_report();
                }
                let is_current = state.current().as_deref() == Some(session.as_str());
                if is_current && state.fault_pending.swap(false, Ordering::AcqRel) {
                    match fault(&mut ws, state, &session).await? {
                        After::Close => return Ok(()),
                        After::Quiet => sending = false,
                        After::Zombie => (sending, acking) = (false, false),
                    }
                }

            }
        }
    }
}
