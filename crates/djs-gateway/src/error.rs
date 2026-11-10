//! Errors returned by shards.

use djs_model::gateway::CloseCode;

/// A boxed error source.
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// An error returned by [`Shard::next_event`](crate::Shard::next_event).
///
/// Most errors are **not fatal**: the shard has already scheduled a reconnect (or skipped the
/// offending payload) and the next call to `next_event` continues normally. Only
/// [`is_fatal`](Self::is_fatal) errors end the shard; after one, `next_event` returns `None`.
#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
pub struct ReceiveError {
    kind: ReceiveErrorKind,
    #[source]
    source: Option<BoxError>,
}

impl ReceiveError {
    pub(crate) fn new(kind: ReceiveErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub(crate) fn with_source(kind: ReceiveErrorKind, source: impl Into<BoxError>) -> Self {
        Self {
            kind,
            source: Some(source.into()),
        }
    }

    /// Returns what went wrong.
    #[must_use]
    pub const fn kind(&self) -> &ReceiveErrorKind {
        &self.kind
    }

    /// Returns `true` if the shard cannot continue (for example because the token is invalid
    /// or a privileged intent is not enabled). No further events will be produced.
    #[must_use]
    pub const fn is_fatal(&self) -> bool {
        matches!(self.kind, ReceiveErrorKind::FatallyClosed { .. })
    }

    /// Consumes the error, returning its kind and underlying source.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        ReceiveErrorKind,
        Option<Box<dyn std::error::Error + Send + Sync>>,
    ) {
        (self.kind, self.source)
    }
}

/// The kind of a [`ReceiveError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReceiveErrorKind {
    /// Connecting to the gateway failed. The shard retries with exponential backoff.
    #[error("failed to connect to the gateway")]
    Connect,
    /// Reading from or writing to the WebSocket failed. The shard reconnects and resumes.
    #[error("the gateway connection failed")]
    Connection,
    /// A compressed message could not be decompressed. The shard reconnects (a broken
    /// decompression context cannot be recovered).
    #[error("failed to decompress a gateway message")]
    Decompress,
    /// A payload could not be deserialized. It was skipped; the connection is unaffected.
    #[error("failed to deserialize a gateway payload")]
    Deserialize,
    /// Discord closed the connection with a close code that forbids reconnecting.
    #[error("the gateway closed the connection with {code:?}; reconnecting is not allowed")]
    FatallyClosed {
        /// The close code Discord sent.
        code: CloseCode,
    },
}

/// An error returned by [`MessageSender`](crate::MessageSender).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SendError {
    /// The command could not be serialized.
    #[error("failed to serialize the gateway command")]
    Serialize(#[source] serde_json::Error),
    /// The shard has been dropped, so the command can never be sent.
    #[error("the shard is gone")]
    Closed,
}
