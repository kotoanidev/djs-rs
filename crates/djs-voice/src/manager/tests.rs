//! The manager and its calls against a mock main gateway (a [`VoiceGateway`] that hands the
//! commands to the test, and the events the test passes to `process`) and the mock voice
//! server of the connection's tests.

use core::time::Duration;
use std::{error::Error, time::Instant};

use djs_model::{
    gateway::{
        DispatchEvent,
        payload::{UpdateVoiceState, VoiceServerUpdate, VoiceStateUpdate},
    },
    id::{ChannelId, GuildId, UserId},
};
use futures_util::{FutureExt, StreamExt};
use serde_json::json;
use tokio::{sync::mpsc, time::timeout};

use super::VoiceManager;
use crate::{
    Call, CallEvent, CallEvents, DisconnectReason, Input, JoinError, TrackError, VoiceGateway,
    connection::{
        Config,
        tests::{self as voice, KEY, Mock, OTHER_KEY, STEP, Server, close, recv_op, send},
    },
    ogg::OggOpus,
};

/// The first shard's guild ID shifted: shard 1 of 2.
const GUILD: u64 = 1 << 22;
const SHARD: u32 = 1;
const BOT: u64 = voice::BOT;
const CHANNEL: u64 = voice::CHANNEL;
const OTHER_CHANNEL: u64 = 4;
const SINE: &[u8] = include_bytes!("../../tests/data/sine.opus");
/// The Opus packet of a frame of silence.
const SILENCE: [u8; 3] = [0xF8, 0xFF, 0xFE];

type Commands = mpsc::UnboundedReceiver<(u32, UpdateVoiceState)>;

/// Hands Opcode 4 to the test, or fails.
struct Gateway {
    commands: mpsc::UnboundedSender<(u32, UpdateVoiceState)>,
    fail: bool,
}

impl VoiceGateway for Gateway {
    fn send(
        &self,
        shard_id: u32,
        command: &UpdateVoiceState,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.fail {
            return Err("the shard is closed".into());
        }
        self.commands.send((shard_id, *command))?;
        Ok(())
    }
}

fn manager_with(timeout: Duration, fail: bool) -> (VoiceManager, Commands) {
    let (commands, rx) = mpsc::unbounded_channel();
    let config = Config {
        timeout,
        ..voice::config()
    };
    let gateway = Gateway { commands, fail };
    (
        VoiceManager::with_config(UserId::new(BOT), 2, gateway, config),
        rx,
    )
}

fn manager() -> (VoiceManager, Commands) {
    manager_with(STEP, false)
}

const fn guild() -> GuildId {
    GuildId::new(GUILD)
}

/// The bot's `VOICE_STATE_UPDATE` in the guild.
fn state(channel: Option<u64>) -> DispatchEvent {
    let state: VoiceStateUpdate = serde_json::from_value(json!({
        "guild_id": GUILD.to_string(),
        "channel_id": channel.map(|channel| channel.to_string()),
        "user_id": BOT.to_string(),
        "session_id": "session",
        "deaf": false, "mute": false, "self_deaf": true, "self_mute": false,
        "self_video": false, "suppress": false, "request_to_speak_timestamp": null
    }))
    .unwrap();
    DispatchEvent::VoiceStateUpdate(Box::new(state))
}

fn server(mock: &Mock) -> DispatchEvent {
    let server: VoiceServerUpdate = serde_json::from_value(json!({
        "token": "voice-token", "guild_id": GUILD.to_string(), "endpoint": mock.endpoint
    }))
    .unwrap();
    DispatchEvent::VoiceServerUpdate(Box::new(server))
}

async fn command(commands: &mut Commands) -> (u32, UpdateVoiceState) {
    timeout(STEP, commands.recv())
        .await
        .expect("no Opcode 4")
        .unwrap()
}

