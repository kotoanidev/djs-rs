//! HTTP interactions (D-395, D-398–D-403): answering the interactions Discord posts to the
//! app's **Interactions Endpoint URL** with the same routes and handlers as a gateway bot.
//!
//! An app with an Interactions Endpoint URL receives its commands, components, autocomplete
//! and modals as HTTP `POST` requests instead of gateway events, so it needs no long-lived
//! connection: it can run in any HTTP server, or on a serverless platform such as Cloudflare
//! Workers. [`Client::interactions_endpoint`](crate::Client::interactions_endpoint) makes an
//! [`InteractionsEndpoint`] from a client; its [`handle`](InteractionsEndpoint::handle) takes
//! a request's `X-Signature-Ed25519` and `X-Signature-Timestamp` headers and its body, and
//! returns the [`HttpResponse`] to send back. There is no HTTP server in djs-rs: mount
//! `handle` in the one the bot already uses.
//!
//! What `handle` does, in order:
//!
//! 1. Verifies the request ([Discord's security
//!    rules](https://docs.discord.com/developers/interactions/overview#setting-up-an-endpoint-validating-security-request-headers)):
//!    the Ed25519 signature of the timestamp and body under the app's public key, and a
//!    timestamp at most 5 minutes from now against replayed requests (D-401). Otherwise
//!    `401`.
//! 2. Answers a `PING` (sent when the URL is saved in the Developer Portal) with a `PONG`.
//! 3. Dispatches the interaction like a gateway `INTERACTION_CREATE`: the cache, the event
//!    streams and collectors, the [`InteractionCreate`](crate::Events::InteractionCreate)
//!    handlers, then the routes ([`Client::command`](crate::Client::command) and the rest).
//! 4. Returns as soon as a handler gives the initial response (`reply`, `defer_reply`,
//!    `update`, `show_modal`, …): it becomes the HTTP body, with no REST request (D-399).
//!    The handlers' remaining work (for example the follow-up after a deferral) is the
//!    returned future, which the server must run: `tokio::spawn` natively,
//!    `ctx.wait_until` on Cloudflare Workers.
//!
//! | Status | When | Body |
//! |---|---|---|
//! | `200` | the initial response, or the `PONG` | its JSON (`Content-Type: application/json`) |
//! | `202` | the initial response had files: it went through REST instead | empty |
//! | `400` | the body is not an interaction | empty |
//! | `401` | the signature or the timestamp is invalid | empty |
//! | `500` | the handlers finished without an initial response | empty |
//!
//! Every handler runs one after another for the request, whatever the client's
//! [`HandlerMode`](crate::HandlerMode). Gateway-only parts stay silent: no other event
//! reaches the handlers, and [`Context::shard`](crate::Context::shard) is the guild's shard
//! only if the client is also logged in (else a
//! [detached](djs_gateway::MessageSender::detached) sender, D-403). Collectors only see
//! the interactions this same client handles, so on a platform that spreads requests over
//! many instances they may never see the next one.
//!
//! discord.js counterpart: none (discord.js only receives interactions over the gateway);
//! the official `discord-interactions` package has the verification (`verifyKey`).
//!
//! # Examples
//!
//! ```no_run
//! use djs_rs::prelude::*;
//!
//! # async fn run(signature: &str, timestamp: &str, body: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::builder(std::env::var("DISCORD_TOKEN")?).build();
//! client.command("ping", |ctx, cmd| async move {
//!     cmd.reply(&ctx, "Pong!").await?;
//!     Ok(())
//! });
//! let endpoint = client.interactions_endpoint(&std::env::var("DISCORD_PUBLIC_KEY")?)?;
//!
//! // In the HTTP server's `POST` handler, with the request's headers and body:
//! let (response, background) = endpoint.handle(signature, timestamp, body).await;
//! tokio::spawn(background);
//! // … answer with `response.status` and `response.body`.
//! # let _ = response;
//! # Ok(())
//! # }
//! ```

use core::{fmt, future::Future, pin::Pin};
use std::sync::{Mutex, PoisonError};

use djs_cache::{Backend, DashMapBackend};
use djs_model::{
    gateway::payload::InteractionCreate,
    id::InteractionId,
    interaction::{InteractionData, InteractionResponse},
    util::Timestamp,
};
use djs_rest::{Error, Rest};
use ed25519_dalek::{Signature, VerifyingKey};
use tokio::sync::oneshot;

use crate::Client;

/// How far `X-Signature-Timestamp` may be from now, in seconds (D-401).
const MAX_AGE_SECS: u64 = 5 * 60;

/// The body of a `PONG`.
const PONG: &[u8] = br#"{"type":1}"#;

/// The future of a request's remaining work.
type Background = Pin<Box<dyn Future<Output = ()> + Send>>;

/// What the handlers did with the initial response (D-399).
pub(crate) enum Initial {
    /// Its JSON, for the HTTP body.
    Body(Vec<u8>),
    /// It had files, so it went through REST.
    Rest,
}

