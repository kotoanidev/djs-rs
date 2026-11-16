//! Step 4's extreme-load tests (D-041, C-129), end to end through the manager and mock voice
//! servers on `127.0.0.1`: many calls playing at once, and a call receiving many speakers.
//! Measurements, not checks of behaviour: `cargo test --release -p djs-voice --lib
//! manager::stress_tests -- --ignored --nocapture --test-threads=1`. Linux only (the CPU
//! time and memory come from `/proc/self`); the sizes can be set with `DJS_VOICE_CALLS` and
//! `DJS_VOICE_SPEAKERS`.

#![expect(
    clippy::cast_precision_loss,
    reason = "counts and sizes printed as measurements"
)]

use core::time::Duration;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use djs_model::{
    gateway::{
        DispatchEvent,
        payload::{VoiceServerUpdate, VoiceStateUpdate},
    },
    id::{ChannelId, GuildId, UserId},
};
use serde_json::json;
use tokio::time::timeout;

use super::{
    VoiceManager,
    tests::{BOT, CHANNEL, Commands, SINE, command, manager, sine_packets},
};
use crate::{
    Call, Input, Loop, ReceiveOptions, VoiceData,
    connection::tests::{KEY, Mock, STEP, Server, send},
};

/// The CPU time this process has used (user and system), from `/proc/self/stat`.
fn cpu() -> Duration {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    // After the command's name, in parentheses: utime and stime are fields 14 and 15.
    let fields: Vec<&str> = stat[stat.rfind(')').unwrap() + 2..].split(' ').collect();
    let ticks: u64 = fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap();
    // The kernel's USER_HZ.
    Duration::from_millis(ticks * 10)
}

/// The process's resident memory, in KiB, from `/proc/self/status`.
fn rss() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let line = status
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .unwrap();
    line.split_whitespace().nth(1).unwrap().parse().unwrap()
}

/// The datagrams the kernel dropped because a UDP socket's receive buffer was full (all
/// sockets: the machine should be otherwise quiet), from `/proc/net/snmp`.
fn udp_drops() -> u64 {
    let snmp = std::fs::read_to_string("/proc/net/snmp").unwrap();
    let mut lines = snmp.lines().filter(|line| line.starts_with("Udp:"));
    let (names, values) = (lines.next().unwrap(), lines.next().unwrap());
    let column = names
        .split_whitespace()
        .position(|name| name == "RcvbufErrors")
        .unwrap();
    values
        .split_whitespace()
        .nth(column)
        .unwrap()
        .parse()
        .unwrap()
}

fn size(variable: &str, default: usize) -> usize {
    std::env::var(variable)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// Joins `guild` through `mock` (SSRC `ssrc`); the voice server's side.
async fn join(
    manager: &VoiceManager,
    commands: &mut Commands,
    guild: u64,
    mock: &Mock,
    ssrc: u32,
) -> (Call, Server) {
    let join = {
        let manager = manager.clone();
        tokio::spawn(async move {
            manager
                .join(GuildId::new(guild), ChannelId::new(CHANNEL))
                .await
        })
    };
    let state: VoiceStateUpdate = serde_json::from_value(json!({
        "guild_id": guild.to_string(), "channel_id": CHANNEL.to_string(),
        "user_id": BOT.to_string(), "session_id": "session",
        "deaf": false, "mute": false, "self_deaf": false, "self_mute": false,
        "self_video": false, "suppress": false, "request_to_speak_timestamp": null
    }))
    .unwrap();
    let server: VoiceServerUpdate = serde_json::from_value(json!({
        "token": "voice-token", "guild_id": guild.to_string(), "endpoint": mock.endpoint
    }))
    .unwrap();
    // Opcode 4 is sent first; then the call waits for both updates.
    command(commands).await;
    manager.process(&DispatchEvent::VoiceStateUpdate(Box::new(state)));
    manager.process(&DispatchEvent::VoiceServerUpdate(Box::new(server)));
    let mut server = mock.accept().await;
    mock.handshake(&mut server, ssrc, KEY, 41_250.0).await;
    let call = timeout(STEP, join).await.unwrap().unwrap().unwrap();
    (call, server)
}

/// What a mock voice server saw of a call's audio.
#[derive(Default)]
struct Seen {
    packets: AtomicU64,
    /// The longest time between two packets, in microseconds.
    gap: AtomicU64,
    /// Gaps over 30 ms (a packet half a frame late or more).
    late: AtomicU64,
}

/// Counts the audio packets `mock` receives (without decrypting them) until the end.
fn count(mock: Arc<Mock>, seen: Arc<Seen>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buffer = [0; 1500];
        let mut last: Option<Instant> = None;
        while let Ok(Ok((len, _))) =
            timeout(Duration::from_secs(2), mock.udp.recv_from(&mut buffer)).await
        {
            // Keepalives are 4 bytes.
            if len < 12 + 20 {
                continue;
            }
            let now = Instant::now();
            if let Some(last) = last.replace(now) {
                let gap = u64::try_from((now - last).as_micros()).unwrap_or(u64::MAX);
                seen.gap.fetch_max(gap, Ordering::Relaxed);
                if gap > 30_000 {
                    seen.late.fetch_add(1, Ordering::Relaxed);
                }
            }
            seen.packets.fetch_add(1, Ordering::Relaxed);
        }
    })
}

