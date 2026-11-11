//! Interactions and interaction responses: the docs' example interactions
//! (`fixtures/docs/application_command_interaction_examples.json`, the autocomplete example in
//! `application_command_autocomplete_response.json`), synthetic component and modal-submit
//! payloads built from the docs tables, and every response type.

use djs_model::{
    application_command::{Choice, CommandOptionType, CommandType},
    component::{ComponentType, TextInput, TextInputStyle},
    id::{AttachmentId, ChannelId, GenericId, MessageId, RoleId, UserId},
    interaction::{
        AutocompleteCallbackData, AutocompleteChoices, CommandDataOption, CommandOptionValue,
        DeferredCallbackData, Interaction, InteractionCallbackResponse, InteractionCallbackType,
        InteractionContextType, InteractionData, InteractionResponse, InteractionType,
        MessageCallbackData, ModalCallbackData, SubmittedComponent,
    },
    message::{AttachmentRequest, AttachmentRequestId, Message, MessageFlags},
    permissions::Permissions,
    poll::{CreatePoll, Media},
    util::Locale,
};
use serde_json::{Value, json};

mod common;

use common::{assert_subset, round_trip};

/// The docs' example interactions; the first one lacks the required `application_id`, which
/// the fixture adds (C-026).
fn examples() -> Vec<Value> {
    let examples: Vec<Value> = serde_json::from_str(include_str!(
        "fixtures/docs/application_command_interaction_examples.json"
    ))
    .unwrap();
    examples
        .into_iter()
        .map(|example| {
            let mut interaction = example["interaction"].clone();
            if interaction.get("application_id").is_none() {
                interaction["application_id"] = "775799577604522054".into();
            }
            interaction
        })
        .collect()
}

/// `is_pending` is an undocumented duplicate of `pending`.
const UNMODELLED: &[&str] = &[
    "member.is_pending",
    "data.resolved.members.809850198683418695.is_pending",
];

#[test]
fn slash_command_example() {
    let json = examples()[0].to_string();
    let interaction: Interaction = round_trip(&json, UNMODELLED);
    assert_eq!(interaction.kind(), InteractionType::ApplicationCommand);
    assert_eq!(interaction.version, 1);
    assert_eq!(interaction.locale, Some(Locale::EnglishUs));
    assert_eq!(
        interaction.app_permissions,
        Permissions::from_bits_retain(442_368)
    );
    let member = interaction.member.as_deref().unwrap();
    assert_eq!(
        member.permissions,
        Permissions::from_bits_retain(2_147_483_647)
    );
    assert_eq!(member.roles, [RoleId::new(539_082_325_061_836_999)]);
    assert_eq!(interaction.author().unwrap().username, "Mason");

    let InteractionData::ApplicationCommand(command) = &interaction.data else {
        panic!("expected a command");
    };
    assert_eq!(
        (command.name.as_str(), command.kind),
        ("cardsearch", CommandType::ChatInput)
    );
    let [option] = command.options.as_slice() else {
        panic!("expected one option");
    };
    assert_eq!(option.name, "cardname");
    assert_eq!(
        option.value,
        CommandOptionValue::String("The Gitrog Monster".into())
    );
    assert!(command.resolved.is_none());
}

#[test]
fn user_command_example() {
    let json = examples()[1].to_string();
    let interaction: Interaction = round_trip(&json, UNMODELLED);
    let command = interaction.data.command().unwrap();
    assert_eq!(command.kind, CommandType::User);
    let target = UserId::new(809_850_198_683_418_695);
    assert_eq!(command.target_id, Some(target.cast()));
    let resolved = interaction.data.resolved().unwrap();
    assert_eq!(resolved.users[&target].username, "VoltyDemo");
    let member = &resolved.members[&target];
    assert!(member.user.is_none());
    assert_eq!(
        member.permissions,
        Permissions::from_bits_retain(246_997_699_136)
    );
    assert_eq!(interaction.author().unwrap().username, "ian");
}

#[test]
fn message_command_example() {
    let json = examples()[2].to_string();
    let interaction: Interaction = round_trip(&json, UNMODELLED);
    let command = interaction.data.command().unwrap();
    assert_eq!(command.kind, CommandType::Message);
    let target = MessageId::new(867_793_854_505_943_041);
    assert_eq!(command.target_id, Some(target.cast()));
    let message = &command.resolved.as_deref().unwrap().messages[&target];
    assert_eq!(message.content, "some message");
}

