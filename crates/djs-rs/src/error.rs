//! The error type of the high-level client.

/// An error that stops [`Client::login`](crate::Client::login).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A REST request needed to start the client failed (for example fetching the gateway URL
    /// with an invalid token).
    #[error("a REST request failed")]
    Rest(#[from] djs_rest::Error),
    /// The gateway connection failed fatally (for example an invalid token or disallowed
    /// intents).
    #[error("the gateway connection failed")]
    Gateway(#[from] djs_gateway::ReceiveError),
    /// [`Client::login`](crate::Client::login) was called while the client was already running.
    #[error("the client is already logged in")]
    AlreadyRunning,
}
