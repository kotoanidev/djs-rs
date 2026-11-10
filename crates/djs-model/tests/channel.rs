//! Channel (de)serialization (D-057): the docs' examples of every channel kind, plus forum,
//! stage and unknown channels.

use djs_model::{
    channel::{
        AutoArchiveDuration, Channel, ChannelFlags, ChannelType, ForumLayoutType, OverwriteType,
        SortOrderType, VideoQualityMode,
    },
    permissions::Permissions,
};
use serde_json::Value;

mod common;

/// Parses a channel, checks that serializing it keeps every field and that parsing the output
/// again yields the same channel.
fn round_trip(json: &str) -> Channel {
    let channel: Channel = common::round_trip(json, &[]);
    let reparsed: Channel =
        serde_json::from_value(serde_json::to_value(&channel).unwrap()).unwrap();
    assert_eq!(reparsed, channel, "second round trip");
    channel
}

#[test]
fn guild_text_channel() {
    let channel = round_trip(include_str!(
        "fixtures/docs/channel_guild_text_channel.json"
    ));
    assert_eq!(channel.kind(), ChannelType::GuildText);
    let Channel::Text(text) = channel else {
        panic!("not a text channel")
    };
    assert_eq!(text.name, "general");
    assert!(text.nsfw);
    assert_eq!(text.rate_limit_per_user, 2);
    assert_eq!(
        text.default_auto_archive_duration,
        Some(AutoArchiveDuration::OneHour)
    );
}

#[test]
fn announcement_channel() {
    let channel = round_trip(include_str!(
        "fixtures/docs/channel_guild_announcement_channel.json"
    ));
    assert!(matches!(channel, Channel::Announcement(_)));
    assert_eq!(channel.kind(), ChannelType::GuildAnnouncement);
}

#[test]
fn voice_channel() {
    let channel = round_trip(include_str!(
        "fixtures/docs/channel_guild_voice_channel.json"
    ));
    let Channel::Voice(voice) = channel else {
        panic!("not a voice channel")
    };
    assert_eq!(voice.bitrate, 64_000);
    assert_eq!(voice.user_limit, 0);
    assert_eq!(voice.rtc_region, None);
    assert_eq!(voice.video_quality_mode, VideoQualityMode::Auto);
}

#[test]
fn dm_and_group_dm() {
    let dm = round_trip(include_str!("fixtures/docs/channel_dm_channel.json"));
    let Channel::Dm(dm) = dm else {
        panic!("not a DM")
    };
    assert_eq!(dm.recipients.len(), 1);

    let group = round_trip(include_str!("fixtures/docs/channel_group_dm_channel.json"));
    assert_eq!(group.name(), Some("Some test channel"));
    assert!(matches!(group, Channel::GroupDm(_)));
}

#[test]
fn category() {
    let channel = round_trip(include_str!("fixtures/docs/channel_channel_category.json"));
    assert_eq!(channel.parent_id(), None);
    let Channel::Category(category) = channel else {
        panic!("not a category")
    };
    assert_eq!(category.name, "Test");
}

#[test]
fn thread() {
    let thread = round_trip(include_str!("fixtures/docs/channel_thread_channel.json"));
    assert_eq!(thread.kind(), ChannelType::PublicThread);
    let metadata = thread.as_thread().unwrap().thread_metadata;
    assert!(!metadata.archived);
    assert_eq!(metadata.auto_archive_duration, AutoArchiveDuration::OneDay);
    assert!(thread.kind().is_thread());
}

#[test]
fn forum_with_tags_and_overwrites() {
    let json = r#"{"id":"1500000000000000001","type":15,"guild_id":"1500000000000000002",
        "name":"help","position":3,"parent_id":null,"nsfw":false,
        "topic":"Ask one question per post","last_message_id":null,"rate_limit_per_user":0,
        "permission_overwrites":[{"id":"1500000000000000002","type":0,"allow":"0","deny":"2048"},
                                 {"id":"1500000000000000009","type":1,"allow":"2048","deny":"0"}],
        "available_tags":[{"id":"1500000000000000010","name":"solved","moderated":true,
                           "emoji_id":null,"emoji_name":"✅"}],
        "default_reaction_emoji":{"emoji_id":null,"emoji_name":"👍"},
        "default_sort_order":1,"default_forum_layout":2,"default_thread_rate_limit_per_user":60,
        "flags":16}"#;
    let channel = round_trip(json);
    assert_eq!(channel.flags(), ChannelFlags::REQUIRE_TAG);
    let overwrites = channel.permission_overwrites();
    assert_eq!(overwrites[0].kind, OverwriteType::Role);
    assert_eq!(overwrites[0].deny, Permissions::SEND_MESSAGES);
    assert_eq!(
        overwrites[0].role_id().map(djs_model::id::Id::get),
        Some(1_500_000_000_000_000_002)
    );
    assert_eq!(
        overwrites[1].user_id().map(djs_model::id::Id::get),
        Some(1_500_000_000_000_000_009)
    );
    let Channel::Forum(forum) = channel else {
        panic!("not a forum")
    };
    assert_eq!(forum.available_tags[0].emoji_name.as_deref(), Some("✅"));
    assert_eq!(forum.default_sort_order, Some(SortOrderType::CreationDate));
    assert_eq!(forum.default_forum_layout, ForumLayoutType::GalleryView);
}

#[test]
fn unknown_channel_types_are_kept() {
    let channel: Channel =
        serde_json::from_str(r#"{"id":"1","type":99,"name":"future","guild_id":"2"}"#).unwrap();
    assert_eq!(channel.kind(), ChannelType::Unknown(99));
    assert_eq!(channel.name(), Some("future"));
    assert_eq!(
        serde_json::to_value(&channel).unwrap()["type"],
        Value::from(99)
    );
}

#[test]
fn missing_required_fields_are_errors() {
    // A text channel needs a name; a thread needs its metadata.
    assert!(serde_json::from_str::<Channel>(r#"{"id":"1","type":0}"#).is_err());
    assert!(serde_json::from_str::<Channel>(r#"{"id":"1","type":11,"name":"t"}"#).is_err());
}
