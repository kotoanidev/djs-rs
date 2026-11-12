//! The [`Rest`] client and its builder.

use core::{fmt, time::Duration};
use std::sync::Arc;

use bytes::Bytes;
use djs_model::rest::FileUpload;
use http::{
    HeaderMap, HeaderName, HeaderValue, StatusCode, Uri,
    header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
};
use http_body_util::BodyExt;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client as HyperClient, connect::HttpConnector},
    rt::{TokioExecutor, TokioTimer},
};
use serde::{Serialize, de::DeserializeOwned};
use tokio::time::Instant;

use crate::{
    error::{ApiError, BoxError, Error, ErrorKind, RateLimitedBody},
    multipart::{Form, PreparedBody, RequestBody},
    query::{Query, QueryWriter},
    ratelimit::{InvalidRequests, RateLimitHeaders, RateLimiter},
    route::{self, Route},
};

/// The header that carries the [audit log reason](Request::reason).
const AUDIT_LOG_REASON: HeaderName = HeaderName::from_static("x-audit-log-reason");

/// The default API base URL.
pub const DEFAULT_API: &str = "https://discord.com/api";

type Connector = HttpsConnector<HttpConnector>;

/// A client for the Discord REST API.
///
/// `Rest` handles authentication, [rate limits](https://docs.discord.com/developers/topics/rate-limits),
/// retries and timeouts. It is cheap to [`Clone`] — clones share the same connection pool and
/// rate-limit state, so create one per bot and pass clones around.
///
/// HTTP/2 is negotiated automatically, so concurrent requests share a single connection.
///
/// discord.js counterpart: `REST` from `@discordjs/rest`.
///
/// # Examples
///
/// ```no_run
/// use djs_model::id::ChannelId;
/// use djs_rest::Rest;
///
/// # async fn example() -> Result<(), djs_rest::Error> {
/// let rest = Rest::new(std::env::var("DISCORD_TOKEN").unwrap());
///
/// let gateway = rest.get_gateway_bot().await?;
/// println!("recommended shards: {}", gateway.shards);
///
/// rest.create_message(ChannelId::new(1_234_567_890_123_456_789), &"Hello!".into())
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Rest {
    inner: Arc<Inner>,
}

struct Inner {
    http: HyperClient<Connector, RequestBody>,
    token: Token,
    /// `{api}/v{version}`, without a trailing slash.
    base: String,
    user_agent: HeaderValue,
    timeout: Duration,
    retries: u8,
    ratelimiter: RateLimiter,
    invalid: InvalidRequests,
    /// `0` = no fail-fast limit.
    invalid_request_limit: u32,
}

/// The authorization header, validated once at construction.
enum Token {
    None,
    Valid(HeaderValue),
    Invalid,
}

impl fmt::Debug for Rest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rest")
            .field("base", &self.inner.base)
            .field(
                "token",
                &match self.inner.token {
                    Token::None => "<none>",
                    Token::Valid(_) => "<redacted>",
                    Token::Invalid => "<invalid>",
                },
            )
            .field("timeout", &self.inner.timeout)
            .field("retries", &self.inner.retries)
            .finish_non_exhaustive()
    }
}

