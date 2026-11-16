//! What a track plays (D-317, D-323, D-338, D-354): Ogg/Opus from a file or any
//! [`AsyncRead`], raw PCM, and anything FFmpeg reads.
//!
//! A track's reader task reads its input ahead into raw frames, Opus packets or 20 ms blocks
//! of PCM, which the call's task turns into packets when they are due (C-126). The inputs
//! that can be opened again, a file and FFmpeg, loop and seek (D-358–D-360).

use core::fmt;
use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::mpsc,
};

use crate::{InputError, codec::FRAME_BYTES, ogg::OggOpus, opus};

/// How many bytes of an Ogg input are read at a time.
const CHUNK: usize = 16 * 1024;
/// How much of FFmpeg's standard error is kept for [`InputError::Ffmpeg`].
const STDERR_TAIL: usize = 4 * 1024;
/// The lines of FFmpeg's standard error an [`InputError::Ffmpeg`] shows.
const STDERR_LINES: usize = 4;
/// The program `Input::ffmpeg` runs, from `PATH`.
const FFMPEG: &str = "ffmpeg";

/// What a track plays, given to [`Call::play`](crate::Call::play) or
/// [`Call::enqueue`](crate::Call::enqueue).
///
/// - **Ogg/Opus** (`.opus` and `.ogg` files with Opus inside, as `ffmpeg -c:a libopus` or
///   `opusenc` make them), from a file ([`ogg_file`](Self::ogg_file)) or from any
///   [`AsyncRead`] ([`ogg`](Self::ogg)): a pipe, a socket, an HTTP body, memory. At volume
///   1.0 its packets are sent as they are, without decoding: Discord wants 48 kHz Opus,
///   which Ogg/Opus always is.
/// - **Raw PCM** ([`pcm`](Self::pcm)): 16-bit little-endian, 48 kHz, interleaved stereo,
///   encoded to Opus by the call.
/// - **Anything FFmpeg reads** ([`ffmpeg`](Self::ffmpeg)): MP3, FLAC, WAV, AAC, video files,
///   HTTP(S) URLs, … FFmpeg must be installed; it decodes, djs-voice encodes.
///
/// The input is read ahead of playback, at most about a second, so a slow source makes the
/// track skip rather than the call stall. A track from a file or FFmpeg can loop
/// ([`TrackHandle::set_loop`](crate::TrackHandle::set_loop)) and seek
/// ([`TrackHandle::seek`](crate::TrackHandle::seek)), as they can be opened again; a track
/// from a reader cannot (D-360).
///
/// discord.js counterpart: `createAudioResource` of `@discordjs/voice`; songbird: `Input`.
///
/// # Examples
///
/// ```no_run
/// # async fn run(call: djs_voice::Call) -> Result<(), djs_voice::InputError> {
/// use djs_voice::Input;
///
/// call.play(Input::ogg_file("music/intro.opus").await?);
/// call.enqueue(Input::ffmpeg("music/song.mp3"));
/// # Ok(()) }
/// ```
pub struct Input {
    source: Source,
    again: Option<Again>,
}

/// What an input reads.
pub(crate) enum Source {
    Ogg {
        reader: Box<dyn AsyncRead + Send + Unpin>,
        ogg: Box<OggOpus>,
        /// The first audio packet, read while checking the input.
        first: Option<Vec<u8>>,
    },
    OggFile(PathBuf),
    Pcm(Box<dyn AsyncRead + Send + Unpin>),
    Ffmpeg {
        program: OsString,
        source: OsString,
    },
}

/// How to open an input again, for loops and seeking.
#[derive(Clone)]
pub(crate) enum Again {
    OggFile(PathBuf),
    Ffmpeg { program: OsString, source: OsString },
}

