//! Compact storage for Discord image hashes.

use core::{fmt, str::FromStr};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Visitor},
};

/// The hash of an image on Discord's CDN — avatars, icons, banners, splashes and so on.
///
/// Discord sends image hashes as 32 hexadecimal characters, prefixed with `a_` when the image
/// is animated (for example `a_1269e74af4df7417b13759eae50c83dc`). Instead of keeping that
/// 34-byte string on the heap, `ImageHash` stores the decoded 16 bytes plus an animation flag:
/// it is 17 bytes, `Copy`, and `Option<ImageHash>` is also 17 bytes.
///
/// Formatting with [`Display`](fmt::Display) reproduces the original string, which is what
/// [CDN URLs](https://docs.discord.com/developers/reference#image-formatting) need.
///
/// # Examples
///
/// ```
/// use djs_model::util::ImageHash;
///
/// let hash: ImageHash = "a_1269e74af4df7417b13759eae50c83dc".parse().unwrap();
/// assert!(hash.is_animated());
/// assert_eq!(hash.to_string(), "a_1269e74af4df7417b13759eae50c83dc");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageHash {
    bytes: [u8; 16],
    animated: bool,
}

impl ImageHash {
    /// Creates an image hash from its raw 16 bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16], animated: bool) -> Self {
        Self { bytes, animated }
    }

    /// Returns the raw 16 bytes of the hash.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.bytes
    }

    /// Returns `true` if the image is animated (the hash had an `a_` prefix).
    ///
    /// Animated images are available as GIF (and WebP/AVIF animations) on the CDN.
    #[must_use]
    pub const fn is_animated(self) -> bool {
        self.animated
    }

    /// Parses an image hash such as `a_1269e74af4df7417b13759eae50c83dc`.
    ///
    /// Both lowercase and uppercase hexadecimal digits are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`ParseImageHashError`] if the input is not 32 hexadecimal characters with an
    /// optional `a_` prefix.
    pub const fn parse(input: &str) -> Result<Self, ParseImageHashError> {
        let mut bytes = input.as_bytes();
        let animated = matches!(bytes, [b'a', b'_', ..]);
        if animated {
            bytes = bytes.split_at(2).1;
        }
        if bytes.len() != 32 {
            return Err(ParseImageHashError(()));
        }

        let mut out = [0_u8; 16];
        let mut i = 0;
        while i < 16 {
            let (Some(high), Some(low)) = (hex_value(bytes[i * 2]), hex_value(bytes[i * 2 + 1]))
            else {
                return Err(ParseImageHashError(()));
            };
            out[i] = (high << 4) | low;
            i += 1;
        }
        Ok(Self::new(out, animated))
    }
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl FromStr for ImageHash {
    type Err = ParseImageHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Formats the hash exactly as Discord sends it: lowercase hex with an `a_` prefix if animated.
impl fmt::Display for ImageHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut buffer = [0_u8; 34];
        let start = if self.animated {
            buffer[0] = b'a';
            buffer[1] = b'_';
            2
        } else {
            0
        };
        for (i, byte) in self.bytes.iter().enumerate() {
            buffer[start + i * 2] = HEX[usize::from(byte >> 4)];
            buffer[start + i * 2 + 1] = HEX[usize::from(byte & 0x0F)];
        }
        let text = core::str::from_utf8(&buffer[..start + 32]).map_err(|_| fmt::Error)?;
        f.write_str(text)
    }
}

/// The error returned when parsing an [`ImageHash`] fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseImageHashError(());

impl fmt::Display for ParseImageHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("image hash must be 32 hexadecimal characters with an optional `a_` prefix")
    }
}

impl core::error::Error for ParseImageHashError {}

impl Serialize for ImageHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ImageHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ImageHashVisitor;

        impl Visitor<'_> for ImageHashVisitor {
            type Value = ImageHash;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an image hash string")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<Self::Value, E> {
                ImageHash::parse(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(ImageHashVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout() {
        assert_eq!(size_of::<ImageHash>(), 17);
        assert_eq!(size_of::<Option<ImageHash>>(), 17);
    }

    #[test]
    fn round_trip() {
        for input in [
            "1269e74af4df7417b13759eae50c83dc",
            "a_1269e74af4df7417b13759eae50c83dc",
            "00000000000000000000000000000000",
            "ffffffffffffffffffffffffffffffff",
        ] {
            assert_eq!(input.parse::<ImageHash>().unwrap().to_string(), input);
        }
    }

    #[test]
    fn uppercase_is_normalised() {
        let hash: ImageHash = "1269E74AF4DF7417B13759EAE50C83DC".parse().unwrap();
        assert_eq!(hash.to_string(), "1269e74af4df7417b13759eae50c83dc");
    }

    #[test]
    fn rejects_invalid() {
        for input in [
            "",
            "a_",
            "1269e74af4df7417b13759eae50c83d",
            "1269e74af4df7417b13759eae50c83dcc",
            "1269e74af4df7417b13759eae50c83dg",
            "b_1269e74af4df7417b13759eae50c83dc",
        ] {
            assert!(
                input.parse::<ImageHash>().is_err(),
                "{input:?} should be rejected"
            );
        }
    }
}
