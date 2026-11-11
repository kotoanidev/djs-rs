//! Errors returned by the REST client.

use core::{fmt, time::Duration};

use djs_model::rest::ErrorCode;
use http::StatusCode;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

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
/// use djs_model::rest::ErrorCode;
///
/// match rest.create_message(channel_id, &"hi".into()).await {
///     Ok(message) => println!("sent {}", message.id),
///     Err(error)
///         if error
///             .api_error()
///             .is_some_and(|api| api.code == ErrorCode::MissingPermissions) =>
///     {
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
    /// [Search Guild Messages](crate::Rest::search_guild_messages) found the guild not indexed
    /// yet (Discord answered `202 Accepted`, D-104). Retry after `retry_after`; Discord sends
    /// zero when it wants "a short delay".
    #[error("the guild is not indexed for search yet; retry after {retry_after:?}")]
    NotYetIndexed {
        /// How long to wait before retrying.
        retry_after: Duration,
        /// How many documents the running indexing has processed so far.
        documents_indexed: u64,
    },
    /// The request was not sent because the client already made as many invalid requests (401,
    /// 403, 429) in the last 10 minutes as the configured
    /// [`invalid_request_limit`](crate::RestBuilder::invalid_request_limit) allows (D-085).
    /// Discord temporarily bans the IP at 10 000.
    #[error("refused to send: {count} invalid requests in the last 10 minutes (limit {limit})")]
    InvalidRequestLimit {
        /// The invalid requests counted in the window.
        count: u32,
        /// The configured limit.
        limit: u32,
    },
}

/// The JSON error body Discord returns with 4xx responses.
///
/// Its [`Display`](fmt::Display) shows the message and code, then one line per
/// [field error](Self::field_errors), e.g.:
///
/// ```text
/// Invalid Form Body (code 50035)
/// embeds[0].title[BASE_TYPE_REQUIRED]: This field is required
/// ```
///
/// See [error messages](https://docs.discord.com/developers/reference#error-messages) and
/// [JSON error codes](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json).
///
/// discord.js counterpart: `DiscordAPIError`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::ErrorCode;
/// use djs_rest::ApiError;
///
/// let error: ApiError = serde_json::from_str(
///     r#"{"code": 50035, "message": "Invalid Form Body", "errors": {
///         "content": {"_errors": [{"code": "BASE_TYPE_MAX_LENGTH",
///                                  "message": "Must be 2000 or fewer in length."}]}}}"#,
/// )
/// .unwrap();
/// assert_eq!(error.code, ErrorCode::InvalidFormBodyOrContentType);
/// assert_eq!(error.field_errors[0].path, "content");
/// assert_eq!(
///     error.to_string(),
///     "Invalid Form Body (code 50035)\ncontent[BASE_TYPE_MAX_LENGTH]: Must be 2000 or fewer in length."
/// );
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ApiError {
    /// The [JSON error code](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json-json-error-codes),
    /// e.g. [`MissingPermissions`](ErrorCode::MissingPermissions) (D-097).
    pub code: ErrorCode,
    /// A human-readable description of the error.
    pub message: String,
    /// The detailed errors of the request's fields (mostly with
    /// [`InvalidFormBodyOrContentType`](ErrorCode::InvalidFormBodyOrContentType)), flattened
    /// from [`errors`](Self::errors) (D-099). Empty if Discord sent none.
    pub field_errors: Vec<FieldError>,
    /// The nested `errors` object as Discord sent it.
    pub errors: Option<Value>,
}

impl<'de> Deserialize<'de> for ApiError {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            code: ErrorCode,
            message: String,
            #[serde(default)]
            errors: Option<Value>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let mut field_errors = Vec::new();
        if let Some(errors) = &raw.errors {
            flatten_errors(errors, &mut String::new(), &mut field_errors);
        }
        Ok(Self {
            code: raw.code,
            message: raw.message,
            field_errors,
            errors: raw.errors,
        })
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (code {})", self.message, self.code.value())?;
        for error in &self.field_errors {
            write!(f, "\n{error}")?;
        }
        Ok(())
    }
}

/// An error of one field of a request body, from the `errors` of an [`ApiError`] (D-099).
///
/// discord.js counterpart: the lines of a `DiscordAPIError` message after the first.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldError {
    /// Where the field is in the request body, e.g. `embeds[0].title`; empty for an error of
    /// the request as a whole.
    pub path: String,
    /// What is wrong, e.g. `BASE_TYPE_REQUIRED` (empty if Discord sent only a message).
    pub code: String,
    /// A human-readable description.
    pub message: String,
}

impl fmt::Display for FieldError {
    /// `path[CODE]: message`, leaving out the parts that are empty.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.path.is_empty(), self.code.is_empty()) {
            (false, false) => write!(f, "{}[{}]: {}", self.path, self.code, self.message),
            (false, true) => write!(f, "{}: {}", self.path, self.message),
            (true, false) => write!(f, "{}: {}", self.code, self.message),
            (true, true) => f.write_str(&self.message),
        }
    }
}

