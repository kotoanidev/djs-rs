//! Internal macros shared by the model types.

/// Defines an integer-backed Discord enum that is forward compatible.
///
/// Discord regularly introduces new values for enumerations such as message types or opcodes.
/// The generated enum therefore always has an `Unknown(repr)` variant that preserves values this
/// version of the library does not know about, instead of failing deserialization.
///
/// Generated items:
/// - the enum itself, marked `#[non_exhaustive]`;
/// - `value()` returning the raw integer;
/// - `From<repr>` / `From<Enum> for repr`;
/// - `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` based on the raw value, so
///   `Unknown(0)` and the variant whose value is `0` compare equal;
/// - `Serialize` / `Deserialize` as the bare integer.
macro_rules! int_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident: $repr:ty {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $value:literal,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug)]
        #[non_exhaustive]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )*
            /// A value that this version of the library does not know about.
            ///
            /// Discord may add new values at any time. They are preserved here so that no
            /// information is lost and deserialization never fails because of them. Prefer
            #[doc = concat!("[`", stringify!($name), "::from`] over constructing this variant directly.")]
            Unknown($repr),
        }

        impl $name {
            /// Returns the raw integer value Discord uses for this variant.
            #[must_use]
            pub const fn value(self) -> $repr {
                match self {
                    $(Self::$variant => $value,)*
                    Self::Unknown(value) => value,
                }
            }

            /// Returns `true` if this is a value the library does not know about.
            #[must_use]
            pub const fn is_unknown(self) -> bool {
                matches!(Self::from_value(self.value()), Self::Unknown(_))
            }

            /// Converts a raw value into the matching variant, or [`Self::Unknown`].
            #[must_use]
            pub const fn from_value(value: $repr) -> Self {
                match value {
                    $($value => Self::$variant,)*
                    other => Self::Unknown(other),
                }
            }
        }

        impl From<$repr> for $name {
            fn from(value: $repr) -> Self {
                Self::from_value(value)
            }
        }

        impl From<$name> for $repr {
            fn from(value: $name) -> Self {
                value.value()
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.value() == other.value()
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.value().cmp(&other.value())
            }
        }

        impl core::hash::Hash for $name {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.value().hash(state);
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                <$repr as serde::Serialize>::serialize(&self.value(), serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                <$repr as serde::Deserialize>::deserialize(deserializer).map(Self::from_value)
            }
        }
    };
}

pub(crate) use int_enum;
