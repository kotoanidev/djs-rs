//! A local mock of the Discord API shared by the integration tests.

#![allow(dead_code, reason = "each test crate uses a different part")]

use std::{
    collections::VecDeque,
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bytes::Bytes;
use djs_rest::Rest;
use http::{Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

/// A request as seen by the mock server.
#[derive(Debug, Clone)]
pub(crate) struct Seen {
    pub(crate) method: String,
    pub(crate) path: String,
    /// The query string, without `?` (empty if none).
    pub(crate) query: String,
    pub(crate) authorization: Option<String>,
    pub(crate) content_type: Option<String>,
    pub(crate) content_length: Option<String>,
    pub(crate) audit_log_reason: Option<String>,
    pub(crate) user_agent: String,
    pub(crate) body: String,
    pub(crate) at: Instant,
}

/// A canned response.
pub(crate) struct Canned {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

impl Canned {
    pub(crate) fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_owned(),
        }
    }

    pub(crate) fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }
}

#[derive(Default)]
pub(crate) struct State {
    responses: VecDeque<Canned>,
    pub(crate) seen: Vec<Seen>,
}

/// Starts a mock server that answers with `responses` in order.
pub(crate) async fn mock(responses: Vec<Canned>) -> (Rest, Arc<Mutex<State>>) {
    mock_with(responses, |builder| builder).await
}

/// Like [`mock`], with extra client options.
pub(crate) async fn mock_with(
    responses: Vec<Canned>,
    configure: impl FnOnce(djs_rest::RestBuilder) -> djs_rest::RestBuilder,
) -> (Rest, Arc<Mutex<State>>) {
    let state = Arc::new(Mutex::new(State {
        responses: responses.into(),
        seen: Vec::new(),
    }));
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();

    let server_state = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = Arc::clone(&server_state);
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    let state = Arc::clone(&state);
                    async move { Ok::<_, Infallible>(handle(&state, request).await) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });

    let rest = configure(
        Rest::builder()
            .token("test-token")
            .api(format!("http://{address}/api"))
            .offset(Duration::ZERO),
    )
    .build();
    (rest, state)
}

async fn handle(state: &Mutex<State>, request: Request<Incoming>) -> Response<Full<Bytes>> {
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
    };
    let mut seen = Seen {
        method: request.method().to_string(),
        path: request.uri().path().to_owned(),
        query: request.uri().query().unwrap_or_default().to_owned(),
        authorization: header("authorization"),
        content_type: header("content-type"),
        content_length: header("content-length"),
        audit_log_reason: header("x-audit-log-reason"),
        user_agent: header("user-agent").unwrap_or_default(),
        body: String::new(),
        at: Instant::now(),
    };
    let body = request.into_body().collect().await.unwrap().to_bytes();
    seen.body = String::from_utf8(body.to_vec()).unwrap();

    let mut state = state.lock().unwrap();
    state.seen.push(seen);
    let canned = state
        .responses
        .pop_front()
        .expect("mock received more requests than expected");
    let mut response = Response::builder().status(canned.status);
    for (name, value) in canned.headers {
        response = response.header(name, value);
    }
    response.body(Full::new(Bytes::from(canned.body))).unwrap()
}

/// The requests the mock has seen so far.
pub(crate) fn seen(state: &Mutex<State>) -> Vec<Seen> {
    state.lock().unwrap().seen.clone()
}
