//! Components against every message and modal example of the components reference
//! (`fixtures/docs/component_reference_examples.json`, extracted from the docs' JSONC examples
//! with comments removed), plus edge cases.

use std::collections::BTreeSet;

use djs_model::{
    channel::ChannelType,
    component::{
        ActionRow, Button, ButtonStyle, Component, ComponentType, Container, InteractiveButton,
        LinkButton, SelectDefaultValueType, Separator, SeparatorSpacing, TextDisplay,
        TextInputStyle, Thumbnail, UnfurledMediaItem,
    },
    message::{Message, MessageFlags},
};
use serde_json::Value;

mod common;

use common::{assert_subset, round_trip};

fn examples() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "fixtures/docs/component_reference_examples.json"
    ))
    .unwrap()
}

fn collect_kinds(components: &[Component], kinds: &mut BTreeSet<u8>) {
    for component in components {
        kinds.insert(component.kind().into());
        collect_kinds(component.children(), kinds);
        match component {
            Component::Section(section) => {
                collect_kinds(std::slice::from_ref(&*section.accessory), kinds);
            }
            Component::Label(label) => {
                collect_kinds(std::slice::from_ref(&*label.component), kinds);
            }
            _ => {}
        }
    }
}

#[test]
fn every_docs_example_round_trips() {
    let mut kinds = BTreeSet::new();
    for example in examples() {
        let name = format!("{} / {}", example["section"], example["title"]);
        let components: Vec<Component> = serde_json::from_value(example["components"].clone())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let written = serde_json::to_value(&components).unwrap();
        assert_subset(&example["components"], &written, &name);
        collect_kinds(&components, &mut kinds);
        assert!(
            !components
                .iter()
                .any(|component| matches!(component, Component::Unknown(_))),
            "{name}"
        );
    }
    // Every documented component type appears in the examples.
    let documented: BTreeSet<u8> = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 17, 18, 19, 21, 22, 23,
    ]
    .into_iter()
    .collect();
    let missing: Vec<_> = documented.difference(&kinds).collect();
    assert!(missing.is_empty(), "types without an example: {missing:?}");
}

fn example(section: &str, title: &str) -> Vec<Component> {
    let example = examples()
        .into_iter()
        .find(|example| example["section"] == section && example["title"] == title)
        .unwrap_or_else(|| panic!("no example {section} / {title}"));
    serde_json::from_value(example["components"].clone()).unwrap()
}

#[test]
fn buttons() {
    let components = example("Action Row", "Message Example");
    let [Component::ActionRow(row)] = components.as_slice() else {
        panic!("expected one action row");
    };
    let [
        Component::Button(accept),
        Component::Button(learn),
        Component::Button(decline),
    ] = row.components.as_slice()
    else {
        panic!("expected three buttons");
    };
    let Button::Interactive(accept) = accept else {
        panic!("expected an interactive button");
    };
    assert_eq!(accept.style, ButtonStyle::Primary);
    assert_eq!(accept.custom_id, "click_yes");
    let Button::Link(learn) = learn else {
        panic!("expected a link button");
    };
    assert_eq!(learn.url, "http://watchanimeattheoffice.com/");
    assert_eq!(decline.style(), ButtonStyle::Danger);
    assert_eq!(decline.custom_id(), Some("click_no"));
    assert!(!decline.disabled());
}

