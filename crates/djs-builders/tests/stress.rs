//! Extreme-load tests of the builders (D-041): every core building maximum-size values,
//! huge inputs rejected in bounded time, and deeply nested trees rejected without exhausting
//! the stack.
//!
//! The load tests are ignored by default because they take seconds and saturate the machine.
//! Run them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-builders --release --test stress -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    mem::size_of,
    sync::{
        Barrier,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use djs_builders::{
    ActionRowBuilder, ButtonBuilder, CheckboxGroupBuilder, ComponentsVersion, ContainerBuilder,
    EmbedBuilder, FileUploadBuilder, LabelBuilder, MediaGalleryBuilder, ModalBuilder,
    RadioGroupBuilder, SectionBuilder, SeparatorBuilder, SlashCommandBuilder,
    SlashCommandStringOption, SlashCommandSubcommandBuilder, StringSelectMenuBuilder,
    TextInputBuilder, ValidationError, embed_length, validate_components, validate_embeds,
};
use djs_model::{
    component::{
        ActionRow, Component, Container, Label, Section, TextDisplay, TextInput, TextInputStyle,
        Thumbnail, UnfurledMediaItem,
    },
    interaction::ModalCallbackData,
    message::{Embed, EmbedField},
    rest::CreateApplicationCommand,
};

/// `text` repeated until it has `length` code points.
fn text(pattern: &str, length: usize) -> String {
    pattern.chars().cycle().take(length).collect()
}

/// An embed just under the 6000-character total, with multi-byte text everywhere.
fn maximum_embed() -> EmbedBuilder {
    (0..25).fold(
        EmbedBuilder::new()
            .title(text("Zażółć 🔥 ", 256))
            .description(text("gęślą jaźń ", 2000))
            .author(text("ąę", 256))
            .footer(text("🔥", 248))
            .url("https://example.com")
            .image("https://example.com/a.png")
            .thumbnail("attachment://b.png")
            .timestamp_now(),
        |embed, index| embed.field(text(&format!("pole {index} "), 30), text("ż🔥", 99)),
    )
}

/// A command just under the 8000-character total: 25 subcommands of 10 options.
fn maximum_command() -> SlashCommandBuilder {
    (0..25).fold(
        SlashCommandBuilder::new("maxed", text("Command description ", 20)),
        |command, index| {
            command.subcommand((0..10).fold(
                SlashCommandSubcommandBuilder::new(
                    format!("sub-{index:08}"),
                    text("subcommand ", 20),
                ),
                |subcommand, option| {
                    subcommand.option(SlashCommandStringOption::new(
                        format!("opt-{option:04}"),
                        text("option description ", 20),
                    ))
                },
            ))
        },
    )
}

/// A Components V2 message of exactly 40 components and 3900 characters of text.
fn maximum_v2_message() -> Vec<Component> {
    let card = (0..9).fold(ContainerBuilder::new(), |card, index| {
        card.component(
            SectionBuilder::new(ButtonBuilder::secondary(format!("edit-{index}")).label("Edit"))
                .text(text("ż🔥 section ", 300)),
        )
    });
    let card = card
        .component((0..10).fold(MediaGalleryBuilder::new(), |gallery, index| {
            gallery.item(format!("https://example.com/{index}.png"))
        }))
        .component((0..5).fold(ActionRowBuilder::new(), |row, index| {
            row.component(ButtonBuilder::primary(format!("page-{index}")).label("Page"))
        }));
    let mut components = vec![card.build_unchecked()];
    components.extend((0..4).map(|_| TextDisplay::new(text("ą🔥 top ", 300)).into()));
    components.push(SeparatorBuilder::new().build_unchecked());
    components
}

/// A modal with every field type at its largest.
fn maximum_modal() -> ModalBuilder {
    ModalBuilder::new(text("m", 100), text("Tytuł 🔥 ", 45))
        .label(
            text("Etykieta ", 45),
            TextInputBuilder::paragraph("body")
                .min_length(4000)
                .max_length(4000)
                .value(text("ż", 4000))
                .placeholder(text("🔥", 100)),
        )
        .component(
            LabelBuilder::new(
                "Menu",
                (0..25).fold(
                    StringSelectMenuBuilder::new("menu").max_values(25),
                    |menu, index| menu.option(text("opcja ", 100), format!("{index:0100}")),
                ),
            )
            .description(text("opis ", 100)),
        )
        .label(
            "Radio",
            (0..10).fold(RadioGroupBuilder::new("radio"), |group, index| {
                group.option(text("ą", 100), format!("{index:0100}"))
            }),
        )
        .label(
            "Checkboxes",
            (0..10).fold(CheckboxGroupBuilder::new("checks"), |group, index| {
                group.option(text("ę", 100), format!("{index:0100}"))
            }),
        )
        .label(
            "Files",
            FileUploadBuilder::new("files")
                .max_values(10)
                .file_types((0..10).map(|index| format!(".ext{index}"))),
        )
}

fn threads() -> usize {
    thread::available_parallelism().map_or(4, usize::from)
}

/// Runs `build` on `threads` threads for `duration` and returns the builds per second.
fn throughput(threads: usize, duration: Duration, build: impl Fn() -> bool + Sync) -> f64 {
    let stop = AtomicBool::new(false);
    let builds = AtomicU64::new(0);
    let failures = AtomicU64::new(0);
    let start = Barrier::new(threads + 1);
    let elapsed = thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let (mut done, mut failed) = (0_u64, 0_u64);
                start.wait();
                while !stop.load(Ordering::Relaxed) {
                    if !build() {
                        failed += 1;
                    }
                    done += 1;
                }
                builds.fetch_add(done, Ordering::Relaxed);
                failures.fetch_add(failed, Ordering::Relaxed);
            });
        }
        start.wait();
        let began = Instant::now();
        thread::sleep(duration);
        stop.store(true, Ordering::Relaxed);
        began.elapsed()
    });
    assert_eq!(failures.into_inner(), 0, "a maximum value failed to build");
    builds.into_inner() as f64 / elapsed.as_secs_f64()
}

