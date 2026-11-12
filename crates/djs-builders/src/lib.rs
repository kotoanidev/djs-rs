#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod check;
mod command;
mod component;
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
    component::{
        ActionRowBuilder, ButtonBuilder, ChannelSelectMenuBuilder, CheckboxBuilder,
        CheckboxGroupBuilder, CheckboxGroupOptionBuilder, ComponentsVersion, ContainerBuilder,
        FileBuilder, FileUploadBuilder, LabelBuilder, MediaGalleryBuilder, MediaGalleryItemBuilder,
        MentionableSelectMenuBuilder, ModalBuilder, PremiumButtonBuilder, RadioGroupBuilder,
        RadioGroupOptionBuilder, RoleSelectMenuBuilder, SectionBuilder, SeparatorBuilder,
        StringSelectMenuBuilder, StringSelectMenuOptionBuilder, TextDisplayBuilder,
        TextInputBuilder, ThumbnailBuilder, UserSelectMenuBuilder, validate_components,
    },
    embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, embed_length, validate_embeds},
    error::ValidationError,
};