#[test]
fn selects_and_modal_inputs() {
    let components = example("Channel Select", "Message Example");
    let Component::ActionRow(row) = &components[0] else {
        panic!("expected an action row");
    };
    let Component::ChannelSelect(select) = &row.components[0] else {
        panic!("expected a channel select");
    };
    assert!(select.channel_types.contains(&ChannelType::GuildText));

    let components = example("User Select", "Modal Example");
    let Component::Label(label) = &components[0] else {
        panic!("expected a label");
    };
    let Component::UserSelect(select) = &*label.component else {
        panic!("expected a user select");
    };
    assert!(
        select
            .default_values
            .iter()
            .all(|value| value.kind == SelectDefaultValueType::User)
    );

    let components = example("Text Input", "Modal Example");
    let text_input = components
        .iter()
        .find_map(|component| match component {
            Component::Label(label) => match &*label.component {
                Component::TextInput(input) => Some(input.clone()),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        text_input.style,
        TextInputStyle::Short | TextInputStyle::Paragraph
    ));
}

#[test]
fn layout_and_content() {
    let components = example("Container", "Message Example");
    let Component::Container(container) = &components[0] else {
        panic!("expected a container");
    };
    assert!(container.accent_color.is_some());
    assert_eq!(container.components.len(), 4);

    let components = example("Section", "Message Example");
    let Component::Section(section) = &components[0] else {
        panic!("expected a section");
    };
    assert!(matches!(
        *section.accessory,
        Component::Thumbnail(_) | Component::Button(_)
    ));

    let components = example("Separator", "Message Example");
    let separator = components
        .iter()
        .find_map(|component| match component {
            Component::Separator(separator) => Some(separator),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        (separator.divider, separator.spacing),
        (true, Some(SeparatorSpacing::Small))
    );
}

#[test]
fn defaults_and_unknown_values() {
    // `required` and `divider` are `true` when absent (and always written, like D-055).
    let component: Component =
        serde_json::from_str(r#"{"type":4,"custom_id":"name","style":1}"#).unwrap();
    let Component::TextInput(input) = &component else {
        panic!("expected a text input");
    };
    assert!(input.required);
    assert_eq!(
        serde_json::to_string(&component).unwrap(),
        r#"{"type":4,"custom_id":"name","style":1,"required":true}"#
    );
    let component: Component = serde_json::from_str(r#"{"type":14,"divider":false}"#).unwrap();
    assert_eq!(
        serde_json::to_string(&component).unwrap(),
        r#"{"type":14,"divider":false}"#
    );

    // Unknown component types keep their type and id.
    let component: Component = serde_json::from_str(r#"{"type":99,"id":7,"x":1}"#).unwrap();
    assert_eq!(component.kind(), ComponentType::Unknown(99));
    assert_eq!(component.id(), Some(7));
    assert_eq!(
        serde_json::to_string(&component).unwrap(),
        r#"{"type":99,"id":7}"#
    );
    let component: Component = serde_json::from_str(r#"{"type":16}"#).unwrap();
    assert!(matches!(component, Component::Unknown(_)));

    // Unknown button styles keep every button field.
    let component: Component =
        serde_json::from_str(r#"{"type":2,"style":9,"custom_id":"new","label":"New"}"#).unwrap();
    let Component::Button(Button::Unknown(button)) = &component else {
        panic!("expected an unknown button");
    };
    assert_eq!(button.style, ButtonStyle::Unknown(9));
    assert_eq!(button.custom_id.as_deref(), Some("new"));
    assert_eq!(component.custom_id(), Some("new"));
    let written = serde_json::to_value(&component).unwrap();
    assert_eq!(written["style"], 9);
    assert_eq!(written["label"], "New");
}

#[test]
fn required_fields() {
    for (json, field) in [
        (r#"{"type":2,"style":1,"label":"x"}"#, "custom_id"),
        (r#"{"type":2,"style":5,"label":"x"}"#, "url"),
        (r#"{"type":2,"style":6}"#, "sku_id"),
        (r#"{"type":2,"custom_id":"x"}"#, "style"),
        (r#"{"type":9,"components":[]}"#, "accessory"),
        (r#"{"type":18,"label":"x"}"#, "component"),
        (r#"{"type":10}"#, "content"),
        (r#"{"type":11}"#, "media"),
        (r#"{"type":13}"#, "file"),
        (r#"{"type":3,"options":[]}"#, "custom_id"),
    ] {
        let error = serde_json::from_str::<Component>(json)
            .unwrap_err()
            .to_string();
        assert!(error.contains(field), "{json}: {error}");
    }
}

#[test]
fn building_components() {
    let mut confirm = InteractiveButton::new(ButtonStyle::Success, "confirm");
    confirm.label = Some("Confirm".into());
    let mut docs = LinkButton::new("https://docs.rs/djs-rs");
    docs.label = Some("Docs".into());
    let mut thumbnail = Thumbnail::new(UnfurledMediaItem::new("attachment://ferris.png"));
    thumbnail.description = Some("Ferris".into());
    let mut container = Container::new(vec![
        TextDisplay::new("**Hello**").into(),
        Separator::new().into(),
        ActionRow::new(vec![confirm.into(), docs.into()]).into(),
    ]);
    container.accent_color = Some(0xF7_4C_00);
    let components = vec![
        Component::from(container),
        djs_model::component::Section::new(
            vec![TextDisplay::new("Ferris").into()],
            thumbnail.into(),
        )
        .into(),
    ];
    assert_eq!(
        serde_json::to_string(&components).unwrap(),
        concat!(
            r#"[{"type":17,"components":[{"type":10,"content":"**Hello**"},{"type":14,"divider":true},"#,
            r#"{"type":1,"components":[{"type":2,"style":3,"custom_id":"confirm","label":"Confirm"},"#,
            r#"{"type":2,"style":5,"url":"https://docs.rs/djs-rs","label":"Docs"}]}],"#,
            r#""accent_color":16206848},"#,
            r#"{"type":9,"components":[{"type":10,"content":"Ferris"}],"#,
            r#""accessory":{"type":11,"media":{"url":"attachment://ferris.png"},"description":"Ferris"}}]"#,
        )
    );
    // What is written can be read back.
    let json = serde_json::to_string(&components).unwrap();
    let again: Vec<Component> = serde_json::from_str(&json).unwrap();
    assert_eq!(again, components);
}

#[test]
fn message_components() {
    let example = examples()
        .into_iter()
        .find(|example| example["section"] == "Container")
        .unwrap();
    let json = serde_json::json!({
        "id": "1", "channel_id": "2",
        "author": { "id": "3", "username": "bot", "discriminator": "0", "global_name": null,
                    "avatar": null, "bot": true },
        "content": "", "timestamp": "2026-11-11T12:00:00.000000+00:00",
        "edited_timestamp": null, "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0,
        "flags": example["flags"], "components": example["components"]
    });
    let message: Message = round_trip(&json.to_string(), &[]);
    assert!(message.flags.contains(MessageFlags::IS_COMPONENTS_V2));
    assert_eq!(message.components[0].kind(), ComponentType::Container);
}
