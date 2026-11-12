//! Transport compression.

use std::io;

/// Transport compression applied by Discord to everything it sends on the connection.
///
/// See [transport compression](https://docs.discord.com/developers/events/gateway#transport-compression).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Compression {
    /// No compression: payloads arrive as WebSocket text messages. The default when no
    /// compression feature is enabled.
    #[cfg_attr(not(any(feature = "zstd-stream", feature = "zlib-stream")), default)]
    None,
    /// `zstd-stream`: one zstd decompression context for the whole connection (the default
    /// with the `zstd-stream` feature).
    ///
    /// Typically reduces gateway bandwidth by 80–90 %.
    #[cfg(feature = "zstd-stream")]
    #[cfg_attr(feature = "zstd-stream", default)]
    ZstdStream,
    /// `zlib-stream`: one zlib inflate context for the whole connection, decoded with the
    /// pure-Rust `zlib-rs` (the default with the `zlib-stream` feature alone).
    ///
    /// Compresses less than zstd and decodes more slowly — in the stress tests, with Discord's
    /// parameters, 29× instead of 39× on small events and 10–30 % fewer events per second —
    /// but needs no C compiler and holds less memory per shard (~50 KiB instead of ~150 KiB).
    ///
    /// discord.js counterpart: `CompressionMethod.ZlibNative` / `ZlibSync`.
    #[cfg(feature = "zlib-stream")]
    #[cfg_attr(all(feature = "zlib-stream", not(feature = "zstd-stream")), default)]
    ZlibStream,
}

impl Compression {
    /// The value of the `compress` gateway query parameter, if any.
    pub(crate) const fn query_value(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            #[cfg(feature = "zstd-stream")]
            Self::ZstdStream => Some("zstd-stream"),
            #[cfg(feature = "zlib-stream")]
            Self::ZlibStream => Some("zlib-stream"),
        }
    }
}

/// Per-connection decompression state.
pub(crate) enum Decompressor {
    #[cfg(feature = "zstd-stream")]
    Zstd(zstd_stream::ZstdDecompressor),
    #[cfg(feature = "zlib-stream")]
    Zlib(zlib_stream::ZlibDecompressor),
}

impl Decompressor {
    /// Creates the decompressor for a new connection, or `None` without compression.
    #[cfg_attr(
        not(feature = "zstd-stream"),
        expect(
            clippy::unnecessary_wraps,
            reason = "fallible with compression features"
        )
    )]
    pub(crate) fn new(compression: Compression) -> io::Result<Option<Self>> {
        match compression {
            Compression::None => Ok(None),
            #[cfg(feature = "zstd-stream")]
            Compression::ZstdStream => Ok(Some(Self::Zstd(zstd_stream::ZstdDecompressor::new()?))),
            #[cfg(feature = "zlib-stream")]
            Compression::ZlibStream => Ok(Some(Self::Zlib(zlib_stream::ZlibDecompressor::new()))),
        }
    }

    /// Decompresses one WebSocket message. Returns the complete gateway payload, or `None` if
    /// the message is only the first part of one (possible with `zlib-stream`).
    #[cfg_attr(
        not(any(feature = "zstd-stream", feature = "zlib-stream")),
        allow(unused_variables)
    )]
    pub(crate) fn decompress(&mut self, input: &[u8]) -> io::Result<Option<&[u8]>> {
        match *self {
            #[cfg(feature = "zstd-stream")]
            Self::Zstd(ref mut zstd) => zstd.decompress(input).map(Some),
            #[cfg(feature = "zlib-stream")]
            Self::Zlib(ref mut zlib) => zlib.decompress(input),
        }
    }
}

#[cfg(feature = "zstd-stream")]
mod zstd_stream {
    use std::io;

    use zstd::stream::raw::{Decoder, InBuffer, Operation, OutBuffer};

    /// Minimum free space ensured in the output buffer before each decompression step.
    const CHUNK: usize = 32 * 1024;
    /// Buffers larger than this are shrunk back after the payload has been handed out, so one
    /// huge `GUILD_CREATE` does not pin megabytes of memory forever.
    const MAX_RETAINED: usize = 1024 * 1024;

    pub(crate) struct ZstdDecompressor {
        decoder: Decoder<'static>,
        buffer: Vec<u8>,
    }

    impl ZstdDecompressor {
        pub(crate) fn new() -> io::Result<Self> {
            Ok(Self {
                decoder: Decoder::new()?,
                buffer: Vec::with_capacity(CHUNK),
            })
        }

