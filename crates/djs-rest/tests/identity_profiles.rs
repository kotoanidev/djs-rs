//! The application identity profile endpoints and Get Gateway against the mock API.

use djs_model::{
    id::{ApplicationId, UserId},
    rest::{
        DeleteApplicationIdentity, GetApplicationIdentitiesByExternalId,
        UpdateApplicationIdentityProfile,
    },
};

mod common;

use common::{Canned, mock, seen};

const PROFILE: &str =
    include_str!("../../djs-model/tests/fixtures/docs/application_identity_profile.json");
const IDENTITIES: &str = r#"{"identities": [{"user_id": "7", "provider_type": "steam",
    "provider_issued_user_id": "player-42"}]}"#;

const APP: ApplicationId = ApplicationId::new(3);
const USER: UserId = UserId::new(7);

#[tokio::test]
async fn identity_profile_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(201, PROFILE),
        Canned::json(204, ""),
        Canned::json(200, PROFILE),
        Canned::json(200, IDENTITIES),
        Canned::json(200, IDENTITIES),
        Canned::json(204, ""),
    ])
    .await;

    let mut body = UpdateApplicationIdentityProfile::default();
    body.username = Some("ferris".into());
    let created = rest
        .update_application_identity_profile(APP, USER, "player-42", &body)
        .await
        .unwrap();
    assert!(created.is_some(), "201 returns the profile");
    let updated = rest
        .update_application_identity_profile(APP, USER, "player-42", &body)
        .await
        .unwrap();
    assert_eq!(updated, None, "204 returns nothing");
    rest.get_application_identity_profile(APP, USER, "player-42")
        .await
        .unwrap();

    let identities = rest
        .get_application_identities_by_user_id(USER, APP)
        .await
        .unwrap();
    assert_eq!(identities[0].user_id, Some(USER));
    let mut query = GetApplicationIdentitiesByExternalId::default();
    query.provider_id = Some("eu west".into());
    rest.get_application_identities_by_external_id(APP, "steam", "player-42", &query)
        .await
        .unwrap();
    rest.delete_application_identity(
        USER,
        APP,
        "steam",
        "player-42",
        &DeleteApplicationIdentity::default(),
    )
    .await
    .unwrap();

    let profile = "/api/v10/applications/3/users/7/identities/player-42/profile";
    let expected = [
        ("PATCH", profile, "", r#"{"username":"ferris"}"#),
        ("PATCH", profile, "", r#"{"username":"ferris"}"#),
        ("GET", profile, "", ""),
        ("GET", "/api/v10/users/7/application-identities/3", "", ""),
        (
            "GET",
            "/api/v10/applications/3/application-identities/steam/player-42",
            "provider_id=eu%20west",
            "",
        ),
        (
            "POST",
            "/api/v10/users/7/application-identities/3/steam/player-42/delete",
            "",
            "{}",
        ),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body)) in seen.iter().zip(&expected) {
        assert_eq!(request.method, *method, "{path}");
        assert_eq!(request.path, *path);
        assert_eq!(request.query, *query, "{path}");
        assert_eq!(request.body, *body, "{path}");
    }
}

#[tokio::test]
async fn get_gateway() {
    let (rest, state) = mock(vec![Canned::json(
        200,
        r#"{"url": "wss://gateway.discord.gg"}"#,
    )])
    .await;
    let gateway = rest.get_gateway().await.unwrap();
    assert_eq!(gateway.url, "wss://gateway.discord.gg");
    assert_eq!(seen(&state)[0].path, "/api/v10/gateway");
}
