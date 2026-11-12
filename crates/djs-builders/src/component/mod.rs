//! Builders for [components](https://docs.discord.com/developers/components/reference):
//! buttons, select menus and action rows of messages, and the Components V2 layout and
//! content components.
//!
//! Every builder converts into a `djs-model` [`Component`](djs_model::component::Component),
//! so builders and model values can be mixed; the rules are checked on the model values,
//! with paths written like Discord's (`components[1].components[0].custom_id`).

/// Implements `id`, `build`, `build_unchecked` and the conversion into `Component` on a
/// builder whose model value is `self.component` and becomes `Component::$variant`.
macro_rules! component_builder {
    ($builder:ident => $variant:ident) => {
        impl $builder {
            /// Sets the component's numeric identifier (Discord generates one otherwise); it
            /// must be unique in the message.
            ///
            /// discord.js counterpart: `setId`.
            pub fn id(mut self, id: i32) -> Self {
                self.component.id = Some(id);
                self
            }

            /// Checks the component (and what it contains) and returns it.
            ///
            /// # Errors
            ///
            /// The first [`ValidationError`](crate::ValidationError) found; see the type docs
            /// for the rules.
            pub fn build(
                self,
            ) -> Result<::djs_model::component::Component, crate::ValidationError> {
                let component = ::djs_model::component::Component::from(self);
                super::validate::component(&crate::check::Path::Root, &component)?;
                Ok(component)
            }

            /// Returns the component without any check (D-180).
            #[must_use]
            pub fn build_unchecked(self) -> ::djs_model::component::Component {
                self.into()
            }
        }

        impl From<$builder> for ::djs_model::component::Component {
            fn from(builder: $builder) -> Self {
                Self::$variant(builder.component)
            }
        }
    };
}

mod action_row;
mod button;
mod content;
mod layout;
mod list;
mod select;
mod validate;

pub use self::{
    action_row::ActionRowBuilder,
    button::{ButtonBuilder, PremiumButtonBuilder},
    content::{
        FileBuilder, MediaGalleryBuilder, MediaGalleryItemBuilder, TextDisplayBuilder,
        ThumbnailBuilder,
    },
    layout::{ContainerBuilder, SectionBuilder, SeparatorBuilder},
    list::{ComponentsVersion, validate_components},
    select::{
        ChannelSelectMenuBuilder, MentionableSelectMenuBuilder, RoleSelectMenuBuilder,
        StringSelectMenuBuilder, StringSelectMenuOptionBuilder, UserSelectMenuBuilder,
    },
};
