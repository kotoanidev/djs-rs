//! Markdown: emphasis, code, quotes, headings, lists and links.
//!
//! Every formatter takes any [`Display`](fmt::Display) content, so they nest without
//! allocating: `bold(italic("x"))` is `**_x_**`. They do not escape the content: escape text
//! from users with [`escape_markdown`](crate::escape_markdown) first.

use core::fmt;

/// Defines a formatter wrapping its content between two markers.
macro_rules! wrapper {
    (
        $(#[$meta:meta])*
        $function:ident => $type:ident, $open:literal, $close:literal,
        $js:literal, $example:literal => $output:literal
    ) => {
        #[doc = concat!("Content written as `", $open, "…", $close, "`. Returned by [`", stringify!($function), "`].")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[must_use = "a formatter does nothing until it is written"]
        pub struct $type<T>(T);

        $(#[$meta])*
        ///
        #[doc = concat!("discord.js counterpart: `", $js, "`.")]
        ///
        /// # Examples
        ///
        /// ```
        #[doc = concat!("use djs_formatters::", stringify!($function), ";")]
        ///
        #[doc = concat!("assert_eq!(", stringify!($function), "(", $example, ").to_string(), ", $output, ");")]
        /// ```
        pub const fn $function<T: fmt::Display>(content: T) -> $type<T> {
            $type(content)
        }

        impl<T: fmt::Display> fmt::Display for $type<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($open, "{}", $close), self.0)
            }
        }
    };
}

wrapper! {
    /// Shows `content` in bold: `**content**`.
    bold => Bold, "**", "**", "bold", r#""important""# => r#""**important**""#
}

wrapper! {
    /// Shows `content` in italics: `_content_`.
    italic => Italic, "_", "_", "italic", r#""maybe""# => r#""_maybe_""#
}

wrapper! {
    /// Underlines `content`: `__content__`.
    underline => Underline, "__", "__", "underline", r#""note""# => r#""__note__""#
}

wrapper! {
    /// Strikes `content` through: `~~content~~`.
    strikethrough => Strikethrough, "~~", "~~", "strikethrough", r#""old""# => r#""~~old~~""#
}

wrapper! {
    /// Hides `content` until it is clicked: `||content||`.
    spoiler => Spoiler, "||", "||", "spoiler", r#""the ending""# => r#""||the ending||""#
}

wrapper! {
    /// Shows `content` as code on one line: `` `content` ``.
    ///
    /// The content cannot contain a backtick; a code block can.
    inline_code => InlineCode, "`", "`", "inlineCode", r#""cargo build""# => r#""`cargo build`""#
}

wrapper! {
    /// Quotes one line: `> content`.
    ///
    /// Only the first line is quoted; [`block_quote`] quotes everything after it.
    quote => Quote, "> ", "", "quote", r#""well said""# => r#""> well said""#
}

wrapper! {
    /// Quotes `content` and everything after it in the message: `>>> content`.
    block_quote => BlockQuote, ">>> ", "", "blockQuote", r#""line 1\nline 2""# => r#"">>> line 1\nline 2""#
}

wrapper! {
    /// Shows `content` small and grey, as a line of subtext: `-# content`.
    subtext => Subtext, "-# ", "", "subtext", r#""edited""# => r#""-# edited""#
}

wrapper! {
    /// Links to `url` without an embed preview: `<url>`.
    hide_link_embed => HideLinkEmbed, "<", ">", "hideLinkEmbed", r#""https://example.com""# => r#""<https://example.com>""#
}

/// A code block: ```` ```language\ncontent\n``` ````. Returned by [`code_block`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct CodeBlock<'a, T> {
    content: T,
    language: Option<&'a str>,
}

/// Shows `content` as a block of code, highlighted for a [`language`](CodeBlock::language)
/// such as `rust` if set (D-213).
///
/// The content cannot contain three backticks in a row.
///
/// discord.js counterpart: `codeBlock`.
///
/// # Examples
///
/// ```
/// use djs_formatters::code_block;
///
/// assert_eq!(code_block("let x = 1;").to_string(), "```\nlet x = 1;\n```");
/// assert_eq!(code_block("let x = 1;").language("rust").to_string(), "```rust\nlet x = 1;\n```");
/// ```
pub const fn code_block<'a, T: fmt::Display>(content: T) -> CodeBlock<'a, T> {
    CodeBlock {
        content,
        language: None,
    }
}

impl<'a, T> CodeBlock<'a, T> {
    /// Sets the language the code is highlighted for (`rust`, `json`, `diff`, …).
    pub const fn language(mut self, language: &'a str) -> Self {
        self.language = Some(language);
        self
    }
}

impl<T: fmt::Display> fmt::Display for CodeBlock<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "```{}\n{}\n```",
            self.language.unwrap_or_default(),
            self.content
        )
    }
}

