//! The player of step 2 end to end (D-350–D-362, C-126): the queue, track events and
//! state, volume, PCM, loops, seeking, failures and the bitrate, through a joined call and
//! the mock voice server.

use core::time::Duration;

use djs_model::id::ChannelId;
use tokio::time::timeout;

use super::tests::{
    CHANNEL, SILENCE, SINE, audio, guild, joined, manager, silence_follows, sine_packets,
};
use crate::{
    Input, InputError, JoinOptions, Loop, PlayState, TrackEnd, TrackError, TrackEvent, TrackEvents,
    TrackHandle,
    codec::{Decoder, MAX_PACKET_VALUES},
    connection::tests::{KEY, Mock, STEP, recv_op},
    opus::packet_samples,
};

/// `frames` frames of 20 ms of s16le stereo PCM: a 440 Hz tone at `level` of the full
/// scale.
fn tone(frames: usize, level: f32) -> Vec<u8> {
    (0..frames * 960)
        .flat_map(|i| {
            #[expect(clippy::cast_precision_loss, reason = "small sample indices")]
            let angle = i as f32 * 440.0 * core::f32::consts::TAU / 48_000.0;
            #[expect(clippy::cast_possible_truncation, reason = "within the 16-bit range")]
            let value = (angle.sin() * level * 32_767.0) as i16;
            let [low, high] = value.to_le_bytes();
            [low, high, low, high]
        })
        .collect()
}

/// A PCM input that never ends (zeros: silence, still encoded).
fn endless() -> Input {
    Input::pcm(tokio::io::repeat(0))
}

async fn next(events: &mut TrackEvents) -> TrackEvent {
    timeout(STEP, events.next())
        .await
        .expect("no track event")
        .expect("the track's events ended")
}

/// Waits for the track's `Ended`, skipping the other events.
async fn ended(events: &mut TrackEvents) -> TrackEnd {
    loop {
        if let TrackEvent::Ended(reason) = next(events).await {
            assert!(events.next().await.is_none(), "the stream ends after Ended");
            return reason;
        }
    }
}

/// The RMS of decoded audio.
fn rms(pcm: &[f32]) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "short buffers")]
    let len = pcm.len() as f32;
    (pcm.iter().map(|value| value * value).sum::<f32>() / len).sqrt()
}

/// The loudness of the `n`-th to the `n + 4`-th next packets, decoded in order (the decoder
/// settles on the first ones).
async fn loudness(mock: &Mock, skip: usize) -> f32 {
    let mut decoder = Decoder::new().unwrap();
    let mut pcm = vec![0.0; MAX_PACKET_VALUES];
    let mut last = Vec::new();
    for n in 0..skip + 5 {
        let (payload, _, _) = audio(mock, KEY).await;
        let samples = decoder.decode(&payload, &mut pcm).unwrap();
        if n >= skip {
            last.extend_from_slice(&pcm[..samples * 2]);
        }
    }
    rms(&last)
}

#[tokio::test]
async fn queued_tracks_play_in_turn() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, mut server) = joined(&manager, &mut commands, &mock).await;
    let first = call.enqueue(Input::pcm(std::io::Cursor::new(tone(5, 0.5))));
    let second = call.enqueue(Input::pcm(std::io::Cursor::new(tone(4, 0.5))));
    let (mut first_events, mut second_events) = (first.events(), second.events());
    let queue = call.queue();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.current(), Some(first.clone()));
    assert_eq!(second.info().state, PlayState::Queued);

    // Nine encoded frames of 20 ms, one track after the other, then silence.
    let (_, sequence, timestamp) = audio(&mock, KEY).await;
    assert_eq!(recv_op(&mut server, 5).await["d"]["speaking"], 1);
    for n in 1..9_u16 {
        let (payload, seq, ts) = audio(&mock, KEY).await;
        assert_ne!(payload, SILENCE);
        assert_eq!(packet_samples(&payload), Some(960));
        assert_eq!(seq, sequence.wrapping_add(n), "no gap between the tracks");
        assert_eq!(ts, timestamp.wrapping_add(960 * u32::from(n)));
    }
    silence_follows(&mock, &mut server).await;
    assert!(matches!(next(&mut first_events).await, TrackEvent::Playing));
    assert!(matches!(ended(&mut first_events).await, TrackEnd::Finished));
    assert!(matches!(
        next(&mut second_events).await,
        TrackEvent::Playing
    ));
    assert!(matches!(
        ended(&mut second_events).await,
        TrackEnd::Finished
    ));
    assert!(queue.is_empty());
    let info = first.info();
    assert_eq!(info.state, PlayState::Ended);
    assert_eq!(info.position, Duration::from_millis(100));
    assert_eq!(first.play(), Err(TrackError::Ended));
}