impl Again {
    pub(crate) fn source(&self) -> Source {
        match self {
            Self::OggFile(path) => Source::OggFile(path.clone()),
            Self::Ffmpeg { program, source } => Source::Ffmpeg {
                program: program.clone(),
                source: source.clone(),
            },
        }
    }
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // No path or URL: a URL may carry a signature.
        f.write_str(match (&self.source, &self.again) {
            (Source::Ogg { .. } | Source::OggFile(_), Some(_)) => "Input::OggFile",
            (Source::Ogg { .. } | Source::OggFile(_), None) => "Input::Ogg",
            (Source::Pcm(_), _) => "Input::Pcm",
            (Source::Ffmpeg { .. }, _) => "Input::Ffmpeg",
        })
    }
}

/// An audio frame read ahead.
#[derive(Debug)]
pub(crate) enum Frame {
    /// An Opus packet, and how long it lasts (samples per channel at 48 kHz).
    Opus { data: Vec<u8>, samples: u32 },
    /// 20 ms of s16le stereo PCM ([`FRAME_BYTES`]).
    Pcm(Vec<u8>),
}

impl Frame {
    /// How long the frame lasts: samples per channel at 48 kHz.
    pub(crate) fn samples(&self) -> u32 {
        match self {
            Self::Opus { samples, .. } => *samples,
            Self::Pcm(_) => 960,
        }
    }
}

/// What a reader sends the player.
#[derive(Debug)]
pub(crate) enum Chunk {
    /// Where the audio that follows starts, for a read that started past the beginning: the
    /// position (samples per channel) asked for, or the start of the frame holding it.
    Start(u64),
    Frame(Frame),
}

/// Where a reader sends its chunks; an error is the last thing it sends.
pub(crate) type Chunks = mpsc::Sender<Result<Chunk, InputError>>;

impl Input {
    /// Ogg/Opus audio read from `reader`.
    ///
    /// Nothing is read until the track plays; an input that turns out not to be Ogg/Opus
    /// (or fails to read) ends its track then. [`ogg_file`](Self::ogg_file) checks a file at
    /// once. The track cannot loop or seek (D-360).
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
            again: None,
        }
    }

    /// Ogg/Opus audio from the file at `path`, opened and checked now: the result is an
    /// error if it cannot be read or does not start like Ogg/Opus. The track can loop and
    /// seek: the file is opened again for them.
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
        let path = path.as_ref();
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
            again: Some(Again::OggFile(path.to_path_buf())),
        })
    }

    /// Raw PCM read from `reader`: 16-bit signed little-endian samples, 48 kHz, two
    /// interleaved channels (left, right), as `ffmpeg -f s16le -ar 48000 -ac 2` writes it.
    /// The call encodes it to Opus; a last frame shorter than 20 ms is completed with
    /// silence. The track cannot loop or seek (D-360).
    ///
    /// discord.js counterpart: `createAudioResource(stream, { inputType: StreamType.Raw })`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(call: djs_voice::Call) {
    /// // One second of silence: 48 000 samples × 2 channels × 2 bytes.
    /// let silence = vec![0_u8; 192_000];
    /// call.play(djs_voice::Input::pcm(std::io::Cursor::new(silence)));
    /// # }
    /// ```
    #[must_use]
    pub fn pcm(reader: impl AsyncRead + Send + Unpin + 'static) -> Self {
        Self {
            source: Source::Pcm(Box::new(reader)),
            again: None,
        }
    }

    /// Anything FFmpeg can read: a file path or a URL (`https://…`, with reconnection on
    /// errors), in any format it decodes; video files play their audio.
    ///
    /// `ffmpeg` is run from `PATH` when the track starts to play (not now, so a queue of
    /// tracks does not run a program for each), and killed when the track ends. Its errors
    /// end the track ([`TrackEnd::Failed`](crate::TrackEnd::Failed)): no program
    /// ([`InputError::FfmpegMissing`]), or a source it cannot read
    /// ([`InputError::Ffmpeg`]). The track can loop and seek: FFmpeg is run again for them.
    ///
    /// discord.js counterpart: `createAudioResource(path)` (`StreamType.Arbitrary`, through
    /// prism-media's FFmpeg); songbird: `File` and `HttpRequest` inputs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn run(call: djs_voice::Call) {
    /// use djs_voice::Input;
    ///
    /// call.enqueue(Input::ffmpeg("music/song.mp3"));
    /// call.enqueue(Input::ffmpeg("https://example.com/radio.aac"));
    /// # }
    /// ```
    #[must_use]
    pub fn ffmpeg(source: impl AsRef<OsStr>) -> Self {
        let program = OsString::from(FFMPEG);
        let source = source.as_ref().to_os_string();
        Self {
            source: Source::Ffmpeg {
                program: program.clone(),
                source: source.clone(),
            },
            again: Some(Again::Ffmpeg { program, source }),
        }
    }

    /// What the input reads first, and how to open it again (`None` for a reader).
    pub(crate) fn into_parts(self) -> (Source, Option<Again>) {
        (self.source, self.again)
    }

    /// Whether the input can be opened again: its track loops and seeks.
    pub(crate) fn seekable(&self) -> bool {
        self.again.is_some()
    }

    /// An FFmpeg input running `program` instead of `ffmpeg`.
    #[cfg(test)]
    pub(crate) fn ffmpeg_with(program: &str, source: impl AsRef<OsStr>) -> Self {
        let program = OsString::from(program);
        let source = source.as_ref().to_os_string();
        Self {
            source: Source::Ffmpeg {
                program: program.clone(),
                source: source.clone(),
            },
            again: Some(Again::Ffmpeg { program, source }),
        }
    }
}

