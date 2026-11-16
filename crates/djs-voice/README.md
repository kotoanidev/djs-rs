# djs-voice

Discord voice for [djs-rs](https://github.com/kotoanidev/djs-rs): joining voice channels,
playing audio into them and receiving it, with the end-to-end encryption (DAVE) Discord
requires for every call since March 2026.

**Status: under construction (Phase 10, step 3).** The API follows songbird's concepts
(D-316): a `VoiceManager` per bot, a `Call` per guild, and tracks played into it, one at a
time, from a queue. Steps 1 to 3 join voice channels (with DAVE), play Ogg/Opus, PCM and
anything FFmpeg reads, with a queue, volume, loops and seeking, receive each user's audio,
and report the call's and the tracks' events; benchmarks come in step 4.

- Works on its own through a small gateway adapter, `VoiceGateway` (D-315, D-324), or
  through djs-rs's `voice` feature, which plugs it into the `Client`.
- Plays Ogg/Opus natively, sent as it is at volume 1.0 (`Input::ogg`, `Input::ogg_file`),
  raw 48 kHz PCM (`Input::pcm`), and any file or URL through FFmpeg (`Input::ffmpeg`, which
  needs `ffmpeg` installed) (D-317, D-354).
- `call.play` plays now, `call.enqueue` after the others (`call.queue()`: skip, clear);
  each track's handle pauses, stops, sets the volume, loops and seeks, and gives its state
  (`info`) and events (`events`) (D-352–D-361).
- `call.subscribe(user)` receives a user's audio as a stream of packets, in order (lost
  ones reported), as Opus or decoded to 48 kHz stereo `i16` (`ReceiveOptions`), ending by
  itself after silence if asked (`EndBehavior`); `CallEvent::SpeakingStarted` and
  `SpeakingStopped` say who speaks (D-363–D-373). Join with `JoinOptions::self_deaf(false)`
  to receive: Discord sends a deafened bot nobody's audio.
- DAVE comes from the [`davey`](https://crates.io/crates/davey) crate (D-312), Opus from
  [`opus2`](https://crates.io/crates/opus2) with libopus 1.5 built from source and linked
  statically (D-350: CMake and a C compiler are needed to build), and the transport
  encryption from `aes-gcm` and `chacha20poly1305` (D-314).

```no_run
use std::{error::Error, time::Duration};

use djs_model::{
    gateway::{DispatchEvent, payload::UpdateVoiceState},
    id::{ChannelId, GuildId, UserId},
};
use djs_voice::{
    CallEvent, EndBehavior, Input, JoinOptions, ReceiveOptions, TrackEvent, VoiceData,
    VoiceGateway, VoiceManager,
};

/// Sends Opcode 4 through the bot's shards (djs-rs's `voice` feature does this for you).
struct Shards;

impl VoiceGateway for Shards {
    fn send(
        &self,
        shard_id: u32,
        command: &UpdateVoiceState,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // e.g. `shards[shard_id].sender().command(command)?`
        Ok(())
    }
}

# async fn next_gateway_event() -> DispatchEvent { unimplemented!() }
# async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
let manager = VoiceManager::new(UserId::new(1), 1, Shards);
// The main gateway's events go through `process`.
let voice = manager.clone();
tokio::spawn(async move {
    loop {
        voice.process(&next_gateway_event().await);
    }
});

// Not deafened: to hear the channel.
let options = JoinOptions::new().self_deaf(false);
let call = manager
    .join_with(GuildId::new(2), ChannelId::new(3), options)
    .await?;
let intro = call.play(Input::ogg_file("intro.opus").await?);
let song = call.enqueue(Input::ffmpeg("song.mp3"));
song.set_volume(0.5)?;
let mut song_events = song.events();
tokio::spawn(async move {
    while let Some(event) = song_events.next().await {
        if let TrackEvent::Ended(reason) = event {
            println!("the song ended: {reason}");
        }
    }
});
let mut events = call.events();
while let Some(event) = events.next().await {
    match event {
        CallEvent::UserJoined(user) => println!("{user} joined"),
        CallEvent::SpeakingStarted(user) => {
            // Record what they say from now on, decoded, until half a second of silence.
            let options = ReceiveOptions::new()
                .decode(true)
                .end(EndBehavior::AfterSilence(Duration::from_millis(500)));
            let mut voice = call.subscribe_with(user, options);
            tokio::spawn(async move {
                let mut samples = Vec::new();
                while let Some(packet) = voice.next().await {
                    if let VoiceData::Pcm(pcm) = packet.audio {
                        samples.extend(pcm);
                    }
                }
            });
        }
        CallEvent::Disconnected(reason) => println!("the call ended: {reason}"),
        _ => {}
    }
}
# Ok(()) }
```

## Licenses

djs-voice is `MIT OR Apache-2.0` like the rest of djs-rs. DAVE's MLS stack under `davey`
uses three HPKE crates (`hpke-rs*`) licensed MPL-2.0, unmodified (D-326): a bot that uses
djs-voice links them, and their source stays available on crates.io. libopus, built into
the bot, is BSD-3-Clause: a bot distributed as a binary must reproduce its copyright notice
(see `NOTICE`).

discord.js counterpart: [`@discordjs/voice`](https://discord.js.org/docs/packages/voice/main);
the concepts are songbird's.

