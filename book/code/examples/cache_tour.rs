//! The guide's "The cache" chapter: what to cache, reading it, the state before an event,
//! fetching through it, and sweeping it.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example cache_tour`. It
//! needs the Server Members and Message Content intents (Developer Portal → Bot); send
//! `!cache` in a server channel, then edit and delete a message.

use std::time::Duration;

use djs_rs::{
    cache::{CacheConfig, Limit, Sweep},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;

    // ANCHOR: config
    // The default cache keeps guilds, channels and roles; add what the bot reads.
    let cache = CacheConfig::builder()
        .members(Limit::Unlimited)
        .users(Limit::Unlimited)
        // The latest 100 messages of each channel.
        .messages(Limit::Max(100))
        // Every 10 minutes, forget messages older than an hour.
        .sweep_messages(Some(Sweep::new(
            Duration::from_secs(10 * 60),
            Duration::from_secs(60 * 60),
        )))
        .build();
    // What the cache can fill depends on the intents: members need GUILD_MEMBERS.
    let client = Client::builder(token)
        .intents(
            Intents::GUILDS
                | Intents::GUILD_MEMBERS
                | Intents::GUILD_MESSAGES
                | Intents::MESSAGE_CONTENT,
        )
        .cache(cache)
        .build();
    // ANCHOR_END: config

    // ANCHOR: read
    client.on(Events::MessageCreate, |ctx, msg| async move {
        let (Some(guild_id), "!cache") = (msg.guild_id, msg.content.trim()) else {
            return Ok(());
        };
        let cache = ctx.cache().ok_or("the client has a cache")?;
        let guild = cache.guilds().get(guild_id).ok_or("a guild of the bot")?;
        let answer = format!(
            "{}: {} members on Discord, {} in the cache; {} users; {} messages of this channel",
            guild.name,
            guild.member_count,
            cache.members(guild_id).len(),
            cache.users().len(),
            cache.messages(msg.channel_id).len(),
        );
        msg.reply(&ctx, answer).await?;
        Ok(())
    });
    // ANCHOR_END: read

    // ANCHOR: old
    // `old` is what the cache held before the event; `None` if it did not hold it.
    client.on(Events::MessageUpdate, |_ctx, update| async move {
        if let Some(old) = &update.old {
            println!("edited: {:?} -> {:?}", old.content, update.content);
        }
        Ok(())
    });

    client.on(Events::MessageDelete, |_ctx, deleted| async move {
        match &deleted.old {
            Some(old) => println!("deleted: {:?} by {}", old.content, old.author.tag()),
            None => println!("deleted message {}, never cached", deleted.id),
        }
        Ok(())
    });
    // ANCHOR_END: old

    // ANCHOR: fetch
    client.on(Events::GuildMemberAdd, |ctx, member| async move {
        let members = ctx.members(member.guild_id);
        // The member was cached from this very event: no request.
        let cached = members.fetch(member.user.id);
        println!("cached: {}", cached.is_cached());
        // `force` asks Discord anyway, and caches the answer.
        let fresh = members.fetch(member.user.id).force().await?;
        println!("{} joined", fresh.display_name());
        Ok(())
    });
    // ANCHOR_END: fetch

    // ANCHOR: sweep
    client.on(Events::ClientReady, |ctx, _ready| async move {
        let cache = ctx.cache().ok_or("the client has a cache")?;
        // A sweep of its own: forget the bots among the users.
        let removed = cache.sweep_users(|user| user.bot);
        println!("{removed} bot user(s) swept");
        Ok(())
    });
    // ANCHOR_END: sweep

    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    client.login().await?;
    Ok(())
}