/// Sends the initial response of an HTTP interaction to its endpoint.
pub(crate) type InitialSender = oneshot::Sender<Initial>;

/// Receives the initial response of an HTTP interaction.
pub(crate) type InitialReceiver = oneshot::Receiver<Initial>;

/// The slot of an HTTP interaction's initial response, in its [`Context`](crate::Context):
/// the first response of that interaction takes it.
pub(crate) struct Inline {
    interaction: InteractionId,
    slot: Mutex<Option<InitialSender>>,
}

impl Inline {
    /// A slot for `interaction`, and the receiver of what goes in it.
    pub(crate) fn new(interaction: InteractionId) -> (Self, InitialReceiver) {
        let (sender, receiver) = oneshot::channel();
        let inline = Self {
            interaction,
            slot: Mutex::new(Some(sender)),
        };
        (inline, receiver)
    }

    /// Takes the slot if `interaction` is this one's and it is still empty.
    pub(crate) fn take(&self, interaction: InteractionId) -> Option<InitialSender> {
        if interaction != self.interaction {
            return None;
        }
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Gives `response`, an HTTP interaction's initial response, to its endpoint: as JSON for
/// the HTTP body, or, if it has files, through REST first (C-139).
pub(crate) async fn respond(
    rest: &Rest,
    initial: InitialSender,
    interaction: InteractionId,
    token: &str,
    response: &InteractionResponse,
) -> Result<(), Error> {
    if response.files().is_empty()
        && let Ok(body) = serde_json::to_vec(response)
    {
        // A closed receiver means the request is gone; there is no one left to answer.
        let _ = initial.send(Initial::Body(body));
        return Ok(());
    }
    rest.create_interaction_response(interaction, token, response)
        .await?;
    let _ = initial.send(Initial::Rest);
    Ok(())
}

/// The response to an HTTP interaction request, returned by
/// [`InteractionsEndpoint::handle`]: send `status` and `body`, with `Content-Type:
/// application/json` when `body` is not empty.
///
/// discord.js counterpart: none.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpResponse {
    /// The HTTP status code: `200`, `202`, `400`, `401` or `500` (see [the
    /// module](self)).
    pub status: u16,
    /// The body: an interaction response's JSON, or empty.
    pub body: Vec<u8>,
}

impl HttpResponse {
    const fn empty(status: u16) -> Self {
        Self {
            status,
            body: Vec::new(),
        }
    }
}

/// The error of [`Client::interactions_endpoint`](crate::Client::interactions_endpoint):
/// the public key is not 64 hexadecimal digits of a valid Ed25519 key.
///
/// discord.js counterpart: none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the public key is not 64 hexadecimal digits of a valid Ed25519 key")]
pub struct InvalidPublicKey;

/// Answers the interactions Discord posts to the app's Interactions Endpoint URL with a
/// [`Client`]'s routes and handlers; made by
/// [`Client::interactions_endpoint`](crate::Client::interactions_endpoint). Cheap to clone.
///
/// See [the module](self) for what [`handle`](Self::handle) does.
///
/// discord.js counterpart: none.
pub struct InteractionsEndpoint<B: Backend = DashMapBackend> {
    client: Client<B>,
    key: VerifyingKey,
}

impl<B: Backend> Clone for InteractionsEndpoint<B> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            key: self.key,
        }
    }
}

impl<B: Backend> fmt::Debug for InteractionsEndpoint<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InteractionsEndpoint")
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl<B: Backend> InteractionsEndpoint<B> {
    pub(crate) fn new(client: Client<B>, public_key: &str) -> Result<Self, InvalidPublicKey> {
        let bytes = decode_hex::<32>(public_key).ok_or(InvalidPublicKey)?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| InvalidPublicKey)?;
        Ok(Self { client, key })
    }

    /// Handles one request to the Interactions Endpoint URL: `signature` and `timestamp` are
    /// its `X-Signature-Ed25519` and `X-Signature-Timestamp` headers (pass `""` for a missing
    /// one), `body` its raw body. Returns the response to send and the future of the
    /// handlers' remaining work, which must be run (`tokio::spawn`, or `ctx.wait_until` on
    /// Cloudflare Workers) for follow-ups to happen.
    ///
    /// Returns as soon as a handler gives the initial response: Discord waits at most 3
    /// seconds for it. See [the module](self) for every status.
    ///
    /// discord.js counterpart: none; `verifyKey` of the `discord-interactions` package for
    /// the verification.
    pub async fn handle(
        &self,
        signature: &str,
        timestamp: &str,
        body: &[u8],
    ) -> (
        HttpResponse,
        impl Future<Output = ()> + Send + 'static + use<B>,
    ) {
        let idle = || -> Background { Box::pin(core::future::ready(())) };
        if !verify(
            &self.key,
            signature,
            timestamp,
            body,
            Timestamp::now().as_unix_secs(),
        ) {
            return (HttpResponse::empty(401), idle());
        }
        let event: InteractionCreate = match serde_json::from_slice(body) {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(%error, "a signed HTTP interaction could not be decoded");
                return (HttpResponse::empty(400), idle());
            }
        };
        if matches!(event.data, InteractionData::Ping) {
            let pong = HttpResponse {
                status: 200,
                body: PONG.to_vec(),
            };
            return (pong, idle());
        }
        let (mut initial, mut work) = self.client.dispatch_http(event);
        tokio::select! {
            biased;
            result = &mut initial => (answer(result.ok()), work),
            () = &mut work => (answer(initial.try_recv().ok()), idle()),
        }
    }
}

