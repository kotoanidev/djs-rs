//! The guide's "Slash commands" chapter: commands registered in a test server, with
//! options, subcommands, ephemeral and deferred replies, and autocomplete.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example commands`;
//! `DISCORD_GUILD_ID` chooses the server (otherwise the first one the bot is in), and
//! `-- --remove` removes the commands again.

use std::{sync::Arc, time::Duration};

use djs_rs::{
    builders::{
        SlashCommandBooleanOption, SlashCommandBuilder, SlashCommandIntegerOption,
        SlashCommandStringOption, SlashCommandSubcommandBuilder, SlashCommandUserOption,
        ValidationError,
    },
    model::{
        application_command::Choice,
        id::GuildId,
        interaction::{AutocompleteChoices, MessageCallbackData},
        message::MessageFlags,
        rest::CreateApplicationCommand,
    },
    prelude::*,
};

/// The fruits `/fruit` suggests.
const FRUITS: &[&str] = &[
    "apple",
    "apricot",
    "banana",
    "blueberry",
    "cherry",
    "grape",
    "kiwi",
    "lemon",
    "mango",
    "orange",
    "peach",
    "pear",
    "plum",
    "raspberry",
    "strawberry",
];

// ANCHOR: define
/// The bot's commands. `build` checks Discord's rules (names, lengths, the order of the
/// options) before anything is sent.
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
            .build()?,
        SlashCommandBuilder::new("fruit", "Picks a fruit")
            .option(
                SlashCommandStringOption::new("name", "Which one")
                    .required(true)
                    .autocomplete(true),
            )
            .build()?,
        SlashCommandBuilder::new("slow", "Answers after a few seconds").build()?,
    ])
}
// ANCHOR_END: define

// ANCHOR: register
/// Registers the commands in the test server once the bot is ready (or removes them).
fn register(client: &Client, guild: Option<GuildId>, remove: bool) -> Result<(), ValidationError> {
    let commands = Arc::new(commands()?);
    client.on(Events::ClientReady, move |ctx, ready| {
        let commands = Arc::clone(&commands);
        async move {
            let guild_id = match guild {
                Some(guild_id) => guild_id,
                None => ready.guilds.first().ok_or("the bot is in no server")?.id,
            };
            // `set` replaces all the bot's commands in that server with these.
            let commands: &[CreateApplicationCommand] = if remove { &[] } else { &commands };
            let registered = ctx.guild_commands(guild_id).set(commands).await?;
            println!("{} command(s) in server {guild_id}", registered.len());
            if remove {
                ctx.shutdown();
            }
            Ok(())
        }
    });
    Ok(())
}
// ANCHOR_END: register

fn routes(client: &Client) {
    // ANCHOR: ping
    client.command("ping", |ctx, cmd| async move {
        cmd.reply(&ctx, "Pong!").await?;
        Ok(())
    });
    // ANCHOR_END: ping

    // ANCHOR: echo
    client.command("echo", |ctx, cmd| async move {
        // Required options are always there; the others may be missing.
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
    // ANCHOR_END: echo

    // ANCHOR: subcommands
    client.command("info user", |ctx, cmd| async move {
        let user = cmd
            .get_user("user")
            .or_else(|| cmd.interaction().author())
            .ok_or("an interaction without a user")?;
        let answer = format!("{} (ID {}), a bot: {}", user.tag(), user.id, user.bot);
        cmd.reply(&ctx, answer).await?;
        Ok(())
    });

    client.command("info server", |ctx, cmd| async move {
        let Some(guild_id) = cmd.interaction().guild_id else {
            let reply = MessageCallbackData::from("This only works in a server.").ephemeral();
            cmd.reply(&ctx, reply).await?;
            return Ok(());
        };
        let guild = ctx.guilds().fetch(guild_id).await?;
        cmd.reply(&ctx, format!("{} (ID {})", guild.name, guild.id))
            .await?;
        Ok(())
    });
    // ANCHOR_END: subcommands

    // ANCHOR: autocomplete
    // Discord asks while the user types; the answer must come within 3 seconds.
    client.autocomplete("fruit", |ctx, search| async move {
        let typed = search.focused_value().unwrap_or_default().to_lowercase();
        let choices = FRUITS
            .iter()
            .filter(|fruit| fruit.contains(typed.as_str()))
            .take(25)
            .map(|&fruit| Choice::new(fruit, fruit.into()))
            .collect();
        search
            .respond(&ctx, AutocompleteChoices::String(choices))
            .await?;
        Ok(())
    });

    client.command("fruit", |ctx, cmd| async move {
        // A suggestion's value, or whatever the user typed instead.
        let name = cmd.get_string("name").unwrap_or_default();
        let reply = if FRUITS.contains(&name) {
            MessageCallbackData::from(format!("You picked {name}."))
        } else {
            MessageCallbackData::from(format!("I know no fruit called {name:?}.")).ephemeral()
        };
        cmd.reply(&ctx, reply).await?;
        Ok(())
    });
    // ANCHOR_END: autocomplete

    // ANCHOR: defer
    // Discord waits 3 seconds for the first answer; `defer_reply` gives 15 minutes.
    client.command("slow", |ctx, cmd| async move {
        cmd.defer_reply(&ctx, MessageFlags::empty()).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        cmd.edit_reply(&ctx, "Done: this replaces the \"thinking…\" message.")
            .await?;
        cmd.follow_up(&ctx, "And this is one more message.").await?;
        Ok(())
    });
    // ANCHOR_END: defer
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;
    let guild = std::env::var("DISCORD_GUILD_ID")
        .ok()
        .map(|id| id.parse::<GuildId>())
        .transpose()?;
    let remove = std::env::args().any(|argument| argument == "--remove");

    // ANCHOR: client
    // Interactions come whatever the intents are; `GUILDS` gives the bot its servers.
    let client = Client::builder(token).intents(Intents::GUILDS).build();
    register(&client, guild, remove)?;
    if !remove {
        routes(&client);
    }
    // ANCHOR_END: client

    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    client.login().await?;
    Ok(())
}
