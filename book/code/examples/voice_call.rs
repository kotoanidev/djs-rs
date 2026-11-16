//! The guide's "Voice" chapter: joining the author's voice channel, a queue of tracks from
//! a `music/` directory, volume and skipping, and who speaks.
//!
//! Run it with `DISCORD_TOKEN=<token> cargo run -p djs-rs-guide --example voice_call` next to
//! a `music/` directory of audio files; it needs `ffmpeg` installed and the Message Content
//! intent. In a server: join a voice channel, then send `!join`, `!play <file>`,
//! `!volume 0.5`, `!skip` and `!leave`.

use std::path::Path;

use djs_rs::{
    cache::{CacheConfig, Limit},
    model::id::GuildId,
    prelude::*,
    voice::{Call, CallEvent, Input, JoinOptions, VoiceManager},
};

// ANCHOR: join
/// `!join`: the author's voice channel, from the cache's voice states.
async fn join(
    ctx: &Context,
    voice: &VoiceManager,
    msg: &djs_rs::model::message::Message,
    guild_id: GuildId,
) -> Result<(), HandlerError> {
    let channel_id = ctx
        .cache()
        .and_then(|cache| cache.voice_states(guild_id).get(msg.author.id))
        .and_then(|state| state.channel_id)
        .ok_or("join a voice channel first")?;
    // Not deafened, to hear who speaks.
    let options = JoinOptions::new().self_deaf(false);
    let call = voice.join_with(guild_id, channel_id, options).await?;

    let mut events = call.events();
    tokio::spawn(async move {
        while let Some(event) = events.next().await {
            match event {
                CallEvent::SpeakingStarted(user) => println!("{user} speaks"),
                CallEvent::Disconnected(reason) => {
                    println!("left the call: {reason}");
                    break;
                }
                _ => {}
            }
        }
    });
    msg.reply(ctx, "Joined.").await?;
    Ok(())
}
// ANCHOR_END: join

// ANCHOR: play
/// `!play <file>`: a file of `music/`, queued after the others. Only plain file names: the
/// name comes from a user, and FFmpeg would open any path or URL it is given.
fn play(call: &Call, name: &str) -> Result<String, HandlerError> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err("give the name of a file in music/".into());
    }
    let path = Path::new("music").join(name);
    if !path.is_file() {
        return Err(format!("music/{name} does not exist").into());
    }
    call.enqueue(Input::ffmpeg(&path));
    Ok(format!("Queued {name} ({} in the queue).", call.queue().len()))
}
// ANCHOR_END: play

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN")?;

    // ANCHOR: client
    // GUILD_VOICE_STATES: the bot learns its own voice state (joining needs it) and, cached,
    // where everyone else is.
    let client = Client::builder(token)
        .intents(
            Intents::GUILDS
                | Intents::GUILD_VOICE_STATES
                | Intents::GUILD_MESSAGES
                | Intents::MESSAGE_CONTENT,
        )
        .cache(
            CacheConfig::builder()
                .voice_states(Limit::Unlimited)
                .build(),
        )
        .build();
    let voice = client.voice().clone();
    // ANCHOR_END: client

    // ANCHOR: commands
    client.on(Events::MessageCreate, move |ctx, msg| {
        let voice = voice.clone();
        async move {
            let Some(guild_id) = msg.guild_id else {
                return Ok(());
            };
            let text = msg.content.trim();
            let (command, argument) = text.split_once(' ').unwrap_or((text, ""));
            if command == "!join" {
                return join(&ctx, &voice, &msg, guild_id).await;
            }
            // Every other command needs the bot in a call in this server.
            let Some(call) = voice.get(guild_id) else {
                return Ok(());
            };
            let answer = match command {
                "!play" => play(&call, argument)?,
                "!volume" => {
                    let volume: f32 = argument.trim().parse()?;
                    let track = call.queue().current().ok_or("nothing is playing")?;
                    // 1.0 is the file's own volume.
                    track.set_volume(volume.clamp(0.0, 2.0))?;
                    format!("Volume {volume}.")
                }
                "!skip" => {
                    call.queue().skip();
                    "Skipped.".to_owned()
                }
                "!leave" => {
                    call.leave().await?;
                    "Bye.".to_owned()
                }
                _ => return Ok(()),
            };
            msg.reply(&ctx, answer).await?;
            Ok(())
        }
    });
    // ANCHOR_END: commands

    let handle = client.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.shutdown();
        }
    });
    client.login().await?;
    Ok(())
}
