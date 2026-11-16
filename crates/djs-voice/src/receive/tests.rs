use core::time::Duration;

use djs_model::id::UserId;
use tokio::{sync::mpsc, time::Instant};

use super::{
    EndBehavior, Item, QUEUE, ReceiveOptions, Receiving, Reorder, SILENCE, VoiceData, VoicePacket,
    VoiceStream,
};
use crate::{ogg::OggOpus, udp::Rtp};

const SINE: &[u8] = include_bytes!("../../tests/data/sine.opus");
const USER: u64 = 6;
/// Nothing passed on.
const NONE: [i32; 0] = [];
/// Nobody stopped speaking.
const NOBODY: [UserId; 0] = [];
const SSRC: u32 = 77;

fn sine() -> Vec<Vec<u8>> {
    let mut ogg = OggOpus::new();
    ogg.push(SINE);
    ogg.finish();
    let mut packets = Vec::new();
    while let Some(packet) = ogg.packet().unwrap() {
        packets.push(packet.to_vec());
    }
    packets
}

fn user() -> UserId {
    UserId::new(USER)
}

/// How [`order`] shows a lost packet.
const fn lost(sequence: i32) -> i32 {
    -sequence - 1
}

/// The sequence numbers of `items`, lost ones as [`lost`] shows them.
fn order(items: &[Item]) -> Vec<i32> {
    items
        .iter()
        .map(|item| match item {
            Item::Packet { sequence, .. } => i32::from(*sequence),
            Item::Lost { sequence, .. } => lost(i32::from(*sequence)),
        })
        .collect()
}

/// Pushes packet `sequence` (20 ms each, timestamps from it) into `reorder`; what it passes
/// on.
fn push(reorder: &mut Reorder, sequence: u16, now: Instant) -> Vec<Item> {
    let mut out = Vec::new();
    let timestamp = u32::from(sequence) * 960;
    reorder.push(sequence, timestamp, &[0xFC, 1, 2], now, &mut out);
    out
}

#[test]
fn packets_in_order_pass_at_once() {
    let now = Instant::now();
    let mut reorder = Reorder::default();
    for sequence in 10..13 {
        assert_eq!(
            order(&push(&mut reorder, sequence, now)),
            [i32::from(sequence)]
        );
    }
    assert_eq!(reorder.deadline(), None);
}

#[test]
fn swapped_packets_are_put_back_in_order() {
    let now = Instant::now();
    let mut reorder = Reorder::default();
    assert_eq!(order(&push(&mut reorder, 1, now)), [1]);
    assert_eq!(order(&push(&mut reorder, 3, now)), NONE, "held for the 2nd");
    assert_eq!(reorder.deadline(), Some(now + Duration::from_millis(60)));
    assert_eq!(order(&push(&mut reorder, 2, now)), [2, 3]);
    assert_eq!(reorder.deadline(), None);
    // A duplicate, and one older than what was passed on.
    assert_eq!(order(&push(&mut reorder, 3, now)), NONE);
    assert_eq!(order(&push(&mut reorder, 1, now)), NONE);
    assert_eq!(order(&push(&mut reorder, 4, now)), [4]);
}

#[test]
fn missing_packets_are_given_up_after_60_ms() {
    let now = Instant::now();
    let mut reorder = Reorder::default();
    push(&mut reorder, 1, now);
    assert_eq!(order(&push(&mut reorder, 3, now)), NONE);
    assert_eq!(
        order(&push(&mut reorder, 5, now + Duration::from_millis(20))),
        NONE
    );
    // Waited for from the first packet held on.
    assert_eq!(reorder.deadline(), Some(now + Duration::from_millis(60)));
    let mut out = Vec::new();
    reorder.flush(&mut out);
    assert_eq!(order(&out), [lost(2), 3, lost(4), 5]);
    // A lost packet's timestamp follows the one before.
    assert_eq!(
        out[0],
        Item::Lost {
            sequence: 2,
            timestamp: 2 * 960,
            samples: 960
        }
    );
    assert_eq!(reorder.deadline(), None);
    // The lost one coming late is dropped.
    assert_eq!(order(&push(&mut reorder, 2, now)), NONE);
    assert_eq!(order(&push(&mut reorder, 6, now)), [6]);
}

