//! [`SoundData`]: a sound sent inline in a request body as a data URI (D-122).

use core::fmt;

use bytes::Bytes;
use compact_str::CompactString;
use serde::{Serialize, Serializer};

use super::image_data::write_base64;

/// A sound sent inside a JSON request body as a data URI (`data:audio/mpeg;base64,…`), like
/// [`ImageData`](super::ImageData) for images (D-122).
///
/// Used by [Create Guild Soundboard Sound](https://docs.discord.com/developers/resources/soundboard#create-guild-soundboard-sound).
/// The sound is kept as [`Bytes`] and base64-encoded while the body is serialized.
///
/// discord.js counterpart: the `file` option of `GuildSoundboardSoundManager#create`.
///
/// # Examples
///
/// ```
/// use djs_model::util::SoundData;
///
/// // A (truncated) Ogg file: the media type is read from the file signature.
/// let sound = SoundData::new(&b"OggS\0\x02"[..]).unwrap();
/// assert_eq!(sound.content_type(), "audio/ogg");
/// assert_eq!(sound.to_string(), "data:audio/ogg;base64,T2dnUwAC");
///
/// assert!(SoundData::new(&b"\x89PNG\r\n\x1a\n"[..]).is_err());
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SoundData {
    data: Bytes,
    content_type: CompactString,
}

impl SoundData {
    /// A sound whose media type is detected from its file signature: MP3 (with an ID3 tag
    /// or starting with a frame) or Ogg.
    ///
    /// # Errors
    ///
    /// [`SoundDataError`] when `data` is neither; use
    /// [`with_content_type`](Self::with_content_type) to send it anyway.
    pub fn new(data: impl Into<Bytes>) -> Result<Self, SoundDataError> {
        let data = data.into();
        let content_type = detect(&data).ok_or(SoundDataError)?;
        Ok(Self {
            data,
            content_type: CompactString::const_new(content_type),
        })
    }

    /// A sound of the given media type (e.g. `audio/mpeg`), which is not checked.
    #[must_use]
    pub fn with_content_type(
        data: impl Into<Bytes>,
        content_type: impl Into<CompactString>,
    ) -> Self {
        Self {
            data: data.into(),
            content_type: content_type.into(),
        }
    }

    /// The sound file's contents.
    #[must_use]
    pub const fn data(&self) -> &Bytes {
        &self.data
    }

    /// The sound's media type (e.g. `audio/ogg`).
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

/// Writes the data URI.
impl fmt::Display for SoundData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("data:")?;
        f.write_str(&self.content_type)?;
        f.write_str(";base64,")?;
        write_base64(&self.data, f)
    }
}

/// Shows the media type and the size, not the contents.
impl fmt::Debug for SoundData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SoundData")
            .field("content_type", &self.content_type)
            .field("len", &self.data.len())
            .finish()
    }
}

impl Serialize for SoundData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// The error of [`SoundData::new`]: the data is not an MP3 or Ogg file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SoundDataError;

impl fmt::Display for SoundDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown sound format (expected MP3 or Ogg)")
    }
}

impl core::error::Error for SoundDataError {}

/// The media type of `data` from its file signature.
fn detect(data: &[u8]) -> Option<&'static str> {
    match data {
        [b'O', b'g', b'g', b'S', ..] => Some("audio/ogg"),
        // An ID3v2 tag, or an MPEG audio frame (11 set sync bits).
        [b'I', b'D', b'3', ..] => Some("audio/mpeg"),
        [0xff, second, ..] if second & 0xe0 == 0xe0 => Some("audio/mpeg"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_signatures() {
        let detected = |data: &[u8]| SoundData::new(data.to_vec()).map(|sound| sound.content_type);
        assert_eq!(detected(b"ID3\x04\0").unwrap(), "audio/mpeg");
        assert_eq!(detected(b"\xff\xfb\x90\x64").unwrap(), "audio/mpeg");
        assert_eq!(detected(b"OggS").unwrap(), "audio/ogg");
        assert_eq!(detected(b"\xff\xd8\xff"), Err(SoundDataError));
        assert_eq!(detected(b"RIFF"), Err(SoundDataError));
        assert_eq!(detected(b""), Err(SoundDataError));
    }

    #[test]
    fn serializes_and_hides_the_contents() {
        let sound = SoundData::with_content_type(&b"abc"[..], "audio/mpeg");
        assert_eq!(
            serde_json::to_string(&sound).unwrap(),
            r#""data:audio/mpeg;base64,YWJj""#
        );
        assert_eq!(
            format!("{sound:?}"),
            r#"SoundData { content_type: "audio/mpeg", len: 3 }"#
        );
    }
}
