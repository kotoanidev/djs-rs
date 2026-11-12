//! The checks shared by every builder: text lengths, URL schemes, and the [`Path`] of a value
//! for error messages.

use std::{borrow::Cow, fmt};

use crate::ValidationError;

/// Where a value sits in the JSON sent to Discord, for the `field` of a [`ValidationError`].
///
/// Built on the stack while walking nested values (`options[2].choices[0]`) and formatted only
/// when an error is reported, so valid input costs no allocation.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Path<'a> {
    /// The value being built.
    Root,
    /// Item `index` of the list `list` inside `parent`.
    Item {
        parent: &'a Path<'a>,
        list: &'static str,
        index: usize,
    },
}

impl<'a> Path<'a> {
    /// The path of item `index` of the list `list` here.
    pub(crate) const fn item(&'a self, list: &'static str, index: usize) -> Self {
        Self::Item {
            parent: self,
            list,
            index,
        }
    }

    /// The path of the field `name` here (borrowed when here is the root).
    pub(crate) fn field(&self, name: &'static str) -> Cow<'static, str> {
        match self {
            Self::Root => name.into(),
            Self::Item { .. } => format!("{self}.{name}").into(),
        }
    }

    /// The path of the entry `key` of the map `map` here, e.g. `name_localizations.pl`.
    pub(crate) fn entry(&self, map: &'static str, key: &str) -> Cow<'static, str> {
        match self {
            Self::Root => format!("{map}.{key}").into(),
            Self::Item { .. } => format!("{self}.{map}.{key}").into(),
        }
    }

    /// This path itself, e.g. `options[1]`.
    pub(crate) fn this(&self) -> Cow<'static, str> {
        self.to_string().into()
    }
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => Ok(()),
            Self::Item {
                parent: Self::Root,
                list,
                index,
            } => write!(f, "{list}[{index}]"),
            Self::Item {
                parent,
                list,
                index,
            } => write!(f, "{parent}.{list}[{index}]"),
        }
    }
}

/// The schemes Discord allows for links: the title and author URLs (D-187).
pub(crate) const WEB: &[&str] = &["http://", "https://"];

/// The schemes allowed for images and icons: links or files uploaded with the message
/// (D-187, D-190).
pub(crate) const WEB_OR_ATTACHMENT: &[&str] = &["http://", "https://", "attachment://"];

/// The length Discord counts for `text`: its Unicode code points without leading and trailing
/// whitespace (D-183; Discord trims Unicode whitespace such as NBSP too, D-188, like
/// [`str::trim`]).
pub(crate) fn length(text: &str) -> usize {
    text.trim().chars().count()
}

/// Whether `text` is empty once trimmed — what Discord treats as no text at all (D-191).
pub(crate) fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// Returns the [`length`] of `text`, or [`ValidationError::TooLong`] if it is above `max`.
///
/// `field` is only called on an error, so a formatted path costs nothing when the text is
/// valid.
pub(crate) fn text(
    field: impl FnOnce() -> Cow<'static, str>,
    text: &str,
    max: usize,
) -> Result<usize, ValidationError> {
    let actual = length(text);
    if actual > max {
        return Err(ValidationError::TooLong {
            field: field(),
            max,
            actual,
        });
    }
    Ok(actual)
}

/// Like [`text`], but a blank text is [`ValidationError::Missing`]: Discord requires it to
/// have at least one character after trimming (D-195).
pub(crate) fn required_text(
    field: impl FnOnce() -> Cow<'static, str>,
    text: &str,
    max: usize,
) -> Result<usize, ValidationError> {
    let actual = length(text);
    if actual == 0 {
        return Err(ValidationError::Missing { field: field() });
    }
    if actual > max {
        return Err(ValidationError::TooLong {
            field: field(),
            max,
            actual,
        });
    }
    Ok(actual)
}

