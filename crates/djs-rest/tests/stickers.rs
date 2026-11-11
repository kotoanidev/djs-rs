//! The sticker endpoints against the mock API: method, path, body and audit log reason of every
//! request, the unwrapped sticker pack list and the multipart form of Create Guild Sticker.

use djs_model::{
    id::{GuildId, StickerId, StickerPackId},
    rest::{CreateGuildSticker, FileUpload, ModifyGuildSticker},
    util::Patch,
};

mod common;

use common::{Canned, mock, seen};

const STANDARD: &str = r#"{"id": "749054660769218631", "name": "Wave", "tags": "wumpus, hello",
    "type": 1, "format_type": 3, "description": "Wumpus waves hello",
    "pack_id": "847199849233514549", "sort_value": 12}"#;

const GUILD_STICKER: &str = r#"{"id": "1260000000000000001", "name": "Ferris", "tags": "crab",
    "type": 2, "format_type": 1, "description": "", "available": true,
    "guild_id": "1100000000000000000",
    "user": {"id": "7", "username": "ferris", "discriminator": "0", "global_name": null,
             "avatar": null}}"#;

const PACK: &str = r#"{"id": "847199849233514549", "stickers": [], "name": "Wumpus Beyond",
    "sku_id": "847199849233514547", "cover_sticker_id": "749053689419006003",
    "description": "Say hello to Wumpus!", "banner_asset_id": "761773777976819732"}"#;

const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);
const ID: StickerId = StickerId::new(1_260_000_000_000_000_001);
const PACK_ID: StickerPackId = StickerPackId::new(847_199_849_233_514_549);

/// `(method, path after /api/v10, body, audit log reason)`.
type Expected = (&'static str, String, &'static str, Option<&'static str>);

#[tokio::test]
async fn sticker_and_pack_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, STANDARD),
        Canned::json(200, &format!(r#"{{"sticker_packs": [{PACK}]}}"#)),
        Canned::json(200, PACK),
    ])
    .await;

    let sticker = rest
        .get_sticker(StickerId::new(749_054_660_769_218_631))
        .await
        .unwrap();
    assert_eq!(sticker.pack_id, Some(PACK_ID));
    let packs = rest.list_sticker_packs().await.unwrap();
    assert_eq!(packs[0].name, "Wumpus Beyond");
    rest.get_sticker_pack(PACK_ID).await.unwrap();

    let expected: Vec<Expected> = vec![
        ("GET", "/stickers/749054660769218631".into(), "", None),
        ("GET", "/sticker-packs".into(), "", None),
        ("GET", format!("/sticker-packs/{PACK_ID}"), "", None),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn guild_sticker_endpoints() {
    let sticker = || Canned::json(200, GUILD_STICKER);
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{GUILD_STICKER}]")),
        sticker(),
        sticker(),
        Canned::json(204, ""),
    ])
    .await;

    let stickers = rest.list_guild_stickers(GUILD).await.unwrap();
    assert!(stickers[0].user.is_some());
    rest.get_guild_sticker(GUILD, ID).await.unwrap();
    let mut modify = ModifyGuildSticker::default();
    modify.name = Some("Crab".into());
    modify.description = Patch::Clear;
    rest.modify_guild_sticker(GUILD, ID, &modify, Some("rename"))
        .await
        .unwrap();
    rest.delete_guild_sticker(GUILD, ID, Some("cleanup"))
        .await
        .unwrap();

    let list = format!("/guilds/{GUILD}/stickers");
    let one = format!("{list}/{ID}");
    let expected: Vec<Expected> = vec![
        ("GET", list, "", None),
        ("GET", one.clone(), "", None),
        (
            "PATCH",
            one.clone(),
            r#"{"name":"Crab","description":null}"#,
            Some("rename"),
        ),
        ("DELETE", one, "", Some("cleanup")),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn create_guild_sticker_sends_form_fields() {
    let (rest, state) = mock(vec![Canned::json(200, GUILD_STICKER)]).await;
    let mut body = CreateGuildSticker::new("Ferris", "crab", FileUpload::new("ferris.png", "png"));
    body.description = "The Rust mascot".into();
    rest.create_guild_sticker(GUILD, &body, Some("mascot"))
        .await
        .unwrap();

    let request = &seen(&state)[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/api/v10/guilds/{GUILD}/stickers"));
    assert_eq!(request.audit_log_reason.as_deref(), Some("mascot"));
    let content_type = request.content_type.as_deref().unwrap();
    let boundary = content_type
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    let part = |headers: &str, value: &str| format!("--{boundary}\r\n{headers}\r\n\r\n{value}\r\n");
    let text = |name: &str, value: &str| {
        part(
            &format!(r#"Content-Disposition: form-data; name="{name}""#),
            value,
        )
    };
    let expected = [
        part(
            "Content-Disposition: form-data; name=\"file\"; filename=\"ferris.png\"\r\nContent-Type: image/png",
            "png",
        ),
        text("name", "Ferris"),
        text("description", "The Rust mascot"),
        text("tags", "crab"),
        format!("--{boundary}--\r\n"),
    ]
    .concat();
    assert_eq!(request.body, expected);
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
    }
}
