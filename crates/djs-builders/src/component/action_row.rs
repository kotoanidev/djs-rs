//! [`ActionRowBuilder`].

use djs_model::component::{ActionRow, Component};

use super::validate;
use crate::{ValidationError, check::Path};

/// Builds an [action row](https://docs.discord.com/developers/components/reference#action-row):
/// a line of up to five buttons, or one select menu.
///
/// Children are added with [`component`](Self::component) — any builder of this crate or a
/// `djs-model` [`Component`] — and [`build`](Self::build) checks the mix (D-198): 1-5 buttons,
/// or exactly one select menu, with unique `custom_id`s; each child is checked too, with paths
/// such as `components[2].label`. Text inputs belong in a modal's
/// [label](crate::LabelBuilder): the old rows of text inputs are rejected (D-207).
///
/// A legacy message (without the Components V2 flag) has at most five rows; check a whole
/// list with [`validate_components`](crate::validate_components).
///
/// discord.js counterpart: `ActionRowBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{ActionRowBuilder, ButtonBuilder, ValidationError};
///
/// let mut row = ActionRowBuilder::new();
/// for page in 1..=3 {
///     row = row.component(ButtonBuilder::secondary(format!("page-{page}")).label(page.to_string()));
/// }
/// assert!(row.build().is_ok());
///
/// let error = ActionRowBuilder::new().build().unwrap_err();
/// assert_eq!(error.to_string(), "components has too few items: 0, at least 1 required");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct ActionRowBuilder {
    row: ActionRow,
}

impl Default for ActionRowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRowBuilder {
    /// Creates an empty row; it needs at least one component.
    ///
    /// discord.js counterpart: `new ActionRowBuilder()`.
    pub fn new() -> Self {
        Self {
            row: ActionRow::new(Vec::new()),
        }
    }

    /// Adds a component after those already added.
    ///
    /// discord.js counterpart: `addComponents`.
    pub fn component(mut self, component: impl Into<Component>) -> Self {
        self.row.components.push(component.into());
        self
    }

    /// Adds several components.
    ///
    /// discord.js counterpart: `addComponents`.
    pub fn components<C: Into<Component>>(
        mut self,
        components: impl IntoIterator<Item = C>,
    ) -> Self {
        self.row
            .components
            .extend(components.into_iter().map(Into::into));
        self
    }

    /// Sets the component's numeric identifier (Discord generates one otherwise); it must be
    /// unique in the message or modal.
    ///
    /// discord.js counterpart: `setId`.
    pub fn id(mut self, id: i32) -> Self {
        self.row.id = Some(id);
        self
    }

    /// Checks the row and its children and returns it as a [`Component`].
    ///
    /// discord.js counterpart: `ActionRowBuilder#toJSON`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::TooFew`] / [`ValidationError::TooMany`] for an empty row or more than
    /// five components, [`ValidationError::Invalid`] for a wrong mix or a repeated
    /// `custom_id`, then the first problem of a child.
    pub fn build(self) -> Result<Component, ValidationError> {
        validate::action_row(&Path::Root, &self.row)?;
        Ok(self.into())
    }

    /// Returns the row without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> Component {
        self.into()
    }
}

impl From<ActionRowBuilder> for Component {
    fn from(row: ActionRowBuilder) -> Self {
        Self::ActionRow(row.row)
    }
}