impl Rest {
    /// Creates a client authenticated with a bot token, using default options.
    ///
    /// Pass the raw token, without the `Bot ` prefix. An invalid token (one containing characters
    /// that cannot appear in an HTTP header) is reported by the first request as
    /// [`ErrorKind::BuildRequest`].
    ///
    /// discord.js counterpart: `new REST().setToken(token)`.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self::builder().token(token).build()
    }

    /// Returns a builder to configure a client.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let rest = djs_rest::Rest::builder()
    ///     .token("my-token")
    ///     .timeout(Duration::from_secs(10))
    ///     .retries(5)
    ///     .build();
    /// # let _ = rest;
    /// ```
    pub fn builder() -> RestBuilder {
        RestBuilder::default()
    }

    /// Returns the number of invalid requests (401, 403 and 429 responses, except 429s with the
    /// `shared` scope) this client made in the last 10 minutes.
    ///
    /// Discord temporarily bans the IP (for about an hour) once it sees 10 000 of them in
    /// 10 minutes. The count is kept in 10-second slots, so it may include requests up to 10 s
    /// older than the window. A warning is logged at every 1 000; see also
    /// [`RestBuilder::invalid_request_limit`].
    ///
    /// discord.js counterpart: the `invalidRequestWarning` event.
    #[must_use]
    pub fn invalid_requests(&self) -> u32 {
        self.inner.invalid.count(Instant::now())
    }

    /// Sends a request and deserializes the JSON response into `T`.
    ///
    /// This is the low-level entry point used by every typed method. Use `T = ()` for endpoints
    /// that return `204 No Content`.
    ///
    /// Requests are rate limited and retried automatically:
    ///
    /// - `429 Too Many Requests` is retried after the time Discord asks for (it does not count
    ///   towards the retry limit);
    /// - `5xx` responses, transport errors and timeouts are retried up to
    ///   [`RestBuilder::retries`] times.
    ///
    /// discord.js counterpart: `REST#request` / `REST#get` / `REST#post`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the request cannot be built or sent, Discord responds with an error
    /// status, or the response body does not match `T`.
    pub async fn request<T: DeserializeOwned>(&self, request: Request<'_>) -> Result<T, Error> {
        let (_, body) = self.execute(request).await?;
        // An empty body (204 No Content) deserializes like `null`, which fits `()` and `Option`.
        let body: &[u8] = if body.is_empty() { b"null" } else { &body };
        serde_json::from_slice(body)
            .map_err(|source| Error::with_source(ErrorKind::Deserialize, source))
    }

    /// Sends a request and returns the raw response body on success.
    ///
    /// Log fields name the route template, never the URI: paths can contain tokens.
    pub(crate) async fn execute(&self, request: Request<'_>) -> Result<(StatusCode, Bytes), Error> {
        let inner = &*self.inner;
        let route = request.route;
        let bucket_route = route.bucket();
        let limits = bucket_route.limits;
        let template = bucket_route.template;
        let uri = inner.uri(route, request.query)?;
        let reason = request.reason.map(reason_header);
        let body = request.prepare_body();

        let mut attempt: u8 = 0;
        loop {
            inner.check_invalid_requests()?;
            let bucket = inner.ratelimiter.bucket(bucket_route);
            // Burst routes (interaction callbacks) do not wait for one another.
            let mut state = if limits.burst {
                None
            } else {
                let mut state = bucket.state.lock().await;
                state.wait(inner.ratelimiter.offset()).await;
                Some(state)
            };
            if limits.global {
                inner.ratelimiter.acquire_global().await;
            }

            let http_request = inner.build_request(route, &uri, reason.as_ref(), &body)?;
            let started = Instant::now();
            let response = match tokio::time::timeout(
                inner.timeout,
                send(&inner.http, http_request),
            )
            .await
            {
                Ok(Ok(response)) => response,
                failed => {
                    drop(state);
                    let error = match failed {
                        Ok(Err(source)) => Error::with_source(ErrorKind::Transport, source),
                        _ => Error::new(ErrorKind::Timeout),
                    };
                    if attempt < inner.retries {
                        attempt += 1;
                        tracing::warn!(route = template, attempt, %error, "request failed, retrying");
                        continue;
                    }
                    return Err(error);
                }
            };

            let now = Instant::now();
            let headers = RateLimitHeaders::parse(&response.headers);
            if let Some(hash) = headers.bucket {
                inner.ratelimiter.learn_hash(bucket_route.template, hash);
            }
            if let Some(state) = &mut state {
                state.update(&headers, now);
            }
            if is_invalid(response.status, &headers) {
                inner.invalid.record(now);
            }
            tracing::debug!(
                route = template,
                status = response.status.as_u16(),
                elapsed_ms = millis(started.elapsed()),
                remaining = headers.remaining,
                "REST request completed"
            );

            match response.status {
                status if status.is_success() => return Ok((status, response.body)),
                StatusCode::TOO_MANY_REQUESTS => {
                    let (retry_after, global) = rate_limited(&response.body, &headers);
                    tracing::warn!(
                        route = template,
                        global,
                        scope = headers.scope,
                        retry_after_ms = millis(retry_after),
                        "rate limited by Discord, retrying"
                    );
                    match &mut state {
                        _ if global && limits.global => inner.ratelimiter.pause_global(retry_after),
                        Some(state) => state.exhaust_for(retry_after, now),
                        // A burst route only delays the request that was limited.
                        None => tokio::time::sleep(retry_after + inner.ratelimiter.offset()).await,
                    }
                }
                status if status.is_server_error() && attempt < inner.retries => {
                    attempt += 1;
                    tracing::warn!(
                        route = template,
                        status = status.as_u16(),
                        attempt,
                        "server error, retrying"
                    );
                }
                status => {
                    let error = serde_json::from_slice::<ApiError>(&response.body)
                        .ok()
                        .map(Box::new);
                    return Err(Error::new(ErrorKind::Response { status, error }));
                }
            }
        }
    }
}

