//! [`EmbedBuilder`] and its pieces, [`EmbedAuthorBuilder`] and [`EmbedFooterBuilder`], plus
//! the message-wide checks [`embed_length`] and [`validate_embeds`].

use compact_str::CompactString;
use djs_model::{
    message::{Embed, EmbedAuthor, EmbedField, EmbedFooter, EmbedImage},
    util::Timestamp,
};

use crate::{
    ValidationError,
    check::{self, WEB, WEB_OR_ATTACHMENT},
};

/// Discord's [embed limits](https://docs.discord.com/developers/resources/message#embed-object-embed-limits).
mod limits {
    pub(super) const TITLE: usize = 256;
    pub(super) const DESCRIPTION: usize = 4096;
    pub(super) const FIELDS: usize = 25;
    pub(super) const FIELD_NAME: usize = 256;
    pub(super) const FIELD_VALUE: usize = 1024;
    pub(super) const FOOTER_TEXT: usize = 2048;
    pub(super) const AUTHOR_NAME: usize = 256;
    /// Across the texts of one embed — and of all embeds of a message.
    pub(super) const TOTAL: usize = 6000;
    /// Embeds in one message.
    pub(super) const EMBEDS: usize = 10;
    /// The largest RGB color.
    pub(super) const COLOR: u32 = 0xFF_FFFF;
}

/// What an embed needs at least one of (D-191), for [`ValidationError::Empty`].
const EMBED_CONTENT: &str =
    "a title, a description, a field, an author, a footer, an image or a thumbnail";

/// Builds an [embed](https://docs.discord.com/developers/resources/message#embed-object) — a
/// rich content block for a message, an interaction reply or a webhook — and checks it
/// against Discord's limits.
///
/// Every setter takes and returns the builder (D-178), stores the value as given and never
/// fails; [`build`](Self::build) checks everything at the end (D-179) and returns the first
/// problem as a [`ValidationError`] (D-182). [`build_unchecked`](Self::build_unchecked) skips
/// the checks (D-180).
///
/// `build` checks what Discord checks, as measured on the live API (D-183, D-188, D-191):
///
/// | Part | Rule |
/// |------|------|
/// | [`title`](Self::title) | at most 256 characters |
/// | [`description`](Self::description) | at most 4096 characters |
/// | [`field`](Self::field)s | at most 25; name at most 256, value at most 1024 characters |
/// | [`footer`](Self::footer) text | at most 2048 characters |
/// | [`author`](Self::author) name | at most 256 characters |
/// | all of the texts above | at most 6000 characters together |
/// | [`color`](Self::color) | at most `0xFFFFFF` |
/// | [`url`](Self::url), author URL | `http://` or `https://` |
/// | [`image`](Self::image), [`thumbnail`](Self::thumbnail), icons | `http://`, `https://` or `attachment://` |
/// | the whole embed | a non-blank title, description, footer or author name, a field, an image or a thumbnail |
///
/// Characters are Unicode code points after trimming leading and trailing whitespace, the way
/// Discord counts them; schemes are compared without regard to case. Empty or blank texts
/// are allowed — Discord accepts them, as long as the embed has some content.
///
/// The 6000-character total also applies to **all embeds of a message together** (at most
/// 10 embeds); check a whole list with [`validate_embeds`].
///
/// discord.js counterpart: `EmbedBuilder`. Unlike discord.js, the setters do not throw, an
/// emoji counts as one character (not two UTF-16 units), and empty field names and values
/// are allowed only because Discord allows them.
///
/// # Examples
///
/// ```
/// use djs_builders::{EmbedBuilder, EmbedFooterBuilder};
///
/// # fn main() -> Result<(), djs_builders::ValidationError> {
/// let embed = EmbedBuilder::new()
///     .title("Server status")
///     .url("https://example.com/status")
///     .description("Everything is running.")
///     .color(0x57_F287)
///     .author("Status bot")
///     .field("Uptime", "14 days")
///     .inline_field("Ping", "42 ms")
///     .inline_field("Players", "17")
///     .footer(EmbedFooterBuilder::new("Updated").icon_url("https://example.com/icon.png"))
///     .thumbnail("attachment://logo.png")
///     .build()?;
///
/// assert_eq!(embed.title.as_deref(), Some("Server status"));
/// assert_eq!(embed.fields.len(), 3);
/// assert!(embed.fields[1].inline);
/// # Ok(())
/// # }
/// ```
///
/// A problem is reported by `build`, not by the setter:
///
/// ```
/// use djs_builders::{EmbedBuilder, ValidationError};
///
/// let error = EmbedBuilder::new()
///     .title("Results")
///     .url("ftp://example.com/results")
///     .build()
///     .unwrap_err();
/// assert_eq!(error.field(), "url");
/// assert!(matches!(error, ValidationError::InvalidUrl { .. }));
/// ```
///
/// Send it with a message:
///
/// ```
/// use djs_builders::EmbedBuilder;
/// use djs_model::rest::CreateMessage;
///
/// # fn main() -> Result<(), djs_builders::ValidationError> {
/// let mut message = CreateMessage::default();
/// message.embeds.push(EmbedBuilder::new().description("Hello!").build()?);
/// assert_eq!(
///     serde_json::to_string(&message).unwrap(),
///     r#"{"embeds":[{"description":"Hello!"}]}"#
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct EmbedBuilder {
    embed: Embed,
}

