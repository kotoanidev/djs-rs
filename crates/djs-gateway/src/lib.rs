#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod compression;
mod config;
mod error;
mod send_limit;
mod shard;
mod tls;

pub use self::{
    compression::Compression,
    config::{Config, ConfigBuilder, DEFAULT_GATEWAY_URL},
    error::{ReceiveError, ReceiveErrorKind, SendError},
    shard::{MessageSender, Shard, ShardState},
};
