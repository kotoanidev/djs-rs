//! The typed getters of interaction data (D-250) and the accessors of [`Interaction`] by kind
//! (D-249), on synthetic payloads built from the docs tables and the docs' context menu
//! examples.

use djs_model::{
    id::{AttachmentId, ChannelId, GenericId, RoleId, UserId},
    interaction::{
        CommandData, CommandOptionValue, ComponentData, Interaction, Mentionable, ModalSubmitData,
    },
};
use serde_json::{Value, json};

const USER: &str = "1300000000000000011";
const OTHER_USER: &str = "1300000000000000016";
const CHANNEL: &str = "1300000000000000012";
const ROLE: &str = "1300000000000000013";
const ATTACHMENT: &str = "1300000000000000015";

fn interaction(kind: u8, data: &Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1300000000000000001", "application_id": "1300000000000000002", "type": kind,
        "token": "token", "version": 1, "app_permissions": "0",
        "guild_id": "1300000000000000003", "channel_id": "1300000000000000004",
        "data": data,
    }))
    .unwrap()
}

fn user(id: &str, name: &str) -> Value {
    json!({ "id": id, "username": name, "discriminator": "0", "global_name": null,
            "avatar": null })
}

fn member() -> Value {
    json!({ "roles": [], "joined_at": "2026-11-11T12:00:00.000000+00:00", "flags": 0,
            "pending": false, "permissions": "8", "nick": "Mixo" })
}

/// Users (one of them a member), a channel, a role and an attachment.
fn resolved() -> Value {
    json!({
        "users": { USER: user(USER, "mixo"), OTHER_USER: user(OTHER_USER, "dm-only") },
        "members": { USER: member() },
        "channels": { CHANNEL: { "id": CHANNEL, "type": 0, "name": "general",
                                 "permissions": "1024" } },
        "roles": { ROLE: { "id": ROLE, "name": "Tester", "color": 0, "hoist": false,
                           "colors": { "primary_color": 0, "secondary_color": null,
                                       "tertiary_color": null },
                           "position": 1, "permissions": "0", "managed": false,
                           "mentionable": false, "flags": 0 } },
        "attachments": { ATTACHMENT: { "id": ATTACHMENT, "filename": "log.txt", "size": 12,
                                       "url": "https://cdn.discordapp.com/x/log.txt",
                                       "proxy_url": "https://media.discordapp.net/x/log.txt" } },
    })
}

fn every_type() -> Value {
    json!([
        { "name": "text", "type": 3, "value": "hi" },
        { "name": "count", "type": 4, "value": -5 },
        { "name": "enabled", "type": 5, "value": true },
        { "name": "who", "type": 6, "value": USER },
        { "name": "stranger", "type": 6, "value": OTHER_USER },
        { "name": "where", "type": 7, "value": CHANNEL },
        { "name": "role", "type": 8, "value": ROLE },
        { "name": "any_user", "type": 9, "value": USER },
        { "name": "any_role", "type": 9, "value": ROLE },
        { "name": "ratio", "type": 10, "value": 0.25 },
        { "name": "file", "type": 11, "value": ATTACHMENT },
    ])
}

fn command(options: &Value) -> CommandData {
    let interaction = interaction(
        2,
        &json!({ "id": "1300000000000000010", "name": "settings", "type": 1,
                 "options": options, "resolved": resolved() }),
    );
    interaction.command().unwrap().clone()
}

#[test]
fn options_are_found_inside_subcommands_and_groups() {
    let flat = command(&every_type());
    assert_eq!((flat.subcommand_group(), flat.subcommand()), (None, None));
    assert_eq!(flat.leaf_options().len(), 11);

    let sub = command(&json!([{ "name": "set", "type": 1, "options": every_type() }]));
    assert_eq!(
        (sub.subcommand_group(), sub.subcommand()),
        (None, Some("set"))
    );
    assert_eq!(sub.leaf_options().len(), 11);

    let group = command(&json!([{ "name": "user", "type": 2, "options": [
        { "name": "set", "type": 1, "options": every_type() }
    ]}]));
    assert_eq!(
        (group.subcommand_group(), group.subcommand()),
        (Some("user"), Some("set"))
    );
    for data in [&flat, &sub, &group] {
        assert_eq!(data.get_string("text"), Some("hi"));
        assert_eq!(data.get_integer("count"), Some(-5));
        assert_eq!(data.get_boolean("enabled"), Some(true));
        assert_eq!(data.get_number("ratio"), Some(0.25));
        assert_eq!(
            data.get("where"),
            Some(&CommandOptionValue::Channel(ChannelId::new(
                1_300_000_000_000_000_012
            )))
        );
    }

    // An option-less subcommand has no leaf options.
    let empty = command(&json!([{ "name": "list", "type": 1 }]));
    assert_eq!(empty.subcommand(), Some("list"));
    assert_eq!(empty.leaf_options(), []);
}

