use crate::macros::int_enum;

int_enum! {
    /// A [gateway opcode](https://docs.discord.com/developers/topics/opcodes-and-status-codes#gateway-gateway-opcodes):
    /// the `op` field of every gateway payload.
    ///
    /// discord.js counterpart: `GatewayOpcodes` (re-exported from discord-api-types).
    pub enum OpCode: u8 {
        /// *Receive.* An event was dispatched.
        Dispatch = 0,
        /// *Send/Receive.* Keeps the connection alive. When received, a heartbeat must be sent
        /// immediately.
        Heartbeat = 1,
        /// *Send.* Starts a new session during the initial handshake.
        Identify = 2,
        /// *Send.* Updates the client's presence.
        PresenceUpdate = 3,
        /// *Send.* Joins, leaves or moves between voice channels.
        VoiceStateUpdate = 4,
        /// *Send.* Resumes a previous session that was disconnected.
        Resume = 6,
        /// *Receive.* The client should reconnect and resume immediately.
        Reconnect = 7,
        /// *Send.* Requests information about offline guild members in a large guild.
        RequestGuildMembers = 8,
        /// *Receive.* The session has been invalidated. The client should reconnect and
        /// identify/resume accordingly.
        InvalidSession = 9,
        /// *Receive.* Sent immediately after connecting; contains the heartbeat interval.
        Hello = 10,
        /// *Receive.* Sent in response to receiving a heartbeat to acknowledge that it has been
        /// received.
        HeartbeatAck = 11,
        /// *Send.* Requests soundboard sounds of a list of guilds.
        RequestSoundboardSounds = 31,
        /// *Send.* Requests ephemeral channel information (such as voice channel statuses) for
        /// a guild.
        RequestChannelInfo = 43,
    }
}
