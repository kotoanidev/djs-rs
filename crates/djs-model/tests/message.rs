//! Messages and everything attached to them: the docs' examples plus payloads that exercise
//! every field (the docs have no examples for most sub-objects).

use djs_model::{
    channel::ChannelType,
    gateway::{DispatchEvent, GatewayEvent, GatewayEventDeserializer},
    id::Id,
    interaction::{GuildInstallOwner, InteractionType},
    message::{
        AllowedMentionType, AllowedMentions, AttachmentFlags, BaseTheme, EmbedType,
        InteractionMetadata, Mention, Message, MessageActivityType, MessageFlags,
        MessageReferenceType, MessageType, Nonce, ReferencedMessage,
    },
    poll::LayoutType,
    user::User,
};
use serde::de::DeserializeSeed;

mod common;

use common::round_trip;

#[test]
fn docs_message() {
    let message: Message = round_trip(include_str!("fixtures/docs/message_message.json"), &[]);
    assert_eq!(message.content, "Supa Hot");
    assert_eq!(message.kind, MessageType::Default);
    assert_eq!(message.author.discriminator, 9999);
    let reaction = &message.reactions[0];
    assert_eq!((reaction.count, reaction.count_details.normal), (1, 1));
    assert_eq!(reaction.emoji.name(), Some("🔥"));
    assert!(message.referenced_message.is_not_fetched());
    assert!(message.member.is_none());
}

#[test]
fn docs_crossposted_message() {
    let message: Message = round_trip(
        include_str!("fixtures/docs/message_crossposted_message.json"),
        &[],
    );
    assert_eq!(message.flags, MessageFlags::IS_CROSSPOST);
    let reference = message.message_reference.unwrap();
    assert_eq!(reference.kind, MessageReferenceType::Default);
    assert_eq!(
        reference.message_id.map(Id::get),
        Some(306_588_351_130_107_906)
    );
    let channel = &message.mention_channels[0];
    assert_eq!(channel.kind, ChannelType::GuildAnnouncement);
    assert_eq!(channel.name, "big-news");
}

#[test]
fn gateway_message_create_extra_fields() {
    let json = include_str!("fixtures/message_create.json");
    let seed = GatewayEventDeserializer::from_json(json).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message)) =
        seed.deserialize(&mut deserializer).unwrap()
    else {
        panic!("expected MESSAGE_CREATE");
    };
    assert_eq!(
        message.member.as_ref().unwrap().nick.as_deref(),
        Some("Nell")
    );
    assert_eq!(message.channel_type, Some(ChannelType::GuildText));
    // `"referenced_message": null` means the replied-to message was deleted (D-061).
    assert!(message.referenced_message.is_deleted());
    assert!(message.nonce.is_some());
}

