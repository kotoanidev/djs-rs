//! Voice: [djs-voice](djs_voice) plugged into the [`Client`](crate::Client) (D-315,
//! D-345–D-348), behind the `voice` feature.
//!
//! [`Client::voice`](crate::Client::voice) gives the client's [`VoiceManager`]: the client
//! sends its Opcode 4 through the guild's shard and passes it the voice events, so a bot
//! only joins and plays. Everything here is djs-voice's API, re-exported.
//!
//! The client needs the `GUILD_VOICE_STATES` intent: without it the bot never learns its
//! voice state, and joins time out (the first `client.voice()` warns about it, D-346).
//!
//! discord.js counterpart: `@discordjs/voice` with `guild.voiceAdapterCreator`.
//!
//! # Examples
//!
//! ```no_run
//! use djs_rs::{model::gateway::Intents, prelude::*, voice::Input};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let client = Client::builder("token")
//!     .intents(Intents::GUILDS | Intents::GUILD_VOICE_STATES)
//!     .build();
//! let voice = client.voice().clone();
//! client.on(Events::MessageCreate, move |_ctx, msg| {
//!     let voice = voice.clone();
//!     async move {
//!         if msg.content == "!play"
//!             && let Some(guild_id) = msg.guild_id
//!         {
//!             # let channel_id = djs_rs::model::id::ChannelId::new(1);
//!             let call = voice.join(guild_id, channel_id).await?;
//!             call.play(Input::ogg_file("song.opus").await?);
//!         }
//!         Ok(())
//!     }
//! });
//! client.login().await?;
//! # Ok(()) }
//! ```

use core::sync::atomic::Ordering;
use std::{error::Error, sync::Arc};

pub use djs_voice::*;

use crate::{
    context::Shared,
    model::{
        gateway::payload::UpdateVoiceState,
        id::{GuildId, UserId},
    },
};

/// The [`VoiceGateway`] of a client: Opcode 4 through the guild's shard, picked with the
/// total shard count of the current `login` (the manager is made before it is known).
pub(crate) struct ClientGateway {
    pub(crate) shared: Arc<Shared>,
}

impl VoiceGateway for ClientGateway {
    fn send(
        &self,
        _shard_id: u32,
        command: &UpdateVoiceState,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let total = self.shared.total_shards.load(Ordering::Acquire);
        if total == 0 {
            return Err("the client is not logged in".into());
        }
        let shard = shard_of(command.guild_id, total);
        let sender = self
            .shared
            .shard(shard)
            .ok_or_else(|| format!("the guild's shard {shard} does not run in this process"))?;
        sender.command(command)?;
        Ok(())
    }
}

/// The shard of a guild: `(guild_id >> 22) % total`, as Discord documents.
pub(crate) fn shard_of(guild_id: GuildId, total: u32) -> u32 {
    let shard = (guild_id.get() >> 22) % u64::from(total.max(1));
    // Below `total`, a `u32`.
    u32::try_from(shard).unwrap_or_default()
}

/// The bot's ID from its token, whose first part is the ID in base64 (D-345); `None` for a
/// token of another shape.
pub(crate) fn bot_id(token: &str) -> Option<UserId> {
    let encoded = token.split('.').next()?;
    let mut decoded = Vec::with_capacity(encoded.len() * 3 / 4);
    let mut bits = 0_u32;
    let mut count = 0;
    for byte in encoded.bytes().filter(|&byte| byte != b'=') {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            // The byte that just completed: the low eight bits above the leftover ones.
            decoded.push(u8::try_from((bits >> count) & 0xFF).ok()?);
        }
    }
    let id = core::str::from_utf8(&decoded).ok()?.parse().ok()?;
    UserId::new_checked(id)
}

#[cfg(test)]
mod tests {
    use super::{bot_id, shard_of};
    use crate::model::id::{GuildId, UserId};

    #[test]
    fn the_bot_id_is_read_from_the_token() {
        // "123456789012345678" in base64, with and without padding, and URL-safe.
        assert_eq!(
            bot_id("MTIzNDU2Nzg5MDEyMzQ1Njc4.not.checked"),
            Some(UserId::new(123_456_789_012_345_678))
        );
        // "12345" needs padding.
        assert_eq!(bot_id("MTIzNDU=.x.y"), Some(UserId::new(12_345)));
        assert_eq!(bot_id("MTIzNDU.x.y"), Some(UserId::new(12_345)));
        assert_eq!(bot_id("test-token"), None, "not base64 of an ID");
        assert_eq!(bot_id(""), None);
        assert_eq!(bot_id("MA.x.y"), None, "the ID 0");
        assert_eq!(bot_id("!!!.x.y"), None);
    }

    #[test]
    fn guilds_map_to_their_shards() {
        assert_eq!(shard_of(GuildId::new(1 << 22), 2), 1);
        assert_eq!(shard_of(GuildId::new(3 << 22), 2), 1);
        assert_eq!(shard_of(GuildId::new(4 << 22), 2), 0);
        assert_eq!(shard_of(GuildId::new(41_771_983_423_143_937), 1), 0);
    }
}
