//! A moderation bot: a log channel for joins, leaves, bans, edits and deletions (with the
//! previous state from the cache), and slash commands limited by Discord to moderators.
//!
//! | Feature | Shows |
//! |---|---|
//! | welcome and leave messages | `GuildMemberAdd`, `GuildMemberRemove` with the cached member |
//! | edit and deletion logs | `MessageUpdate` / `MessageDelete` / `MessageBulkDelete` with `old` |
//! | ban log | `GuildBanAdd` (the `GUILD_MODERATION` intent) |
//! | `/timeout user minutes [reason]` | `MemberManager::timeout` (0 minutes removes it) |
//! | `/ban user [reason] [delete_messages]` | `MemberManager::ban` |
//! | `/purge count` | listing and bulk-deleting messages |
//! | `/members` | `fetch_all` through the gateway, filling the cache |
//!
//! The commands use `default_member_permissions`, so Discord only offers them to members with
//! the permission (server admins can change that in the server's settings). A real bot would
//! also check that the moderator ranks above the target; Discord only checks the bot's rank.
//!
//! # Running
//!
//! 1. Enable the **Server Members** and **Message Content** privileged intents in the
//!    [Developer Portal](https://discord.com/developers/applications). The bot needs the
//!    Moderate Members, Ban Members, Manage Messages and View Audit Log permissions for
//!    everything to work.
//! 2. Put `DISCORD_TOKEN` and `DISCORD_TEST_CHANNEL_ID` (the log channel) in `.env`;
//!    `DISCORD_TEST_GUILD_ID` chooses the server of the commands (the first one otherwise).
//! 3. Run `cargo run --example moderation`. `cargo run --example moderation -- --cleanup`
//!    removes the commands again.
//!
//! discord.js counterpart: the "moderation" and "events" sections of the discord.js guide.

use std::time::Instant;

use djs_rs::{
    builders::{
        EmbedBuilder, SlashCommandBuilder, SlashCommandIntegerOption, SlashCommandStringOption,
        SlashCommandUserOption, ValidationError,
    },
    cache::{CacheConfig, Limit},
    formatters::{TimestampStyle, channel_mention, escape_markdown, time, user_mention},
    model::{
        id::{ChannelId, GuildId},
        interaction::{InteractionContextType, MessageCallbackData},
        message::{AllowedMentions, MessageFlags},
        permissions::Permissions,
        rest::{CreateApplicationCommand, CreateGuildBan, CreateMessage, GetChannelMessages},
        util::Timestamp,
    },
    prelude::*,
};

#[path = "common/mod.rs"]
mod common;

/// The longest timeout Discord allows: 28 days.
const MAX_TIMEOUT_MINUTES: i64 = 28 * 24 * 60;

/// Discord only bulk-deletes messages younger than 14 days.
const BULK_DELETE_AGE_MS: u64 = 14 * 24 * 60 * 60 * 1000;

fn commands() -> Result<Vec<CreateApplicationCommand>, ValidationError> {
    Ok(vec![
        SlashCommandBuilder::new("timeout", "Times a member out (0 minutes ends a timeout)")
            .option(SlashCommandUserOption::new("user", "The member").required(true))
            .option(
                SlashCommandIntegerOption::new("minutes", "For how long")
                    .required(true)
                    .min_value(0)
                    .max_value(MAX_TIMEOUT_MINUTES),
            )
            .option(
                SlashCommandStringOption::new("reason", "Shown in the audit log").max_length(400),
            )
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .contexts([InteractionContextType::Guild])
            .build()?,
        SlashCommandBuilder::new("ban", "Bans a user")
            .option(SlashCommandUserOption::new("user", "The user").required(true))
            .option(
                SlashCommandStringOption::new("reason", "Shown in the audit log").max_length(400),
            )
            .option(
                SlashCommandIntegerOption::new("delete_messages", "Delete their recent messages")
                    .choice("None", 0)
                    .choice("The last hour", 3600)
                    .choice("The last day", 86_400)
                    .choice("The last week", 604_800),
            )
            .default_member_permissions(Permissions::BAN_MEMBERS)
            .contexts([InteractionContextType::Guild])
            .build()?,
        SlashCommandBuilder::new("purge", "Deletes the latest messages of this channel")
            .option(
                SlashCommandIntegerOption::new("count", "How many")
                    .required(true)
                    .min_value(1)
                    .max_value(100),
            )
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .contexts([InteractionContextType::Guild])
            .build()?,
        SlashCommandBuilder::new("members", "Fetches every member of the server")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .contexts([InteractionContextType::Guild])
            .build()?,
    ])
}

