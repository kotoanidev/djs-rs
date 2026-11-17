#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
// Without the gateway, the client's internals that only it reads stay compiled but unused
// (C-141); the default build, with the gateway, is linted in full.
#![cfg_attr(not(feature = "gateway"), allow(dead_code, unused_imports))]

mod cache_ops;
mod client;
pub mod collector;
mod context;
mod dispatch;
mod error;
pub mod event;
pub mod ext;
mod handler;
#[cfg(feature = "http-interactions")]
#[cfg_attr(docsrs, doc(cfg(feature = "http-interactions")))]
pub mod http_interactions;
pub mod manager;
pub mod router;
#[cfg(feature = "sharding")]
#[cfg_attr(docsrs, doc(cfg(feature = "sharding")))]
pub mod sharding;
#[cfg(feature = "voice")]
#[cfg_attr(docsrs, doc(cfg(feature = "voice")))]
pub mod voice;
pub mod webhook;

pub use self::{
    client::{Client, ClientBuilder, EventStream, HandlerMode},
    context::Context,
    error::Error,
    event::Events,
    handler::HandlerError,
    webhook::WebhookClient,
};

#[cfg(feature = "gateway")]
#[cfg_attr(docsrs, doc(cfg(feature = "gateway")))]
pub use self::client::Shards;

/// Discord API types — re-export of [`djs_model`].
pub use djs_model as model;

/// The REST client — re-export of [`djs_rest`].
pub use djs_rest as rest;

/// The gateway client — re-export of [`djs_gateway`].
#[cfg(feature = "gateway")]
#[cfg_attr(docsrs, doc(cfg(feature = "gateway")))]
pub use djs_gateway as gateway;

/// The in-memory cache — re-export of [`djs_cache`]. Enable it with
/// [`ClientBuilder::cache`].
pub use djs_cache as cache;

/// Validated builders for embeds, commands and components — re-export of [`djs_builders`].
pub use djs_builders as builders;

/// Mentions, timestamps, markdown and escaping for message text — re-export of
/// [`djs_formatters`].
pub use djs_formatters as formatters;

/// An insertion-ordered map with discord.js' `Collection` helpers — re-export of
/// [`djs_collection`].
pub use djs_collection as collection;

/// The most commonly used items, for a single glob import:
///
/// ```
/// use djs_rs::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        Client, Context, Events, HandlerError,
        ext::{ChannelExt, GuildExt, InteractionExt, MemberExt, MessageExt, UserExt, WebhookExt},
        model::gateway::Intents,
    };
}

/// The repository README's example, compiled as a doctest (not run: it needs a token) so it
/// cannot drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct RepositoryReadme;
