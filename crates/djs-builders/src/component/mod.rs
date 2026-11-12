//! Builders for [components](https://docs.discord.com/developers/components/reference):
//! buttons, select menus and action rows of messages.
//!
//! Every builder converts into a `djs-model` [`Component`](djs_model::component::Component),
//! so builders and model values can be mixed; the rules are checked on the model values,
//! with paths written like Discord's (`components[1].components[0].custom_id`).

mod action_row;
mod button;
mod list;
mod select;
mod validate;

pub use self::{
    action_row::ActionRowBuilder,
    button::{ButtonBuilder, PremiumButtonBuilder},
    list::{ComponentsVersion, validate_components},
    select::{
        ChannelSelectMenuBuilder, MentionableSelectMenuBuilder, RoleSelectMenuBuilder,
        StringSelectMenuBuilder, StringSelectMenuOptionBuilder, UserSelectMenuBuilder,
    },
};
