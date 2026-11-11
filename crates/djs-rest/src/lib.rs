#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod client;
mod endpoints;
mod error;
mod multipart;
mod query;
mod ratelimit;
mod route;
mod tls;

pub use self::{
    client::{DEFAULT_API, Request, Rest, RestBuilder},
    error::{ApiError, Error, ErrorKind, FieldError},
    route::{ReactionEmoji, Route},
};