impl Inner {
    /// Fails if the configured invalid-request limit is reached (D-085).
    fn check_invalid_requests(&self) -> Result<(), Error> {
        if self.invalid_request_limit == 0 {
            return Ok(());
        }
        let count = self.invalid.count(Instant::now());
        if count >= self.invalid_request_limit {
            return Err(Error::new(ErrorKind::InvalidRequestLimit {
                count,
                limit: self.invalid_request_limit,
            }));
        }
        Ok(())
    }

    /// Builds the full URI of a route.
    fn uri(&self, route: Route<'_>, query: Option<&dyn Query>) -> Result<Uri, Error> {
        let mut uri = String::with_capacity(self.base.len() + 64);
        uri.push_str(&self.base);
        route.write_path(&mut uri);
        if let Some(query) = query {
            query.write(&mut QueryWriter::new(&mut uri));
        }
        Uri::try_from(uri).map_err(|source| Error::with_source(ErrorKind::BuildRequest, source))
    }

    /// Builds one HTTP request attempt. Cloning the body is cheap (`Bytes` is reference counted).
    /// Routes authenticated by a token in their path do not get the client's token (D-121).
    fn build_request(
        &self,
        route: Route<'_>,
        uri: &Uri,
        reason: Option<&HeaderValue>,
        body: &PreparedBody,
    ) -> Result<http::Request<RequestBody>, Error> {
        let mut builder = http::Request::builder()
            .method(route.method())
            .uri(uri.clone())
            .header(USER_AGENT, self.user_agent.clone());
        match &self.token {
            _ if !route.authenticated() => {}
            Token::None => {}
            Token::Valid(token) => builder = builder.header(AUTHORIZATION, token.clone()),
            Token::Invalid => {
                return Err(Error::with_source(
                    ErrorKind::BuildRequest,
                    "the token contains characters that are not allowed in an HTTP header",
                ));
            }
        }
        if let Some(reason) = reason {
            builder = builder.header(AUDIT_LOG_REASON, reason.clone());
        }
        if let Some(content_type) = body.content_type() {
            builder = builder.header(CONTENT_TYPE, content_type.clone());
        }
        builder
            .body(body.body())
            .map_err(|source| Error::with_source(ErrorKind::BuildRequest, source))
    }
}

