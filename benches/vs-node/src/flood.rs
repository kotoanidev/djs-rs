//! Scenario 2: an end-to-end `MESSAGE_CREATE` flood through the mock gateway.
//!
//! The measured window runs from the first counted event to the last one, so connecting,
//! `READY` and the `GUILD_CREATE` before the flood are not part of it.

use std::{hint::black_box, sync::Arc};

use crate::{Options, TOKEN, metrics::Window, serenity_cache};
use djs_gateway::{Compression, Config, Shard};
use djs_model::gateway::{DispatchEvent, GatewayEvent, Intents, ShardId};
use djs_rs::{Client, Events, HandlerMode, Shards, cache::CacheConfig};

const INTENTS: Intents = Intents::GUILDS
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::MESSAGE_CONTENT);

/// A djs-rs [`Client`] with one `MESSAGE_CREATE` handler, as a bot would register it;
/// `--cache discord-js` gives it the cache discord.js keeps by default (D-174).
pub(crate) async fn client(options: &Options, allocator: &str) {
    let window = Arc::new(Window::new(options.get("events", 1_000_000)));
    let sequential = options.get("mode", String::from("concurrent")) == "sequential";
    let current_thread = options.get("runtime", String::from("multi")) == "current";
    let cached = match options.get("cache", String::from("none")).as_str() {
        "none" => false,
        "discord-js" | "serenity" => true,
        other => panic!("unknown --cache {other} (none, discord-js, serenity)"),
    };
    let serenity = options.get("cache", String::new()) == "serenity";
    // One fixed shard with a gateway URL: the client never calls Get Gateway Bot (which would
    // go to the real Discord API, D-146).
    let mut builder = Client::builder(TOKEN)
        .intents(INTENTS)
        .gateway_url(options.required("gateway"))
        .shards(Shards::Count(1))
        .compression(Compression::None)
        .handler_mode(if sequential {
            HandlerMode::Sequential
        } else {
            HandlerMode::Concurrent
        });
    if cached {
        builder = builder.cache(if serenity {
            serenity_cache()
        } else {
            CacheConfig::discord_js()
        });
    }
    let client = builder.build();
    let counter = Arc::clone(&window);
    client.on(Events::MessageCreate, move |_ctx, message| {
        let counter = Arc::clone(&counter);
        async move {
            black_box(&message.content);
            counter.hit();
            Ok(())
        }
    });
    crate::login(&client);
    window.wait().await;
    let mut variant = match (sequential, current_thread) {
        (false, false) => "Client + messageCreate handler",
        (true, false) => "Client + messageCreate handler, HandlerMode::Sequential",
        (false, true) => "Client + messageCreate handler, current-thread runtime",
        (true, true) => "Client + messageCreate handler, Sequential, current-thread runtime",
    }
    .to_owned();
    if serenity {
        variant.push_str(", serenity's default cache");
    } else if cached {
        variant.push_str(", CacheConfig::discord_js()");
    }
    println!("{}", window.result("djs-rs", &variant, allocator));
    // Exit without the clean disconnect; the mock server does not care.
    std::process::exit(0);
}

/// A bare djs-gateway [`Shard`], the layer the `Client` is built on (the counterpart of
/// `@discordjs/ws`).
pub(crate) async fn shard(options: &Options, allocator: &str) {
    let window = Window::new(options.get("events", 1_000_000));
    let config = Config::builder(TOKEN, INTENTS)
        .gateway_url(options.required("gateway"))
        .compression(Compression::None)
        .build();
    let mut shard = Shard::with_config(ShardId::ONE, config);
    while let Some(event) = shard.next_event().await {
        if let Ok(GatewayEvent::Dispatch(_, DispatchEvent::MessageCreate(message))) = event {
            black_box(&message.content);
            if window.hit() {
                break;
            }
        }
    }
    println!(
        "{}",
        window.result("djs-rs", "djs-gateway Shard (typed events)", allocator)
    );
    std::process::exit(0);
}