/// The size of a [`heading`].
///
/// discord.js counterpart: `HeadingLevel`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HeadingLevel {
    /// The largest: `# content`.
    #[default]
    One,
    /// `## content`.
    Two,
    /// The smallest: `### content`.
    Three,
}

/// A heading: `# content`, `## content` or `### content`. Returned by [`heading`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct Heading<T> {
    content: T,
    level: HeadingLevel,
}

/// Shows `content` as a heading, the largest unless a smaller
/// [`level`](Heading::level) is set (D-213).
///
/// discord.js counterpart: `heading`.
///
/// # Examples
///
/// ```
/// use djs_formatters::{HeadingLevel, heading};
///
/// assert_eq!(heading("Rules").to_string(), "# Rules");
/// assert_eq!(heading("Rules").level(HeadingLevel::Three).to_string(), "### Rules");
/// ```
pub const fn heading<T: fmt::Display>(content: T) -> Heading<T> {
    Heading {
        content,
        level: HeadingLevel::One,
    }
}

impl<T> Heading<T> {
    /// Sets the size of the heading.
    pub const fn level(mut self, level: HeadingLevel) -> Self {
        self.level = level;
        self
    }
}

impl<T: fmt::Display> fmt::Display for Heading<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marker = match self.level {
            HeadingLevel::One => "#",
            HeadingLevel::Two => "##",
            HeadingLevel::Three => "###",
        };
        write!(f, "{marker} {}", self.content)
    }
}

/// A masked link: `[content](url)` or `[content](url "title")`. Returned by [`hyperlink`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct Hyperlink<'a, T, U> {
    content: T,
    url: U,
    title: Option<&'a str>,
}

/// Shows `content` as a link to `url`, with a [`title`](Hyperlink::title) shown on hover if
/// set (D-213).
///
/// Wrap the URL in `<…>` (`hyperlink("docs", hide_link_embed(url))`) to avoid an embed
/// preview.
///
/// discord.js counterpart: `hyperlink`.
///
/// # Examples
///
/// ```
/// use djs_formatters::hyperlink;
///
/// assert_eq!(
///     hyperlink("the docs", "https://docs.rs").to_string(),
///     "[the docs](https://docs.rs)"
/// );
/// assert_eq!(
///     hyperlink("the docs", "https://docs.rs").title("Rust docs").to_string(),
///     r#"[the docs](https://docs.rs "Rust docs")"#
/// );
/// ```
pub const fn hyperlink<'a, T: fmt::Display, U: fmt::Display>(
    content: T,
    url: U,
) -> Hyperlink<'a, T, U> {
    Hyperlink {
        content,
        url,
        title: None,
    }
}

impl<'a, T, U> Hyperlink<'a, T, U> {
    /// Sets the text shown when the link is hovered.
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
}

impl<T: fmt::Display, U: fmt::Display> fmt::Display for Hyperlink<'_, T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.title {
            Some(title) => write!(f, "[{}]({} \"{title}\")", self.content, self.url),
            None => write!(f, "[{}]({})", self.content, self.url),
        }
    }
}

/// An item of a [list](unordered_list): a line of text, or a nested list indented under the
/// line before it.
///
/// A flat list takes plain [`Display`](fmt::Display) items (`&["a", "b"]`); nested lists take
/// these.
///
/// discord.js counterpart: an element of a `RecursiveArray`.
///
/// # Examples
///
/// ```
/// use djs_formatters::{ListItem, unordered_list};
///
/// let items = [
///     ListItem::Text("Fruit"),
///     ListItem::Nested(&[ListItem::Text("Apple"), ListItem::Text("Pear")]),
///     ListItem::Text("Bread"),
/// ];
/// assert_eq!(unordered_list(&items).to_string(), "- Fruit\n  - Apple\n  - Pear\n- Bread");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListItem<'a, T> {
    /// A line of the list.
    Text(T),
    /// A list nested one level deeper.
    Nested(&'a [ListItem<'a, T>]),
}

/// What a list can hold: any [`Display`](fmt::Display) value, or a [`ListItem`].
///
/// Implemented by this crate only: every `Display` type and `ListItem` already are.
pub trait ListEntry: private::WriteEntry {}

impl<T: private::WriteEntry + ?Sized> ListEntry for T {}

mod private {
    use core::fmt;

    /// How a [`ListEntry`](super::ListEntry) writes itself; private, so the trait cannot be
    /// implemented elsewhere and its method is not part of the API.
    pub trait WriteEntry {
        /// Writes the entry as the line(s) of a list at `depth` (0 for the top), numbered from
        /// `number` when the list is ordered, after a line break unless `first`. Returns the
        /// number of the next line.
        fn write_entry(
            &self,
            f: &mut fmt::Formatter<'_>,
            depth: usize,
            number: Option<u64>,
            first: &mut bool,
        ) -> Result<Option<u64>, fmt::Error>;
    }

