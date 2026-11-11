//! Soundboard sounds and lobbies against the docs' examples.

use djs_model::{
    lobby::{Lobby, LobbyMember, LobbyMemberFlags, LobbyMessage},
    message::MessageType,
    soundboard::SoundboardSound,
};

mod common;

use common::round_trip;

#[test]
fn default_sound() {
    let sound: SoundboardSound = round_trip(
        include_str!("fixtures/docs/soundboard_default_sound.json"),
        &[],
    );
    assert_eq!(sound.emoji_name.as_deref(), Some("🦆"));
    assert!(sound.guild_id.is_none() && sound.emoji_id.is_none() && sound.available);

    // `volume` is written as a float (`1.0`), which `serde_json::Value` does not equate with the
    // example's `1`; it is checked separately.
    let sound: SoundboardSound = round_trip(
        include_str!("fixtures/docs/soundboard_guild_sound.json"),
        &["volume"],
    );
    assert!((sound.volume - 1.0).abs() < f64::EPSILON);
    assert!(sound.guild_id.is_some() && sound.emoji_id.is_some());
    // Nullable emoji fields are written as `null` (D-049).
    let written = serde_json::to_value(&sound).unwrap();
    assert!(written["emoji_name"].is_null());
}

#[test]
fn lobby_example() {
    let lobby: Lobby = round_trip(include_str!("fixtures/docs/lobby_lobby.json"), &[]);
    assert_eq!(lobby.metadata["topic"], "we need more redstone");
    let [member] = lobby.members.as_slice() else {
        panic!("expected one member");
    };
    assert_eq!(member.flags, LobbyMemberFlags::CAN_LINK_LOBBY);
    assert!(member.metadata.is_empty());
    assert_eq!(
        member.additional_name.as_deref(),
        Some("Twindly Shimmerleaf")
    );
    assert!(lobby.linked_channel.is_none());

    let member: LobbyMember = serde_json::from_str(r#"{"id":"1"}"#).unwrap();
    assert_eq!(serde_json::to_string(&member).unwrap(), r#"{"id":"1"}"#);
}

#[test]
fn lobby_message() {
    let json = r#"{"id":"10","type":0,"content":"gg","lobby_id":"20","channel_id":"20",
        "author":{"id":"30","username":"player","discriminator":"0","global_name":null,"avatar":null},
        "lobby_member":{"additional_name":"Shimmerleaf"},"metadata":null,
        "moderation_metadata":{"verdict":"ok"},"flags":0,"application_id":"40"}"#;
    let message: LobbyMessage = round_trip(json, &[]);
    assert_eq!(message.kind, MessageType::Default);
    assert_eq!(message.lobby_id.get(), message.channel_id.get());
    assert_eq!(message.moderation_metadata["verdict"], "ok");
    assert_eq!(
        message.lobby_member.map(|member| member.additional_name),
        Some("Shimmerleaf".into())
    );
}
