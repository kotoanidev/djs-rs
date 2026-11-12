//! Escaping markdown so that text is shown as written (D-212).

use std::borrow::Cow;

/// Which kinds of markdown [`escape`](Self::escape) neutralizes.
///
/// [`new`](Self::new) (and `Default`) gives discord.js' defaults: everything except
/// headings, lists and masked links; [`none`](Self::none) starts from nothing. Each setter
/// turns one kind on or off.
///
/// # Rules
///
/// Escaping puts a backslash before the characters that would start formatting. The rules:
///
/// - **Asterisks and underscores** are escaped by run: [`bold`](EscapeOptions::bold) (`**`)
///   and [`underline`](EscapeOptions::underline) (`__`) escape a run two characters at a
///   time, [`italic`](EscapeOptions::italic) a single one — a run of one, or the first of an
///   odd run. With both (the default) every character of the run is escaped.
/// - **Tildes and bars** are escaped in pairs (`~~` for strikethrough, `||` for spoilers); a
///   single `~` or `|` formats nothing and is kept.
/// - **Backticks:** runs of one or two are escaped for [inline code](EscapeOptions::inline_code),
///   runs of three or more for [code blocks](EscapeOptions::code_block). Turning off
///   [`code_block_content`](EscapeOptions::code_block_content) or
///   [`inline_code_content`](EscapeOptions::inline_code_content) leaves the text between two
///   delimiters untouched.
/// - **Backslashes** are doubled ([`backslash`](EscapeOptions::backslash)).
/// - At the start of a line, on request: a heading's `#`, a bullet `-` or `*`, a numbered
///   list's `.`; and anywhere on request the `[` of a masked link `[text](url)`.
/// - **Links and tokens are copied as they are:** a link (`http://` or `https://`, up to the
///   next whitespace) and a `<…>` token on one line — a mention, emoji, timestamp, guild
///   navigation, e-mail, phone or hidden link: content starting with `@`, `#`, `+` or
///   containing `:` or `@` without whitespace, or a command mention `</name …:id>`. Escaping
///   them would break them.
///
/// For ordinary text the result is the same as discord.js'. In unusual input it differs on
/// purpose: every backtick of a long run is escaped (discord.js leaves the fourth of four
/// alone), every masked link of a line is escaped (discord.js only the first), text after a
/// custom emoji on the same line is still escaped (discord.js stops escaping `_` there), and
/// nothing inside a link is touched (discord.js escapes `**` and backslashes in links).
///
/// discord.js counterpart: `EscapeMarkdownOptions`.
///
/// # Examples
///
/// ```
/// use djs_formatters::EscapeOptions;
///
/// let options = EscapeOptions::new().heading(true).code_block_content(false);
/// assert_eq!(options.escape("# **hi**\n```**raw**```"), "\\# \\*\\*hi\\*\\*\n\\`\\`\\`**raw**\\`\\`\\`");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one switch per kind of markdown, as in discord.js"
)]
pub struct EscapeOptions {
    code_block: bool,
    inline_code: bool,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    spoiler: bool,
    code_block_content: bool,
    inline_code_content: bool,
    backslash: bool,
    heading: bool,
    bulleted_list: bool,
    numbered_list: bool,
    masked_link: bool,
}