#[tokio::test]
async fn skip_clear_pause_and_stop_drive_the_queue() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, _server) = joined(&manager, &mut commands, &mock).await;
    let queue = call.queue();
    assert!(!queue.skip(), "nothing to skip");
    let tracks: Vec<TrackHandle> = (0..4).map(|_| call.enqueue(endless())).collect();
    let mut events: Vec<TrackEvents> = tracks.iter().map(TrackHandle::events).collect();
    assert!(matches!(next(&mut events[0]).await, TrackEvent::Playing));

    assert!(queue.skip());
    assert!(matches!(ended(&mut events[0]).await, TrackEnd::Skipped));
    assert_eq!(queue.current(), Some(tracks[1].clone()));
    assert!(matches!(next(&mut events[1]).await, TrackEvent::Playing));

    // A waiting track stopped leaves the queue.
    tracks[2].stop().unwrap();
    assert!(matches!(ended(&mut events[2]).await, TrackEnd::Stopped));
    assert_eq!(queue.len(), 2);

    tracks[1].pause().unwrap();
    assert!(matches!(next(&mut events[1]).await, TrackEvent::Paused));
    assert_eq!(tracks[1].info().state, PlayState::Paused);
    let position = tracks[1].info().position;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        tracks[1].info().position,
        position,
        "a paused track keeps its place"
    );
    tracks[1].play().unwrap();
    assert!(matches!(next(&mut events[1]).await, TrackEvent::Playing));
    assert_eq!(tracks[1].info().state, PlayState::Playing);

    queue.clear();
    assert!(matches!(ended(&mut events[3]).await, TrackEnd::Stopped));
    assert_eq!(queue.len(), 1, "the current track goes on");
    tracks[1].stop().unwrap();
    assert!(matches!(ended(&mut events[1]).await, TrackEnd::Stopped));
    assert!(queue.is_empty());
    assert_eq!(tracks[1].stop(), Err(TrackError::Ended));

    // A waiting track paused starts paused.
    let paused = call.enqueue(endless());
    paused.pause().unwrap();
    let mut paused_events = paused.events();
    let other = call.enqueue(endless());
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert_eq!(paused.info().state, PlayState::Paused, "it started paused");
    assert_eq!(paused.info().position, Duration::ZERO);
    assert_eq!(other.info().state, PlayState::Queued);
    paused.play().unwrap();
    assert!(matches!(
        next(&mut paused_events).await,
        TrackEvent::Playing
    ));
}

#[tokio::test]
async fn play_replaces_the_current_track_and_the_queue_goes_on() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, _server) = joined(&manager, &mut commands, &mock).await;
    let first = call.enqueue(endless());
    let waiting = call.enqueue(Input::pcm(std::io::Cursor::new(tone(2, 0.5))));
    let mut first_events = first.events();
    let mut waiting_events = waiting.events();
    assert!(matches!(next(&mut first_events).await, TrackEvent::Playing));
    let now = call.play(Input::pcm(std::io::Cursor::new(tone(2, 0.5))));
    let mut now_events = now.events();
    assert!(matches!(ended(&mut first_events).await, TrackEnd::Replaced));
    assert_eq!(call.queue().current(), Some(now.clone()));
    assert_eq!(call.queue().len(), 2);
    assert!(matches!(ended(&mut now_events).await, TrackEnd::Finished));
    assert!(matches!(
        next(&mut waiting_events).await,
        TrackEvent::Playing
    ));
    assert!(matches!(
        ended(&mut waiting_events).await,
        TrackEnd::Finished
    ));
}

