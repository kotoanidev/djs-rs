//! The command builders: the JSON they produce and every rule `build()` checks. Cases marked
//! "live" reproduce what Discord accepted or rejected on the live API (D-195); D-196 cases are
//! rejected although Discord accepts them.

use djs_builders::{
    ContextMenuCommandBuilder, SlashCommandAttachmentOption, SlashCommandBooleanOption,
    SlashCommandBuilder, SlashCommandChannelOption, SlashCommandIntegerOption,
    SlashCommandMentionableOption, SlashCommandNumberOption, SlashCommandRoleOption,
    SlashCommandStringOption, SlashCommandSubcommandBuilder, SlashCommandSubcommandGroupBuilder,
    SlashCommandUserOption, ValidationError,
};
use djs_model::{
    application::IntegrationType,
    application_command::{Choice, CommandOption},
    channel::ChannelType,
    interaction::InteractionContextType,
    permissions::Permissions,
    util::Locale,
};
use serde_json::json;

fn invalid(error: &ValidationError) -> (&str, &'static str) {
    match error {
        ValidationError::Invalid { field, reason } => (field, reason),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

fn slash(name: &str) -> SlashCommandBuilder {
    SlashCommandBuilder::new(name, "A command")
}

fn with_option(option: impl Into<CommandOption>) -> Result<(), ValidationError> {
    slash("test").option(option).build().map(drop)
}

#[test]
fn a_full_command_ends_up_in_the_json() {
    let command = SlashCommandBuilder::new("ban", "Bans a member")
        .name_localization(Locale::Polish, "zbanuj")
        .description_localizations([
            (Locale::Polish, "Banuje członka"),
            (Locale::German, "Bannt"),
        ])
        .default_member_permissions(Permissions::BAN_MEMBERS)
        .contexts([InteractionContextType::Guild])
        .integration_types([IntegrationType::GuildInstall])
        .nsfw(true)
        .option(SlashCommandUserOption::new("user", "Who").required(true))
        .option(
            SlashCommandStringOption::new("reason", "Why")
                .min_length(3)
                .max_length(512)
                .autocomplete(true),
        )
        .option(
            SlashCommandIntegerOption::new("days", "Messages to delete")
                .choice("None", 0)
                .choice("A week", 7),
        )
        .option(
            SlashCommandNumberOption::new("weight", "Weight")
                .min_value(0.5)
                .max_value(2.0),
        )
        .option(SlashCommandBooleanOption::new("silent", "Quietly"))
        .option(SlashCommandRoleOption::new("role", "A role"))
        .option(SlashCommandMentionableOption::new("ping", "Who to ping"))
        .option(
            SlashCommandChannelOption::new("log", "Log channel")
                .channel_types([ChannelType::GuildText]),
        )
        .option(SlashCommandAttachmentOption::new("proof", "Screenshot").file_types(["png", "jpg"]))
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&command).unwrap(),
        json!({
            "name": "ban",
            "name_localizations": {"pl": "zbanuj"},
            "description": "Bans a member",
            "description_localizations": {"de": "Bannt", "pl": "Banuje członka"},
            "type": 1,
            "default_member_permissions": "4",
            "contexts": [0],
            "integration_types": [0],
            "nsfw": true,
            "options": [
                {"type": 6, "name": "user", "description": "Who", "required": true},
                {"type": 3, "name": "reason", "description": "Why", "min_length": 3, "max_length": 512, "autocomplete": true},
                {"type": 4, "name": "days", "description": "Messages to delete",
                 "choices": [{"name": "None", "value": 0}, {"name": "A week", "value": 7}]},
                {"type": 10, "name": "weight", "description": "Weight", "min_value": 0.5, "max_value": 2.0},
                {"type": 5, "name": "silent", "description": "Quietly"},
                {"type": 8, "name": "role", "description": "A role"},
                {"type": 9, "name": "ping", "description": "Who to ping"},
                {"type": 7, "name": "log", "description": "Log channel", "channel_types": [0]},
                {"type": 11, "name": "proof", "description": "Screenshot", "file_types": ["png", "jpg"]}
            ]
        })
    );
}