/// Collects the errors of a nested `errors` value. Object keys extend the path (`.name`, or
/// `[n]` for array indices), except keys starting with `_` such as `_errors`, the lists of
/// `{code, message}` errors of a field. Depth is bounded by `serde_json`'s recursion limit.
fn flatten_errors(value: &Value, path: &mut String, out: &mut Vec<FieldError>) {
    match value {
        Value::Object(object) => {
            if let (Some(Value::String(code)), Some(Value::String(message))) =
                (object.get("code"), object.get("message"))
            {
                out.push(FieldError {
                    path: path.clone(),
                    code: code.clone(),
                    message: message.clone(),
                });
                return;
            }
            for (key, child) in object {
                let length = path.len();
                if !key.starts_with('_') {
                    if path.is_empty() {
                        path.push_str(key);
                    } else if key.bytes().all(|byte| byte.is_ascii_digit()) {
                        path.push('[');
                        path.push_str(key);
                        path.push(']');
                    } else {
                        path.push('.');
                        path.push_str(key);
                    }
                }
                flatten_errors(child, path, out);
                path.truncate(length);
            }
        }
        Value::Array(items) => {
            for item in items {
                flatten_errors(item, path, out);
            }
        }
        Value::String(message) => out.push(FieldError {
            path: path.clone(),
            code: String::new(),
            message: message.clone(),
        }),
        _ => {}
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

/// The error for a `202 Accepted` answer of Search Guild Messages (D-104), whose body looks
/// like `{"message": …, "code": 110000, "documents_indexed": 0, "retry_after": 2}`.
pub(crate) fn not_yet_indexed(body: &[u8]) -> Error {
    #[derive(Deserialize, Default)]
    struct NotIndexed {
        #[serde(default)]
        retry_after: f64,
        #[serde(default)]
        documents_indexed: u64,
    }

    let body: NotIndexed = serde_json::from_slice(body).unwrap_or_default();
    Error::new(ErrorKind::NotYetIndexed {
        retry_after: seconds(body.retry_after),
        documents_indexed: body.documents_indexed,
    })
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
                code: ErrorCode::MissingPermissions,
                message: "Missing Permissions".into(),
                field_errors: Vec::new(),
                errors: None,
            })),
        });
        assert_eq!(
            error.to_string(),
            "Discord responded with 403 Forbidden: Missing Permissions (code 50013)"
        );
        assert_eq!(error.status(), Some(StatusCode::FORBIDDEN));
        assert_eq!(
            error.api_error().unwrap().code,
            ErrorCode::MissingPermissions
        );
    }

    fn parse(json: &str) -> ApiError {
        serde_json::from_str(json).unwrap()
    }

    fn lines(error: &ApiError) -> Vec<String> {
        error.field_errors.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn field_errors_of_the_docs_examples() {
        // The docs' "Array Error", "Object Error" and "Request Error" examples.
        let array = parse(
            r#"{"code": 50035, "errors": {"activities": {"0": {
                "platform": {"_errors": [{"code": "BASE_TYPE_CHOICES",
                    "message": "Value must be one of ('desktop', 'android', 'ios')."}]},
                "type": {"_errors": [{"code": "BASE_TYPE_CHOICES",
                    "message": "Value must be one of (0, 1, 2, 3, 4, 5)."}]}}}},
                "message": "Invalid Form Body"}"#,
        );
        assert_eq!(array.code, ErrorCode::InvalidFormBodyOrContentType);
        assert_eq!(
            lines(&array),
            [
                "activities[0].platform[BASE_TYPE_CHOICES]: Value must be one of ('desktop', 'android', 'ios').",
                "activities[0].type[BASE_TYPE_CHOICES]: Value must be one of (0, 1, 2, 3, 4, 5).",
            ]
        );
        assert!(array.errors.is_some());

        let object = parse(
            r#"{"code": 50035, "errors": {"access_token": {"_errors": [
                {"code": "BASE_TYPE_REQUIRED", "message": "This field is required"}]}},
                "message": "Invalid Form Body"}"#,
        );
        assert_eq!(
            object.to_string(),
            "Invalid Form Body (code 50035)\naccess_token[BASE_TYPE_REQUIRED]: This field is required"
        );

        let request = parse(
            r#"{"code": 50035, "message": "Invalid Form Body", "errors": {"_errors": [
                {"code": "APPLICATION_COMMAND_TOO_LARGE",
                 "message": "Command exceeds maximum size (8000)"}]}}"#,
        );
        assert_eq!(request.field_errors[0].path, "");
        assert_eq!(
            lines(&request),
            ["APPLICATION_COMMAND_TOO_LARGE: Command exceeds maximum size (8000)"]
        );
    }

    #[test]
    fn unusual_error_shapes() {
        // Top-level indices (bulk command overwrites), a field named `code`, a plain string,
        // and an unknown error code.
        let error = parse(
            r#"{"code": 123456, "message": "Something", "errors": {
                "0": {"code": {"_errors": [{"code": "NUMBER_TYPE_MAX", "message": "Too big"}]}},
                "name": "Must be set",
                "other": 5}}"#,
        );
        assert_eq!(error.code, ErrorCode::Unknown(123_456));
        assert_eq!(
            lines(&error),
            ["0.code[NUMBER_TYPE_MAX]: Too big", "name: Must be set"]
        );
        let plain = parse(r#"{"code": 0, "message": "404: Not Found"}"#);
        assert_eq!(plain.code, ErrorCode::GeneralError);
        assert!(plain.field_errors.is_empty() && plain.errors.is_none());
        assert_eq!(plain.to_string(), "404: Not Found (code 0)");
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
