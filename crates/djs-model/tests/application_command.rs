//! Application commands against the docs' examples.
//!
//! The docs show the bodies used to *create* commands; the tests add the fields Discord adds
//! (`id`, `application_id`, `version`) and decode them as returned commands.

use djs_model::{
    application_command::{
        ApplicationCommand, CommandOption, CommandOptionType, CommandPermission,
        CommandPermissionType, CommandType, EntryPointHandlerType, GuildCommandPermissions,
    },
    channel::ChannelType,
    id::{GenericId, GuildId},
    permissions::Permissions,
    util::Locale,
};
use serde_json::{Value, json};

mod common;

use common::assert_subset;

fn examples() -> Vec<(String, Value)> {
    let examples: Vec<Value> = serde_json::from_str(include_str!(
        "fixtures/docs/application_command_examples.json"
    ))
    .unwrap();
    examples
        .into_iter()
        .map(|example| {
            let mut command = example["command"].clone();
            command["id"] = "1300000000000000001".into();
            command["application_id"] = "1300000000000000002".into();
            command["version"] = "1300000000000000003".into();
            (example["heading"].as_str().unwrap().to_owned(), command)
        })
        .collect()
}

fn decode(json: &Value) -> ApplicationCommand {
    let command: ApplicationCommand = serde_json::from_value(json.clone()).unwrap();
    assert_subset(json, &serde_json::to_value(&command).unwrap(), "$");
    command
}

fn find(heading: &str) -> Vec<ApplicationCommand> {
    examples()
        .iter()
        .filter(|(name, _)| name == heading)
        .map(|(_, json)| decode(json))
        .collect()
}

#[test]
fn every_docs_example_round_trips() {
    let examples = examples();
    assert_eq!(examples.len(), 11);
    for (_, json) in &examples {
        decode(json);
    }
}

#[test]
fn slash_command_with_choices() {
    let command = &find("Example Slash Command")[0];
    assert_eq!(command.kind, CommandType::ChatInput);
    let CommandOption::String(animal) = &command.options[0] else {
        panic!("expected a string option");
    };
    assert!(animal.required);
    assert_eq!(animal.choices.len(), 3);
    assert_eq!(animal.choices[0].value, "animal_dog");
    let CommandOption::Boolean(only_smol) = &command.options[1] else {
        panic!("expected a boolean option");
    };
    assert!(!only_smol.required);
}

#[test]
fn subcommand_groups() {
    let walkthrough = find("Example Walkthrough");
    let command = walkthrough.last().unwrap();
    let groups = &command.options;
    assert!(
        groups
            .iter()
            .all(|group| group.kind() == CommandOptionType::SubCommandGroup)
    );
    let user = &groups[0];
    assert_eq!(user.name(), "user");
    let get = &user.options()[0];
    assert_eq!(get.kind(), CommandOptionType::SubCommand);
    assert!(
        get.options()
            .iter()
            .any(|option| option.kind() == CommandOptionType::Channel)
    );
}

#[test]
fn other_command_types_and_localization() {
    let user = &find("Example User Command")[0];
    assert_eq!(user.kind, CommandType::User);
    assert!(user.description.is_empty());
    let entry = &find("Example Entry Point Command")[0];
    assert_eq!(entry.kind, CommandType::PrimaryEntryPoint);
    assert_eq!(
        entry.handler,
        Some(EntryPointHandlerType::DiscordLaunchActivity)
    );
    let permissions = &find("Example of editing permissions")[0];
    assert_eq!(
        permissions.default_member_permissions,
        Some(Permissions::empty())
    );

    let localized = &find("Localization")[0];
    assert_eq!(localized.name_localizations[&Locale::ChineseChina], "生日");
    let option = localized.options[0].common();
    assert!(!option.name_localizations.is_empty());

    let localized = &find("Retrieving localized commands")[0];
    assert_eq!(localized.name_localized.as_deref(), Some("生日"));
    assert_eq!(localized.options[0].common().name_localized, Some("岁数"));
}

fn option(json: Value) -> Result<CommandOption, serde_json::Error> {
    serde_json::from_value(json)
}

#[test]
fn typed_option_values() {
    let integer = option(json!({
        "type": 4, "name": "count", "description": "How many",
        "min_value": 1, "max_value": 10, "choices": [{ "name": "one", "value": 1 }]
    }))
    .unwrap();
    let CommandOption::Integer(integer) = integer else {
        panic!("expected an integer option");
    };
    assert_eq!((integer.min_value, integer.max_value), (Some(1), Some(10)));
    assert_eq!(integer.choices[0].value, 1);

    // Number options accept integers and decimals.
    let number = option(json!({
        "type": 10, "name": "ratio", "description": "Ratio",
        "min_value": 0, "max_value": 2.5, "choices": [{ "name": "half", "value": 0.5 }]
    }))
    .unwrap();
    let CommandOption::Number(number) = &number else {
        panic!("expected a number option");
    };
    assert_eq!((number.min_value, number.max_value), (Some(0.0), Some(2.5)));
    assert_eq!(
        serde_json::to_value(CommandOption::Number(number.clone())).unwrap()["max_value"],
        2.5
    );

    let channel = option(json!({
        "type": 7, "name": "where", "description": "Where", "channel_types": [0, 5]
    }))
    .unwrap();
    let CommandOption::Channel(channel) = channel else {
        panic!("expected a channel option");
    };
    assert_eq!(
        channel.channel_types,
        [ChannelType::GuildText, ChannelType::GuildAnnouncement]
    );

    // Values of the wrong type are rejected.
    for json in [
        json!({ "type": 3, "name": "s", "description": "d", "choices": [{ "name": "x", "value": 1 }] }),
        json!({ "type": 4, "name": "i", "description": "d", "min_value": 1.5 }),
        json!({ "type": 4, "name": "i", "description": "d", "choices": [{ "name": "x", "value": "1" }] }),
        json!({ "type": 10, "name": "n", "description": "d", "max_value": "2" }),
    ] {
        assert!(option(json.clone()).is_err(), "{json}");
    }

    // Unknown option types keep the common fields.
    let unknown =
        option(json!({ "type": 42, "name": "new", "description": "New", "required": true }))
            .unwrap();
    assert_eq!(unknown.kind(), CommandOptionType::Unknown(42));
    assert_eq!(unknown.name(), "new");
    assert_eq!(serde_json::to_value(&unknown).unwrap()["type"], 42);
}

#[test]
fn command_permissions() {
    let guild_id = GuildId::new(1_300_000_000_000_000_100);
    let permissions: GuildCommandPermissions = serde_json::from_value(json!({
        "id": "1300000000000000001",
        "application_id": "1300000000000000002",
        "guild_id": "1300000000000000100",
        "permissions": [
            { "id": "1300000000000000100", "type": 1, "permission": false },
            { "id": "1300000000000000099", "type": 3, "permission": false },
            { "id": "1300000000000000005", "type": 2, "permission": true }
        ]
    }))
    .unwrap();
    let [everyone, channels, user] = permissions.permissions.as_slice() else {
        panic!("expected three permissions");
    };
    assert_eq!(everyone.id, CommandPermission::everyone(guild_id));
    assert_eq!(Some(channels.id), CommandPermission::all_channels(guild_id));
    assert_eq!(channels.kind, CommandPermissionType::Channel);
    assert!(user.permission);
    assert_eq!(
        CommandPermission::all_channels(GuildId::new(1)),
        None::<GenericId>
    );
}
