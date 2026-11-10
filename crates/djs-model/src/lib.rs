#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    clippy::struct_excessive_bools,
    reason = "model types mirror Discord's JSON objects field by field"
)]

mod macros;

pub mod application;
pub mod channel;
pub mod gateway;
pub mod guild;
pub mod id;
pub mod invite;
pub mod message;
pub mod permissions;
pub mod rest;
pub mod sku;
pub mod user;
pub mod util;
