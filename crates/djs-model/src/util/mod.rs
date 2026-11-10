//! General-purpose value types used throughout the Discord API.

mod image_hash;
mod timestamp;

pub use self::{
    image_hash::{ImageHash, ParseImageHashError},
    timestamp::{ParseTimestampError, Timestamp},
};
