use crate::macros::int_enum;

int_enum! {
    /// A [gateway close event code](https://docs.discord.com/developers/topics/opcodes-and-status-codes#gateway-gateway-close-event-codes):
    /// the reason Discord gives when it closes the WebSocket connection.
    ///
    /// Use [`CloseCode::can_reconnect`] to decide whether reconnecting is allowed.
    ///
    /// discord.js counterpart: `GatewayCloseCodes` (re-exported from discord-api-types).
    pub enum CloseCode: u16 {
        /// Something went wrong on Discord's side. Reconnecting is allowed.
        UnknownError = 4000,
        /// The client sent an invalid opcode or an invalid payload for an opcode.
        UnknownOpcode = 4001,
        /// The client sent a payload Discord could not decode.
        DecodeError = 4002,
        /// The client sent a payload before identifying, or the session was invalidated.
        NotAuthenticated = 4003,
        /// The token sent in the identify payload is incorrect. **Do not reconnect.**
        AuthenticationFailed = 4004,
        /// The client sent more than one identify payload.
        AlreadyAuthenticated = 4005,
        /// The sequence number sent when resuming was invalid. Reconnect and start a new session.
        InvalidSequence = 4007,
        /// The client is sending payloads too quickly.
        RateLimited = 4008,
        /// The session timed out. Reconnect and start a new session.
        SessionTimedOut = 4009,
        /// The client sent an invalid shard when identifying. **Do not reconnect.**
        InvalidShard = 4010,
        /// The session would have handled too many guilds; sharding is required.
        /// **Do not reconnect.**
        ShardingRequired = 4011,
        /// The client requested an invalid gateway version. **Do not reconnect.**
        InvalidApiVersion = 4012,
        /// The client sent an invalid intent bitfield. **Do not reconnect.**
        InvalidIntents = 4013,
        /// The client requested an intent it is not enabled or approved for (usually a privileged
        /// intent). **Do not reconnect.**
        DisallowedIntents = 4014,
    }
}

impl CloseCode {
    /// Returns `true` if Discord allows reconnecting after receiving this close code.
    ///
    /// Unknown codes are assumed to allow reconnecting, matching the documented behaviour of
    /// unexpected disconnects.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::CloseCode;
    ///
    /// assert!(CloseCode::SessionTimedOut.can_reconnect());
    /// assert!(!CloseCode::AuthenticationFailed.can_reconnect());
    /// ```
    #[must_use]
    pub const fn can_reconnect(self) -> bool {
        !matches!(
            self,
            Self::AuthenticationFailed
                | Self::InvalidShard
                | Self::ShardingRequired
                | Self::InvalidApiVersion
                | Self::InvalidIntents
                | Self::DisallowedIntents
        )
    }
}
