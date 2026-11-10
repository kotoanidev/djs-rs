//! [Applications](https://docs.discord.com/developers/resources/application): the apps (and
//! their bot users) registered in the Developer Portal.

use crate::macros::int_flags;

int_flags! {
    /// [Flags of an application](https://docs.discord.com/developers/resources/application#application-object-application-flags).
    ///
    /// The `GATEWAY_*` flags tell whether the app may use the privileged
    /// [intents](crate::gateway::Intents): the plain flag is granted after approval (bots in 100
    /// or more servers), the `*_LIMITED` flag when it is enabled in the Developer Portal for a
    /// bot in fewer than 100 servers.
    ///
    /// discord.js counterpart: `ApplicationFlags` / `ApplicationFlagsBitField`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::application::ApplicationFlags;
    ///
    /// let flags: ApplicationFlags = serde_json::from_str("532480").unwrap();
    /// assert!(flags.contains(ApplicationFlags::GATEWAY_MESSAGE_CONTENT_LIMITED));
    /// ```
    pub struct ApplicationFlags: u64 {
        /// The app's embedded activity has been released.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const EMBEDDED_RELEASED = 1 << 1;
        /// The app can manage emojis.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const MANAGED_EMOJI = 1 << 2;
        /// The app's embedded activity supports in-app purchases.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const EMBEDDED_IAP = 1 << 3;
        /// The app can create group DMs.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const GROUP_DM_CREATE = 1 << 4;
        /// The app uses the [Auto Moderation API](https://docs.discord.com/developers/resources/auto-moderation)
        /// (shown as a badge on its profile).
        const APPLICATION_AUTO_MODERATION_RULE_CREATE_BADGE = 1 << 6;
        /// The app has connected to RPC.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const RPC_HAS_CONNECTED = 1 << 11;
        /// Approved for the privileged presence intent (bots in 100 or more servers).
        const GATEWAY_PRESENCE = 1 << 12;
        /// Presence intent enabled in the Developer Portal (bots in fewer than 100 servers).
        const GATEWAY_PRESENCE_LIMITED = 1 << 13;
        /// Approved for the privileged guild members intent (bots in 100 or more servers).
        const GATEWAY_GUILD_MEMBERS = 1 << 14;
        /// Guild members intent enabled in the Developer Portal (bots in fewer than 100
        /// servers).
        const GATEWAY_GUILD_MEMBERS_LIMITED = 1 << 15;
        /// Unusual growth of the app prevents its verification.
        const VERIFICATION_PENDING_GUILD_LIMIT = 1 << 16;
        /// The app is embedded in the Discord client (not publicly available).
        const EMBEDDED = 1 << 17;
        /// Approved for the privileged message content intent (bots in 100 or more servers).
        const GATEWAY_MESSAGE_CONTENT = 1 << 18;
        /// Message content intent enabled in the Developer Portal (bots in fewer than 100
        /// servers).
        const GATEWAY_MESSAGE_CONTENT_LIMITED = 1 << 19;
        /// The embedded app is made by Discord.
        ///
        /// **Undocumented:** known from discord-api-types, not in the official docs; may
        /// change without notice.
        const EMBEDDED_FIRST_PARTY = 1 << 20;
        /// The app has registered global application commands (shown as a badge on its
        /// profile).
        const APPLICATION_COMMAND_BADGE = 1 << 23;
    }
}