#[test]
fn subcommands_and_groups_end_up_in_the_json() {
    let command = slash("config")
        .subcommand(SlashCommandSubcommandBuilder::new("show", "Shows them"))
        .subcommand_group(
            SlashCommandSubcommandGroupBuilder::new("set", "Changes one")
                .name_localization(Locale::Polish, "ustaw")
                .subcommand(
                    SlashCommandSubcommandBuilder::new("prefix", "Sets the prefix").option(
                        SlashCommandStringOption::new("value", "New prefix").required(true),
                    ),
                ),
        )
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&command.options).unwrap(),
        json!([
            {"type": 1, "name": "show", "description": "Shows them"},
            {"type": 2, "name": "set", "name_localizations": {"pl": "ustaw"}, "description": "Changes one",
             "options": [{"type": 1, "name": "prefix", "description": "Sets the prefix",
                          "options": [{"type": 3, "name": "value", "description": "New prefix", "required": true}]}]}
        ])
    );
}

#[test]
fn context_menu_commands_have_a_type_and_no_description() {
    let message = ContextMenuCommandBuilder::message("Report message")
        .name_localization(Locale::Polish, "Zgłoś")
        .default_member_permissions(Permissions::empty())
        .contexts([InteractionContextType::Guild])
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&message).unwrap(),
        json!({"name": "Report message", "name_localizations": {"pl": "Zgłoś"}, "type": 3,
               "default_member_permissions": "0", "contexts": [0]})
    );
    let user = ContextMenuCommandBuilder::user("Profile").build().unwrap();
    assert_eq!(
        serde_json::to_value(&user).unwrap(),
        json!({"name": "Profile", "type": 2})
    );
}

#[test]
fn live_names_follow_discords_pattern() {
    for name in ["ping", "donʼt", "नमस्ते", &"ż".repeat(32), "user-info_2"] {
        assert!(slash(name).build().is_ok(), "{name}");
        let option = SlashCommandStringOption::new(name, "x");
        assert!(with_option(option).is_ok(), "option {name}");
    }
    assert_eq!(
        slash("Żółw").build(),
        Err(ValidationError::InvalidName {
            field: "name".into(),
            character: 'Ż'
        })
    );
    assert_eq!(
        slash("don't").build(),
        Err(ValidationError::InvalidName {
            field: "name".into(),
            character: '\''
        })
    );
    assert_eq!(
        slash(&"a".repeat(33)).build(),
        Err(ValidationError::TooLong {
            field: "name".into(),
            max: 32,
            actual: 33
        })
    );
    assert_eq!(
        slash("").build(),
        Err(ValidationError::Missing {
            field: "name".into()
        })
    );
    let error = slash("ok")
        .name_localization(Locale::German, "Bann")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "name_localizations.de");
    let error = with_option(SlashCommandStringOption::new("two words", "x")).unwrap_err();
    assert_eq!(
        error,
        ValidationError::InvalidName {
            field: "options[0].name".into(),
            character: ' '
        }
    );
}

#[test]
fn live_descriptions_count_code_points_after_trimming() {
    assert!(
        SlashCommandBuilder::new("a", "🔥".repeat(100))
            .build()
            .is_ok()
    );
    assert!(
        SlashCommandBuilder::new("a", format!("  {}  ", "a".repeat(100)))
            .build()
            .is_ok()
    );
    assert_eq!(
        SlashCommandBuilder::new("a", "🔥".repeat(101)).build(),
        Err(ValidationError::TooLong {
            field: "description".into(),
            max: 100,
            actual: 101
        })
    );
    assert_eq!(
        SlashCommandBuilder::new("a", "   ").build(),
        Err(ValidationError::Missing {
            field: "description".into()
        })
    );
    let error = slash("a")
        .description_localization(Locale::French, "x".repeat(101))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "description_localizations.fr");
    let error = with_option(SlashCommandUserOption::new("user", "")).unwrap_err();
    assert_eq!(error.field(), "options[0].description");
}

#[test]
fn live_option_names_are_unique_per_level() {
    let error = slash("a")
        .option(SlashCommandStringOption::new("a", "x"))
        .option(SlashCommandIntegerOption::new("a", "y"))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[1].name",
            "another option at this level has the same name"
        )
    );
    // The same name in two subcommands is fine.
    let command = slash("a")
        .subcommand(
            SlashCommandSubcommandBuilder::new("one", "x")
                .option(SlashCommandStringOption::new("v", "x")),
        )
        .subcommand(
            SlashCommandSubcommandBuilder::new("two", "x")
                .option(SlashCommandStringOption::new("v", "x")),
        )
        .build();
    assert!(command.is_ok());
}

