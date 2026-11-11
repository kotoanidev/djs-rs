//! Small serde helpers shared by the model types.

/// `#[serde(default = "crate::serde_util::default_true")]` for booleans that are `true` when
/// absent (D-055).
pub(crate) const fn default_true() -> bool {
    true
}

/// (De)serializes `Option<flags>` as a decimal string (e.g. `Application::flags_new`).
pub(crate) mod option_flags_string {
    use bitflags::Flags;
    use compact_str::CompactString;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    #[expect(clippy::ref_option, reason = "signature required by serde(with)")]
    pub(crate) fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Flags<Bits = u64>,
        S: Serializer,
    {
        match value {
            Some(flags) => serializer.collect_str(&flags.bits()),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: Flags<Bits = u64>,
        D: Deserializer<'de>,
    {
        Option::<CompactString>::deserialize(deserializer)?
            .map(|text| {
                text.parse()
                    .map(T::from_bits_retain)
                    .map_err(|_| D::Error::custom("expected flags as a decimal string"))
            })
            .transpose()
    }
}

/// `#[serde(default, deserialize_with = "crate::serde_util::null_as_default")]`: reads an
/// explicit `null` like an absent field (e.g. nullable localization dictionaries).
pub(crate) fn null_as_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    <Option<T> as serde::Deserialize>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// A placeholder ID of a guild template or "create guild" body: an integer (or, as Discord also
/// accepts, a numeric string) that stands for an object that does not exist yet (D-059).
struct Placeholder(u64);

impl<'de> serde::Deserialize<'de> for Placeholder {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PlaceholderVisitor;

        impl serde::de::Visitor<'_> for PlaceholderVisitor {
            type Value = Placeholder;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a placeholder ID as an integer or numeric string")
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Placeholder, E> {
                Ok(Placeholder(value))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Placeholder, E> {
                u64::try_from(value)
                    .map(Placeholder)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Signed(value), &self))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Placeholder, E> {
                value
                    .parse()
                    .map(Placeholder)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_any(PlaceholderVisitor)
    }
}

/// `#[serde(with = "crate::serde_util::placeholder_id")]` for `u64` placeholder IDs: written as
/// integers, read from integers or numeric strings (D-059).
pub(crate) mod placeholder_id {
    use serde::{Deserialize, Deserializer, Serializer};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "signature required by serde(with)"
    )]
    pub(crate) fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(*value)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        super::Placeholder::deserialize(deserializer).map(|placeholder| placeholder.0)
    }
}

/// Like [`placeholder_id`] for `Option<u64>` (`null` / absent = `None`).
pub(crate) mod option_placeholder_id {
    use serde::{Deserialize, Deserializer, Serializer};

    #[expect(clippy::ref_option, reason = "signature required by serde(with)")]
    pub(crate) fn serialize<S: Serializer>(
        value: &Option<u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(id) => serializer.serialize_u64(*id),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u64>, D::Error> {
        Option::<super::Placeholder>::deserialize(deserializer)
            .map(|placeholder| placeholder.map(|placeholder| placeholder.0))
    }
}
