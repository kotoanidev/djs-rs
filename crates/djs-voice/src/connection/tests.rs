//! The connection against a mock voice server: a WebSocket server and a UDP socket on
//! `127.0.0.1` that answer IP discovery and decrypt the audio they receive.

use core::time::Duration;
use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    net::{TcpListener, TcpStream, UdpSocket},
    sync::mpsc::UnboundedReceiver,
    time::timeout,
};
use tokio_websockets::{CloseCode, Message, ServerBuilder, WebSocketStream};

use super::{Command, Config, ConnectError, ConnectionEvent, End, Info, connect, jitter, url};
use crate::{
    crypto::{Cipher, Mode},
    payload::{Event, MICROPHONE},
    udp::DISCOVERY,
};

type Server = WebSocketStream<TcpStream>;

const STEP: Duration = Duration::from_secs(5);
const KEY: [u8; 32] = [7; 32];
const OTHER_KEY: [u8; 32] = [9; 32];
/// The address the mock says it sees the client at, so Select Protocol must carry it.
const SEEN_AT: &str = "203.0.113.5";
const SEEN_PORT: u16 = 4242;

struct Mock {
    listener: TcpListener,
    udp: UdpSocket,
    endpoint: String,
}

impl Mock {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        Self {
            listener,
            udp,
            endpoint,
        }
    }

    fn info(&self) -> Info {
        Info {
            guild_id: djs_model::id::GuildId::new(1),
            user_id: djs_model::id::UserId::new(2),
            session_id: "session".into(),
            endpoint: self.endpoint.clone(),
            token: "voice-token".into(),
        }
    }

    async fn accept(&self) -> Server {
        let (stream, _) = timeout(STEP, self.listener.accept())
            .await
            .expect("no connection")
            .unwrap();
        let (request, server) = ServerBuilder::new().accept(stream).await.unwrap();
        assert_eq!(request.uri().query(), Some("v=8"));
        server
    }

    /// The server's side of a handshake: returns the client's Identify. Sends an
    /// undocumented opcode with sequence number 1 between Ready and Session Description.
    async fn handshake(
        &self,
        server: &mut Server,
        ssrc: u32,
        key: [u8; 32],
        interval: f64,
    ) -> Value {
        send(
            server,
            json!({"op": 8, "d": {"heartbeat_interval": interval}}),
        )
        .await;
        let identify = recv_op(server, 0).await;
        self.ready(
            server,
            ssrc,
            &["aead_aes256_gcm_rtpsize", "aead_xchacha20_poly1305_rtpsize"],
        )
        .await;
        self.discovery(ssrc).await;
        send(
            server,
            json!({"op": 18, "d": {"user_id": "5", "flags": 2}, "seq": 1}),
        )
        .await;
        let select = recv_op(server, 1).await;
        assert_eq!(
            select["d"],
            json!({"protocol": "udp", "data": {
                "address": SEEN_AT, "port": SEEN_PORT, "mode": "aead_aes256_gcm_rtpsize"
            }})
        );
        send(
            server,
            json!({"op": 4, "d": {
                "mode": "aead_aes256_gcm_rtpsize", "secret_key": key, "dave_protocol_version": 0
            }}),
        )
        .await;
        identify
    }

    async fn ready(&self, server: &mut Server, ssrc: u32, modes: &[&str]) {
        let port = self.udp.local_addr().unwrap().port();
        send(
            server,
            json!({"op": 2, "d": {
                "ssrc": ssrc, "ip": "127.0.0.1", "port": port, "modes": modes,
                "heartbeat_interval": 1
            }}),
        )
        .await;
    }

    /// Answers the client's IP discovery request.
    async fn discovery(&self, ssrc: u32) {
        let mut buffer = [0; 1500];
        let (len, from) = timeout(STEP, self.udp.recv_from(&mut buffer))
            .await
            .expect("no discovery request")
            .unwrap();
        assert_eq!(len, DISCOVERY);
        assert_eq!(buffer[..4], [0, 1, 0, 70]);
        assert_eq!(buffer[4..8], ssrc.to_be_bytes());
        let mut response = [0; DISCOVERY];
        response[..4].copy_from_slice(&[0, 2, 0, 70]);
        response[4..8].copy_from_slice(&ssrc.to_be_bytes());
        response[8..8 + SEEN_AT.len()].copy_from_slice(SEEN_AT.as_bytes());
        response[72..].copy_from_slice(&SEEN_PORT.to_be_bytes());
        self.udp.send_to(&response, from).await.unwrap();
    }

    /// The next UDP packet of at least `min` bytes.
    async fn packet(&self, min: usize) -> (Vec<u8>, SocketAddr) {
        let mut buffer = [0; 1500];
        loop {
            let (len, from) = timeout(STEP, self.udp.recv_from(&mut buffer))
                .await
                .expect("no packet")
                .unwrap();
            if len >= min {
                return (buffer[..len].to_vec(), from);
            }
        }
    }

    /// The next audio packet, decrypted with `key`: its header and its payload.
    async fn audio(&self, key: [u8; 32]) -> (Vec<u8>, Vec<u8>) {
        // Keepalives are 4 bytes; audio has a header and the overhead.
        let (mut packet, _) = self.packet(12 + 20).await;
        let range = Cipher::new(Mode::Aes256Gcm, &key)
            .open(&mut packet)
            .expect("the packet does not decrypt");
        (packet[..12].to_vec(), packet[range].to_vec())
    }
}