#[test]
fn autocomplete_example() {
    // The docs show only `type` and `data`; the test adds the fields every interaction has.
    let mut json: Value = serde_json::from_str(include_str!(
        "fixtures/docs/application_command_autocomplete_response.json"
    ))
    .unwrap();
    json["id"] = "1300000000000000001".into();
    json["application_id"] = "1300000000000000002".into();
    json["token"] = "token".into();
    // The command `version` in the data is undocumented there.
    let interaction: Interaction = round_trip(&json.to_string(), &["data.version"]);
    let InteractionData::Autocomplete(command) = &interaction.data else {
        panic!("expected an autocomplete interaction");
    };
    let focused = command.focused().unwrap();
    assert_eq!(focused.name, "variant");
    assert_eq!(
        focused.value,
        CommandOptionValue::Focused {
            kind: CommandOptionType::String,
            value: "data a user is typ".into(),
        }
    );
}

fn interaction_json(kind: u8, data: &Value) -> Value {
    json!({
        "id": "1300000000000000001", "application_id": "1300000000000000002", "type": kind,
        "token": "token", "version": 1, "app_permissions": "0", "locale": "pl",
        "guild_id": "1300000000000000003", "guild_locale": "en-US",
        "guild": { "id": "1300000000000000003", "locale": "en-US", "features": ["COMMUNITY"] },
        "channel_id": "1300000000000000004",
        "channel": { "id": "1300000000000000004", "type": 0, "name": "general",
                     "guild_id": "1300000000000000003", "nsfw": false, "flags": 0,
                     "permissions": "1024", "position": 3, "topic": null,
                     "last_message_id": "1300000000000000099", "rate_limit_per_user": 0 },
        "member": { "user": { "id": "1300000000000000005", "username": "mixo",
                              "discriminator": "0", "global_name": "Mixo", "avatar": null },
                    "roles": [], "joined_at": "2026-11-11T12:00:00.000000+00:00",
                    "deaf": false, "mute": false, "flags": 0, "pending": false,
                    "permissions": "8" },
        "entitlements": [],
        "authorizing_integration_owners": { "0": "1300000000000000003" },
        "context": 0,
        "attachment_size_limit": 10_485_760,
        "data": data,
    })
}

#[test]
fn every_option_type() {
    let data = json!({
        "id": "1300000000000000010", "name": "settings", "type": 1,
        "options": [{ "name": "user", "type": 2, "options": [{ "name": "set", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "hi" },
            { "name": "count", "type": 4, "value": -5 },
            { "name": "enabled", "type": 5, "value": true },
            { "name": "who", "type": 6, "value": "1300000000000000011" },
            { "name": "where", "type": 7, "value": "1300000000000000012" },
            { "name": "role", "type": 8, "value": "1300000000000000013" },
            { "name": "any", "type": 9, "value": "1300000000000000014" },
            { "name": "ratio", "type": 10, "value": 0.25 },
            { "name": "file", "type": 11, "value": "1300000000000000015" },
            { "name": "future", "type": 99, "value": "x" }
        ]}]}]
    });
    let json = interaction_json(2, &data);
    // The value of an unknown option type is not kept.
    let interaction: Interaction = round_trip(
        &json.to_string(),
        &["data.options[].options[].options[].value"],
    );
    assert_eq!(interaction.context, Some(InteractionContextType::Guild));
    assert_eq!(interaction.attachment_size_limit, Some(10_485_760));
    assert_eq!(
        interaction
            .authorizing_integration_owners
            .guild_install
            .and_then(djs_model::interaction::GuildInstallOwner::guild_id),
        interaction.guild_id
    );
    let guild = interaction.guild.as_ref().unwrap();
    assert_eq!(guild.features.len(), 1);
    let channel = interaction.channel.as_deref().unwrap();
    assert_eq!(channel.name.as_deref(), Some("general"));

    let command = interaction.data.command().unwrap();
    let CommandOptionValue::SubCommandGroup(group) = &command.options[0].value else {
        panic!("expected a subcommand group");
    };
    let CommandOptionValue::SubCommand(options) = &group[0].value else {
        panic!("expected a subcommand");
    };
    let values: Vec<&CommandOptionValue> = options.iter().map(|option| &option.value).collect();
    assert_eq!(
        values,
        [
            &CommandOptionValue::String("hi".into()),
            &CommandOptionValue::Integer(-5),
            &CommandOptionValue::Boolean(true),
            &CommandOptionValue::User(UserId::new(1_300_000_000_000_000_011)),
            &CommandOptionValue::Channel(ChannelId::new(1_300_000_000_000_000_012)),
            &CommandOptionValue::Role(RoleId::new(1_300_000_000_000_000_013)),
            &CommandOptionValue::Mentionable(GenericId::new(1_300_000_000_000_000_014)),
            &CommandOptionValue::Number(0.25),
            &CommandOptionValue::Attachment(AttachmentId::new(1_300_000_000_000_000_015)),
            &CommandOptionValue::Unknown(CommandOptionType::Unknown(99)),
        ]
    );
    assert!(command.focused().is_none());
}

