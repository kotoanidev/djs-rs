//! [Channels](https://docs.discord.com/developers/resources/channel): guild text, voice, forum
//! and media channels, threads and direct messages.

use crate::macros::int_flags;

int_flags! {
    /// [Flags of a channel](https://docs.discord.com/developers/resources/channel#channel-object-channel-flags).
    ///
    /// discord.js counterpart: `ChannelFlags` / `ChannelFlagsBitField`.
    pub struct ChannelFlags: u64 {
        /// The channel was removed from the guild's feed.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const GUILD_FEED_REMOVED = 1 << 0;
        /// The thread is pinned to the top of its forum or media channel.
        const PINNED = 1 << 1;
        /// The channel was removed from the guild's active channels.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const ACTIVE_CHANNELS_REMOVED = 1 << 2;
        /// New threads in this forum or media channel must have a tag (`applied_tags`).
        const REQUIRE_TAG = 1 << 4;
        /// The channel is marked as spam.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const IS_SPAM = 1 << 5;
        /// The channel is a guild resource channel (Server Guide).
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const IS_GUILD_RESOURCE_CHANNEL = 1 << 7;
        /// Clyde AI is enabled in the channel (feature retired by Discord).
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const CLYDE_AI = 1 << 8;
        /// The channel is scheduled for deletion.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const IS_SCHEDULED_FOR_DELETION = 1 << 9;
        /// Media download options are hidden. Media channels only.
        const HIDE_MEDIA_DOWNLOAD_OPTIONS = 1 << 15;
        /// The channel's metadata is hidden because the current user cannot view the channel.
        /// Only set on channels received over the gateway.
        const CHANNEL_OBFUSCATED = 1 << 17;
        /// A spoiler channel: users must opt in to see its contents. Textual guild channels and
        /// voice channels.
        const IS_SPOILER_CHANNEL = 1 << 21;
    }
}
