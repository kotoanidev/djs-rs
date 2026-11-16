//! The `voice` feature (D-345–D-348) end to end: a real `Client` against a mock gateway on
//! `127.0.0.1`, which answers Opcode 4 with the voice events, and a mock voice server (a
//! WebSocket and a UDP socket, without DAVE).

#![cfg(feature = "voice")]

use std::time::Duration;

use djs_rs::{
    Shards,
    gateway::Compression,
    model::id::{ChannelId, GuildId},
    prelude::*,
    voice::{CallEvent, DisconnectReason, JoinError},
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    net::{TcpListener, TcpStream, UdpSocket},
    sync::mpsc,
    time::timeout,
};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

const STEP: Duration = Duration::from_secs(5);
/// A token whose first part is the bot's ID, 1, in base64 (D-345); the rest is not checked.
const TOKEN: &str = "MQ.mock.token";
const GUILD: u64 = 2;
const CHANNEL: u64 = 3;

type Server = WebSocketStream<TcpStream>;

async fn send(ws: &mut Server, payload: Value) {
    ws.send(Message::text(payload.to_string())).await.unwrap();
}

/// The next JSON message with opcode `op`.
async fn recv_op(ws: &mut Server, op: u64) -> Value {
    loop {
        let message = timeout(STEP, ws.next())
            .await
            .expect("nothing received")
            .expect("the connection ended")
            .unwrap();
        if let Some(text) = message.as_text() {
            let payload: Value = serde_json::from_str(text).unwrap();
            if payload["op"] == op {
                return payload;
            }
        }
    }
}

/// Waits until the client closes the connection.
async fn closed(ws: &mut Server) {
    loop {
        match timeout(STEP, ws.next()).await.expect("never closed") {
            Some(Ok(message)) if message.is_close() => return,
            Some(Ok(_)) => {}
            Some(Err(_)) | None => return,
        }
    }
}

/// The main gateway: Hello, READY, then the voice events for the Opcode 4 it gets, pointing
/// at `voice_endpoint`. Returns that Opcode 4.
async fn mock_gateway(
    listener: TcpListener,
    voice_endpoint: String,
    joined: mpsc::UnboundedSender<Value>,
) {
    let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    send(
        &mut ws,
        json!({"op": 10, "d": {"heartbeat_interval": 45_000}}),
    )
    .await;
    recv_op(&mut ws, 2).await;
    send(
        &mut ws,
        json!({"t": "READY", "s": 1, "op": 0, "d": {
            "v": 10,
            "user": {"id": "1", "username": "test-bot", "discriminator": "0", "global_name": null,
                     "avatar": null, "bot": true},
            "guilds": [], "session_id": "s", "resume_gateway_url": "ws://127.0.0.1:9",
            "application": {"id": "1"}
        }}),
    )
    .await;
    let opcode_4 = recv_op(&mut ws, 4).await;
    joined.send(opcode_4.clone()).unwrap();
    send(
        &mut ws,
        json!({"t": "VOICE_STATE_UPDATE", "s": 2, "op": 0, "d": {
            "guild_id": GUILD.to_string(), "channel_id": CHANNEL.to_string(), "user_id": "1",
            "session_id": "voice-session", "deaf": false, "mute": false, "self_deaf": true,
            "self_mute": false, "self_video": false, "suppress": false,
            "request_to_speak_timestamp": null
        }}),
    )
    .await;
    send(
        &mut ws,
        json!({"t": "VOICE_SERVER_UPDATE", "s": 3, "op": 0, "d": {
            "token": "voice-token", "guild_id": GUILD.to_string(), "endpoint": voice_endpoint
        }}),
    )
    .await;
    closed(&mut ws).await;
}