#[test]
fn option_values_are_checked_against_the_type() {
    for (option, expected) in [
        (
            json!({ "name": "n", "type": 4, "value": "12" }),
            Ok(CommandOptionValue::Integer(12)),
        ),
        (
            json!({ "name": "n", "type": 10, "value": 3 }),
            Ok(CommandOptionValue::Number(3.0)),
        ),
        (
            json!({ "name": "n", "type": 4, "value": 1.5 }),
            Err("integer"),
        ),
        (json!({ "name": "n", "type": 3, "value": 1 }), Err("string")),
        (
            json!({ "name": "n", "type": 6, "value": "me" }),
            Err("snowflake"),
        ),
        (json!({ "name": "n", "type": 5 }), Err("value")),
        // A focused option keeps whatever was typed, even if it is not valid yet.
        (
            json!({ "name": "n", "type": 4, "value": "1x", "focused": true }),
            Ok(CommandOptionValue::Focused {
                kind: CommandOptionType::Integer,
                value: "1x".into(),
            }),
        ),
    ] {
        let decoded = serde_json::from_value::<CommandDataOption>(option.clone());
        match (decoded, expected) {
            (Ok(decoded), Ok(expected)) => assert_eq!(decoded.value, expected, "{option}"),
            (Err(error), Err(field)) => {
                assert!(error.to_string().contains(field), "{option}: {error}");
            }
            (decoded, expected) => panic!("{option}: {decoded:?} vs {expected:?}"),
        }
    }
}

#[test]
fn component_interactions() {
    let message = json!({
        "id": "1300000000000000020", "channel_id": "1300000000000000004",
        "author": { "id": "1300000000000000002", "username": "bot", "discriminator": "0",
                    "global_name": null, "avatar": null, "bot": true },
        "content": "Pick someone", "timestamp": "2026-11-11T12:00:00.000000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0,
        "flags": 0,
        "components": [{ "type": 1, "id": 1, "components": [
            { "type": 5, "id": 2, "custom_id": "pick_user" }
        ]}]
    });
    let mut json = interaction_json(
        3,
        &json!({
            "custom_id": "pick_user", "component_type": 5,
            "values": ["1300000000000000005"],
            "resolved": {
                "users": { "1300000000000000005": { "id": "1300000000000000005",
                    "username": "mixo", "discriminator": "0", "global_name": null,
                    "avatar": null } },
                "members": { "1300000000000000005": { "roles": [],
                    "joined_at": "2026-11-11T12:00:00.000000+00:00", "flags": 0,
                    "pending": false, "permissions": "8" } }
            }
        }),
    );
    json["message"] = message;
    let interaction: Interaction = round_trip(&json.to_string(), &[]);
    let InteractionData::MessageComponent(data) = &interaction.data else {
        panic!("expected a component interaction");
    };
    assert_eq!(data.component_type, ComponentType::UserSelect);
    assert_eq!(
        data.ids::<djs_model::id::marker::UserMarker>()
            .collect::<Vec<_>>(),
        [UserId::new(1_300_000_000_000_000_005)]
    );
    assert_eq!(interaction.data.custom_id(), Some("pick_user"));
    assert_eq!(
        interaction.message.as_deref().unwrap().content,
        "Pick someone"
    );

    // A button: no values, no resolved data.
    let json = interaction_json(3, &json!({ "custom_id": "click_yes", "component_type": 2 }));
    let interaction: Interaction = round_trip(&json.to_string(), &[]);
    let InteractionData::MessageComponent(data) = &interaction.data else {
        panic!("expected a component interaction");
    };
    assert!(data.values.is_empty() && data.resolved.is_none());
}

