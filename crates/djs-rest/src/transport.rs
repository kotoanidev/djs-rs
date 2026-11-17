//! The HTTP transport under [`Rest`](crate::Rest) (D-396, D-404): what sends a request and
//! waits.
//!
//! [`Rest`](crate::Rest) builds every request (route, headers, body, rate limits, retries)
//! and hands it to a [`Transport`] to send. With the default `hyper` feature the transport is
//! hyper with rustls, so nothing needs to be set. Elsewhere — on `wasm32-unknown-unknown`,
//! such as Cloudflare Workers, which send HTTP with the platform's `fetch` — implement
//! [`Transport`] and pass it to [`RestBuilder::transport`](crate::RestBuilder::transport).
//!
//! discord.js counterpart: the `makeRequest` option of `RESTOptions` (`@discordjs/rest`),
//! which replaces `undici`'s `request`.

use core::{fmt, future::Future, pin::Pin, time::Duration};
use std::error::Error;

use bytes::Bytes;

// `std::time` has no clock on `wasm32-unknown-unknown`; `web-time` reads the JS host's (D-397).
// Natively tokio's `Instant`, which tests can pause.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) use tokio::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) use web_time::Instant;

/// A boxed, `Send` future: what the [`Transport`] methods return.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Sends HTTP requests and waits for [`Rest`](crate::Rest) (D-404).
///
/// [`send`](Self::send) performs one request; [`Rest`](crate::Rest) does everything around
/// it (authentication, rate limits, retries) and gives up on it after its
/// [timeout](crate::RestBuilder::timeout), which it measures with [`sleep`](Self::sleep), as
/// it does every rate-limit and retry wait. Both futures must be `Send`: on a
/// single-threaded target whose futures are not (Cloudflare Workers' `fetch`), wrap them
/// (for example in `worker::send::SendFuture`).
///
/// discord.js counterpart: the `makeRequest` option of `RESTOptions` (`@discordjs/rest`).
///
/// # Examples
///
/// A transport that answers every request with `204 No Content` (useful in tests):
///
/// ```
/// use std::time::Duration;
///
/// use bytes::Bytes;
/// use djs_rest::{Body, BoxFuture, Rest, Transport};
///
/// struct NoContent;
///
/// impl Transport for NoContent {
///     fn send(
///         &self,
///         _request: http::Request<Body>,
///     ) -> BoxFuture<'_, Result<http::Response<Bytes>, Box<dyn std::error::Error + Send + Sync>>>
///     {
///         Box::pin(async {
///             Ok(http::Response::builder().status(204).body(Bytes::new())?)
///         })
///     }
///
///     fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
///         Box::pin(tokio::time::sleep(duration))
///     }
/// }
///
/// let rest = Rest::builder().token("my-token").transport(NoContent).build();
/// # let _ = rest;
/// ```
pub trait Transport: Send + Sync + 'static {
    /// Sends `request` and returns its response, with the whole body. A failure to get a
    /// response (connection, TLS, …) is an error, which [`Rest`](crate::Rest) retries like a
    /// server error; any HTTP status is a response.
    ///
    /// The request has its method, absolute URI, headers (with `Content-Type` when there is a
    /// body) and [`Body`]; `Content-Length` is [`Body::len`].
    ///
    /// discord.js counterpart: `makeRequest`.
    fn send(
        &self,
        request: http::Request<Body>,
    ) -> BoxFuture<'_, Result<http::Response<Bytes>, Box<dyn Error + Send + Sync>>>;

    /// Waits for `duration`: rate limits, retries and the request timeout.
    ///
    /// discord.js counterpart: none (Node's timers).
    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()>;
}

/// The body of a request a [`Transport`] sends: a sequence of byte chunks.
///
/// A JSON body is one chunk; a `multipart/form-data` body is its part headers and each file's
/// data as is, so files are not copied to send them (D-087). Cloning is cheap (the chunks are
/// reference counted).
///
/// discord.js counterpart: the `body` of `RequestInit` (`undici`).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Body {
    chunks: Vec<Bytes>,
}

impl Body {
    pub(crate) const fn new(chunks: Vec<Bytes>) -> Self {
        Self { chunks }
    }

    /// Returns the chunks, in order (some may be empty).
    #[must_use]
    pub fn chunks(&self) -> &[Bytes] {
        &self.chunks
    }

    /// Returns the length of the whole body, in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.iter().map(Bytes::len).sum()
    }

    /// Returns whether the body is empty (a request without a body).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(Bytes::is_empty)
    }

    /// Returns the whole body as one buffer: the chunk itself if there is one, else a copy
    /// of them all.
    #[must_use]
    pub fn into_bytes(mut self) -> Bytes {
        match self.chunks.len() {
            0 => Bytes::new(),
            1 => self.chunks.swap_remove(0),
            _ => self.chunks.concat().into(),
        }
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body")
            .field("chunks", &self.chunks.len())
            .field("len", &self.len())
            .finish()
    }
}

