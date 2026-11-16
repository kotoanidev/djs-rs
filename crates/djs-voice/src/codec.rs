//! Opus encoding and decoding (D-350), behind a private module as DAVE is (D-312): the
//! player encodes PCM (`Input::pcm`, `ffmpeg`) and an Ogg/Opus track whose volume is not 1.0,
//! which it decodes first (D-351, D-353).
//!
//! Audio is 48 kHz stereo throughout, as Discord wants it: PCM is interleaved (left, right)
//! `f32` in `-1.0..=1.0` here, converted from the s16le of the inputs.

use opus2::{Application, Bitrate, Channels, SoftClip};

/// The interleaved values of a 20 ms stereo frame: 960 samples per channel.
pub(crate) const FRAME_VALUES: usize = 960 * 2;
/// The bytes of a 20 ms stereo frame of s16le PCM.
pub(crate) const FRAME_BYTES: usize = FRAME_VALUES * 2;
/// The interleaved values of the longest Opus packet, 120 ms.
pub(crate) const MAX_PACKET_VALUES: usize = crate::opus::MAX_PACKET_SAMPLES as usize * 2;
/// Room for any packet the encoder makes: libopus's recommended maximum.
pub(crate) const MAX_PACKET: usize = 4000;
/// The bitrate of encoded audio unless the bot sets one (D-357), as songbird's.
pub(crate) const DEFAULT_BITRATE: u32 = 128_000;
/// The bitrates libopus accepts.
const BITRATES: core::ops::RangeInclusive<u32> = 500..=512_000;
const SAMPLE_RATE: u32 = 48_000;

/// `bits` per second within what libopus accepts.
pub(crate) fn clamp_bitrate(bits: u32) -> u32 {
    bits.clamp(*BITRATES.start(), *BITRATES.end())
}

/// An Opus encoder for a call's audio: 48 kHz stereo, libopus's `Audio` application.
pub(crate) struct Encoder {
    opus: opus2::Encoder,
    /// Keeps audio made louder than its original (volume above 1.0) from clipping harshly.
    clip: SoftClip,
}

impl Encoder {
    /// An encoder at `bitrate` bits per second (clamped to what libopus accepts); `None`
    /// when libopus cannot make one, which it always can with these parameters.
    pub(crate) fn new(bitrate: u32) -> Option<Self> {
        let mut opus = opus2::Encoder::new(SAMPLE_RATE, Channels::Stereo, Application::Audio)
            .inspect_err(|error| tracing::warn!(%error, "could not create an Opus encoder"))
            .ok()?;
        set_bitrate(&mut opus, bitrate);
        Some(Self {
            opus,
            clip: SoftClip::new(Channels::Stereo),
        })
    }

    pub(crate) fn set_bitrate(&mut self, bitrate: u32) {
        set_bitrate(&mut self.opus, bitrate);
    }

    /// Encodes `pcm` (interleaved stereo, a duration Opus can hold: 2.5 to 120 ms) at
    /// `volume` into `out`; returns the packet's length, or `None` when libopus refuses the
    /// duration. `pcm` is scaled in place.
    pub(crate) fn encode(&mut self, pcm: &mut [f32], volume: f32, out: &mut [u8]) -> Option<usize> {
        if (volume - 1.0).abs() > f32::EPSILON {
            for value in pcm.iter_mut() {
                *value *= volume;
            }
        }
        if volume > 1.0 {
            self.clip.apply(pcm);
        }
        self.opus
            .encode_float(pcm, out)
            .inspect_err(|error| tracing::debug!(%error, len = pcm.len(), "Opus encoding failed"))
            .ok()
    }
}

fn set_bitrate(opus: &mut opus2::Encoder, bitrate: u32) {
    // Within `BITRATES`, an `i32`.
    let bits = i32::try_from(clamp_bitrate(bitrate)).unwrap_or(i32::MAX);
    if let Err(error) = opus.set_bitrate(Bitrate::Bits(bits)) {
        tracing::warn!(%error, bits, "could not set the Opus bitrate");
    }
}

/// An Opus decoder for one track: 48 kHz stereo (mono packets are upmixed).
pub(crate) struct Decoder {
    opus: opus2::Decoder,
}

impl Decoder {
    /// A decoder; `None` when libopus cannot make one, which it always can.
    pub(crate) fn new() -> Option<Self> {
        let opus = opus2::Decoder::new(SAMPLE_RATE, Channels::Stereo)
            .inspect_err(|error| tracing::warn!(%error, "could not create an Opus decoder"))
            .ok()?;
        Some(Self { opus })
    }

    /// Decodes `packet` into `out` (room for [`MAX_PACKET_VALUES`]); returns the samples per
    /// channel, or `None` for a packet libopus cannot decode.
    pub(crate) fn decode(&mut self, packet: &[u8], out: &mut [f32]) -> Option<usize> {
        self.opus
            .decode_float(packet, out, false)
            .inspect_err(|error| tracing::debug!(%error, "Opus decoding failed"))
            .ok()
    }
}