    /// Writes one line of a list: the separator, the indentation and the marker.
    pub(super) fn write_line(
        f: &mut fmt::Formatter<'_>,
        depth: usize,
        number: Option<u64>,
        first: &mut bool,
        text: &dyn fmt::Display,
    ) -> Result<Option<u64>, fmt::Error> {
        if !*first {
            f.write_str("\n")?;
        }
        *first = false;
        for _ in 0..depth {
            f.write_str("  ")?;
        }
        if let Some(number) = number {
            write!(f, "{number}. {text}")?;
            Ok(Some(number.saturating_add(1)))
        } else {
            write!(f, "- {text}")?;
            Ok(None)
        }
    }
}

impl<T: fmt::Display + ?Sized> private::WriteEntry for T {
    fn write_entry(
        &self,
        f: &mut fmt::Formatter<'_>,
        depth: usize,
        number: Option<u64>,
        first: &mut bool,
    ) -> Result<Option<u64>, fmt::Error> {
        private::write_line(f, depth, number, first, &self)
    }
}

impl<T: fmt::Display> private::WriteEntry for ListItem<'_, T> {
    fn write_entry(
        &self,
        f: &mut fmt::Formatter<'_>,
        depth: usize,
        number: Option<u64>,
        first: &mut bool,
    ) -> Result<Option<u64>, fmt::Error> {
        match self {
            Self::Text(text) => private::write_line(f, depth, number, first, text),
            Self::Nested(items) => {
                // A nested ordered list counts from 1 again.
                let mut nested = number.map(|_| 1);
                for item in *items {
                    nested = item.write_entry(f, depth + 1, nested, first)?;
                }
                Ok(number)
            }
        }
    }
}

/// A bulleted list: one `- item` per line. Returned by [`unordered_list`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct UnorderedList<'a, T> {
    items: &'a [T],
}

/// Shows `items` as a bulleted list, one per line; [`ListItem::Nested`] indents a list
/// under the line before it (two spaces per level).
///
/// discord.js counterpart: `unorderedList`.
///
/// # Examples
///
/// ```
/// use djs_formatters::unordered_list;
///
/// assert_eq!(unordered_list(&["eggs", "milk"]).to_string(), "- eggs\n- milk");
/// ```
pub const fn unordered_list<T: ListEntry>(items: &[T]) -> UnorderedList<'_, T> {
    UnorderedList { items }
}

impl<T: ListEntry> fmt::Display for UnorderedList<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for item in self.items {
            item.write_entry(f, 0, None, &mut first)?;
        }
        Ok(())
    }
}

/// A numbered list: `1. item`, `2. item`, … Returned by [`ordered_list`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct OrderedList<'a, T> {
    items: &'a [T],
    start: u64,
}

/// Shows `items` as a numbered list from 1, or from [`start`](OrderedList::start) (D-213);
/// nested lists count from 1.
///
/// Each line carries its own number (discord.js writes the first number on every line;
/// Discord shows both the same way).
///
/// discord.js counterpart: `orderedList`.
///
/// # Examples
///
/// ```
/// use djs_formatters::ordered_list;
///
/// assert_eq!(ordered_list(&["wake up", "code"]).to_string(), "1. wake up\n2. code");
/// assert_eq!(ordered_list(&["third"]).start(3).to_string(), "3. third");
/// ```
pub const fn ordered_list<T: ListEntry>(items: &[T]) -> OrderedList<'_, T> {
    OrderedList { items, start: 1 }
}

impl<T> OrderedList<'_, T> {
    /// Sets the number of the first line (at least 1; 0 counts as 1, as in discord.js).
    pub const fn start(mut self, start: u64) -> Self {
        self.start = if start == 0 { 1 } else { start };
        self
    }
}

impl<T: ListEntry> fmt::Display for OrderedList<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut number = Some(self.start);
        for item in self.items {
            number = item.write_entry(f, 0, number, &mut first)?;
        }
        Ok(())
    }
}

/// A text face Discord offers as a slash command, ready to send.
///
/// discord.js counterpart: `Faces`.
///
/// # Examples
///
/// ```
/// use djs_formatters::Face;
///
/// assert_eq!(format!("{} oh well", Face::Shrug), r"¯\_(ツ)_/¯ oh well");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Face {
    /// `¯\_(ツ)_/¯` (the underscore escaped, so it is not read as italics).
    Shrug,
    /// `(╯°□°)╯︵ ┻━┻`.
    Tableflip,
    /// `┬─┬ノ( º _ ºノ)`.
    Unflip,
}

impl Face {
    /// The face as written in a message.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shrug => r"¯\_(ツ)_/¯",
            Self::Tableflip => "(╯°□°)╯︵ ┻━┻",
            Self::Unflip => "┬─┬ノ( º _ ºノ)",
        }
    }
}

impl fmt::Display for Face {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
