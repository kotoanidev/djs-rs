//! The lobby endpoints against the mock API: method, path, query and body of every request.

use djs_model::{
    id::{ChannelId, LobbyId, MessageId, UserId},
    lobby::LobbyMemberFlags,
    rest::{
        AddLobbyMember, BulkLobbyMember, CreateLobby, CreateOrJoinLobby, GetLobbyMessages,
        LobbyMemberData, LobbyMetadata, ModifyLobby, SendLobbyMessage,
    },
    util::Patch,
};

use std::sync::Mutex;

mod common;

use common::{Canned, State, mock, seen};

const LOBBY: &str = include_str!("../../djs-model/tests/fixtures/docs/lobby_lobby.json");
const MEMBER: &str = r#"{"id": "7", "flags": 1, "additional_name": "Ferris"}"#;
const MESSAGE: &str = r#"{"id": "10", "type": 0, "content": "gg", "lobby_id": "96008815106887111",
    "channel_id": "96008815106887111", "author": {"id": "7", "username": "ferris",
    "discriminator": "0", "global_name": null, "avatar": null}, "flags": 0,
    "application_id": "41771983429993937"}"#;
const INVITE: &str = r#"{"code": "abc123"}"#;

const ID: LobbyId = LobbyId::new(96_008_815_106_887_111);

/// `(method, path after /api/v10, query, body)`.
type Expected = (&'static str, String, &'static str, &'static str);

/// Checks the requests the mock received against `expected`.
fn check(state: &Mutex<State>, expected: &[Expected]) {
    let seen = seen(state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
    }
}

#[tokio::test]
async fn lobby_and_member_endpoints() {
    let lobby = || Canned::json(200, LOBBY);
    let no_content = || Canned::json(204, "");
    let (rest, state) = mock(vec![
        lobby(),
        lobby(),
        lobby(),
        lobby(),
        no_content(),
        Canned::json(200, MEMBER),
        Canned::json(200, &format!("[{MEMBER}]")),
        no_content(),
        no_content(),
    ])
    .await;
    let ferris = UserId::new(7);

    let mut create = CreateLobby::default();
    create.members = vec![LobbyMemberData::new(ferris)];
    let created = rest.create_lobby(&create).await.unwrap();
    assert_eq!(created.id, ID);
    let mut join = CreateOrJoinLobby::new("lobby-secret");
    join.idle_timeout_seconds = Some(60);
    rest.create_or_join_lobby(&join).await.unwrap();
    rest.get_lobby(ID).await.unwrap();
    let mut modify = ModifyLobby::default();
    modify.metadata = Some(LobbyMetadata::new());
    rest.modify_lobby(ID, &modify).await.unwrap();
    rest.delete_lobby(ID).await.unwrap();

    let mut add = AddLobbyMember::default();
    add.flags = Some(LobbyMemberFlags::CAN_LINK_LOBBY);
    add.additional_name = Patch::Set("Ferris".into());
    let member = rest.add_lobby_member(ID, ferris, &add).await.unwrap();
    assert_eq!(member.additional_name.as_deref(), Some("Ferris"));
    let upserted = rest
        .bulk_update_lobby_members(
            ID,
            &[
                BulkLobbyMember::upsert(LobbyMemberData::new(ferris)),
                BulkLobbyMember::remove(UserId::new(8)),
            ],
        )
        .await
        .unwrap();
    assert_eq!(upserted.len(), 1);
    rest.remove_lobby_member(ID, UserId::new(8)).await.unwrap();
    rest.leave_lobby(ID).await.unwrap();

    let one = format!("/lobbies/{ID}");
    let members = format!("{one}/members");
    check(
        &state,
        &[
            ("POST", "/lobbies".into(), "", r#"{"members":[{"id":"7"}]}"#),
            (
                "PUT",
                "/lobbies".into(),
                "",
                r#"{"secret":"lobby-secret","idle_timeout_seconds":60}"#,
            ),
            ("GET", one.clone(), "", ""),
            ("PATCH", one.clone(), "", r#"{"metadata":{}}"#),
            ("DELETE", one, "", ""),
            (
                "PUT",
                format!("{members}/7"),
                "",
                r#"{"flags":1,"additional_name":"Ferris"}"#,
            ),
            (
                "POST",
                format!("{members}/bulk"),
                "",
                r#"[{"id":"7"},{"id":"8","remove_member":true}]"#,
            ),
            ("DELETE", format!("{members}/8"), "", ""),
            ("DELETE", format!("{members}/@me"), "", ""),
        ],
    );
}

#[tokio::test]
async fn channel_and_message_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, LOBBY),
        Canned::json(200, LOBBY),
        Canned::json(200, INVITE),
        Canned::json(200, INVITE),
        Canned::json(200, MESSAGE),
        Canned::json(200, &format!("[{MESSAGE}]")),
        Canned::json(204, ""),
    ])
    .await;

    rest.link_channel_to_lobby(ID, ChannelId::new(5))
        .await
        .unwrap();
    rest.unlink_channel_from_lobby(ID).await.unwrap();
    let invite = rest.create_lobby_channel_invite_for_self(ID).await.unwrap();
    assert_eq!(invite.code, "abc123");
    rest.create_lobby_channel_invite_for_user(ID, UserId::new(7))
        .await
        .unwrap();

    let sent = rest
        .send_lobby_message(ID, &SendLobbyMessage::new("gg"))
        .await
        .unwrap();
    assert_eq!(sent.content, "gg");
    let mut query = GetLobbyMessages::default();
    query.limit = Some(200);
    rest.get_lobby_messages(ID, &query).await.unwrap();
    let mut decision = LobbyMetadata::new();
    decision.insert("verdict".into(), "ok".into());
    rest.update_lobby_message_moderation_metadata(ID, MessageId::new(10), &decision)
        .await
        .unwrap();

    let one = format!("/lobbies/{ID}");
    check(
        &state,
        &[
            (
                "PATCH",
                format!("{one}/channel-linking"),
                "",
                r#"{"channel_id":"5"}"#,
            ),
            ("PATCH", format!("{one}/channel-linking"), "", "{}"),
            ("POST", format!("{one}/members/@me/invites"), "", ""),
            ("POST", format!("{one}/members/7/invites"), "", ""),
            ("POST", format!("{one}/messages"), "", r#"{"content":"gg"}"#),
            ("GET", format!("{one}/messages"), "limit=200", ""),
            (
                "PUT",
                format!("{one}/messages/10/moderation-metadata"),
                "",
                r#"{"verdict":"ok"}"#,
            ),
        ],
    );
}
