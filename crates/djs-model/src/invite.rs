//! [Invites](https://docs.discord.com/developers/resources/invite) to guilds, group DMs and
//! voice channels.

use crate::macros::int_flags;

int_flags! {
    /// [Flags of a guild invite](https://docs.discord.com/developers/resources/invite#invite-object-guild-invite-flags).
    ///
    /// discord.js counterpart: `InviteFlags` / `InviteFlagsBitField`.
    pub struct InviteFlags: u64 {
        /// A guest invite: it only grants access to one voice channel.
        const IS_GUEST_INVITE = 1 << 0;
    }
}
