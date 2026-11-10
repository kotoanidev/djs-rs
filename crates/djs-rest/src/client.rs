//! The [`Rest`] client and its builder.

use core::{fmt, time::Duration};
use std::sync::Arc;

use bytes::Bytes;
use http::{
    HeaderMap, HeaderValue, StatusCode, Uri,
    header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
};
use http_body_util::{BodyExt, Full};
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client as HyperClient, connect::HttpConnector},
    rt::{TokioExecutor, TokioTimer},
};
use serde::{Serialize, de::DeserializeOwned};
use tokio::time::Instant;

use crate::{
    error::{ApiError, BoxError, Error, ErrorKind, RateLimitedBody},
    ratelimit::{RateLimitHeaders, RateLimiter},
    route::Route,
};

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
    http: HyperClient<Connector, Full<Bytes>>,
    token: Token,
    /// `{api}/v{version}`, without a trailing slash.
    base: String,
    user_agent: HeaderValue,
    timeout: Duration,
    retries: u8,
    ratelimiter: RateLimiter,
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
    pub async fn request<T: DeserializeOwned>(&self, request: Request) -> Result<T, Error> {
        let body = self.execute(request).await?;
        // An empty body (204 No Content) deserializes like `null`, which fits `()` and `Option`.
        let body: &[u8] = if body.is_empty() { b"null" } else { &body };
        serde_json::from_slice(body)
            .map_err(|source| Error::with_source(ErrorKind::Deserialize, source))
    }

    /// Sends a request and returns the raw response body on success.
    async fn execute(&self, request: Request) -> Result<Bytes, Error> {
        let inner = &*self.inner;
        let route = request.route;
        let bucket_route = route.bucket();
        let method = route.method();
        let uri = inner.uri(route)?;

        let mut attempt: u8 = 0;
        loop {
            let bucket = inner.ratelimiter.bucket(bucket_route);
            let mut state = bucket.state.lock().await;
            state.wait(inner.ratelimiter.offset()).await;
            inner.ratelimiter.acquire_global().await;

            let http_request = inner.build_request(&method, &uri, request.body.as_ref())?;
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
                        tracing::warn!(%method, %uri, attempt, %error, "request failed, retrying");
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
            state.update(&headers, now);
            tracing::debug!(
                %method,
                %uri,
                status = response.status.as_u16(),
                elapsed_ms = millis(started.elapsed()),
                remaining = headers.remaining,
                "REST request completed"
            );

            match response.status {
                status if status.is_success() => return Ok(response.body),
                StatusCode::TOO_MANY_REQUESTS => {
                    let (retry_after, global) = rate_limited(&response.body, &headers);
                    tracing::warn!(
                        %method,
                        %uri,
                        global,
                        scope = headers.scope,
                        retry_after_ms = millis(retry_after),
                        "rate limited by Discord, retrying"
                    );
                    if global {
                        inner.ratelimiter.pause_global(retry_after);
                    } else {
                        state.exhaust_for(retry_after, now);
                    }
                }
                status if status.is_server_error() && attempt < inner.retries => {
                    attempt += 1;
                    tracing::warn!(%method, %uri, status = status.as_u16(), attempt, "server error, retrying");
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
    /// Builds the full URI of a route.
    fn uri(&self, route: Route) -> Result<Uri, Error> {
        let mut uri = String::with_capacity(self.base.len() + 64);
        uri.push_str(&self.base);
        route.write_path(&mut uri);
        Uri::try_from(uri).map_err(|source| Error::with_source(ErrorKind::BuildRequest, source))
    }

    /// Builds one HTTP request attempt. Cloning the body is cheap (`Bytes` is reference counted).
    fn build_request(
        &self,
        method: &http::Method,
        uri: &Uri,
        body: Option<&Bytes>,
    ) -> Result<http::Request<Full<Bytes>>, Error> {
        let mut builder = http::Request::builder()
            .method(method.clone())
            .uri(uri.clone())
            .header(USER_AGENT, self.user_agent.clone());
        match &self.token {
            Token::None => {}
            Token::Valid(token) => builder = builder.header(AUTHORIZATION, token.clone()),
            Token::Invalid => {
                return Err(Error::with_source(
                    ErrorKind::BuildRequest,
                    "the token contains characters that are not allowed in an HTTP header",
                ));
            }
        }
        if body.is_some() {
            builder = builder.header(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        builder
            .body(Full::new(body.cloned().unwrap_or_default()))
            .map_err(|source| Error::with_source(ErrorKind::BuildRequest, source))
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
    http: &HyperClient<Connector, Full<Bytes>>,
    request: http::Request<Full<Bytes>>,
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
#[derive(Clone, Debug)]
pub struct Request {
    route: Route,
    body: Option<Bytes>,
}

impl Request {
    /// Creates a request without a body.
    #[must_use]
    pub const fn new(route: Route) -> Self {
        Self { route, body: None }
    }

    /// Sets a JSON body.
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
    pub const fn route(&self) -> Route {
        self.route
    }
}

/// Configures and builds a [`Rest`] client.
///
/// Defaults match `@discordjs/rest`: API `https://discord.com/api`, version 10, 15 s timeout,
/// 3 retries, 50 requests per second globally, 50 ms rate-limit offset.
///
/// discord.js counterpart: `RESTOptions`.
#[derive(Debug)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct RestBuilder {
    token: Option<String>,
    api: String,
    version: u8,
    timeout: Duration,
    retries: u8,
    global_requests_per_second: u32,
    offset: Duration,
    user_agent_appendix: Option<String>,
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
        }
    }
}

impl RestBuilder {
    /// Sets the bot token (without the `Bot ` prefix).
    ///
    /// Without a token, only unauthenticated endpoints (webhooks with tokens, interaction
    /// callbacks) can be used.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
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
            .with_provider_and_webpki_roots(rustls::crypto::ring::default_provider())
            .expect("the ring provider supports the default protocol versions")
            .https_or_http()
            .enable_all_versions()
            .build();
        let http = HyperClient::builder(TokioExecutor::new())
            .timer(TokioTimer::new())
            .pool_timer(TokioTimer::new())
            .build(connector);

        let token = match self.token {
            None => Token::None,
            Some(token) => match HeaderValue::try_from(format!("Bot {token}")) {
                Ok(mut value) => {
                    value.set_sensitive(true);
                    Token::Valid(value)
                }
                Err(_) => Token::Invalid,
            },
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
            }),
        }
    }
}