/// A message that sets (almost) every field.
const FULL: &str = r##"{
    "id": "1300000000000000001",
    "channel_id": "1300000000000000002",
    "guild_id": "1300000000000000003",
    "author": { "id": "1300000000000000004", "username": "nelly", "discriminator": "0",
                "global_name": "Nelly", "avatar": null },
    "member": { "roles": [], "joined_at": "2024-01-01T00:00:00.000000+00:00", "deaf": false,
                "mute": false, "flags": 0 },
    "content": "hello <@1300000000000000005>",
    "timestamp": "2026-11-11T10:00:00.000000+00:00",
    "edited_timestamp": "2026-11-11T10:05:00.000000+00:00",
    "tts": false,
    "mention_everyone": false,
    "mentions": [
        { "id": "1300000000000000005", "username": "kuba", "discriminator": "0",
          "global_name": null, "avatar": null, "public_flags": 64,
          "member": { "roles": ["1300000000000000006"],
                      "joined_at": "2024-02-02T00:00:00.000000+00:00",
                      "deaf": false, "mute": false, "flags": 0, "nick": "Kuba" } }
    ],
    "mention_roles": ["1300000000000000006"],
    "attachments": [
        { "id": "1300000000000000007", "filename": "voice-message.ogg",
          "content_type": "audio/ogg", "size": 21000,
          "url": "https://cdn.discordapp.com/attachments/1/2/voice-message.ogg",
          "proxy_url": "https://media.discordapp.net/attachments/1/2/voice-message.ogg",
          "duration_secs": 4.25, "waveform": "AAAAAAAAAAAA", "flags": 8, "ephemeral": true }
    ],
    "embeds": [
        { "type": "rich", "title": "Title", "description": "Description",
          "url": "https://example.com", "timestamp": "2026-11-11T09:00:00.000000+00:00",
          "color": 5793266,
          "footer": { "text": "Footer", "icon_url": "https://example.com/f.png" },
          "image": { "url": "https://example.com/i.png", "width": 640, "height": 480,
                     "flags": 32 },
          "thumbnail": { "url": "https://example.com/t.png" },
          "video": { "url": "https://example.com/v.mp4" },
          "provider": { "name": "Example" },
          "author": { "name": "Author", "url": "https://example.com/a" },
          "fields": [ { "name": "A", "value": "1", "inline": true },
                      { "name": "B", "value": "2" } ],
          "flags": 16 },
        { "type": "poll_result", "fields": [ { "name": "total_votes", "value": "3" } ] }
    ],
    "reactions": [
        { "count": 3, "count_details": { "burst": 1, "normal": 2 }, "me": true,
          "me_burst": false, "emoji": { "id": "1300000000000000008", "name": "ferris",
                                         "animated": true },
          "burst_colors": ["#ff0000", "#00ff00"] }
    ],
    "nonce": "1300000000000000009",
    "pinned": true,
    "webhook_id": "1300000000000000010",
    "type": 19,
    "activity": { "type": 1, "party_id": "spotify:1" },
    "application": { "id": "1300000000000000011", "name": "Game", "icon": null,
                     "description": "", "cover_image": null },
    "application_id": "1300000000000000011",
    "flags": 4128,
    "message_reference": { "type": 0, "message_id": "1300000000000000012",
                           "channel_id": "1300000000000000002",
                           "guild_id": "1300000000000000003" },
    "referenced_message": {
        "id": "1300000000000000012", "channel_id": "1300000000000000002",
        "author": { "id": "1300000000000000005", "username": "kuba", "discriminator": "0",
                    "global_name": null, "avatar": null },
        "content": "original", "timestamp": "2026-11-11T09:59:00.000000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
    },
    "interaction_metadata": {
        "id": "1300000000000000013", "type": 5,
        "user": { "id": "1300000000000000004", "username": "nelly", "discriminator": "0",
                  "global_name": "Nelly", "avatar": null },
        "authorizing_integration_owners": { "0": "1300000000000000003",
                                            "1": "1300000000000000004" },
        "triggering_interaction_metadata": {
            "id": "1300000000000000014", "type": 3,
            "user": { "id": "1300000000000000004", "username": "nelly",
                      "discriminator": "0", "global_name": "Nelly", "avatar": null },
            "authorizing_integration_owners": { "0": "0" },
            "interacted_message_id": "1300000000000000015"
        }
    },
    "interaction": {
        "id": "1300000000000000013", "type": 2, "name": "ping",
        "user": { "id": "1300000000000000004", "username": "nelly", "discriminator": "0",
                  "global_name": "Nelly", "avatar": null }
    },
    "thread": { "id": "1300000000000000016", "type": 11, "guild_id": "1300000000000000003",
                "name": "thread", "parent_id": "1300000000000000002",
                "owner_id": "1300000000000000004", "message_count": 0, "member_count": 1,
                "rate_limit_per_user": 0, "flags": 0,
                "thread_metadata": { "archived": false, "auto_archive_duration": 1440,
                                     "archive_timestamp": "2026-11-11T10:00:00.000000+00:00",
                                     "locked": false } },
    "sticker_items": [ { "id": "1300000000000000017", "name": "wave", "format_type": 1 } ],
    "position": 7,
    "role_subscription_data": { "role_subscription_listing_id": "1300000000000000018",
                                "tier_name": "Gold", "total_months_subscribed": 12,
                                "is_renewal": true },
    "poll": {
        "question": { "text": "Tabs or spaces?" },
        "answers": [ { "answer_id": 1, "poll_media": { "text": "Tabs" } },
                     { "answer_id": 2, "poll_media": { "text": "Spaces",
                                                       "emoji": { "id": null,
                                                                  "name": "🚀" } } } ],
        "expiry": "2026-11-12T10:00:00.000000+00:00",
        "allow_multiselect": false,
        "layout_type": 1,
        "results": { "is_finalized": false,
                     "answer_counts": [ { "id": 2, "count": 5, "me_voted": true } ] }
    },
    "call": { "participants": ["1300000000000000004"],
              "ended_timestamp": "2026-11-11T11:00:00.000000+00:00" },
    "shared_client_theme": { "colors": ["5865F2", "7258F2"], "gradient_angle": 90,
                             "base_mix": 58, "base_theme": 3 },
    "message_snapshots": [
        { "message": { "type": 0, "content": "forwarded", "embeds": [], "attachments": [],
                       "timestamp": "2026-11-10T10:00:00.000000+00:00",
                       "edited_timestamp": null, "flags": 0, "mentions": [],
                       "mention_roles": [] } }
    ],
    "channel_type": 0
}"##;

