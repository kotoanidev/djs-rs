//! Components V2 layout builders: [`ContainerBuilder`], [`SectionBuilder`] and
//! [`SeparatorBuilder`].

use compact_str::CompactString;
use djs_model::component::{
    Component, Container, Section, Separator, SeparatorSpacing, TextDisplay,
};

/// Builds a [container](https://docs.discord.com/developers/components/reference#container):
/// a box around 1-40 components with an optional accent color, like an embed made of
/// components.
///
/// It holds action rows, text displays (or [`text`](Self::text)), sections, media galleries,
/// separators and files (D-198, D-204); [`build`](Self::build) checks the mix, each child, the
/// accent color (at most `0xFFFFFF`) and that `custom_id`s inside are unique.
///
/// discord.js counterpart: `ContainerBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{
///     ActionRowBuilder, ButtonBuilder, ContainerBuilder, MediaGalleryBuilder, SectionBuilder,
///     SeparatorBuilder, ThumbnailBuilder,
/// };
///
/// # fn main() -> Result<(), djs_builders::ValidationError> {
/// let card = ContainerBuilder::new()
///     .accent_color(0x58_65F2)
///     .text("## New release")
///     .component(
///         SectionBuilder::new(ThumbnailBuilder::new("attachment://logo.png"))
///             .text("djs-rs 0.1 is out")
///             .text("-# November 12"),
///     )
///     .component(SeparatorBuilder::new())
///     .component(MediaGalleryBuilder::new().item("https://example.com/a.png"))
///     .component(
///         ActionRowBuilder::new()
///             .component(ButtonBuilder::link("https://example.com").label("Download")),
///     )
///     .build()?;
/// assert_eq!(card.children().len(), 5);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct ContainerBuilder {
    component: Container,
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerBuilder {
    /// Creates an empty container; it needs 1-40 components.
    ///
    /// discord.js counterpart: `new ContainerBuilder()`.
    pub fn new() -> Self {
        Self {
            component: Container::new(Vec::new()),
        }
    }

    /// Adds a text display showing `content` (markdown, 1-4000 characters).
    ///
    /// discord.js counterpart: `addTextDisplayComponents`.
    pub fn text(self, content: impl Into<CompactString>) -> Self {
        self.component(TextDisplay::new(content))
    }

    /// Adds a component after those already added: an action row, text display, section,
    /// media gallery, separator or file — a builder of this crate or a `djs-model`
    /// [`Component`].
    ///
    /// discord.js counterpart: `addActionRowComponents`, `addSectionComponents`, … (one per
    /// type).
    pub fn component(mut self, component: impl Into<Component>) -> Self {
        self.component.components.push(component.into());
        self
    }

    /// Adds several components.
    pub fn components<C: Into<Component>>(
        mut self,
        components: impl IntoIterator<Item = C>,
    ) -> Self {
        self.component
            .components
            .extend(components.into_iter().map(Into::into));
        self
    }

    /// Sets the color of the left border as an RGB integer (`0xRRGGBB`, at most `0xFFFFFF`).
    ///
    /// discord.js counterpart: `setAccentColor`.
    pub fn accent_color(mut self, color: u32) -> Self {
        self.component.accent_color = Some(color);
        self
    }

    /// Sets whether the whole container is blurred until clicked.
    ///
    /// discord.js counterpart: `setSpoiler`.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.component.spoiler = spoiler;
        self
    }
}

component_builder!(ContainerBuilder => Container);

/// Builds a [section](https://docs.discord.com/developers/components/reference#section): 1-3
/// text displays next to an accessory — a [button](crate::ButtonBuilder) or a
/// [thumbnail](crate::ThumbnailBuilder).
///
/// The accessory is given to [`new`](Self::new), so it cannot be forgotten (D-204).
///
/// discord.js counterpart: `SectionBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{ButtonBuilder, SectionBuilder};
///
/// let section = SectionBuilder::new(ButtonBuilder::secondary("settings").label("Settings"))
///     .text("**Notifications** are on")
///     .build()
///     .unwrap();
/// assert_eq!(section.children().len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct SectionBuilder {
    component: Section,
}

impl SectionBuilder {
    /// Creates a section with `accessory` (a button or a thumbnail) and no text yet; it needs
    /// 1-3 texts.
    ///
    /// discord.js counterpart: `new SectionBuilder().setButtonAccessory(…)` /
    /// `setThumbnailAccessory(…)`.
    pub fn new(accessory: impl Into<Component>) -> Self {
        Self {
            component: Section::new(Vec::new(), accessory.into()),
        }
    }

    /// Adds a text display showing `content` (markdown, 1-4000 characters).
    ///
    /// discord.js counterpart: `addTextDisplayComponents`.
    pub fn text(self, content: impl Into<CompactString>) -> Self {
        self.component(TextDisplay::new(content))
    }

    /// Adds a text display (a [`TextDisplayBuilder`](crate::TextDisplayBuilder) or a
    /// `djs-model` value).
    ///
    /// discord.js counterpart: `addTextDisplayComponents`.
    pub fn component(mut self, component: impl Into<Component>) -> Self {
        self.component.components.push(component.into());
        self
    }
}

component_builder!(SectionBuilder => Section);

/// Builds a [separator](https://docs.discord.com/developers/components/reference#separator):
/// vertical space between components, with a line by default.
///
/// discord.js counterpart: `SeparatorBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::SeparatorBuilder;
/// use djs_model::component::SeparatorSpacing;
///
/// let gap = SeparatorBuilder::new().divider(false).spacing(SeparatorSpacing::Large).build().unwrap();
/// assert_eq!(
///     serde_json::to_string(&gap).unwrap(),
///     r#"{"type":14,"divider":false,"spacing":2}"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct SeparatorBuilder {
    component: Separator,
}

impl SeparatorBuilder {
    /// Creates a separator with a line and small spacing.
    ///
    /// discord.js counterpart: `new SeparatorBuilder()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether a line is drawn (default: yes).
    ///
    /// discord.js counterpart: `setDivider`.
    pub fn divider(mut self, divider: bool) -> Self {
        self.component.divider = divider;
        self
    }

    /// Sets the amount of space (default: small).
    ///
    /// discord.js counterpart: `setSpacing`.
    pub fn spacing(mut self, spacing: SeparatorSpacing) -> Self {
        self.component.spacing = Some(spacing);
        self
    }
}

component_builder!(SeparatorBuilder => Separator);
