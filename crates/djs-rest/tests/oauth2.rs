//! The `OAuth2` endpoints against the mock API: the form bodies of the token endpoints, which
//! are sent without the client's token, and the two information endpoints.

use djs_model::{
    id::ApplicationId,
    oauth2::{AccessTokenResponse, Scope},
};
use djs_rest::{
    ErrorKind,
    oauth2::{OAuth2Credentials, TokenTypeHint},
};

mod common;

use common::{Canned, mock, seen};

const TOKEN: &str = r#"{"access_token": "user-access-token", "token_type": "Bearer",
    "expires_in": 604800, "refresh_token": "user-refresh-token", "scope": "identify guilds"}"#;
const CLIENT_TOKEN: &str = r#"{"access_token": "owner-access-token", "token_type": "Bearer",
    "expires_in": 604800, "scope": "identify applications.commands.update"}"#;
const APPLICATION: &str =
    include_str!("../../djs-model/tests/fixtures/docs/application_application.json");
const AUTHORIZATION: &str = include_str!(
    "../../djs-model/tests/fixtures/docs/oauth2_current_authorization_information.json"
);

const FORM: &str = "application/x-www-form-urlencoded";

fn credentials() -> OAuth2Credentials {
    OAuth2Credentials::new(ApplicationId::new(157), "client-secret-for-tests")
}

#[tokio::test]
async fn token_endpoints_send_forms_without_the_client_token() {
    let (rest, state) = mock(vec![
        Canned::json(200, TOKEN),
        Canned::json(200, TOKEN),
        Canned::json(200, CLIENT_TOKEN),
        Canned::json(200, ""),
    ])
    .await;
    let credentials = credentials();

    let token: AccessTokenResponse = rest
        .exchange_code(&credentials, "the-code", "https://example.com/cb")
        .await
        .unwrap();
    assert_eq!(token.access_token, "user-access-token");
    assert_eq!(
        token.scopes().collect::<Vec<_>>(),
        [Scope::Identify, Scope::Guilds]
    );
    rest.refresh_access_token(&credentials, "user-refresh-token")
        .await
        .unwrap();
    let owner = rest
        .client_credentials_grant(
            &credentials,
            &[Scope::Identify, Scope::ApplicationsCommandsUpdate],
        )
        .await
        .unwrap();
    assert_eq!(owner.refresh_token, None);
    rest.revoke_token(
        &credentials,
        "user-access-token",
        Some(TokenTypeHint::AccessToken),
    )
    .await
    .unwrap();

    let secret = "&client_id=157&client_secret=client-secret-for-tests";
    let expected = [
        (
            "/api/v10/oauth2/token",
            format!(
                "grant_type=authorization_code&code=the-code\
                 &redirect_uri=https%3A%2F%2Fexample.com%2Fcb{secret}"
            ),
        ),
        (
            "/api/v10/oauth2/token",
            format!("grant_type=refresh_token&refresh_token=user-refresh-token{secret}"),
        ),
        (
            "/api/v10/oauth2/token",
            format!(
                "grant_type=client_credentials\
                 &scope=identify%20applications.commands.update{secret}"
            ),
        ),
        (
            "/api/v10/oauth2/token/revoke",
            format!("token=user-access-token&token_type_hint=access_token{secret}"),
        ),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (path, body)) in seen.iter().zip(&expected) {
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, *path);
        assert_eq!(request.content_type.as_deref(), Some(FORM));
        assert_eq!(request.body, *body);
        assert_eq!(
            request.authorization, None,
            "{path} must not carry the bot token"
        );
    }
}

#[tokio::test]
async fn information_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, APPLICATION),
        Canned::json(200, AUTHORIZATION),
    ])
    .await;
    rest.get_current_bot_application_information()
        .await
        .unwrap();
    let information = rest.get_current_authorization_information().await.unwrap();
    assert_eq!(information.scopes, [Scope::GuildsJoin, Scope::Identify]);

    let seen = seen(&state);
    assert_eq!(seen[0].path, "/api/v10/oauth2/applications/@me");
    assert_eq!(seen[1].path, "/api/v10/oauth2/@me");
    assert!(
        seen.iter()
            .all(|request| request.authorization.as_deref() == Some("Bot test-token"))
    );
}

#[tokio::test]
async fn rejected_code_is_a_response_error() {
    let (rest, _) = mock(vec![Canned::json(
        400,
        r#"{"error": "invalid_grant", "error_description": "Invalid \"code\" in request."}"#,
    )])
    .await;
    let error = rest
        .exchange_code(&credentials(), "used-code", "https://example.com/cb")
        .await
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Response { status, error: None } if status.as_u16() == 400
    ));
}

#[test]
fn request_debug_hides_the_body() {
    let request = djs_rest::Request::new(djs_rest::Route::CreateMessage {
        channel_id: djs_model::id::ChannelId::new(1),
    })
    .json(&serde_json::json!({"access_token": "user-access-token"}))
    .unwrap();
    let debug = format!("{request:?}");
    assert!(!debug.contains("user-access-token"), "{debug}");
    assert!(debug.contains("<36 bytes>"), "{debug}");
}