#[test]
fn live_required_options_come_first() {
    let error = slash("echo")
        .option(SlashCommandStringOption::new("prefix", "x"))
        .option(SlashCommandStringOption::new("text", "y").required(true))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[1].required",
            "required options must come before optional ones"
        )
    );
    assert!(
        slash("echo")
            .option(SlashCommandStringOption::new("text", "y").required(true))
            .option(SlashCommandStringOption::new("prefix", "x"))
            .build()
            .is_ok()
    );
}

#[test]
fn live_string_lengths_are_bounded_and_ordered() {
    let length = |min: Option<u16>, max: Option<u16>| {
        let mut option = SlashCommandStringOption::new("text", "x");
        if let Some(min) = min {
            option = option.min_length(min);
        }
        if let Some(max) = max {
            option = option.max_length(max);
        }
        with_option(option)
    };
    assert!(length(Some(0), Some(6000)).is_ok());
    assert!(length(Some(10), Some(10)).is_ok());
    assert!(length(None, Some(1)).is_ok());
    assert_eq!(
        length(Some(10), Some(5)),
        Err(ValidationError::OutOfRange {
            field: "options[0].max_length".into(),
            min: 10,
            max: 6000,
            actual: 5
        })
    );
    assert_eq!(
        length(None, Some(0)),
        Err(ValidationError::OutOfRange {
            field: "options[0].max_length".into(),
            min: 1,
            max: 6000,
            actual: 0
        })
    );
    assert_eq!(
        length(Some(6001), None),
        Err(ValidationError::OutOfRange {
            field: "options[0].min_length".into(),
            min: 0,
            max: 6000,
            actual: 6001
        })
    );
}

#[test]
fn live_choices_have_names_and_short_values() {
    let error =
        with_option(SlashCommandStringOption::new("pick", "x").choice("   ", "v")).unwrap_err();
    assert_eq!(
        error,
        ValidationError::Missing {
            field: "options[0].choices[0].name".into()
        }
    );
    let error =
        with_option(SlashCommandStringOption::new("pick", "x").choice("ok", "v".repeat(101)))
            .unwrap_err();
    assert_eq!(error.field(), "options[0].choices[0].value");
    let error = with_option(SlashCommandIntegerOption::new("pick", "x").choice("n".repeat(101), 1))
        .unwrap_err();
    assert_eq!(error.field(), "options[0].choices[0].name");

    let mut translated = Choice::new("Dog", 1);
    translated
        .name_localizations
        .insert(Locale::Polish, String::new().into());
    let error =
        with_option(SlashCommandIntegerOption::new("pet", "x").choices([translated])).unwrap_err();
    assert_eq!(error.field(), "options[0].choices[0].name_localizations.pl");
}

#[test]
fn at_most_25_options_and_choices() {
    let options = |count: usize| {
        (0..count).fold(slash("a"), |command, index| {
            command.option(SlashCommandBooleanOption::new(format!("o{index}"), "x"))
        })
    };
    assert!(options(25).build().is_ok());
    assert_eq!(
        options(26).build(),
        Err(ValidationError::TooMany {
            field: "options".into(),
            max: 25,
            actual: 26
        })
    );
    let choices = |count: i64| {
        (0..count).fold(SlashCommandIntegerOption::new("n", "x"), |option, value| {
            option.choice(format!("c{value}"), value)
        })
    };
    assert!(with_option(choices(25)).is_ok());
    assert_eq!(
        with_option(choices(26)),
        Err(ValidationError::TooMany {
            field: "options[0].choices".into(),
            max: 25,
            actual: 26
        })
    );
}

#[test]
fn options_that_cannot_work_are_rejected() {
    // D-196: Discord accepts these two, but the option cannot work as written.
    let error = with_option(
        SlashCommandIntegerOption::new("n", "x")
            .min_value(10)
            .max_value(5),
    )
    .unwrap_err();
    assert_eq!(invalid(&error).0, "options[0].max_value");
    let error = with_option(
        SlashCommandNumberOption::new("n", "x")
            .min_value(1.5)
            .max_value(1.0),
    )
    .unwrap_err();
    assert_eq!(invalid(&error).0, "options[0].max_value");
    let error = with_option(
        SlashCommandStringOption::new("s", "x")
            .choice("a", "a")
            .autocomplete(true),
    )
    .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[0].autocomplete",
            "an option with choices cannot use autocomplete"
        )
    );
    let error = with_option(
        SlashCommandIntegerOption::new("n", "x")
            .choice("a", 1)
            .autocomplete(true),
    )
    .unwrap_err();
    assert_eq!(invalid(&error).0, "options[0].autocomplete");
    // Equal bounds are fine.
    assert!(
        with_option(
            SlashCommandIntegerOption::new("n", "x")
                .min_value(5)
                .max_value(5)
        )
        .is_ok()
    );
}