#[test]
fn sequence_numbers_wrap() {
    let now = Instant::now();
    let mut reorder = Reorder::default();
    assert_eq!(order(&push(&mut reorder, 65_534, now)), [65_534]);
    assert_eq!(order(&push(&mut reorder, 0, now)), NONE);
    assert_eq!(order(&push(&mut reorder, 65_535, now)), [65_535, 0]);
    assert_eq!(order(&push(&mut reorder, 1, now)), [1]);
}

#[test]
fn a_far_jump_is_a_new_stream() {
    let now = Instant::now();
    let mut reorder = Reorder::default();
    push(&mut reorder, 100, now);
    assert_eq!(order(&push(&mut reorder, 102, now)), NONE);
    // Not 1 900 lost packets: what is held goes, then the new stream.
    assert_eq!(
        order(&push(&mut reorder, 2_000, now)),
        [lost(101), 102, 2_000]
    );
    assert_eq!(order(&push(&mut reorder, 2_001, now)), [2_001]);
    // Far behind is new too.
    assert_eq!(order(&push(&mut reorder, 50, now)), [50]);
}

fn rtp(sequence: u16) -> Rtp {
    Rtp {
        sequence,
        timestamp: u32::from(sequence) * 960,
        ssrc: SSRC,
    }
}

fn stream(receiving: &mut Receiving, options: ReceiveOptions) -> VoiceStream {
    let (tx, rx) = mpsc::channel(QUEUE);
    receiving.subscribe(user(), options, tx);
    VoiceStream::new(rx, options)
}

/// Packet `sequence` of `opus` arrives at `now`; whether it was wanted.
fn arrive(receiving: &mut Receiving, sequence: u16, opus: &[u8], now: Instant) -> bool {
    let arrival = receiving.arrived(rtp(sequence), now).expect("a known user");
    assert_eq!(arrival.user_id, user());
    if arrival.wanted {
        receiving.opened(arrival.user_id, rtp(sequence), opus, now);
    }
    arrival.wanted
}

fn received(stream: &mut VoiceStream) -> Vec<VoicePacket> {
    let mut packets = Vec::new();
    while let Ok(packet) = stream.try_next() {
        packets.push(packet);
    }
    packets
}

#[test]
fn speaking_starts_with_a_packet_and_stops_100_ms_after_the_last() {
    let now = Instant::now();
    let mut receiving = Receiving::default();
    assert!(receiving.arrived(rtp(1), now).is_none(), "an unknown SSRC");
    receiving.map(user(), SSRC, now);
    let first = receiving.arrived(rtp(1), now).unwrap();
    assert!(first.started && !first.wanted);
    let later = now + Duration::from_millis(20);
    assert!(!receiving.arrived(rtp(2), later).unwrap().started);
    let stop = later + Duration::from_millis(100);
    assert_eq!(receiving.deadline(), Some(stop));
    assert_eq!(receiving.tick(stop - Duration::from_millis(1)), NOBODY);
    assert_eq!(receiving.tick(stop), [user()]);
    assert_eq!(receiving.deadline(), None);
    assert!(receiving.arrived(rtp(3), stop).unwrap().started, "again");
    // Gone from the channel: their SSRC means nothing.
    receiving.left(user());
    assert!(receiving.arrived(rtp(4), stop).is_none());
}