/// Encodes an audit log reason for its header: percent-encoded UTF-8, which Discord decodes.
fn reason_header(reason: &str) -> HeaderValue {
    let mut encoded = String::with_capacity(reason.len());
    route::write_text(&mut encoded, reason);
    // Percent-encoding leaves only visible ASCII, which is always a valid header value.
    HeaderValue::try_from(encoded).unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// Whether Discord counts a response towards its invalid-request limit: 401, 403, and 429
/// unless the limit was a `shared` resource limit.
fn is_invalid(status: StatusCode, headers: &RateLimitHeaders<'_>) -> bool {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => true,
        StatusCode::TOO_MANY_REQUESTS => !headers
            .scope
            .is_some_and(|scope| scope.eq_ignore_ascii_case("shared")),
        _ => false,
    }
}

/// Extracts how long to wait and whether the limit is global from a 429 response.
fn rate_limited(body: &[u8], headers: &RateLimitHeaders<'_>) -> (Duration, bool) {
    let body: Option<RateLimitedBody> = serde_json::from_slice(body).ok();
    let retry_after = body
        .as_ref()
        .map(RateLimitedBody::retry_after)
        .or(headers.retry_after)
        .unwrap_or(Duration::from_secs(1));
    let global = headers.global || body.is_some_and(|body| body.global);
    (retry_after, global)
}

/// Whole milliseconds of a duration, saturating, for log fields.
fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

struct RawResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

async fn send(
    http: &HyperClient<Connector, RequestBody>,
    request: http::Request<RequestBody>,
) -> Result<RawResponse, BoxError> {
    let response = http.request(request).await?;
    let (parts, body) = response.into_parts();
    let body = body.collect().await?.to_bytes();
    Ok(RawResponse {
        status: parts.status,
        headers: parts.headers,
        body,
    })
}

/// A request to send with [`Rest::request`].
///
/// # Examples
///
/// ```
/// use djs_model::{id::ChannelId, rest::CreateMessage};
/// use djs_rest::{Request, Route};
///
/// let request = Request::new(Route::CreateMessage { channel_id: ChannelId::new(1) })
///     .json(&CreateMessage::from("hi"))
///     .unwrap();
/// assert_eq!(request.route(), Route::CreateMessage { channel_id: ChannelId::new(1) });
/// ```
#[derive(Clone)]
pub struct Request<'a> {
    route: Route<'a>,
    reason: Option<&'a str>,
    query: Option<&'a dyn Query>,
    body: Option<Bytes>,
    /// Whether [`body`](Self::body) is `application/x-www-form-urlencoded` instead of JSON.
    urlencoded: bool,
    files: &'a [FileUpload],
    /// A file sent under its own form field name instead of `files[n]` (D-111).
    form_file: Option<(&'static str, &'a FileUpload)>,
    /// Text form fields, for endpoints without `payload_json` (Create Guild Sticker).
    form_text: &'a [(&'static str, &'a str)],
}

impl<'a> Request<'a> {
    /// Creates a request without a body.
    #[must_use]
    pub const fn new(route: Route<'a>) -> Self {
        Self {
            route,
            reason: None,
            query: None,
            body: None,
            urlencoded: false,
            files: &[],
            form_file: None,
            form_text: &[],
        }
    }

    /// Sets the reason shown in the guild's
    /// [audit log](https://docs.discord.com/developers/resources/audit-log) for the action
    /// (the `X-Audit-Log-Reason` header, sent percent-encoded, so any text is allowed). Only
    /// endpoints whose docs mention the header use it; `None` sends no reason.
    ///
    /// discord.js counterpart: the `reason` option of `REST#request`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::id::{ChannelId, MessageId};
    /// use djs_rest::{Request, Route};
    ///
    /// let route = Route::PinMessage {
    ///     channel_id: ChannelId::new(1),
    ///     message_id: MessageId::new(2),
    /// };
    /// let request = Request::new(route).reason(Some("Important"));
    /// assert_eq!(request.audit_log_reason(), Some("Important"));
    /// ```
    #[must_use]
    pub const fn reason(mut self, reason: Option<&'a str>) -> Self {
        self.reason = reason;
        self
    }

