#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod compression;
mod config;
mod error;
mod queue;
mod send_limit;
mod shard;
mod sharding;
mod tls;

pub use self::{
    compression::Compression,
    config::{Config, ConfigBuilder, DEFAULT_GATEWAY_URL},
    error::{ReceiveError, ReceiveErrorKind, SendError},
    queue::{IdentifyQueue, LocalIdentifyQueue},
    shard::{MessageSender, Shard, ShardState},
    sharding::create_shards,
};