#[test]
fn modal_submit() {
    let data = json!({
        "custom_id": "feedback",
        "components": [
            { "type": 10, "id": 1 },
            { "type": 18, "id": 2, "component": { "type": 4, "id": 3, "custom_id": "text",
                                                  "value": "Great library" } },
            { "type": 18, "id": 4, "component": { "type": 3, "id": 5, "custom_id": "pick",
                                                  "values": ["a", "b"] } },
            { "type": 18, "id": 6, "component": { "type": 6, "id": 7, "custom_id": "roles",
                                                  "values": ["1300000000000000030"] } },
            { "type": 18, "id": 8, "component": { "type": 19, "id": 9, "custom_id": "files",
                                                  "values": ["1300000000000000031"] } },
            { "type": 18, "id": 10, "component": { "type": 21, "id": 11, "custom_id": "radio",
                                                   "value": null } },
            { "type": 18, "id": 12, "component": { "type": 22, "id": 13, "custom_id": "checks",
                                                   "values": [] } },
            { "type": 18, "id": 14, "component": { "type": 23, "id": 15, "custom_id": "agree",
                                                   "value": true } },
            { "type": 1, "id": 16, "components": [
                { "type": 4, "id": 17, "custom_id": "legacy", "value": "old style" }
            ]},
            { "type": 77, "id": 18 }
        ],
        "resolved": {
            "attachments": { "1300000000000000031": {
                "id": "1300000000000000031", "filename": "log.txt", "size": 12,
                "url": "https://cdn.discordapp.com/x/log.txt",
                "proxy_url": "https://media.discordapp.net/x/log.txt" } },
            "roles": { "1300000000000000030": {
                "id": "1300000000000000030", "name": "Tester", "color": 0,
                "colors": { "primary_color": 0, "secondary_color": null,
                            "tertiary_color": null },
                "hoist": false,
                "position": 1, "permissions": "0", "managed": false, "mentionable": false,
                "flags": 0 } }
        }
    });
    let json = interaction_json(5, &data);
    let interaction: Interaction = round_trip(&json.to_string(), &[]);
    let InteractionData::ModalSubmit(modal) = &interaction.data else {
        panic!("expected a modal submit");
    };
    assert_eq!(modal.text_value("text"), Some("Great library"));
    assert_eq!(modal.text_value("legacy"), Some("old style"));
    let Some(SubmittedComponent::RoleSelect(roles)) = modal.get("roles") else {
        panic!("expected a role select");
    };
    assert_eq!(roles.values, [RoleId::new(1_300_000_000_000_000_030)]);
    let Some(SubmittedComponent::FileUpload(files)) = modal.get("files") else {
        panic!("expected a file upload");
    };
    let resolved = modal.resolved.as_deref().unwrap();
    assert_eq!(resolved.attachments[&files.values[0]].filename, "log.txt");
    assert!(matches!(
        modal.get("radio"),
        Some(SubmittedComponent::RadioGroup(radio)) if radio.value.is_none()
    ));
    assert!(matches!(
        modal.get("agree"),
        Some(SubmittedComponent::Checkbox(checkbox)) if checkbox.value
    ));
    assert_eq!(
        modal.get("pick").unwrap().kind(),
        ComponentType::StringSelect
    );
    assert_eq!(
        modal.get("checks").unwrap().kind(),
        ComponentType::CheckboxGroup
    );
    let unknown = modal.components.last().unwrap();
    assert_eq!(
        (unknown.kind(), unknown.id()),
        (ComponentType::Unknown(77), Some(18))
    );
}

#[test]
fn ping_and_unknown_types() {
    let json = r#"{"id":"1","application_id":"2","type":1,"token":"t","version":1}"#;
    let interaction: Interaction = serde_json::from_str(json).unwrap();
    assert_eq!(interaction.data, InteractionData::Ping);
    let written = serde_json::to_value(&interaction).unwrap();
    assert!(written.get("data").is_none());
    assert_subset(&serde_json::from_str(json).unwrap(), &written, "$");

    let json = r#"{"id":"1","application_id":"2","type":42,"token":"t","data":{"x":1}}"#;
    let interaction: Interaction = serde_json::from_str(json).unwrap();
    assert_eq!(interaction.kind(), InteractionType::Unknown(42));
    assert_eq!(serde_json::to_value(&interaction).unwrap()["type"], 42);
}