    /// Uploads `files` with the request (D-087). The request is then sent as
    /// `multipart/form-data`: file `n` as the part `files[n]` (with its content type, guessed
    /// from its name if not given, D-095) and the JSON body as `payload_json`. The file data is
    /// not copied, not even for retries.
    ///
    /// The JSON body must describe the files in its `attachments`; the body structs of
    /// `djs_model::rest` do that automatically for their own `files` field (D-094), which the
    /// typed [`Rest`] methods pass here.
    ///
    /// discord.js counterpart: the `files` option of `REST#request`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::{
    ///     id::ChannelId,
    ///     rest::{CreateMessage, FileUpload},
    /// };
    /// use djs_rest::{Request, Route};
    ///
    /// let mut body = CreateMessage::from("The log:");
    /// body.files.push(FileUpload::new("log.txt", "everything is fine\n"));
    /// let request = Request::new(Route::CreateMessage { channel_id: ChannelId::new(1) })
    ///     .json(&body)
    ///     .unwrap()
    ///     .files(&body.files);
    /// assert_eq!(request.uploads().len(), 1);
    /// ```
    #[must_use]
    pub const fn files(mut self, files: &'a [FileUpload]) -> Self {
        self.files = files;
        self
    }

    /// Returns the files set with [`files`](Self::files).
    #[must_use]
    pub const fn uploads(&self) -> &'a [FileUpload] {
        self.files
    }

    /// Uploads `file` as the form field `name` (e.g. `target_users_file`, D-111), which
    /// makes the request `multipart/form-data` like [`files`](Self::files).
    pub(crate) const fn form_file(mut self, name: &'static str, file: &'a FileUpload) -> Self {
        self.form_file = Some((name, file));
        self
    }

    /// Sets an `application/x-www-form-urlencoded` body (the `OAuth2` token endpoints, D-132).
    pub(crate) fn form_urlencoded(mut self, body: String) -> Self {
        self.body = Some(Bytes::from(body));
        self.urlencoded = true;
        self
    }

    /// Builds the body once, for every attempt.
    fn prepare_body(self) -> PreparedBody {
        if self.urlencoded {
            return PreparedBody::urlencoded(self.body.unwrap_or_default());
        }
        let form = Form {
            files: self.files,
            file: self.form_file,
            text: self.form_text,
        };
        PreparedBody::new(self.body, &form)
    }

    /// Sends `fields` as text form fields, which makes the request `multipart/form-data`
    /// (for endpoints that do not read `payload_json`).
    pub(crate) const fn form_text(mut self, fields: &'a [(&'static str, &'a str)]) -> Self {
        self.form_text = fields;
        self
    }

    /// Sets the query parameters (D-092), written into the URI when the request is sent.
    pub(crate) const fn query(mut self, query: &'a dyn Query) -> Self {
        self.query = Some(query);
        self
    }

    /// Returns the audit log reason set with [`reason`](Self::reason).
    #[must_use]
    pub const fn audit_log_reason(&self) -> Option<&'a str> {
        self.reason
    }

    /// Sets a JSON body. With [files](Self::files), it is sent as the `payload_json` part.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::BuildRequest`] if `body` cannot be serialized.
    pub fn json<B: Serialize + ?Sized>(mut self, body: &B) -> Result<Self, Error> {
        let bytes = serde_json::to_vec(body)
            .map_err(|source| Error::with_source(ErrorKind::BuildRequest, source))?;
        self.body = Some(Bytes::from(bytes));
        Ok(self)
    }

    /// Returns the route of the request.
    #[must_use]
    pub const fn route(&self) -> Route<'a> {
        self.route
    }
}

