//! The modal builders. The rules come from Discord's documentation only: a modal cannot be
//! sent without a user's interaction, so none of them was tried on the live API (D-200).

use djs_builders::{
    ButtonBuilder, CheckboxBuilder, CheckboxGroupBuilder, CheckboxGroupOptionBuilder,
    FileUploadBuilder, LabelBuilder, ModalBuilder, RadioGroupBuilder, RadioGroupOptionBuilder,
    StringSelectMenuBuilder, TextInputBuilder, UserSelectMenuBuilder, ValidationError,
};
use djs_model::component::{ActionRow, Component, TextInput, TextInputStyle};
use serde_json::json;

fn invalid(error: &ValidationError) -> (&str, &'static str) {
    match error {
        ValidationError::Invalid { field, reason } => (field, reason),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

/// A modal with one label around `field`.
fn with(field: impl Into<Component>) -> Result<(), ValidationError> {
    ModalBuilder::new("modal", "Title")
        .label("Field", field)
        .build()
        .map(drop)
}

fn out_of_range(field: &str, min: i64, max: i64, actual: i64) -> Result<(), ValidationError> {
    Err(ValidationError::OutOfRange {
        field: format!("components[0].component.{field}").into(),
        min,
        max,
        actual,
    })
}

#[test]
fn a_form_ends_up_in_the_json() {
    let modal = ModalBuilder::new("feedback", "Your feedback")
        .text("Thanks for helping!")
        .label(
            "Topic",
            TextInputBuilder::short("topic")
                .min_length(2)
                .max_length(80)
                .placeholder("What is it about?")
                .value("Bug")
                .id(7),
        )
        .component(
            LabelBuilder::new(
                "Details",
                TextInputBuilder::paragraph("body").required(false),
            )
            .description("Optional")
            .id(3),
        )
        .label(
            "Rating",
            RadioGroupBuilder::new("rating")
                .option("Good", "good")
                .option_with(
                    RadioGroupOptionBuilder::new("Bad", "bad")
                        .description("Tell us why")
                        .default(true),
                ),
        )
        .label(
            "Screenshots",
            FileUploadBuilder::new("shots")
                .min_values(0)
                .max_values(3)
                .required(false)
                .file_types(["image", ".pdf"]),
        )
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&modal).unwrap(),
        json!({"custom_id": "feedback", "title": "Your feedback", "components": [
            {"type": 10, "content": "Thanks for helping!"},
            {"type": 18, "label": "Topic", "component": {
                "type": 4, "id": 7, "custom_id": "topic", "style": 1, "min_length": 2,
                "max_length": 80, "required": true, "value": "Bug",
                "placeholder": "What is it about?"
            }},
            {"type": 18, "id": 3, "label": "Details", "description": "Optional", "component": {
                "type": 4, "custom_id": "body", "style": 2, "required": false
            }},
            {"type": 18, "label": "Rating", "component": {
                "type": 21, "custom_id": "rating", "required": true, "options": [
                    {"value": "good", "label": "Good"},
                    {"value": "bad", "label": "Bad", "description": "Tell us why", "default": true}
                ]
            }},
            {"type": 18, "label": "Screenshots", "component": {
                "type": 19, "custom_id": "shots", "min_values": 0, "max_values": 3,
                "required": false, "file_types": ["image", ".pdf"]
            }}
        ]})
    );
}

#[test]
fn checkboxes_end_up_in_the_json() {
    let modal = ModalBuilder::new("signup", "Sign up")
        .label(
            "Topics",
            CheckboxGroupBuilder::new("topics")
                .option("News", "news")
                .options([CheckboxGroupOptionBuilder::new("Events", "events").default(true)])
                .min_values(1)
                .max_values(2),
        )
        .label(
            "Newsletter",
            CheckboxBuilder::new("newsletter").default(true),
        )
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&modal.components).unwrap(),
        json!([
            {"type": 18, "label": "Topics", "component": {
                "type": 22, "custom_id": "topics", "options": [
                    {"value": "news", "label": "News"},
                    {"value": "events", "label": "Events", "default": true}
                ], "min_values": 1, "max_values": 2, "required": true
            }},
            {"type": 18, "label": "Newsletter", "component": {
                "type": 23, "custom_id": "newsletter", "default": true
            }}
        ])
    );
}

