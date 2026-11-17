#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod cdn;
mod client;
mod endpoints;
mod error;
mod multipart;
pub mod oauth2;
mod query;
mod ratelimit;
mod route;
#[cfg(feature = "hyper")]
mod tls;
mod transport;

pub use self::{
    client::{DEFAULT_API, Request, Rest, RestBuilder},
    error::{ApiError, Error, ErrorKind, FieldError},
    route::{ReactionEmoji, Route},
    transport::{Body, BoxFuture, Transport},
};
