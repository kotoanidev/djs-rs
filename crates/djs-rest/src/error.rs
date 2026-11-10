//! Errors returned by the REST client.

use core::{fmt, time::Duration};

use http::StatusCode;
use serde::Deserialize;

/// A boxed error source.
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// An error returned by [`Rest`](crate::Rest).
///
/// The error is deliberately small (a kind plus an optional boxed source) so `Result<T, Error>`
/// stays cheap to move around. Inspect it with [`kind`](Self::kind), or use the shortcuts
/// [`status`](Self::status) and [`api_error`](Self::api_error).
///
/// # Examples
///
/// ```no_run
/// # async fn example(rest: djs_rest::Rest, channel_id: djs_model::id::ChannelId) {
/// use djs_rest::ErrorKind;
///
/// match rest.create_message(channel_id, &"hi".into()).await {
///     Ok(message) => println!("sent {}", message.id),
///     Err(error) if error.api_error().is_some_and(|api| api.code == 50013) => {
///         eprintln!("missing permissions");
///     }
///     Err(error) => eprintln!("request failed: {error}"),
/// }
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
pub struct Error {
    kind: ErrorKind,
    #[source]
    source: Option<BoxError>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub(crate) fn with_source(kind: ErrorKind, source: impl Into<BoxError>) -> Self {
        Self {
            kind,
            source: Some(source.into()),
        }
    }

    /// Returns what went wrong.
    #[must_use]
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Consumes the error, returning its kind and underlying source.
    #[must_use]
    pub fn into_parts(self) -> (ErrorKind, Option<Box<dyn std::error::Error + Send + Sync>>) {
        (self.kind, self.source)
    }

    /// Returns the HTTP status code, if Discord responded with an error status.
    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        match &self.kind {
            ErrorKind::Response { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns the error body Discord sent, if any.
    #[must_use]
    pub fn api_error(&self) -> Option<&ApiError> {
        match &self.kind {
            ErrorKind::Response { error, .. } => error.as_deref(),
            _ => None,
        }
    }
}

/// The kind of an [`Error`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The request could not be built — for example the token contains characters that are not
    /// allowed in an HTTP header, or the body could not be serialized.
    #[error("failed to build the request")]
    BuildRequest,
    /// The request could not be sent or the response could not be received (DNS, TCP, TLS,
    /// HTTP protocol errors). Retried automatically up to the configured number of times.
    #[error("failed to send the request or receive the response")]
    Transport,
    /// No complete response arrived within the configured timeout. Retried automatically up to
    /// the configured number of times.
    #[error("the request timed out")]
    Timeout,
    /// Discord responded with a non-success status code.
    #[error("Discord responded with {status}{}", error.as_ref().map(|error| format!(": {error}")).unwrap_or_default())]
    Response {
        /// The HTTP status code.
        status: StatusCode,
        /// The parsed error body, if the response contained one.
        error: Option<Box<ApiError>>,
    },
    /// The response body could not be deserialized into the expected type.
    #[error("failed to deserialize the response body")]
    Deserialize,
}

/// The JSON error body Discord returns with 4xx responses.
///
/// See [JSON error codes](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json).
///
/// discord.js counterpart: `DiscordAPIError`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ApiError {
    /// The [JSON error code](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json-json-error-codes),
    /// e.g. `50013` for "Missing Permissions".
    pub code: u32,
    /// A human-readable description of the error.
    pub message: String,
    /// Detailed validation errors for form bodies (error code `50035`), as sent by Discord.
    #[serde(default)]
    pub errors: Option<serde_json::Value>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)
    }
}

/// The body of a 429 response.
#[derive(Debug, Deserialize)]
pub(crate) struct RateLimitedBody {
    /// Seconds to wait before retrying.
    pub(crate) retry_after: f64,
    /// Whether the global rate limit was hit.
    #[serde(default)]
    pub(crate) global: bool,
}

impl RateLimitedBody {
    pub(crate) fn retry_after(&self) -> Duration {
        seconds(self.retry_after)
    }
}

/// Converts a (possibly malformed) number of seconds into a `Duration`, clamping invalid values
/// to zero and absurdly large ones to one hour.
pub(crate) fn seconds(value: f64) -> Duration {
    if value.is_finite() && value > 0.0 {
        Duration::from_secs_f64(value.min(3_600.0))
    } else {
        Duration::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        let error = Error::new(ErrorKind::Response {
            status: StatusCode::FORBIDDEN,
            error: Some(Box::new(ApiError {
                code: 50013,
                message: "Missing Permissions".into(),
                errors: None,
            })),
        });
        assert_eq!(
            error.to_string(),
            "Discord responded with 403 Forbidden: Missing Permissions (code 50013)"
        );
        assert_eq!(error.status(), Some(StatusCode::FORBIDDEN));
        assert_eq!(error.api_error().unwrap().code, 50013);
    }

    #[test]
    fn size() {
        // Kind + optional boxed source; keep `Result<T, Error>` small.
        assert!(size_of::<Error>() <= 48, "{}", size_of::<Error>());
    }

    #[test]
    fn seconds_is_robust() {
        assert_eq!(seconds(f64::NAN), Duration::ZERO);
        assert_eq!(seconds(-1.0), Duration::ZERO);
        assert_eq!(seconds(0.25), Duration::from_millis(250));
        assert_eq!(seconds(1e12), Duration::from_secs(3_600));
    }
}
