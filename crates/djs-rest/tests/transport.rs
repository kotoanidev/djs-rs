//! A custom [`Transport`] (D-404): it sends every request, and every wait (rate limits, the
//! timeout) goes through its `sleep`.

use std::{
    collections::VecDeque,
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use djs_model::{id::ChannelId, rest::CreateMessage};
use djs_rest::{Body, BoxFuture, ErrorKind, Request, Rest, Route, Transport};

/// What the transport saw: requests (method, path, authorization, body) and sleeps.
#[derive(Default)]
struct Seen {
    requests: Vec<(String, String, Option<String>, Bytes)>,
    sleeps: Vec<Duration>,
}

/// Answers with the queued responses, in order; with none left, never answers.
#[derive(Clone, Default)]
struct Scripted {
    responses: Arc<Mutex<VecDeque<(u16, &'static str)>>>,
    seen: Arc<Mutex<Seen>>,
}

impl Transport for Scripted {
    fn send(
        &self,
        request: http::Request<Body>,
    ) -> BoxFuture<'_, Result<http::Response<Bytes>, Box<dyn Error + Send + Sync>>> {
        let authorization = request
            .headers()
            .get("authorization")
            .map(|value| value.to_str().unwrap().to_owned());
        let (parts, body) = request.into_parts();
        self.seen.lock().unwrap().requests.push((
            parts.method.to_string(),
            parts.uri.path().to_owned(),
            authorization,
            body.into_bytes(),
        ));
        let next = self.responses.lock().unwrap().pop_front();
        Box::pin(async move {
            let Some((status, body)) = next else {
                return std::future::pending().await;
            };
            let mut response = http::Response::builder().status(status);
            if status == 429 {
                response = response.header("x-ratelimit-scope", "user");
            }
            Ok(response.body(Bytes::from_static(body.as_bytes()))?)
        })
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        self.seen.lock().unwrap().sleeps.push(duration);
        // Time passes at once: the test never waits.
        Box::pin(std::future::ready(()))
    }
}

fn rest(transport: &Scripted) -> Rest {
    Rest::builder()
        .token("secret")
        .retries(0)
        .transport(transport.clone())
        .build()
}

#[tokio::test]
async fn requests_and_rate_limit_waits_go_through_the_transport() {
    let transport = Scripted::default();
    transport.responses.lock().unwrap().extend([
        (
            429,
            r#"{"message":"You are being rate limited.","retry_after":0.25,"global":false}"#,
        ),
        (204, ""),
    ]);
    let request = Request::new(Route::CreateMessage {
        channel_id: ChannelId::new(5),
    })
    .json(&CreateMessage::from("hi"))
    .unwrap();
    rest(&transport).request::<()>(request).await.unwrap();

    let seen = transport.seen.lock().unwrap();
    assert_eq!(seen.requests.len(), 2, "sent again after the 429");
    let (method, path, authorization, body) = &seen.requests[1];
    assert_eq!(method, "POST");
    assert_eq!(path, "/api/v10/channels/5/messages");
    assert_eq!(authorization.as_deref(), Some("Bot secret"));
    assert!(body.starts_with(br#"{"content":"hi""#), "{body:?}");
    // Each attempt arms the timeout (15 s); the 429 waits until retry_after plus the 50 ms
    // offset from when it came (a little less by the time the wait starts).
    let waits: Vec<_> = seen
        .sleeps
        .iter()
        .filter(|&&sleep| sleep != Duration::from_secs(15))
        .collect();
    assert_eq!(waits.len(), 1, "{waits:?}");
    assert!(
        (Duration::from_millis(250)..=Duration::from_millis(300)).contains(waits[0]),
        "{waits:?}"
    );
}

#[tokio::test]
async fn the_timeout_is_measured_with_the_transports_sleep() {
    // No response ever comes; the transport's sleep ends at once, so the request times out.
    let transport = Scripted::default();
    let error = rest(&transport)
        .request::<()>(Request::new(Route::GetGateway))
        .await
        .unwrap_err();
    assert!(matches!(error.kind(), ErrorKind::Timeout), "{error:?}");
    assert_eq!(
        transport.seen.lock().unwrap().sleeps,
        [Duration::from_secs(15)]
    );
}
