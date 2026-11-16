//! The guide's "Components and embeds" chapter: an embed with buttons and a select menu,
//! routed by custom ID, and a modal.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example buttons_and_embeds`;
//! `DISCORD_GUILD_ID` chooses the server (otherwise the first one the bot is in), and
//! `-- --remove` removes the commands again.

use std::sync::Arc;

use djs_rs::{
    builders::{
        ActionRowBuilder, ButtonBuilder, EmbedBuilder, ModalBuilder, SlashCommandBuilder,
        StringSelectMenuBuilder, TextInputBuilder, ValidationError,
    },
    model::{
        component::Component, id::GuildId, interaction::MessageCallbackData, message::Embed,
        rest::CreateApplicationCommand,
    },
    prelude::*,
};

// ANCHOR: card
/// The card `/card` shows: an embed, a row of buttons and a row with a select menu. The
/// like button carries its count in its custom ID (`like:3`), so the bot keeps no state.
fn card(likes: u32) -> Result<(Embed, Vec<Component>), ValidationError> {
    let embed = EmbedBuilder::new()
        .title("djs-rs")
        .url("https://github.com/kotoanidev/djs-rs")
        .description("Discord bots in Rust, the discord.js way.")
        .color(0x00_5f_87)
        .inline_field("Likes", likes.to_string())
        .inline_field("Language", "Rust")
        .footer("Click a button or pick a flavor")
        .timestamp_now()
        .build()?;
    let buttons = ActionRowBuilder::new()
        .component(ButtonBuilder::primary(format!("like:{likes}")).label("Like"))
        .component(ButtonBuilder::secondary("form").label("Tell us more"))
        .component(ButtonBuilder::link("https://kotoanidev.github.io/djs-rs/").label("Guide"))
        .build()?;
    let menu = ActionRowBuilder::new()
        .component(
            StringSelectMenuBuilder::new("flavor")
                .placeholder("Your favourite flavor")
                .option("Vanilla", "vanilla")
                .option("Chocolate", "chocolate")
                .option("Strawberry", "strawberry"),
        )
        .build()?;
    Ok((embed, vec![buttons, menu]))
}
// ANCHOR_END: card

fn routes(client: &Client) {
    // ANCHOR: reply
    client.command("card", |ctx, cmd| async move {
        let (embed, components) = card(0)?;
        let reply = MessageCallbackData::default()
            .embeds([embed])
            .components(components);
        cmd.reply(&ctx, reply).await?;
        Ok(())
    });
    // ANCHOR_END: reply

    // ANCHOR: buttons
    // `like:3` reaches the route `like` with the arguments `3`.
    client.component("like", |ctx, button| async move {
        let likes: u32 = button.args().parse()?;
        let (embed, components) = card(likes + 1)?;
        // `update` edits the message the button is on, and answers the click.
        let update = MessageCallbackData::default()
            .embeds([embed])
            .components(components);
        button.update(&ctx, update).await?;
        Ok(())
    });
    // ANCHOR_END: buttons

    // ANCHOR: select
    client.component("flavor", |ctx, menu| async move {
        let picked = menu.values.first().map_or("nothing", |value| value.as_str());
        let reply = MessageCallbackData::from(format!("You picked {picked}.")).ephemeral();
        menu.reply(&ctx, reply).await?;
        Ok(())
    });
    // ANCHOR_END: select

    // ANCHOR: show_modal
    client.component("form", |ctx, button| async move {
        let form = ModalBuilder::new("about", "About you")
            .label("Your name", TextInputBuilder::short("name").max_length(80))
            .label(
                "What are you building?",
                TextInputBuilder::paragraph("project")
                    .max_length(1000)
                    .required(false),
            )
            .build()?;
        // A modal must be the first answer to the click.
        button.show_modal(&ctx, form).await?;
        Ok(())
    });

    client.modal("about", |ctx, modal| async move {
        let name = modal.text_value("name").unwrap_or_default();
        let project = modal.text_value("project").unwrap_or_default();
        let answer = if project.is_empty() {
            format!("Nice to meet you, {name}!")
        } else {
            format!("Nice to meet you, {name}! Good luck with {project}.")
        };
        // The text is the user's: no pings.
        let reply = MessageCallbackData::from(answer).no_mentions().ephemeral();
        modal.reply(&ctx, reply).await?;
        Ok(())
    });
    // ANCHOR_END: show_modal
}

/// Registers `/card` in the test server once the bot is ready (or removes it).
fn register(client: &Client, guild: Option<GuildId>, remove: bool) -> Result<(), ValidationError> {
    let commands: Arc<[CreateApplicationCommand]> = if remove {
        Arc::new([])
    } else {
        Arc::new([SlashCommandBuilder::new("card", "Shows a card with buttons").build()?])
    };
    client.on(Events::ClientReady, move |ctx, ready| {
        let commands = Arc::clone(&commands);
        async move {
            let guild_id = match guild {
                Some(guild_id) => guild_id,
                None => ready.guilds.first().ok_or("the bot is in no server")?.id,
            };
            ctx.guild_commands(guild_id).set(&commands).await?;
            if remove {
                ctx.shutdown();
            }
            Ok(())
        }
    });
    Ok(())
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

    let client = Client::builder(token).intents(Intents::GUILDS).build();
    register(&client, guild, remove)?;
    if !remove {
        routes(&client);
    }

    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    client.login().await?;
    Ok(())
}