#[test]
fn getters_check_the_name_and_the_type() {
    let data = command(&every_type());
    assert_eq!(data.get_string("missing"), None);
    assert_eq!(data.get_string("count"), None);
    assert_eq!(data.get_integer("ratio"), None);
    assert_eq!(data.get_number("count"), None);
    assert_eq!(data.get_boolean("text"), None);
    assert!(data.get_user("where").is_none());
    assert!(data.get_role("who").is_none());
    assert!(data.get_channel("role").is_none());
    assert!(data.get_attachment("text").is_none());
    assert!(data.get_mentionable("who").is_none());
    assert!(data.focused_value().is_none());
    // Without resolved data, the objects are not found.
    let mut bare = data.clone();
    bare.resolved = None;
    assert!(bare.get_user("who").is_none() && bare.get_channel("where").is_none());
}

#[test]
fn resolved_objects() {
    let data = command(&every_type());
    let user_id = UserId::new(1_300_000_000_000_000_011);
    assert_eq!(data.get_user("who").unwrap().id, user_id);
    let (user, member) = data.get_member("who").unwrap();
    assert_eq!((user.id, member.nick.as_deref()), (user_id, Some("Mixo")));
    // A user who is not a member (for example outside the guild).
    assert_eq!(data.get_user("stranger").unwrap().username, "dm-only");
    assert!(data.get_member("stranger").is_none());

    assert_eq!(
        data.get_channel("where").unwrap().name.as_deref(),
        Some("general")
    );
    assert_eq!(data.get_role("role").unwrap().name, "Tester");
    assert_eq!(data.get_attachment("file").unwrap().filename, "log.txt");

    // Mentionable options resolve to either kind, and the user and role getters accept them.
    let Some(Mentionable::User { user, member }) = data.get_mentionable("any_user") else {
        panic!("expected a user");
    };
    assert_eq!((user.id, member.is_some()), (user_id, true));
    let role = data.get_mentionable("any_role").unwrap();
    assert!(matches!(role, Mentionable::Role(role) if role.name == "Tester"));
    assert_eq!(role.id(), GenericId::new(1_300_000_000_000_000_013));
    assert_eq!(data.get_user("any_user").unwrap().id, user_id);
    assert_eq!(
        data.get_role("any_role").unwrap().id,
        RoleId::new(1_300_000_000_000_000_013)
    );
    assert!(data.get_user("any_role").is_none());
}

#[test]
fn autocomplete() {
    let interaction = interaction(
        4,
        &json!({ "id": "1300000000000000010", "name": "search", "type": 1, "options": [
            { "name": "deep", "type": 1, "options": [
                { "name": "limit", "type": 4, "value": 5 },
                { "name": "query", "type": 3, "value": "djs-r", "focused": true },
            ]}
        ]}),
    );
    assert!(interaction.command().is_none());
    assert!(interaction.component().is_none() && interaction.modal().is_none());
    let data = interaction.autocomplete().unwrap();
    assert_eq!(data.focused_value(), Some("djs-r"));
    assert_eq!(data.focused().unwrap().name, "query");
    assert_eq!(data.get_integer("limit"), Some(5));
    // The focused option is not a typed value yet.
    assert_eq!(data.get_string("query"), None);
}

#[test]
fn context_menu_targets() {
    let examples: Vec<Value> = serde_json::from_str(include_str!(
        "fixtures/docs/application_command_interaction_examples.json"
    ))
    .unwrap();
    let user_command: Interaction =
        serde_json::from_value(examples[1]["interaction"].clone()).unwrap();
    let data = user_command.command().unwrap();
    let target = UserId::new(809_850_198_683_418_695);
    assert_eq!(data.target_user().unwrap().id, target);
    let (user, _member) = data.target_member().unwrap();
    assert_eq!(user.id, target);
    assert!(data.target_message().is_none());

    let message_command: Interaction =
        serde_json::from_value(examples[2]["interaction"].clone()).unwrap();
    let data = message_command.command().unwrap();
    assert_eq!(
        data.target_message().unwrap().id,
        data.target_id.unwrap().cast()
    );
    assert!(data.target_user().is_none() && data.target_member().is_none());
}

fn component(component_type: u8, values: &[&str]) -> ComponentData {
    let interaction = interaction(
        3,
        &json!({ "custom_id": "pick", "component_type": component_type, "values": values,
                 "resolved": resolved() }),
    );
    assert!(interaction.command().is_none());
    interaction.component().unwrap().clone()
}

