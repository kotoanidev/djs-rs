//! Slash commands through the router: registration in a test guild, options, subcommands,
//! autocomplete, a modal awaited with `await_modal_submit`, ephemeral and deferred replies.
//!
//! | Command | Shows |
//! |---|---|
//! | `/ping` | an ephemeral reply |
//! | `/echo text [times] [private]` | string, integer (with choices) and boolean options, no pings |
//! | `/info user [user]`, `/info server` | subcommands, an embed, Discord timestamps |
//! | `/docs topic` | autocomplete |
//! | `/feedback` | a modal and `await_modal_submit` |
//! | `/slow` | a deferred reply, `edit_reply` and `follow_up` |
//!
//! # Running
//!
//! 1. Invite the bot to a test server with the `applications.commands` scope (part of the
//!    default invite of applications created since 2023).
//! 2. Put `DISCORD_TOKEN` in `.env` (see `.env.example`); `DISCORD_TEST_GUILD_ID` chooses the
//!    server, otherwise the first one the bot is in is used.
//! 3. Run `cargo run --example slash_commands` and type `/` in the server. Guild commands
//!    appear at once (global ones, registered with `ctx.commands()`, are meant for bots in
//!    production).
//! 4. `cargo run --example slash_commands -- --cleanup` removes the bot's commands from the
//!    server again and exits.
//!
//! No intent is needed: interactions arrive whatever the intents are.
//!
//! discord.js counterpart: the "slash commands" and "modals" sections of the discord.js guide.

use std::{sync::Arc, time::Duration};

use djs_rs::{
    builders::{
        EmbedBuilder, ModalBuilder, SlashCommandBooleanOption, SlashCommandBuilder,
        SlashCommandIntegerOption, SlashCommandStringOption, SlashCommandSubcommandBuilder,
        SlashCommandUserOption, TextInputBuilder, ValidationError,
    },
    formatters::{TimestampStyle, time, user_mention},
    model::{
        application_command::Choice,
        id::GuildId,
        interaction::{AutocompleteChoices, InteractionContextType, MessageCallbackData},
        message::MessageFlags,
        rest::CreateApplicationCommand,
        util::Timestamp,
    },
    prelude::*,
};

#[path = "common/mod.rs"]
mod common;

/// The topics `/docs` suggests, with their section of the README.
const TOPICS: &[(&str, &str)] = &[
    ("Events", "events"),
    ("Intents and partials", "intents-and-partials"),
    (
        "Managers and structure methods",
        "managers-and-structure-methods",
    ),
    ("Interactions", "interactions"),
    ("The router", "the-router"),
    ("Collectors", "collectors"),
    ("Webhooks without a bot", "webhooks-without-a-bot"),
    ("The cache", "the-cache"),
    ("The global allocator", "performance-the-global-allocator"),
    ("Cargo features", "cargo-features"),
];

const README: &str = "https://github.com/kotoanidev/djs-rs/blob/main/crates/djs-rs/README.md";

/// The commands this example registers. `build` checks Discord's rules (names, lengths,
/// option order) before anything is sent.
fn commands() -> Result<Vec<CreateApplicationCommand>, ValidationError> {
    Ok(vec![
        SlashCommandBuilder::new("ping", "Checks that the bot answers").build()?,
        SlashCommandBuilder::new("echo", "Repeats a text")
            .option(
                SlashCommandStringOption::new("text", "What to repeat")
                    .required(true)
                    .max_length(200),
            )
            .option(
                SlashCommandIntegerOption::new("times", "How many times")
                    .choice("Once", 1)
                    .choice("Twice", 2)
                    .choice("Three times", 3),
            )
            .option(SlashCommandBooleanOption::new(
                "private",
                "Only you see the answer",
            ))
            .build()?,
        SlashCommandBuilder::new("info", "Shows information")
            .subcommand(
                SlashCommandSubcommandBuilder::new("user", "About a user")
                    .option(SlashCommandUserOption::new("user", "Who (you by default)")),
            )
            .subcommand(SlashCommandSubcommandBuilder::new(
                "server",
                "About this server",
            ))
            .contexts([InteractionContextType::Guild])
            .build()?,
        SlashCommandBuilder::new("docs", "Links a section of the djs-rs documentation")
            .option(
                SlashCommandStringOption::new("topic", "The section")
                    .required(true)
                    .autocomplete(true),
            )
            .build()?,
        SlashCommandBuilder::new("feedback", "Sends feedback through a form").build()?,
        SlashCommandBuilder::new("slow", "Answers after a few seconds").build()?,
    ])
}