impl EmbedBuilder {
    /// Creates a builder for an empty embed.
    ///
    /// An embed needs some content before [`build`](Self::build) accepts it: a title, a
    /// description, a field, an author, a footer, an image or a thumbnail.
    ///
    /// discord.js counterpart: `new EmbedBuilder()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the title (at most 256 characters), shown in bold at the top.
    ///
    /// discord.js counterpart: `EmbedBuilder#setTitle`.
    pub fn title(mut self, title: impl Into<CompactString>) -> Self {
        self.embed.title = Some(title.into());
        self
    }

    /// Sets the URL the title links to (`http://` or `https://`).
    ///
    /// Embeds of one message that share a URL are shown as one by Discord, which is how
    /// several images can appear in a single embed.
    ///
    /// discord.js counterpart: `EmbedBuilder#setURL`.
    pub fn url(mut self, url: impl Into<CompactString>) -> Self {
        self.embed.url = Some(url.into());
        self
    }

    /// Sets the description (at most 4096 characters), the main text of the embed. It can
    /// use markdown.
    ///
    /// discord.js counterpart: `EmbedBuilder#setDescription`.
    pub fn description(mut self, description: impl Into<CompactString>) -> Self {
        self.embed.description = Some(description.into());
        self
    }

    /// Sets the color of the left border as an RGB integer (`0xRRGGBB`, at most `0xFFFFFF`).
    ///
    /// discord.js counterpart: `EmbedBuilder#setColor` with a number.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::EmbedBuilder;
    ///
    /// let embed = EmbedBuilder::new().title("x").color(0x58_65F2).build().unwrap();
    /// assert_eq!(embed.color, Some(0x58_65F2));
    /// ```
    pub fn color(mut self, color: u32) -> Self {
        self.embed.color = Some(color);
        self
    }

    /// Sets the color of the left border from its red, green and blue parts.
    ///
    /// discord.js counterpart: `EmbedBuilder#setColor` with an `[r, g, b]` tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::EmbedBuilder;
    ///
    /// let embed = EmbedBuilder::new().title("x").color_rgb(88, 101, 242).build().unwrap();
    /// assert_eq!(embed.color, Some(0x58_65F2));
    /// ```
    pub fn color_rgb(self, red: u8, green: u8, blue: u8) -> Self {
        self.color(u32::from_be_bytes([0, red, green, blue]))
    }

    /// Sets the author shown above the title: a name (at most 256 characters), or an
    /// [`EmbedAuthorBuilder`] with a link and an icon too.
    ///
    /// discord.js counterpart: `EmbedBuilder#setAuthor`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::{EmbedAuthorBuilder, EmbedBuilder};
    ///
    /// let plain = EmbedBuilder::new().author("Status bot").build().unwrap();
    /// assert_eq!(plain.author.unwrap().name, "Status bot");
    ///
    /// let linked = EmbedBuilder::new()
    ///     .author(
    ///         EmbedAuthorBuilder::new("Status bot")
    ///             .url("https://example.com")
    ///             .icon_url("https://example.com/avatar.png"),
    ///     )
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(linked.author.unwrap().url.as_deref(), Some("https://example.com"));
    /// ```
    pub fn author(mut self, author: impl Into<EmbedAuthorBuilder>) -> Self {
        self.embed.author = Some(Box::new(author.into().author));
        self
    }