#[test]
fn a_modal_has_a_title_and_1_to_5_labels_or_texts() {
    let texts = |count: usize| {
        (0..count).fold(ModalBuilder::new("modal", "Title"), |modal, index| {
            modal.text(index.to_string())
        })
    };
    assert!(texts(5).build().is_ok());
    assert_eq!(
        texts(0).build(),
        Err(ValidationError::TooFew {
            field: "components".into(),
            min: 1,
            actual: 0
        })
    );
    assert_eq!(
        texts(6).build(),
        Err(ValidationError::TooMany {
            field: "components".into(),
            max: 5,
            actual: 6
        })
    );
    assert_eq!(
        ModalBuilder::new("modal", " ").text("t").build(),
        Err(ValidationError::Missing {
            field: "title".into()
        })
    );
    assert_eq!(
        ModalBuilder::new("modal", "🔥".repeat(46))
            .text("t")
            .build(),
        Err(ValidationError::TooLong {
            field: "title".into(),
            max: 45,
            actual: 46
        })
    );
    assert!(
        ModalBuilder::new("modal", "🔥".repeat(45))
            .text("t")
            .build()
            .is_ok()
    );
    assert_eq!(
        ModalBuilder::new("", "Title").text("t").build(),
        Err(ValidationError::Missing {
            field: "custom_id".into()
        })
    );
    let error = ModalBuilder::new("modal", "Title")
        .component(ButtonBuilder::primary("b").label("x"))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[0]",
            "a modal can only hold labels and text displays"
        )
    );
    let error = ModalBuilder::new("modal", "Title")
        .text("")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "components[0].content");
}

#[test]
fn rows_of_text_inputs_are_rejected_but_can_be_sent_unchecked() {
    // The deprecated format (D-207): an action row around a text input with its own label.
    let row: Component =
        ActionRow::new(vec![TextInput::new("old", TextInputStyle::Short).into()]).into();
    let error = ModalBuilder::new("modal", "Title")
        .component(row.clone())
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "components[0]");
    let modal = ModalBuilder::new("modal", "Title")
        .component(row.clone())
        .build_unchecked();
    assert_eq!(modal.components, [row]);
}

#[test]
fn a_label_has_a_text_a_description_and_one_field() {
    let field = || TextInputBuilder::short("f");
    assert_eq!(
        ModalBuilder::new("modal", "Title")
            .label("  ", field())
            .build(),
        Err(ValidationError::Missing {
            field: "components[0].label".into()
        })
    );
    assert_eq!(
        LabelBuilder::new("a".repeat(46), field()).build(),
        Err(ValidationError::TooLong {
            field: "label".into(),
            max: 45,
            actual: 46
        })
    );
    assert_eq!(
        LabelBuilder::new("Field", field())
            .description("a".repeat(101))
            .build(),
        Err(ValidationError::TooLong {
            field: "description".into(),
            max: 100,
            actual: 101
        })
    );
    let error = LabelBuilder::new("Field", ButtonBuilder::primary("b").label("x"))
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "component");
    // The field is checked too, with its path.
    let error = ModalBuilder::new("modal", "Title")
        .text("intro")
        .label("Field", TextInputBuilder::short(""))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "components[1].component.custom_id");
}

#[test]
fn text_inputs_have_bounded_lengths() {
    assert!(
        with(
            TextInputBuilder::short("t")
                .min_length(4000)
                .max_length(4000)
        )
        .is_ok()
    );
    assert!(with(TextInputBuilder::short("t").min_length(0).max_length(1)).is_ok());
    assert_eq!(
        with(TextInputBuilder::short("t").min_length(4001)),
        out_of_range("min_length", 0, 4000, 4001)
    );
    assert_eq!(
        with(TextInputBuilder::short("t").max_length(0)),
        out_of_range("max_length", 1, 4000, 0)
    );
    assert_eq!(
        with(TextInputBuilder::short("t").max_length(4001)),
        out_of_range("max_length", 1, 4000, 4001)
    );
    assert_eq!(
        with(
            TextInputBuilder::paragraph("t")
                .min_length(10)
                .max_length(5)
        ),
        out_of_range("max_length", 10, 4000, 5)
    );
    assert_eq!(
        TextInputBuilder::short("t").value("a".repeat(4001)).build(),
        Err(ValidationError::TooLong {
            field: "value".into(),
            max: 4000,
            actual: 4001
        })
    );
    assert_eq!(
        TextInputBuilder::short("t")
            .placeholder("a".repeat(101))
            .build(),
        Err(ValidationError::TooLong {
            field: "placeholder".into(),
            max: 100,
            actual: 101
        })
    );
}

#[test]
fn select_menus_in_a_modal_are_enabled_and_need_a_value_while_required() {
    let menu = || StringSelectMenuBuilder::new("menu").option("a", "a");
    assert!(with(menu()).is_ok());
    let error = with(menu().disabled(true)).unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[0].component.disabled",
            "a select menu in a modal cannot be disabled"
        )
    );
    // A disabled menu is fine in a message.
    assert!(menu().disabled(true).build().is_ok());
    let error = with(UserSelectMenuBuilder::new("user").min_values(0)).unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[0].component.min_values",
            "min_values must be at least 1 while the field is required"
        )
    );
    assert!(
        with(
            UserSelectMenuBuilder::new("user")
                .min_values(0)
                .required(false)
        )
        .is_ok()
    );
}