async fn send(server: &mut Server, payload: Value) {
    server
        .send(Message::text(payload.to_string()))
        .await
        .unwrap();
}

async fn close(server: &mut Server, code: u16) {
    let code = CloseCode::try_from(code).unwrap();
    server.send(Message::close(Some(code), "")).await.unwrap();
}

/// The next text message from the client, without answering anything.
async fn recv(server: &mut Server) -> Value {
    loop {
        let message = timeout(STEP, server.next())
            .await
            .expect("the client sent nothing")
            .expect("the connection ended")
            .unwrap();
        if let Some(text) = message.as_text() {
            return serde_json::from_str(text).unwrap();
        }
    }
}

/// The next message from the client with opcode `op`, acknowledging heartbeats on the way.
async fn recv_op(server: &mut Server, op: u64) -> Value {
    loop {
        let message = recv(server).await;
        if message["op"] == op {
            return message;
        }
        if message["op"] == 3 {
            let nonce = message["d"]["t"].clone();
            send(server, json!({"op": 6, "d": {"t": nonce}})).await;
        }
    }
}

/// A client configuration for the tests: quick retries and no keepalives.
fn config() -> Config {
    Config {
        timeout: STEP,
        retry_delays: vec![Duration::from_millis(5); 5],
        keepalive: Duration::from_secs(3600),
        discovery_retry: Duration::from_millis(200),
        ..Config::default()
    }
}

async fn event(events: &mut UnboundedReceiver<ConnectionEvent>) -> ConnectionEvent {
    timeout(STEP, events.recv())
        .await
        .expect("no event")
        .expect("the connection's task ended")
}

/// Connects a client to `mock` (SSRC 7, `KEY`), returning the server side too.
async fn connected(
    mock: &Mock,
    config: Config,
    interval: f64,
) -> (super::Handle, UnboundedReceiver<ConnectionEvent>, Server) {
    let (client, mut server) = tokio::join!(connect(mock.info(), config), async {
        let mut server = mock.accept().await;
        mock.handshake(&mut server, 7, KEY, interval).await;
        server
    });
    let (handle, mut events) = client.unwrap();
    // The message sent during the handshake comes first.
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Gateway(Event::Other(18))
    ));
    // Make sure the client is past the handshake before the test goes on.
    handle.send(Command::Speaking(MICROPHONE));
    assert_eq!(recv_op(&mut server, 5).await["d"]["ssrc"], 7);
    (handle, events, server)
}

