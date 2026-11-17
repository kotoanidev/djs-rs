//! A djs-rs bot on Cloudflare Workers (D-406): it answers `/ping` and `/slow` at the app's
//! Interactions Endpoint URL, with the same routes a gateway bot registers, and sends its
//! REST requests through the platform's `fetch`.
//!
//! Run it locally with `npx wrangler dev` (the secrets `DISCORD_TOKEN` and
//! `DISCORD_PUBLIC_KEY` in `.dev.vars`), deploy it with `npx wrangler deploy` after
//! `npx wrangler secret put DISCORD_TOKEN` (and `DISCORD_PUBLIC_KEY`). The commands are
//! registered once, from anywhere (the guide's `http_interactions` example registers them).

use std::{error::Error as StdError, sync::OnceLock, time::Duration};

use bytes::Bytes;
use djs_rest::{Body, BoxFuture, Rest, Transport};
use djs_rs::{
    Client, http_interactions::InteractionsEndpoint, model::message::MessageFlags,
    prelude::InteractionExt,
};
use worker::{
    Context, Delay, Env, Fetch, Headers, Method, Request, RequestInit, Response, event,
    js_sys::Uint8Array, send::SendFuture,
};

// ANCHOR: transport
/// djs-rs' REST requests and waits, through Workers' `fetch` and timers (D-404). Their
/// futures are not `Send` (the Workers runtime has one thread); `SendFuture` makes them so.
struct FetchTransport;

impl Transport for FetchTransport {
    fn send(
        &self,
        request: http::Request<Body>,
    ) -> BoxFuture<'_, Result<http::Response<Bytes>, Box<dyn StdError + Send + Sync>>> {
        Box::pin(SendFuture::new(async move {
            // `worker::Error` holds JS values, which are not `Send`: keep its message only.
            fetch(request)
                .await
                .map_err(|error| error.to_string().into())
        }))
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        Box::pin(SendFuture::new(Delay::from(duration)))
    }
}

/// Sends `request` with `fetch` and reads the whole response.
async fn fetch(request: http::Request<Body>) -> worker::Result<http::Response<Bytes>> {
    let (parts, body) = request.into_parts();
    let headers = Headers::new();
    for (name, value) in &parts.headers {
        headers.set(name.as_str(), value.to_str().map_err(|e| e.to_string())?)?;
    }
    let mut init = RequestInit::new();
    init.with_method(Method::from(parts.method.as_str().to_owned()))
        .with_headers(headers);
    if !body.is_empty() {
        init.with_body(Some(Uint8Array::from(&body.into_bytes()[..]).into()));
    }
    let request = Request::new_with_init(&parts.uri.to_string(), &init)?;
    let mut response = Fetch::Request(request).send().await?;
    let mut builder = http::Response::builder().status(response.status_code());
    for (name, value) in response.headers().entries() {
        builder = builder.header(name, value);
    }
    let body = Bytes::from(response.bytes().await?);
    builder.body(body).map_err(|e| e.to_string().into())
}
// ANCHOR_END: transport

// ANCHOR: routes
/// The bot's routes: the same functions a gateway bot registers.
fn routes(client: &Client) {
    client.command("ping", |ctx, cmd| async move {
        cmd.reply(&ctx, "Pong! (from Cloudflare Workers)").await?;
        Ok(())
    });
    client.command("slow", |ctx, cmd| async move {
        cmd.defer_reply(&ctx, MessageFlags::empty()).await?;
        // The Worker keeps running for this, through `ctx.wait_until` below. Handlers must
        // be `Send`: a JS future awaited in one is wrapped in `SendFuture`.
        SendFuture::new(Delay::from(Duration::from_secs(5))).await;
        cmd.edit_reply(&ctx, "Done, five seconds later.").await?;
        Ok(())
    });
}
// ANCHOR_END: routes

// ANCHOR: fetch
/// The endpoint, built by the isolate's first request and reused by the next ones.
static ENDPOINT: OnceLock<InteractionsEndpoint> = OnceLock::new();

fn endpoint(env: &Env) -> worker::Result<&'static InteractionsEndpoint> {
    if let Some(endpoint) = ENDPOINT.get() {
        return Ok(endpoint);
    }
    // Rust panics would otherwise show only as `unreachable` in the Workers log.
    std::panic::set_hook(Box::new(|info| worker::console_error!("{info}")));
    let token = env.secret("DISCORD_TOKEN")?.to_string();
    let rest = Rest::builder()
        .token(token.clone())
        .transport(FetchTransport)
        .build();
    let client = Client::builder(token).rest(rest).build();
    routes(&client);
    let public_key = env.secret("DISCORD_PUBLIC_KEY")?.to_string();
    let endpoint = client
        .interactions_endpoint(&public_key)
        .map_err(|e| e.to_string())?;
    Ok(ENDPOINT.get_or_init(|| endpoint))
}

#[event(fetch)]
async fn main(mut request: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    let endpoint = endpoint(&env)?;
    let headers = request.headers();
    let signature = headers.get("x-signature-ed25519")?.unwrap_or_default();
    let timestamp = headers.get("x-signature-timestamp")?.unwrap_or_default();
    let body = request.bytes().await?;
    let (response, background) = endpoint.handle(&signature, &timestamp, &body).await;
    // The handlers' work after their first response (the edit of `/slow`).
    ctx.wait_until(background);
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    Ok(Response::from_bytes(response.body)?
        .with_status(response.status)
        .with_headers(headers))
}
// ANCHOR_END: fetch
