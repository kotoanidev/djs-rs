//! What a track plays (D-317, D-323, D-338): in step 1, Ogg/Opus from a file or any
//! [`AsyncRead`], sent as it is (no decoding, no re-encoding).

use core::fmt;
use std::path::Path;

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::mpsc,
};

use crate::{InputError, ogg::OggOpus, opus};

/// How many bytes of an input are read at a time.
const CHUNK: usize = 16 * 1024;

/// What a track plays, given to [`Call::play`](crate::Call::play).
///
/// In this version: Ogg/Opus audio (`.opus` and `.ogg` files with Opus inside, as `ffmpeg
/// -c:a libopus` or `opusenc` make them), from a file ([`ogg_file`](Self::ogg_file)) or from
/// any [`AsyncRead`] ([`ogg`](Self::ogg)): a pipe, a socket, an HTTP body, memory. Its packets
/// are sent as they are, without decoding: Discord wants 48 kHz Opus, which Ogg/Opus always
/// is. Later versions add raw PCM and anything `ffmpeg` can read (D-317).
///
/// The input is read ahead of playback, at most about a second, so a slow source makes the
/// track skip rather than the call stall.
///
/// discord.js counterpart: `createAudioResource(stream, { inputType: StreamType.OggOpus })`
/// of `@discordjs/voice`; songbird: `Input`.
///
/// # Examples
///
/// ```no_run
/// # async fn run(call: djs_voice::Call) -> Result<(), djs_voice::InputError> {
/// use djs_voice::Input;
///
/// let input = Input::ogg_file("music/intro.opus").await?;
/// let track = call.play(input);
/// # Ok(()) }
/// ```
pub struct Input {
    source: Source,
}

enum Source {
    Ogg {
        reader: Box<dyn AsyncRead + Send + Unpin>,
        ogg: Box<OggOpus>,
        /// The first audio packet, read while checking the input.
        first: Option<Vec<u8>>,
    },
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Source::Ogg { .. } => f.write_str("Input::Ogg"),
        }
    }
}

/// An audio packet read ahead, and how long it lasts (samples at 48 kHz).
#[derive(Debug)]
pub(crate) struct Packet {
    pub(crate) data: Vec<u8>,
    pub(crate) samples: u32,
}

impl Input {
    /// Ogg/Opus audio read from `reader`.
    ///
    /// Nothing is read until the track plays; an input that turns out not to be Ogg/Opus
    /// (or fails to read) ends its track then. [`ogg_file`](Self::ogg_file) checks a file at
    /// once.
    ///
    /// discord.js counterpart: `createAudioResource(stream, { inputType: StreamType.OggOpus })`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn download() -> Vec<u8> { Vec::new() }
    /// # async fn run(call: djs_voice::Call) {
    /// use djs_voice::Input;
    ///
    /// // Any `AsyncRead`: here bytes already in memory.
    /// let bytes: Vec<u8> = download().await;
    /// call.play(Input::ogg(std::io::Cursor::new(bytes)));
    /// # }
    /// ```
    #[must_use]
    pub fn ogg(reader: impl AsyncRead + Send + Unpin + 'static) -> Self {
        Self {
            source: Source::Ogg {
                reader: Box::new(reader),
                ogg: Box::new(OggOpus::new()),
                first: None,
            },
        }
    }

    /// Ogg/Opus audio from the file at `path`, opened and checked now: the result is an
    /// error if it cannot be read or does not start like Ogg/Opus.
    ///
    /// discord.js counterpart: `createAudioResource(createReadStream(path), { inputType:
    /// StreamType.OggOpus })`.
    ///
    /// # Errors
    ///
    /// [`InputError::Io`] when the file cannot be opened or read, [`InputError::NotOgg`] or
    /// [`InputError::NotOpus`] when it is not Ogg/Opus.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(call: djs_voice::Call) -> Result<(), djs_voice::InputError> {
    /// let track = call.play(djs_voice::Input::ogg_file("intro.opus").await?);
    /// # Ok(()) }
    /// ```
    pub async fn ogg_file(path: impl AsRef<Path>) -> Result<Self, InputError> {
        let mut reader = tokio::fs::File::open(path).await?;
        let mut ogg = Box::new(OggOpus::new());
        let mut chunk = vec![0; CHUNK];
        let first = loop {
            if let Some(packet) = ogg.packet()? {
                break Some(packet.to_vec());
            }
            let read = reader.read(&mut chunk).await?;
            if read == 0 {
                // An empty stream is still checked: `finish` makes the reader say whether
                // it saw Ogg/Opus at all.
                ogg.finish();
                break ogg.packet()?.map(<[u8]>::to_vec);
            }
            ogg.push(&chunk[..read]);
        };
        Ok(Self {
            source: Source::Ogg {
                reader: Box::new(reader),
                ogg,
                first,
            },
        })
    }

