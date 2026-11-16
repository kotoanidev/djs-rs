//! The smallest twilight bot: answers `!ping` with `Pong!`. The same bot as
//! `benches/vs-serenity/bots/djs-rs`, written the way twilight's README writes one.

use std::{env, error::Error, sync::Arc};

use twilight_cache_inmemory::{DefaultInMemoryCache, ResourceType};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_http::Client as HttpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;
    let intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES | Intents::MESSAGE_CONTENT;
    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);
    let http = Arc::new(HttpClient::new(token));
    let cache = DefaultInMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(event) => event,
            Err(error) => {
                eprintln!("could not receive an event: {error}");
                continue;
            }
        };
        cache.update(&event);
        tokio::spawn(handle_event(event, Arc::clone(&http)));
    }
    Ok(())
}

async fn handle_event(event: Event, http: Arc<HttpClient>) {
    if let Event::MessageCreate(message) = event
        && message.content == "!ping"
        && let Err(error) = http
            .create_message(message.channel_id)
            .content("Pong!")
            .reply(message.id)
            .await
    {
        eprintln!("could not reply: {error}");
    }
}
