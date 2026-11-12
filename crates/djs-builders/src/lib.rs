#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod check;
mod embed;
mod error;

pub use self::{
    embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, embed_length, validate_embeds},
    error::ValidationError,
};
