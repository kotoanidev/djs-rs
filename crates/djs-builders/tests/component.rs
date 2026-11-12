//! The message component builders (buttons, select menus, action rows) and
//! `validate_components`. Cases marked "live" reproduce what Discord accepted or rejected on
//! the live API (D-200, D-201).

use djs_builders::{
    ActionRowBuilder, ButtonBuilder, ChannelSelectMenuBuilder, ComponentsVersion,
    MentionableSelectMenuBuilder, RoleSelectMenuBuilder, StringSelectMenuBuilder,
    StringSelectMenuOptionBuilder, UserSelectMenuBuilder, ValidationError, validate_components,
};
use djs_model::{
    channel::ChannelType,
    component::{Component, Container, Section, SelectOption, TextDisplay},
    emoji::PartialEmoji,
    id::{ChannelId, RoleId, SkuId, UserId},
};
use serde_json::json;

fn fire() -> PartialEmoji {
    PartialEmoji::Unicode {
        name: "🔥".into()
    }
}

fn row(component: impl Into<Component>) -> Result<Component, ValidationError> {
    ActionRowBuilder::new().component(component).build()
}

fn invalid(error: &ValidationError) -> (&str, &'static str) {
    match error {
        ValidationError::Invalid { field, reason } => (field, reason),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn buttons_end_up_in_the_json() {
    let row = ActionRowBuilder::new()
        .id(1)
        .component(ButtonBuilder::primary("a").label("A"))
        .component(ButtonBuilder::secondary("b").emoji(fire()).disabled(true))
        .component(ButtonBuilder::success("c").label("C").emoji(fire()).id(7))
        .component(ButtonBuilder::link("https://example.com").label("Site"))
        .component(ButtonBuilder::premium(SkuId::new(5)).disabled(true))
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&row).unwrap(),
        json!({"type": 1, "id": 1, "components": [
            {"type": 2, "style": 1, "custom_id": "a", "label": "A"},
            {"type": 2, "style": 2, "custom_id": "b", "emoji": {"id": null, "name": "🔥"}, "disabled": true},
            {"type": 2, "id": 7, "style": 3, "custom_id": "c", "label": "C", "emoji": {"id": null, "name": "🔥"}},
            {"type": 2, "style": 5, "url": "https://example.com", "label": "Site"},
            {"type": 2, "style": 6, "sku_id": "5", "disabled": true}
        ]})
    );
    let danger = ButtonBuilder::danger("d").label("D").build().unwrap();
    assert_eq!(
        serde_json::to_value(&danger).unwrap(),
        json!({"type": 2, "style": 4, "custom_id": "d", "label": "D"})
    );
}

#[test]
fn live_custom_ids_count_code_points_without_trimming() {
    assert!(
        ButtonBuilder::primary("🔥".repeat(100))
            .label("x")
            .build()
            .is_ok()
    );
    assert!(ButtonBuilder::primary("   ").label("x").build().is_ok());
    assert_eq!(
        ButtonBuilder::primary("a".repeat(101)).label("x").build(),
        Err(ValidationError::TooLong {
            field: "custom_id".into(),
            max: 100,
            actual: 101
        })
    );
    assert_eq!(
        ButtonBuilder::primary("").label("x").build(),
        Err(ValidationError::Missing {
            field: "custom_id".into()
        })
    );
}

#[test]
fn live_a_button_needs_a_label_or_an_emoji() {
    assert!(
        ButtonBuilder::primary("a")
            .label("🔥".repeat(80))
            .build()
            .is_ok()
    );
    assert!(ButtonBuilder::primary("a").emoji(fire()).build().is_ok());
    assert_eq!(
        ButtonBuilder::primary("a").label("🔥".repeat(81)).build(),
        Err(ValidationError::TooLong {
            field: "label".into(),
            max: 80,
            actual: 81
        })
    );
    for button in [
        ButtonBuilder::primary("a"),
        ButtonBuilder::primary("a").label("   "),
        ButtonBuilder::link("https://example.com"),
    ] {
        assert_eq!(
            button.build(),
            Err(ValidationError::Missing {
                field: "label".into()
            })
        );
    }
}

#[test]
fn live_links_allow_http_https_and_discord() {
    for url in [
        "http://example.com",
        "HTTPS://example.com",
        "discord://-/channels/@me",
    ] {
        assert!(ButtonBuilder::link(url).label("x").build().is_ok(), "{url}");
    }
    let error = ButtonBuilder::link("ftp://example.com")
        .label("x")
        .build()
        .unwrap_err();
    assert_eq!(
        error,
        ValidationError::InvalidUrl {
            field: "url".into(),
            allowed: &["http://", "https://", "discord://"]
        }
    );
    let long = format!("https://example.com/{}", "a".repeat(493));
    assert_eq!(
        ButtonBuilder::link(long).label("x").build(),
        Err(ValidationError::TooLong {
            field: "url".into(),
            max: 512,
            actual: 513
        })
    );
}