        /// Each WebSocket message holds exactly one gateway payload but does not end a zstd
        /// frame, so the decoder is driven until the input is consumed and no output remains.
        pub(crate) fn decompress(&mut self, input: &[u8]) -> io::Result<&[u8]> {
            if self.buffer.capacity() > MAX_RETAINED {
                self.buffer = Vec::with_capacity(CHUNK);
            }
            self.buffer.clear();

            let mut input = InBuffer::around(input);
            loop {
                if self.buffer.capacity() - self.buffer.len() < CHUNK {
                    self.buffer.reserve(CHUNK);
                }
                let position = self.buffer.len();
                let mut output = OutBuffer::around_pos(&mut self.buffer, position);
                self.decoder.run(&mut input, &mut output)?;
                let output_full = output.pos() == output.capacity();
                if input.pos() == input.src.len() && !output_full {
                    break;
                }
            }
            Ok(&self.buffer)
        }
    }

    #[cfg(test)]
    mod tests {
        use zstd::stream::raw::Encoder;

        use super::*;

        /// Compresses messages the way Discord does: one long-lived stream, flushed (but not
        /// finished) after every message.
        struct StreamCompressor(Encoder<'static>);

        impl StreamCompressor {
            fn compress(&mut self, data: &[u8]) -> Vec<u8> {
                let mut out = Vec::with_capacity(data.len() + 64);
                let mut input = InBuffer::around(data);
                loop {
                    out.reserve(1024);
                    let position = out.len();
                    let mut output = OutBuffer::around_pos(&mut out, position);
                    self.0.run(&mut input, &mut output).unwrap();
                    if input.pos() == input.src.len() {
                        break;
                    }
                }
                loop {
                    out.reserve(1024);
                    let position = out.len();
                    let mut output = OutBuffer::around_pos(&mut out, position);
                    if self.0.flush(&mut output).unwrap() == 0 {
                        break;
                    }
                }
                out
            }
        }

        #[test]
        fn stream_of_messages() {
            let mut compressor = StreamCompressor(Encoder::new(3).unwrap());
            let mut decompressor = ZstdDecompressor::new().unwrap();
            for i in 0..50 {
                let payload = format!(
                    r#"{{"op":0,"s":{i},"t":"X","d":{{"n":"{}"}}}}"#,
                    "y".repeat(i * 37)
                );
                let compressed = compressor.compress(payload.as_bytes());
                assert_eq!(
                    decompressor.decompress(&compressed).unwrap(),
                    payload.as_bytes()
                );
            }
        }

        #[test]
        fn large_payload_spans_many_chunks() {
            let mut compressor = StreamCompressor(Encoder::new(3).unwrap());
            let mut decompressor = ZstdDecompressor::new().unwrap();
            let payload: Vec<u8> = (0..2_000_000_u32).map(|i| (i % 251) as u8).collect();
            let compressed = compressor.compress(&payload);
            assert_eq!(decompressor.decompress(&compressed).unwrap(), &payload[..]);
            // The oversized buffer is released on the next message.
            let small = compressor.compress(b"{}");
            assert_eq!(decompressor.decompress(&small).unwrap(), b"{}");
            assert!(decompressor.buffer.capacity() <= MAX_RETAINED);
        }

        #[test]
        fn garbage_is_an_error() {
            let mut decompressor = ZstdDecompressor::new().unwrap();
            assert!(decompressor.decompress(b"definitely not zstd").is_err());
        }
    }
}

#[cfg(feature = "zlib-stream")]
mod zlib_stream {
    use core::mem;
    use std::io;

    use flate2::{Decompress, FlushDecompress, Status};

    /// Every complete payload ends with the empty stored block of a `Z_SYNC_FLUSH`.
    const SYNC_FLUSH_SUFFIX: [u8; 4] = [0x00, 0x00, 0xff, 0xff];
    /// Minimum free space ensured in the output buffer before each inflate step.
    const CHUNK: usize = 32 * 1024;
    /// Buffers larger than this are shrunk back after use, so one huge `GUILD_CREATE` does not
    /// pin megabytes of memory forever.
    const MAX_RETAINED: usize = 1024 * 1024;

    pub(crate) struct ZlibDecompressor {
        inflate: Decompress,
        /// The compressed start of a payload split over several WebSocket messages.
        pending: Vec<u8>,
        buffer: Vec<u8>,
    }

    impl ZlibDecompressor {
        pub(crate) fn new() -> Self {
            Self {
                // Discord's stream starts with a zlib header.
                inflate: Decompress::new(true),
                pending: Vec::new(),
                buffer: Vec::with_capacity(CHUNK),
            }
        }

        /// Messages are buffered until one ends with the sync-flush suffix; the collected
        /// bytes then inflate to exactly one gateway payload. In practice Discord sends one
        /// payload per message, which is inflated straight from the message without copying.
        pub(crate) fn decompress(&mut self, input: &[u8]) -> io::Result<Option<&[u8]>> {
            let complete = input.ends_with(&SYNC_FLUSH_SUFFIX);
            if self.pending.is_empty() && complete {
                self.inflate_payload(input)?;
                return Ok(Some(&self.buffer));
            }
            self.pending.extend_from_slice(input);
            if !complete {
                return Ok(None);
            }
            let mut pending = mem::take(&mut self.pending);
            let result = self.inflate_payload(&pending);
            pending.clear();
            if pending.capacity() <= MAX_RETAINED {
                self.pending = pending;
            }
            result?;
            Ok(Some(&self.buffer))
        }

