//! The guide's "The REST API" chapter: the REST client on its own, without a gateway
//! connection: sending, editing, listing page by page, deleting, and Discord's errors.
//!
//! Run it with `DISCORD_TOKEN=<token> DISCORD_CHANNEL_ID=<id> cargo run -p djs-rs-guide
//! --example rest_calls`, with the ID of a channel the bot can write in (Developer Mode →
//! right-click the channel → Copy Channel ID). It sends one message and deletes it again.

use std::time::Duration;

use djs_rs::{
    model::{
        id::{ChannelId, MessageId},
        rest::{CreateMessage, EditMessage, ErrorCode, GetChannelMessages},
    },
    rest::Rest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;
    let channel: ChannelId = std::env::var("DISCORD_CHANNEL_ID")?.parse()?;

    // ANCHOR: client
    // A client of its own; in a bot, `ctx.rest()` is the client's one.
    let rest = Rest::builder()
        .token(token)
        // Each attempt gets 10 seconds, and a failed one is tried twice more.
        .timeout(Duration::from_secs(10))
        .retries(2)
        .build();
    // ANCHOR_END: client

    // ANCHOR: send
    let sent = rest
        .create_message(channel, &CreateMessage::from("Hello from djs-rest!"))
        .await?;
    let edit = EditMessage::default().content("Hello again, edited.");
    let edited = rest.edit_message(channel, sent.id, &edit).await?;
    println!("sent and edited message {}: {:?}", edited.id, edited.content);
    // ANCHOR_END: send

    // ANCHOR: pages
    // Up to 100 messages per request; `before` asks for the page before the last one seen.
    let mut query = GetChannelMessages::default();
    query.limit = Some(100);
    let mut seen = 0;
    let mut oldest: Option<MessageId> = None;
    for _ in 0..3 {
        query.before = oldest;
        let page = rest.get_channel_messages(channel, &query).await?;
        seen += page.len();
        // Newest first: the page's last message is its oldest.
        match page.last() {
            Some(message) if page.len() == 100 => oldest = Some(message.id),
            _ => break,
        }
    }
    println!("{seen} message(s) in the channel's latest pages");
    // ANCHOR_END: pages

    // ANCHOR: errors
    rest.delete_message(channel, sent.id).await?;
    // Deleting it again fails: Discord answers 404 with its error code 10008.
    match rest.delete_message(channel, sent.id).await {
        Ok(()) => println!("deleted twice?"),
        Err(error)
            if error
                .api_error()
                .is_some_and(|api| api.code == ErrorCode::UnknownMessage) =>
        {
            println!("already gone ({})", error.status().map_or(0, |status| status.as_u16()));
        }
        Err(error) => return Err(error.into()),
    }
    // ANCHOR_END: errors
    Ok(())
}