/// At most `max` characters of `text`, escaped, for an embed field.
fn quote(text: &str, max: usize) -> String {
    if text.is_empty() {
        return "*(no text)*".to_owned();
    }
    let clipped: String = text.chars().take(max).collect();
    let ellipsis = if clipped.len() < text.len() {
        "…"
    } else {
        ""
    };
    format!("{}{ellipsis}", escape_markdown(&clipped))
}

/// Sends a log line (and an optional embed) to the log channel, without pinging anyone.
async fn log(
    ctx: &Context,
    channel: ChannelId,
    text: String,
    embed: Option<EmbedBuilder>,
) -> Result<(), HandlerError> {
    let mut body = CreateMessage::from(text).no_mentions();
    if let Some(embed) = embed {
        body = body.embed(embed.build()?);
    }
    ctx.messages(channel).send(body).await?;
    Ok(())
}

/// Registers the logging handlers.
fn logs(client: &Client, channel: ChannelId) {
    client.on(Events::GuildMemberAdd, move |ctx, member| async move {
        let created = time(member.user.id.timestamp()).style(TimestampStyle::RelativeTime);
        let count = ctx
            .cache()
            .map_or(0, |cache| cache.members(member.guild_id).len());
        // This message may ping the new member, and only them.
        let mut mentions = AllowedMentions::default();
        mentions.users = vec![member.user.id];
        let body = CreateMessage::from(format!(
            "👋 Welcome {}! (account created {created}; {count} members cached)",
            user_mention(member.user.id)
        ))
        .allowed_mentions(mentions);
        ctx.messages(channel).send(body).await?;
        Ok(())
    });

    client.on(Events::GuildMemberRemove, move |ctx, removed| async move {
        // `old` is the member as the cache held it: when they joined, their nickname, …
        let joined = removed
            .old
            .as_ref()
            .and_then(|member| member.joined_at)
            .map_or_else(
                || "at an unknown time".to_owned(),
                |at| time(at).style(TimestampStyle::RelativeTime).to_string(),
            );
        log(
            &ctx,
            channel,
            format!("🚪 {} left (joined {joined}).", removed.user.tag()),
            None,
        )
        .await
    });

    client.on(Events::GuildBanAdd, move |ctx, ban| async move {
        log(
            &ctx,
            channel,
            format!("🔨 {} was banned.", ban.user.tag()),
            None,
        )
        .await
    });

    client.on(Events::MessageUpdate, move |ctx, update| async move {
        // Without the old message in the cache there is nothing to compare; embeds being
        // added to a link also send an update with the same text.
        let Some(old) = &update.old else {
            return Ok(());
        };
        if update.author.bot || old.content == update.content {
            return Ok(());
        }
        let embed = EmbedBuilder::new()
            .field("Before", quote(&old.content, 1000))
            .field("After", quote(&update.content, 1000));
        let text = format!(
            "✏️ {} edited a message in {}.",
            update.author.tag(),
            channel_mention(update.channel_id)
        );
        log(&ctx, channel, text, Some(embed)).await
    });

    client.on(Events::MessageDelete, move |ctx, deleted| async move {
        let text = match &deleted.old {
            Some(old) if old.author.bot => return Ok(()),
            Some(old) => format!(
                "🗑️ A message by {} was deleted in {}.",
                old.author.tag(),
                channel_mention(old.channel_id)
            ),
            None => format!(
                "🗑️ A message (not in the cache) was deleted in {}.",
                channel_mention(deleted.channel_id)
            ),
        };
        let embed = deleted
            .old
            .as_ref()
            .map(|old| EmbedBuilder::new().description(quote(&old.content, 4000)));
        log(&ctx, channel, text, embed).await
    });

    client.on(Events::MessageBulkDelete, move |ctx, bulk| async move {
        let text = format!(
            "🗑️ {} messages were deleted at once in {} ({} of them were cached).",
            bulk.new.ids.len(),
            channel_mention(bulk.new.channel_id),
            bulk.old.len()
        );
        log(&ctx, channel, text, None).await
    });
}