#[test]
fn maximum_values_are_valid() {
    let embed = maximum_embed().build().unwrap();
    assert!(
        (5900..=6000).contains(&embed_length(&embed)),
        "{}",
        embed_length(&embed)
    );
    maximum_command().build().unwrap();
    validate_components(&maximum_v2_message(), ComponentsVersion::V2).unwrap();
    maximum_modal().build().unwrap();
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn every_core_builds_maximum_values() {
    let cores = threads();
    let duration = Duration::from_secs(2);
    // The values are made once; each build clones one (every string is copied, as when a
    // bot assembles a value from its own strings) and checks it.
    let (embed, command, modal) = (maximum_embed(), maximum_command(), maximum_modal());
    let message = maximum_v2_message();
    let cases: [(&str, &(dyn Fn() -> bool + Sync)); 4] = [
        ("embed (6000 chars, 25 fields): clone + build", &|| {
            embed.clone().build().is_ok()
        }),
        ("command (25 × 10 options): clone + build", &|| {
            command.clone().build().is_ok()
        }),
        ("V2 message (40 components): validate_components", &|| {
            validate_components(&message, ComponentsVersion::V2).is_ok()
        }),
        ("modal (5 maximal fields): clone + build", &|| {
            modal.clone().build().is_ok()
        }),
    ];
    for (name, build) in cases {
        let one = throughput(1, duration, build);
        let all = throughput(cores, duration, build);
        println!(
            "{name}: {one:.0}/s on 1 thread, {all:.0}/s on {cores} threads ({:.1}×)",
            all / one
        );
    }
}

/// Times `check` and asserts it returns `expected` within a second.
fn bounded(
    name: &str,
    expected: &ValidationError,
    check: impl FnOnce() -> Result<(), ValidationError>,
) {
    let began = Instant::now();
    let result = check();
    let elapsed = began.elapsed();
    println!("{name}: rejected in {elapsed:?}");
    assert_eq!(result.as_ref(), Err(expected), "{name}");
    assert!(elapsed < Duration::from_secs(1), "{name} took {elapsed:?}");
}

fn too_many(field: &str, max: usize, actual: usize) -> ValidationError {
    ValidationError::TooMany {
        field: field.to_owned().into(),
        max,
        actual,
    }
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn huge_inputs_are_rejected_in_bounded_time() {
    const MILLION: usize = 1_000_000;

    // 10 MB of text is counted once.
    let title = text("ż", 5 * MILLION);
    let embed = EmbedBuilder::new().title(title);
    bounded(
        "a 10 MB embed title",
        &ValidationError::TooLong {
            field: "title".into(),
            max: 256,
            actual: 5 * MILLION,
        },
        || embed.build().map(drop),
    );

    let fields = EmbedBuilder::new()
        .title("t")
        .fields((0..MILLION).map(|_| EmbedField::new("n", "v", false)));
    bounded(
        "1 000 000 embed fields",
        &too_many("fields", 25, MILLION),
        || fields.build().map(drop),
    );

    let embeds = vec![Embed::default(); MILLION];
    bounded("1 000 000 embeds", &too_many("embeds", 10, MILLION), || {
        validate_embeds(&embeds)
    });

    let options = (0..MILLION).fold(StringSelectMenuBuilder::new("menu"), |menu, index| {
        menu.option("o", index.to_string())
    });
    bounded(
        "1 000 000 select options",
        &too_many("options", 25, MILLION),
        || options.build().map(drop),
    );

    let command = (0..MILLION).fold(
        SlashCommandBuilder::new("huge", "huge"),
        |command, index| command.option(SlashCommandStringOption::new(format!("o{index}"), "d")),
    );
    bounded(
        "1 000 000 command options",
        &too_many("options", 25, MILLION),
        || command.build().map(drop),
    );

    let texts: Vec<Component> = vec![TextDisplay::new("t").into(); MILLION];
    let container = ContainerBuilder::new().components(texts.clone());
    bounded(
        "1 000 000 components in a container",
        &too_many("components", 40, MILLION),
        || container.build().map(drop),
    );
    let modal = ModalBuilder::new("m", "t");
    let modal = texts.iter().cloned().fold(modal, ModalBuilder::component);
    bounded(
        "1 000 000 components in a modal",
        &too_many("components", 5, MILLION),
        || modal.build().map(drop),
    );

    // 200 000 rows of 5 buttons: a million custom_ids to compare.
    let rows: Vec<Component> = (0..200_000)
        .map(|row| {
            (0..5)
                .fold(ActionRowBuilder::new(), |builder, index| {
                    builder.component(ButtonBuilder::primary(format!("{row}-{index}")).label("b"))
                })
                .build_unchecked()
        })
        .collect();
    bounded(
        "a V2 message of 1 000 000 buttons",
        &too_many("components", 40, 1_200_000),
        || validate_components(&rows, ComponentsVersion::V2),
    );
    bounded(
        "a legacy message of 1 000 000 buttons",
        &too_many("components", 5, 200_000),
        || validate_components(&rows, ComponentsVersion::Legacy),
    );
    bounded(
        "a V2 message of 1 000 000 text displays",
        &too_many("components", 40, MILLION),
        || validate_components(&texts, ComponentsVersion::V2),
    );
}

/// Takes a nested tree apart iteratively: dropping it recursively would need a deep stack.
fn dismantle(mut component: Component) {
    loop {
        let next = match &mut component {
            Component::ActionRow(row) => row.components.pop(),
            Component::Container(container) => container.components.pop(),
            Component::Section(section) => section.components.pop(),
            Component::Label(label) => Some(std::mem::replace(
                &mut *label.component,
                TextDisplay::new("").into(),
            )),
            _ => None,
        };
        match next {
            Some(next) => component = next,
            None => break,
        }
    }
}

#[test]
fn deeply_nested_trees_are_rejected_without_exhausting_the_stack() {
    const DEPTH: usize = 200_000;
    let leaf = || -> Component { TextDisplay::new("leaf").into() };

    let rows = (0..DEPTH).fold(leaf(), |inner, _| ActionRow::new(vec![inner]).into());
    let error =
        validate_components(std::slice::from_ref(&rows), ComponentsVersion::V2).unwrap_err();
    assert_eq!(error.field(), "components[0].components[0]");
    dismantle(rows);

    let containers = (0..DEPTH).fold(leaf(), |inner, _| Container::new(vec![inner]).into());
    let error =
        validate_components(std::slice::from_ref(&containers), ComponentsVersion::V2).unwrap_err();
    assert_eq!(error.field(), "components[0].components[0]");
    dismantle(containers);

    let thumbnail =
        || -> Component { Thumbnail::new(UnfurledMediaItem::new("attachment://a")).into() };
    let sections = (0..DEPTH).fold(leaf(), |inner, _| {
        Section::new(vec![inner], thumbnail()).into()
    });
    let error =
        validate_components(std::slice::from_ref(&sections), ComponentsVersion::V2).unwrap_err();
    assert_eq!(error.field(), "components[0].components[0]");
    dismantle(sections);

    // `build()` drops the rejected modal, recursively: a smaller depth keeps that drop within
    // the test thread's stack. The path shows the check stopped at the first level.
    let field = TextInput::new("deep", TextInputStyle::Short).into();
    let labels = (0..1000).fold(field, |inner, _| Label::new("l", inner).into());
    let error = ModalBuilder::new("m", "t")
        .component(labels)
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "components[0].component");
}

#[test]
fn builders_are_the_size_of_what_they_build() {
    use djs_builders as b;
    use djs_model::component as c;
    macro_rules! same_size {
        ($($builder:ty => $model:ty),* $(,)?) => {
            $(
                println!("{}: {} B", stringify!($builder), size_of::<$builder>());
                assert_eq!(size_of::<$builder>(), size_of::<$model>(), stringify!($builder));
            )*
        };
    }
    same_size!(
        b::EmbedBuilder => Embed,
        b::SlashCommandBuilder => CreateApplicationCommand,
        b::ContextMenuCommandBuilder => CreateApplicationCommand,
        b::ButtonBuilder => c::Button,
        b::StringSelectMenuBuilder => c::StringSelect,
        b::ActionRowBuilder => c::ActionRow,
        b::ContainerBuilder => c::Container,
        b::SectionBuilder => c::Section,
        b::ModalBuilder => ModalCallbackData,
        b::LabelBuilder => c::Label,
        b::TextInputBuilder => c::TextInput,
    );
}