/// Discord's side of a join to `channel` on `mock`: the two updates, then the voice
/// server's handshake (SSRC `ssrc`, `key`). Returns the voice server's connection.
async fn answer(
    manager: &VoiceManager,
    mock: &Mock,
    channel: u64,
    ssrc: u32,
    key: [u8; 32],
) -> Server {
    manager.process(&state(Some(channel)));
    manager.process(&server(mock));
    let mut server = mock.accept().await;
    let identify = mock.handshake(&mut server, ssrc, key, 41_250.0).await;
    assert_eq!(
        identify["d"],
        json!({
            "server_id": GUILD.to_string(), "user_id": BOT.to_string(),
            "session_id": "session", "token": "voice-token", "max_dave_protocol_version": 1
        })
    );
    server
}

/// A call in `CHANNEL` (SSRC 7, `KEY`), with its events and the voice server's side.
async fn joined(
    manager: &VoiceManager,
    commands: &mut Commands,
    mock: &Mock,
) -> (Call, CallEvents, Server) {
    let join = {
        let manager = manager.clone();
        tokio::spawn(async move { manager.join(guild(), ChannelId::new(CHANNEL)).await })
    };
    assert_eq!(
        command(commands).await,
        (
            SHARD,
            UpdateVoiceState::new(guild(), Some(ChannelId::new(CHANNEL)), false, true)
        ),
        "deafened, not muted (D-341)"
    );
    let server = answer(manager, mock, CHANNEL, 7, KEY).await;
    let call = timeout(STEP, join).await.unwrap().unwrap().unwrap();
    let events = call.events();
    (call, events, server)
}

async fn event(events: &mut CallEvents) -> CallEvent {
    timeout(STEP, events.next())
        .await
        .expect("no event")
        .expect("the call ended")
}

fn sine_packets() -> Vec<Vec<u8>> {
    let mut ogg = OggOpus::new();
    ogg.push(SINE);
    ogg.finish();
    let mut packets = Vec::new();
    while let Some(packet) = ogg.packet().unwrap() {
        packets.push(packet.to_vec());
    }
    packets
}

/// The next audio payload the voice server gets, with its RTP sequence and timestamp.
async fn audio(mock: &Mock, key: [u8; 32]) -> (Vec<u8>, u16, u32) {
    let (header, payload) = mock.audio(key).await;
    let sequence = u16::from_be_bytes([header[2], header[3]]);
    let timestamp = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
    (payload, sequence, timestamp)
}

/// Reads audio until silence, then checks the five frames of silence and Speaking 0.
async fn silence_follows(mock: &Mock, server: &mut Server) {
    loop {
        if audio(mock, KEY).await.0 == SILENCE {
            break;
        }
    }
    for _ in 1..5 {
        assert_eq!(audio(mock, KEY).await.0, SILENCE);
    }
    assert_eq!(recv_op(server, 5).await["d"]["speaking"], 0);
}

/// Waits until the manager no longer has the guild's call.
async fn forgotten(manager: &VoiceManager) {
    timeout(STEP, async {
        while manager.get(guild()).is_some() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the call is still there");
}

/// Waits until the voice server's connection is closed by the client.
async fn closed(server: &mut Server) {
    loop {
        match timeout(STEP, server.next()).await.expect("not closed") {
            Some(Ok(message)) if message.is_close() => return,
            None | Some(Err(_)) => return,
            Some(Ok(_)) => {}
        }
    }
}

#[tokio::test]
async fn joins_plays_in_real_time_and_leaves() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, mut events, mut server) = joined(&manager, &mut commands, &mock).await;
    assert_eq!(call.guild_id(), guild());
    assert_eq!(call.channel_id(), Some(ChannelId::new(CHANNEL)));
    assert!(manager.get(guild()).is_some());
    let calls = manager.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].inner.id, call.inner.id);

    let expected = sine_packets();
    assert!(!expected[1..].contains(&expected[0]), "packets tell apart");
    let track = call.play(Input::ogg(SINE));
    assert_eq!(
        recv_op(&mut server, 5).await["d"],
        json!({"speaking": 1, "delay": 0, "ssrc": 7})
    );
    let (first, sequence, timestamp) = audio(&mock, KEY).await;
    assert_eq!(first, expected[0]);
    let start = Instant::now();
    for (n, packet) in expected.iter().enumerate().take(10).skip(1) {
        let (payload, seq, ts) = audio(&mock, KEY).await;
        assert_eq!(&payload, packet);
        let n = u32::try_from(n).unwrap();
        assert_eq!(seq, sequence.wrapping_add(u16::try_from(n).unwrap()));
        assert_eq!(ts, timestamp.wrapping_add(960 * n));
    }
    // Nine packets of 20 ms after the first.
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(170),
        "{elapsed:?}: too fast"
    );
    // Generous: a loaded machine may be late, never early.
    assert!(elapsed < Duration::from_secs(1), "{elapsed:?}: too slow");

    track.pause().unwrap();
    silence_follows(&mock, &mut server).await;
    track.play().unwrap();
    assert_eq!(recv_op(&mut server, 5).await["d"]["speaking"], 1);
    let (payload, _, _) = audio(&mock, KEY).await;
    assert!(
        expected[10..].contains(&payload),
        "it goes on where it paused"
    );
    track.stop().unwrap();
    silence_follows(&mock, &mut server).await;
    assert_eq!(track.play(), Err(TrackError::Ended));

    call.leave().await.unwrap();
    assert_eq!(
        command(&mut commands).await,
        (SHARD, UpdateVoiceState::new(guild(), None, false, false))
    );
    assert!(matches!(
        event(&mut events).await,
        CallEvent::Disconnected(DisconnectReason::Left)
    ));
    assert!(events.next().await.is_none());
    assert!(call.events().next().await.is_none(), "the call has ended");
    assert!(manager.get(guild()).is_none());
    assert!(manager.calls().is_empty());
    closed(&mut server).await;
}

