#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    clippy::struct_excessive_bools,
    reason = "model types mirror Discord's JSON objects field by field"
)]

mod macros;
mod serde_util;

pub mod application;
pub mod auto_moderation;
pub mod channel;
pub mod emoji;
pub mod gateway;
pub mod guild;
pub mod guild_scheduled_event;
pub mod id;
pub mod invite;
pub mod message;
pub mod oauth2;
pub mod permissions;
pub mod rest;
pub mod sku;
pub mod stage_instance;
pub mod sticker;
pub mod team;
pub mod user;
pub mod util;
pub mod webhook;
