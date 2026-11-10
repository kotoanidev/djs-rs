//! Presence data sent over the gateway: activities and statuses.

use crate::macros::int_flags;

int_flags! {
    /// [Flags of an activity](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-flags):
    /// what the activity supports (joining, spectating, …).
    ///
    /// discord.js counterpart: `ActivityFlags` / `ActivityFlagsBitField`.
    pub struct ActivityFlags: u64 {
        /// The activity is an instanced game session.
        const INSTANCE = 1 << 0;
        /// Others can join the activity.
        const JOIN = 1 << 1;
        /// Others can spectate the activity.
        const SPECTATE = 1 << 2;
        /// Others can ask to join the activity.
        const JOIN_REQUEST = 1 << 3;
        /// A Spotify-style synced activity ("Listen along").
        const SYNC = 1 << 4;
        /// The activity can be played.
        const PLAY = 1 << 5;
        /// Party privacy: only friends can join.
        const PARTY_PRIVACY_FRIENDS = 1 << 6;
        /// Party privacy: members of the same voice channel can join.
        const PARTY_PRIVACY_VOICE_CHANNEL = 1 << 7;
        /// The activity is an embedded app (Activity) running in Discord.
        const EMBEDDED = 1 << 8;
    }
}
