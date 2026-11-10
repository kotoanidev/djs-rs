//! [Guilds](https://docs.discord.com/developers/resources/guild) (servers).

use serde::{Deserialize, Serialize};

use crate::{id::GuildId, macros::int_flags};

/// A guild that is not (or not yet) available to the client.
///
/// The [`Ready`](crate::gateway::payload::Ready) event lists every guild the bot is in as
/// unavailable; full guild data then arrives in `GUILD_CREATE` events. A guild also becomes
/// unavailable during a Discord outage.
///
/// See the [unavailable guild object](https://docs.discord.com/developers/resources/guild#unavailable-guild-object).
///
/// discord.js counterpart: guilds with `available === false`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnavailableGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// Whether the guild is unavailable due to an outage.
    ///
    /// In `GUILD_DELETE` events this is `false` (absent) when the bot was removed from the guild
    /// rather than the guild becoming unavailable.
    #[serde(default)]
    pub unavailable: bool,
}

int_flags! {
    /// [Flags of a guild's system channel](https://docs.discord.com/developers/resources/guild#guild-object-system-channel-flags):
    /// which automatic messages Discord does **not** post there.
    ///
    /// discord.js counterpart: `GuildSystemChannelFlags` / `SystemChannelFlagsBitField`.
    pub struct SystemChannelFlags: u64 {
        /// No message when a member joins.
        const SUPPRESS_JOIN_NOTIFICATIONS = 1 << 0;
        /// No message when the guild is boosted.
        const SUPPRESS_PREMIUM_SUBSCRIPTIONS = 1 << 1;
        /// No server setup tips.
        const SUPPRESS_GUILD_REMINDER_NOTIFICATIONS = 1 << 2;
        /// No sticker reply button on join messages.
        const SUPPRESS_JOIN_NOTIFICATION_REPLIES = 1 << 3;
        /// No message when a role subscription is purchased or renewed.
        const SUPPRESS_ROLE_SUBSCRIPTION_PURCHASE_NOTIFICATIONS = 1 << 4;
        /// No sticker reply button on role subscription messages.
        const SUPPRESS_ROLE_SUBSCRIPTION_PURCHASE_NOTIFICATION_REPLIES = 1 << 5;
    }
}

int_flags! {
    /// [Flags of a guild member](https://docs.discord.com/developers/resources/guild#guild-member-object-guild-member-flags).
    ///
    /// Only [`BYPASSES_VERIFICATION`](Self::BYPASSES_VERIFICATION) can be changed by bots
    /// (when modifying a member); the others are set by Discord.
    ///
    /// discord.js counterpart: `GuildMemberFlags` / `GuildMemberFlagsBitField`.
    pub struct MemberFlags: u64 {
        /// The member left the guild and joined again.
        const DID_REJOIN = 1 << 0;
        /// The member completed onboarding.
        const COMPLETED_ONBOARDING = 1 << 1;
        /// The member is exempt from the guild's verification requirements. Editable.
        const BYPASSES_VERIFICATION = 1 << 2;
        /// The member started onboarding.
        const STARTED_ONBOARDING = 1 << 3;
        /// The member is a guest who can only access the voice channel they were invited to.
        const IS_GUEST = 1 << 4;
        /// The member started the Server Guide new-member actions.
        const STARTED_HOME_ACTIONS = 1 << 5;
        /// The member completed the Server Guide new-member actions.
        const COMPLETED_HOME_ACTIONS = 1 << 6;
        /// AutoMod blocks the member's username, display name or nickname.
        const AUTOMOD_QUARANTINED_USERNAME = 1 << 7;
        /// AutoMod blocks the member's bio.
        ///
        /// **Deprecated** by Discord.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const AUTOMOD_QUARANTINED_BIO = 1 << 8;
        /// The member dismissed the DM settings upsell.
        const DM_SETTINGS_UPSELL_ACKNOWLEDGED = 1 << 9;
        /// AutoMod blocks the member's guild tag.
        const AUTOMOD_QUARANTINED_GUILD_TAG = 1 << 10;
    }
}

int_flags! {
    /// [Flags of a role](https://docs.discord.com/developers/topics/permissions#role-object-role-flags).
    ///
    /// discord.js counterpart: `RoleFlags` / `RoleFlagsBitField`.
    pub struct RoleFlags: u64 {
        /// Members can pick the role in an onboarding prompt.
        const IN_PROMPT = 1 << 0;
    }
}