/// The HTTP response for what the handlers did with the initial response.
fn answer(initial: Option<Initial>) -> HttpResponse {
    match initial {
        Some(Initial::Body(body)) => HttpResponse { status: 200, body },
        Some(Initial::Rest) => HttpResponse::empty(202),
        None => {
            tracing::warn!("the handlers of an HTTP interaction finished without responding");
            HttpResponse::empty(500)
        }
    }
}

/// Whether `signature` (hexadecimal) signs `timestamp` followed by `body` under `key`, and
/// `timestamp` (Unix seconds) is at most [`MAX_AGE_SECS`] from `now`.
fn verify(key: &VerifyingKey, signature: &str, timestamp: &str, body: &[u8], now: i64) -> bool {
    let Ok(signed_at) = timestamp.parse::<i64>() else {
        return false;
    };
    if now.abs_diff(signed_at) > MAX_AGE_SECS {
        return false;
    }
    let Some(signature) = decode_hex::<64>(signature) else {
        return false;
    };
    let mut message = Vec::with_capacity(timestamp.len() + body.len());
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);
    key.verify_strict(&message, &Signature::from_bytes(&signature))
        .is_ok()
}

/// Decodes exactly `N` bytes of hexadecimal (either case).
fn decode_hex<const N: usize>(hex: &str) -> Option<[u8; N]> {
    const fn digit(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    let hex = hex.as_bytes();
    if hex.len() != 2 * N {
        return None;
    }
    let mut out = [0; N];
    for (byte, pair) in out.iter_mut().zip(hex.as_chunks::<2>().0) {
        *byte = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use core::fmt::Write as _;

    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
    }

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn sign(timestamp: &str, body: &[u8]) -> String {
        let mut message = timestamp.as_bytes().to_vec();
        message.extend_from_slice(body);
        hex(&key().sign(&message).to_bytes())
    }

    #[test]
    fn decodes_hex_of_the_exact_length_only() {
        assert_eq!(decode_hex::<2>("0aFf"), Some([0x0a, 0xff]));
        assert_eq!(decode_hex::<2>("0aF"), None);
        assert_eq!(decode_hex::<2>("0aFf0"), None);
        assert_eq!(decode_hex::<2>("0aFg"), None);
        assert_eq!(decode_hex::<1>("é"), None);
    }

    #[test]
    fn verifies_the_signature_of_the_timestamp_and_body() {
        let public = key().verifying_key();
        let (timestamp, body) = ("1800000000", br#"{"type":1}"#.as_slice());
        let signature = sign(timestamp, body);
        assert!(verify(&public, &signature, timestamp, body, NOW));
        assert!(verify(
            &public,
            &signature.to_uppercase(),
            timestamp,
            body,
            NOW
        ));
        // Another body, timestamp or key; a malformed signature.
        assert!(!verify(
            &public,
            &signature,
            timestamp,
            br#"{"type":2}"#,
            NOW
        ));
        assert!(!verify(&public, &signature, "1800000001", body, NOW));
        let other = SigningKey::from_bytes(&[8; 32]).verifying_key();
        assert!(!verify(&other, &signature, timestamp, body, NOW));
        assert!(!verify(&public, &signature[2..], timestamp, body, NOW));
        assert!(!verify(&public, "", timestamp, body, NOW));
    }

    #[test]
    fn rejects_timestamps_more_than_five_minutes_away() {
        let public = key().verifying_key();
        let body = b"{}";
        for (offset, valid) in [
            (0, true),
            (300, true),
            (-300, true),
            (301, false),
            (-301, false),
        ] {
            let timestamp = (NOW + offset).to_string();
            let signature = sign(&timestamp, body);
            assert_eq!(
                verify(&public, &signature, &timestamp, body, NOW),
                valid,
                "{offset}"
            );
        }
        let signature = sign("soon", body);
        assert!(!verify(&public, &signature, "soon", body, NOW));
    }

    #[test]
    fn the_endpoint_needs_a_valid_public_key() {
        let client = Client::builder("token").build();
        let hex = hex(&key().verifying_key().to_bytes());
        assert!(client.interactions_endpoint(&hex).is_ok());
        assert_eq!(
            client.interactions_endpoint(&hex[1..]).unwrap_err(),
            InvalidPublicKey
        );
        assert_eq!(
            client.interactions_endpoint("zz").unwrap_err(),
            InvalidPublicKey
        );
    }
}
