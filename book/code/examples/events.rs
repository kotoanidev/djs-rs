//! The guide's "Events and intents" chapter: a bot that logs members joining, edited
//! messages (with their text before the edit) and reactions, and answers `!cached`.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example events`. It needs
//! the **Server Members** and **Message Content** intents turned on in the Developer Portal.

use djs_rs::{
    cache::{CacheConfig, Limit},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;

    // ANCHOR: client
    let intents = Intents::GUILDS
        | Intents::GUILD_MEMBERS
        | Intents::GUILD_MESSAGES
        | Intents::MESSAGE_CONTENT
        | Intents::GUILD_MESSAGE_REACTIONS;
    let client = Client::builder(token)
        .intents(intents)
        // Keep the last 200 messages of each channel, so an edit can show the text before.
        .cache(CacheConfig::builder().messages(Limit::Max(200)).build())
        .build();
    // ANCHOR_END: client

    // ANCHOR: member
    client.on(Events::GuildMemberAdd, |_ctx, joined| async move {
        println!(
            "{} joined guild {}",
            joined.member.user.tag(),
            joined.guild_id
        );
        Ok(())
    });
    // ANCHOR_END: member

    // ANCHOR: update
    client.on(Events::MessageUpdate, |_ctx, update| async move {
        // `old` is the message as the cache held it before the edit, if it held it.
        let before = update
            .old
            .as_ref()
            .map_or("(not in the cache)", |old| old.content.as_str());
        // The payload itself is the message after the edit.
        println!("edited: {before:?} -> {:?}", update.content);
        Ok(())
    });
    // ANCHOR_END: update

    // ANCHOR: reaction
    client.on(Events::MessageReactionAdd, |_ctx, reaction| async move {
        let emoji = reaction.emoji.name().unwrap_or("an emoji");
        println!(
            "user {} reacted with {emoji} to message {}",
            reaction.user_id, reaction.message_id
        );
        Ok(())
    });
    // ANCHOR_END: reaction

    // ANCHOR: context
    client.on(Events::MessageCreate, |ctx, msg| async move {
        if msg.content == "!cached" {
            // The context reaches the cache, the REST client and the event's shard.
            let cached = ctx
                .cache()
                .map_or(0, |cache| cache.messages(msg.channel_id).len());
            let answer = format!(
                "I remember {cached} messages of this channel (shard {}).",
                ctx.shard_id()
            );
            msg.reply(&ctx, answer).await?;
        }
        Ok(())
    });
    // ANCHOR_END: context

    // ANCHOR: check
    // Does every event a handler listens to come with one of the client's intents?
    let missing = client.required_intents().difference(intents);
    if !missing.is_empty() {
        eprintln!("some handlers' events need intents the client does not ask for: {missing:?}");
    }
    // ANCHOR_END: check

    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    client.login().await?;
    Ok(())
}
