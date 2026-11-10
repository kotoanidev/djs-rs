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

/// Defines a `u64` bitflags type (D-047) that (de)serializes as a JSON integer.
///
/// Wraps [`bitflags::bitflags!`] and adds the standard derives plus `Serialize`/`Deserialize`.
/// Unknown bits are preserved in both directions (C-010), so values received from a newer API
/// version round-trip unchanged.
macro_rules! int_flags {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident: u64 {
            $(
                $(#[$($flag_attr:tt)*])*
                const $flag:ident = $value:expr;
            )*
        }
    ) => {
        ::bitflags::bitflags! {
            $(#[$meta])*
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
            $vis struct $name: u64 {
                $(
                    $(#[$($flag_attr)*])*
                    const $flag = $value;
                )*
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_u64(self.bits())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                <u64 as ::serde::Deserialize>::deserialize(deserializer).map(Self::from_bits_retain)
            }
        }
    };
}

pub(crate) use int_flags;

/// Defines a string-valued Discord enum that is forward compatible (the string counterpart of
/// [`int_enum!`], see C-010).
///
/// Generated items:
/// - the enum, marked `#[non_exhaustive]`, with an `Unknown(CompactString)` variant that keeps
///   values this version of the library does not know about;
/// - `as_str()` returning the string Discord uses, `is_unknown()`;
/// - `From<&str>`, `AsRef<str>`, `Display`;
/// - `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` based on the string, so `Unknown("x")` and
///   the variant whose value is `"x"` compare equal;
/// - `Serialize` / `Deserialize` as the bare string.
macro_rules! str_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $value:literal,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
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
            Unknown(::compact_str::CompactString),
        }

        impl $name {
            /// Returns the string Discord uses for this value.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $value,)*
                    Self::Unknown(value) => value,
                }
            }

            /// Returns `true` for a value this version of the library does not know about.
            #[must_use]
            pub const fn is_unknown(&self) -> bool {
                matches!(self, Self::Unknown(_))
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($value => Self::$variant,)*
                    other => Self::Unknown(other.into()),
                }
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl Eq for $name {}

        impl ::core::hash::Hash for $name {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                self.as_str().hash(state);
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.as_str().cmp(other.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct EnumVisitor;

                impl ::serde::de::Visitor<'_> for EnumVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.write_str(concat!("a ", stringify!($name), " string"))
                    }

                    fn visit_str<E: ::serde::de::Error>(self, value: &str) -> Result<$name, E> {
                        Ok($name::from(value))
                    }
                }

                deserializer.deserialize_str(EnumVisitor)
            }
        }
    };
}

pub(crate) use str_enum;
