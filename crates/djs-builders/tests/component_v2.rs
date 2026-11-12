//! The Components V2 builders. Cases marked "live" reproduce what Discord accepted or rejected
//! on the live API (D-205).

use djs_builders::{
    ActionRowBuilder, ButtonBuilder, ComponentsVersion, ContainerBuilder, FileBuilder,
    MediaGalleryBuilder, MediaGalleryItemBuilder, SectionBuilder, SeparatorBuilder,
    TextDisplayBuilder, ThumbnailBuilder, ValidationError, validate_components,
};
use djs_model::component::{Component, SeparatorSpacing, TextDisplay};
use serde_json::json;

const IMAGE: &str = "https://example.com/a.png";

fn invalid(error: &ValidationError) -> (&str, &'static str) {
    match error {
        ValidationError::Invalid { field, reason } => (field, reason),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

fn button(id: &str) -> ButtonBuilder {
    ButtonBuilder::primary(id).label("x")
}

#[test]
fn a_card_ends_up_in_the_json() {
    let card = ContainerBuilder::new()
        .id(1)
        .accent_color(0x58_65F2)
        .spoiler(true)
        .text("## Title")
        .component(
            SectionBuilder::new(
                ThumbnailBuilder::new("attachment://logo.png")
                    .description("Logo")
                    .spoiler(true),
            )
            .text("Body")
            .component(TextDisplayBuilder::new("-# small").id(4)),
        )
        .component(
            SeparatorBuilder::new()
                .divider(false)
                .spacing(SeparatorSpacing::Large),
        )
        .component(
            MediaGalleryBuilder::new().item(IMAGE).item_with(
                MediaGalleryItemBuilder::new("attachment://b.png")
                    .description("B")
                    .spoiler(true),
            ),
        )
        .component(FileBuilder::new("attachment://report.pdf").spoiler(true))
        .component(
            ActionRowBuilder::new()
                .component(ButtonBuilder::link("https://example.com").label("Open")),
        )
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&card).unwrap(),
        json!({"type": 17, "id": 1, "accent_color": 0x58_65F2, "spoiler": true, "components": [
            {"type": 10, "content": "## Title"},
            {"type": 9, "components": [
                {"type": 10, "content": "Body"},
                {"type": 10, "id": 4, "content": "-# small"}
             ], "accessory": {"type": 11, "media": {"url": "attachment://logo.png"}, "description": "Logo", "spoiler": true}},
            {"type": 14, "divider": false, "spacing": 2},
            {"type": 12, "items": [
                {"media": {"url": IMAGE}},
                {"media": {"url": "attachment://b.png"}, "description": "B", "spoiler": true}
            ]},
            {"type": 13, "file": {"url": "attachment://report.pdf"}, "spoiler": true},
            {"type": 1, "components": [{"type": 2, "style": 5, "url": "https://example.com", "label": "Open"}]}
        ]})
    );
}

#[test]
fn live_a_container_holds_1_to_40_components() {
    assert_eq!(
        ContainerBuilder::new().build(),
        Err(ValidationError::TooFew {
            field: "components".into(),
            min: 1,
            actual: 0
        })
    );
    let texts = |count: usize| {
        (0..count).fold(ContainerBuilder::new(), |container, index| {
            container.text(index.to_string())
        })
    };
    assert!(texts(40).build().is_ok());
    assert_eq!(
        texts(41).build(),
        Err(ValidationError::TooMany {
            field: "components".into(),
            max: 40,
            actual: 41
        })
    );
    let error = ContainerBuilder::new()
        .component(button("a"))
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "components[0]");
    assert_eq!(
        ContainerBuilder::new()
            .text("x")
            .accent_color(0x100_0000)
            .build(),
        Err(ValidationError::OutOfRange {
            field: "accent_color".into(),
            min: 0,
            max: 0xFF_FFFF,
            actual: 0x100_0000
        })
    );
}

#[test]
fn custom_ids_are_unique_inside_a_container() {
    let error = ContainerBuilder::new()
        .component(ActionRowBuilder::new().component(button("same")))
        .component(SectionBuilder::new(button("same")).text("t"))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "components[1].accessory.custom_id",
            "another component of the message has the same custom_id"
        )
    );
}