#[test]
fn numbers_stay_within_discords_range() {
    let max = (1_i64 << 53) - 1;
    assert!(
        with_option(
            SlashCommandIntegerOption::new("n", "x")
                .min_value(-max)
                .max_value(max)
        )
        .is_ok()
    );
    assert_eq!(
        with_option(SlashCommandIntegerOption::new("n", "x").max_value(max + 1)),
        Err(ValidationError::OutOfRange {
            field: "options[0].max_value".into(),
            min: -max,
            max,
            actual: max + 1
        })
    );
    let error =
        with_option(SlashCommandIntegerOption::new("n", "x").choice("big", i64::MIN)).unwrap_err();
    assert_eq!(error.field(), "options[0].choices[0].value");

    let two_53 = 9_007_199_254_740_992.0;
    assert!(
        with_option(
            SlashCommandNumberOption::new("n", "x")
                .min_value(-two_53)
                .max_value(two_53)
        )
        .is_ok()
    );
    for value in [f64::NAN, f64::INFINITY, two_53 * 2.0] {
        let error =
            with_option(SlashCommandNumberOption::new("n", "x").min_value(value)).unwrap_err();
        assert_eq!(invalid(&error).0, "options[0].min_value");
    }
}

#[test]
fn subcommands_and_options_do_not_mix() {
    let error = slash("a")
        .option(SlashCommandStringOption::new("text", "x"))
        .subcommand(SlashCommandSubcommandBuilder::new("sub", "x"))
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "options[1]");
    let error = slash("a")
        .subcommand(SlashCommandSubcommandBuilder::new("sub", "x"))
        .option(SlashCommandStringOption::new("text", "x"))
        .build()
        .unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[1]",
            "subcommands and subcommand groups cannot be mixed with other options"
        )
    );
    // A subcommand cannot contain a subcommand; a group only contains subcommands.
    let error = slash("a")
        .subcommand(
            SlashCommandSubcommandBuilder::new("outer", "x")
                .option(SlashCommandSubcommandBuilder::new("inner", "x")),
        )
        .build()
        .unwrap_err();
    assert_eq!(invalid(&error).0, "options[0].options[0]");
    let group = SlashCommandSubcommandGroupBuilder::new("group", "x");
    let error = slash("a")
        .option(CommandOption::from(group.clone()))
        .subcommand_group(
            group
                .clone()
                .subcommand(SlashCommandSubcommandBuilder::new("s", "x")),
        )
        .build()
        .unwrap_err();
    // The two groups share a name.
    assert_eq!(error.field(), "options[1].name");
    let mut nested = group.build_unchecked();
    if let CommandOption::SubCommandGroup(inner) = &mut nested {
        inner
            .options
            .push(SlashCommandStringOption::new("text", "x").build_unchecked());
    }
    let error = slash("a").option(nested).build().unwrap_err();
    assert_eq!(
        invalid(&error),
        (
            "options[0].options[0]",
            "a subcommand group can only contain subcommands"
        )
    );
}

#[test]
fn nested_problems_name_their_full_path() {
    let error = slash("config")
        .subcommand_group(
            SlashCommandSubcommandGroupBuilder::new("set", "x").subcommand(
                SlashCommandSubcommandBuilder::new("prefix", "x")
                    .option(SlashCommandStringOption::new("value", "x").required(true))
                    .option(SlashCommandStringOption::new("scope", "")),
            ),
        )
        .build()
        .unwrap_err();
    assert_eq!(
        error,
        ValidationError::Missing {
            field: "options[0].options[0].options[1].description".into()
        }
    );
}