        fn inflate_payload(&mut self, input: &[u8]) -> io::Result<()> {
            if self.buffer.capacity() > MAX_RETAINED {
                self.buffer = Vec::with_capacity(CHUNK);
            }
            self.buffer.clear();

            let mut consumed = 0;
            loop {
                if self.buffer.capacity() - self.buffer.len() < CHUNK {
                    self.buffer.reserve(CHUNK);
                }
                let (in_before, out_before) = (self.inflate.total_in(), self.buffer.len());
                let status = self
                    .inflate
                    .decompress_vec(&input[consumed..], &mut self.buffer, FlushDecompress::Sync)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                // At most `input.len()`, so it fits.
                consumed += usize::try_from(self.inflate.total_in() - in_before)
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
                let output_full = self.buffer.len() == self.buffer.capacity();
                if consumed == input.len() && !output_full {
                    return Ok(());
                }
                let progressed =
                    self.inflate.total_in() != in_before || self.buffer.len() != out_before;
                if status == Status::StreamEnd || !progressed {
                    // Discord never ends the stream; trailing bytes mean a corrupt payload.
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "zlib-stream payload did not inflate completely",
                    ));
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use flate2::{Compress, Compression, FlushCompress};

        use super::*;

        /// Compresses messages the way Discord does: one long-lived zlib stream, sync-flushed
        /// after every payload.
        struct StreamCompressor(Compress);

        impl StreamCompressor {
            fn new() -> Self {
                Self(Compress::new(Compression::default(), true))
            }

            fn compress(&mut self, data: &[u8]) -> Vec<u8> {
                let mut out = Vec::with_capacity(data.len() / 2 + 64);
                let start = self.0.total_in();
                loop {
                    out.reserve(1024);
                    let read = usize::try_from(self.0.total_in() - start).unwrap();
                    self.0
                        .compress_vec(&data[read..], &mut out, FlushCompress::Sync)
                        .unwrap();
                    let read = usize::try_from(self.0.total_in() - start).unwrap();
                    if read == data.len() && out.len() < out.capacity() {
                        break;
                    }
                }
                assert!(out.ends_with(&SYNC_FLUSH_SUFFIX));
                out
            }
        }

        fn payload(i: usize) -> String {
            format!(
                r#"{{"op":0,"s":{i},"t":"X","d":{{"n":"{}"}}}}"#,
                "y".repeat(i * 37)
            )
        }

        #[test]
        fn stream_of_messages() {
            let mut compressor = StreamCompressor::new();
            let mut decompressor = ZlibDecompressor::new();
            for i in 0..50 {
                let payload = payload(i);
                let compressed = compressor.compress(payload.as_bytes());
                assert_eq!(
                    decompressor.decompress(&compressed).unwrap(),
                    Some(payload.as_bytes())
                );
            }
        }

        #[test]
        fn payload_split_over_messages() {
            let mut compressor = StreamCompressor::new();
            let mut decompressor = ZlibDecompressor::new();
            for i in 0..20 {
                let payload = payload(i * 3);
                let compressed = compressor.compress(payload.as_bytes());
                let (head, tail) = compressed.split_at(compressed.len() / 2);
                assert_eq!(decompressor.decompress(head).unwrap(), None);
                assert_eq!(
                    decompressor.decompress(tail).unwrap(),
                    Some(payload.as_bytes())
                );
            }
            assert_eq!(decompressor.pending, [] as [u8; 0]);
        }

        #[test]
        fn large_payload_spans_many_chunks() {
            let mut compressor = StreamCompressor::new();
            let mut decompressor = ZlibDecompressor::new();
            let payload: Vec<u8> = (0..2_000_000_u32).map(|i| (i % 251) as u8).collect();
            let compressed = compressor.compress(&payload);
            assert_eq!(
                decompressor.decompress(&compressed).unwrap(),
                Some(&payload[..])
            );
            // The oversized buffer is released on the next message.
            let small = compressor.compress(b"{}");
            assert_eq!(decompressor.decompress(&small).unwrap(), Some(&b"{}"[..]));
            assert!(decompressor.buffer.capacity() <= MAX_RETAINED);
        }

        #[test]
        fn garbage_is_an_error() {
            let mut decompressor = ZlibDecompressor::new();
            let mut garbage = b"definitely not zlib".to_vec();
            garbage.extend_from_slice(&SYNC_FLUSH_SUFFIX);
            assert!(decompressor.decompress(&garbage).is_err());
        }
    }
}