#[test]
fn each_subscription_gets_the_packets_as_it_asked() {
    let now = Instant::now();
    let packets = sine();
    let mut receiving = Receiving::default();
    receiving.map(user(), SSRC, now);
    let mut opus = stream(&mut receiving, ReceiveOptions::new());
    let mut pcm = stream(&mut receiving, ReceiveOptions::new().decode(true));
    for (sequence, packet) in (1..).zip(&packets[..3]) {
        assert!(arrive(&mut receiving, sequence, packet, now));
    }
    // The 4th is lost, the 5th comes.
    assert!(arrive(&mut receiving, 5, &packets[4], now));
    let opus_packets = received(&mut opus);
    assert_eq!(opus_packets.len(), 3, "the 5th waits for the 4th");
    for (packet, sent) in opus_packets.iter().zip(&packets) {
        assert_eq!(packet.audio, VoiceData::Opus(sent.clone()));
    }
    assert_eq!(opus_packets[1].sequence, 2);
    assert_eq!(opus_packets[1].timestamp, 2 * 960);
    let given_up = now + Duration::from_millis(60);
    assert_eq!(receiving.deadline(), Some(given_up));
    assert_eq!(receiving.tick(given_up), NOBODY);
    let rest = received(&mut opus);
    assert_eq!(rest[0].audio, VoiceData::Lost);
    assert_eq!((rest[0].sequence, rest[0].timestamp), (4, 4 * 960));
    assert_eq!(rest[1].audio, VoiceData::Opus(packets[4].clone()));
    // PCM: 20 ms of stereo each, the lost one rebuilt.
    let pcm_packets = received(&mut pcm);
    assert_eq!(pcm_packets.len(), 5);
    for packet in &pcm_packets {
        assert!(matches!(&packet.audio, VoiceData::Pcm(samples) if samples.len() == 1920));
    }
    let VoiceData::Pcm(rebuilt) = &pcm_packets[3].audio else {
        unreachable!()
    };
    assert!(
        rebuilt.iter().any(|sample| *sample != 0),
        "concealed, not silent"
    );
}

#[test]
fn a_decoding_stream_waits_for_the_packet_after_a_lost_one() {
    let packets = sine();
    let (tx, rx) = mpsc::channel(QUEUE);
    let mut stream = VoiceStream::new(rx, ReceiveOptions::new().decode(true));
    let packet = |sequence: u16| Item::Packet {
        sequence,
        timestamp: u32::from(sequence) * 960,
        opus: packets[usize::from(sequence) + 30].clone(),
    };
    let lost = |sequence: u16| Item::Lost {
        sequence,
        timestamp: u32::from(sequence) * 960,
        samples: 960,
    };
    let decoded = |packet: Result<VoicePacket, mpsc::error::TryRecvError>| {
        let packet = packet.expect("a packet");
        let VoiceData::Pcm(samples) = packet.audio else {
            panic!("not decoded: {:?}", packet.audio);
        };
        assert_eq!(samples.len(), 1920, "packet {}", packet.sequence);
        packet.sequence
    };
    tx.try_send(packet(1)).unwrap();
    tx.try_send(lost(2)).unwrap();
    assert_eq!(decoded(stream.try_next()), 1);
    // The lost packet waits for the one after it, which may carry it.
    assert_eq!(stream.try_next(), Err(mpsc::error::TryRecvError::Empty));
    tx.try_send(packet(3)).unwrap();
    assert_eq!(decoded(stream.try_next()), 2);
    assert_eq!(decoded(stream.try_next()), 3);
    // Followed by a packet that is not the next one (5 dropped for a slow reader): 4 is
    // concealed. A lost packet at the end, too.
    tx.try_send(lost(4)).unwrap();
    tx.try_send(packet(6)).unwrap();
    tx.try_send(lost(7)).unwrap();
    drop(tx);
    assert_eq!(decoded(stream.try_next()), 4);
    assert_eq!(decoded(stream.try_next()), 6);
    assert_eq!(decoded(stream.try_next()), 7);
    assert_eq!(
        stream.try_next(),
        Err(mpsc::error::TryRecvError::Disconnected)
    );
}

