//! The [gateway](https://docs.discord.com/developers/events/gateway): opcodes, intents, close
//! codes, payloads and events.
//!
//! This module contains only data types. The connection itself lives in the `djs-gateway`
//! crate.

mod close_code;
mod event;
mod intents;
mod opcode;
pub mod payload;
mod shard_id;

pub use self::{
    close_code::CloseCode,
    event::{DispatchEvent, GatewayEvent, GatewayEventDeserializer},
    intents::Intents,
    opcode::OpCode,
    shard_id::ShardId,
};

/// The Discord API version targeted by this crate (and by discord.js v14).
pub const API_VERSION: u8 = 10;