    /// Sets the footer: a text (at most 2048 characters), or an [`EmbedFooterBuilder`] with
    /// an icon too. A [`timestamp`](Self::timestamp) is shown next to it.
    ///
    /// discord.js counterpart: `EmbedBuilder#setFooter`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::{EmbedBuilder, EmbedFooterBuilder};
    ///
    /// let embed = EmbedBuilder::new()
    ///     .footer(EmbedFooterBuilder::new("Page 1 of 3").icon_url("attachment://icon.png"))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(embed.footer.unwrap().text, "Page 1 of 3");
    /// ```
    pub fn footer(mut self, footer: impl Into<EmbedFooterBuilder>) -> Self {
        self.embed.footer = Some(Box::new(footer.into().footer));
        self
    }

    /// Adds a field on its own line: a `name` (at most 256 characters) above a `value` (at
    /// most 1024). An embed has at most 25 fields.
    ///
    /// discord.js counterpart: `EmbedBuilder#addFields` with `{ name, value }`.
    pub fn field(
        mut self,
        name: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.embed.fields.push(EmbedField::new(name, value, false));
        self
    }

    /// Adds an inline field, shown next to the inline fields around it (up to three per
    /// row on desktop).
    ///
    /// discord.js counterpart: `EmbedBuilder#addFields` with `{ name, value, inline: true }`.
    pub fn inline_field(
        mut self,
        name: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.embed.fields.push(EmbedField::new(name, value, true));
        self
    }

    /// Adds several fields at once, after those already added.
    ///
    /// discord.js counterpart: `EmbedBuilder#addFields`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::EmbedBuilder;
    /// use djs_model::message::EmbedField;
    ///
    /// let scores = [("Ada", 31), ("Grace", 27)];
    /// let embed = EmbedBuilder::new()
    ///     .title("Scores")
    ///     .fields(scores.iter().map(|(name, score)| EmbedField::new(*name, score.to_string(), true)))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(embed.fields[1].value, "27");
    /// ```
    pub fn fields(mut self, fields: impl IntoIterator<Item = EmbedField>) -> Self {
        self.embed.fields.extend(fields);
        self
    }

    /// Sets the large image below the fields: a link (`http://`, `https://`) or a file
    /// uploaded with the message (`attachment://name.png`).
    ///
    /// discord.js counterpart: `EmbedBuilder#setImage`.
    pub fn image(mut self, url: impl Into<CompactString>) -> Self {
        self.embed.image = Some(Box::new(EmbedImage::new(url)));
        self
    }

    /// Sets the small image in the top-right corner, with the same URLs as
    /// [`image`](Self::image).
    ///
    /// discord.js counterpart: `EmbedBuilder#setThumbnail`.
    pub fn thumbnail(mut self, url: impl Into<CompactString>) -> Self {
        self.embed.thumbnail = Some(Box::new(EmbedImage::new(url)));
        self
    }

    /// Sets the time shown in the footer, in each viewer's time zone.
    ///
    /// A timestamp alone is not content: an embed with only a timestamp is rejected.
    ///
    /// discord.js counterpart: `EmbedBuilder#setTimestamp` with a date.
    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.embed.timestamp = Some(timestamp);
        self
    }

    /// Sets the time shown in the footer to now (the system clock).
    ///
    /// discord.js counterpart: `EmbedBuilder#setTimestamp()` without an argument.
    ///
    /// # Panics
    ///
    /// If the system clock is set before 1970 (see [`Timestamp::now`]).
    pub fn timestamp_now(self) -> Self {
        self.timestamp(Timestamp::now())
    }

    /// Checks the embed against Discord's limits (see the [type docs](Self)) and returns it.
    ///
    /// discord.js counterpart: `EmbedBuilder#toJSON` (which does not validate; the setters
    /// do).
    ///
    /// # Errors
    ///
    /// The first [`ValidationError`] found, checking the parts in this order: title,
    /// description, URL, color, fields, footer, image, thumbnail, author, the 6000-character
    /// total ([`TooLong`](ValidationError::TooLong) with the field `embed`), and finally
    /// whether the embed has any content ([`Empty`](ValidationError::Empty)).
    pub fn build(self) -> Result<Embed, ValidationError> {
        validate(&self.embed)?;
        Ok(self.embed)
    }

    /// Returns the embed without any check (D-180) — for example when Discord has raised a
    /// limit before this crate has. Discord rejects an invalid embed with
    /// `50035 Invalid Form Body`.
    ///
    /// discord.js counterpart: `EmbedBuilder#toJSON` with `disableValidators()`.
    #[must_use]
    pub fn build_unchecked(self) -> Embed {
        self.embed
    }
}