/// Returns [`ValidationError::TooMany`] if a list of `actual` items is above `max`.
pub(crate) fn count(
    field: impl FnOnce() -> Cow<'static, str>,
    actual: usize,
    max: usize,
) -> Result<(), ValidationError> {
    if actual > max {
        return Err(ValidationError::TooMany {
            field: field(),
            max,
            actual,
        });
    }
    Ok(())
}

/// Returns [`ValidationError::InvalidUrl`] unless `url` starts with one of `allowed`,
/// ignoring ASCII case (Discord accepts `HTTPS://`, D-190).
pub(crate) fn url(
    field: &'static str,
    url: &str,
    allowed: &'static [&'static str],
) -> Result<(), ValidationError> {
    let has_scheme = |scheme: &&str| {
        url.as_bytes()
            .get(..scheme.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(scheme.as_bytes()))
    };
    if allowed.iter().any(has_scheme) {
        Ok(())
    } else {
        Err(ValidationError::InvalidUrl {
            field: field.into(),
            allowed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_counts_code_points_after_trimming() {
        assert_eq!(length(""), 0);
        assert_eq!(length("  \n\t "), 0);
        assert_eq!(length("abc"), 3);
        assert_eq!(length("  abc  "), 3);
        assert_eq!(length("a b"), 3);
        // U+1F525: one code point, two UTF-16 units, four bytes.
        assert_eq!(length("🔥🔥"), 2);
        // A family emoji: one grapheme, seven code points.
        assert_eq!(length("👨‍👩‍👧‍👦"), 7);
        // NBSP is Unicode whitespace and trimmed; a zero-width space is not.
        assert_eq!(length("\u{a0}x\u{a0}"), 1);
        assert_eq!(length("\u{200b}"), 1);
    }

    #[test]
    fn text_reports_the_path_only_on_errors() {
        assert_eq!(text(|| unreachable!(), "abc", 3), Ok(3));
        assert_eq!(
            text(|| "title".into(), "abcd", 3),
            Err(ValidationError::TooLong {
                field: "title".into(),
                max: 3,
                actual: 4
            })
        );
    }

    #[test]
    fn paths_are_written_like_discord_writes_them() {
        let root = Path::Root;
        assert_eq!(root.field("name"), "name");
        assert!(matches!(root.field("name"), Cow::Borrowed(_)));
        assert_eq!(
            root.entry("name_localizations", "pl"),
            "name_localizations.pl"
        );
        let option = root.item("options", 2);
        let choice = option.item("choices", 0);
        assert_eq!(option.this(), "options[2]");
        assert_eq!(choice.field("name"), "options[2].choices[0].name");
        assert_eq!(
            choice.entry("name_localizations", "de"),
            "options[2].choices[0].name_localizations.de"
        );
    }

    #[test]
    fn a_required_text_must_not_be_blank() {
        assert_eq!(required_text(|| unreachable!(), " a ", 1), Ok(1));
        assert_eq!(
            required_text(|| "description".into(), "   ", 100),
            Err(ValidationError::Missing {
                field: "description".into()
            })
        );
        assert!(matches!(
            required_text(|| "description".into(), "ab", 1),
            Err(ValidationError::TooLong { .. })
        ));
    }

    #[test]
    fn url_checks_the_scheme_without_regard_to_case() {
        assert!(url("url", "https://example.com", WEB).is_ok());
        assert!(url("url", "HTTP://example.com", WEB).is_ok());
        assert!(url("url", "attachment://a.png", WEB).is_err());
        assert!(url("url", "attachment://a.png", WEB_OR_ATTACHMENT).is_ok());
        assert!(url("url", "ftp://example.com", WEB_OR_ATTACHMENT).is_err());
        assert!(url("url", "http:/", WEB).is_err());
        assert!(url("url", "", WEB).is_err());
        // Multi-byte text where the scheme would end must not panic.
        assert!(url("url", "ħttps://", WEB).is_err());
    }
}
