//! `EmbedBuilder`: the JSON it produces and every rule `build()` checks. The cases marked
//! "live" reproduce what Discord accepted or rejected on the live API (D-183, D-188, D-191).

use djs_builders::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ValidationError, embed_length,
    validate_embeds,
};
use djs_model::{
    message::{Embed, EmbedAuthor, EmbedField, EmbedFooter},
    util::Timestamp,
};
use serde_json::json;

/// Puts a text into one part of an embed.
type WithText = fn(String) -> EmbedBuilder;

/// Builds an embed with a URL in one of its parts.
type BuildWithUrl = dyn Fn(&str) -> Result<Embed, ValidationError>;

fn too_long(field: &str, max: usize, actual: usize) -> ValidationError {
    ValidationError::TooLong {
        field: field.to_owned().into(),
        max,
        actual,
    }
}

fn invalid_url(field: &str, error: &ValidationError) -> bool {
    matches!(error, ValidationError::InvalidUrl { field: f, .. } if f == field)
}

fn is_empty(error: &ValidationError) -> bool {
    matches!(error, ValidationError::Empty { field, .. } if field == "embed")
}

/// Starts a builder from an embed written as JSON.
fn embed(value: serde_json::Value) -> EmbedBuilder {
    EmbedBuilder::from(serde_json::from_value::<Embed>(value).unwrap())
}

#[test]
fn every_part_ends_up_in_the_json() {
    let embed = EmbedBuilder::new()
        .title("Title")
        .url("https://example.com")
        .description("Description")
        .color_rgb(0x12, 0x34, 0x56)
        .author(
            EmbedAuthorBuilder::new("Author")
                .url("https://example.com/author")
                .icon_url("attachment://author.png"),
        )
        .footer(EmbedFooterBuilder::new("Footer").icon_url("https://example.com/footer.png"))
        .field("A", "1")
        .inline_field("B", "2")
        .fields([EmbedField::new("C", "3", true)])
        .image("attachment://image.png")
        .thumbnail("https://example.com/thumbnail.png")
        .timestamp(Timestamp::from_unix_micros(1_700_000_000_000_000))
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&embed).unwrap(),
        json!({
            "title": "Title",
            "description": "Description",
            "url": "https://example.com",
            "timestamp": "2023-11-14T22:13:20.000000+00:00",
            "color": 0x12_3456,
            "footer": {"text": "Footer", "icon_url": "https://example.com/footer.png"},
            "image": {"url": "attachment://image.png"},
            "thumbnail": {"url": "https://example.com/thumbnail.png"},
            "author": {
                "name": "Author",
                "url": "https://example.com/author",
                "icon_url": "attachment://author.png"
            },
            "fields": [
                {"name": "A", "value": "1"},
                {"name": "B", "value": "2", "inline": true},
                {"name": "C", "value": "3", "inline": true}
            ]
        })
    );
}

#[test]
fn a_setter_replaces_its_part_and_fields_accumulate() {
    let embed = EmbedBuilder::new()
        .title("first")
        .title("second")
        .author("first")
        .author("second")
        .field("a", "1")
        .field("b", "2")
        .build()
        .unwrap();
    assert_eq!(embed.title.as_deref(), Some("second"));
    assert_eq!(embed.author.unwrap().name, "second");
    assert_eq!(embed.fields.len(), 2);
}

#[test]
fn plain_texts_and_model_types_convert_into_the_pieces() {
    let name = String::from("Ada");
    let from_string_ref = EmbedBuilder::new().author(&name).build_unchecked();
    let from_string = EmbedBuilder::new().author(name.clone()).build_unchecked();
    let from_model = EmbedBuilder::new()
        .author(EmbedAuthor::new("Ada"))
        .footer(EmbedFooter::new("f"))
        .build_unchecked();
    for embed in [from_string_ref, from_string, from_model] {
        assert_eq!(embed.author.unwrap().name, "Ada");
    }
    let footer = EmbedBuilder::new()
        .footer("Page 1")
        .build()
        .unwrap()
        .footer
        .unwrap();
    assert_eq!(footer.text, "Page 1");
    assert_eq!(footer.icon_url, None);
}