/// Starts from an existing embed — for example one received in a message, to send it again
/// with changes. Every part is kept; a setter replaces its part (and
/// [`field`](EmbedBuilder::field) adds after the existing fields).
///
/// discord.js counterpart: `EmbedBuilder.from(embed)` / `new EmbedBuilder(data)`.
///
/// # Examples
///
/// ```
/// use djs_builders::EmbedBuilder;
/// use djs_model::message::Embed;
///
/// let received: Embed = serde_json::from_str(r#"{"title":"Poll","description":"Open"}"#).unwrap();
/// let closed = EmbedBuilder::from(received).description("Closed").build().unwrap();
/// assert_eq!(closed.title.as_deref(), Some("Poll"));
/// assert_eq!(closed.description.as_deref(), Some("Closed"));
/// ```
impl From<Embed> for EmbedBuilder {
    fn from(embed: Embed) -> Self {
        Self { embed }
    }
}

/// Checks everything [`EmbedBuilder::build`] documents, in that order.
fn validate(embed: &Embed) -> Result<(), ValidationError> {
    let mut total = 0;
    if let Some(title) = &embed.title {
        total += check::text(|| "title".into(), title, limits::TITLE)?;
    }
    if let Some(description) = &embed.description {
        total += check::text(|| "description".into(), description, limits::DESCRIPTION)?;
    }
    if let Some(url) = &embed.url {
        check::url("url", url, WEB)?;
    }
    if let Some(color) = embed.color
        && color > limits::COLOR
    {
        return Err(ValidationError::OutOfRange {
            field: "color".into(),
            min: 0,
            max: limits::COLOR.into(),
            actual: color.into(),
        });
    }
    check::count("fields", embed.fields.len(), limits::FIELDS)?;
    for (index, field) in embed.fields.iter().enumerate() {
        total += check::text(
            || format!("fields[{index}].name").into(),
            &field.name,
            limits::FIELD_NAME,
        )?;
        total += check::text(
            || format!("fields[{index}].value").into(),
            &field.value,
            limits::FIELD_VALUE,
        )?;
    }
    if let Some(footer) = &embed.footer {
        total += validate_footer(footer, &IN_EMBED_FOOTER)?;
    }
    if let Some(image) = &embed.image {
        check::url("image.url", &image.url, WEB_OR_ATTACHMENT)?;
    }
    if let Some(thumbnail) = &embed.thumbnail {
        check::url("thumbnail.url", &thumbnail.url, WEB_OR_ATTACHMENT)?;
    }
    if let Some(author) = &embed.author {
        total += validate_author(author, &IN_EMBED_AUTHOR)?;
    }
    if total > limits::TOTAL {
        return Err(ValidationError::TooLong {
            field: "embed".into(),
            max: limits::TOTAL,
            actual: total,
        });
    }
    if !has_content(embed) {
        return Err(ValidationError::Empty {
            field: "embed".into(),
            needs: EMBED_CONTENT,
        });
    }
    Ok(())
}

/// Whether Discord counts `embed` as having content (D-191): blank texts, the URL, the
/// timestamp and the color do not count; any field, even one with blank texts, does.
fn has_content(embed: &Embed) -> bool {
    let text = |text: Option<&str>| text.is_some_and(|text| !check::is_blank(text));
    text(embed.title.as_deref())
        || text(embed.description.as_deref())
        || !embed.fields.is_empty()
        || text(embed.footer.as_ref().map(|footer| footer.text.as_str()))
        || text(embed.author.as_ref().map(|author| author.name.as_str()))
        || embed.image.is_some()
        || embed.thumbnail.is_some()
}

/// Returns the number of characters of `embed` that count toward Discord's 6000-character
/// limit: its title, description, field names and values, footer text and author name, each
/// measured in Unicode code points after trimming (D-183).
///
/// discord.js counterpart: `embedLength` (which counts UTF-16 code units, untrimmed).
///
/// # Examples
///
/// ```
/// use djs_builders::{EmbedBuilder, embed_length};
///
/// let embed = EmbedBuilder::new()
///     .title("  Hi  ")        // 2: surrounding whitespace is not counted
///     .description("🔥🔥🔥") // 3: one per code point
///     .field("a", "bc")       // 3
///     .build_unchecked();
/// assert_eq!(embed_length(&embed), 8);
/// ```
#[must_use]
pub fn embed_length(embed: &Embed) -> usize {
    let mut total = 0;
    for text in [&embed.title, &embed.description].into_iter().flatten() {
        total += check::length(text);
    }
    for field in &embed.fields {
        total += check::length(&field.name) + check::length(&field.value);
    }
    if let Some(footer) = &embed.footer {
        total += check::length(&footer.text);
    }
    if let Some(author) = &embed.author {
        total += check::length(&author.name);
    }
    total
}

