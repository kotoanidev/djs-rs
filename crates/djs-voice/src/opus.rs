//! What djs-voice reads from Opus packets without decoding them: their duration, from the
//! table of contents byte (RFC 6716, section 3.1). The player paces packets sent unchanged
//! (Ogg/Opus input, D-317) by it.

#![allow(dead_code, reason = "used by the player from step 1e")]

/// The samples per channel at 48 kHz of one frame, by the frame-size bits of the
/// configuration: SILK-only (10, 20, 40 and 60 ms), hybrid (10 and 20 ms) and CELT-only
/// (2.5, 5, 10 and 20 ms).
const SILK: [u32; 4] = [480, 960, 1920, 2880];
const HYBRID: [u32; 2] = [480, 960];
const CELT: [u32; 4] = [120, 240, 480, 960];

/// The longest a packet may last: 120 ms at 48 kHz.
pub(crate) const MAX_PACKET_SAMPLES: u32 = 5760;

/// The duration of an Opus packet in samples per channel at 48 kHz (960 for 20 ms), read
/// from its table of contents byte; `None` for a packet that cannot be valid (empty, no frame
/// count, or longer than 120 ms).
pub(crate) fn packet_samples(packet: &[u8]) -> Option<u32> {
    let &toc = packet.first()?;
    let config = toc >> 3;
    let frame = match config {
        0..=11 => SILK[usize::from(config & 0b11)],
        12..=15 => HYBRID[usize::from(config & 0b1)],
        _ => CELT[usize::from(config & 0b11)],
    };
    let frames = match toc & 0b11 {
        0 => 1,
        1 | 2 => 2,
        // Code 3: the frame count is in the low six bits of the second byte.
        _ => u32::from(*packet.get(1)? & 0b11_1111),
    };
    let samples = frame * frames;
    (frames > 0 && samples <= MAX_PACKET_SAMPLES).then_some(samples)
}

#[cfg(test)]
mod tests {
    use super::packet_samples;

    /// A table of contents byte: the configuration, the stereo flag and the frame-count code.
    fn toc(config: u8, stereo: bool, code: u8) -> u8 {
        (config << 3) | (u8::from(stereo) << 2) | code
    }

    #[test]
    fn durations_follow_the_table_of_contents() {
        // (TOC byte, second byte, samples): one entry per frame size and frame-count code.
        let cases: [(u8, u8, Option<u32>); 16] = [
            (toc(0, false, 0), 0, Some(480)),   // SILK narrowband 10 ms
            (toc(1, false, 0), 0, Some(960)),   // SILK narrowband 20 ms
            (toc(2, false, 0), 0, Some(1920)),  // SILK 40 ms
            (toc(3, false, 0), 0, Some(2880)),  // SILK 60 ms
            (toc(12, false, 0), 0, Some(480)),  // hybrid 10 ms
            (toc(13, false, 0), 0, Some(960)),  // hybrid 20 ms
            (toc(16, false, 0), 0, Some(120)),  // CELT 2.5 ms
            (toc(17, false, 0), 0, Some(240)),  // CELT 5 ms
            (toc(18, false, 0), 0, Some(480)),  // CELT 10 ms
            (toc(31, true, 0), 0, Some(960)),   // CELT fullband 20 ms, stereo
            (toc(31, true, 1), 0, Some(1920)),  // two frames of equal size
            (toc(31, false, 2), 0, Some(1920)), // two frames of different sizes
            (toc(31, true, 3), 6, Some(5760)),  // six frames of 20 ms: the 120 ms limit
            (toc(31, true, 3), 7, None),        // seven frames of 20 ms: too long
            (toc(3, false, 3), 2, Some(5760)),  // two SILK frames of 60 ms
            (toc(16, false, 3), 0, None),       // code 3 without frames
        ];
        for (toc, count, samples) in cases {
            assert_eq!(packet_samples(&[toc, count]), samples, "TOC {toc:#010b}");
        }
        assert_eq!(packet_samples(&[]), None);
        assert_eq!(
            packet_samples(&[toc(31, true, 3)]),
            None,
            "code 3 needs a second byte"
        );
        assert_eq!(
            packet_samples(&[toc(31, true, 0)]),
            Some(960),
            "codes 0 to 2 need no second byte"
        );
    }
}
