//! The data (`d` field) of gateway payloads.
//!
//! - [`incoming`] payloads are received from Discord (events).
//! - [`outgoing`] payloads are sent to Discord (commands); wrap them in [`CommandPayload`].
//!
//! Everything is re-exported at this level.

pub mod incoming;
pub mod outgoing;

pub use self::{incoming::*, outgoing::*};