/// A command whose names, descriptions and string choice values add up to exactly `target`
/// code points, padded with `pad` (the same shape as the live test).
fn command_of_total(name: &str, target: usize, pad: char) -> SlashCommandBuilder {
    let text = |prefix: String, length: usize| -> String {
        prefix
            .chars()
            .chain(std::iter::repeat_n(pad, length - 3))
            .collect()
    };
    let mut running = name.chars().count() + 100;
    let mut first = SlashCommandStringOption::new("o0", "p".repeat(100));
    for index in 0..25 {
        first = first.choice(
            text(format!("c{index:02}"), 100),
            text(format!("v{index:02}"), 100),
        );
    }
    running += 2 + 100 + 25 * 200;
    let mut second_description = 100;
    running += 2 + second_description;
    let mut choices = Vec::new();
    while target - running >= 200 + 6 {
        choices.push((100, 100));
        running += 200;
    }
    let mut remainder = target - running;
    if remainder < 6 {
        second_description -= 6 - remainder;
        remainder = 6;
    }
    choices.push((remainder / 2, remainder - remainder / 2));
    let mut second = SlashCommandStringOption::new("o1", "q".repeat(second_description));
    for (index, (name_length, value_length)) in choices.into_iter().enumerate() {
        second = second.choice(
            text(format!("c{index:02}"), name_length),
            text(format!("v{index:02}"), value_length),
        );
    }
    SlashCommandBuilder::new(name, "d".repeat(100))
        .option(first)
        .option(second)
}

#[test]
fn live_a_command_has_at_most_8000_characters() {
    assert!(command_of_total("total", 8000, 'a').build().is_ok());
    assert_eq!(
        command_of_total("total", 8001, 'a').build(),
        Err(ValidationError::TooLong {
            field: "command".into(),
            max: 8000,
            actual: 8001
        })
    );
    // Code points: this one has 15 000+ UTF-16 units and was accepted.
    assert!(command_of_total("total", 8000, '🔥').build().is_ok());
    // The longest translation of a field counts.
    assert_eq!(
        command_of_total("total", 8000, 'a')
            .name_localization(Locale::Polish, "totalx")
            .build(),
        Err(ValidationError::TooLong {
            field: "command".into(),
            max: 8000,
            actual: 8001
        })
    );
}

#[test]
fn live_context_menu_names_allow_anything_up_to_32_characters() {
    for name in ["Report 🔥", "Show Profile", &"Ż".repeat(32)] {
        assert!(
            ContextMenuCommandBuilder::user(name).build().is_ok(),
            "{name}"
        );
    }
    assert_eq!(
        ContextMenuCommandBuilder::message("x".repeat(33)).build(),
        Err(ValidationError::TooLong {
            field: "name".into(),
            max: 32,
            actual: 33
        })
    );
    assert_eq!(
        ContextMenuCommandBuilder::message("  ").build(),
        Err(ValidationError::Missing {
            field: "name".into()
        })
    );
    let error = ContextMenuCommandBuilder::user("Profile")
        .name_localization(Locale::Japanese, "")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "name_localizations.ja");
}

#[test]
fn options_and_subcommands_can_be_checked_on_their_own() {
    let error = SlashCommandStringOption::new("pick", "x")
        .choice("", "v")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "choices[0].name");
    let error = SlashCommandSubcommandBuilder::new("sub", "x")
        .option(SlashCommandStringOption::new("Bad", "x"))
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "options[0].name");
    let error = SlashCommandSubcommandGroupBuilder::new("group", "")
        .build()
        .unwrap_err();
    assert_eq!(error.field(), "description");
    let option = SlashCommandUserOption::new("user", "Who")
        .required(true)
        .build()
        .unwrap();
    assert_eq!(option.name(), "user");
}

#[test]
fn build_unchecked_keeps_an_invalid_command() {
    let command = SlashCommandBuilder::new("Bad Name", "")
        .option(
            SlashCommandIntegerOption::new("n", "x")
                .min_value(9)
                .max_value(1),
        )
        .build_unchecked();
    assert_eq!(command.name, "Bad Name");
    let menu = ContextMenuCommandBuilder::user("").build_unchecked();
    assert_eq!(menu.name, "");
}

#[test]
fn model_options_can_be_added_directly() {
    let raw: CommandOption =
        serde_json::from_value(json!({"type": 3, "name": "text", "description": "x"})).unwrap();
    let command = slash("a").option(raw.clone()).build().unwrap();
    assert_eq!(command.options, vec![raw]);
}
