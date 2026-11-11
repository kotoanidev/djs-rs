//! The mock Discord HTTP API: `GET /gateway/bot` (for the gateway scenarios) and a Create
//! Message endpoint answering with a message and rate-limit headers that never block.

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    Method, Request, Response, body::Incoming, header::CONTENT_TYPE, server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use serde_json::json;
use tokio::net::TcpListener;

use crate::payloads::sent_message;

/// Serves the API on a free port, after printing `{"api": "http://127.0.0.1:<port>/api"}`.
pub(crate) async fn run() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let address = listener.local_addr().expect("a bound address");
    println!("{}", json!({ "api": format!("http://{address}/api") }));
    serve(listener, None).await;
}

/// Serves HTTP/1.1 connections (with keep-alive) on `listener` forever. `gateway` is the URL
/// returned by `GET /gateway/bot`, if any.
pub(crate) async fn serve(listener: TcpListener, gateway: Option<Arc<str>>) {
    let sequence = Arc::new(AtomicU64::new(0));
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let _ = stream.set_nodelay(true);
        let gateway = gateway.clone();
        let sequence = Arc::clone(&sequence);
        tokio::spawn(async move {
            let service = service_fn(move |request: Request<Incoming>| {
                let gateway = gateway.clone();
                let sequence = Arc::clone(&sequence);
                async move { Ok::<_, Infallible>(respond(request, gateway.as_deref(), &sequence).await) }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

fn json_response(status: u16, body: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .expect("a valid response")
}

async fn respond(
    request: Request<Incoming>,
    gateway: Option<&str>,
    sequence: &AtomicU64,
) -> Response<Full<Bytes>> {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    // Read the whole body, as a real server does before answering.
    let _ = request.into_body().collect().await;

    if let (&Method::GET, "/api/v10/gateway/bot", Some(url)) = (&method, path.as_str(), gateway) {
        return json_response(
            200,
            json!({
                "url": url,
                "shards": 1,
                "session_start_limit": {
                    "total": 1000, "remaining": 1000, "reset_after": 0, "max_concurrency": 1
                }
            })
            .to_string(),
        );
    }
    let channel = path
        .strip_prefix("/api/v10/channels/")
        .and_then(|rest| rest.strip_suffix("/messages"));
    if let (&Method::POST, Some(channel)) = (&method, channel) {
        let reset = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_secs_f64()
            + 60.0;
        let body = sent_message(channel, sequence.fetch_add(1, Ordering::Relaxed));
        return Response::builder()
            .status(200)
            .header(CONTENT_TYPE, "application/json")
            .header("x-ratelimit-limit", "10000")
            .header("x-ratelimit-remaining", "9999")
            .header("x-ratelimit-reset", format!("{reset:.3}"))
            .header("x-ratelimit-reset-after", "60.000")
            .header("x-ratelimit-bucket", "0123456789abcdef0123456789abcdef")
            .body(Full::new(Bytes::from(body)))
            .expect("a valid response");
    }
    json_response(
        404,
        r#"{"message": "404: Not Found", "code": 0}"#.to_owned(),
    )
}