/// Reads `source` from `start` (samples per channel) into `chunks`, until its end, an error
/// (sent last), or the receiver is dropped.
pub(crate) async fn read(source: Source, start: u64, chunks: Chunks) {
    let result = match source {
        Source::Ogg { reader, ogg, first } => read_ogg(reader, ogg, first, start, &chunks).await,
        Source::OggFile(path) => match tokio::fs::File::open(path).await {
            Ok(file) => read_ogg(Box::new(file), Box::default(), None, start, &chunks).await,
            Err(error) => Err(error.into()),
        },
        Source::Pcm(reader) => read_pcm(reader, None, &chunks).await.map(drop),
        Source::Ffmpeg { program, source } => ffmpeg(&program, &source, start, &chunks).await,
    };
    if let Err(error) = result {
        let _ = chunks.send(Err(error)).await;
    }
}

/// Where an Ogg read is, before `start`: packets are skipped (without decoding) until the one
/// holding it.
struct Skip {
    start: u64,
    at: u64,
}

impl Skip {
    /// Whether the packet of `samples` that comes next is played: the first one that is
    /// played after a skip sends [`Chunk::Start`] first.
    async fn keep(&mut self, samples: u32, chunks: &Chunks) -> Option<bool> {
        if self.start == 0 {
            return Some(true);
        }
        if self.at + u64::from(samples) <= self.start {
            self.at += u64::from(samples);
            return Some(false);
        }
        let at = self.at;
        self.start = 0;
        chunks.send(Ok(Chunk::Start(at))).await.ok()?;
        Some(true)
    }
}

