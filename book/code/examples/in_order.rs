//! The guide's "Events and intents" chapter: handlers that run one after another, in the
//! order of the events, and a stream of every event next to them.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example in_order`.

use djs_rs::{HandlerMode, event::Dispatch, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;

    // ANCHOR: sequential
    let client = Client::builder(token)
        .intents(Intents::GUILDS | Intents::GUILD_MESSAGES)
        // Each shard calls its handlers one at a time, in the order of its events.
        .handler_mode(HandlerMode::Sequential)
        .build();

    client.on(Events::MessageCreate, |_ctx, msg| async move {
        println!("created {}", msg.id);
        Ok(())
    });
    client.on(Events::MessageDelete, |_ctx, deleted| async move {
        // Never printed before the "created" line of the same message.
        println!("deleted {}", deleted.id);
        Ok(())
    });
    // ANCHOR_END: sequential

    // ANCHOR: stream
    let mut events = client.events();
    tokio::spawn(async move {
        let mut messages = 0_u64;
        while let Some(event) = events.next().await {
            if let Dispatch::MessageCreate(_) = event.event {
                messages += 1;
                if messages.is_multiple_of(100) {
                    println!(
                        "{messages} messages so far (the last on shard {})",
                        event.shard
                    );
                }
            }
        }
    });
    // ANCHOR_END: stream

    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    client.login().await?;
    Ok(())
}
