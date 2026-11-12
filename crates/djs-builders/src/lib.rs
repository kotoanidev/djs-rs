#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod check;
mod command;
mod embed;
mod error;

pub use self::{
    command::{
        ContextMenuCommandBuilder, SlashCommandAttachmentOption, SlashCommandBooleanOption,
        SlashCommandBuilder, SlashCommandChannelOption, SlashCommandIntegerOption,
        SlashCommandMentionableOption, SlashCommandNumberOption, SlashCommandRoleOption,
        SlashCommandStringOption, SlashCommandSubcommandBuilder,
        SlashCommandSubcommandGroupBuilder, SlashCommandUserOption,
    },
    embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, embed_length, validate_embeds},
    error::ValidationError,
};
