//! [`ValidationError`]: what `build()` returns when a Discord limit or rule is broken.

use std::{borrow::Cow, fmt};

/// A Discord limit or rule broken by a builder's content, returned by `build()` (D-179).
///
/// `build()` stops at the first violation it finds and returns it (D-182). `field` is the
/// path of the offending value in the JSON sent to Discord, written the way Discord writes it
/// in its own `50035 Invalid Form Body` errors — `title`, `fields[3].name`, `author.url` — so
/// both kinds of error point at the same place.
///
/// Texts are measured like Discord measures them: in Unicode code points
/// ([`str::chars`]), without leading and trailing whitespace (D-183, confirmed on the live
/// API). An emoji such as 🔥 counts once — discord.js would count it twice.
///
/// discord.js counterpart: the errors thrown by the setters of `@discordjs/builders`
/// (`ValidationError` / `CombinedError` from `@sapphire/shapeshift`).
///
/// # Examples
///
/// ```
/// use djs_builders::{EmbedBuilder, ValidationError};
///
/// let error = EmbedBuilder::new().title("x".repeat(300)).build().unwrap_err();
/// assert_eq!(
///     error,
///     ValidationError::TooLong { field: "title".into(), max: 256, actual: 300 }
/// );
/// assert_eq!(error.field(), "title");
/// assert_eq!(error.to_string(), "title is too long: 300 characters, at most 256 allowed");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationError {
    /// A text is longer than Discord allows.
    TooLong {
        /// The path of the text, e.g. `description` or `fields[2].value`.
        field: Cow<'static, str>,
        /// The most characters (code points after trimming) Discord allows.
        max: usize,
        /// The characters the text has, counted the same way.
        actual: usize,
    },
    /// A list has more items than Discord allows.
    TooMany {
        /// The path of the list, e.g. `fields`.
        field: Cow<'static, str>,
        /// The most items Discord allows.
        max: usize,
        /// The items the list has.
        actual: usize,
    },
    /// A value that Discord requires is not set.
    Missing {
        /// The path of the value.
        field: Cow<'static, str>,
    },
    /// A number is outside the range Discord allows.
    OutOfRange {
        /// The path of the number, e.g. `color`.
        field: Cow<'static, str>,
        /// The smallest value allowed.
        min: i64,
        /// The largest value allowed.
        max: i64,
        /// The value given.
        actual: i64,
    },
    /// A URL does not start with one of the schemes Discord allows for it (D-187, D-190).
    ///
    /// Only the scheme is checked; Discord itself rejects a malformed address such as a bare
    /// `https://`.
    InvalidUrl {
        /// The path of the URL, e.g. `author.url`.
        field: Cow<'static, str>,
        /// The allowed scheme prefixes, e.g. `["http://", "https://"]` (compared without
        /// regard to case).
        allowed: &'static [&'static str],
    },
    /// A command, subcommand or option name has a character Discord does not allow (D-194):
    /// anything but letters, numbers, `-`, `_` and `ʼ`, or a letter that has a lower-case
    /// form.
    InvalidName {
        /// The path of the name, e.g. `options[0].name`.
        field: Cow<'static, str>,
        /// The first character that is not allowed.
        character: char,
    },
    /// Values that are valid one by one do not work together — for example a required option
    /// after an optional one, or `choices` together with `autocomplete`.
    Invalid {
        /// The path of the value that breaks the rule, e.g. `options[1].required`.
        field: Cow<'static, str>,
        /// What is wrong, for the error message.
        reason: &'static str,
    },
    /// A value has no content at all, which Discord rejects — for example an embed with
    /// nothing but a color.
    Empty {
        /// The path of the value, e.g. `embed`.
        field: Cow<'static, str>,
        /// What the value needs at least one of, for the error message.
        needs: &'static str,
    },
}

impl ValidationError {
    /// Returns the path of the value that breaks the rule, e.g. `fields[3].name`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_builders::EmbedBuilder;
    ///
    /// let error = EmbedBuilder::new().field("ok", "y".repeat(2000)).build().unwrap_err();
    /// assert_eq!(error.field(), "fields[0].value");
    /// ```
    #[must_use]
    pub fn field(&self) -> &str {
        match self {
            Self::TooLong { field, .. }
            | Self::TooMany { field, .. }
            | Self::Missing { field }
            | Self::OutOfRange { field, .. }
            | Self::InvalidUrl { field, .. }
            | Self::InvalidName { field, .. }
            | Self::Invalid { field, .. }
            | Self::Empty { field, .. } => field,
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { field, max, actual } => write!(
                f,
                "{field} is too long: {actual} characters, at most {max} allowed"
            ),
            Self::TooMany { field, max, actual } => write!(
                f,
                "{field} has too many items: {actual}, at most {max} allowed"
            ),
            Self::Missing { field } => write!(f, "{field} is required"),
            Self::OutOfRange {
                field,
                min,
                max,
                actual,
            } => write!(
                f,
                "{field} is out of range: {actual} is not between {min} and {max}"
            ),
            Self::InvalidUrl { field, allowed } => {
                write!(f, "{field} must start with ")?;
                for (index, scheme) in allowed.iter().enumerate() {
                    let separator = match index {
                        0 => "",
                        _ if index + 1 == allowed.len() => " or ",
                        _ => ", ",
                    };
                    write!(f, "{separator}{scheme}")?;
                }
                Ok(())
            }
            Self::InvalidName { field, character } => {
                let mut lower = character.to_lowercase();
                if lower.len() == 1 && lower.next() == Some(*character) {
                    write!(f, "{field} cannot contain {character:?}")
                } else {
                    write!(
                        f,
                        "{field} must be lower case: use '{}' instead of '{character}'",
                        character.to_lowercase()
                    )
                }
            }
            Self::Invalid { field, reason } => write!(f, "{field} is invalid: {reason}"),
            Self::Empty { field, needs } => {
                write!(f, "{field} has no content: it needs {needs}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::ValidationError;

    #[test]
    fn every_variant_has_a_readable_message() {
        let cases = [
            (
                ValidationError::TooMany {
                    field: "fields".into(),
                    max: 25,
                    actual: 26,
                },
                "fields has too many items: 26, at most 25 allowed",
            ),
            (
                ValidationError::Missing {
                    field: "name".into(),
                },
                "name is required",
            ),
            (
                ValidationError::OutOfRange {
                    field: "color".into(),
                    min: 0,
                    max: 0xFF_FFFF,
                    actual: 0x100_0000,
                },
                "color is out of range: 16777216 is not between 0 and 16777215",
            ),
            (
                ValidationError::InvalidUrl {
                    field: "url".into(),
                    allowed: &["http://", "https://"],
                },
                "url must start with http:// or https://",
            ),
            (
                ValidationError::InvalidUrl {
                    field: "image.url".into(),
                    allowed: &["http://", "https://", "attachment://"],
                },
                "image.url must start with http://, https:// or attachment://",
            ),
            (
                ValidationError::InvalidName {
                    field: "name".into(),
                    character: 'Ż',
                },
                "name must be lower case: use 'ż' instead of 'Ż'",
            ),
            (
                ValidationError::InvalidName {
                    field: "options[0].name".into(),
                    character: ' ',
                },
                "options[0].name cannot contain ' '",
            ),
            (
                ValidationError::Invalid {
                    field: "options[1].required".into(),
                    reason: "required options must come before optional ones",
                },
                "options[1].required is invalid: required options must come before optional ones",
            ),
            (
                ValidationError::Empty {
                    field: "embed".into(),
                    needs: "a title",
                },
                "embed has no content: it needs a title",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
            assert!(message.starts_with(error.field()));
        }
    }
}
