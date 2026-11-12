#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod collection;
pub mod iter;
mod random;

pub use self::collection::{Collection, DefaultHashBuilder};