/// Shows the route, the reason and the query, but only the size of the body and the names of
/// the form parts: bodies can hold secrets (access tokens, client secrets; C-011).
impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Size(usize);
        impl fmt::Debug for Size {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "<{} bytes>", self.0)
            }
        }
        let files: Vec<&str> = self.files.iter().map(|file| file.name.as_str()).collect();
        let text: Vec<&str> = self.form_text.iter().map(|(name, _)| *name).collect();
        f.debug_struct("Request")
            .field("route", &self.route)
            .field("reason", &self.reason)
            .field("query", &self.query)
            .field("body", &self.body.as_ref().map(|body| Size(body.len())))
            .field("urlencoded", &self.urlencoded)
            .field("files", &files)
            .field("form_file", &self.form_file.map(|(name, _)| name))
            .field("form_text", &text)
            .finish()
    }
}

/// Configures and builds a [`Rest`] client.
///
/// Defaults match `@discordjs/rest`: API `https://discord.com/api`, version 10, 15 s timeout,
/// 3 retries, 50 requests per second globally, 50 ms rate-limit offset.
///
/// discord.js counterpart: `RESTOptions`.
#[must_use = "a builder does nothing until `build` is called"]
pub struct RestBuilder {
    token: Option<(AuthScheme, String)>,
    api: String,
    version: u8,
    timeout: Duration,
    retries: u8,
    global_requests_per_second: u32,
    offset: Duration,
    user_agent_appendix: Option<String>,
    invalid_request_limit: u32,
}

/// How the token authenticates: a bot token or an `OAuth2` access token (D-108).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthScheme {
    Bot,
    Bearer,
}

impl AuthScheme {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Bot => "Bot",
            Self::Bearer => "Bearer",
        }
    }
}

impl fmt::Debug for RestBuilder {
    /// Never shows the token (C-011).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RestBuilder")
            .field(
                "token",
                &match self.token {
                    None => "<none>",
                    Some((AuthScheme::Bot, _)) => "Bot <redacted>",
                    Some((AuthScheme::Bearer, _)) => "Bearer <redacted>",
                },
            )
            .field("api", &self.api)
            .field("version", &self.version)
            .field("timeout", &self.timeout)
            .field("retries", &self.retries)
            .field(
                "global_requests_per_second",
                &self.global_requests_per_second,
            )
            .field("offset", &self.offset)
            .field("user_agent_appendix", &self.user_agent_appendix)
            .field("invalid_request_limit", &self.invalid_request_limit)
            .finish()
    }
}

impl Default for RestBuilder {
    fn default() -> Self {
        Self {
            token: None,
            api: DEFAULT_API.to_owned(),
            version: djs_model::gateway::API_VERSION,
            timeout: Duration::from_secs(15),
            retries: 3,
            global_requests_per_second: 50,
            offset: Duration::from_millis(50),
            user_agent_appendix: None,
            invalid_request_limit: 0,
        }
    }
}