#[test]
fn a_dropped_stream_unsubscribes() {
    let now = Instant::now();
    let packets = sine();
    let mut receiving = Receiving::default();
    receiving.map(user(), SSRC, now);
    let kept = stream(&mut receiving, ReceiveOptions::new());
    drop(stream(&mut receiving, ReceiveOptions::new()));
    assert!(arrive(&mut receiving, 1, &packets[0], now));
    drop(kept);
    // Noticed when sending to it.
    assert!(arrive(&mut receiving, 2, &packets[1], now));
    assert!(
        !arrive(&mut receiving, 3, &packets[2], now),
        "nobody receives"
    );
}

#[test]
fn streams_end_after_silence_or_inactivity() {
    let now = Instant::now();
    let packets = sine();
    let second = Duration::from_secs(1);
    let mut receiving = Receiving::default();
    receiving.map(user(), SSRC, now);
    let mut silence = stream(
        &mut receiving,
        ReceiveOptions::new().end(EndBehavior::AfterSilence(second)),
    );
    let mut inactivity = stream(
        &mut receiving,
        ReceiveOptions::new().end(EndBehavior::AfterInactivity(second)),
    );
    let mut manual = stream(&mut receiving, ReceiveOptions::new());
    // Nobody spoke yet: no stream ends.
    assert_eq!(receiving.deadline(), None);
    arrive(&mut receiving, 1, &packets[0], now);
    let quiet = now + Duration::from_millis(100);
    arrive(&mut receiving, 2, &SILENCE, quiet);
    arrive(
        &mut receiving,
        3,
        &SILENCE,
        quiet + Duration::from_millis(20),
    );
    receiving.tick(now + second);
    assert_eq!(received(&mut silence).len(), 3);
    assert_eq!(
        silence.try_next(),
        Err(mpsc::error::TryRecvError::Disconnected)
    );
    assert_eq!(received(&mut inactivity).len(), 3);
    assert_eq!(inactivity.try_next(), Err(mpsc::error::TryRecvError::Empty));
    receiving.tick(quiet + Duration::from_millis(20) + second);
    assert_eq!(
        inactivity.try_next(),
        Err(mpsc::error::TryRecvError::Disconnected)
    );
    assert_eq!(received(&mut manual).len(), 3);
    assert_eq!(manual.try_next(), Err(mpsc::error::TryRecvError::Empty));
}

#[test]
fn a_new_ssrc_or_connection_starts_over() {
    let now = Instant::now();
    let packets = sine();
    let mut receiving = Receiving::default();
    receiving.map(user(), SSRC, now);
    let mut opus = stream(&mut receiving, ReceiveOptions::new());
    arrive(&mut receiving, 1, &packets[0], now);
    arrive(&mut receiving, 3, &packets[2], now);
    // Another SSRC: the held packet goes, the new source's numbers are not compared.
    receiving.map(user(), SSRC + 1, now);
    assert!(receiving.arrived(rtp(4), now).is_none(), "the old SSRC");
    let arrival = receiving
        .arrived(
            Rtp {
                ssrc: SSRC + 1,
                ..rtp(500)
            },
            now,
        )
        .unwrap();
    receiving.opened(arrival.user_id, rtp(500), &packets[3], now);
    let sequences: Vec<u16> = received(&mut opus)
        .iter()
        .map(|packet| packet.sequence)
        .collect();
    assert_eq!(sequences, [1, 2, 3, 500]);
    // A new connection: nobody is known, the speaking stop.
    assert_eq!(receiving.reset(), [user()]);
    assert!(receiving.arrived(rtp(501), now).is_none());
    assert_eq!(receiving.deadline(), None);
    // The subscription stays, for when the user is known again.
    receiving.map(user(), SSRC, now);
    assert!(arrive(&mut receiving, 9, &packets[4], now));
}

