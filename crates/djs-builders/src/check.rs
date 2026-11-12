//! The checks shared by every builder: text lengths and URL schemes.

use std::borrow::Cow;

use crate::ValidationError;

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

/// Returns [`ValidationError::TooMany`] if a list of `actual` items is above `max`.
pub(crate) fn count(field: &'static str, actual: usize, max: usize) -> Result<(), ValidationError> {
    if actual > max {
        return Err(ValidationError::TooMany {
            field: field.into(),
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
