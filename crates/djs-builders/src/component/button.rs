//! [`ButtonBuilder`] and [`PremiumButtonBuilder`].

use compact_str::CompactString;
use djs_model::{
    component::{Button, ButtonStyle, Component, InteractiveButton, LinkButton, PremiumButton},
    emoji::PartialEmoji,
    id::SkuId,
};

use super::validate;
use crate::{ValidationError, check::Path};

/// Builds a [button](https://docs.discord.com/developers/components/reference#button) for an
/// [action row](crate::ActionRowBuilder) or a section accessory.
///
/// Each style has its own constructor (D-197): [`primary`](Self::primary),
/// [`secondary`](Self::secondary), [`success`](Self::success) and [`danger`](Self::danger)
/// take the `custom_id` your app receives when the button is clicked; [`link`](Self::link)
/// takes a URL and sends no interaction. Premium buttons have their own builder,
/// [`PremiumButtonBuilder`] (D-202).
///
/// [`build`](Self::build) checks what Discord checks (D-200): a `custom_id` of 1-100
/// characters (counted as given, not trimmed), a label of at most 80 characters that is not
/// blank — or an emoji instead —, and for links a URL of at most 512 characters starting with
/// `http://`, `https://` or `discord://`.
///
/// discord.js counterpart: `ButtonBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{ActionRowBuilder, ButtonBuilder};
/// use djs_model::emoji::PartialEmoji;
///
/// # fn main() -> Result<(), djs_builders::ValidationError> {
/// let row = ActionRowBuilder::new()
///     .component(ButtonBuilder::success("confirm").label("Confirm"))
///     .component(ButtonBuilder::danger("cancel").label("Cancel"))
///     .component(ButtonBuilder::secondary("like").emoji(PartialEmoji::Unicode { name: "👍".into() }))
///     .component(ButtonBuilder::link("https://example.com/help").label("Help"))
///     .build()?;
/// assert_eq!(row.children().len(), 4);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct ButtonBuilder {
    button: Button,
}

impl ButtonBuilder {
    fn interactive(style: ButtonStyle, custom_id: impl Into<CompactString>) -> Self {
        Self {
            button: InteractiveButton::new(style, custom_id).into(),
        }
    }

    /// Creates a button in the most prominent style (blurple), sending `custom_id`
    /// (1-100 characters) to your app when clicked.
    ///
    /// discord.js counterpart: `setStyle(ButtonStyle.Primary).setCustomId(…)`.
    pub fn primary(custom_id: impl Into<CompactString>) -> Self {
        Self::interactive(ButtonStyle::Primary, custom_id)
    }

    /// Creates a button in the neutral style (grey).
    ///
    /// discord.js counterpart: `setStyle(ButtonStyle.Secondary).setCustomId(…)`.
    pub fn secondary(custom_id: impl Into<CompactString>) -> Self {
        Self::interactive(ButtonStyle::Secondary, custom_id)
    }

    /// Creates a button in the confirming style (green).
    ///
    /// discord.js counterpart: `setStyle(ButtonStyle.Success).setCustomId(…)`.
    pub fn success(custom_id: impl Into<CompactString>) -> Self {
        Self::interactive(ButtonStyle::Success, custom_id)
    }

    /// Creates a button in the destructive style (red).
    ///
    /// discord.js counterpart: `setStyle(ButtonStyle.Danger).setCustomId(…)`.
    pub fn danger(custom_id: impl Into<CompactString>) -> Self {
        Self::interactive(ButtonStyle::Danger, custom_id)
    }

    /// Creates a button that opens `url` (`http://`, `https://` or `discord://`, at most 512
    /// characters). It sends no interaction to your app.
    ///
    /// discord.js counterpart: `setStyle(ButtonStyle.Link).setURL(…)`.
    pub fn link(url: impl Into<CompactString>) -> Self {
        Self {
            button: LinkButton::new(url).into(),
        }
    }

    /// Creates a button to buy the SKU `sku_id`, which shows the SKU's name and price and
    /// cannot have a label or emoji (D-202).
    ///
    /// discord.js counterpart: `setStyle(ButtonStyle.Premium).setSKUId(…)`.
    pub fn premium(sku_id: SkuId) -> PremiumButtonBuilder {
        PremiumButtonBuilder {
            button: PremiumButton::new(sku_id),
        }
    }

