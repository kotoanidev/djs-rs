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