#[tokio::test]
async fn connects_identifies_and_sends_audio() {
    let mock = Mock::new().await;
    let (client, (mut server, identify)) = tokio::join!(connect(mock.info(), config()), async {
        let mut server = mock.accept().await;
        let identify = mock.handshake(&mut server, 7, KEY, 41_250.0).await;
        (server, identify)
    });
    assert_eq!(
        identify,
        json!({"op": 0, "d": {
            "server_id": "1", "user_id": "2", "session_id": "session",
            "token": "voice-token", "max_dave_protocol_version": 1
        }})
    );
    let (handle, _events) = client.unwrap();
    assert_eq!(handle.dave_protocol_version(), 0);
    assert!(handle.send(Command::Speaking(MICROPHONE)));
    assert_eq!(
        recv_op(&mut server, 5).await["d"],
        json!({"speaking": 1, "delay": 0, "ssrc": 7})
    );
    let mut sender = handle.sender();
    let frames: [&[u8]; 3] = [b"frame one", b"frame two", b"three"];
    let mut headers = Vec::new();
    for frame in frames {
        sender.send(frame, 960).await.unwrap();
        let (header, payload) = mock.audio(KEY).await;
        assert_eq!(payload, frame);
        headers.push(header);
    }
    for header in &headers {
        assert_eq!(header[..2], [0x80, 0x78]);
        assert_eq!(header[8..12], 7_u32.to_be_bytes());
    }
    let sequence = |header: &[u8]| u16::from_be_bytes([header[2], header[3]]);
    let timestamp =
        |header: &[u8]| u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
    for pair in headers.windows(2) {
        assert_eq!(sequence(&pair[1]), sequence(&pair[0]).wrapping_add(1));
        assert_eq!(timestamp(&pair[1]), timestamp(&pair[0]).wrapping_add(960));
    }
}

#[tokio::test]
async fn heartbeats_acknowledge_the_last_sequence_number() {
    let mock = Mock::new().await;
    let (_handle, mut events, mut server) = connected(&mock, config(), 100.0).await;
    // The sequence number 1 of the handshake, then 4.
    let first = recv_op(&mut server, 3).await;
    assert_eq!(first["d"]["seq_ack"], 1);
    send(&mut server, json!({"op": 6, "d": {"t": first["d"]["t"]}})).await;
    send(
        &mut server,
        json!({"op": 11, "d": {"user_ids": ["5", "6"]}, "seq": 4}),
    )
    .await;
    let ConnectionEvent::Gateway(Event::ClientsConnect(users)) = event(&mut events).await else {
        panic!("not the clients");
    };
    assert_eq!(users.len(), 2);
    loop {
        let heartbeat = recv_op(&mut server, 3).await;
        send(
            &mut server,
            json!({"op": 6, "d": {"t": heartbeat["d"]["t"]}}),
        )
        .await;
        if heartbeat["d"]["seq_ack"] == 4 {
            break;
        }
    }
}

#[tokio::test]
async fn a_lost_connection_resumes() {
    let mock = Mock::new().await;
    let (handle, mut events, mut server) = connected(&mock, config(), 41_250.0).await;
    close(&mut server, 4015).await;
    let mut server = mock.accept().await;
    assert_eq!(
        recv(&mut server).await,
        json!({"op": 7, "d": {
            "server_id": "1", "session_id": "session", "token": "voice-token", "seq_ack": 1
        }})
    );
    send(
        &mut server,
        json!({"op": 8, "d": {"heartbeat_interval": 41_250.0}}),
    )
    .await;
    send(&mut server, json!({"op": 9, "d": null})).await;
    // A message the client missed, re-sent after the resume.
    send(
        &mut server,
        json!({"op": 13, "d": {"user_id": "5"}, "seq": 2}),
    )
    .await;
    assert!(matches!(event(&mut events).await, ConnectionEvent::Resumed));
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Gateway(Event::ClientDisconnect(_))
    ));
    // The same session: the same UDP socket, SSRC and key.
    handle.sender().send(b"after", 960).await.unwrap();
    assert_eq!(mock.audio(KEY).await.1, b"after");
}

#[tokio::test]
async fn a_dropped_connection_resumes() {
    let mock = Mock::new().await;
    let (_handle, mut events, server) = connected(&mock, config(), 41_250.0).await;
    drop(server);
    let mut server = mock.accept().await;
    assert_eq!(recv(&mut server).await["op"], 7);
    send(
        &mut server,
        json!({"op": 8, "d": {"heartbeat_interval": 41_250.0}}),
    )
    .await;
    send(&mut server, json!({"op": 9})).await;
    assert!(matches!(event(&mut events).await, ConnectionEvent::Resumed));
}