impl Default for EscapeOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Defines the setters of [`EscapeOptions`].
macro_rules! setters {
    ($($(#[$meta:meta])* $field:ident,)*) => {
        $(
            $(#[$meta])*
            pub const fn $field(mut self, on: bool) -> Self {
                self.$field = on;
                self
            }
        )*
    };
}

impl EscapeOptions {
    /// discord.js' defaults: code, emphasis, strikethrough, spoilers and backslashes are
    /// escaped, also inside code; headings, lists and masked links are not.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            code_block: true,
            inline_code: true,
            bold: true,
            italic: true,
            underline: true,
            strikethrough: true,
            spoiler: true,
            code_block_content: true,
            inline_code_content: true,
            backslash: true,
            heading: false,
            bulleted_list: false,
            numbered_list: false,
            masked_link: false,
        }
    }

    /// Nothing escaped, but code content not skipped: a start for escaping a few kinds.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            code_block: false,
            inline_code: false,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            spoiler: false,
            code_block_content: true,
            inline_code_content: true,
            backslash: false,
            heading: false,
            bulleted_list: false,
            numbered_list: false,
            masked_link: false,
        }
    }

    setters! {
        /// Escapes runs of three or more backticks (code blocks). Default: on.
        #[must_use]
        code_block,
        /// Escapes runs of one or two backticks (inline code). Default: on.
        #[must_use]
        inline_code,
        /// Escapes `**`. Default: on.
        #[must_use]
        bold,
        /// Escapes a single `*` or `_`. Default: on.
        #[must_use]
        italic,
        /// Escapes `__`. Default: on.
        #[must_use]
        underline,
        /// Escapes `~~`. Default: on.
        #[must_use]
        strikethrough,
        /// Escapes `||`. Default: on.
        #[must_use]
        spoiler,
        /// Escapes the text inside code blocks too; off leaves it as written. Default: on.
        #[must_use]
        code_block_content,
        /// Escapes the text inside inline code too; off leaves it as written. Default: on.
        #[must_use]
        inline_code_content,
        /// Doubles backslashes. Default: on. (discord.js: the `escape` option.)
        #[must_use]
        backslash,
        /// Escapes the `#` of a heading at the start of a line. Default: off.
        #[must_use]
        heading,
        /// Escapes a `-` or `*` bullet at the start of a line. Default: off.
        #[must_use]
        bulleted_list,
        /// Escapes the `.` of a numbered list item (`1.`) at the start of a line. Default: off.
        #[must_use]
        numbered_list,
        /// Escapes the `[` of a masked link `[text](url)`. Default: off.
        #[must_use]
        masked_link,
    }

    /// Escapes the enabled kinds of markdown in `text`, borrowing it when nothing needs
    /// escaping. See the [type docs](Self#rules) for the rules.
    ///
    /// discord.js counterpart: `escapeMarkdown(text, options)`.
    #[must_use]
    pub fn escape<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let mut scanner = Scanner {
            options: *self,
            bytes: text.as_bytes(),
            text,
            out: None,
            copied: 0,
            line: Line::default(),
        };
        scanner.run();
        scanner.finish()
    }
}

/// Escapes `text` with discord.js' defaults ([`EscapeOptions::new`]): emphasis, code,
/// strikethrough, spoilers and backslashes, outside links and mentions.
///
/// discord.js counterpart: `escapeMarkdown`.
///
/// # Examples
///
/// ```
/// use djs_formatters::escape_markdown;
///
/// assert_eq!(escape_markdown("**not bold** and `code`"), r"\*\*not bold\*\* and \`code\`");
/// // Links and mentions keep working:
/// assert_eq!(escape_markdown("see https://x.dev/a_b_c by <@1>"), "see https://x.dev/a_b_c by <@1>");
/// // Nothing to escape: the text is borrowed.
/// assert!(matches!(escape_markdown("plain"), std::borrow::Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn escape_markdown(text: &str) -> Cow<'_, str> {
    EscapeOptions::new().escape(text)
}

/// Defines the single-kind escapers.
macro_rules! escapers {
    ($($function:ident => $option:ident, $js:literal, $what:literal, $input:literal => $output:literal;)*) => {
        $(
            #[doc = concat!("Escapes only ", $what, " in `text` ([`EscapeOptions::", stringify!($option), "`]).")]
            ///
            #[doc = concat!("discord.js counterpart: `", $js, "`.")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!("assert_eq!(djs_formatters::", stringify!($function), "(", $input, "), ", $output, ");")]
            /// ```
            #[must_use]
            pub fn $function(text: &str) -> Cow<'_, str> {
                EscapeOptions::none().$option(true).escape(text)
            }
        )*
    };
}

