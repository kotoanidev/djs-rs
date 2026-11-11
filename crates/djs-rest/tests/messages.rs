//! The message, reaction and pin endpoints against the mock API: method, path, query, body
//! and audit log reason of every request.

use djs_model::{
    id::{ChannelId, EmojiId, MessageId, UserId},
    message::ReactionType,
    rest::{
        BulkDeleteMessages, CreateMessage, EditMessage, FileUpload, GetChannelMessages,
        GetChannelPins, GetReactions,
    },
    util::Patch,
};
use djs_rest::ReactionEmoji;

mod common;

use common::{Canned, mock, seen};

const MESSAGE: &str = r#"{
    "id": "1300000000000000001", "channel_id": "1200000000000000000",
    "author": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
               "avatar": null, "bot": true},
    "content": "Pong!", "timestamp": "2026-11-11T12:00:00.000000+00:00",
    "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
    "type": 0
}"#;

const USER: &str = r#"{"id": "7", "username": "ferris", "discriminator": "0", "global_name": null, "avatar": null}"#;

const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);
const MSG: MessageId = MessageId::new(1_300_000_000_000_000_001);

/// `(method, path after /api/v10, query, body, audit log reason)`.
type Expected = (
    &'static str,
    String,
    &'static str,
    &'static str,
    Option<&'static str>,
);

fn path(rest: &str) -> String {
    format!("/api/v10/channels/{CHANNEL}{rest}")
}

#[tokio::test]
async fn message_endpoints() {
    let no_content = || Canned::json(204, "");
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{MESSAGE}]")),
        Canned::json(200, MESSAGE),
        Canned::json(200, MESSAGE),
        Canned::json(200, MESSAGE),
        Canned::json(200, MESSAGE),
        no_content(),
        no_content(),
        Canned::json(
            200,
            &format!(r#"{{"items":[{{"pinned_at":"2026-11-11T12:00:00+00:00","message":{MESSAGE}}}],"has_more":true}}"#),
        ),
        no_content(),
        no_content(),
    ])
    .await;

    let mut query = GetChannelMessages::default();
    query.before = Some(MSG);
    query.limit = Some(2);
    let messages = rest.get_channel_messages(CHANNEL, &query).await.unwrap();
    assert_eq!(messages.len(), 1);

    let message = rest.get_channel_message(CHANNEL, MSG).await.unwrap();
    assert_eq!(message.id, MSG);

    let mut body = CreateMessage::from("hi");
    body.sticker_ids.push(djs_model::id::StickerId::new(1));
    rest.create_message(CHANNEL, &body).await.unwrap();

    rest.crosspost_message(CHANNEL, MSG).await.unwrap();

    let mut edit = EditMessage::default();
    edit.content = Patch::Set("edited".into());
    edit.components = Patch::Clear;
    rest.edit_message(CHANNEL, MSG, &edit).await.unwrap();

    rest.delete_message(CHANNEL, MSG).await.unwrap();

    let bulk = BulkDeleteMessages::new(vec![MessageId::new(1), MessageId::new(2)]);
    rest.bulk_delete_messages(CHANNEL, &bulk, Some("cleanup"))
        .await
        .unwrap();

    let mut pins_query = GetChannelPins::default();
    pins_query.limit = Some(1);
    let pins = rest.get_channel_pins(CHANNEL, &pins_query).await.unwrap();
    assert!(pins.has_more);
    assert_eq!(pins.items[0].message.id, MSG);

    rest.pin_message(CHANNEL, MSG, Some("important"))
        .await
        .unwrap();
    rest.unpin_message(CHANNEL, MSG, None).await.unwrap();

    check(&seen(&state), &message_expectations());
}