#[test]
fn live_a_section_has_1_to_3_texts_and_a_button_or_thumbnail() {
    let section = |texts: usize| {
        (0..texts).fold(SectionBuilder::new(button("b")), |section, index| {
            section.text(index.to_string())
        })
    };
    assert!(section(1).build().is_ok());
    assert!(section(3).build().is_ok());
    assert_eq!(
        section(0).build(),
        Err(ValidationError::TooFew {
            field: "components".into(),
            min: 1,
            actual: 0
        })
    );
    assert_eq!(
        section(4).build(),
        Err(ValidationError::TooMany {
            field: "components".into(),
            max: 3,
            actual: 4
        })
    );
    let error = SectionBuilder::new(TextDisplayBuilder::new("not an accessory"))
        .text("t")
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "accessory",
            "a section accessory must be a button or a thumbnail"
        )
    );
    let error = SectionBuilder::new(button("b"))
        .component(SeparatorBuilder::new())
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        ("components[0]", "a section can only hold text displays")
    );
    // The accessory is checked too.
    let error = SectionBuilder::new(ButtonBuilder::primary("b"))
        .text("t")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "accessory.label");
}

#[test]
fn live_files_need_an_uploaded_file() {
    assert!(FileBuilder::new("attachment://probe.txt").build().is_ok());
    assert!(FileBuilder::new("ATTACHMENT://probe.txt").build().is_ok());
    assert_eq!(
        FileBuilder::new("https://example.com/a.txt").build(),
        Err(ValidationError::InvalidUrl {
            field: "file.url".into(),
            allowed: &["attachment://"]
        })
    );
}

#[test]
fn media_needs_a_link_or_an_uploaded_file() {
    // Discord accepts any scheme for a thumbnail and does not show it (D-205); D-190 rejects it.
    let error = ThumbnailBuilder::new("ftp://example.com/a.png")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "media.url");
    assert!(ThumbnailBuilder::new("attachment://a.png").build().is_ok());
    let error = MediaGalleryBuilder::new()
        .item(IMAGE)
        .item("ftp://x")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "items[1].media.url");
}

#[test]
fn live_galleries_have_1_to_10_items() {
    let gallery = |items: usize| {
        (0..items).fold(MediaGalleryBuilder::new(), |gallery, _| gallery.item(IMAGE))
    };
    assert!(gallery(10).build().is_ok());
    assert_eq!(
        gallery(0).build(),
        Err(ValidationError::TooFew {
            field: "items".into(),
            min: 1,
            actual: 0
        })
    );
    assert_eq!(
        gallery(11).build(),
        Err(ValidationError::TooMany {
            field: "items".into(),
            max: 10,
            actual: 11
        })
    );
}

#[test]
fn live_descriptions_have_1_to_1024_code_points() {
    assert!(
        ThumbnailBuilder::new(IMAGE)
            .description("🔥".repeat(1024))
            .build()
            .is_ok()
    );
    assert_eq!(
        ThumbnailBuilder::new(IMAGE)
            .description("a".repeat(1025))
            .build(),
        Err(ValidationError::TooLong {
            field: "description".into(),
            max: 1024,
            actual: 1025
        })
    );
    assert_eq!(
        ThumbnailBuilder::new(IMAGE).description("  ").build(),
        Err(ValidationError::Missing {
            field: "description".into()
        })
    );
    let error = MediaGalleryBuilder::new()
        .item_with(MediaGalleryItemBuilder::new(IMAGE).description("a".repeat(1025)))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "items[0].description");
}

#[test]
fn live_text_displays_are_not_blank() {
    assert_eq!(
        TextDisplayBuilder::new("   ").build(),
        Err(ValidationError::Missing {
            field: "content".into()
        })
    );
    assert_eq!(
        TextDisplayBuilder::new("a".repeat(4001)).build(),
        Err(ValidationError::TooLong {
            field: "content".into(),
            max: 4000,
            actual: 4001
        })
    );
    let error = ContainerBuilder::new()
        .text("ok")
        .text("")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "components[1].content");
}

#[test]
fn a_whole_v2_message_is_checked() {
    let card = ContainerBuilder::new()
        .component(SectionBuilder::new(button("a")).text("t"))
        .build()
        .unwrap();
    let row = ActionRowBuilder::new()
        .component(button("b"))
        .build()
        .unwrap();
    assert!(validate_components(&[card.clone(), row], ComponentsVersion::V2).is_ok());
    let again = ActionRowBuilder::new()
        .component(button("a"))
        .build()
        .unwrap();
    let error = validate_components(&[card, again], ComponentsVersion::V2).unwrap_err();
    assert_eq!(error.field(), "components[1].components[0].custom_id");
    // A thumbnail is only an accessory.
    let thumbnail = ThumbnailBuilder::new(IMAGE).build().unwrap();
    let error = validate_components(&[thumbnail], ComponentsVersion::V2).unwrap_err();
    assert_eq!(invalid(&error).0, "components[0]");
    // Raw model values get the same checks.
    let raw: Component = TextDisplay::new("").into();
    let error = validate_components(&[raw], ComponentsVersion::V2).unwrap_err();
    assert_eq!(error.field(), "components[0].content");
}
