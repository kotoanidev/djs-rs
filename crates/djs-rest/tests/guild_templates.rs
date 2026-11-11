//! The guild template endpoints against the mock API: method, path and body of every request.

use djs_model::{
    id::GuildId,
    rest::{CreateGuildTemplate, ModifyGuildTemplate},
    util::Patch,
};

mod common;

use common::{Canned, mock, seen};

const TEMPLATE: &str = include_str!("../../djs-model/tests/fixtures/docs/guild_template.json");
const CODE: &str = "hgM48av5Q69A";
const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);

#[tokio::test]
async fn guild_template_endpoints() {
    let template = || Canned::json(200, TEMPLATE);
    let (rest, state) = mock(vec![
        template(),
        Canned::json(200, &format!("[{TEMPLATE}]")),
        template(),
        template(),
        template(),
        template(),
    ])
    .await;

    let fetched = rest.get_guild_template(CODE).await.unwrap();
    assert_eq!(fetched.code, CODE);
    assert_eq!(rest.get_guild_templates(GUILD).await.unwrap().len(), 1);
    let mut create = CreateGuildTemplate::new("Study group");
    create.description = Some("For students".into());
    rest.create_guild_template(GUILD, &create).await.unwrap();
    rest.sync_guild_template(GUILD, CODE).await.unwrap();
    let mut modify = ModifyGuildTemplate::default();
    modify.name = Some("Book club".into());
    modify.description = Patch::Clear;
    rest.modify_guild_template(GUILD, CODE, &modify)
        .await
        .unwrap();
    rest.delete_guild_template(GUILD, CODE).await.unwrap();

    let list = format!("/guilds/{GUILD}/templates");
    let one = format!("{list}/{CODE}");
    let expected = [
        ("GET", format!("/guilds/templates/{CODE}"), ""),
        ("GET", list.clone(), ""),
        (
            "POST",
            list,
            r#"{"name":"Study group","description":"For students"}"#,
        ),
        ("PUT", one.clone(), ""),
        (
            "PATCH",
            one.clone(),
            r#"{"name":"Book club","description":null}"#,
        ),
        ("DELETE", one, ""),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, body)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.body, *body, "{what}");
    }
}
