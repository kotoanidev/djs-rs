//! Voice states and regions against the docs' example.

use djs_model::voice::{VoiceRegion, VoiceState};

mod common;

use common::round_trip;

#[test]
fn voice_state_example() {
    let state: VoiceState = round_trip(include_str!("fixtures/docs/voice_voice_state.json"), &[]);
    assert!(state.self_mute && !state.self_deaf && !state.self_video && !state.self_stream);
    assert!(state.guild_id.is_none() && state.member.is_none());
    assert!(state.channel_id.is_some());
    assert!(state.request_to_speak_timestamp.is_some());
    // Leaving a channel: `channel_id` is written as `null` (D-049).
    let json = r#"{"guild_id":"1","channel_id":null,"user_id":"2","session_id":"s","deaf":false,"mute":false,"self_deaf":false,"self_mute":false,"self_video":false,"suppress":false,"request_to_speak_timestamp":null}"#;
    let left: VoiceState = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_string(&left).unwrap(), json);
}

#[test]
fn voice_region() {
    let json =
        r#"{"id":"rotterdam","name":"Rotterdam","optimal":true,"deprecated":false,"custom":false}"#;
    let region: VoiceRegion = round_trip(json, &[]);
    assert!(region.optimal);
    assert_eq!(serde_json::to_string(&region).unwrap(), json);
}