#[test]
fn options_default_to_opus_and_manual() {
    assert_eq!(ReceiveOptions::default(), ReceiveOptions::new());
    assert_eq!(EndBehavior::default(), EndBehavior::Manual);
    let options = ReceiveOptions::new()
        .decode(true)
        .end(EndBehavior::AfterInactivity(Duration::ZERO));
    assert!(options.decode);
    assert_eq!(options.end, EndBehavior::AfterInactivity(Duration::ZERO));
}

/// Step 4's measurement of receiving (C-129): `cargo test --release -p djs-voice --lib
/// receive::tests::throughput -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement; run it in release"]
fn throughput() {
    use std::time::Instant as Clock;

    let packets = sine();
    let now = Instant::now();
    let count: u32 = 200_000;
    let best = |run: &mut dyn FnMut(u32)| {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let started = Clock::now();
            run(count);
            best = best.min(started.elapsed().as_secs_f64());
        }
        best / f64::from(count) * 1e9
    };
    // The reorder buffer alone: in order, then every 50th pair swapped.
    let mut out = Vec::new();
    for swapped in [false, true] {
        let per = best(&mut |count| {
            let mut reorder = Reorder::default();
            for n in 0..count {
                let sequence = u16::try_from(n % 65_536).unwrap();
                let sequence = match (swapped, n % 50) {
                    (true, 10) => sequence.wrapping_add(1),
                    (true, 11) => sequence.wrapping_sub(1),
                    _ => sequence,
                };
                let opus = &packets[usize::try_from(n).unwrap() % packets.len()];
                reorder.push(sequence, u32::from(sequence) * 960, opus, now, &mut out);
                out.clear();
            }
        });
        println!(
            "reorder, {}: {per:.0} ns per packet",
            if swapped { "2 % swapped" } else { "in order" }
        );
    }
    // Through `Receiving`, to Opus streams (each packet copied to each), drained as a reader
    // would.
    for subscriptions in [1, 10] {
        let per = best(&mut |count| {
            let mut receiving = Receiving::default();
            receiving.map(user(), SSRC, now);
            let mut streams: Vec<VoiceStream> = (0..subscriptions)
                .map(|_| stream(&mut receiving, ReceiveOptions::new()))
                .collect();
            for n in 0..count {
                let sequence = u16::try_from(n % 65_536).unwrap();
                let opus = &packets[usize::try_from(n).unwrap() % packets.len()];
                arrive(&mut receiving, sequence, opus, now);
                if n % 128 == 0 {
                    for stream in &mut streams {
                        while stream.try_next().is_ok() {}
                    }
                }
            }
        });
        println!("receiving to {subscriptions} Opus stream(s): {per:.0} ns per packet");
    }
    // Decoded: one PCM stream; the call's side (routing) and the reader's (decoding, D-374)
    // timed apart.
    let decoded: u32 = 20_000;
    let (mut routing, mut reading) = (f64::MAX, f64::MAX);
    for _ in 0..3 {
        let mut receiving = Receiving::default();
        receiving.map(user(), SSRC, now);
        let mut pcm = stream(&mut receiving, ReceiveOptions::new().decode(true));
        let (mut routed, mut read) = (0.0, 0.0);
        for n in 0..decoded {
            let sequence = u16::try_from(n % 65_536).unwrap();
            let opus = &packets[usize::try_from(n).unwrap() % packets.len()];
            let started = Clock::now();
            arrive(&mut receiving, sequence, opus, now);
            routed += started.elapsed().as_secs_f64();
            if n % 128 == 127 {
                let started = Clock::now();
                while pcm.try_next().is_ok() {}
                read += started.elapsed().as_secs_f64();
            }
        }
        routing = routing.min(routed);
        reading = reading.min(read);
    }
    println!(
        "receiving to a PCM stream: the call {:.0} ns per packet, its reader decoding {:.1} us",
        routing / f64::from(decoded) * 1e9,
        reading / f64::from(decoded) * 1e6
    );
}