#[test]
fn full_message() {
    let message: Message = round_trip(FULL, &[]);
    assert_eq!(message.kind, MessageType::Reply);
    assert_eq!(
        message.flags,
        MessageFlags::SUPPRESS_NOTIFICATIONS | MessageFlags::HAS_THREAD
    );
    assert_eq!(
        message.nonce,
        Some(Nonce::String("1300000000000000009".into()))
    );
    assert_eq!(message.member.as_ref().unwrap().roles.len(), 0);

    // Mentions carry their members and deref to the user (D-060).
    let mention = &message.mentions[0];
    assert_eq!(mention.username, "kuba");
    assert_eq!(
        mention.member.as_ref().unwrap().nick.as_deref(),
        Some("Kuba")
    );

    let attachment = &message.attachments[0];
    assert_eq!(attachment.duration_secs, Some(4.25));
    assert_eq!(attachment.flags, AttachmentFlags::IS_SPOILER);
    assert!(attachment.ephemeral);

    let embed = &message.embeds[0];
    assert_eq!(embed.kind, Some(EmbedType::Rich));
    assert_eq!(embed.image.as_ref().unwrap().width, Some(640));
    assert_eq!(embed.fields.len(), 2);
    assert!(embed.fields[0].inline && !embed.fields[1].inline);
    assert_eq!(message.embeds[1].kind, Some(EmbedType::PollResult));

    let reaction = &message.reactions[0];
    assert_eq!(
        reaction.emoji.id().map(Id::get),
        Some(1_300_000_000_000_000_008)
    );
    assert_eq!(reaction.count_details.burst, 1);

    assert_eq!(
        message.activity.as_ref().unwrap().kind,
        MessageActivityType::Join
    );
    let replied = message.referenced_message.as_message().unwrap();
    assert_eq!(replied.content, "original");

    let InteractionMetadata::ModalSubmit(modal) = message.interaction_metadata.as_deref().unwrap()
    else {
        panic!("expected modal submit metadata");
    };
    let owners = modal.authorizing_integration_owners;
    assert_eq!(
        owners.guild_install.and_then(GuildInstallOwner::guild_id),
        message.guild_id
    );
    assert_eq!(owners.user_install, Some(message.author.id));
    let InteractionMetadata::MessageComponent(component) = &*modal.triggering_interaction_metadata
    else {
        panic!("expected component metadata");
    };
    assert_eq!(
        component.authorizing_integration_owners.guild_install,
        Some(GuildInstallOwner::BotDm)
    );
    assert_eq!(
        message.interaction.as_ref().unwrap().kind,
        InteractionType::ApplicationCommand
    );

    assert!(message.thread.as_ref().unwrap().as_thread().is_some());
    assert_eq!(message.sticker_items[0].name, "wave");
    assert_eq!(message.position, Some(7));
    assert!(message.role_subscription_data.as_ref().unwrap().is_renewal);

    let poll = message.poll.as_ref().unwrap();
    assert_eq!(poll.layout_type, LayoutType::Default);
    assert_eq!(
        poll.answers[1].poll_media.emoji.as_ref().unwrap().name(),
        Some("🚀")
    );
    assert_eq!(poll.results.as_ref().unwrap().answer_counts[0].count, 5);

    assert_eq!(message.call.as_ref().unwrap().participants.len(), 1);
    assert_eq!(
        message.shared_client_theme.as_ref().unwrap().base_theme,
        Some(BaseTheme::Darker)
    );
    assert_eq!(message.message_snapshots[0].message.content, "forwarded");
}

#[test]
fn referenced_message_has_three_states() {
    let mut json: serde_json::Value = serde_json::from_str(FULL).unwrap();
    let message: Message = serde_json::from_value(json.clone()).unwrap();
    assert!(matches!(
        message.referenced_message,
        ReferencedMessage::Message(_)
    ));

    json["referenced_message"] = serde_json::Value::Null;
    let message: Message = serde_json::from_value(json.clone()).unwrap();
    assert!(message.referenced_message.is_deleted());
    assert!(message.referenced_message.as_message().is_none());
    let written = serde_json::to_value(&message).unwrap();
    assert!(written["referenced_message"].is_null());
    assert!(
        written
            .as_object()
            .unwrap()
            .contains_key("referenced_message")
    );

    json.as_object_mut().unwrap().remove("referenced_message");
    let message: Message = serde_json::from_value(json).unwrap();
    assert!(message.referenced_message.is_not_fetched());
    let written = serde_json::to_value(&message).unwrap();
    assert!(
        !written
            .as_object()
            .unwrap()
            .contains_key("referenced_message")
    );
}

