//! Components V2 content builders: [`TextDisplayBuilder`], [`ThumbnailBuilder`],
//! [`MediaGalleryBuilder`] with [`MediaGalleryItemBuilder`], and [`FileBuilder`].

use compact_str::CompactString;
use djs_model::component::{
    File, MediaGallery, MediaGalleryItem, TextDisplay, Thumbnail, UnfurledMediaItem,
};

/// Builds a [text display](https://docs.discord.com/developers/components/reference#text-display):
/// markdown text in a Components V2 message, a container or a section (1-4000 characters,
/// not blank). All text displays of a message share 4000 characters (D-200).
///
/// Sections and containers also take text directly with `.text("…")` (D-204).
///
/// discord.js counterpart: `TextDisplayBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::TextDisplayBuilder;
///
/// let text = TextDisplayBuilder::new("## Release notes\n- faster cache").build().unwrap();
/// assert_eq!(text.kind(), djs_model::component::ComponentType::TextDisplay);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct TextDisplayBuilder {
    component: TextDisplay,
}

impl TextDisplayBuilder {
    /// Creates a text display showing `content` (markdown, 1-4000 characters).
    ///
    /// discord.js counterpart: `new TextDisplayBuilder().setContent(…)`.
    pub fn new(content: impl Into<CompactString>) -> Self {
        Self {
            component: TextDisplay::new(content),
        }
    }
}

component_builder!(TextDisplayBuilder => TextDisplay);

/// Builds a [thumbnail](https://docs.discord.com/developers/components/reference#thumbnail):
/// a small image, the accessory of a [section](crate::SectionBuilder).
///
/// The URL is a link (`http://`, `https://`) or a file uploaded with the message
/// (`attachment://name.png`); other schemes are rejected although Discord accepts them, since
/// it cannot show them (D-190, D-205). The description (alt text) has 1-1024 characters.
///
/// discord.js counterpart: `ThumbnailBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a section or built"]
pub struct ThumbnailBuilder {
    component: Thumbnail,
}

impl ThumbnailBuilder {
    /// Creates a thumbnail showing `url`.
    ///
    /// discord.js counterpart: `new ThumbnailBuilder().setURL(…)`.
    pub fn new(url: impl Into<CompactString>) -> Self {
        Self {
            component: Thumbnail::new(UnfurledMediaItem::new(url)),
        }
    }

    /// Sets the alt text (1-1024 characters).
    ///
    /// discord.js counterpart: `setDescription`.
    pub fn description(mut self, description: impl Into<CompactString>) -> Self {
        self.component.description = Some(description.into());
        self
    }

    /// Sets whether the image is blurred until clicked.
    ///
    /// discord.js counterpart: `setSpoiler`.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.component.spoiler = spoiler;
        self
    }
}

component_builder!(ThumbnailBuilder => Thumbnail);

/// Builds a [media gallery](https://docs.discord.com/developers/components/reference#media-gallery):
/// a grid of 1-10 images and videos.
///
/// Add plain items with [`item`](Self::item) and items with alt text or a spoiler with
/// [`item_with`](Self::item_with) (D-204). Each URL is a link or an uploaded file.
///
/// discord.js counterpart: `MediaGalleryBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{MediaGalleryBuilder, MediaGalleryItemBuilder};
///
/// let gallery = MediaGalleryBuilder::new()
///     .item("https://example.com/a.png")
///     .item_with(MediaGalleryItemBuilder::new("attachment://b.png").description("A chart").spoiler(true))
///     .build()
///     .unwrap();
/// assert_eq!(gallery.kind(), djs_model::component::ComponentType::MediaGallery);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct MediaGalleryBuilder {
    component: MediaGallery,
}

impl Default for MediaGalleryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaGalleryBuilder {
    /// Creates an empty gallery; it needs 1-10 items.
    ///
    /// discord.js counterpart: `new MediaGalleryBuilder()`.
    pub fn new() -> Self {
        Self {
            component: MediaGallery::new(Vec::new()),
        }
    }

    /// Adds an item showing `url`.
    ///
    /// discord.js counterpart: `addItems`.
    pub fn item(mut self, url: impl Into<CompactString>) -> Self {
        self.component
            .items
            .push(MediaGalleryItem::new(UnfurledMediaItem::new(url)));
        self
    }

    /// Adds an item with alt text or a spoiler.
    ///
    /// discord.js counterpart: `addItems(new MediaGalleryItemBuilder()…)`.
    pub fn item_with(mut self, item: MediaGalleryItemBuilder) -> Self {
        self.component.items.push(item.item);
        self
    }

    /// Adds several items: [`MediaGalleryItemBuilder`]s or `djs-model`
    /// [`MediaGalleryItem`]s.
    ///
    /// discord.js counterpart: `addItems`.
    pub fn items<I: Into<MediaGalleryItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.component
            .items
            .extend(items.into_iter().map(Into::into));
        self
    }
}

component_builder!(MediaGalleryBuilder => MediaGallery);

/// Builds an [item of a media gallery](https://docs.discord.com/developers/components/reference#media-gallery-media-gallery-item-structure)
/// with alt text or a spoiler, for [`MediaGalleryBuilder::item_with`].
///
/// discord.js counterpart: `MediaGalleryItemBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "an item does nothing until it is added to a gallery"]
pub struct MediaGalleryItemBuilder {
    item: MediaGalleryItem,
}

impl MediaGalleryItemBuilder {
    /// Creates an item showing `url` (a link or `attachment://name`).
    ///
    /// discord.js counterpart: `new MediaGalleryItemBuilder().setURL(…)`.
    pub fn new(url: impl Into<CompactString>) -> Self {
        Self {
            item: MediaGalleryItem::new(UnfurledMediaItem::new(url)),
        }
    }

    /// Sets the alt text (1-1024 characters).
    ///
    /// discord.js counterpart: `setDescription`.
    pub fn description(mut self, description: impl Into<CompactString>) -> Self {
        self.item.description = Some(description.into());
        self
    }

    /// Sets whether the item is blurred until clicked.
    ///
    /// discord.js counterpart: `setSpoiler`.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.item.spoiler = spoiler;
        self
    }
}

impl From<MediaGalleryItemBuilder> for MediaGalleryItem {
    fn from(item: MediaGalleryItemBuilder) -> Self {
        item.item
    }
}

/// Builds a [file component](https://docs.discord.com/developers/components/reference#file):
/// a file uploaded with the message, shown as a downloadable box.
///
/// The URL must be `attachment://name` naming one of the message's files — Discord refuses
/// links here (D-205).
///
/// discord.js counterpart: `FileBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::FileBuilder;
///
/// assert!(FileBuilder::new("attachment://report.pdf").build().is_ok());
/// assert!(FileBuilder::new("https://example.com/report.pdf").build().is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a message or built"]
pub struct FileBuilder {
    component: File,
}

impl FileBuilder {
    /// Creates a file component showing the uploaded file `url` (`attachment://name`).
    ///
    /// discord.js counterpart: `new FileBuilder().setURL(…)`.
    pub fn new(url: impl Into<CompactString>) -> Self {
        Self {
            component: File::new(UnfurledMediaItem::new(url)),
        }
    }

    /// Sets whether the file is blurred until clicked.
    ///
    /// discord.js counterpart: `setSpoiler`.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.component.spoiler = spoiler;
        self
    }
}

component_builder!(FileBuilder => File);