/// Registers the commands in the test guild once logged in (the application ID is known from
/// `READY` on), or removes them with `--cleanup`. `set` replaces all the bot's commands in that
/// guild.
fn register(client: &Client, guild: Option<GuildId>, cleanup: bool) -> Result<(), ValidationError> {
    let commands = Arc::new(commands()?);
    let handle = client.clone();
    client.on(Events::ClientReady, move |ctx, ready| {
        let commands = Arc::clone(&commands);
        let handle = handle.clone();
        async move {
            let guild_id = match guild {
                Some(guild_id) => guild_id,
                None => ready.guilds.first().ok_or("the bot is in no guild")?.id,
            };
            if cleanup {
                ctx.guild_commands(guild_id).set(&[]).await?;
                println!("Removed the bot's commands from guild {guild_id}.");
                handle.shutdown();
                return Ok(());
            }
            let registered = ctx.guild_commands(guild_id).set(&commands).await?;
            let names: Vec<_> = registered
                .values()
                .map(|command| format!("/{}", command.name))
                .collect();
            println!(
                "Logged in as {}; registered in guild {guild_id}: {}",
                ready.user.tag(),
                names.join(", ")
            );
            Ok(())
        }
    });
    Ok(())
}

/// `/ping`, `/echo` and `/slow`.
fn simple_commands(client: &Client) {
    client.command("ping", |ctx, cmd| async move {
        // An interaction ID is a snowflake: it tells when Discord created the interaction.
        let created = cmd.interaction().id.timestamp_ms();
        let now = u64::try_from(Timestamp::now().as_unix_millis())?;
        let latency = now.saturating_sub(created);
        cmd.reply(
            &ctx,
            MessageCallbackData::from(format!(
                "Pong! The command reached the bot in {latency} ms."
            ))
            .ephemeral(),
        )
        .await?;
        Ok(())
    });

    client.command("echo", |ctx, cmd| async move {
        let text = cmd.get_string("text").unwrap_or_default();
        let times = usize::try_from(cmd.get_integer("times").unwrap_or(1))?;
        // The text is the user's: never let it ping `@everyone`, roles or users.
        let mut reply = MessageCallbackData::from(vec![text; times].join("\n")).no_mentions();
        if cmd.get_boolean("private").unwrap_or(false) {
            reply = reply.ephemeral();
        }
        cmd.reply(&ctx, reply).await?;
        Ok(())
    });

    // Discord waits 3 seconds for the first answer; `defer_reply` buys 15 minutes.
    client.command("slow", |ctx, cmd| async move {
        cmd.defer_reply(&ctx, MessageFlags::empty()).await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        cmd.edit_reply(&ctx, "Done: this edits the \"thinking…\" message.")
            .await?;
        cmd.follow_up(&ctx, "And this is a follow-up message.")
            .await?;
        Ok(())
    });
}

