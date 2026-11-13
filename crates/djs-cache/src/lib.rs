#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod backend;
mod cache;
mod config;
mod dashmap_store;
mod limits;
mod model;
mod permissions;
mod update;
mod view;

pub use backend::{Backend, Store};
pub use cache::Cache;
pub use config::{CacheConfig, CacheConfigBuilder, Limit, Resource, Sweep, SweepConfig};
pub use dashmap_store::{DashMapBackend, DashMapStore};
pub use model::{CachedGuild, CachedMember, CachedMessage, CachedPresence, CachedUser};
pub use update::{Previous, UpdateCache};
pub use view::{CacheView, ChannelView, GuildView};
