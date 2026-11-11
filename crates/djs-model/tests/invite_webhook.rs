//! Invites, webhooks and stage instances against the docs' examples.
//!
//! The docs' invite example uses the placeholder avatar `deadbeef`, which is not a valid image
//! hash; the fixture uses a real-looking hash instead.

use djs_model::{
    channel::ChannelType,
    guild::VerificationLevel,
    invite::{Invite, InviteTargetType, InviteType, InviteWithMetadata},
    stage_instance::{PrivacyLevel, StageInstance},
    webhook::{Webhook, WebhookType},
};

mod common;

use common::round_trip;

#[test]
fn invite() {
    let invite: Invite = round_trip(include_str!("fixtures/docs/invite_invite_object.json"), &[]);
    assert_eq!(invite.kind, InviteType::Guild);
    assert_eq!(invite.code, "0vCdhLbwjZZTWZLD");
    let guild = invite.guild.as_ref().unwrap();
    assert_eq!(guild.verification_level, VerificationLevel::Medium);
    assert_eq!(guild.premium_subscription_count, Some(5));
    assert_eq!(
        invite.channel.as_ref().unwrap().kind,
        ChannelType::GuildText
    );
    assert_eq!(invite.target_type, Some(InviteTargetType::Stream));
    assert_eq!(invite.inviter.as_ref().unwrap().username, "speed");
}

#[test]
fn invite_with_metadata() {
    // The docs show the metadata alone; combine it with the invite example.
    let mut invite: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/docs/invite_invite_object.json")).unwrap();
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/docs/invite_invite_metadata.json")).unwrap();
    for (key, value) in metadata.as_object().unwrap() {
        invite[key] = value.clone();
    }
    let json = invite.to_string();
    let with_metadata: InviteWithMetadata = round_trip(&json, &[]);
    let metadata = with_metadata.metadata.unwrap();
    assert_eq!(metadata.uses, 0);
    assert!(!metadata.temporary);
    // Derefs to the invite.
    assert_eq!(with_metadata.code, "0vCdhLbwjZZTWZLD");

    // Without `MANAGE_GUILD`, Get Guild Invites sends no metadata (D-117).
    let without: InviteWithMetadata =
        round_trip(include_str!("fixtures/docs/invite_invite_object.json"), &[]);
    assert_eq!(without.metadata, None);
    assert_eq!(without.code, "0vCdhLbwjZZTWZLD");
}

#[test]
fn webhooks() {
    let incoming: Webhook = round_trip(
        include_str!("fixtures/docs/webhook_incoming_webhook.json"),
        &[],
    );
    assert_eq!(incoming.kind, WebhookType::Incoming);
    assert!(incoming.token.is_some());

    let follower: Webhook = round_trip(
        include_str!("fixtures/docs/webhook_channel_follower_webhook.json"),
        &[],
    );
    assert_eq!(follower.kind, WebhookType::ChannelFollower);
    assert_eq!(follower.source_channel.unwrap().name, "announcements");

    let application: Webhook = round_trip(
        include_str!("fixtures/docs/webhook_application_webhook.json"),
        &[],
    );
    assert_eq!(application.kind, WebhookType::Application);
}

#[test]
fn stage_instance() {
    let stage: StageInstance = round_trip(
        include_str!("fixtures/docs/stage_instance_stage_instance.json"),
        &[],
    );
    // The docs example still uses the deprecated public level.
    assert_eq!(stage.privacy_level, PrivacyLevel::Public);
}
