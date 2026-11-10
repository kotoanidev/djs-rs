//! Transport compression.

use std::io;

/// Transport compression applied by Discord to everything it sends on the connection.
///
/// See [transport compression](https://docs.discord.com/developers/events/gateway#transport-compression).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Compression {
    /// No compression: payloads arrive as WebSocket text messages.
    #[cfg_attr(not(feature = "zstd-stream"), default)]
    None,
    /// `zstd-stream`: one zstd decompression context for the whole connection (default).
    ///
    /// Typically reduces gateway bandwidth by 80–90 %.
    #[cfg(feature = "zstd-stream")]
    #[cfg_attr(feature = "zstd-stream", default)]
    ZstdStream,
}

impl Compression {
    /// The value of the `compress` gateway query parameter, if any.
    pub(crate) const fn query_value(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            #[cfg(feature = "zstd-stream")]
            Self::ZstdStream => Some("zstd-stream"),
        }
    }
}

/// Per-connection decompression state.
pub(crate) enum Decompressor {
    #[cfg(feature = "zstd-stream")]
    Zstd(zstd_stream::ZstdDecompressor),
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
        }
    }

    /// Decompresses one WebSocket message into a complete gateway payload.
    #[cfg_attr(not(feature = "zstd-stream"), allow(unused_variables))]
    pub(crate) fn decompress(&mut self, input: &[u8]) -> io::Result<&[u8]> {
        match *self {
            #[cfg(feature = "zstd-stream")]
            Self::Zstd(ref mut zstd) => zstd.decompress(input),
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