#[test]
fn a_received_embed_is_kept_unless_a_setter_changes_it() {
    let received: Embed = serde_json::from_value(json!({
        "type": "rich",
        "title": "Poll",
        "description": "Open",
        "color": 1,
        "fields": [{"name": "Yes", "value": "3", "inline": true}],
        "thumbnail": {"url": "https://example.com/t.png", "proxy_url": "https://media.example/t.png", "width": 64, "height": 64}
    }))
    .unwrap();
    let unchanged = EmbedBuilder::from(received.clone()).build().unwrap();
    assert_eq!(unchanged, received);

    let edited = EmbedBuilder::from(received)
        .description("Closed")
        .field("No", "1")
        .build()
        .unwrap();
    assert_eq!(edited.description.as_deref(), Some("Closed"));
    assert_eq!(edited.title.as_deref(), Some("Poll"));
    assert_eq!(edited.fields.len(), 2);
    assert_eq!(edited.thumbnail.unwrap().width, Some(64));
}

#[test]
fn each_text_limit_is_inclusive() {
    let cases: [(&str, usize, WithText); 6] = [
        ("title", 256, |text| EmbedBuilder::new().title(text)),
        ("description", 4096, |text| {
            EmbedBuilder::new().description(text)
        }),
        ("fields[0].name", 256, |text| {
            EmbedBuilder::new().field(text, "v")
        }),
        ("fields[0].value", 1024, |text| {
            EmbedBuilder::new().field("n", text)
        }),
        ("footer.text", 2048, |text| EmbedBuilder::new().footer(text)),
        ("author.name", 256, |text| EmbedBuilder::new().author(text)),
    ];
    for (field, max, builder) in cases {
        assert!(builder("a".repeat(max)).build().is_ok(), "{field} at {max}");
        assert_eq!(
            builder("a".repeat(max + 1)).build(),
            Err(too_long(field, max, max + 1)),
            "{field} above {max}"
        );
    }
}

#[test]
fn the_path_of_a_field_names_its_index() {
    let error = EmbedBuilder::new()
        .field("a", "1")
        .field("b", "2")
        .field("c".repeat(257), "3")
        .build()
        .unwrap_err();
    assert_eq!(error, too_long("fields[2].name", 256, 257));
}

#[test]
fn live_lengths_are_code_points_after_trimming() {
    // Accepted on the live API: 256 x U+1F525 (512 UTF-16 units), 51 family emoji + "a"
    // (256 code points), 256 letters padded with spaces, NBSP and newlines.
    // Man, ZWJ, woman, ZWJ, girl: one grapheme, five code points (as in the live test).
    let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}";
    let accepted = [
        "🔥".repeat(256),
        format!("{}a", family.repeat(51)),
        format!("   {}   ", "a".repeat(256)),
        format!("\u{a0}\n{}\n\u{a0}", "a".repeat(256)),
        "é".repeat(256),
    ];
    for title in accepted {
        assert!(EmbedBuilder::new().title(title).build().is_ok());
    }
    // Rejected: 257 x U+1F525 and 52 family emoji (260 code points, 52 graphemes).
    assert_eq!(
        EmbedBuilder::new().title("🔥".repeat(257)).build(),
        Err(too_long("title", 256, 257))
    );
    assert_eq!(
        EmbedBuilder::new().title(family.repeat(52)).build(),
        Err(too_long("title", 256, 260))
    );
}

#[test]
fn at_most_25_fields() {
    let fields = |count| (0..count).map(|index| EmbedField::new(index.to_string(), "v", false));
    assert!(EmbedBuilder::new().fields(fields(25)).build().is_ok());
    assert_eq!(
        EmbedBuilder::new().fields(fields(26)).build(),
        Err(ValidationError::TooMany {
            field: "fields".into(),
            max: 25,
            actual: 26
        })
    );
}

#[test]
fn the_texts_of_an_embed_share_6000_characters() {
    let at_limit = || {
        EmbedBuilder::new()
            .title("a".repeat(256))
            .description("b".repeat(4096))
            .field("c".repeat(256), "d".repeat(1024))
            .footer("e".repeat(300))
    };
    // 256 + 4096 + 256 + 1024 + 300 = 5932; the author name brings it to 6000.
    assert!(at_limit().author("f".repeat(68)).build().is_ok());
    assert_eq!(
        at_limit().author("f".repeat(69)).build(),
        Err(too_long("embed", 6000, 6001))
    );
}

