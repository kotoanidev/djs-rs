//! General-purpose value types used throughout the Discord API.

mod image_data;
mod image_hash;
mod json_value;
mod locale;
mod patch;
mod timestamp;

pub use self::{
    image_data::{ImageData, ImageDataError},
    image_hash::{ImageHash, ParseImageHashError},
    json_value::{JsonValue, JsonValueError},
    locale::Locale,
    patch::Patch,
    timestamp::{ParseTimestampError, Timestamp},
};