#[test]
fn nonces() {
    let parse = |json: &str| serde_json::from_str::<Nonce>(json).unwrap();
    assert_eq!(parse("42"), Nonce::Integer(42));
    assert_eq!(parse("-7"), Nonce::Integer(-7));
    assert_eq!(parse(r#""abc""#), Nonce::String("abc".into()));
    // Beyond `i64`: kept as text.
    assert_eq!(
        parse("18446744073709551615"),
        Nonce::String("18446744073709551615".into())
    );
    assert_eq!(serde_json::to_string(&Nonce::Integer(42)).unwrap(), "42");
    assert!(serde_json::from_str::<Nonce>("1.5").is_err());
}

#[test]
fn mention_decodes_like_user() {
    // Every field `User` knows; `RawMention` must list the same ones.
    let json = r#"{
        "id": "80351110224678912", "username": "nelly", "discriminator": "1337",
        "global_name": "Nelly", "avatar": "8342729096ea3675442027381ff50dfe",
        "bot": true, "system": true, "banner": "06c16474723fe537c283b8efa61a30c8",
        "accent_color": 16711680, "public_flags": 131072,
        "avatar_decoration_data": { "asset": "a_fed43ab12698df65902ba06727e20c0e",
                                    "sku_id": "1144058522808614923" },
        "collectibles": { "nameplate": { "sku_id": "1349849614353829990",
                                         "asset": "nameplates/nameplates/twilight/",
                                         "label": "", "palette": "cobalt" } },
        "primary_guild": { "identity_guild_id": "1234647491267808778",
                           "identity_enabled": true, "tag": "DISC",
                           "badge": "7d1734ae5a615e82bc7a4033b98fade8" }
    }"#;
    let user: User = serde_json::from_str(json).unwrap();
    let mention: Mention = serde_json::from_str(json).unwrap();
    assert_eq!(mention.user, user);
    assert!(mention.member.is_none());
    assert_eq!(
        serde_json::to_value(&mention).unwrap(),
        serde_json::to_value(&user).unwrap()
    );
}

#[test]
fn interaction_metadata_variants() {
    let user = r#"{"id":"1","username":"a","discriminator":"0","global_name":null,"avatar":null}"#;
    let decode = |extra: &str| {
        serde_json::from_str::<InteractionMetadata>(&format!(
            r#"{{"id":"9","user":{user},"authorizing_integration_owners":{{}}{extra}}}"#
        ))
    };
    let command = decode(r#","type":2,"target_message_id":"5""#).unwrap();
    let InteractionMetadata::ApplicationCommand(metadata) = &command else {
        panic!("expected a command");
    };
    assert_eq!(metadata.target_message_id.map(Id::get), Some(5));
    assert_eq!(command.kind(), InteractionType::ApplicationCommand);
    assert_eq!(command.id().get(), 9);
    assert_eq!(command.user().username, "a");
    assert_eq!(
        *command.authorizing_integration_owners(),
        djs_model::interaction::AuthorizingIntegrationOwners::default()
    );

    // Unknown interaction types keep the common fields.
    let unknown = decode(r#","type":42"#).unwrap();
    assert!(matches!(unknown, InteractionMetadata::Unknown(_)));
    assert_eq!(unknown.kind(), InteractionType::Unknown(42));
    let written = serde_json::to_value(&unknown).unwrap();
    assert_eq!(written["type"], 42);

    // Required fields of a variant.
    let error = decode(r#","type":3"#).unwrap_err().to_string();
    assert!(error.contains("interacted_message_id"), "{error}");
    let error = decode(r#","type":5"#).unwrap_err().to_string();
    assert!(error.contains("triggering_interaction_metadata"), "{error}");
}

#[test]
fn allowed_mentions() {
    assert_eq!(
        serde_json::to_string(&AllowedMentions::default()).unwrap(),
        "{}"
    );
    let mentions: AllowedMentions =
        serde_json::from_str(r#"{"parse":["users","everyone"],"replied_user":true}"#).unwrap();
    assert_eq!(
        mentions.parse,
        [AllowedMentionType::Users, AllowedMentionType::Everyone]
    );
    assert!(mentions.replied_user && mentions.roles.is_empty());
}