escapers! {
    escape_code_block => code_block, "escapeCodeBlock", "runs of three or more backticks",
        r#""```rust```""# => r#"r"\`\`\`rust\`\`\`""#;
    escape_inline_code => inline_code, "escapeInlineCode", "runs of one or two backticks",
        r#""`a` ``b``""# => r#"r"\`a\` \`\`b\`\`""#;
    escape_bold => bold, "escapeBold", "`**`", r#""**a**""# => r#"r"\*\*a\*\*""#;
    escape_italic => italic, "escapeItalic", "a single `*` or `_`",
        r#""*a* _b_""# => r#"r"\*a\* \_b\_""#;
    escape_underline => underline, "escapeUnderline", "`__`", r#""__a__""# => r#"r"\_\_a\_\_""#;
    escape_strikethrough => strikethrough, "escapeStrikethrough", "`~~`",
        r#""~~a~~ ~b""# => r#"r"\~\~a\~\~ ~b""#;
    escape_spoiler => spoiler, "escapeSpoiler", "`||`", r#""||a||""# => r#"r"\|\|a\|\|""#;
    escape_escape => backslash, "escapeEscape", "backslashes", r#"r"a\b""# => r#"r"a\\b""#;
    escape_heading => heading, "escapeHeading", "headings at the start of a line",
        r###""## Title""### => r###"r"\## Title""###;
    escape_bulleted_list => bulleted_list, "escapeBulletedList",
        "bullets at the start of a line", r#""- item""# => r#"r"\- item""#;
    escape_numbered_list => numbered_list, "escapeNumberedList",
        "numbered list items at the start of a line", r#""1. item""# => r#"r"1\. item""#;
    escape_masked_link => masked_link, "escapeMaskedLink", "masked links",
        r#""[a](https://b.c)""# => r#"r"\[a](https://b.c)""#;
}

/// No position: an unset mark.
const NONE: usize = usize::MAX;

/// What the line being scanned needs escaped apart from single characters.
#[derive(Clone, Copy, Debug, Default)]
struct Line {
    /// The index after the line's `\n` (or the text's end).
    end: usize,
    /// Positions to escape at the start of the line: a bullet, a heading's `#`, a numbered
    /// item's `.`.
    marks: [usize; 3],
    /// The last `](` of the line followed by a URL and `)`: a `[` at least two bytes before
    /// it starts a masked link.
    link_middle: usize,
}

/// One pass over the text, copying it lazily into `out` from the first escape on.
struct Scanner<'a> {
    options: EscapeOptions,
    text: &'a str,
    bytes: &'a [u8],
    out: Option<String>,
    copied: usize,
    line: Line,
}

/// Whether `byte` is whitespace for links and tokens.
const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

impl<'a> Scanner<'a> {
    /// Puts a backslash before the byte at `index` (an ASCII character, so a char boundary);
    /// indexes come in increasing order.
    fn escape_at(&mut self, index: usize) {
        let out = self
            .out
            .get_or_insert_with(|| String::with_capacity(self.text.len() + 16));
        out.push_str(&self.text[self.copied..index]);
        out.push('\\');
        self.copied = index;
    }