#[test]
fn select_menu_selections() {
    let users = component(5, &[OTHER_USER, USER]);
    // In the order of the values, not of the IDs.
    let names: Vec<_> = users.users().map(|user| user.username.as_str()).collect();
    assert_eq!(names, ["dm-only", "mixo"]);
    let members: Vec<_> = users
        .members()
        .map(|(user, _)| user.username.as_str())
        .collect();
    assert_eq!(members, ["mixo"]);
    assert_eq!(users.roles().count(), 0);

    let roles = component(6, &[ROLE]);
    assert_eq!(roles.roles().next().unwrap().name, "Tester");
    let channels = component(8, &[CHANNEL]);
    assert_eq!(
        channels.channels().next().unwrap().id,
        ChannelId::new(1_300_000_000_000_000_012)
    );

    let mentionables = component(7, &[ROLE, USER]);
    let ids: Vec<_> = mentionables.mentionables().map(|m| m.id()).collect();
    assert_eq!(
        ids,
        [
            GenericId::new(1_300_000_000_000_000_013),
            GenericId::new(1_300_000_000_000_000_011)
        ]
    );
    assert_eq!(mentionables.users().count(), 1);
    assert_eq!(mentionables.roles().count(), 1);

    // String selects and buttons have nothing to resolve.
    assert_eq!(component(3, &["a"]).users().count(), 0);
    assert_eq!(component(2, &[]).mentionables().count(), 0);
}

fn modal() -> ModalSubmitData {
    let label = |id: u32, component: Value| json!({ "type": 18, "id": id, "component": component });
    let interaction = interaction(
        5,
        &json!({
            "custom_id": "form",
            "components": [
                label(1, json!({ "type": 4, "id": 2, "custom_id": "text", "value": "Hello" })),
                label(3, json!({ "type": 3, "id": 4, "custom_id": "pick", "values": ["a", "b"] })),
                label(5, json!({ "type": 5, "id": 6, "custom_id": "users",
                                 "values": [USER, OTHER_USER] })),
                label(7, json!({ "type": 6, "id": 8, "custom_id": "roles", "values": [ROLE] })),
                label(9, json!({ "type": 7, "id": 10, "custom_id": "anything",
                                 "values": [USER, ROLE] })),
                label(11, json!({ "type": 8, "id": 12, "custom_id": "channels",
                                  "values": [CHANNEL] })),
                label(13, json!({ "type": 19, "id": 14, "custom_id": "files",
                                  "values": [ATTACHMENT] })),
                label(15, json!({ "type": 21, "id": 16, "custom_id": "radio", "value": "b" })),
                label(17, json!({ "type": 21, "id": 18, "custom_id": "no_radio",
                                  "value": null })),
                label(19, json!({ "type": 22, "id": 20, "custom_id": "checks",
                                  "values": ["x"] })),
                label(21, json!({ "type": 23, "id": 22, "custom_id": "agree", "value": true })),
            ],
            "resolved": resolved(),
        }),
    );
    interaction.modal().unwrap().clone()
}

#[test]
fn modal_fields() {
    let modal = modal();
    assert_eq!(modal.text_value("text"), Some("Hello"));
    assert_eq!(modal.string_select_values("pick").unwrap(), ["a", "b"]);
    assert_eq!(modal.checkbox_group_values("checks").unwrap(), ["x"]);
    assert_eq!(modal.radio_group_value("radio"), Some("b"));
    assert_eq!(modal.radio_group_value("no_radio"), None);
    assert_eq!(modal.checkbox_value("agree"), Some(true));

    let users: Vec<_> = modal
        .selected_users("users")
        .unwrap()
        .map(|u| u.id)
        .collect();
    assert_eq!(
        users,
        [
            UserId::new(1_300_000_000_000_000_011),
            UserId::new(1_300_000_000_000_000_016)
        ]
    );
    assert_eq!(modal.selected_members("users").unwrap().count(), 1);
    assert_eq!(
        modal.selected_roles("roles").unwrap().next().unwrap().name,
        "Tester"
    );
    assert_eq!(
        modal
            .selected_channels("channels")
            .unwrap()
            .next()
            .unwrap()
            .name
            .as_deref(),
        Some("general")
    );
    assert_eq!(
        modal.uploaded_files("files").unwrap().next().unwrap().id,
        AttachmentId::new(1_300_000_000_000_000_015)
    );

    // A mentionable select feeds the user, member, role and mentionable getters.
    assert_eq!(modal.selected_users("anything").unwrap().count(), 1);
    assert_eq!(modal.selected_members("anything").unwrap().count(), 1);
    assert_eq!(modal.selected_roles("anything").unwrap().count(), 1);
    let kinds: Vec<_> = modal
        .selected_mentionables("anything")
        .unwrap()
        .map(|m| matches!(m, Mentionable::User { .. }))
        .collect();
    assert_eq!(kinds, [true, false]);
}

#[test]
fn modal_getters_check_the_custom_id_and_the_type() {
    let modal = modal();
    // The custom ID may be a temporary.
    assert!(modal.selected_users(&String::from("users")).is_some());
    assert!(modal.text_value("pick").is_none());
    assert!(modal.string_select_values("text").is_none());
    assert!(modal.checkbox_group_values("pick").is_none());
    assert!(modal.radio_group_value("agree").is_none());
    assert!(modal.checkbox_value("radio").is_none());
    assert!(modal.selected_users("roles").is_none());
    assert!(modal.selected_members("channels").is_none());
    assert!(modal.selected_roles("users").is_none());
    assert!(modal.selected_channels("anything").is_none());
    assert!(modal.selected_mentionables("users").is_none());
    assert!(modal.uploaded_files("missing").is_none());
}
