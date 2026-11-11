#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod client;
mod endpoints;
mod error;
mod multipart;
mod ratelimit;
mod route;

pub use self::{
    client::{DEFAULT_API, Request, Rest, RestBuilder},
    error::{ApiError, Error, ErrorKind},
    route::Route,
};
