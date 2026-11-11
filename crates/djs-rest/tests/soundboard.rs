//! The soundboard endpoints against the mock API: method, path, body and audit log reason of
//! every request, and the unwrapping of the guild sound list.

use djs_model::{
    id::{ChannelId, GuildId, SoundboardSoundId},
    rest::{CreateGuildSoundboardSound, ModifyGuildSoundboardSound, SendSoundboardSound},
    util::{Patch, SoundData},
};

mod common;

use common::{Canned, mock, seen};

const DEFAULT_SOUND: &str =
    include_str!("../../djs-model/tests/fixtures/docs/soundboard_default_sound.json");
const GUILD_SOUND: &str =
    include_str!("../../djs-model/tests/fixtures/docs/soundboard_guild_sound.json");

const GUILD: GuildId = GuildId::new(613_425_648_685_547_541);
const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);
const ID: SoundboardSoundId = SoundboardSoundId::new(1_106_714_396_018_884_649);

/// `(method, path after /api/v10, body, audit log reason)`.
type Expected = (&'static str, String, String, Option<&'static str>);

#[tokio::test]
async fn soundboard_endpoints() {
    let sound = || Canned::json(200, GUILD_SOUND);
    let (rest, state) = mock(vec![
        Canned::json(204, ""),
        Canned::json(200, &format!("[{DEFAULT_SOUND}]")),
        Canned::json(200, &format!(r#"{{"items": [{GUILD_SOUND}]}}"#)),
        sound(),
        sound(),
        sound(),
        Canned::json(204, ""),
    ])
    .await;

    let mut send = SendSoundboardSound::new(ID);
    send.source_guild_id = Some(GUILD);
    rest.send_soundboard_sound(CHANNEL, &send).await.unwrap();
    let defaults = rest.list_default_soundboard_sounds().await.unwrap();
    assert_eq!(defaults[0].guild_id, None);
    let sounds = rest.list_guild_soundboard_sounds(GUILD).await.unwrap();
    assert_eq!(sounds[0].sound_id, ID);
    rest.get_guild_soundboard_sound(GUILD, ID).await.unwrap();
    let sound_data = SoundData::new(&b"ID3\x04\x00 mp3"[..]).unwrap();
    let mut create = CreateGuildSoundboardSound::new("Yay", sound_data.clone());
    create.volume = Some(0.5);
    rest.create_guild_soundboard_sound(GUILD, &create, Some("party"))
        .await
        .unwrap();
    let mut modify = ModifyGuildSoundboardSound::default();
    modify.volume = Patch::Set(1.0);
    modify.emoji_id = Patch::Clear;
    modify.emoji_name = Patch::Set("🎉".into());
    rest.modify_guild_soundboard_sound(GUILD, ID, &modify, None)
        .await
        .unwrap();
    rest.delete_guild_soundboard_sound(GUILD, ID, Some("cleanup"))
        .await
        .unwrap();

    let list = format!("/guilds/{GUILD}/soundboard-sounds");
    let one = format!("{list}/{ID}");
    let expected: Vec<Expected> = vec![
        (
            "POST",
            format!("/channels/{CHANNEL}/send-soundboard-sound"),
            format!(r#"{{"sound_id":"{ID}","source_guild_id":"{GUILD}"}}"#),
            None,
        ),
        (
            "GET",
            "/soundboard-default-sounds".into(),
            String::new(),
            None,
        ),
        ("GET", list.clone(), String::new(), None),
        ("GET", one.clone(), String::new(), None),
        (
            "POST",
            list,
            format!(r#"{{"name":"Yay","sound":"{sound_data}","volume":0.5}}"#),
            Some("party"),
        ),
        (
            "PATCH",
            one.clone(),
            r#"{"volume":1.0,"emoji_id":null,"emoji_name":"🎉"}"#.to_owned(),
            None,
        ),
        ("DELETE", one, String::new(), Some("cleanup")),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, body, reason)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