/// Waits with `transport` until `deadline`, if it is ahead.
pub(crate) async fn sleep_until(transport: &dyn Transport, deadline: Instant) {
    let now = Instant::now();
    if deadline > now {
        transport.sleep(deadline - now).await;
    }
}

/// The default transport: hyper with rustls (the `hyper` feature, D-405).
#[cfg(feature = "hyper")]
pub(crate) mod hyper {
    use core::{
        convert::Infallible,
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };
    use std::error::Error;

    use bytes::Bytes;
    use http_body_util::BodyExt;
    use hyper::body::{Frame, SizeHint};
    use hyper_rustls::HttpsConnector;
    use hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::{TokioExecutor, TokioTimer},
    };

    use super::{Body, BoxFuture, Transport};

    /// hyper's pooled client over HTTPS (HTTP/2 or HTTP/1.1) or plain HTTP.
    pub(crate) struct HyperTransport {
        client: Client<HttpsConnector<HttpConnector>, HyperBody>,
    }

    impl HyperTransport {
        pub(crate) fn new() -> Self {
            let connector = hyper_rustls::HttpsConnectorBuilder::new()
                .with_tls_config(crate::tls::client_config())
                .https_or_http()
                .enable_all_versions()
                .build();
            let client = Client::builder(TokioExecutor::new())
                .timer(TokioTimer::new())
                .pool_timer(TokioTimer::new())
                .build(connector);
            Self { client }
        }
    }

    impl Transport for HyperTransport {
        fn send(
            &self,
            request: http::Request<Body>,
        ) -> BoxFuture<'_, Result<http::Response<Bytes>, Box<dyn Error + Send + Sync>>> {
            let response = self.client.request(request.map(HyperBody::from));
            Box::pin(async move {
                let (parts, body) = response.await?.into_parts();
                let body = body.collect().await?.to_bytes();
                Ok(http::Response::from_parts(parts, body))
            })
        }

        fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
            Box::pin(tokio::time::sleep(duration))
        }
    }

    /// A [`Body`] for hyper: its chunks of known total length (so `Content-Length` is set).
    #[derive(Debug)]
    pub(crate) struct HyperBody {
        chunks: std::vec::IntoIter<Bytes>,
        remaining: u64,
    }

    impl From<Body> for HyperBody {
        fn from(body: Body) -> Self {
            Self {
                remaining: body.len() as u64,
                chunks: body.chunks.into_iter(),
            }
        }
    }

    impl hyper::body::Body for HyperBody {
        type Data = Bytes;
        type Error = Infallible;

        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
            let this = self.get_mut();
            let next = this.chunks.by_ref().find(|chunk| !chunk.is_empty());
            if let Some(chunk) = &next {
                this.remaining -= chunk.len() as u64;
            }
            Poll::Ready(next.map(|chunk| Ok(Frame::data(chunk))))
        }

        fn is_end_stream(&self) -> bool {
            self.remaining == 0
        }

        fn size_hint(&self) -> SizeHint {
            SizeHint::with_exact(self.remaining)
        }
    }

    #[cfg(test)]
    mod tests {
        use hyper::body::Body as _;

        use super::*;

        #[tokio::test]
        async fn sends_every_non_empty_chunk_with_the_exact_length() {
            let body = Body::new(vec![
                Bytes::from_static(b"ab"),
                Bytes::new(),
                Bytes::from_static(b"c"),
            ]);
            let mut body = HyperBody::from(body);
            assert_eq!(body.size_hint().exact(), Some(3));
            let mut frames = Vec::new();
            while let Some(frame) = body.frame().await {
                frames.push(frame.unwrap().into_data().unwrap());
            }
            assert_eq!(frames, [&b"ab"[..], b"c"]);
            assert!(body.is_end_stream());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_joins_its_chunks_only_when_needed() {
        let one = Bytes::from_static(b"{}");
        let body = Body::new(vec![one.clone()]);
        assert_eq!(body.len(), 2);
        assert!(!body.is_empty());
        // One chunk is returned as is, without a copy.
        assert_eq!(body.into_bytes().as_ptr(), one.as_ptr());

        let body = Body::new(vec![
            Bytes::from_static(b"ab"),
            Bytes::new(),
            Bytes::from_static(b"c"),
        ]);
        assert_eq!(body.chunks().len(), 3);
        assert_eq!(body.len(), 3);
        assert_eq!(body.into_bytes(), "abc");
        assert!(Body::default().is_empty());
        assert!(Body::new(vec![Bytes::new()]).is_empty());
        assert_eq!(Body::default().into_bytes(), "");
    }
}