    /// Sets the text on the button (at most 80 characters, not blank). A button needs a label
    /// or an [emoji](Self::emoji).
    ///
    /// discord.js counterpart: `setLabel`.
    pub fn label(mut self, label: impl Into<CompactString>) -> Self {
        let label = Some(label.into());
        match &mut self.button {
            Button::Interactive(button) => button.label = label,
            Button::Link(button) => button.label = label,
            _ => {}
        }
        self
    }

    /// Sets the emoji shown before the label (or alone).
    ///
    /// discord.js counterpart: `setEmoji`.
    pub fn emoji(mut self, emoji: impl Into<PartialEmoji>) -> Self {
        let emoji = Some(emoji.into());
        match &mut self.button {
            Button::Interactive(button) => button.emoji = emoji,
            Button::Link(button) => button.emoji = emoji,
            _ => {}
        }
        self
    }

    /// Sets whether the button is greyed out and cannot be clicked.
    ///
    /// discord.js counterpart: `setDisabled`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        match &mut self.button {
            Button::Interactive(button) => button.disabled = disabled,
            Button::Link(button) => button.disabled = disabled,
            _ => {}
        }
        self
    }

    /// Sets the component's numeric identifier (Discord generates one otherwise); it must be
    /// unique in the message.
    ///
    /// discord.js counterpart: `setId`.
    pub fn id(mut self, id: i32) -> Self {
        match &mut self.button {
            Button::Interactive(button) => button.id = Some(id),
            Button::Link(button) => button.id = Some(id),
            _ => {}
        }
        self
    }

    /// Checks the button and returns it as a [`Component`].
    ///
    /// discord.js counterpart: `ButtonBuilder#toJSON`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::Missing`] for an empty `custom_id` or a button with neither a label
    /// nor an emoji, [`ValidationError::TooLong`] for a `custom_id`, label or URL above its
    /// limit, [`ValidationError::InvalidUrl`] for a link with another scheme.
    pub fn build(self) -> Result<Component, ValidationError> {
        validate::button(&Path::Root, &self.button)?;
        Ok(self.into())
    }

    /// Returns the button without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> Component {
        self.into()
    }
}

impl From<ButtonBuilder> for Component {
    fn from(button: ButtonBuilder) -> Self {
        Self::Button(button.button)
    }
}

/// Builds a [premium button](https://docs.discord.com/developers/components/reference#button-button-styles),
/// which lets users buy one of your app's SKUs; created by [`ButtonBuilder::premium`].
///
/// Discord shows the SKU's name, icon and price on it, so it has no label, emoji or
/// `custom_id`, and clicking it sends no interaction. Its own type (D-202) makes adding a
/// label a compile-time error.
///
/// discord.js counterpart: `ButtonBuilder` with `ButtonStyle.Premium`.
///
/// # Examples
///
/// ```
/// use djs_builders::{ActionRowBuilder, ButtonBuilder};
/// use djs_model::id::SkuId;
///
/// let row = ActionRowBuilder::new()
///     .component(ButtonBuilder::premium(SkuId::new(1_088_510_058_284_990_888)))
///     .build()
///     .unwrap();
/// assert_eq!(row.children().len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a row or built"]
pub struct PremiumButtonBuilder {
    button: PremiumButton,
}

impl PremiumButtonBuilder {
    /// Sets whether the button is greyed out and cannot be clicked.
    ///
    /// discord.js counterpart: `setDisabled`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.button.disabled = disabled;
        self
    }

    /// Sets the component's numeric identifier (Discord generates one otherwise); it must be
    /// unique in the message.
    ///
    /// discord.js counterpart: `setId`.
    pub fn id(mut self, id: i32) -> Self {
        self.button.id = Some(id);
        self
    }

    /// Returns the button as a [`Component`]. It returns a `Result` like every other builder
    /// (D-179), so code can treat all buttons alike.
    ///
    /// # Errors
    ///
    /// None: a premium button has nothing that can break a rule.
    pub fn build(self) -> Result<Component, ValidationError> {
        Ok(self.into())
    }

    /// Returns the button as a [`Component`] (D-180; the same as [`build`](Self::build)).
    #[must_use]
    pub fn build_unchecked(self) -> Component {
        self.into()
    }
}

impl From<PremiumButtonBuilder> for Component {
    fn from(button: PremiumButtonBuilder) -> Self {
        button.button.into()
    }
}
