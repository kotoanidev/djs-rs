#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod client;
mod context;
mod error;
pub mod event;
pub mod ext;
mod handler;

pub use self::{
    client::{Client, ClientBuilder, EventStream, HandlerMode, Shards},
    context::Context,
    error::Error,
    event::Events,
    handler::HandlerError,
};

/// Discord API types — re-export of [`djs_model`].
pub use djs_model as model;

/// The REST client — re-export of [`djs_rest`].
pub use djs_rest as rest;

/// The gateway client — re-export of [`djs_gateway`].
pub use djs_gateway as gateway;

/// The in-memory cache — re-export of [`djs_cache`]. Enable it with
/// [`ClientBuilder::cache`].
pub use djs_cache as cache;

/// The most commonly used items, for a single glob import:
///
/// ```
/// use djs_rs::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        Client, Context, Events, HandlerError, ext::MessageExt, model::gateway::Intents,
    };
}