#[test]
fn live_a_row_holds_one_to_five_buttons_or_one_menu() {
    let buttons = |count: usize| {
        (0..count).fold(ActionRowBuilder::new(), |row, index| {
            row.component(ButtonBuilder::primary(index.to_string()).label("x"))
        })
    };
    assert!(buttons(5).build().is_ok());
    assert_eq!(
        buttons(0).build(),
        Err(ValidationError::TooFew {
            field: "components".into(),
            min: 1,
            actual: 0
        })
    );
    assert_eq!(
        buttons(6).build(),
        Err(ValidationError::TooMany {
            field: "components".into(),
            max: 5,
            actual: 6
        })
    );
    let menu = || StringSelectMenuBuilder::new("menu").option("a", "a");
    let error = buttons(1).component(menu()).build().unwrap_err();
    assert_eq!(
        invalid(&error),
        ("components[1]", "a select menu fills a whole action row")
    );
    let error = ActionRowBuilder::new()
        .component(menu())
        .component(ButtonBuilder::primary("b").label("x"))
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "components[0]");
    let error = row(TextDisplay::new("text")).unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[0]",
            "an action row can only hold buttons or one select menu"
        )
    );
}

#[test]
fn live_custom_ids_are_unique_within_a_row() {
    let error = ActionRowBuilder::new()
        .component(ButtonBuilder::primary("same").label("1"))
        .component(ButtonBuilder::primary("same").label("2"))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[1].custom_id",
            "another component has the same custom_id"
        )
    );
}

#[test]
fn problems_of_a_child_name_its_place_in_the_row() {
    let error = ActionRowBuilder::new()
        .component(ButtonBuilder::primary("a").label("ok"))
        .component(ButtonBuilder::primary("b"))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "components[1].label");
}

#[test]
fn a_string_select_ends_up_in_the_json() {
    let menu = StringSelectMenuBuilder::new("color")
        .placeholder("Pick")
        .min_values(0)
        .max_values(2)
        .option("Red", "red")
        .option_with(
            StringSelectMenuOptionBuilder::new("Blue", "blue")
                .description("Sky")
                .emoji(fire())
                .default(true),
        )
        .options([SelectOption::new("Green", "green")])
        .id(3)
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&menu).unwrap(),
        json!({"type": 3, "id": 3, "custom_id": "color", "placeholder": "Pick", "min_values": 0,
               "max_values": 2, "required": true, "options": [
            {"label": "Red", "value": "red"},
            {"label": "Blue", "value": "blue", "description": "Sky", "emoji": {"id": null, "name": "🔥"}, "default": true},
            {"label": "Green", "value": "green"}
        ]})
    );
}

#[test]
fn live_select_values_and_options_are_consistent() {
    let three = || {
        StringSelectMenuBuilder::new("s")
            .option("a", "a")
            .option("b", "b")
            .option("c", "c")
    };
    assert!(three().min_values(2).max_values(3).build().is_ok());
    assert_eq!(
        three().min_values(3).max_values(2).build(),
        Err(ValidationError::OutOfRange {
            field: "min_values".into(),
            min: 0,
            max: 2,
            actual: 3
        })
    );
    assert_eq!(
        three().max_values(5).build(),
        Err(ValidationError::TooFew {
            field: "options".into(),
            min: 5,
            actual: 3
        })
    );
    assert_eq!(
        StringSelectMenuBuilder::new("s").build(),
        Err(ValidationError::TooFew {
            field: "options".into(),
            min: 1,
            actual: 0
        })
    );
    assert!(matches!(
        three().min_values(26).build(),
        Err(ValidationError::OutOfRange { .. })
    ));
    assert!(matches!(
        three().max_values(0).build(),
        Err(ValidationError::OutOfRange { .. })
    ));
    let many = (0..26).fold(StringSelectMenuBuilder::new("s"), |menu, index| {
        menu.option(index.to_string(), index.to_string())
    });
    assert_eq!(
        many.build(),
        Err(ValidationError::TooMany {
            field: "options".into(),
            max: 25,
            actual: 26
        })
    );
}

#[test]
fn live_select_options_need_labels_and_unique_values() {
    assert_eq!(
        StringSelectMenuBuilder::new("s").option("   ", "v").build(),
        Err(ValidationError::Missing {
            field: "options[0].label".into()
        })
    );
    let error = StringSelectMenuBuilder::new("s")
        .option("a", "v")
        .option("b", "v")
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[1].value",
            "another option of the menu has the same value"
        )
    );
    let error = StringSelectMenuBuilder::new("s")
        .option_with(StringSelectMenuOptionBuilder::new("a", "v").description("d".repeat(101)))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "options[0].description");
    let error = StringSelectMenuBuilder::new("s")
        .option("a", "v")
        .placeholder("p".repeat(151))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "placeholder");
}