impl RestBuilder {
    /// Sets the bot token (without the `Bot ` prefix).
    ///
    /// Without a token, only unauthenticated endpoints (webhooks with tokens, interaction
    /// callbacks) can be used. Replaces a [`bearer_token`](Self::bearer_token).
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some((AuthScheme::Bot, token.into()));
        self
    }

    /// Authenticates with an `OAuth2` access token (`Authorization: Bearer …`) instead of a
    /// bot token (D-108), to act for the user who authorized the application — e.g. for
    /// [Edit Application Command Permissions](crate::Rest::edit_application_command_permissions).
    /// Replaces a [`token`](Self::token).
    ///
    /// Discord limits requests per token, so use one client per access token: its
    /// rate-limit state is its own.
    ///
    /// discord.js counterpart: `RESTOptions#authPrefix` set to `'Bearer'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_rest::Rest;
    ///
    /// let user_rest = Rest::builder().bearer_token("an-oauth2-access-token").build();
    /// assert!(!format!("{user_rest:?}").contains("an-oauth2-access-token"));
    /// ```
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some((AuthScheme::Bearer, token.into()));
        self
    }

    /// Sets the API base URL, without the version (default `https://discord.com/api`).
    ///
    /// Useful to route requests through a proxy. Plain `http://` URLs are allowed for that
    /// purpose.
    pub fn api(mut self, api: impl Into<String>) -> Self {
        self.api = api.into();
        self
    }

    /// Sets the API version (default 10).
    pub const fn version(mut self, version: u8) -> Self {
        self.version = version;
        self
    }

    /// Sets how long to wait for a complete response before giving up (default 15 s).
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets how many times a request is retried after a server error, transport error or timeout
    /// (default 3). Rate-limited requests are always retried and do not count.
    pub const fn retries(mut self, retries: u8) -> Self {
        self.retries = retries;
        self
    }

    /// Sets the maximum number of requests per second across all routes (default 50).
    ///
    /// The limit is strict: no sliding second ever contains more requests (the window is
    /// extended by the [offset](Self::offset)), while a burst of up to `limit` requests is sent
    /// at once. Unlike discord.js' fixed one-second windows, this never lets twice the limit
    /// through around a window boundary.
    ///
    /// `0` disables the proactive global limit; global rate-limit responses from Discord are
    /// still honoured.
    pub const fn global_requests_per_second(mut self, limit: u32) -> Self {
        self.global_requests_per_second = limit;
        self
    }

    /// Sets the extra delay added to every rate-limit wait to absorb clock skew and latency
    /// (default 50 ms).
    pub const fn offset(mut self, offset: Duration) -> Self {
        self.offset = offset;
        self
    }

    /// Makes requests fail immediately with [`ErrorKind::InvalidRequestLimit`] once the client
    /// made `limit` invalid requests (401, 403, 429) in the last 10 minutes (default `0`: no
    /// limit). Discord temporarily bans the IP at 10 000, so a limit below that (for example
    /// 9 000) stops a misbehaving bot before it is locked out (D-085).
    ///
    /// The count is shared by every clone of the client; see [`Rest::invalid_requests`].
    pub const fn invalid_request_limit(mut self, limit: u32) -> Self {
        self.invalid_request_limit = limit;
        self
    }

    /// Appends text to the `User-Agent` header, e.g. your bot's name.
    pub fn user_agent_appendix(mut self, appendix: impl Into<String>) -> Self {
        self.user_agent_appendix = Some(appendix.into());
        self
    }

    /// Builds the client.
    ///
    /// # Panics
    ///
    /// Panics if `user_agent_appendix` contains characters that are not allowed in an HTTP
    /// header.
    #[must_use]
    pub fn build(self) -> Rest {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(crate::tls::client_config())
            .https_or_http()
            .enable_all_versions()
            .build();
        let http = HyperClient::builder(TokioExecutor::new())
            .timer(TokioTimer::new())
            .pool_timer(TokioTimer::new())
            .build(connector);

        let token = match self.token {
            None => Token::None,
            Some((scheme, token)) => {
                match HeaderValue::try_from(format!("{} {token}", scheme.prefix())) {
                    Ok(mut value) => {
                        value.set_sensitive(true);
                        Token::Valid(value)
                    }
                    Err(_) => Token::Invalid,
                }
            }
        };

        let mut user_agent = format!(
            "DiscordBot (https://github.com/kotoanidev/djs-rs, {})",
            env!("CARGO_PKG_VERSION")
        );
        if let Some(appendix) = &self.user_agent_appendix {
            user_agent.push(' ');
            user_agent.push_str(appendix);
        }

        Rest {
            inner: Arc::new(Inner {
                http,
                token,
                base: format!("{}/v{}", self.api.trim_end_matches('/'), self.version),
                user_agent: HeaderValue::try_from(user_agent)
                    .expect("the user agent appendix contains invalid header characters"),
                timeout: self.timeout,
                retries: self.retries,
                ratelimiter: RateLimiter::new(self.global_requests_per_second, self.offset),
                invalid: InvalidRequests::new(),
                invalid_request_limit: self.invalid_request_limit,
            }),
        }
    }
}
