//! The user endpoints against the mock API: method, path, query and body of every request.

use djs_model::{
    id::{ApplicationId, GuildId, UserId},
    rest::{CreateDm, CreateGroupDm, GetCurrentUserGuilds, ModifyCurrentUser},
    user::ApplicationRoleConnection,
    util::Patch,
};

mod common;

use common::{Canned, mock, mock_with, seen};

const USER: &str = r#"{"id": "7", "username": "ferris", "discriminator": "0", "global_name": null, "avatar": null}"#;

const DM: &str = r#"{"id": "1200000000000000000", "type": 1, "last_message_id": null,
    "recipients": [{"id": "7", "username": "ferris", "discriminator": "0", "global_name": null,
                    "avatar": null}]}"#;

const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);
const APP: ApplicationId = ApplicationId::new(1_100_000_000_000_000_001);

/// `(method, path after /api/v10, query, body)`.
type Expected = (&'static str, String, &'static str, &'static str);

#[tokio::test]
async fn user_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, USER),
        Canned::json(200, USER),
        Canned::json(200, USER),
        Canned::json(
            200,
            r#"[{"id": "1100000000000000000", "name": "Rust", "icon": null, "banner": null,
                 "owner": false, "permissions": "1024", "features": []}]"#,
        ),
        Canned::json(204, ""),
        Canned::json(200, DM),
        Canned::json(200, DM),
    ])
    .await;

    assert_eq!(rest.get_current_user().await.unwrap().username, "ferris");
    rest.get_user(UserId::new(7)).await.unwrap();
    let mut modify = ModifyCurrentUser::default();
    modify.username = Some("crab".into());
    modify.banner = Patch::Clear;
    rest.modify_current_user(&modify).await.unwrap();
    let mut page = GetCurrentUserGuilds::default();
    page.after = Some(GuildId::new(5));
    page.shard = Some(0);
    page.with_counts = Some(true);
    let guilds = rest.get_current_user_guilds(&page).await.unwrap();
    assert_eq!(guilds[0].id, GUILD);
    rest.leave_guild(GUILD).await.unwrap();
    let dm = rest
        .create_dm(&CreateDm::new(UserId::new(7)))
        .await
        .unwrap();
    assert_eq!(dm.id().get(), 1_200_000_000_000_000_000);
    let mut group = CreateGroupDm::new(vec!["user-access-token".into()]);
    group.nicks.insert(UserId::new(7), "Ferris".into());
    rest.create_group_dm(&group).await.unwrap();

    let expected: Vec<Expected> = vec![
        ("GET", "/users/@me".into(), "", ""),
        ("GET", "/users/7".into(), "", ""),
        (
            "PATCH",
            "/users/@me".into(),
            "",
            r#"{"username":"crab","banner":null}"#,
        ),
        (
            "GET",
            "/users/@me/guilds".into(),
            "after=5&shard=0&with_counts=true",
            "",
        ),
        ("DELETE", format!("/users/@me/guilds/{GUILD}"), "", ""),
        (
            "POST",
            "/users/@me/channels".into(),
            "",
            r#"{"recipient_id":"7"}"#,
        ),
        (
            "POST",
            "/users/@me/channels".into(),
            "",
            r#"{"access_tokens":["user-access-token"],"nicks":{"7":"Ferris"}}"#,
        ),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn oauth2_user_endpoints_use_the_bearer_token() {
    let connection = r#"{"id": "1", "name": "ferris", "type": "github", "verified": true,
        "friend_sync": false, "show_activity": true, "two_way_link": false, "visibility": 1}"#;
    let role_connection = r#"{"platform_name": "Chess Club", "platform_username": null,
        "metadata": {"rating": "1850"}}"#;
    let (rest, state) = mock_with(
        vec![
            Canned::json(
                200,
                r#"{"user": {"id": "7", "username": "ferris", "discriminator": "0",
                    "global_name": null, "avatar": null}, "nick": null, "roles": [],
                    "joined_at": "2026-11-11T12:00:00+00:00", "deaf": false, "mute": false,
                    "flags": 0}"#,
            ),
            Canned::json(200, &format!("[{connection}]")),
            Canned::json(200, role_connection),
            Canned::json(200, role_connection),
            Canned::json(204, ""),
        ],
        |builder| builder.bearer_token("user-access-token"),
    )
    .await;

    rest.get_current_user_guild_member(GUILD).await.unwrap();
    let connections = rest.get_current_user_connections().await.unwrap();
    assert!(connections[0].verified);
    let fetched = rest
        .get_current_user_application_role_connection(APP)
        .await
        .unwrap();
    assert_eq!(fetched.metadata["rating"], "1850");
    let mut update = ApplicationRoleConnection::default();
    update.platform_name = Some("Chess Club".into());
    update.metadata.insert("rating".into(), "1900".into());
    rest.update_current_user_application_role_connection(APP, &update)
        .await
        .unwrap();
    rest.delete_current_user_application_role_connection(APP)
        .await
        .unwrap();

    let role_connection_path = format!("/users/@me/applications/{APP}/role-connection");
    let expected: Vec<Expected> = vec![
        ("GET", format!("/users/@me/guilds/{GUILD}/member"), "", ""),
        ("GET", "/users/@me/connections".into(), "", ""),
        ("GET", role_connection_path.clone(), "", ""),
        (
            "PUT",
            role_connection_path.clone(),
            "",
            r#"{"platform_name":"Chess Club","metadata":{"rating":"1900"}}"#,
        ),
        ("DELETE", role_connection_path, "", ""),
    ];
    let seen = seen(&state);
    check(&seen, &expected);
    assert!(
        seen.iter()
            .all(|request| request.authorization.as_deref() == Some("Bearer user-access-token"))
    );
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
    }
}