    /// Reads the input's audio packets into `packets` until its end, an error, or the
    /// receiver is dropped. Packets that are not valid Opus are skipped.
    pub(crate) async fn read(self, packets: mpsc::Sender<Packet>) -> Result<(), InputError> {
        let Source::Ogg {
            mut reader,
            mut ogg,
            first,
        } = self.source;
        if let Some(data) = first
            && !forward(data, &packets).await
        {
            return Ok(());
        }
        let mut chunk = vec![0; CHUNK];
        let mut ended = false;
        loop {
            while let Some(data) = ogg.packet()? {
                let data = data.to_vec();
                if !forward(data, &packets).await {
                    return Ok(());
                }
            }
            if ended {
                return Ok(());
            }
            let read = reader.read(&mut chunk).await?;
            if read == 0 {
                ogg.finish();
                ended = true;
            } else {
                ogg.push(&chunk[..read]);
            }
        }
    }
}

/// Sends a packet to the player; `false` once the player no longer wants any.
async fn forward(data: Vec<u8>, packets: &mpsc::Sender<Packet>) -> bool {
    let Some(samples) = opus::packet_samples(&data) else {
        tracing::debug!(len = data.len(), "skipping a packet that is not Opus");
        return true;
    };
    packets.send(Packet { data, samples }).await.is_ok()
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::Input;
    use crate::InputError;

    const SINE: &[u8] = include_bytes!("../tests/data/sine.opus");

    async fn packets(input: Input) -> (Vec<super::Packet>, Result<(), InputError>) {
        let (tx, mut rx) = mpsc::channel(4);
        let reader = tokio::spawn(input.read(tx));
        let mut packets = Vec::new();
        while let Some(packet) = rx.recv().await {
            packets.push(packet);
        }
        (packets, reader.await.unwrap())
    }

    #[tokio::test]
    async fn reads_every_packet_of_a_stream() {
        let (packets, result) = packets(Input::ogg(SINE)).await;
        result.unwrap();
        assert_eq!(packets.len(), 51);
        assert!(packets.iter().all(|packet| packet.samples == 960));
    }

    #[tokio::test]
    async fn files_are_checked_when_opened() {
        let dir = std::env::temp_dir().join(format!("djs-voice-input-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("sine.opus");
        std::fs::write(&good, SINE).unwrap();
        let (packets, result) = packets(Input::ogg_file(&good).await.unwrap()).await;
        result.unwrap();
        assert_eq!(packets.len(), 51, "the packet read while checking is kept");
        let text = dir.join("text.opus");
        std::fs::write(&text, b"not audio at all").unwrap();
        assert!(matches!(
            Input::ogg_file(&text).await,
            Err(InputError::NotOgg)
        ));
        assert!(matches!(
            Input::ogg_file(dir.join("missing.opus")).await,
            Err(InputError::Io(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn readers_report_inputs_that_are_not_ogg_opus() {
        let (packets, result) = packets(Input::ogg(&b"plain text"[..])).await;
        assert!(packets.is_empty());
        assert!(matches!(result, Err(InputError::NotOgg)));
    }

    #[tokio::test]
    async fn reading_stops_when_the_player_does() {
        let (tx, mut rx) = mpsc::channel(1);
        let reader = tokio::spawn(Input::ogg(SINE).read(tx));
        rx.recv().await.unwrap();
        drop(rx);
        reader.await.unwrap().unwrap();
    }
}
