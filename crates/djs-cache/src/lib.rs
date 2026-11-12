#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod backend;
mod cache;
mod config;
mod dashmap_store;
mod limits;

pub use backend::{Backend, Store};
pub use cache::Cache;
pub use config::{CacheConfig, CacheConfigBuilder, Limit, Resource};
pub use dashmap_store::{DashMapBackend, DashMapStore};