#[test]
fn auto_populated_selects_end_up_in_the_json() {
    let users = UserSelectMenuBuilder::new("users")
        .max_values(2)
        .default_users([UserId::new(1), UserId::new(2)])
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&users).unwrap(),
        json!({"type": 5, "custom_id": "users", "max_values": 2, "required": true, "default_values": [
            {"id": "1", "type": "user"}, {"id": "2", "type": "user"}
        ]})
    );
    let roles = RoleSelectMenuBuilder::new("roles")
        .default_roles([RoleId::new(3)])
        .build()
        .unwrap();
    assert_eq!(
        roles.kind(),
        djs_model::component::ComponentType::RoleSelect
    );
    let mentionables = MentionableSelectMenuBuilder::new("m")
        .max_values(2)
        .default_users([UserId::new(1)])
        .default_roles([RoleId::new(3)])
        .build();
    assert!(mentionables.is_ok());
    let channels = ChannelSelectMenuBuilder::new("c")
        .channel_types([ChannelType::GuildText])
        .default_channels([ChannelId::new(4)])
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&channels).unwrap(),
        json!({"type": 8, "custom_id": "c", "channel_types": [0], "required": true,
               "default_values": [{"id": "4", "type": "channel"}]})
    );
}

#[test]
fn default_values_fit_min_and_max_values() {
    assert_eq!(
        UserSelectMenuBuilder::new("u")
            .default_users([UserId::new(1), UserId::new(2)])
            .build(),
        Err(ValidationError::TooMany {
            field: "default_values".into(),
            max: 1,
            actual: 2
        })
    );
    assert_eq!(
        RoleSelectMenuBuilder::new("r")
            .min_values(2)
            .max_values(3)
            .default_roles([RoleId::new(1)])
            .build(),
        Err(ValidationError::TooFew {
            field: "default_values".into(),
            min: 2,
            actual: 1
        })
    );
    // A user select with a role as default value can only come from a raw model value.
    let raw: Component = serde_json::from_value(json!({
        "type": 5, "custom_id": "u", "default_values": [{"id": "1", "type": "role"}]
    }))
    .unwrap();
    let error = row(raw).unwrap_err();
    assert_eq!(invalid(&error).0, "components[0].default_values[0].type");
}

#[test]
fn legacy_messages_have_up_to_five_rows() {
    let button_row = |id: &str| row(ButtonBuilder::primary(id).label("x")).unwrap();
    let rows: Vec<Component> = (0..5).map(|index| button_row(&index.to_string())).collect();
    assert!(validate_components(&rows, ComponentsVersion::Legacy).is_ok());
    let mut six = rows.clone();
    six.push(button_row("5"));
    assert_eq!(
        validate_components(&six, ComponentsVersion::Legacy),
        Err(ValidationError::TooMany {
            field: "components".into(),
            max: 5,
            actual: 6
        })
    );
    let error = validate_components(&[TextDisplay::new("hi").into()], ComponentsVersion::Legacy)
        .unwrap_err();
    assert_eq!(invalid(&error).0, "components[0]");
    // Each row is checked too.
    let error = validate_components(
        &[ActionRowBuilder::new().build_unchecked()],
        ComponentsVersion::Legacy,
    )
    .unwrap_err();
    assert_eq!(error.field(), "components[0].components");
}

#[test]
fn live_custom_ids_are_unique_in_the_whole_message() {
    let button_row = |id: &str| row(ButtonBuilder::primary(id).label("x")).unwrap();
    let error = validate_components(
        &[button_row("a"), button_row("a")],
        ComponentsVersion::Legacy,
    )
    .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[1].components[0].custom_id",
            "another component of the message has the same custom_id"
        )
    );
    // A section accessory counts too.
    let section = Section::new(
        vec![TextDisplay::new("text").into()],
        ButtonBuilder::primary("a").label("x").build().unwrap(),
    );
    let error =
        validate_components(&[button_row("a"), section.into()], ComponentsVersion::V2).unwrap_err();
    assert_eq!(error.field(), "components[1].accessory.custom_id");
}

#[test]
fn live_v2_messages_have_up_to_40_components_counting_nested_ones() {
    let container = |texts: usize| -> Component {
        Container::new(
            (0..texts)
                .map(|index| TextDisplay::new(format!("t{index}")).into())
                .collect(),
        )
        .into()
    };
    assert!(validate_components(&[container(39)], ComponentsVersion::V2).is_ok());
    assert_eq!(
        validate_components(&[container(40)], ComponentsVersion::V2),
        Err(ValidationError::TooMany {
            field: "components".into(),
            max: 40,
            actual: 41
        })
    );
    let error = validate_components(
        &[ButtonBuilder::primary("a").label("x").build().unwrap()],
        ComponentsVersion::V2,
    )
    .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[0]",
            "this component cannot be at the top level of a message"
        )
    );
}

#[test]
fn live_v2_text_is_at_most_4000_code_points_in_all() {
    let text = |content: String| -> Component { TextDisplay::new(content).into() };
    assert!(
        validate_components(
            &[text("🔥".repeat(2001)), text("a".repeat(1999))],
            ComponentsVersion::V2
        )
        .is_ok()
    );
    assert_eq!(
        validate_components(
            &[text("a".repeat(2000)), text("a".repeat(2001))],
            ComponentsVersion::V2
        ),
        Err(ValidationError::TooLong {
            field: "components".into(),
            max: 4000,
            actual: 4001
        })
    );
}
