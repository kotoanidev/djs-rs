//! The voice endpoints against the mock API: method, path and body of every request.

use djs_model::{
    id::{ChannelId, GuildId, UserId},
    rest::{ModifyCurrentUserVoiceState, ModifyUserVoiceState},
    util::Patch,
};

mod common;

use common::{Canned, mock, seen};

const STATE: &str = include_str!("../../djs-model/tests/fixtures/docs/voice_voice_state.json");
const REGIONS: &str = r#"[{"id": "rotterdam", "name": "Rotterdam", "optimal": true, "deprecated": false, "custom": false}]"#;

const GUILD: GuildId = GuildId::new(41_771_983_423_143_937);

#[tokio::test]
async fn voice_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, REGIONS),
        Canned::json(200, STATE),
        Canned::json(200, STATE),
        Canned::json(204, ""),
        Canned::json(204, ""),
    ])
    .await;
    let stage_channel = ChannelId::new(5);

    let regions = rest.list_voice_regions().await.unwrap();
    assert!(regions[0].optimal);
    let own = rest.get_current_user_voice_state(GUILD).await.unwrap();
    assert!(own.self_mute);
    rest.get_user_voice_state(GUILD, UserId::new(7))
        .await
        .unwrap();

    let mut current = ModifyCurrentUserVoiceState::default();
    current.channel_id = Some(stage_channel);
    current.request_to_speak_timestamp = Patch::Clear;
    rest.modify_current_user_voice_state(GUILD, &current)
        .await
        .unwrap();
    let mut other = ModifyUserVoiceState::default();
    other.channel_id = Some(stage_channel);
    other.suppress = Some(true);
    rest.modify_user_voice_state(GUILD, UserId::new(7), &other)
        .await
        .unwrap();

    let states = format!("/api/v10/guilds/{GUILD}/voice-states");
    let expected = [
        ("GET", "/api/v10/voice/regions".to_owned(), ""),
        ("GET", format!("{states}/@me"), ""),
        ("GET", format!("{states}/7"), ""),
        (
            "PATCH",
            format!("{states}/@me"),
            r#"{"channel_id":"5","request_to_speak_timestamp":null}"#,
        ),
        (
            "PATCH",
            format!("{states}/7"),
            r#"{"channel_id":"5","suppress":true}"#,
        ),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, body)) in seen.iter().zip(&expected) {
        assert_eq!(request.method, *method, "{path}");
        assert_eq!(request.path, *path);
        assert_eq!(request.body, *body, "{path}");
    }
}