/// Checks the limits that apply to all embeds of one message together: at most 10 embeds
/// and at most 6000 characters across them (see [`embed_length`]).
///
/// The embeds themselves are not checked again; build each with [`EmbedBuilder::build`].
///
/// # Errors
///
/// [`ValidationError::TooMany`] for more than 10 embeds, then
/// [`ValidationError::TooLong`] for more than 6000 characters, both with the field `embeds`.
///
/// # Examples
///
/// ```
/// use djs_builders::{EmbedBuilder, ValidationError, validate_embeds};
///
/// let page = EmbedBuilder::new().description("x".repeat(4000)).build().unwrap();
/// assert!(validate_embeds(std::slice::from_ref(&page)).is_ok());
/// assert_eq!(
///     validate_embeds(&[page.clone(), page]),
///     Err(ValidationError::TooLong { field: "embeds".into(), max: 6000, actual: 8000 })
/// );
/// ```
pub fn validate_embeds(embeds: &[Embed]) -> Result<(), ValidationError> {
    check::count("embeds", embeds.len(), limits::EMBEDS)?;
    let total = embeds.iter().map(embed_length).sum();
    if total > limits::TOTAL {
        return Err(ValidationError::TooLong {
            field: "embeds".into(),
            max: limits::TOTAL,
            actual: total,
        });
    }
    Ok(())
}

/// The paths of an author's parts in a [`ValidationError`].
struct AuthorPaths {
    name: &'static str,
    url: &'static str,
    icon_url: &'static str,
}

const IN_EMBED_AUTHOR: AuthorPaths = AuthorPaths {
    name: "author.name",
    url: "author.url",
    icon_url: "author.icon_url",
};

const ALONE_AUTHOR: AuthorPaths = AuthorPaths {
    name: "name",
    url: "url",
    icon_url: "icon_url",
};

/// Checks an author and returns the length of its name.
fn validate_author(author: &EmbedAuthor, paths: &AuthorPaths) -> Result<usize, ValidationError> {
    let length = check::text(|| paths.name.into(), &author.name, limits::AUTHOR_NAME)?;
    if let Some(url) = &author.url {
        check::url(paths.url, url, WEB)?;
    }
    if let Some(icon_url) = &author.icon_url {
        check::url(paths.icon_url, icon_url, WEB_OR_ATTACHMENT)?;
    }
    Ok(length)
}

/// The paths of a footer's parts in a [`ValidationError`].
struct FooterPaths {
    text: &'static str,
    icon_url: &'static str,
}

const IN_EMBED_FOOTER: FooterPaths = FooterPaths {
    text: "footer.text",
    icon_url: "footer.icon_url",
};

const ALONE_FOOTER: FooterPaths = FooterPaths {
    text: "text",
    icon_url: "icon_url",
};

/// Checks a footer and returns the length of its text.
fn validate_footer(footer: &EmbedFooter, paths: &FooterPaths) -> Result<usize, ValidationError> {
    let length = check::text(|| paths.text.into(), &footer.text, limits::FOOTER_TEXT)?;
    if let Some(icon_url) = &footer.icon_url {
        check::url(paths.icon_url, icon_url, WEB_OR_ATTACHMENT)?;
    }
    Ok(length)
}

/// Builds the [author](https://docs.discord.com/developers/resources/message#embed-object-embed-author-structure)
/// of an embed — a name with an optional link and icon — for [`EmbedBuilder::author`].
///
/// A plain name converts into it, so `.author("Bot")` is enough when there is no link or
/// icon. [`EmbedBuilder::build`] checks it as part of the embed; [`build`](Self::build) checks
/// it on its own.
///
/// discord.js counterpart: the `EmbedAuthorOptions` passed to `EmbedBuilder#setAuthor`.
///
/// # Examples
///
/// ```
/// use djs_builders::{EmbedAuthorBuilder, EmbedBuilder};
///
/// let embed = EmbedBuilder::new()
///     .author(EmbedAuthorBuilder::new("Ada").icon_url("https://example.com/ada.png"))
///     .build()
///     .unwrap();
/// assert_eq!(embed.author.unwrap().icon_url.as_deref(), Some("https://example.com/ada.png"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[must_use = "a builder does nothing until it is passed to `EmbedBuilder::author` or built"]
pub struct EmbedAuthorBuilder {
    author: EmbedAuthor,
}