#[tokio::test]
async fn a_track_plays_to_its_end_and_a_new_one_replaces_the_old() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, mut server) = joined(&manager, &mut commands, &mock).await;
    let first = call.play(Input::ogg(SINE));
    recv_op(&mut server, 5).await;
    audio(&mock, KEY).await;
    let second = call.play(Input::ogg(SINE));
    assert_eq!(first.pause(), Err(TrackError::Ended), "replaced (D-335)");
    let expected = sine_packets();
    // The first's packets in flight, then the second from its start, to its end.
    loop {
        if audio(&mock, KEY).await.0 == expected[0] {
            break;
        }
    }
    for packet in &expected[1..] {
        assert_eq!(&audio(&mock, KEY).await.0, packet);
    }
    for _ in 0..5 {
        assert_eq!(audio(&mock, KEY).await.0, SILENCE);
    }
    assert_eq!(recv_op(&mut server, 5).await["d"]["speaking"], 0);
    assert_eq!(second.stop(), Err(TrackError::Ended));
}

#[tokio::test]
async fn joining_times_out_without_an_answer() {
    let (manager, mut commands) = manager_with(Duration::from_millis(300), false);
    let join = manager.join(guild(), ChannelId::new(CHANNEL));
    let (result, _) = tokio::join!(join, command(&mut commands));
    assert!(matches!(result, Err(JoinError::Timeout)));
    assert!(
        manager.get(guild()).is_none(),
        "gone before the join returned"
    );
}

#[tokio::test]
async fn an_opcode_4_that_cannot_be_sent_fails_the_join() {
    let (manager, _commands) = manager_with(STEP, true);
    let error = manager
        .join(guild(), ChannelId::new(CHANNEL))
        .await
        .unwrap_err();
    assert!(matches!(error, JoinError::Gateway(_)));
    assert_eq!(error.source().unwrap().to_string(), "the shard is closed");
    forgotten(&manager).await;
}

#[tokio::test]
async fn joining_again_moves_the_same_call() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, mut events, mut old) = joined(&manager, &mut commands, &mock).await;
    let join = {
        let manager = manager.clone();
        tokio::spawn(async move { manager.join(guild(), ChannelId::new(OTHER_CHANNEL)).await })
    };
    assert_eq!(
        command(&mut commands).await.1.channel_id,
        Some(ChannelId::new(OTHER_CHANNEL))
    );
    let _new = answer(&manager, &mock, OTHER_CHANNEL, 9, OTHER_KEY).await;
    let moved = timeout(STEP, join).await.unwrap().unwrap().unwrap();
    assert_eq!(moved.inner.id, call.inner.id, "the same call (D-340)");
    assert_eq!(call.channel_id(), Some(ChannelId::new(OTHER_CHANNEL)));
    closed(&mut old).await;
    // Audio goes to the new connection.
    call.play(Input::ogg(SINE));
    let (payload, _, _) = audio(&mock, OTHER_KEY).await;
    assert_eq!(payload, sine_packets()[0]);
    assert!(
        events.next().now_or_never().is_none(),
        "a join's move is not an event"
    );
}