#[test]
fn required_fields() {
    for (json, field) in [
        (r#"{"id":"1","application_id":"2","type":1}"#, "token"),
        (
            r#"{"id":"1","application_id":"2","type":2,"token":"t"}"#,
            "data",
        ),
        (
            r#"{"id":"1","application_id":"2","type":2,"token":"t","data":{"name":"x"}}"#,
            "id",
        ),
        (
            r#"{"id":"1","application_id":"2","type":3,"token":"t","data":{"component_type":2}}"#,
            "custom_id",
        ),
        (
            r#"{"id":"1","application_id":"2","type":5,"token":"t","data":{"custom_id":"m","components":[{"type":4,"custom_id":"x"}]}}"#,
            "value",
        ),
        (
            r#"{"id":"1","application_id":"2","type":5,"token":"t","data":{"custom_id":"m","components":[{"type":18}]}}"#,
            "component",
        ),
    ] {
        let error = serde_json::from_str::<Interaction>(json)
            .unwrap_err()
            .to_string();
        assert!(error.contains(field), "{json}: {error}");
    }
}

#[test]
fn message_resolved() {
    let json = json!({
        "id": "1", "channel_id": "2",
        "author": { "id": "3", "username": "bot", "discriminator": "0", "global_name": null,
                    "avatar": null, "bot": true },
        "content": "", "timestamp": "2026-11-11T12:00:00.000000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0,
        "flags": 32768,
        "resolved": { "channels": { "4": { "id": "4", "type": 0, "name": "general",
                                           "flags": 0 } } }
    });
    let message: Message = round_trip(&json.to_string(), &[]);
    let resolved = message.resolved.as_deref().unwrap();
    assert_eq!(
        resolved.channels[&ChannelId::new(4)].name.as_deref(),
        Some("general")
    );
}

/// Serializes `response`, compares with `expected` and checks that it decodes back.
fn check_response(response: &InteractionResponse, expected: &str) {
    assert_eq!(serde_json::to_string(response).unwrap(), expected);
    let decoded: InteractionResponse = serde_json::from_str(expected).unwrap();
    assert_eq!(&decoded, response, "{expected}");
}

#[test]
fn responses() {
    check_response(&InteractionResponse::Pong, r#"{"type":1}"#);
    check_response(
        &InteractionResponse::DeferredChannelMessageWithSource(DeferredCallbackData::default()),
        r#"{"type":5}"#,
    );
    check_response(
        &InteractionResponse::DeferredChannelMessageWithSource(DeferredCallbackData::new(
            MessageFlags::EPHEMERAL,
        )),
        r#"{"type":5,"data":{"flags":64}}"#,
    );
    check_response(&InteractionResponse::DeferredUpdateMessage, r#"{"type":6}"#);
    check_response(&InteractionResponse::PremiumRequired, r#"{"type":10}"#);
    check_response(&InteractionResponse::LaunchActivity, r#"{"type":12}"#);

    // A message with an uploaded file and a poll.
    let mut reply = MessageCallbackData::from("Vote!");
    reply.attachments = Some(vec![AttachmentRequest::upload(0, "chart.png")]);
    let mut poll = CreatePoll::new("Best crab?", [Media::new("Ferris"), Media::new("Corro")]);
    poll.allow_multiselect = Some(true);
    reply.poll = Some(poll);
    check_response(
        &InteractionResponse::ChannelMessageWithSource(Box::new(reply)),
        concat!(
            r#"{"type":4,"data":{"content":"Vote!","attachments":[{"id":0,"filename":"chart.png"}],"#,
            r#""poll":{"question":{"text":"Best crab?"},"answers":[{"poll_media":{"text":"Ferris"}},"#,
            r#"{"poll_media":{"text":"Corro"}}],"allow_multiselect":true}}}"#,
        ),
    );

    // An update that removes all components and keeps everything else.
    let mut update = MessageCallbackData::default();
    update.components = Some(Vec::new());
    check_response(
        &InteractionResponse::UpdateMessage(Box::new(update)),
        r#"{"type":7,"data":{"components":[]}}"#,
    );

    check_response(
        &InteractionResponse::Modal(Box::new(ModalCallbackData::new(
            "m",
            "Title",
            vec![TextInput::new("name", TextInputStyle::Short).into()],
        ))),
        r#"{"type":9,"data":{"custom_id":"m","title":"Title","components":[{"type":4,"custom_id":"name","style":1,"required":true}]}}"#,
    );
}

#[test]
fn autocomplete_responses() {
    for (choices, expected) in [
        (
            AutocompleteChoices::String(vec![Choice::new("Gitrog", "gitrog".into())]),
            r#"{"type":8,"data":{"choices":[{"name":"Gitrog","value":"gitrog"}]}}"#,
        ),
        (
            AutocompleteChoices::Integer(vec![Choice::new("One", 1), Choice::new("Two", 2)]),
            r#"{"type":8,"data":{"choices":[{"name":"One","value":1},{"name":"Two","value":2}]}}"#,
        ),
        (
            AutocompleteChoices::Number(vec![Choice::new("Half", 0.5), Choice::new("One", 1.0)]),
            r#"{"type":8,"data":{"choices":[{"name":"Half","value":0.5},{"name":"One","value":1.0}]}}"#,
        ),
        (
            AutocompleteChoices::String(Vec::new()),
            r#"{"type":8,"data":{"choices":[]}}"#,
        ),
    ] {
        check_response(
            &InteractionResponse::AutocompleteResult(AutocompleteCallbackData::new(choices)),
            expected,
        );
    }
    // Integers mixed with fractional numbers are a number list.
    let decoded: InteractionResponse = serde_json::from_str(
        r#"{"type":8,"data":{"choices":[{"name":"One","value":1},{"name":"Half","value":0.5}]}}"#,
    )
    .unwrap();
    let InteractionResponse::AutocompleteResult(data) = decoded else {
        panic!("expected an autocomplete result");
    };
    assert!(matches!(data.choices, AutocompleteChoices::Number(_)));
    // Strings mixed with numbers are invalid.
    assert!(
        serde_json::from_str::<InteractionResponse>(
            r#"{"type":8,"data":{"choices":[{"name":"a","value":"a"},{"name":"b","value":2}]}}"#,
        )
        .is_err()
    );
}

#[test]
fn unknown_responses_and_attachment_ids() {
    let decoded: InteractionResponse = serde_json::from_str(r#"{"type":99,"data":{}}"#).unwrap();
    assert_eq!(
        decoded,
        InteractionResponse::Unknown(InteractionCallbackType::Unknown(99))
    );
    assert_eq!(serde_json::to_string(&decoded).unwrap(), r#"{"type":99}"#);

    for (json, id) in [
        (r#"{"id":3}"#, AttachmentRequestId::Upload(3)),
        (
            r#"{"id":"1300000000000000031"}"#,
            AttachmentRequestId::Existing(AttachmentId::new(1_300_000_000_000_000_031)),
        ),
        (
            r#"{"id":1300000000000000031}"#,
            AttachmentRequestId::Existing(AttachmentId::new(1_300_000_000_000_000_031)),
        ),
    ] {
        let attachment: AttachmentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(attachment.id, id, "{json}");
    }
    assert_eq!(
        serde_json::to_string(&AttachmentRequest::existing(AttachmentId::new(5))).unwrap(),
        r#"{"id":"5"}"#
    );
}

#[test]
fn callback_response() {
    let json = json!({
        "interaction": {
            "id": "1300000000000000001", "type": 2,
            "response_message_id": "1300000000000000040",
            "response_message_loading": false, "response_message_ephemeral": true
        },
        "resource": {
            "type": 4,
            "message": {
                "id": "1300000000000000040", "channel_id": "1300000000000000004",
                "author": { "id": "1300000000000000002", "username": "bot",
                            "discriminator": "0", "global_name": null, "avatar": null,
                            "bot": true },
                "content": "Hello", "timestamp": "2026-11-11T12:00:00.000000+00:00",
                "edited_timestamp": null, "tts": false, "mention_everyone": false,
                "mentions": [], "mention_roles": [], "attachments": [], "embeds": [],
                "pinned": false, "type": 20, "flags": 64
            }
        }
    });
    let response: InteractionCallbackResponse = round_trip(&json.to_string(), &[]);
    assert_eq!(
        response.interaction.kind,
        InteractionType::ApplicationCommand
    );
    assert_eq!(response.interaction.response_message_ephemeral, Some(true));
    let resource = response.resource.unwrap();
    assert_eq!(
        resource.kind,
        InteractionCallbackType::ChannelMessageWithSource
    );
    assert_eq!(resource.message.unwrap().content, "Hello");

    let json = json!({
        "interaction": { "id": "1", "type": 2, "activity_instance_id": "i-123" },
        "resource": { "type": 12, "activity_instance": { "id": "i-123" } }
    });
    let response: InteractionCallbackResponse = round_trip(&json.to_string(), &[]);
    assert_eq!(
        response.resource.unwrap().activity_instance.unwrap().id,
        "i-123"
    );
}
