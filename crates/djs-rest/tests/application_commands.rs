//! The application command endpoints against the mock API: method, path, query and body of
//! every request.

use djs_model::{
    application_command::{CommandPermission, CommandPermissionType},
    id::{ApplicationId, CommandId, GuildId, RoleId},
    rest::{
        CreateApplicationCommand, EditApplicationCommand, EditApplicationCommandPermissions,
        GetApplicationCommands,
    },
    util::Patch,
};

mod common;

use common::{Canned, mock, mock_with, seen};

const APP: ApplicationId = ApplicationId::new(1_100_000_000_000_000_001);
const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);
const COMMAND: CommandId = CommandId::new(1_400_000_000_000_000_001);

const COMMAND_JSON: &str = r#"{
    "id": "1400000000000000001", "application_id": "1100000000000000001", "version": "1",
    "type": 1, "name": "ping", "description": "Checks that the bot is alive",
    "default_member_permissions": null
}"#;

const PERMISSIONS_JSON: &str = r#"{
    "id": "1400000000000000001", "application_id": "1100000000000000001",
    "guild_id": "1100000000000000000",
    "permissions": [{"id": "1500000000000000000", "type": 1, "permission": true}]
}"#;

/// `(method, path after /api/v10/applications/{APP}, query, body)`.
type Expected = (&'static str, String, &'static str, String);

#[tokio::test]
async fn global_command_endpoints() {
    let command = || Canned::json(200, COMMAND_JSON);
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{COMMAND_JSON}]")),
        Canned::json(201, COMMAND_JSON),
        command(),
        command(),
        Canned::json(204, ""),
        Canned::json(200, &format!("[{COMMAND_JSON}]")),
    ])
    .await;

    let mut query = GetApplicationCommands::default();
    query.with_localizations = Some(true);
    let commands = rest
        .get_global_application_commands(APP, &query)
        .await
        .unwrap();
    assert_eq!(commands[0].name, "ping");

    let ping = CreateApplicationCommand::chat_input("ping", "Checks that the bot is alive");
    let created = rest
        .create_global_application_command(APP, &ping)
        .await
        .unwrap();
    assert_eq!(created.id, COMMAND);

    rest.get_global_application_command(APP, COMMAND)
        .await
        .unwrap();
    let mut edit = EditApplicationCommand::default();
    edit.default_member_permissions = Patch::Clear;
    rest.edit_global_application_command(APP, COMMAND, &edit)
        .await
        .unwrap();
    rest.delete_global_application_command(APP, COMMAND)
        .await
        .unwrap();
    let all = [ping, CreateApplicationCommand::user("Show avatar")];
    let registered = rest
        .bulk_overwrite_global_application_commands(APP, &all)
        .await
        .unwrap();
    assert_eq!(registered.len(), 1);

    let one = format!("/commands/{COMMAND}");
    let expected: Vec<Expected> = vec![
        (
            "GET",
            "/commands".into(),
            "with_localizations=true",
            String::new(),
        ),
        ("POST", "/commands".into(), "", ping_json()),
        ("GET", one.clone(), "", String::new()),
        (
            "PATCH",
            one.clone(),
            "",
            r#"{"default_member_permissions":null}"#.into(),
        ),
        ("DELETE", one, "", String::new()),
        (
            "PUT",
            "/commands".into(),
            "",
            format!(r#"[{},{{"name":"Show avatar","type":2}}]"#, ping_json()),
        ),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn guild_command_endpoints() {
    let command = || Canned::json(200, COMMAND_JSON);
    let (rest, state) = mock(vec![
        Canned::json(200, "[]"),
        command(),
        command(),
        command(),
        Canned::json(204, ""),
        Canned::json(200, "[]"),
        Canned::json(200, &format!("[{PERMISSIONS_JSON}]")),
        Canned::json(200, PERMISSIONS_JSON),
    ])
    .await;

    rest.get_guild_application_commands(APP, GUILD, &GetApplicationCommands::default())
        .await
        .unwrap();
    let ping = CreateApplicationCommand::chat_input("ping", "Checks that the bot is alive");
    rest.create_guild_application_command(APP, GUILD, &ping)
        .await
        .unwrap();
    rest.get_guild_application_command(APP, GUILD, COMMAND)
        .await
        .unwrap();
    let mut edit = EditApplicationCommand::default();
    edit.nsfw = Some(true);
    rest.edit_guild_application_command(APP, GUILD, COMMAND, &edit)
        .await
        .unwrap();
    rest.delete_guild_application_command(APP, GUILD, COMMAND)
        .await
        .unwrap();
    let none = rest
        .bulk_overwrite_guild_application_commands(APP, GUILD, &[])
        .await
        .unwrap();
    assert_eq!(none.len(), 0);

    let all = rest
        .get_guild_application_command_permissions(APP, GUILD)
        .await
        .unwrap();
    assert_eq!(all[0].permissions[0].kind, CommandPermissionType::Role);
    let one = rest
        .get_application_command_permissions(APP, GUILD, COMMAND)
        .await
        .unwrap();
    assert_eq!(one.guild_id, GUILD);

    let commands = format!("/guilds/{GUILD}/commands");
    let command_path = format!("{commands}/{COMMAND}");
    let expected: Vec<Expected> = vec![
        ("GET", commands.clone(), "", String::new()),
        ("POST", commands.clone(), "", ping_json()),
        ("GET", command_path.clone(), "", String::new()),
        ("PATCH", command_path.clone(), "", r#"{"nsfw":true}"#.into()),
        ("DELETE", command_path.clone(), "", String::new()),
        ("PUT", commands.clone(), "", "[]".into()),
        ("GET", format!("{commands}/permissions"), "", String::new()),
        (
            "GET",
            format!("{command_path}/permissions"),
            "",
            String::new(),
        ),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn command_permissions_are_edited_with_a_bearer_token() {
    let (rest, state) = mock_with(vec![Canned::json(200, PERMISSIONS_JSON)], |builder| {
        builder.bearer_token("user-access-token")
    })
    .await;
    let role = RoleId::new(1_500_000_000_000_000_000);
    let body = EditApplicationCommandPermissions::new(vec![CommandPermission::new(
        role.cast(),
        CommandPermissionType::Role,
        true,
    )]);
    let permissions = rest
        .edit_application_command_permissions(APP, GUILD, COMMAND, &body)
        .await
        .unwrap();
    assert_eq!(permissions.permissions, body.permissions);

    let request = &seen(&state)[0];
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.path,
        format!("/api/v10/applications/{APP}/guilds/{GUILD}/commands/{COMMAND}/permissions")
    );
    assert_eq!(
        request.body,
        r#"{"permissions":[{"id":"1500000000000000000","type":1,"permission":true}]}"#
    );
    assert_eq!(
        request.authorization.as_deref(),
        Some("Bearer user-access-token")
    );
}

fn ping_json() -> String {
    r#"{"name":"ping","description":"Checks that the bot is alive","type":1}"#.into()
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(
            request.path,
            format!("/api/v10/applications/{APP}{path}"),
            "{what}"
        );
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
    }
}
