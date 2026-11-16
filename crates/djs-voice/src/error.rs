//! The errors of djs-voice: one type per operation (D-336).

use core::fmt;
use std::{error::Error, io};

use crate::ogg::OggError;

/// Why [`VoiceManager::join`](crate::VoiceManager::join) failed.
///
/// discord.js counterpart: none (`joinVoiceChannel` returns a `VoiceConnection` whose state
/// becomes `Disconnected` or `Destroyed`); songbird: `JoinError`.
#[derive(Debug)]
#[non_exhaustive]
pub enum JoinError {
    /// The [`VoiceGateway`](crate::VoiceGateway) could not send Opcode 4 (Update Voice
    /// State) through the guild's shard; its error is the [`source`](Error::source).
    Gateway(Box<dyn Error + Send + Sync>),
    /// Discord did not answer, or the voice connection was not ready to send audio, within
    /// 10 seconds (D-322). A bot without the Connect permission in the channel gets no
    /// answer at all, so it sees this too.
    Timeout,
    /// The voice connection failed; why is the [`source`](Error::source).
    Connection(Box<dyn Error + Send + Sync>),
    /// The call was left, or joined to another channel, before this join finished.
    Cancelled,
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Gateway(_) => "the voice state update could not be sent to the gateway",
            Self::Timeout => "joining the voice channel timed out",
            Self::Connection(_) => "the voice connection failed",
            Self::Cancelled => "the join was cancelled by a leave or another join",
        })
    }
}

impl Error for JoinError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Gateway(error) | Self::Connection(error) => Some(&**error),
            Self::Timeout | Self::Cancelled => None,
        }
    }
}

/// Why an [`Input`](crate::Input) cannot be played.
///
/// discord.js counterpart: the errors of `createAudioResource` and prism-media's demuxers.
#[derive(Debug)]
#[non_exhaustive]
pub enum InputError {
    /// Reading the input failed.
    Io(io::Error),
    /// The input is not an Ogg stream: no Ogg page in its first 64 KiB.
    NotOgg,
    /// The Ogg stream does not hold Opus audio from its start, or its Opus headers are
    /// malformed.
    NotOpus,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "reading the input failed: {error}"),
            Self::NotOgg => f.write_str("the input is not an Ogg stream"),
            Self::NotOpus => f.write_str("the Ogg stream holds no well-formed Opus audio"),
        }
    }
}

impl Error for InputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NotOgg | Self::NotOpus => None,
        }
    }
}

impl From<io::Error> for InputError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<OggError> for InputError {
    fn from(error: OggError) -> Self {
        match error {
            OggError::NotOgg => Self::NotOgg,
            OggError::NotOpus | OggError::BadHead | OggError::BadTags => Self::NotOpus,
        }
    }
}

/// Why a [`TrackHandle`](crate::TrackHandle) control did nothing (D-339).
///
/// discord.js counterpart: none (`AudioPlayer#pause` returns `false`); songbird:
/// `ControlError`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrackError {
    /// The track already ended: it played to its end, was stopped or replaced, its input
    /// failed, or its call ended.
    Ended,
}

impl fmt::Display for TrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ended => "the track has ended",
        })
    }
}

impl Error for TrackError {}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{InputError, JoinError, TrackError};
    use crate::ogg::OggError;

    #[test]
    fn errors_describe_themselves_and_their_causes() {
        let cause: Box<dyn Error + Send + Sync> = "shard closed".into();
        let error = JoinError::Gateway(cause);
        assert_eq!(
            error.to_string(),
            "the voice state update could not be sent to the gateway"
        );
        assert_eq!(error.source().unwrap().to_string(), "shard closed");
        assert!(JoinError::Timeout.source().is_none());
        let io = InputError::from(std::io::Error::other("disk"));
        assert_eq!(io.to_string(), "reading the input failed: disk");
        assert!(io.source().is_some());
        assert_eq!(TrackError::Ended.to_string(), "the track has ended");
    }

    #[test]
    fn ogg_errors_become_input_errors() {
        assert!(matches!(
            InputError::from(OggError::NotOgg),
            InputError::NotOgg
        ));
        for error in [OggError::NotOpus, OggError::BadHead, OggError::BadTags] {
            assert!(matches!(InputError::from(error), InputError::NotOpus));
        }
    }
}