/// Registers the moderation commands' handlers.
fn moderation(client: &Client) {
    client.command("timeout", |ctx, cmd| async move {
        let guild_id = cmd.interaction().guild_id.ok_or("not in a server")?;
        let user = cmd.get_user("user").ok_or("missing user")?;
        let minutes = cmd.get_integer("minutes").unwrap_or(0);
        let reason = cmd.get_string("reason");
        let until = if minutes == 0 {
            None
        } else {
            let end = Timestamp::now().as_unix_secs() + minutes * 60;
            Some(Timestamp::from_unix_secs(end).ok_or("the time is out of range")?)
        };
        let text = match ctx.members(guild_id).timeout(user.id, until, reason).await {
            Ok(_) => match until {
                Some(until) => format!(
                    "{} is timed out, until {}.",
                    user_mention(user.id),
                    time(until).style(TimestampStyle::RelativeTime)
                ),
                None => format!("{}'s timeout is over.", user_mention(user.id)),
            },
            // Missing permissions, a member ranked above the bot, the server owner, …
            Err(error) => format!("Could not time {} out: {error}", user.tag()),
        };
        // Only the moderator sees the answer.
        cmd.reply(&ctx, MessageCallbackData::from(text).ephemeral())
            .await?;
        Ok(())
    });

    client.command("ban", |ctx, cmd| async move {
        let guild_id = cmd.interaction().guild_id.ok_or("not in a server")?;
        let user = cmd.get_user("user").ok_or("missing user")?;
        let mut ban = CreateGuildBan::default();
        ban.delete_message_seconds = cmd
            .get_integer("delete_messages")
            .map(u32::try_from)
            .transpose()?;
        let reason = cmd.get_string("reason");
        let text = match ctx.members(guild_id).ban(user.id, &ban, reason).await {
            Ok(()) => format!("{} is banned.", user.tag()),
            Err(error) => format!("Could not ban {}: {error}", user.tag()),
        };
        // Only the moderator sees the answer.
        cmd.reply(&ctx, MessageCallbackData::from(text).ephemeral())
            .await?;
        Ok(())
    });

    client.command("purge", |ctx, cmd| async move {
        let channel = cmd.interaction().channel_id.ok_or("not in a channel")?;
        let count = u8::try_from(cmd.get_integer("count").unwrap_or(1))?;
        // Listing and deleting can take longer than the 3 seconds of a first answer.
        cmd.defer_reply(&ctx, MessageFlags::EPHEMERAL).await?;
        let mut query = GetChannelMessages::default();
        query.limit = Some(count);
        let messages = ctx.messages(channel).list(&query).await?;
        let now = u64::try_from(Timestamp::now().as_unix_millis())?;
        let ids: Vec<_> = messages
            .keys()
            .copied()
            .filter(|id| now.saturating_sub(id.timestamp_ms()) < BULK_DELETE_AGE_MS)
            .collect();
        let deleted = ids.len();
        match ids.as_slice() {
            [] => {}
            // Bulk deletion takes 2 to 100 messages.
            [id] => ctx.messages(channel).delete(*id).await?,
            _ => {
                ctx.messages(channel)
                    .bulk_delete(ids, Some("/purge"))
                    .await?;
            }
        }
        let skipped = messages.len() - deleted;
        cmd.edit_reply(
            &ctx,
            format!("Deleted {deleted} messages ({skipped} older than 14 days were kept)."),
        )
        .await?;
        Ok(())
    });

    client.command("members", |ctx, cmd| async move {
        let guild_id = cmd.interaction().guild_id.ok_or("not in a server")?;
        cmd.defer_reply(&ctx, MessageFlags::EPHEMERAL).await?;
        let started = Instant::now();
        // Asked through the gateway, answered in chunks of 1 000 members, all cached.
        let members = ctx.members(guild_id).fetch_all().await?;
        let bots = members.values().filter(|member| member.user.bot).count();
        cmd.edit_reply(
            &ctx,
            format!(
                "{} members ({bots} bots), fetched in {:.0?}.",
                members.len(),
                started.elapsed()
            ),
        )
        .await?;
        Ok(())
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    common::init_tracing();
    let channel: ChannelId = common::config_value("DISCORD_TEST_CHANNEL_ID")
        .ok_or("DISCORD_TEST_CHANNEL_ID (the log channel) is not set")?
        .parse()?;
    let guild: Option<GuildId> = common::test_guild()?;
    let cleanup = common::cleanup_requested();
    let commands = std::sync::Arc::new(commands()?);

    let client = Client::builder(common::token()?)
        .intents(
            Intents::GUILDS
                | Intents::GUILD_MEMBERS
                | Intents::GUILD_MODERATION
                | Intents::GUILD_MESSAGES
                | Intents::MESSAGE_CONTENT,
        )
        // Members for the welcome count and the leave log, the latest 200 messages of each
        // channel for the edit and deletion logs.
        .cache(
            CacheConfig::builder()
                .members(Limit::Unlimited)
                .users(Limit::Unlimited)
                .messages(Limit::Max(200))
                .build(),
        )
        .build();

    let handle = client.clone();
    client.on(Events::ClientReady, move |ctx, ready| {
        let commands = std::sync::Arc::clone(&commands);
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
            ctx.guild_commands(guild_id).set(&commands).await?;
            println!(
                "Logged in as {}; logging to channel {channel}, commands in guild {guild_id}.",
                ready.user.tag()
            );
            Ok(())
        }
    });

    if !cleanup {
        logs(&client, channel);
        moderation(&client);
    }

    common::shutdown_on_ctrl_c(&client);
    client.login().await?;
    Ok(())
}