    fn finish(self) -> Cow<'a, str> {
        match self.out {
            None => Cow::Borrowed(self.text),
            Some(mut out) => {
                out.push_str(&self.text[self.copied..]);
                Cow::Owned(out)
            }
        }
    }

    /// The length of the run of `byte` starting at `start`.
    fn run_length(&self, start: usize, byte: u8) -> usize {
        self.bytes[start..]
            .iter()
            .position(|&other| other != byte)
            .unwrap_or(self.bytes.len() - start)
    }

    fn is_mark(&self, index: usize) -> bool {
        self.line.marks.contains(&index)
    }

    fn run(&mut self) {
        let len = self.bytes.len();
        let mut index = 0;
        while index < len {
            if index >= self.line.end {
                self.load_line(index);
            }
            index = match self.bytes[index] {
                b'h' | b'H' => self.link(index),
                b'<' => self.token(index),
                b'\\' => {
                    if self.options.backslash {
                        self.escape_at(index);
                    }
                    index + 1
                }
                b'*' => self.emphasis(index, b'*', self.options.bold),
                b'_' => self.emphasis(index, b'_', self.options.underline),
                b'~' => self.pairs(index, b'~', self.options.strikethrough),
                b'|' => self.pairs(index, b'|', self.options.spoiler),
                b'`' => self.backticks(index),
                b'[' => {
                    let middle = self.line.link_middle;
                    if self.options.masked_link && middle != NONE && middle >= index + 2 {
                        self.escape_at(index);
                    }
                    index + 1
                }
                _ => {
                    if self.is_mark(index) {
                        self.escape_at(index);
                    }
                    index + 1
                }
            };
        }
    }

    /// Prepares the line containing `index`: where it ends, and what its start and masked
    /// links need escaped.
    fn load_line(&mut self, index: usize) {
        let bytes = self.bytes;
        let previous_end = self.line.end.min(index);
        let start = bytes[previous_end..index]
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(previous_end, |offset| previous_end + offset + 1);
        let end = bytes[index..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(bytes.len(), |offset| index + offset + 1);
        let content_end = if end > start && bytes[end - 1] == b'\n' {
            end - 1
        } else {
            end
        };
        let line = &bytes[start..content_end];
        let mut marks = [NONE; 3];
        let options = self.options;
        if options.bulleted_list || options.heading || options.numbered_list {
            let spaces = line.iter().take_while(|&&byte| byte == b' ').count();
            // `- item` / `* item`.
            if options.bulleted_list
                && matches!(line.get(spaces), Some(b'-' | b'*'))
                && line.get(spaces + 1) == Some(&b' ')
            {
                marks[0] = start + spaces;
            }
            // `# Title`, `## Title`, `### Title`, also after up to two spaces and a bullet.
            if options.heading {
                let mut at = spaces;
                if spaces <= 2
                    && matches!(line.get(at), Some(b'-' | b'*'))
                    && line.get(at + 1) == Some(&b' ')
                {
                    at += 2;
                    at += line[at..].iter().take_while(|&&byte| byte == b' ').count();
                }
                let hashes = line[at..].iter().take_while(|&&byte| byte == b'#').count();
                if (1..=3).contains(&hashes) && line.get(at + hashes) == Some(&b' ') {
                    marks[1] = start + at;
                }
            }
            // `1. item`.
            if options.numbered_list {
                let digits = line[spaces..]
                    .iter()
                    .take_while(|byte| byte.is_ascii_digit())
                    .count();
                if digits > 0 && line.get(spaces + digits) == Some(&b'.') {
                    marks[2] = start + spaces + digits;
                }
            }
        }
        let mut link_middle = NONE;
        if options.masked_link
            && let Some(close) = line.iter().rposition(|&byte| byte == b')')
            && close >= 3
        {
            // The last `](` with at least one byte of URL before the `)`.
            link_middle = line[..close - 1]
                .windows(2)
                .rposition(|pair| pair == b"](")
                .map_or(NONE, |middle| start + middle);
        }
        self.line = Line {
            end,
            marks,
            link_middle,
        };
    }

    /// Copies a link (`http://` or `https://` up to the next whitespace) as it is.
    fn link(&self, index: usize) -> usize {
        let rest = &self.bytes[index..];
        let scheme = if rest.len() > 8 && rest[..8].eq_ignore_ascii_case(b"https://") {
            8
        } else if rest.len() > 7 && rest[..7].eq_ignore_ascii_case(b"http://") {
            7
        } else {
            return index + 1;
        };
        index
            + scheme
            + rest[scheme..]
                .iter()
                .position(|&byte| is_space(byte))
                .unwrap_or(rest.len() - scheme)
    }

    /// Copies a `<…>` token (see the module docs) as it is.
    fn token(&self, index: usize) -> usize {
        let rest = &self.bytes[index + 1..];
        let Some(close) = rest
            .iter()
            .position(|&byte| matches!(byte, b'<' | b'>' | b'\n'))
        else {
            return index + 1;
        };
        if rest[close] != b'>' || close == 0 {
            return index + 1;
        }
        let content = &rest[..close];
        let protected = if content[0] == b'/' {
            // `</name sub:id>`: a colon followed by digits at the end.
            let digits = content
                .iter()
                .rev()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            digits > 0 && content.len() > digits + 1 && content[content.len() - digits - 1] == b':'
        } else {
            !content.iter().any(|&byte| is_space(byte))
                && (matches!(content[0], b'@' | b'#' | b'+')
                    || content.iter().any(|&byte| matches!(byte, b':' | b'@')))
        };
        if protected {
            index + close + 2
        } else {
            index + 1
        }
    }

    /// Escapes a run of `*` or `_`: in pairs for bold / underline (`pairs`), a single one for
    /// italics, or all of it with both.
    fn emphasis(&mut self, index: usize, byte: u8, pairs: bool) -> usize {
        let length = self.run_length(index, byte);
        let single = self.options.italic;
        for offset in 0..length {
            let escaped = match (pairs, single) {
                (true, true) => true,
                (true, false) => offset < length / 2 * 2,
                (false, true) => length % 2 == 1 && offset == 0,
                (false, false) => false,
            };
            if escaped || self.is_mark(index + offset) {
                self.escape_at(index + offset);
            }
        }
        index + length
    }

    /// Escapes a run of `~` or `|` in pairs.
    fn pairs(&mut self, index: usize, byte: u8, on: bool) -> usize {
        let length = self.run_length(index, byte);
        if on {
            for offset in 0..length / 2 * 2 {
                self.escape_at(index + offset);
            }
        }
        index + length
    }

    /// Escapes `count` bytes from `index`.
    fn escape_all(&mut self, index: usize, count: usize) {
        for offset in 0..count {
            self.escape_at(index + offset);
        }
    }

    /// Handles a run of backticks: code delimiters, possibly skipping the code between two.
    fn backticks(&mut self, index: usize) -> usize {
        let length = self.run_length(index, b'`');
        let options = self.options;
        if length >= 3 && !options.code_block_content {
            let after = index + 3;
            let close = self.bytes[after..]
                .windows(3)
                .position(|window| window == b"```")
                .map(|offset| after + offset);
            if options.code_block {
                self.escape_all(index, 3);
            }
            let Some(close) = close else {
                return after;
            };
            if options.code_block {
                self.escape_all(close, 3);
            }
            return close + 3;
        }
        if length == 1 && !options.inline_code_content {
            let close = self.single_backtick(index + 1);
            if options.inline_code {
                self.escape_at(index);
            }
            let Some(close) = close else {
                return index + 1;
            };
            if options.inline_code {
                self.escape_at(close);
            }
            return close + 1;
        }
        let on = if length >= 3 {
            options.code_block
        } else {
            options.inline_code
        };
        if on {
            self.escape_all(index, length);
        }
        index + length
    }

    /// The next backtick standing alone from `from`, not past a code block when code block
    /// content is skipped.
    fn single_backtick(&self, from: usize) -> Option<usize> {
        let mut index = from;
        while let Some(offset) = self.bytes[index..].iter().position(|&byte| byte == b'`') {
            let start = index + offset;
            let length = self.run_length(start, b'`');
            if length == 1 {
                return Some(start);
            }
            if length >= 3 && !self.options.code_block_content {
                return None;
            }
            index = start + length;
        }
        None
    }
}