#[test]
fn file_uploads_take_up_to_10_files_and_types() {
    assert!(with(FileUploadBuilder::new("f").min_values(10).max_values(10)).is_ok());
    assert_eq!(
        with(FileUploadBuilder::new("f").min_values(11)),
        out_of_range("min_values", 0, 10, 11)
    );
    assert_eq!(
        with(FileUploadBuilder::new("f").max_values(11)),
        out_of_range("max_values", 1, 10, 11)
    );
    // `max_values` defaults to 1.
    assert_eq!(
        with(FileUploadBuilder::new("f").min_values(2)),
        out_of_range("min_values", 0, 1, 2)
    );
    let types =
        |count: usize| FileUploadBuilder::new("f").file_types((0..count).map(|i| i.to_string()));
    assert!(with(types(10)).is_ok());
    assert_eq!(
        with(types(11)),
        Err(ValidationError::TooMany {
            field: "components[0].component.file_types".into(),
            max: 10,
            actual: 11
        })
    );
    let error = with(FileUploadBuilder::new("f").min_values(0)).unwrap_err();
    assert_eq!(invalid(&error).0, "components[0].component.min_values");
    assert!(with(FileUploadBuilder::new("f").min_values(0).required(false)).is_ok());
}

#[test]
fn a_radio_group_has_2_to_10_options_and_one_default() {
    let group = |count: usize| {
        (0..count).fold(RadioGroupBuilder::new("r"), |group, index| {
            group.option("Option", index.to_string())
        })
    };
    assert!(with(group(2)).is_ok());
    assert!(with(group(10)).is_ok());
    assert_eq!(
        group(1).build(),
        Err(ValidationError::TooFew {
            field: "options".into(),
            min: 2,
            actual: 1
        })
    );
    assert_eq!(
        group(11).build(),
        Err(ValidationError::TooMany {
            field: "options".into(),
            max: 10,
            actual: 11
        })
    );
    let selected = |value: &str| RadioGroupOptionBuilder::new("Option", value).default(true);
    let error = group(2)
        .option_with(selected("x"))
        .option_with(selected("y"))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[3].default",
            "only one option of a radio group can be selected"
        )
    );
    let error = group(2).option("Again", "1").build().unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[2].value",
            "another option of the group has the same value"
        )
    );
    assert_eq!(
        group(2).option(" ", "x").build(),
        Err(ValidationError::Missing {
            field: "options[2].label".into()
        })
    );
    assert_eq!(
        group(2)
            .option_with(RadioGroupOptionBuilder::new("x", "x").description("a".repeat(101)))
            .build(),
        Err(ValidationError::TooLong {
            field: "options[2].description".into(),
            max: 100,
            actual: 101
        })
    );
}

#[test]
fn a_checkbox_group_has_1_to_10_options_and_bounded_values() {
    let group = |count: usize| {
        (0..count).fold(CheckboxGroupBuilder::new("c"), |group, index| {
            group.option("Option", index.to_string())
        })
    };
    assert!(with(group(1)).is_ok());
    // `max_values` defaults to the number of options.
    assert!(with(group(10).min_values(10)).is_ok());
    assert_eq!(
        group(0).build(),
        Err(ValidationError::TooFew {
            field: "options".into(),
            min: 1,
            actual: 0
        })
    );
    assert_eq!(
        group(11).build(),
        Err(ValidationError::TooMany {
            field: "options".into(),
            max: 10,
            actual: 11
        })
    );
    assert_eq!(
        group(2).max_values(3).build(),
        Err(ValidationError::TooFew {
            field: "options".into(),
            min: 3,
            actual: 2
        })
    );
    assert_eq!(
        group(2).min_values(3).build(),
        Err(ValidationError::OutOfRange {
            field: "min_values".into(),
            min: 0,
            max: 2,
            actual: 3
        })
    );
    let error = with(group(2).min_values(0)).unwrap_err();
    assert_eq!(invalid(&error).0, "components[0].component.min_values");
    assert!(with(group(2).min_values(0).required(false)).is_ok());
}

#[test]
fn a_checkbox_needs_a_custom_id() {
    assert!(with(CheckboxBuilder::new("c")).is_ok());
    assert_eq!(
        CheckboxBuilder::new("").build(),
        Err(ValidationError::Missing {
            field: "custom_id".into()
        })
    );
    assert_eq!(
        CheckboxBuilder::new("a".repeat(101)).build(),
        Err(ValidationError::TooLong {
            field: "custom_id".into(),
            max: 100,
            actual: 101
        })
    );
}

#[test]
fn custom_ids_are_unique_in_a_modal() {
    let error = ModalBuilder::new("modal", "Title")
        .label("A", TextInputBuilder::short("same"))
        .label("B", CheckboxBuilder::new("same"))
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "components[1].component.custom_id");
    // The modal's own custom_id is separate from its fields'.
    assert!(
        ModalBuilder::new("same", "Title")
            .label("A", TextInputBuilder::short("same"))
            .build()
            .is_ok()
    );
}