async fn read_ogg(
    mut reader: Box<dyn AsyncRead + Send + Unpin>,
    mut ogg: Box<OggOpus>,
    first: Option<Vec<u8>>,
    start: u64,
    chunks: &Chunks,
) -> Result<(), InputError> {
    let mut skip = Skip { start, at: 0 };
    if let Some(data) = first
        && !forward(data, &mut skip, chunks).await
    {
        return Ok(());
    }
    let mut chunk = vec![0; CHUNK];
    let mut ended = false;
    loop {
        while let Some(data) = ogg.packet()? {
            let data = data.to_vec();
            if !forward(data, &mut skip, chunks).await {
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

/// Sends an Opus packet to the player; `false` once the player no longer wants any.
async fn forward(data: Vec<u8>, skip: &mut Skip, chunks: &Chunks) -> bool {
    let Some(samples) = opus::packet_samples(&data) else {
        tracing::debug!(len = data.len(), "skipping a packet that is not Opus");
        return true;
    };
    match skip.keep(samples, chunks).await {
        Some(true) => chunks
            .send(Ok(Chunk::Frame(Frame::Opus { data, samples })))
            .await
            .is_ok(),
        Some(false) => true,
        None => false,
    }
}

/// Reads PCM in 20 ms frames, the last one completed with silence; sends
/// [`Chunk::Start`]`(start)` before the first. Returns `false` once the player no longer
/// wants any.
async fn read_pcm(
    mut reader: impl AsyncRead + Unpin,
    mut start: Option<u64>,
    chunks: &Chunks,
) -> Result<bool, InputError> {
    let mut frame = vec![0; FRAME_BYTES];
    let mut filled = 0;
    loop {
        let read = reader.read(&mut frame[filled..]).await?;
        filled += read;
        if read != 0 && filled < FRAME_BYTES {
            continue;
        }
        if filled == 0 {
            return Ok(true);
        }
        // A short last frame: the rest is silence (zeros).
        frame[filled..].fill(0);
        if let Some(start) = start.take()
            && chunks.send(Ok(Chunk::Start(start))).await.is_err()
        {
            return Ok(false);
        }
        let full = core::mem::replace(&mut frame, vec![0; FRAME_BYTES]);
        if chunks
            .send(Ok(Chunk::Frame(Frame::Pcm(full))))
            .await
            .is_err()
        {
            return Ok(false);
        }
        if read == 0 {
            return Ok(true);
        }
        filled = 0;
    }
}

/// Runs FFmpeg on `source` from `start` and reads its PCM (C-126).
async fn ffmpeg(
    program: &OsStr,
    source: &OsStr,
    start: u64,
    chunks: &Chunks,
) -> Result<(), InputError> {
    let mut command = Command::new(program);
    command.args(["-nostdin", "-hide_banner", "-loglevel", "error"]);
    if start > 0 {
        command.arg("-ss").arg(seconds(start));
    }
    if is_http(source) {
        command.args([
            "-reconnect",
            "1",
            "-reconnect_streamed",
            "1",
            "-reconnect_delay_max",
            "5",
        ]);
    }
    command
        .arg("-i")
        .arg(source)
        .args(["-vn", "-f", "s16le", "-ar", "48000", "-ac", "2", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => InputError::FfmpegMissing,
        _ => InputError::Io(error),
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("FFmpeg's standard output is not piped"))?;
    let stderr = child.stderr.take().map(|stderr| tokio::spawn(tail(stderr)));
    if !read_pcm(stdout, (start > 0).then_some(start), chunks).await? {
        // The track ended: dropping the child kills FFmpeg.
        return Ok(());
    }
    let status = child.wait().await?;
    if status.success() {
        return Ok(());
    }
    let message = match stderr {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };
    Err(InputError::Ffmpeg {
        code: status.code(),
        message,
    })
}

/// The last lines FFmpeg wrote to its standard error.
async fn tail(mut stderr: impl AsyncRead + Unpin) -> String {
    let mut kept = Vec::new();
    let mut chunk = [0; 1024];
    while let Ok(read @ 1..) = stderr.read(&mut chunk).await {
        kept.extend_from_slice(&chunk[..read]);
        if kept.len() > STDERR_TAIL {
            kept.drain(..kept.len() - STDERR_TAIL);
        }
    }
    let text = String::from_utf8_lossy(&kept);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    lines[lines.len().saturating_sub(STDERR_LINES)..].join("\n")
}

/// `samples` at 48 kHz as FFmpeg's seconds, to the microsecond.
fn seconds(samples: u64) -> String {
    format!(
        "{}.{:06}",
        samples / 48_000,
        samples % 48_000 * 1_000_000 / 48_000
    )
}

fn is_http(source: &OsStr) -> bool {
    source.to_str().is_some_and(|source| {
        let source = source.trim_start().to_ascii_lowercase();
        source.starts_with("http://") || source.starts_with("https://")
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tokio::sync::mpsc;

    use super::{Chunk, Frame, Input, Source, is_http, read, seconds};
    use crate::{InputError, codec::FRAME_BYTES};

    const SINE: &[u8] = include_bytes!("../tests/data/sine.opus");

    /// What reading `input` from `start` sends.
    async fn chunks(input: Input, start: u64) -> Vec<Result<Chunk, InputError>> {
        let (tx, mut rx) = mpsc::channel(4);
        let (source, _) = input.into_parts();
        let reader = tokio::spawn(read(source, start, tx));
        let mut chunks = Vec::new();
        while let Some(chunk) = rx.recv().await {
            chunks.push(chunk);
        }
        reader.await.unwrap();
        chunks
    }

    fn frames(chunks: &[Result<Chunk, InputError>]) -> Vec<&Frame> {
        chunks
            .iter()
            .filter_map(|chunk| match chunk {
                Ok(Chunk::Frame(frame)) => Some(frame),
                _ => None,
            })
            .collect()
    }

    /// A directory of its own for a test's files.
    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("djs-voice-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn reads_every_packet_of_a_stream() {
        let chunks = chunks(Input::ogg(SINE), 0).await;
        let frames = frames(&chunks);
        assert_eq!(frames.len(), 51);
        assert_eq!(chunks.len(), 51, "no start, no error");
        assert!(frames.iter().all(|frame| frame.samples() == 960));
        assert!(!Input::ogg(SINE).seekable());
    }

    #[tokio::test]
    async fn files_are_checked_when_opened_and_read_again_from_a_place() {
        let dir = dir("input");
        let good = dir.join("sine.opus");
        std::fs::write(&good, SINE).unwrap();
        let input = Input::ogg_file(&good).await.unwrap();
        assert!(input.seekable());
        assert_eq!(format!("{input:?}"), "Input::OggFile");
        let read = chunks(input, 0).await;
        assert_eq!(
            frames(&read).len(),
            51,
            "the packet read while checking is kept"
        );
        // From 0.5 s: 25 packets of 20 ms skipped. 24 010 is inside the 26th packet.
        let (_, again) = Input::ogg_file(&good).await.unwrap().into_parts();
        for (start, at, left) in [(24_000, 24_000, 26), (24_010, 24_000, 26), (60_000, 0, 0)] {
            let read = chunks(
                Input {
                    source: again.as_ref().unwrap().source(),
                    again: None,
                },
                start,
            )
            .await;
            if left == 0 {
                assert!(read.is_empty(), "past the end");
                continue;
            }
            assert!(matches!(read[0], Ok(Chunk::Start(position)) if position == at));
            assert_eq!(frames(&read).len(), left);
        }
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
        let read = chunks(
            Input {
                source: Source::OggFile(dir.join("missing.opus")),
                again: None,
            },
            0,
        )
        .await;
        assert!(matches!(read[..], [Err(InputError::Io(_))]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn readers_report_inputs_that_are_not_ogg_opus() {
        let chunks = chunks(Input::ogg(&b"plain text"[..]), 0).await;
        assert!(matches!(chunks[..], [Err(InputError::NotOgg)]));
    }

    #[tokio::test]
    async fn reading_stops_when_the_player_does() {
        let (tx, mut rx) = mpsc::channel(1);
        let (source, _) = Input::ogg(SINE).into_parts();
        let reader = tokio::spawn(read(source, 0, tx));
        rx.recv().await.unwrap().unwrap();
        drop(rx);
        reader.await.unwrap();
    }

    #[tokio::test]
    async fn pcm_comes_in_frames_of_20_ms() {
        // Two and a half frames, from a reader that gives a few bytes at a time.
        let bytes: Vec<u8> = (0..FRAME_BYTES * 5 / 2)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let (client, mut server) = tokio::io::duplex(64);
        let writer = {
            let bytes = bytes.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                for piece in bytes.chunks(1000) {
                    server.write_all(piece).await.unwrap();
                }
            })
        };
        let read = chunks(Input::pcm(client), 0).await;
        writer.await.unwrap();
        let frames = frames(&read);
        assert_eq!(frames.len(), 3);
        let Frame::Pcm(last) = frames[2] else {
            panic!("{:?}", frames[2])
        };
        assert_eq!(last.len(), FRAME_BYTES);
        assert_eq!(last[..FRAME_BYTES / 2], bytes[FRAME_BYTES * 2..]);
        assert!(last[FRAME_BYTES / 2..].iter().all(|&byte| byte == 0));
        assert!(chunks(Input::pcm(&b""[..]), 0).await.is_empty());
        assert!(!Input::pcm(&b""[..]).seekable());
    }

    #[test]
    fn ffmpeg_arguments_are_formatted() {
        assert_eq!(seconds(0), "0.000000");
        assert_eq!(seconds(72_000), "1.500000");
        assert_eq!(seconds(48_001), "1.000020");
        assert!(is_http("https://example.com/a.mp3".as_ref()));
        assert!(is_http(" HTTP://example.com".as_ref()));
        assert!(!is_http("music/https.mp3".as_ref()));
    }

    #[tokio::test]
    async fn a_missing_ffmpeg_is_reported() {
        let input = Input::ffmpeg_with("djs-voice-no-such-program", "song.mp3");
        assert!(input.seekable());
        let read = chunks(input, 0).await;
        assert!(matches!(read[..], [Err(InputError::FfmpegMissing)]));
    }

    /// Whether `ffmpeg` runs here; the tests that need it pass without it.
    fn have_ffmpeg() -> bool {
        let found = std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success());
        if !found {
            eprintln!("ffmpeg is not installed: skipped");
        }
        found
    }

    /// A WAV file of `seconds` of a 44.1 kHz mono tone: FFmpeg resamples it.
    fn wav(path: &std::path::Path, seconds: u32) {
        let samples = 44_100 * seconds;
        let mut file = Vec::new();
        file.extend_from_slice(b"RIFF");
        file.extend_from_slice(&(36 + samples * 2).to_le_bytes());
        file.extend_from_slice(b"WAVEfmt ");
        file.extend_from_slice(&16_u32.to_le_bytes());
        file.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        file.extend_from_slice(&1_u16.to_le_bytes()); // mono
        file.extend_from_slice(&44_100_u32.to_le_bytes());
        file.extend_from_slice(&88_200_u32.to_le_bytes());
        file.extend_from_slice(&2_u16.to_le_bytes());
        file.extend_from_slice(&16_u16.to_le_bytes());
        file.extend_from_slice(b"data");
        file.extend_from_slice(&(samples * 2).to_le_bytes());
        for i in 0..samples {
            let value = (f64::from(i) * 440.0 * core::f64::consts::TAU / 44_100.0).sin() * 8000.0;
            #[expect(clippy::cast_possible_truncation, reason = "within ±8000")]
            file.extend_from_slice(&(value as i16).to_le_bytes());
        }
        std::fs::write(path, file).unwrap();
    }

    #[tokio::test]
    async fn ffmpeg_decodes_and_seeks() {
        if !have_ffmpeg() {
            return;
        }
        let dir = dir("ffmpeg");
        let path = dir.join("tone.wav");
        wav(&path, 1);
        // One second: 50 frames of 20 ms.
        let read = chunks(Input::ffmpeg(&path), 0).await;
        assert_eq!(frames(&read).len(), 50, "{read:?}");
        assert!(read.iter().all(Result::is_ok));
        // From 0.5 s: 25 frames.
        let read = chunks(Input::ffmpeg(&path), 24_000).await;
        assert!(matches!(read[0], Ok(Chunk::Start(24_000))));
        assert_eq!(frames(&read).len(), 25);
        // A file that does not exist: FFmpeg says why.
        let read = chunks(Input::ffmpeg(dir.join("missing.mp3")), 0).await;
        match &read[..] {
            [Err(InputError::Ffmpeg { code, message })] => {
                assert_ne!(*code, Some(0));
                assert!(message.contains("missing.mp3"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