#[tokio::test]
async fn unacknowledged_heartbeats_reconnect() {
    let mock = Mock::new().await;
    // The mock never reads the first connection again, so its heartbeats go unanswered.
    let (_handle, mut events, _server) = connected(&mock, config(), 30.0).await;
    let mut server = mock.accept().await;
    assert_eq!(recv(&mut server).await["op"], 7);
    send(
        &mut server,
        json!({"op": 8, "d": {"heartbeat_interval": 41_250.0}}),
    )
    .await;
    send(&mut server, json!({"op": 9})).await;
    assert!(matches!(event(&mut events).await, ConnectionEvent::Resumed));
}

#[tokio::test]
async fn an_invalid_session_identifies_anew() {
    let mock = Mock::new().await;
    let (handle, mut events, mut server) = connected(&mock, config(), 41_250.0).await;
    close(&mut server, 4006).await;
    let mut server = mock.accept().await;
    // A new session: another SSRC and key; the speaking flags are sent again.
    mock.handshake(&mut server, 9, OTHER_KEY, 41_250.0).await;
    assert_eq!(
        recv_op(&mut server, 5).await["d"],
        json!({"speaking": 1, "delay": 0, "ssrc": 9})
    );
    // The new session's messages follow the event that announces it.
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Reconnected
    ));
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Gateway(Event::Other(18))
    ));
    handle.sender().send(b"new session", 960).await.unwrap();
    let (header, payload) = mock.audio(OTHER_KEY).await;
    assert_eq!(
        (&header[8..12], &payload[..]),
        (&9_u32.to_be_bytes()[..], &b"new session"[..])
    );
}

#[tokio::test]
async fn stop_codes_end_the_connection() {
    for code in [4014, 4022] {
        let mock = Mock::new().await;
        let (handle, mut events, mut server) = connected(&mock, config(), 41_250.0).await;
        close(&mut server, code).await;
        assert!(
            matches!(event(&mut events).await, ConnectionEvent::Ended(End::Stopped(found)) if found == code)
        );
        assert!(
            timeout(Duration::from_millis(200), mock.listener.accept())
                .await
                .is_err(),
            "no reconnection after {code}"
        );
        // The task ends right after its last event.
        timeout(STEP, async {
            while handle.send(Command::Speaking(0)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the task has not ended");
    }
}

#[tokio::test]
async fn resuming_gives_up_after_the_retries() {
    let mock = Mock::new().await;
    let (_handle, mut events, mut server) = connected(&mock, config(), 41_250.0).await;
    close(&mut server, 4015).await;
    // The immediate attempt and one per delay (D-331), each refused the same way.
    for _ in 0..6 {
        let mut server = mock.accept().await;
        assert_eq!(recv(&mut server).await["op"], 7);
        close(&mut server, 4015).await;
    }
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Ended(End::Lost(ConnectError::Closed(Some(4015))))
    ));
    assert!(
        timeout(Duration::from_millis(200), mock.listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_rejected_resume_identifies_anew() {
    let mock = Mock::new().await;
    let (_handle, mut events, mut server) = connected(&mock, config(), 41_250.0).await;
    close(&mut server, 4015).await;
    let mut server = mock.accept().await;
    assert_eq!(recv(&mut server).await["op"], 7);
    close(&mut server, 4006).await;
    let mut server = mock.accept().await;
    mock.handshake(&mut server, 11, OTHER_KEY, 41_250.0).await;
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Reconnected
    ));
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Gateway(Event::Other(18))
    ));
}

#[tokio::test]
async fn closing_ends_the_connection() {
    let mock = Mock::new().await;
    let (handle, mut events, mut server) = connected(&mock, config(), 41_250.0).await;
    assert!(handle.send(Command::Close));
    let message = timeout(STEP, server.next()).await.unwrap();
    assert!(matches!(&message, Some(Ok(message)) if message.is_close()) || message.is_none());
    assert!(matches!(
        event(&mut events).await,
        ConnectionEvent::Ended(End::Closed)
    ));
}