#[tokio::test]
async fn the_volume_scales_pcm_and_ogg_opus_goes_as_it_is_at_1() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, _server) = joined(&manager, &mut commands, &mock).await;
    let track = call.play(Input::pcm(std::io::Cursor::new(tone(200, 0.5))));
    // The tone's RMS is 0.5 / √2.
    let full = loudness(&mock, 3).await;
    assert!((full - 0.354).abs() < 0.05, "rms {full}");
    track.set_volume(0.5).unwrap();
    assert!((track.info().volume - 0.5).abs() < f32::EPSILON);
    let half = loudness(&mock, 5).await;
    assert!((half - 0.177).abs() < 0.03, "rms {half}");
    track.set_volume(f32::NAN).unwrap();
    assert!(
        track.info().volume.abs() < f32::EPSILON,
        "NaN counts as 0.0"
    );
    track.set_volume(7.0).unwrap();
    assert!((track.info().volume - 2.0).abs() < f32::EPSILON, "clamped");
    track.stop().unwrap();

    let expected = sine_packets();
    let track = call.play(Input::ogg(SINE));
    // After the PCM in flight and the silence of its end: the file's packets as they are.
    let mut in_flight = 0;
    while !expected.contains(&audio(&mock, KEY).await.0) {
        in_flight += 1;
        assert!(in_flight < 20, "not sent as it is at 1.0");
    }
    track.set_volume(0.5).unwrap();
    // Re-encoded: none of the next packets is the file's.
    for _ in 0..3 {
        audio(&mock, KEY).await;
    }
    for _ in 0..5 {
        let (payload, _, _) = audio(&mock, KEY).await;
        assert!(!expected.contains(&payload), "re-encoded at 0.5");
        assert_eq!(packet_samples(&payload), Some(960));
    }
    track.set_volume(1.0).unwrap();
    for _ in 0..3 {
        audio(&mock, KEY).await;
    }
    let (payload, _, _) = audio(&mock, KEY).await;
    assert!(expected.contains(&payload), "as it is again at 1.0");
}