/// Converts s16le PCM to `f32` in `-1.0..=1.0`; `out` holds one value per two bytes.
pub(crate) fn from_s16le(bytes: &[u8], out: &mut [f32]) {
    for (value, pair) in out.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *value = f32::from(i16::from_le_bytes(*pair)) / 32_768.0;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Decoder, Encoder, FRAME_BYTES, FRAME_VALUES, MAX_PACKET, MAX_PACKET_VALUES, clamp_bitrate,
        from_s16le,
    };
    use crate::opus::packet_samples;

    /// `samples` per channel of a 440 Hz tone at half the full scale.
    fn tone(samples: usize) -> Vec<f32> {
        (0..samples)
            .flat_map(|i| {
                #[expect(clippy::cast_precision_loss, reason = "small sample indices")]
                let value = (i as f32 * 440.0 * core::f32::consts::TAU / 48_000.0).sin() * 0.5;
                [value, value]
            })
            .collect()
    }

    fn rms(pcm: &[f32]) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "short buffers")]
        let len = pcm.len() as f32;
        (pcm.iter().map(|value| value * value).sum::<f32>() / len).sqrt()
    }

    #[test]
    fn frames_survive_encoding_and_decoding() {
        let mut encoder = Encoder::new(128_000).unwrap();
        let mut decoder = Decoder::new().unwrap();
        let mut packet = [0; MAX_PACKET];
        let mut output = vec![0.0; MAX_PACKET_VALUES];
        let mut last = 0.0;
        // A few frames, so the codec's look-ahead has settled.
        for _ in 0..5 {
            let mut pcm = tone(960);
            let len = encoder.encode(&mut pcm, 1.0, &mut packet).unwrap();
            assert_eq!(packet_samples(&packet[..len]), Some(960));
            assert_eq!(decoder.decode(&packet[..len], &mut output), Some(960));
            last = rms(&output[..FRAME_VALUES]);
        }
        // The tone's RMS is 0.5 / √2.
        assert!((last - 0.354).abs() < 0.05, "rms {last}");
    }

    #[test]
    fn the_volume_scales_the_audio() {
        let mut decoder = Decoder::new().unwrap();
        let mut packet = [0; MAX_PACKET];
        let mut output = vec![0.0; MAX_PACKET_VALUES];
        let mut loudness = |volume| {
            let mut encoder = Encoder::new(128_000).unwrap();
            let mut last = 0.0;
            for _ in 0..5 {
                let mut pcm = tone(960);
                let len = encoder.encode(&mut pcm, volume, &mut packet).unwrap();
                decoder.decode(&packet[..len], &mut output).unwrap();
                last = rms(&output[..FRAME_VALUES]);
            }
            last
        };
        let half = loudness(0.5);
        assert!((half - 0.177).abs() < 0.03, "rms {half}");
        assert!(loudness(0.0) < 0.01);
        // Twice as loud reaches the full scale; the soft clip keeps it within it.
        let double = loudness(2.0);
        assert!(double > 0.5 && double <= 1.0, "rms {double}");
    }

    #[test]
    fn longer_frames_keep_their_duration() {
        let mut encoder = Encoder::new(64_000).unwrap();
        let mut packet = [0; MAX_PACKET];
        for samples in [480, 1920, 2880, 5760] {
            let mut pcm = tone(samples);
            let len = encoder.encode(&mut pcm, 0.5, &mut packet).unwrap();
            assert_eq!(packet_samples(&packet[..len]), u32::try_from(samples).ok());
        }
        // 7.5 ms is not a duration Opus can hold.
        assert_eq!(encoder.encode(&mut tone(360), 1.0, &mut packet), None);
    }

    #[test]
    fn bitrates_stay_within_what_libopus_accepts() {
        assert_eq!(clamp_bitrate(0), 500);
        assert_eq!(clamp_bitrate(96_000), 96_000);
        assert_eq!(clamp_bitrate(u32::MAX), 512_000);
        let mut encoder = Encoder::new(u32::MAX).unwrap();
        encoder.set_bitrate(8_000);
        let mut packet = [0; MAX_PACKET];
        let len = encoder.encode(&mut tone(960), 1.0, &mut packet).unwrap();
        // 8 kb/s for 20 ms: about 20 bytes.
        assert!(len < 60, "{len} bytes");
    }

    #[test]
    fn s16le_becomes_floats() {
        let mut bytes = [0; FRAME_BYTES];
        bytes[..6].copy_from_slice(&[0x00, 0x80, 0xFF, 0x7F, 0x00, 0x40]);
        let mut out = [1.0; FRAME_VALUES];
        from_s16le(&bytes, &mut out);
        assert_eq!(out[..4], [-1.0, 32_767.0 / 32_768.0, 0.5, 0.0]);
    }

    #[test]
    fn damaged_packets_are_refused() {
        let mut decoder = Decoder::new().unwrap();
        let mut output = vec![0.0; MAX_PACKET_VALUES];
        // A code-3 packet that claims more frames than it has.
        assert_eq!(decoder.decode(&[0xFB, 0x3F], &mut output), None);
    }
}
