//! Receiving audio of step 3 end to end (D-363–D-373, C-128): subscriptions, the order of
//! packets, lost ones, decoding and the speaking events, through a joined call and the mock
//! voice server.

use djs_model::id::UserId;
use serde_json::json;
use tokio::time::timeout;

use super::tests::{event, joined, manager, sine_packets};
use crate::{
    CallEvent, ReceiveOptions, VoiceData, VoicePacket, VoiceStream,
    connection::tests::{KEY, Mock, STEP, send},
};

/// The user who speaks, and their audio's source.
const SPEAKER: u64 = 6;
const SSRC: u32 = 77;

async fn next(stream: &mut VoiceStream) -> VoicePacket {
    timeout(STEP, stream.next())
        .await
        .expect("no packet")
        .expect("the stream ended")
}

#[tokio::test]
async fn others_audio_comes_in_order_and_decoded_with_speaking_events() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, mut events, mut server) = joined(&manager, &mut commands, &mock).await;
    let speaker = UserId::new(SPEAKER);
    let mut opus = call.subscribe(speaker);
    let mut pcm = call.subscribe_with(speaker, ReceiveOptions::new().decode(true));
    // Someone else's packets are not theirs.
    let mut nobody = call.subscribe(UserId::new(SPEAKER + 1));
    send(
        &mut server,
        json!({"op": 5, "d": {"user_id": SPEAKER.to_string(), "ssrc": SSRC, "speaking": 1}}),
    )
    .await;
    assert!(matches!(
        event(&mut events).await,
        CallEvent::Speaking { user_id, ssrc, .. } if user_id == speaker && ssrc == SSRC
    ));
    // The 3rd and 4th packets swapped, the 5th lost on the way.
    let packets = sine_packets();
    for index in [0_u16, 1, 3, 2, 5] {
        let at = usize::from(index);
        let timestamp = u32::from(index) * 960;
        mock.send_audio(KEY, SSRC, 100 + index, timestamp, &packets[at])
            .await;
    }
    assert!(matches!(
        event(&mut events).await,
        CallEvent::SpeakingStarted(user_id) if user_id == speaker
    ));
    for index in 0..6_u16 {
        let packet = next(&mut opus).await;
        assert_eq!(packet.sequence, 100 + index);
        assert_eq!(packet.timestamp, u32::from(index) * 960);
        let expected = if index == 4 {
            VoiceData::Lost
        } else {
            VoiceData::Opus(packets[usize::from(index)].clone())
        };
        assert_eq!(packet.audio, expected, "packet {index}");
    }
    for _ in 0..6 {
        let packet = next(&mut pcm).await;
        assert!(matches!(packet.audio, VoiceData::Pcm(samples) if samples.len() == 1920));
    }
    // 100 ms after the last packet.
    assert!(matches!(
        event(&mut events).await,
        CallEvent::SpeakingStopped(user_id) if user_id == speaker
    ));
    assert!(nobody.rx.try_recv().is_err());
    // The streams end with the call.
    call.leave().await.unwrap();
    assert_eq!(timeout(STEP, opus.next()).await.unwrap(), None);
    assert_eq!(timeout(STEP, pcm.next()).await.unwrap(), None);
    // Subscribing to an ended call: a stream that has ended.
    assert_eq!(
        timeout(STEP, call.subscribe(speaker).next()).await.unwrap(),
        None
    );
}