/// 20 ms frames of s16le stereo PCM: a 440 Hz tone at half the full scale.
fn tone(frames: usize) -> Vec<u8> {
    (0..frames * 960)
        .flat_map(|i| {
            #[expect(clippy::cast_precision_loss, reason = "small sample indices")]
            let angle = i as f32 * 440.0 * core::f32::consts::TAU / 48_000.0;
            #[expect(clippy::cast_possible_truncation, reason = "within the 16-bit range")]
            let value = (angle.sin() * 0.5 * 32_767.0) as i16;
            let [low, high] = value.to_le_bytes();
            [low, high, low, high]
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "an extreme-load measurement; run it in release"]
async fn many_calls_play_at_once() {
    let calls = size("DJS_VOICE_CALLS", 200);
    let seconds = 5;
    let (manager, mut commands) = manager();
    let before = rss();
    let mut joined = Vec::new();
    let started = Instant::now();
    for index in 0..calls {
        let mock = Arc::new(Mock::new().await);
        let guild = (u64::try_from(index).unwrap() + 1) << 22;
        let ssrc = u32::try_from(index).unwrap() + 1;
        let (call, server) = join(&manager, &mut commands, guild, &mock, ssrc).await;
        joined.push((call, server, mock));
    }
    let joining = started.elapsed();
    let memory = rss().saturating_sub(before);
    println!(
        "{calls} calls joined in {:.2} s; {} KiB of memory, {:.0} KiB per call (with its mock)",
        joining.as_secs_f64(),
        memory,
        memory as f64 / calls as f64
    );
    let dir = std::env::temp_dir().join(format!("djs-voice-stress-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sine.opus");
    std::fs::write(&path, SINE).unwrap();
    // One copy of the PCM for every call, so its memory is not counted as theirs.
    let pcm: Arc<[u8]> = tone(seconds * 50 + 50).into();
    for kind in [
        "Ogg/Opus as it is",
        "Ogg/Opus at volume 0.5",
        "PCM (encoded)",
    ] {
        let seen: Vec<Arc<Seen>> = (0..calls).map(|_| Arc::default()).collect();
        let counters: Vec<_> = joined
            .iter()
            .zip(&seen)
            .map(|((_, _, mock), seen)| count(Arc::clone(mock), Arc::clone(seen)))
            .collect();
        let mut tracks = Vec::new();
        for (call, _, _) in &joined {
            let track = if kind == "PCM (encoded)" {
                call.play(Input::pcm(std::io::Cursor::new(Arc::clone(&pcm))))
            } else {
                let track = call.play(Input::ogg_file(&path).await.unwrap());
                track.set_loop(Loop::Forever).unwrap();
                if kind.contains("0.5") {
                    track.set_volume(0.5).unwrap();
                }
                track
            };
            tracks.push(track);
        }
        // Past the start (the first frames, encoders made), then measured.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let cpu_before = cpu();
        let packets_before: u64 = seen
            .iter()
            .map(|seen| seen.packets.load(Ordering::Relaxed))
            .sum();
        let measured = Instant::now();
        tokio::time::sleep(Duration::from_secs(u64::try_from(seconds).unwrap())).await;
        let elapsed = measured.elapsed().as_secs_f64();
        let used = cpu().saturating_sub(cpu_before).as_secs_f64();
        let packets: u64 = seen
            .iter()
            .map(|seen| seen.packets.load(Ordering::Relaxed))
            .sum::<u64>()
            - packets_before;
        let playing = rss().saturating_sub(before);
        for track in &tracks {
            let _ = track.stop();
        }
        for counter in counters {
            counter.await.unwrap();
        }
        let gap = seen
            .iter()
            .map(|seen| seen.gap.load(Ordering::Relaxed))
            .max()
            .unwrap_or_default();
        let late: u64 = seen
            .iter()
            .map(|seen| seen.late.load(Ordering::Relaxed))
            .sum();
        let expected = calls as f64 * elapsed * 50.0;
        println!(
            "{kind}, {calls} calls: {packets} packets in {elapsed:.2} s ({:.1} % of 50 per \
             second per call); {:.2} cores busy ({:.0} us per call per second of audio, \
             {:.0} calls per core); the longest gap {:.1} ms, {late} over 30 ms; {playing} KiB \
             of memory while playing ({:.0} KiB per call)",
            packets as f64 / expected * 100.0,
            used / elapsed,
            used / elapsed / calls as f64 * 1e6,
            elapsed / used * calls as f64,
            gap as f64 / 1e3,
            playing as f64 / calls as f64,
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
    for (call, _, _) in &joined {
        let _ = timeout(STEP, call.leave()).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "an extreme-load measurement; run it in release"]
#[expect(clippy::too_many_lines, reason = "one measurement after another")]
async fn a_call_receives_many_speakers() {
    let speakers = size("DJS_VOICE_SPEAKERS", 50);
    let seconds: u32 = 5;
    let mock = Arc::new(Mock::new().await);
    let (manager, mut commands) = manager();
    let (call, mut server) = join(&manager, &mut commands, 1 << 22, &mock, 7).await;
    let packets = sine_packets();
    // Each speaker received twice: as Opus, and decoded.
    let mut streams = Vec::new();
    for speaker in 0..speakers {
        let user = UserId::new(100 + u64::try_from(speaker).unwrap());
        let ssrc = 1_000 + u32::try_from(speaker).unwrap();
        send(
            &mut server,
            json!({"op": 5, "d": {"user_id": user.to_string(), "ssrc": ssrc, "speaking": 1}}),
        )
        .await;
        streams.push((call.subscribe(user), false));
        streams.push((
            call.subscribe_with(user, ReceiveOptions::new().decode(true)),
            true,
        ));
    }
    let received: Vec<Arc<AtomicU64>> = streams.iter().map(|_| Arc::default()).collect();
    let lost = Arc::new(AtomicU64::new(0));
    let readers: Vec<_> = streams
        .into_iter()
        .zip(&received)
        .map(|((mut stream, _), count)| {
            let count = Arc::clone(count);
            let lost = Arc::clone(&lost);
            tokio::spawn(async move {
                while let Ok(Some(packet)) = timeout(Duration::from_secs(2), stream.next()).await {
                    if packet.audio == VoiceData::Lost {
                        lost.fetch_add(1, Ordering::Relaxed);
                    }
                    count.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();
    // Meanwhile the bot plays, re-encoding (volume 0.5): are its packets late?
    let dir = std::env::temp_dir().join(format!("djs-voice-speakers-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sine.opus");
    std::fs::write(&path, SINE).unwrap();
    let track = call.play(Input::ogg_file(&path).await.unwrap());
    track.set_loop(Loop::Forever).unwrap();
    track.set_volume(0.5).unwrap();
    let seen = Arc::new(Seen::default());
    let counter = count(Arc::clone(&mock), Arc::clone(&seen));
    tokio::time::sleep(Duration::from_millis(500)).await;
    let played_before = seen.packets.load(Ordering::Relaxed);
    seen.gap.store(0, Ordering::Relaxed);
    seen.late.store(0, Ordering::Relaxed);
    // Every speaker sends a packet every 20 ms.
    let cpu_before = cpu();
    let drops_before = udp_drops();
    let started = Instant::now();
    let mut ticker = tokio::time::interval(Duration::from_millis(20));
    let frames = seconds * 50;
    for frame in 0..frames {
        ticker.tick().await;
        let sequence = u16::try_from(frame).unwrap();
        let packet = &packets[usize::try_from(frame).unwrap() % packets.len()];
        for speaker in 0..speakers {
            let ssrc = 1_000 + u32::try_from(speaker).unwrap();
            mock.send_audio(KEY, ssrc, sequence, frame * 960, packet)
                .await;
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    let used = cpu().saturating_sub(cpu_before).as_secs_f64();
    let drops = udp_drops() - drops_before;
    let played = seen.packets.load(Ordering::Relaxed) - played_before;
    let _ = track.stop();
    for reader in readers {
        reader.await.unwrap();
    }
    counter.await.unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let total: u64 = received
        .iter()
        .map(|count| count.load(Ordering::Relaxed))
        .sum();
    let sent = u64::from(frames) * u64::try_from(speakers).unwrap();
    println!(
        "{speakers} speakers for {elapsed:.2} s: {sent} packets sent, {total} received over \
         {} streams ({:.1} % of 2 per packet), {} lost ({drops} datagrams dropped by the \
         kernel, its receive buffer full); {:.2} cores busy (with the mock's \
         sealing), {:.1} us per packet received",
        speakers * 2,
        total as f64 / (sent as f64 * 2.0) * 100.0,
        lost.load(Ordering::Relaxed),
        used / elapsed,
        used / sent as f64 * 1e6,
    );
    println!(
        "  meanwhile the bot sent {played} packets ({:.1} % of 50 per second), the longest \
         gap {:.1} ms, {} over 30 ms",
        played as f64 / (elapsed * 50.0) * 100.0,
        seen.gap.load(Ordering::Relaxed) as f64 / 1e3,
        seen.late.load(Ordering::Relaxed),
    );
    let _ = timeout(STEP, call.leave()).await;
}
