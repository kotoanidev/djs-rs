//! The connection between the manager and its cluster processes (D-285): TCP on `127.0.0.1`,
//! frames of a 4-byte big-endian length followed by that many bytes of JSON, and a token that
//! each process presents first.

use core::{fmt, hash::BuildHasher};
use std::{collections::hash_map::RandomState, io};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The largest frame accepted (16 MiB): a peer cannot make the other side allocate more.
pub(crate) const MAX_FRAME: u32 = 16 * 1024 * 1024;

/// What a cluster process sends to the manager.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum ToManager {
    /// The first frame of a connection: who the process is, and the proof that the manager
    /// started it.
    Hello { cluster: u32, token: String },
    /// Asks for a turn to send IDENTIFY for one of the process's shards (D-299).
    Identify { shard: u32 },
}

/// What the manager sends to a cluster process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum ToCluster {
    /// The answer to a valid [`ToManager::Hello`].
    Welcome,
    /// Stop the client; the process then exits.
    Shutdown,
    /// The turn asked by a [`ToManager::Identify`]: the shard may identify now.
    Identify { shard: u32 },
}

/// Writes `message` as one frame.
pub(crate) async fn write_frame<W, T>(writer: &mut W, message: &T) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let json = serde_json::to_vec(message).map_err(io::Error::other)?;
    let length = u32::try_from(json.len())
        .ok()
        .filter(|length| *length <= MAX_FRAME)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    let mut frame = Vec::with_capacity(4 + json.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&json);
    writer.write_all(&frame).await?;
    writer.flush().await
}

/// Reads one frame; `Ok(None)` when the connection was closed between frames.
pub(crate) async fn read_frame<R, T>(reader: &mut R) -> io::Result<Option<T>>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut length = [0; 4];
    match reader.read_exact(&mut length).await {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = u32::from_be_bytes(length);
    if length > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut json = vec![0; length as usize];
    reader.read_exact(&mut json).await?;
    serde_json::from_slice(&json)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// A secret that only the manager and the processes it started know (C-112): 128 bits from
/// two `RandomState`s, whose `SipHash` keys the standard library takes from the operating
/// system's random source.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Token(String);

impl Token {
    pub(crate) fn generate() -> Self {
        let high = RandomState::new().hash_one(0x6a73_5f72_735f_6970_u64);
        let low = RandomState::new().hash_one(high);
        Self(format!("{high:016x}{low:016x}"))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Compares with `other` in a time that does not depend on where they differ.
    pub(crate) fn matches(&self, other: &str) -> bool {
        let (ours, theirs) = (self.0.as_bytes(), other.as_bytes());
        ours.len() == theirs.len()
            && ours
                .iter()
                .zip(theirs)
                .fold(0, |difference, (a, b)| difference | (a ^ b))
                == 0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_FRAME, ToCluster, ToManager, Token, read_frame, write_frame};

    #[tokio::test]
    async fn frames_round_trip() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        let hello = ToManager::Hello {
            cluster: 3,
            token: "t".into(),
        };
        write_frame(&mut a, &hello).await.unwrap();
        write_frame(&mut a, &ToCluster::Shutdown).await.unwrap();
        write_frame(&mut a, &ToManager::Identify { shard: 7 })
            .await
            .unwrap();
        assert_eq!(
            read_frame::<_, ToManager>(&mut b).await.unwrap(),
            Some(hello)
        );
        assert_eq!(
            read_frame::<_, ToCluster>(&mut b).await.unwrap(),
            Some(ToCluster::Shutdown)
        );
        assert_eq!(
            read_frame::<_, ToManager>(&mut b).await.unwrap(),
            Some(ToManager::Identify { shard: 7 })
        );
        drop(a);
        assert_eq!(read_frame::<_, ToCluster>(&mut b).await.unwrap(), None);
    }

    #[tokio::test]
    async fn the_wire_format_is_length_prefixed_json() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        write_frame(&mut a, &ToCluster::Welcome).await.unwrap();
        drop(a);
        let mut bytes = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut b, &mut bytes)
            .await
            .unwrap();
        assert_eq!(bytes, b"\0\0\0\x10{\"op\":\"welcome\"}");
    }

    #[tokio::test]
    async fn oversized_and_broken_frames_are_errors() {
        let (mut a, mut b) = tokio::io::duplex(64);
        tokio::io::AsyncWriteExt::write_all(&mut a, &(MAX_FRAME + 1).to_be_bytes())
            .await
            .unwrap();
        assert!(read_frame::<_, ToCluster>(&mut b).await.is_err());

        let (mut a, mut b) = tokio::io::duplex(64);
        tokio::io::AsyncWriteExt::write_all(&mut a, b"\0\0\0\x02{}")
            .await
            .unwrap();
        assert!(read_frame::<_, ToCluster>(&mut b).await.is_err());
    }

    #[test]
    fn tokens_are_random_and_compared_whole() {
        let token = Token::generate();
        assert_eq!(token.as_str().len(), 32);
        assert_ne!(token.as_str(), Token::generate().as_str());
        assert!(token.matches(token.as_str()));
        assert!(!token.matches(&token.as_str()[..31]));
        assert!(!token.matches(""));
        assert_eq!(format!("{token:?}"), "Token(<redacted>)");
    }
}
