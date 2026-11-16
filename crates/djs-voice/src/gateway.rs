//! The adapter between djs-voice and the main gateway (D-315, D-324, D-334).

use std::error::Error;

use djs_model::gateway::payload::UpdateVoiceState;

/// Sends Opcode 4 (Update Voice State) through a shard of the main gateway: how a
/// [`VoiceManager`](crate::VoiceManager) joins, moves between and leaves voice channels.
///
/// The manager picks the guild's shard itself (`(guild_id >> 22) % shard_count`) and calls
/// [`send`](Self::send) with it; the events Discord answers with reach the manager through
/// [`VoiceManager::process`](crate::VoiceManager::process). djs-rs's `voice` feature
/// implements both for its `Client`; with djs-gateway alone, implement this trait over the
/// shards' `MessageSender`s.
///
/// Discord documents the command at
/// <https://discord.com/developers/docs/events/gateway-events#update-voice-state>.
///
/// discord.js counterpart: `DiscordGatewayAdapterLibraryMethods#sendPayload` of
/// `@discordjs/voice`'s adapters.
///
/// # Examples
///
/// ```
/// use std::error::Error;
///
/// use djs_model::gateway::payload::UpdateVoiceState;
/// use djs_voice::VoiceGateway;
///
/// /// Hands each command to the shard's own channel.
/// struct Shards {
///     senders: Vec<tokio::sync::mpsc::UnboundedSender<UpdateVoiceState>>,
/// }
///
/// impl VoiceGateway for Shards {
///     fn send(
///         &self,
///         shard_id: u32,
///         command: &UpdateVoiceState,
///     ) -> Result<(), Box<dyn Error + Send + Sync>> {
///         let sender = usize::try_from(shard_id)
///             .ok()
///             .and_then(|id| self.senders.get(id))
///             .ok_or("no such shard")?;
///         sender.send(*command)?;
///         Ok(())
///     }
/// }
/// ```
pub trait VoiceGateway: Send + Sync + 'static {
    /// Sends `command` through the shard `shard_id`.
    ///
    /// It is called from async code and must not block: queue the command (djs-gateway's
    /// `MessageSender::command` does) rather than wait for it to be sent.
    ///
    /// # Errors
    ///
    /// When the command cannot be sent, for example because the shard is closed; the error
    /// becomes [`JoinError::Gateway`](crate::JoinError::Gateway), or the result of
    /// [`Call::leave`](crate::Call::leave).
    fn send(
        &self,
        shard_id: u32,
        command: &UpdateVoiceState,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