/// The voice server's handshake, without DAVE; returns the client's Identify and the
/// connection, kept open until the client closes it.
async fn mock_voice(listener: TcpListener, udp: UdpSocket) -> (Value, Server) {
    let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
    let (_, mut ws) = ServerBuilder::new().accept(stream).await.unwrap();
    send(
        &mut ws,
        json!({"op": 8, "d": {"heartbeat_interval": 41_250.0}}),
    )
    .await;
    let identify = recv_op(&mut ws, 0).await;
    let port = udp.local_addr().unwrap().port();
    send(
        &mut ws,
        json!({"op": 2, "d": {
            "ssrc": 7, "ip": "127.0.0.1", "port": port, "modes": ["aead_aes256_gcm_rtpsize"]
        }}),
    )
    .await;
    // IP discovery: the address the client is seen at.
    let mut buffer = [0; 1500];
    let (_, from) = timeout(STEP, udp.recv_from(&mut buffer))
        .await
        .unwrap()
        .unwrap();
    let mut response = [0; 74];
    response[..4].copy_from_slice(&[0, 2, 0, 70]);
    response[4..8].copy_from_slice(&7_u32.to_be_bytes());
    response[8..17].copy_from_slice(b"127.0.0.1");
    response[72..].copy_from_slice(&from.port().to_be_bytes());
    udp.send_to(&response, from).await.unwrap();
    recv_op(&mut ws, 1).await;
    let key = [7_u8; 32];
    send(
        &mut ws,
        json!({"op": 4, "d": {
            "mode": "aead_aes256_gcm_rtpsize", "secret_key": key, "dave_protocol_version": 0
        }}),
    )
    .await;
    (identify, ws)
}

#[tokio::test]
async fn the_client_joins_through_its_shard_and_leaves_on_shutdown() {
    let gateway = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_url = format!("ws://{}", gateway.local_addr().unwrap());
    let voice = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let voice_endpoint = format!("ws://{}", voice.local_addr().unwrap());
    let udp = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let (joined_tx, mut joined) = mpsc::unbounded_channel();
    let gateway = tokio::spawn(mock_gateway(gateway, voice_endpoint, joined_tx));
    let voice_server = tokio::spawn(mock_voice(voice, udp));

    let client = Client::builder(TOKEN)
        .intents(Intents::GUILDS | Intents::GUILD_VOICE_STATES)
        .gateway_url(gateway_url)
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .build();
    let (ready_tx, mut ready) = mpsc::unbounded_channel();
    client.on(Events::ClientReady, move |_ctx, _ready| {
        let ready_tx = ready_tx.clone();
        async move {
            ready_tx.send(())?;
            Ok(())
        }
    });
    let running = {
        let client = client.clone();
        tokio::spawn(async move { client.login().await })
    };
    timeout(STEP, ready.recv()).await.unwrap().unwrap();

    let (guild, channel) = (GuildId::new(GUILD), ChannelId::new(CHANNEL));
    let call = timeout(STEP, client.voice().join(guild, channel))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        joined.recv().await.unwrap()["d"],
        json!({"guild_id": "2", "channel_id": "3", "self_mute": false, "self_deaf": true})
    );
    assert_eq!(call.channel_id(), Some(channel));
    let (identify, mut voice_ws) = voice_server.await.unwrap();
    assert_eq!(
        identify["d"],
        json!({
            "server_id": "2", "user_id": "1", "session_id": "voice-session",
            "token": "voice-token", "max_dave_protocol_version": 1
        })
    );
    let mut events = call.events();

    client.shutdown();
    timeout(STEP, running).await.unwrap().unwrap().unwrap();
    assert!(matches!(
        timeout(STEP, events.next()).await.unwrap(),
        Some(CallEvent::Disconnected(DisconnectReason::Left))
    ));
    assert!(client.voice().calls().is_empty());
    closed(&mut voice_ws).await;
    timeout(STEP, gateway).await.unwrap().unwrap();
}

#[tokio::test]
async fn joining_before_login_fails() {
    let client = Client::builder(TOKEN).build();
    let error = client
        .voice()
        .join(GuildId::new(GUILD), ChannelId::new(CHANNEL))
        .await
        .unwrap_err();
    assert!(matches!(error, JoinError::Gateway(_)));
    assert_eq!(
        std::error::Error::source(&error).unwrap().to_string(),
        "the client is not logged in"
    );
}
