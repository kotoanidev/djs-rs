//! `OAuth2` token responses and authorization information against the docs' examples.
//!
//! The fixtures replace the docs' example tokens with plain placeholders. The guild of the bot
//! authorization example predates role `colors` and string `permissions`, so its role gets
//! `colors` and the value of `permissions_new` as `permissions`.

use djs_model::{
    id::{GuildId, WebhookId},
    oauth2::{AccessTokenResponse, CurrentAuthorizationInformation, Scope},
};

mod common;

use common::round_trip;

#[test]
fn bot_authorization_token_response() {
    let raw = include_str!("fixtures/docs/oauth2_bot_authorization_token_response.json");
    let response: AccessTokenResponse = serde_json::from_str(raw).unwrap();
    assert_eq!(response.scopes().collect::<Vec<_>>(), [Scope::Bot]);
    assert_eq!(response.expires_in, 604_800);
    let guild = response.guild.as_deref().unwrap();
    assert_eq!(guild.name, "SomeTest");
    let debug = format!("{response:?}");
    assert!(!debug.contains("access-token-from-the-docs"));
    assert!(!debug.contains("refresh-token-from-the-docs"));
}

#[test]
fn webhook_token_response() {
    let response: AccessTokenResponse = round_trip(
        include_str!("fixtures/docs/oauth2_webhook_token_response.json"),
        &[],
    );
    let webhook = response.webhook.as_deref().unwrap();
    assert_eq!(webhook.id, WebhookId::new(347_114_750_880_120_863));
    assert_eq!(
        webhook.guild_id,
        Some(GuildId::new(290_926_792_226_357_250))
    );
    assert!(!format!("{response:?}").contains("webhook-token-from-the-docs"));
}

#[test]
fn current_authorization_information() {
    let information: CurrentAuthorizationInformation = round_trip(
        include_str!("fixtures/docs/oauth2_current_authorization_information.json"),
        // Fields of the old application object that `PartialApplication` does not model.
        &[
            "application.hook",
            "application.bot_public",
            "application.bot_require_code_grant",
            "application.verify_key",
        ],
    );
    assert_eq!(information.scopes, [Scope::GuildsJoin, Scope::Identify]);
    assert_eq!(
        information.user.unwrap().global_name.as_deref(),
        Some("Discord")
    );
}
