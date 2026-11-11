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

/// A Unix time as an integer (or float) in milliseconds or seconds (D-075).
struct UnixTime<const MILLIS: bool>(crate::util::Timestamp);

impl<'de, const MILLIS: bool> serde::Deserialize<'de> for UnixTime<MILLIS> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UnixTimeVisitor<const MILLIS: bool>;

        impl<const MILLIS: bool> UnixTimeVisitor<MILLIS> {
            fn convert<E: serde::de::Error>(value: i64) -> Result<UnixTime<MILLIS>, E> {
                let timestamp = if MILLIS {
                    crate::util::Timestamp::from_unix_millis(value)
                } else {
                    crate::util::Timestamp::from_unix_secs(value)
                };
                timestamp.map(UnixTime).ok_or_else(|| {
                    E::invalid_value(serde::de::Unexpected::Signed(value), &"a Unix time")
                })
            }
        }

        impl<const MILLIS: bool> serde::de::Visitor<'_> for UnixTimeVisitor<MILLIS> {
            type Value = UnixTime<MILLIS>;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(if MILLIS {
                    "a Unix time in milliseconds"
                } else {
                    "a Unix time in seconds"
                })
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Self::convert(value)
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                i64::try_from(value)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Unsigned(value), &self))
                    .and_then(Self::convert)
            }

            #[expect(
                clippy::cast_possible_truncation,
                reason = "fractions of the unit are dropped; out-of-range values saturate and are rejected"
            )]
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                if !value.is_finite() {
                    return Err(E::invalid_value(serde::de::Unexpected::Float(value), &self));
                }
                Self::convert(value as i64)
            }
        }

        deserializer.deserialize_any(UnixTimeVisitor::<MILLIS>)
    }
}

macro_rules! unix_time_module {
    ($millis:literal, $module:ident, $as_unix:ident, $doc:literal) => {
        #[doc = concat!("`#[serde(with = …)]` for a [`Timestamp`](crate::util::Timestamp) sent as Unix ", $doc, " (D-075).")]
        pub(crate) mod $module {
            use serde::{Deserialize, Deserializer, Serializer};

            use crate::util::Timestamp;

            pub(crate) fn serialize<S: Serializer>(
                value: &Timestamp,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                serializer.serialize_i64(value.$as_unix())
            }

            pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Timestamp, D::Error> {
                super::UnixTime::<$millis>::deserialize(deserializer).map(|time| time.0)
            }
        }
    };
}

macro_rules! option_unix_time_module {
    ($millis:literal, $module:ident, $as_unix:ident, $doc:literal) => {
        #[doc = concat!("`#[serde(with = …)]` for an `Option<Timestamp>` sent as Unix ", $doc, " (D-075; `null` / absent = `None`).")]
        pub(crate) mod $module {
            use serde::{Deserialize, Deserializer, Serializer};

            use crate::util::Timestamp;

            #[expect(clippy::ref_option, reason = "signature required by serde(with)")]
            pub(crate) fn serialize<S: Serializer>(
                value: &Option<Timestamp>,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                match value {
                    Some(value) => serializer.serialize_i64(value.$as_unix()),
                    None => serializer.serialize_none(),
                }
            }

            pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Option<Timestamp>, D::Error> {
                Option::<super::UnixTime<$millis>>::deserialize(deserializer)
                    .map(|time| time.map(|time| time.0))
            }
        }
    };
}

option_unix_time_module!(true, option_unix_millis, as_unix_millis, "milliseconds");
unix_time_module!(false, unix_secs, as_unix_secs, "seconds");
option_unix_time_module!(false, option_unix_secs, as_unix_secs, "seconds");

/// Single-pass decoding of "an object plus extra fields" (D-073): a [`MapAccess`] adapter hands
/// the extra keys to an [`Extras`] collector and every other key to the base type's own
/// `Deserialize`, without buffering and without copying the base type's field list.
///
/// [`MapAccess`]: serde::de::MapAccess
pub(crate) mod extras {
    use std::borrow::Cow;

    use serde::Deserialize;
    use serde::de::{
        DeserializeSeed, Deserializer, IntoDeserializer, MapAccess, Visitor,
        value::{BorrowedStrDeserializer, MapAccessDeserializer, StringDeserializer},
    };

