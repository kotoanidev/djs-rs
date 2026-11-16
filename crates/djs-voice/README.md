# djs-voice

Discord voice for [djs-rs](https://github.com/kotoanidev/djs-rs): joining voice channels,
playing audio into them and receiving it, with the end-to-end encryption (DAVE) Discord
requires for every call since March 2026.

**Status: under construction (Phase 10, step 1).** The API follows songbird's concepts
(D-316): a `VoiceManager` per bot, a `Call` per guild, and tracks played into it. Step 1
joins voice channels (with DAVE), plays Ogg/Opus and reports the call's events; the queue,
PCM, `ffmpeg` and volume come in step 2, receiving audio in step 3.

- Works on its own through a small gateway adapter, `VoiceGateway` (D-315, D-324), or
  through djs-rs's `voice` feature, which plugs it into the `Client`.
- Plays Ogg/Opus natively (`Input::ogg`, `Input::ogg_file`); any format through `ffmpeg`,
  and raw PCM, later (D-317).
- DAVE comes from the [`davey`](https://crates.io/crates/davey) crate (D-312), Opus from
  `audiopus` (D-313) and the transport encryption from
  `aes-gcm` and `chacha20poly1305` (D-314).

```no_run
use std::error::Error;

use djs_model::{
    gateway::{DispatchEvent, payload::UpdateVoiceState},
    id::{ChannelId, GuildId, UserId},
};
use djs_voice::{CallEvent, Input, VoiceGateway, VoiceManager};

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

let call = manager.join(GuildId::new(2), ChannelId::new(3)).await?;
let track = call.play(Input::ogg_file("song.opus").await?);
let mut events = call.events();
while let Some(event) = events.next().await {
    match event {
        CallEvent::UserJoined(user) => println!("{user} joined"),
        CallEvent::Disconnected(reason) => println!("the call ended: {reason}"),
        _ => {}
    }
}
# Ok(()) }
```

## Licenses

djs-voice is `MIT OR Apache-2.0` like the rest of djs-rs. DAVE's MLS stack under `davey`
uses three HPKE crates (`hpke-rs*`) licensed MPL-2.0, unmodified (D-326): a bot that uses
djs-voice links them, and their source stays available on crates.io.

discord.js counterpart: [`@discordjs/voice`](https://discord.js.org/docs/packages/voice/main);
the concepts are songbird's.