#[test]
fn live_color_is_at_most_ffffff() {
    assert!(
        EmbedBuilder::new()
            .title("x")
            .color(0xFF_FFFF)
            .build()
            .is_ok()
    );
    assert_eq!(
        EmbedBuilder::new().title("x").color(0x100_0000).build(),
        Err(ValidationError::OutOfRange {
            field: "color".into(),
            min: 0,
            max: 0xFF_FFFF,
            actual: 0x100_0000
        })
    );
    assert_eq!(
        EmbedBuilder::new()
            .title("x")
            .color_rgb(255, 255, 255)
            .build()
            .unwrap()
            .color,
        Some(0xFF_FFFF)
    );
}

#[test]
fn live_link_urls_allow_http_and_https_in_any_case() {
    for url in [
        "http://example.com",
        "https://example.com",
        "HTTPS://example.com",
    ] {
        assert!(
            EmbedBuilder::new().title("x").url(url).build().is_ok(),
            "{url}"
        );
        let author = EmbedAuthorBuilder::new("a").url(url);
        assert!(EmbedBuilder::new().author(author).build().is_ok(), "{url}");
    }
    // Rejected on the live API with URL_TYPE_INVALID_SCHEME.
    let error = EmbedBuilder::new()
        .title("x")
        .url("ftp://example.com")
        .build()
        .unwrap_err();
    assert!(invalid_url("url", &error));
    let error = EmbedBuilder::new()
        .author(EmbedAuthorBuilder::new("a").url("attachment://a.png"))
        .build()
        .unwrap_err();
    assert!(invalid_url("author.url", &error));
    assert_eq!(
        error.to_string(),
        "author.url must start with http:// or https://"
    );
}

#[test]
fn images_and_icons_also_allow_attachments_but_no_other_scheme() {
    let image = |url: &str| EmbedBuilder::new().image(url).build();
    let thumbnail = |url: &str| EmbedBuilder::new().thumbnail(url).build();
    let footer_icon = |url: &str| {
        EmbedBuilder::new()
            .footer(EmbedFooterBuilder::new("f").icon_url(url))
            .build()
    };
    let author_icon = |url: &str| {
        EmbedBuilder::new()
            .author(EmbedAuthorBuilder::new("a").icon_url(url))
            .build()
    };
    let cases: [(&str, &BuildWithUrl); 4] = [
        ("image.url", &image),
        ("thumbnail.url", &thumbnail),
        ("footer.icon_url", &footer_icon),
        ("author.icon_url", &author_icon),
    ];
    for (field, build) in cases {
        for url in ["https://example.com/a.png", "Attachment://a.png"] {
            assert!(build(url).is_ok(), "{field}: {url}");
        }
        // Discord accepts this one and just does not show it; D-190 rejects it anyway.
        assert!(invalid_url(
            field,
            &build("ftp://example.com/a.png").unwrap_err()
        ));
    }
}

#[test]
fn live_empty_texts_are_accepted_next_to_content() {
    let cases = [
        EmbedBuilder::new().title("").description("ok"),
        EmbedBuilder::new().title("   ").description("ok"),
        EmbedBuilder::new().title("ok").description(""),
        EmbedBuilder::new().title("ok").footer(""),
        EmbedBuilder::new().title("ok").author(""),
        EmbedBuilder::new().title("ok").field("", "v"),
        EmbedBuilder::new().title("ok").field("n", ""),
        EmbedBuilder::new().title("ok").field("  ", "v"),
    ];
    for builder in cases {
        assert!(builder.clone().build().is_ok(), "{builder:?}");
    }
}

#[test]
fn live_an_embed_needs_content() {
    let accepted = [
        EmbedBuilder::new().image("https://example.com/a.png"),
        EmbedBuilder::new().thumbnail("https://example.com/a.png"),
        EmbedBuilder::new().footer("f"),
        EmbedBuilder::new().author("a"),
        EmbedBuilder::new().field("n", "v"),
        EmbedBuilder::new().field("", ""),
        EmbedBuilder::new().title("t"),
        EmbedBuilder::new().description("d"),
    ];
    for builder in accepted {
        assert!(builder.clone().build().is_ok(), "{builder:?}");
    }
    let rejected = [
        EmbedBuilder::new(),
        EmbedBuilder::new().color(1),
        EmbedBuilder::new().title(""),
        EmbedBuilder::new().title("   "),
        EmbedBuilder::new().description(""),
        EmbedBuilder::new().url("https://example.com"),
        EmbedBuilder::new().timestamp_now(),
        EmbedBuilder::new().footer(""),
        // Not tried on the live API; assumed to behave like the footer.
        EmbedBuilder::new().author(""),
    ];
    for builder in rejected {
        assert!(
            is_empty(&builder.clone().build().unwrap_err()),
            "{builder:?}"
        );
    }
    assert_eq!(
        EmbedBuilder::new().build().unwrap_err().to_string(),
        "embed has no content: it needs a title, a description, a field, an author, a footer, \
         an image or a thumbnail"
    );
}

