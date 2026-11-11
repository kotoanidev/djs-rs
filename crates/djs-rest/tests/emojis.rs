//! The emoji endpoints against the mock API: method, path, body and audit log reason of every
//! request, and the unwrapping of the application emoji list.

use djs_model::{
    id::{ApplicationId, EmojiId, GuildId, RoleId},
    rest::{CreateApplicationEmoji, CreateGuildEmoji, ModifyApplicationEmoji, ModifyGuildEmoji},
    util::{ImageData, Patch},
};

mod common;

use common::{Canned, mock, seen};

const EMOJI: &str = r#"{"id": "1250000000000000001", "name": "ferris", "roles": [],
    "user": {"id": "7", "username": "ferris", "discriminator": "0", "global_name": null,
             "avatar": null},
    "require_colons": true, "managed": false, "animated": false, "available": true}"#;

const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);
const APP: ApplicationId = ApplicationId::new(1_100_000_000_000_000_001);
const ID: EmojiId = EmojiId::new(1_250_000_000_000_000_001);
const ROLE: RoleId = RoleId::new(1_100_000_000_000_000_002);

/// A 1×1 transparent GIF.
const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";

/// `(method, path after /api/v10, body, audit log reason)`.
type Expected = (&'static str, String, String, Option<&'static str>);

fn image() -> ImageData {
    ImageData::new(GIF).unwrap()
}

#[tokio::test]
async fn guild_emoji_endpoints() {
    let emoji = || Canned::json(200, EMOJI);
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{EMOJI}]")),
        emoji(),
        emoji(),
        emoji(),
        Canned::json(204, ""),
    ])
    .await;

    let emojis = rest.list_guild_emojis(GUILD).await.unwrap();
    assert_eq!(emojis[0].id, ID);
    assert!(
        rest.get_guild_emoji(GUILD, ID)
            .await
            .unwrap()
            .user
            .is_some()
    );
    let mut create = CreateGuildEmoji::new("ferris", image());
    create.roles.push(ROLE);
    rest.create_guild_emoji(GUILD, &create, Some("mascot"))
        .await
        .unwrap();
    let mut modify = ModifyGuildEmoji::default();
    modify.name = Some("crab".into());
    modify.roles = Patch::Clear;
    rest.modify_guild_emoji(GUILD, ID, &modify, Some("rename"))
        .await
        .unwrap();
    rest.delete_guild_emoji(GUILD, ID, None).await.unwrap();

    let list = format!("/guilds/{GUILD}/emojis");
    let one = format!("{list}/{ID}");
    let expected: Vec<Expected> = vec![
        ("GET", list.clone(), String::new(), None),
        ("GET", one.clone(), String::new(), None),
        (
            "POST",
            list,
            format!(
                r#"{{"name":"ferris","image":"{}","roles":["{ROLE}"]}}"#,
                image()
            ),
            Some("mascot"),
        ),
        (
            "PATCH",
            one.clone(),
            r#"{"name":"crab","roles":null}"#.to_owned(),
            Some("rename"),
        ),
        ("DELETE", one, String::new(), None),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn application_emoji_endpoints() {
    let emoji = || Canned::json(200, EMOJI);
    let (rest, state) = mock(vec![
        Canned::json(200, &format!(r#"{{"items": [{EMOJI}, {EMOJI}]}}"#)),
        emoji(),
        emoji(),
        emoji(),
        Canned::json(204, ""),
    ])
    .await;

    let emojis = rest.list_application_emojis(APP).await.unwrap();
    assert_eq!(emojis.len(), 2);
    assert_eq!(emojis[1].name, "ferris");
    rest.get_application_emoji(APP, ID).await.unwrap();
    rest.create_application_emoji(APP, &CreateApplicationEmoji::new("loading", image()))
        .await
        .unwrap();
    let mut modify = ModifyApplicationEmoji::default();
    modify.name = Some("spinner".into());
    rest.modify_application_emoji(APP, ID, &modify)
        .await
        .unwrap();
    rest.delete_application_emoji(APP, ID).await.unwrap();

    let list = format!("/applications/{APP}/emojis");
    let one = format!("{list}/{ID}");
    let expected: Vec<Expected> = vec![
        ("GET", list.clone(), String::new(), None),
        ("GET", one.clone(), String::new(), None),
        (
            "POST",
            list,
            format!(r#"{{"name":"loading","image":"{}"}}"#, image()),
            None,
        ),
        (
            "PATCH",
            one.clone(),
            r#"{"name":"spinner"}"#.to_owned(),
            None,
        ),
        ("DELETE", one, String::new(), None),
    ];
    check(&seen(&state), &expected);
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, body, reason)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, "", "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
        assert!(request.authorization.is_some(), "{what}");
    }
}
