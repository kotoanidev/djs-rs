//! [`Patch`], the value of a request field that can be left unchanged, cleared or set.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A field of a modify request that can be left unchanged, cleared or set (D-083, D-101).
///
/// Discord's `PATCH` endpoints distinguish three cases for nullable fields: a field that is
/// left out keeps its current value, `null` clears it, and a value sets it. `Patch` names
/// these cases after what they do:
///
/// | Variant | Sent as | Effect |
/// |---|---|---|
/// | [`Keep`](Self::Keep) (default) | *(field left out)* | the value stays as it is |
/// | [`Clear`](Self::Clear) | `null` | the value is removed or reset |
/// | [`Set`](Self::Set) | the value | the value is replaced |
///
/// Request bodies skip `Keep` fields with `#[serde(skip_serializing_if = "Patch::is_keep")]`.
/// Serializing a `Keep` on its own is an error, because no JSON value means "leave out".
///
/// discord.js counterpart: passing `undefined`, `null` or a value in the options of an edit
/// method.
///
/// # Examples
///
/// ```
/// use djs_model::util::Patch;
///
/// let mut nickname: Patch<String> = Patch::default();
/// assert!(nickname.is_keep());
///
/// nickname = "Ferris".to_owned().into();
/// assert_eq!(nickname, Patch::Set("Ferris".to_owned()));
///
/// // `None` clears, `Some` sets.
/// assert_eq!(Patch::<u32>::from(None), Patch::Clear);
/// assert_eq!(Patch::from(Some(5)), Patch::Set(5));
///
/// assert_eq!(serde_json::to_string(&Patch::<u32>::Clear).unwrap(), "null");
/// assert_eq!(serde_json::to_string(&Patch::Set(5)).unwrap(), "5");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Patch<T> {
    /// Leave the current value unchanged: the field is not sent.
    #[default]
    Keep,
    /// Clear the value: the field is sent as `null`.
    Clear,
    /// Set the value.
    Set(T),
}

impl<T> Patch<T> {
    /// Returns `true` for [`Keep`](Self::Keep) (used to skip the field when serializing).
    #[must_use]
    pub const fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }

    /// Returns `true` for [`Clear`](Self::Clear).
    #[must_use]
    pub const fn is_clear(&self) -> bool {
        matches!(self, Self::Clear)
    }

    /// Returns `true` for [`Set`](Self::Set).
    #[must_use]
    pub const fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    /// Returns the value of [`Set`](Self::Set), if any.
    #[must_use]
    pub fn into_set(self) -> Option<T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Keep | Self::Clear => None,
        }
    }

    /// Borrows the value: `Patch<&T>`.
    #[must_use]
    pub const fn as_ref(&self) -> Patch<&T> {
        match self {
            Self::Keep => Patch::Keep,
            Self::Clear => Patch::Clear,
            Self::Set(value) => Patch::Set(value),
        }
    }

    /// Converts the value of [`Set`](Self::Set) with `f`, keeping `Keep` and `Clear`.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Patch<U> {
        match self {
            Self::Keep => Patch::Keep,
            Self::Clear => Patch::Clear,
            Self::Set(value) => Patch::Set(f(value)),
        }
    }
}

impl<T> From<T> for Patch<T> {
    /// Sets the value.
    fn from(value: T) -> Self {
        Self::Set(value)
    }
}

impl<T> From<Option<T>> for Patch<T> {
    /// `Some` sets the value, `None` clears it.
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Clear, Self::Set)
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Set(value) => value.serialize(serializer),
            Self::Clear => serializer.serialize_none(),
            Self::Keep => Err(serde::ser::Error::custom(
                "`Patch::Keep` has no JSON value; skip the field with \
                 `#[serde(skip_serializing_if = \"Patch::is_keep\")]`",
            )),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    /// `null` is [`Clear`](Self::Clear) and a value is [`Set`](Self::Set); a missing field
    /// becomes [`Keep`](Self::Keep) with `#[serde(default)]`.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Body {
        #[serde(default, skip_serializing_if = "Patch::is_keep")]
        nick: Patch<String>,
        #[serde(default, skip_serializing_if = "Patch::is_keep")]
        channel_id: Patch<u64>,
    }

    #[test]
    fn fields_are_left_out_cleared_or_set() {
        let body = Body {
            nick: Patch::Set("Ferris".into()),
            channel_id: Patch::Clear,
        };
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, r#"{"nick":"Ferris","channel_id":null}"#);
        assert_eq!(serde_json::from_str::<Body>(&json).unwrap(), body);

        let unchanged = Body {
            nick: Patch::Keep,
            channel_id: Patch::Keep,
        };
        assert_eq!(serde_json::to_string(&unchanged).unwrap(), "{}");
        assert_eq!(serde_json::from_str::<Body>("{}").unwrap(), unchanged);
    }

    #[test]
    fn keep_alone_cannot_be_serialized() {
        let error = serde_json::to_string(&Patch::<u8>::Keep).unwrap_err();
        assert!(error.to_string().contains("skip_serializing_if"), "{error}");
    }

    #[test]
    fn helpers() {
        let set = Patch::Set(2);
        assert!(set.is_set() && !set.is_keep() && !set.is_clear());
        assert_eq!(set.map(|value| value * 2), Patch::Set(4));
        assert_eq!(set.as_ref(), Patch::Set(&2));
        assert_eq!(set.into_set(), Some(2));
        assert_eq!(Patch::<u8>::Clear.into_set(), None);
        assert_eq!(Patch::<u8>::Clear.map(u16::from), Patch::Clear);
        assert_eq!(Patch::<u8>::Keep.map(u16::from), Patch::Keep);
    }
}