#[test]
fn the_first_problem_is_reported() {
    let error = EmbedBuilder::new()
        .author("a".repeat(300))
        .description("d".repeat(5000))
        .title("t".repeat(300))
        .build()
        .unwrap_err();
    assert_eq!(error, too_long("title", 256, 300));

    // Per-part limits come before the total, the total before the content check.
    let error = embed(json!({"color": 0x0100_0000})).build().unwrap_err();
    assert_eq!(error.field(), "color");
}

#[test]
fn build_unchecked_keeps_an_invalid_embed() {
    let embed = EmbedBuilder::new()
        .title("t".repeat(300))
        .url("ftp://example.com")
        .color(u32::MAX)
        .build_unchecked();
    assert_eq!(embed.title.as_deref().map(str::len), Some(300));
    assert_eq!(embed.color, Some(u32::MAX));
}

#[test]
fn the_pieces_can_be_checked_on_their_own() {
    assert_eq!(
        EmbedAuthorBuilder::new("a".repeat(257)).build(),
        Err(too_long("name", 256, 257))
    );
    let error = EmbedAuthorBuilder::new("a")
        .url("attachment://a.png")
        .build()
        .unwrap_err();
    assert!(invalid_url("url", &error));
    let error = EmbedAuthorBuilder::new("a")
        .icon_url("ftp://a")
        .build()
        .unwrap_err();
    assert!(invalid_url("icon_url", &error));
    assert_eq!(
        EmbedFooterBuilder::new("f".repeat(2049)).build(),
        Err(too_long("text", 2048, 2049))
    );
    let error = EmbedFooterBuilder::new("f")
        .icon_url("ftp://a")
        .build()
        .unwrap_err();
    assert!(invalid_url("icon_url", &error));

    let author = EmbedAuthorBuilder::new("Ada")
        .url("https://example.com")
        .build()
        .unwrap();
    assert_eq!(author.url.as_deref(), Some("https://example.com"));
    let footer = EmbedFooterBuilder::new("f".repeat(3000)).build_unchecked();
    assert_eq!(footer.text.len(), 3000);
}

#[test]
fn embed_length_counts_the_six_texts() {
    let embed = embed(json!({
        "title": " ab ",
        "description": "🔥",
        "url": "https://not-counted.example",
        "fields": [{"name": "c", "value": "de"}, {"name": "f", "value": "g"}],
        "footer": {"text": "hi", "icon_url": "https://not-counted.example"},
        "author": {"name": "jkl"},
        "image": {"url": "https://not-counted.example"}
    }))
    .build_unchecked();
    // 2 + 1 + (1 + 2) + (1 + 1) + 2 + 3
    assert_eq!(embed_length(&embed), 13);
    assert_eq!(embed_length(&Embed::default()), 0);
}

#[test]
fn a_message_has_at_most_10_embeds_and_6000_characters() {
    let small = EmbedBuilder::new().title("x").build().unwrap();
    assert_eq!(validate_embeds(&[]), Ok(()));
    assert_eq!(validate_embeds(&vec![small.clone(); 10]), Ok(()));
    assert_eq!(
        validate_embeds(&vec![small; 11]),
        Err(ValidationError::TooMany {
            field: "embeds".into(),
            max: 10,
            actual: 11
        })
    );

    let half = EmbedBuilder::new()
        .description("d".repeat(3000))
        .build()
        .unwrap();
    let one = EmbedBuilder::new().title("t").build().unwrap();
    assert_eq!(validate_embeds(&[half.clone(), half.clone()]), Ok(()));
    assert_eq!(
        validate_embeds(&[half.clone(), half, one]),
        Err(too_long("embeds", 6000, 6001))
    );
}
