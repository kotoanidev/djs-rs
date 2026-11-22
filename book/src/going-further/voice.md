# Voice

A bot can join voice channels, play audio into them and hear what is said there. Voice
lives in the `djs-voice` crate, which the `voice` feature plugs into the client:

```toml
djs-rs = { version = "0.1.0-alpha.1", features = ["voice"] }
```

It builds libopus from source and links it in, so building needs CMake and a C compiler.
Playing anything but Ogg/Opus and raw PCM needs [FFmpeg](https://ffmpeg.org) installed.

This chapter's program is
[`voice_call.rs`](https://github.com/kotoanidev/djs-rs/blob/main/book/code/examples/voice_call.rs):
a small music bot playing the files of a `music/` directory.

## The client

```rust,no_run
{{#include ../../code/examples/voice_call.rs:client}}
```

`client.voice()` is the client's [`VoiceManager`]: one per bot, with one [`Call`] per
server. The manager sends the gateway's voice requests through the right shard and
follows the voice events by itself; a bot only joins and plays. The client needs the
`GUILD_VOICE_STATES` intent, without which a join never completes; the cache of voice
states tells the program which channel a user is in.

(discord.js: `@discordjs/voice` with `joinVoiceChannel` and `guild.voiceAdapterCreator`.)

## Joining

```rust,no_run
{{#include ../../code/examples/voice_call.rs:join}}
```

`join` returns once the bot is connected and its call is encrypted. Every call is
encrypted end to end with Discord's DAVE protocol, which Discord requires of every client
since March 2026; djs-voice does it with no setting. A bot joins deafened unless
`JoinOptions` says otherwise: Discord sends a deafened bot nobody's audio.

`call.events()` reports what happens in the call: users joining and leaving, who starts and
stops speaking, and the disconnection, with its reason.

## Playing

```rust,no_run
{{#include ../../code/examples/voice_call.rs:play}}
```

An [`Input`] is what a track plays:

- `Input::ogg_file` and `Input::ogg` read Ogg/Opus, Discord's own format, and send it as it
  is: no decoding, no encoding, a fraction of a CPU core for hundreds of calls.
- `Input::pcm` reads raw 48 kHz stereo samples, encoded on the way.
- `Input::ffmpeg` reads anything FFmpeg reads (MP3, FLAC, WAV, a URL…).

`call.play` plays a track now, `call.enqueue` after the others. Both return a
[`TrackHandle`]: `pause`, `play`, `stop`, `set_volume`, `seek`, `set_loop`, its state
(`info`) and its events (`events`).

Whatever FFmpeg is given, it opens: a path, a URL, a device. The program therefore takes
only plain file names of its `music/` directory from users. A bot that plays links should
check them the same way (an allowed host, `https://` only).

(discord.js: `createAudioPlayer`, `createAudioResource`, `player.play`; djs-voice has a
queue of its own.)

## The commands

```rust,no_run
{{#include ../../code/examples/voice_call.rs:commands}}
```

`voice.get(guild)` is the server's call, if the bot is in one. `call.queue()` is its queue:
`current` is the track playing, `skip` goes to the next one, `clear` empties it. `leave`
ends the call.

## Hearing

`call.subscribe(user)` returns a stream of that user's audio: their packets in order,
lost ones reported, as Opus or (`ReceiveOptions::decode`) as 48 kHz stereo samples, and
`EndBehavior` ends it after a silence. Only the users a call is subscribed to are
decrypted and decoded, so listening to one speaker in a crowded channel costs one
speaker. The djs-voice
[README](https://github.com/kotoanidev/djs-rs/blob/main/crates/djs-voice/README.md)
records a user until they stop speaking.

(discord.js: `connection.receiver.subscribe`.)

## What's next

The last chapter answers interactions over HTTP, without the gateway.

[`VoiceManager`]: ../api/djs_voice/struct.VoiceManager.html
[`Call`]: ../api/djs_voice/struct.Call.html
[`Input`]: ../api/djs_voice/struct.Input.html
[`TrackHandle`]: ../api/djs_voice/struct.TrackHandle.html