/// `/info user` and `/info server`: subcommands have routes of their own.
fn info_commands(client: &Client) {
    client.command("info user", |ctx, cmd| async move {
        let user = cmd
            .get_user("user")
            .or_else(|| cmd.interaction().author())
            .ok_or("an interaction without a user")?;
        let created = time(user.id.timestamp()).style(TimestampStyle::RelativeTime);
        let embed = EmbedBuilder::new()
            .title(user.display_name())
            .description(user_mention(user.id).to_string())
            .inline_field("Username", user.tag())
            .inline_field("Bot", if user.bot { "yes" } else { "no" })
            .field("Account created", created.to_string())
            .footer(format!("ID {}", user.id))
            .build()?;
        cmd.reply(&ctx, MessageCallbackData::default().embed(embed))
            .await?;
        Ok(())
    });

    client.command("info server", |ctx, cmd| async move {
        let guild_id = cmd.interaction().guild_id.ok_or("not in a server")?;
        let guild = ctx.guilds().fetch(guild_id).await?;
        let created = time(guild.id.timestamp()).style(TimestampStyle::LongDate);
        let embed = EmbedBuilder::new()
            .title(guild.name.as_str())
            .inline_field("Owner", user_mention(guild.owner_id).to_string())
            .inline_field("Created", created.to_string())
            .inline_field("Features", guild.features.len().to_string())
            .footer(format!("ID {}", guild.id))
            .build()?;
        cmd.reply(&ctx, MessageCallbackData::default().embed(embed))
            .await?;
        Ok(())
    });
}

/// `/docs` with its autocomplete.
fn docs_command(client: &Client) {
    // Discord asks while the user types; answer within 3 seconds.
    client.autocomplete("docs", |ctx, search| async move {
        let typed = search.focused_value().unwrap_or_default().to_lowercase();
        let choices = TOPICS
            .iter()
            .filter(|(title, _)| title.to_lowercase().contains(&typed))
            .map(|&(title, anchor)| Choice::new(title, anchor.into()))
            .collect();
        search
            .respond(&ctx, AutocompleteChoices::String(choices))
            .await?;
        Ok(())
    });

    client.command("docs", |ctx, cmd| async move {
        // The value is the anchor of a suggestion, or whatever the user typed.
        let topic = cmd.get_string("topic").unwrap_or_default();
        let reply = match TOPICS.iter().find(|(_, anchor)| *anchor == topic) {
            Some((title, anchor)) => {
                MessageCallbackData::from(format!("{title}: <{README}#{anchor}>"))
            }
            None => MessageCallbackData::from(format!(
                "No section is called {topic:?}; pick a suggestion."
            ))
            .ephemeral(),
        };
        cmd.reply(&ctx, reply).await?;
        Ok(())
    });
}

/// `/feedback`: a modal, answered by a new interaction awaited right here.
fn feedback_command(client: &Client) {
    client.command("feedback", |ctx, cmd| async move {
        let form = ModalBuilder::new("feedback", "Feedback")
            .label("Topic", TextInputBuilder::short("topic").max_length(80))
            .label(
                "What would you change?",
                TextInputBuilder::paragraph("text").max_length(1000),
            )
            .build()?;
        cmd.show_modal(&ctx, form).await?;
        // A closed modal sends nothing, so the wait needs a time limit.
        let Ok(submitted) = cmd
            .await_modal_submit(&ctx, Duration::from_secs(300))
            .filter(|modal| modal.custom_id == "feedback")
            .await
        else {
            return Ok(());
        };
        let topic = submitted.text_value("topic").unwrap_or_default();
        let text = submitted.text_value("text").unwrap_or_default();
        println!("Feedback about {topic:?}: {text:?}");
        submitted
            .reply(
                &ctx,
                MessageCallbackData::from(format!("Thanks for your feedback about {topic:?}!"))
                    .ephemeral(),
            )
            .await?;
        Ok(())
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    common::init_tracing();
    let guild = common::test_guild()?;
    let cleanup = common::cleanup_requested();

    // No intent is needed for interactions; `GUILDS` keeps the guild list.
    let client = Client::builder(common::token()?)
        .intents(Intents::GUILDS)
        .build();
    register(&client, guild, cleanup)?;
    if !cleanup {
        simple_commands(&client);
        info_commands(&client);
        docs_command(&client);
        feedback_command(&client);
    }

    common::shutdown_on_ctrl_c(&client);
    client.login().await?;
    Ok(())
}
