# djs-voice

Discord voice for [djs-rs](https://github.com/kotoanidev/djs-rs): joining voice channels,
playing audio into them and receiving it, with the end-to-end encryption (DAVE) Discord
requires for every call since March 2026.

**Status: under construction (Phase 10, step 1).** The API follows songbird's concepts
(D-316): a `VoiceManager` per bot, a `Call` per guild, and tracks played into it. Nothing
is usable yet.

- Works on its own through a small gateway adapter (D-315), or through djs-rs's `voice`
  feature, which plugs it into the `Client`.
- Plays Ogg/Opus natively, any format through `ffmpeg`, and raw PCM (D-317).
- DAVE comes from the [`davey`](https://crates.io/crates/davey) crate (D-312), Opus from
  `audiopus` (D-313) and the transport encryption from
  `aes-gcm` and `chacha20poly1305` (D-314).

## Licenses

djs-voice is `MIT OR Apache-2.0` like the rest of djs-rs. DAVE's MLS stack under `davey`
uses three HPKE crates (`hpke-rs*`) licensed MPL-2.0, unmodified (D-326): a bot that uses
djs-voice links them, and their source stays available on crates.io.

discord.js counterpart: [`@discordjs/voice`](https://discord.js.org/docs/packages/voice/main);
the concepts are songbird's.

