//! The chainable setters of the message bodies (D-290, D-291): each setter writes the same
//! JSON as setting the field by hand, whatever the field's shape (`Option`, `Vec`, `Patch`).

use djs_model::{
    component::{ActionRow, LinkButton},
    id::Id,
    interaction::MessageCallbackData,
    message::{AllowedMentions, Embed, MessageFlags, MessageReference, Nonce},
    rest::{
        CreateFollowupMessage, CreateMessage, EditMessage, EditWebhookMessage, ExecuteWebhook,
        FileUpload,
    },
    util::Patch,
};
use serde::Serialize;

fn json(body: &impl Serialize) -> String {
    serde_json::to_string(body).unwrap()
}

fn embed(title: &str) -> Embed {
    let mut embed = Embed::default();
    embed.title = Some(title.into());
    embed
}

fn row() -> ActionRow {
    ActionRow::new(vec![LinkButton::new("https://example.com").into()])
}

#[test]
fn every_body_converts_from_text() {
    let text = String::from("hi");
    let compact = compact_str::CompactString::from("hi");
    assert_eq!(CreateMessage::from("hi"), CreateMessage::from(text.clone()));
    assert_eq!(
        CreateMessage::from("hi"),
        CreateMessage::from(compact.clone())
    );
    assert_eq!(EditMessage::from("hi").content, Patch::Set("hi".into()));
    assert_eq!(
        EditMessage::from(text.clone()),
        EditMessage::from(compact.clone())
    );
    assert_eq!(
        EditWebhookMessage::from(text.clone()).content,
        Patch::Set("hi".into())
    );
    assert_eq!(
        EditWebhookMessage::from(compact.clone()),
        EditWebhookMessage::from("hi")
    );
    assert_eq!(
        ExecuteWebhook::from(compact.clone()),
        ExecuteWebhook::from("hi")
    );
    assert_eq!(
        CreateFollowupMessage::from(compact.clone()),
        CreateFollowupMessage::from("hi")
    );
    assert_eq!(
        MessageCallbackData::from(compact).content.as_deref(),
        Some("hi")
    );
    assert_eq!(
        json(&MessageCallbackData::from(text)),
        r#"{"content":"hi"}"#
    );
}

#[test]
fn an_ephemeral_reply_in_one_expression() {
    let reply = MessageCallbackData::from("Done").ephemeral().no_mentions();
    assert_eq!(
        json(&reply),
        r#"{"content":"Done","allowed_mentions":{},"flags":64}"#
    );
    // The flag setters add to the flags set before; `flags` replaces them.
    let reply = MessageCallbackData::default()
        .ephemeral()
        .suppress_embeds()
        .suppress_notifications();
    assert_eq!(
        reply.flags,
        Some(
            MessageFlags::EPHEMERAL
                | MessageFlags::SUPPRESS_EMBEDS
                | MessageFlags::SUPPRESS_NOTIFICATIONS
        )
    );
    let reply = reply.flags(MessageFlags::SUPPRESS_EMBEDS);
    assert_eq!(reply.flags, Some(MessageFlags::SUPPRESS_EMBEDS));
    let followup = CreateFollowupMessage::from("Later").ephemeral();
    assert_eq!(json(&followup), r#"{"content":"Later","flags":64}"#);
}

#[test]
fn list_setters_push_or_replace_on_every_field_shape() {
    // `Vec` (an empty list is not sent).
    let message = CreateMessage::from("x")
        .embed(embed("a"))
        .embed(embed("b"))
        .component(row());
    assert_eq!(message.embeds.len(), 2);
    assert_eq!(message.components.len(), 1);
    assert_eq!(
        json(&CreateMessage::from("x").embeds([])),
        r#"{"content":"x"}"#
    );

    // `Option<Vec>`: an empty list is sent, and removes the embeds of an updated message.
    let update = MessageCallbackData::default()
        .embed(embed("a"))
        .embed(embed("b"));
    assert_eq!(update.embeds.as_ref().map(Vec::len), Some(2));
    assert_eq!(
        json(&MessageCallbackData::default().embeds([]).components([])),
        r#"{"embeds":[],"components":[]}"#
    );
    assert_eq!(json(&MessageCallbackData::default()), "{}");

    // `Patch<Vec>`: kept until set; a push on a kept list sets it to that one item.
    let edit = EditMessage::default().embed(embed("a"));
    assert_eq!(edit.embeds, Patch::Set(vec![embed("a")]));
    let edit = edit.embed(embed("b"));
    assert_eq!(edit.embeds, Patch::Set(vec![embed("a"), embed("b")]));
    assert_eq!(
        json(&EditWebhookMessage::default().embeds([]).attachments([])),
        r#"{"embeds":[],"attachments":[]}"#
    );
    assert_eq!(json(&EditWebhookMessage::default()), "{}");
}

#[test]
fn setters_match_the_fields_set_by_hand() {
    let mut by_hand = CreateMessage::from("Reply");
    by_hand.message_reference = Some(MessageReference::reply(Id::new(7)));
    by_hand.allowed_mentions = Some(AllowedMentions::default());
    by_hand.sticker_ids = vec![Id::new(1), Id::new(2)];
    by_hand.nonce = Some(Nonce::Integer(9));
    by_hand.enforce_nonce = true;
    by_hand.tts = true;
    by_hand.files.push(FileUpload::new("a.txt", &b"a"[..]));
    let chained = CreateMessage::from("Reply")
        .reply_to(Id::new(7))
        .no_mentions()
        .sticker(Id::new(1))
        .sticker(Id::new(2))
        .nonce(Nonce::Integer(9), true)
        .tts(true)
        .file(FileUpload::new("a.txt", &b"a"[..]));
    assert_eq!(chained, by_hand);
    assert_eq!(
        chained.clone().stickers([Id::new(3)]).sticker_ids,
        vec![Id::new(3)]
    );
    assert_eq!(chained.files([]).files, Vec::new());

    let mut by_hand = ExecuteWebhook::from("Post");
    by_hand.username = Some("Bot".into());
    by_hand.avatar_url = Some("https://example.com/a.png".into());
    by_hand.thread_name = Some("Topic".into());
    by_hand.applied_tags = vec![Id::new(5)];
    by_hand.with_components = Some(true);
    let chained = ExecuteWebhook::from("Post")
        .username("Bot")
        .avatar_url("https://example.com/a.png")
        .thread_name("Topic")
        .applied_tags([Id::new(5)])
        .with_components(true);
    assert_eq!(chained, by_hand);
    assert_eq!(
        ExecuteWebhook::default().thread_id(Id::new(4)).thread_id,
        Some(Id::new(4))
    );

    let edit = EditWebhookMessage::from("Edited")
        .thread_id(Id::new(4))
        .with_components(false)
        .suppress_embeds();
    assert_eq!(edit.thread_id, Some(Id::new(4)));
    assert_eq!(edit.with_components, Some(false));
    assert_eq!(edit.flags, Patch::Set(MessageFlags::SUPPRESS_EMBEDS));
    // The query fields are not part of the JSON.
    assert_eq!(json(&edit), r#"{"content":"Edited","flags":4}"#);
}