    /// What the collector did with a key.
    pub(crate) enum Visit {
        /// The collector read the value; the base type does not see the key.
        Consumed,
        /// The key belongs to the base type.
        Pass,
        /// The collector read the value as this integer (an ID it also needs); the base type
        /// sees the key and gets the integer as its value.
        Replay(u64),
    }

    /// Collects the extra fields of an event.
    pub(crate) trait Extras<'de>: Default {
        /// Handles `key`, reading its value from `map` unless it returns [`Visit::Pass`].
        fn visit<A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<Visit, A::Error>;

        /// Whether a failure of the base type is expected (the payload is another variant,
        /// e.g. an unavailable guild); the base type's value is then `None`.
        fn recover(&self) -> bool {
            false
        }
    }

    /// A map key, borrowed from the input when possible.
    struct Key<'de>(Cow<'de, str>);

    impl<'de> Deserialize<'de> for Key<'de> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct KeyVisitor;

            impl<'de> Visitor<'de> for KeyVisitor {
                type Value = Key<'de>;

                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("a field name")
                }

                fn visit_borrowed_str<E: serde::de::Error>(
                    self,
                    value: &'de str,
                ) -> Result<Key<'de>, E> {
                    Ok(Key(Cow::Borrowed(value)))
                }

                fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Key<'de>, E> {
                    Ok(Key(Cow::Owned(value.to_owned())))
                }

                fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Key<'de>, E> {
                    Ok(Key(Cow::Owned(value)))
                }
            }

            deserializer.deserialize_str(KeyVisitor)
        }
    }

    struct ExtrasMapAccess<'e, A, E> {
        inner: A,
        extras: &'e mut E,
        replay: Option<u64>,
    }

    impl<'de, A: MapAccess<'de>, E: Extras<'de>> MapAccess<'de> for ExtrasMapAccess<'_, A, E> {
        type Error = A::Error;

        fn next_key_seed<K: DeserializeSeed<'de>>(
            &mut self,
            seed: K,
        ) -> Result<Option<K::Value>, A::Error> {
            loop {
                let Some(Key(key)) = self.inner.next_key::<Key<'de>>()? else {
                    return Ok(None);
                };
                match self.extras.visit(&key, &mut self.inner)? {
                    Visit::Consumed => {}
                    visit => {
                        if let Visit::Replay(value) = visit {
                            self.replay = Some(value);
                        }
                        return match key {
                            Cow::Borrowed(key) => seed
                                .deserialize(BorrowedStrDeserializer::new(key))
                                .map(Some),
                            Cow::Owned(key) => {
                                seed.deserialize(StringDeserializer::new(key)).map(Some)
                            }
                        };
                    }
                }
            }
        }

        fn next_value_seed<V: DeserializeSeed<'de>>(
            &mut self,
            seed: V,
        ) -> Result<V::Value, A::Error> {
            match self.replay.take() {
                Some(value) => seed.deserialize(value.into_deserializer()),
                None => self.inner.next_value_seed(seed),
            }
        }

        fn size_hint(&self) -> Option<usize> {
            self.inner.size_hint()
        }
    }

    struct ExtrasVisitor<T, E>(core::marker::PhantomData<fn() -> (T, E)>);

    impl<'de, T: Deserialize<'de>, E: Extras<'de>> Visitor<'de> for ExtrasVisitor<T, E> {
        type Value = (Option<T>, E);

        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("an object")
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            let mut extras = E::default();
            let base = T::deserialize(MapAccessDeserializer::new(ExtrasMapAccess {
                inner: map,
                extras: &mut extras,
                replay: None,
            }));
            match base {
                Ok(base) => Ok((Some(base), extras)),
                Err(_) if extras.recover() => Ok((None, extras)),
                Err(error) => Err(error),
            }
        }
    }

    /// Decodes a `T` and its extras from one object. `T` is `None` only when
    /// [`Extras::recover`] accepted a failure of `T`.
    pub(crate) fn deserialize<'de, T, E, D>(deserializer: D) -> Result<(Option<T>, E), D::Error>
    where
        T: Deserialize<'de>,
        E: Extras<'de>,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ExtrasVisitor(core::marker::PhantomData))
    }
}
