#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod escape;
mod link;
mod markdown;
mod mention;
mod timestamp;

pub use self::{
    escape::{
        EscapeOptions, escape_bold, escape_bulleted_list, escape_code_block, escape_escape,
        escape_heading, escape_inline_code, escape_italic, escape_markdown, escape_masked_link,
        escape_numbered_list, escape_spoiler, escape_strikethrough, escape_underline,
    },
    link::{
        ApplicationDirectory, ChannelLink, Email, MessageLink, PhoneNumber, application_directory,
        channel_link, email, message_link, phone_number,
    },
    markdown::{
        BlockQuote, Bold, CodeBlock, Face, Heading, HeadingLevel, HideLinkEmbed, Hyperlink,
        InlineCode, Italic, ListEntry, ListItem, OrderedList, Quote, Spoiler, Strikethrough,
        Subtext, Underline, UnorderedList, block_quote, bold, code_block, heading, hide_link_embed,
        hyperlink, inline_code, italic, ordered_list, quote, spoiler, strikethrough, subtext,
        underline, unordered_list,
    },
    mention::{
        ChannelMention, Emoji, GuildNavigation, LinkedRoleMention, RoleMention,
        SlashCommandMention, UserMention, channel_mention, emoji, linked_role_mention,
        role_mention, slash_command_mention, user_mention,
    },
    timestamp::{Time, TimestampStyle, UnixSeconds, time},
};