impl EmbedAuthorBuilder {
    /// Creates an author named `name` (at most 256 characters).
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            author: EmbedAuthor::new(name),
        }
    }

    /// Sets the URL the name links to (`http://` or `https://`; not `attachment://`).
    pub fn url(mut self, url: impl Into<CompactString>) -> Self {
        self.author.url = Some(url.into());
        self
    }

    /// Sets the icon shown before the name: a link (`http://`, `https://`) or an uploaded
    /// file (`attachment://name.png`).
    pub fn icon_url(mut self, url: impl Into<CompactString>) -> Self {
        self.author.icon_url = Some(url.into());
        self
    }

    /// Checks the author on its own and returns it; the paths in a [`ValidationError`] are
    /// `name`, `url` and `icon_url`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::TooLong`] for a name above 256 characters,
    /// [`ValidationError::InvalidUrl`] for a URL with a scheme Discord does not allow.
    pub fn build(self) -> Result<EmbedAuthor, ValidationError> {
        validate_author(&self.author, &ALONE_AUTHOR)?;
        Ok(self.author)
    }

    /// Returns the author without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> EmbedAuthor {
        self.author
    }
}

/// Builds the [footer](https://docs.discord.com/developers/resources/message#embed-object-embed-footer-structure)
/// of an embed — a text with an optional icon — for [`EmbedBuilder::footer`].
///
/// A plain text converts into it, so `.footer("Page 1")` is enough when there is no icon.
/// [`EmbedBuilder::build`] checks it as part of the embed; [`build`](Self::build) checks it on
/// its own.
///
/// discord.js counterpart: the `EmbedFooterOptions` passed to `EmbedBuilder#setFooter`.
///
/// # Examples
///
/// ```
/// use djs_builders::{EmbedBuilder, EmbedFooterBuilder};
///
/// let embed = EmbedBuilder::new()
///     .footer(EmbedFooterBuilder::new("Powered by djs-rs").icon_url("https://example.com/i.png"))
///     .build()
///     .unwrap();
/// assert_eq!(embed.footer.unwrap().icon_url.as_deref(), Some("https://example.com/i.png"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[must_use = "a builder does nothing until it is passed to `EmbedBuilder::footer` or built"]
pub struct EmbedFooterBuilder {
    footer: EmbedFooter,
}

impl EmbedFooterBuilder {
    /// Creates a footer with `text` (at most 2048 characters).
    pub fn new(text: impl Into<CompactString>) -> Self {
        Self {
            footer: EmbedFooter::new(text),
        }
    }

    /// Sets the icon shown before the text: a link (`http://`, `https://`) or an uploaded
    /// file (`attachment://name.png`).
    pub fn icon_url(mut self, url: impl Into<CompactString>) -> Self {
        self.footer.icon_url = Some(url.into());
        self
    }

    /// Checks the footer on its own and returns it; the paths in a [`ValidationError`] are
    /// `text` and `icon_url`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::TooLong`] for a text above 2048 characters,
    /// [`ValidationError::InvalidUrl`] for an icon with a scheme Discord does not allow.
    pub fn build(self) -> Result<EmbedFooter, ValidationError> {
        validate_footer(&self.footer, &ALONE_FOOTER)?;
        Ok(self.footer)
    }

    /// Returns the footer without any check (D-180).
    #[must_use]
    pub fn build_unchecked(self) -> EmbedFooter {
        self.footer
    }
}

/// Implements the conversions of a plain text (and of the finished model type) into a piece
/// builder.
macro_rules! from_text {
    ($builder:ident, $model:ident, $part:ident) => {
        impl From<&str> for $builder {
            fn from(text: &str) -> Self {
                Self::new(text)
            }
        }

        impl From<&String> for $builder {
            fn from(text: &String) -> Self {
                Self::new(text)
            }
        }

        impl From<String> for $builder {
            fn from(text: String) -> Self {
                Self::new(text)
            }
        }

        impl From<CompactString> for $builder {
            fn from(text: CompactString) -> Self {
                Self::new(text)
            }
        }

        impl From<$model> for $builder {
            fn from($part: $model) -> Self {
                Self { $part }
            }
        }
    };
}

from_text!(EmbedAuthorBuilder, EmbedAuthor, author);
from_text!(EmbedFooterBuilder, EmbedFooter, footer);
