use core::fmt;

use bitflags::bitflags;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Visitor},
};

bitflags! {
    /// [Gateway intents](https://docs.discord.com/developers/events/gateway#gateway-intents):
    /// the groups of events a client subscribes to when identifying.
    ///
    /// Only request what the bot needs — every intent increases the amount of data Discord
    /// sends and the client must process.
    ///
    /// Three intents are **privileged** ([`Intents::PRIVILEGED`]) and must be enabled in the
    /// Developer Portal (and, for bots in 100+ guilds, approved by Discord) before they can be
    /// requested; otherwise the gateway closes with
    /// [`CloseCode::DisallowedIntents`](crate::gateway::CloseCode::DisallowedIntents).
    ///
    /// Unknown bits received from or sent to Discord are preserved.
    ///
    /// discord.js counterpart: `GatewayIntentBits` / `IntentsBitField`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::Intents;
    ///
    /// let intents = Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    /// assert_eq!(intents.bits(), 33281);
    /// assert!(intents.intersects(Intents::PRIVILEGED));
    /// ```
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Intents: u64 {
        /// Guild lifecycle, roles, channels, threads and stage instances.
        const GUILDS = 1 << 0;
        /// Guild member add/update/remove and thread member updates. **Privileged.**
        const GUILD_MEMBERS = 1 << 1;
        /// Bans and audit log entry creation.
        const GUILD_MODERATION = 1 << 2;
        /// Emoji, sticker and soundboard sound updates.
        const GUILD_EXPRESSIONS = 1 << 3;
        /// Integration create/update/delete.
        const GUILD_INTEGRATIONS = 1 << 4;
        /// Webhook updates.
        const GUILD_WEBHOOKS = 1 << 5;
        /// Invite create/delete.
        const GUILD_INVITES = 1 << 6;
        /// Voice state updates and voice channel effects.
        const GUILD_VOICE_STATES = 1 << 7;
        /// Presence updates. **Privileged.**
        const GUILD_PRESENCES = 1 << 8;
        /// Messages in guilds.
        const GUILD_MESSAGES = 1 << 9;
        /// Reactions in guilds.
        const GUILD_MESSAGE_REACTIONS = 1 << 10;
        /// Typing indicators in guilds.
        const GUILD_MESSAGE_TYPING = 1 << 11;
        /// Messages in direct messages.
        const DIRECT_MESSAGES = 1 << 12;
        /// Reactions in direct messages.
        const DIRECT_MESSAGE_REACTIONS = 1 << 13;
        /// Typing indicators in direct messages.
        const DIRECT_MESSAGE_TYPING = 1 << 14;
        /// Access to message content, embeds, attachments, components and polls.
        /// **Privileged.**
        ///
        /// Not an event group: without it, those message fields are empty except in DMs and
        /// messages that mention the bot.
        const MESSAGE_CONTENT = 1 << 15;
        /// Guild scheduled events.
        const GUILD_SCHEDULED_EVENTS = 1 << 16;
        /// Auto moderation rule create/update/delete.
        const AUTO_MODERATION_CONFIGURATION = 1 << 20;
        /// Auto moderation action execution.
        const AUTO_MODERATION_EXECUTION = 1 << 21;
        /// Poll votes in guilds.
        const GUILD_MESSAGE_POLLS = 1 << 24;
        /// Poll votes in direct messages.
        const DIRECT_MESSAGE_POLLS = 1 << 25;
    }
}

impl Intents {
    /// The privileged intents: [`GUILD_MEMBERS`](Self::GUILD_MEMBERS),
    /// [`GUILD_PRESENCES`](Self::GUILD_PRESENCES) and [`MESSAGE_CONTENT`](Self::MESSAGE_CONTENT).
    pub const PRIVILEGED: Self = Self::GUILD_MEMBERS
        .union(Self::GUILD_PRESENCES)
        .union(Self::MESSAGE_CONTENT);

    /// Every known intent that is not privileged.
    pub const NON_PRIVILEGED: Self = Self::all().difference(Self::PRIVILEGED);
}

impl Serialize for Intents {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for Intents {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IntentsVisitor;

        impl Visitor<'_> for IntentsVisitor {
            type Value = Intents;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an intents bitfield integer")
            }

            fn visit_u64<E: DeError>(self, value: u64) -> Result<Intents, E> {
                Ok(Intents::from_bits_retain(value))
            }
        }

        deserializer.deserialize_u64(IntentsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privileged() {
        assert_eq!(Intents::PRIVILEGED.bits(), (1 << 1) | (1 << 8) | (1 << 15));
        assert!(!Intents::NON_PRIVILEGED.intersects(Intents::PRIVILEGED));
        assert_eq!(
            Intents::NON_PRIVILEGED | Intents::PRIVILEGED,
            Intents::all()
        );
    }

    #[test]
    fn serde_preserves_unknown_bits() {
        let intents: Intents = serde_json::from_str("1073741825").unwrap();
        assert!(intents.contains(Intents::GUILDS));
        assert_eq!(serde_json::to_string(&intents).unwrap(), "1073741825");
    }
}