#[tokio::test]
async fn files_loop_and_seek_and_readers_cannot() {
    let dir = std::env::temp_dir().join(format!("djs-voice-player-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sine.opus");
    std::fs::write(&path, SINE).unwrap();
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, mut server) = joined(&manager, &mut commands, &mock).await;
    let expected = sine_packets();

    // Played twice: 102 packets, one `Looped`.
    let track = call.play(Input::ogg_file(&path).await.unwrap());
    track.set_loop(Loop::Times(1)).unwrap();
    assert_eq!(track.info().looping, Loop::Times(1));
    let mut events = track.events();
    assert_eq!(recv_op(&mut server, 5).await["d"]["speaking"], 1);
    for n in 0..expected.len() * 2 {
        let (payload, _, _) = audio(&mock, KEY).await;
        assert_eq!(payload, expected[n % expected.len()], "packet {n}");
    }
    silence_follows(&mock, &mut server).await;
    assert!(matches!(next(&mut events).await, TrackEvent::Playing));
    assert!(matches!(next(&mut events).await, TrackEvent::Looped));
    assert!(matches!(ended(&mut events).await, TrackEnd::Finished));
    assert_eq!(track.info().looping, Loop::Times(0));

    // Seeking to 0.5 s: the 26th packet (25 × 20 ms) plays next.
    let track = call.play(Input::ogg_file(&path).await.unwrap());
    audio(&mock, KEY).await;
    let position = track.seek(Duration::from_millis(500)).await.unwrap();
    assert_eq!(position, Duration::from_millis(500));
    let mut in_flight = 0;
    while audio(&mock, KEY).await.0 != expected[25] {
        in_flight += 1;
        assert!(in_flight < 3, "the new place does not play");
    }
    assert!(track.info().position >= Duration::from_millis(520));
    // Back to the start, then past the end: the track ends.
    assert_eq!(track.seek(Duration::ZERO).await, Ok(Duration::ZERO));
    let mut events = track.events();
    assert_eq!(
        track.seek(Duration::from_secs(5)).await,
        Err(TrackError::Ended)
    );
    assert!(matches!(ended(&mut events).await, TrackEnd::Finished));

    // A waiting track starts where it was sought to.
    let current = call.play(endless());
    let waiting = call.enqueue(Input::ogg_file(&path).await.unwrap());
    assert_eq!(
        waiting.seek(Duration::from_millis(800)).await,
        Ok(Duration::from_millis(800))
    );
    assert_eq!(waiting.info().position, Duration::from_millis(800));
    assert_eq!(waiting.info().state, PlayState::Queued);
    // Readers cannot be opened again (D-360).
    assert_eq!(
        current.set_loop(Loop::Forever),
        Err(TrackError::Unsupported)
    );
    assert_eq!(
        current.seek(Duration::from_secs(1)).await,
        Err(TrackError::Unsupported)
    );
    current.stop().unwrap();
    // 0.8 s is in the 41st packet.
    let mut in_flight = 0;
    while audio(&mock, KEY).await.0 != expected[40] {
        in_flight += 1;
        // What was sent before: the end of the last track, silence, the reader.
        assert!(in_flight < 40, "the waiting track did not start at 0.8 s");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_failed_input_ends_its_track_and_the_next_plays() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, _server) = joined(&manager, &mut commands, &mock).await;
    let not_ogg = call.enqueue(Input::ogg(&b"plain text"[..]));
    let no_ffmpeg = call.enqueue(Input::ffmpeg_with("djs-voice-no-such-program", "a.mp3"));
    let good = call.enqueue(Input::pcm(std::io::Cursor::new(tone(2, 0.5))));
    let (mut a, mut b, mut c) = (not_ogg.events(), no_ffmpeg.events(), good.events());
    match ended(&mut a).await {
        TrackEnd::Failed(error) => assert!(matches!(*error, InputError::NotOgg)),
        other => panic!("{other:?}"),
    }
    match ended(&mut b).await {
        TrackEnd::Failed(error) => assert!(matches!(*error, InputError::FfmpegMissing)),
        other => panic!("{other:?}"),
    }
    assert!(matches!(next(&mut c).await, TrackEvent::Playing));
    assert!(matches!(ended(&mut c).await, TrackEnd::Finished));
}

#[tokio::test]
async fn the_end_of_the_call_ends_its_tracks() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, _server) = joined(&manager, &mut commands, &mock).await;
    let current = call.enqueue(endless());
    let waiting = call.enqueue(endless());
    let (mut a, mut b) = (current.events(), waiting.events());
    call.leave().await.unwrap();
    assert!(matches!(ended(&mut a).await, TrackEnd::CallEnded));
    assert!(matches!(ended(&mut b).await, TrackEnd::CallEnded));
    let late = call.play(endless());
    assert_eq!(late.info().state, PlayState::Ended);
    assert!(late.events().next().await.is_none());
    assert!(call.queue().is_empty());
}

#[tokio::test]
async fn joining_again_changes_the_bitrate() {
    let mock = Mock::new().await;
    let (manager, mut commands) = manager();
    let (call, _events, _server) = joined(&manager, &mut commands, &mock).await;
    call.play(Input::pcm(std::io::Cursor::new(tone(500, 0.5))));
    let mut size = 0;
    for _ in 0..10 {
        size += audio(&mock, KEY).await.0.len();
    }
    // Already in the channel: the join answers at once, with the new bitrate.
    let options = JoinOptions::new().bitrate(8_000);
    let again = manager.join_with(guild(), ChannelId::new(CHANNEL), options);
    let (again, _) = tokio::join!(again, async {
        // Opcode 4 is sent again.
        super::tests::command(&mut commands).await
    });
    again.unwrap();
    for _ in 0..3 {
        audio(&mock, KEY).await;
    }
    let mut small = 0;
    for _ in 0..10 {
        small += audio(&mock, KEY).await.0.len();
    }
    // 128 kb/s is about 320 bytes per 20 ms; 8 kb/s about 20.
    assert!(
        small * 4 < size,
        "{small} bytes at 8 kb/s, {size} at 128 kb/s"
    );
}