#[tokio::test]
async fn a_moderator_move_connects_again() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, mut events, mut old) = joined(&manager, &mut commands, &mock).await;
    close(&mut old, 4014).await;
    let _new = answer(&manager, &mock, OTHER_CHANNEL, 9, OTHER_KEY).await;
    assert!(matches!(event(&mut events).await, CallEvent::Reconnected));
    assert_eq!(call.channel_id(), Some(ChannelId::new(OTHER_CHANNEL)));
    assert!(manager.get(guild()).is_some());
}

#[tokio::test]
async fn being_disconnected_ends_the_call() {
    for state_first in [true, false] {
        let mock = Mock::new().await;
        let (manager, mut commands) = manager();
        let (call, mut events, mut server) = joined(&manager, &mut commands, &mock).await;
        if state_first {
            manager.process(&state(None));
            close(&mut server, 4014).await;
        } else {
            close(&mut server, 4014).await;
            // Let the close arrive first: the call waits for a move then.
            tokio::time::sleep(Duration::from_millis(50)).await;
            manager.process(&state(None));
        }
        match event(&mut events).await {
            CallEvent::Disconnected(DisconnectReason::Closed { code, .. }) => {
                assert_eq!(code, 4014);
            }
            other => panic!("{other:?}"),
        }
        assert!(events.next().await.is_none());
        assert_eq!(call.channel_id(), None);
        assert!(manager.get(guild()).is_none());
    }
}

#[tokio::test]
async fn a_stop_code_ends_the_call() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, mut events, mut server) = joined(&manager, &mut commands, &mock).await;
    let track = call.play(Input::ogg(SINE));
    close(&mut server, 4022).await;
    let reason = match event(&mut events).await {
        CallEvent::Disconnected(reason) => reason,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        reason.to_string(),
        "the voice gateway closed the call with code 4022"
    );
    assert_eq!(
        track.pause(),
        Err(TrackError::Ended),
        "the call's tracks end"
    );
    forgotten(&manager).await;
}

#[tokio::test]
async fn users_and_speaking_become_events() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (_call, mut events, mut server) = joined(&manager, &mut commands, &mock).await;
    send(
        &mut server,
        json!({"op": 11, "d": {"user_ids": ["5", "6"]}}),
    )
    .await;
    send(&mut server, json!({"op": 13, "d": {"user_id": "5"}})).await;
    send(
        &mut server,
        json!({"op": 5, "d": {"user_id": "6", "ssrc": 77, "speaking": 1}}),
    )
    .await;
    let user = UserId::new;
    assert!(matches!(event(&mut events).await, CallEvent::UserJoined(id) if id == user(5)));
    assert!(matches!(event(&mut events).await, CallEvent::UserJoined(id) if id == user(6)));
    assert!(matches!(event(&mut events).await, CallEvent::UserLeft(id) if id == user(5)));
    match event(&mut events).await {
        CallEvent::Speaking { user_id, ssrc, .. } => assert_eq!((user_id, ssrc), (user(6), 77)),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn other_users_and_guilds_are_ignored() {
    let (manager, _commands) = manager();
    // No call: nothing happens.
    manager.process(&state(Some(CHANNEL)));
    assert!(manager.get(guild()).is_none());
    let other: VoiceStateUpdate = serde_json::from_value(json!({
        "guild_id": GUILD.to_string(), "channel_id": "3", "user_id": "99",
        "session_id": "s", "request_to_speak_timestamp": null
    }))
    .unwrap();
    manager.process(&DispatchEvent::VoiceStateUpdate(Box::new(other)));
    assert!(format!("{manager:?}").contains("calls: 0"));
}