/// The requests of `message_endpoints`, in order.
fn message_expectations() -> Vec<Expected> {
    let message_path = format!("/messages/{MSG}");
    vec![
        (
            "GET",
            path("/messages"),
            "before=1300000000000000001&limit=2",
            "",
            None,
        ),
        ("GET", path(&message_path), "", "", None),
        (
            "POST",
            path("/messages"),
            "",
            r#"{"content":"hi","sticker_ids":["1"]}"#,
            None,
        ),
        (
            "POST",
            path(&format!("{message_path}/crosspost")),
            "",
            "",
            None,
        ),
        (
            "PATCH",
            path(&message_path),
            "",
            r#"{"content":"edited","components":null}"#,
            None,
        ),
        ("DELETE", path(&message_path), "", "", None),
        (
            "POST",
            path("/messages/bulk-delete"),
            "",
            r#"{"messages":["1","2"]}"#,
            Some("cleanup"),
        ),
        ("GET", path("/messages/pins"), "limit=1", "", None),
        (
            "PUT",
            path(&format!("/messages/pins/{MSG}")),
            "",
            "",
            Some("important"),
        ),
        (
            "DELETE",
            path(&format!("/messages/pins/{MSG}")),
            "",
            "",
            None,
        ),
    ]
}

#[tokio::test]
async fn reaction_endpoints() {
    let no_content = || Canned::json(204, "");
    let (rest, state) = mock(vec![
        no_content(),
        no_content(),
        no_content(),
        Canned::json(200, &format!("[{USER}]")),
        no_content(),
        no_content(),
    ])
    .await;
    let thumbs = ReactionEmoji::Unicode("👍");
    let ferris = ReactionEmoji::Custom {
        name: "ferris",
        id: EmojiId::new(3),
    };

    rest.create_reaction(CHANNEL, MSG, thumbs).await.unwrap();
    rest.delete_own_reaction(CHANNEL, MSG, ferris)
        .await
        .unwrap();
    rest.delete_user_reaction(CHANNEL, MSG, thumbs, UserId::new(7))
        .await
        .unwrap();
    let mut query = GetReactions::default();
    query.kind = Some(ReactionType::Burst);
    query.limit = Some(100);
    let users = rest
        .get_reactions(CHANNEL, MSG, ferris, &query)
        .await
        .unwrap();
    assert_eq!(users[0].username, "ferris");
    rest.delete_all_reactions(CHANNEL, MSG).await.unwrap();
    rest.delete_all_reactions_for_emoji(CHANNEL, MSG, thumbs)
        .await
        .unwrap();

    let reactions = format!("/messages/{MSG}/reactions");
    let expected: Vec<Expected> = vec![
        (
            "PUT",
            path(&format!("{reactions}/%F0%9F%91%8D/@me")),
            "",
            "",
            None,
        ),
        (
            "DELETE",
            path(&format!("{reactions}/ferris%3A3/@me")),
            "",
            "",
            None,
        ),
        (
            "DELETE",
            path(&format!("{reactions}/%F0%9F%91%8D/7")),
            "",
            "",
            None,
        ),
        (
            "GET",
            path(&format!("{reactions}/ferris%3A3")),
            "type=1&limit=100",
            "",
            None,
        ),
        ("DELETE", path(&reactions), "", "", None),
        (
            "DELETE",
            path(&format!("{reactions}/%F0%9F%91%8D")),
            "",
            "",
            None,
        ),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn edit_message_with_files_is_multipart() {
    let (rest, state) = mock(vec![Canned::json(200, MESSAGE)]).await;
    let mut edit = EditMessage::default();
    edit.files.push(FileUpload::new("a.txt", "a"));
    rest.edit_message(CHANNEL, MSG, &edit).await.unwrap();
    let request = &seen(&state)[0];
    assert!(
        request
            .content_type
            .as_deref()
            .unwrap()
            .starts_with("multipart/form-data; boundary=")
    );
    assert!(
        request
            .body
            .contains(r#"{"attachments":[{"id":0,"filename":"a.txt"}]}"#)
    );
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body, reason)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, *path, "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
