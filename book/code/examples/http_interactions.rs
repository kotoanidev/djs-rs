//! The guide's "HTTP interactions" chapter: a bot that receives its interactions at the
//! app's Interactions Endpoint URL, served with axum, instead of over the gateway.
//!
//! Run it with `DISCORD_TOKEN=<token> DISCORD_PUBLIC_KEY=<key> cargo run -p djs-rs-guide
//! --example http_interactions`; it registers `/ping` and `/slow` and listens on
//! `127.0.0.1:8080` (`PORT` changes the port). Discord must reach it over HTTPS: expose it
//! (for example `cloudflared tunnel --url http://localhost:8080`) and set that URL as the
//! Interactions Endpoint URL in the Developer Portal.

use std::time::Duration;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use djs_rs::{
    builders::SlashCommandBuilder, http_interactions::InteractionsEndpoint,
    model::message::MessageFlags, prelude::*,
};

// ANCHOR: routes
/// The bot's routes: exactly what a gateway bot would register.
fn routes(client: &Client) {
    client.command("ping", |ctx, cmd| async move {
        cmd.reply(&ctx, "Pong! (over HTTP)").await?;
        Ok(())
    });
    // A slow command defers: the HTTP response goes back at once, the edit follows.
    client.command("slow", |ctx, cmd| async move {
        cmd.defer_reply(&ctx, MessageFlags::empty()).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        cmd.edit_reply(&ctx, "Done, five seconds later.").await?;
        Ok(())
    });
}
// ANCHOR_END: routes

// ANCHOR: handler
/// The `POST` handler of the Interactions Endpoint URL.
async fn interactions(
    State(endpoint): State<InteractionsEndpoint>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let header = |name| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    };
    let signature = header("x-signature-ed25519");
    let timestamp = header("x-signature-timestamp");
    let (response, background) = endpoint.handle(signature, timestamp, &body).await;
    // What the handlers do after their first response (the edit of `/slow`).
    tokio::spawn(background);
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        response.body,
    )
}
// ANCHOR_END: handler

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    // ANCHOR: main
    let client = Client::builder(std::env::var("DISCORD_TOKEN")?).build();
    routes(&client);

    // No `READY` here: the commands are registered through REST (creating one that exists
    // updates it).
    let application = client.rest().get_current_application().await?;
    for command in [
        SlashCommandBuilder::new("ping", "Checks that the bot answers").build()?,
        SlashCommandBuilder::new("slow", "Answers after five seconds").build()?,
    ] {
        client
            .rest()
            .create_global_application_command(application.id, &command)
            .await?;
    }

    let endpoint = client.interactions_endpoint(&std::env::var("DISCORD_PUBLIC_KEY")?)?;
    let app = Router::new()
        .route("/", post(interactions))
        .with_state(endpoint);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    println!("listening on http://127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    // ANCHOR_END: main
    Ok(())
}