#[tokio::test]
async fn commands_reach_the_gateway() {
    let mock = Mock::new().await;
    let (handle, _events, mut server) = connected(&mock, config(), 41_250.0).await;
    handle.send(Command::Text(
        json!({"op": 23, "d": {"transition_id": 3}}).to_string(),
    ));
    assert_eq!(recv_op(&mut server, 23).await["d"]["transition_id"], 3);
    handle.send(Command::Binary(vec![26, 1, 2, 3]));
    let message = loop {
        let message = timeout(STEP, server.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if message.is_binary() {
            break message;
        }
    };
    assert_eq!(message.as_payload()[..], [26, 1, 2, 3]);
}

#[tokio::test]
async fn keepalives_carry_the_ssrc() {
    let mock = Mock::new().await;
    let config = Config {
        keepalive: Duration::from_millis(20),
        ..config()
    };
    let _connected = connected(&mock, config, 41_250.0).await;
    let (packet, _) = mock.packet(4).await;
    assert_eq!(packet, 7_u32.to_be_bytes());
}

#[tokio::test]
async fn handshake_failures_are_reported() {
    // No supported encryption mode.
    let mock = Mock::new().await;
    let (client, ()) = tokio::join!(connect(mock.info(), config()), async {
        let mut server = mock.accept().await;
        send(
            &mut server,
            json!({"op": 8, "d": {"heartbeat_interval": 41_250.0}}),
        )
        .await;
        recv_op(&mut server, 0).await;
        mock.ready(&mut server, 7, &["xsalsa20_poly1305_lite_rtpsize"])
            .await;
        // Keep the connection open until the client gives up.
        let _ = timeout(STEP, server.next()).await;
    });
    assert!(matches!(client, Err(ConnectError::NoMode)));

    // Closed during the handshake.
    let mock = Mock::new().await;
    let (client, ()) = tokio::join!(connect(mock.info(), config()), async {
        let mut server = mock.accept().await;
        recv_op(&mut server, 0).await;
        close(&mut server, 4004).await;
    });
    assert!(matches!(client, Err(ConnectError::Closed(Some(4004)))));

    // Silent server.
    let mock = Mock::new().await;
    let quick = Config {
        timeout: Duration::from_millis(200),
        ..config()
    };
    let (client, _server) = tokio::join!(connect(mock.info(), quick), mock.accept());
    assert!(matches!(client, Err(ConnectError::Timeout)));

    // Nobody listening.
    let mock = Mock::new().await;
    let info = mock.info();
    drop(mock);
    assert!(matches!(
        connect(info, config()).await,
        Err(ConnectError::WebSocket(_))
    ));

    // A Hello without a usable interval.
    let mock = Mock::new().await;
    let (client, ()) = tokio::join!(connect(mock.info(), config()), async {
        let mut server = mock.accept().await;
        send(
            &mut server,
            json!({"op": 8, "d": {"heartbeat_interval": 0}}),
        )
        .await;
        let _ = timeout(STEP, server.next()).await;
    });
    assert!(matches!(client, Err(ConnectError::Unexpected(_))));
}

#[test]
fn endpoints_become_urls() {
    assert_eq!(
        url("sweetwater-12345.discord.media:2048"),
        "wss://sweetwater-12345.discord.media:2048/?v=8"
    );
    assert_eq!(url("ws://127.0.0.1:9/"), "ws://127.0.0.1:9/?v=8");
    assert_eq!(url("wss://proxy.example"), "wss://proxy.example/?v=8");
}

#[test]
fn delays_are_jittered_down_to_half() {
    let delay = Duration::from_secs(4);
    for _ in 0..1000 {
        let jittered = jitter(delay);
        assert!((delay / 2..=delay).contains(&jittered), "{jittered:?}");
    }
    assert_eq!(jitter(Duration::ZERO), Duration::ZERO);
}

#[test]
fn the_token_stays_out_of_debug_output() {
    let info = Info {
        guild_id: djs_model::id::GuildId::new(1),
        user_id: djs_model::id::UserId::new(2),
        session_id: "session".into(),
        endpoint: "host:1".into(),
        token: "voice-token".into(),
    };
    assert!(!format!("{info:?}").contains("voice-token"));
}
