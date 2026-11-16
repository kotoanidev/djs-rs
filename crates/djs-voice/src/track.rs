//! Tracks (D-323, D-339, D-352–D-361): what a call plays, their handles, state and events.
//!
//! A track's state is shared between its handles and its call's task (C-126): atomics for
//! what [`TrackHandle::info`] reads, and its event streams. Whoever ends a track (a handle
//! stopping, replacing or skipping it, or the task when it played to its end) sets the
//! reason once, the first wins, and sends it to the streams.

use core::{
    fmt,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    task::{Context, Poll},
    time::Duration,
};
use std::sync::{Arc, Mutex, Weak};

use futures_util::Stream;
use tokio::sync::{mpsc, oneshot};

use crate::{
    InputError, TrackError,
    call::{CallCommand, lock},
    queue::Shared,
};

/// `samples` per channel at 48 kHz as a duration: 62 500 / 3 ns each.
pub(crate) fn duration(samples: u64) -> Duration {
    Duration::from_nanos(samples.saturating_mul(62_500) / 3)
}

/// `duration` as samples per channel at 48 kHz.
pub(crate) fn samples(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos() * 3 / 62_500).unwrap_or(u64::MAX)
}

/// A track, shared by its handles, its call's queue and its task.
pub(crate) struct Track {
    pub(crate) id: u64,
    /// Its input can be opened again: it loops and seeks (D-360).
    seekable: bool,
    /// The call's task took its input: it is playing or paused (or ended).
    started: AtomicBool,
    paused: AtomicBool,
    ended: AtomicBool,
    /// Samples per channel at 48 kHz played; for a waiting track, where it will start.
    position: AtomicU64,
    /// The volume's bits (an `f32`).
    volume: AtomicU32,
    looping: Mutex<Loop>,
    subscribers: Mutex<Subscribers>,
    call: Weak<Shared>,
}

#[derive(Default)]
struct Subscribers {
    senders: Vec<mpsc::UnboundedSender<TrackEvent>>,
    /// The track ended: new streams end at once.
    closed: bool,
}

impl Track {
    pub(crate) fn new(id: u64, seekable: bool, call: Weak<Shared>) -> Arc<Self> {
        Arc::new(Self {
            id,
            seekable,
            started: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            position: AtomicU64::new(0),
            volume: AtomicU32::new(1.0_f32.to_bits()),
            looping: Mutex::new(Loop::Off),
            subscribers: Mutex::new(Subscribers::default()),
            call,
        })
    }

    /// Ends the track for `reason`, unless it already ended; whether it did now. The
    /// streams get [`TrackEvent::Ended`] and end.
    pub(crate) fn end(&self, reason: TrackEnd) -> bool {
        let mut subscribers = lock(&self.subscribers);
        if subscribers.closed {
            return false;
        }
        self.ended.store(true, Ordering::Release);
        let event = TrackEvent::Ended(reason);
        for subscriber in &subscribers.senders {
            let _ = subscriber.send(event.clone());
        }
        subscribers.senders.clear();
        subscribers.closed = true;
        true
    }

    /// Sends `event` to the track's streams.
    pub(crate) fn emit(&self, event: &TrackEvent) {
        lock(&self.subscribers)
            .senders
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }

    pub(crate) fn ended(&self) -> bool {
        self.ended.load(Ordering::Acquire)
    }

    pub(crate) fn started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }

    /// The call's task took the input (under the queue's lock).
    pub(crate) fn start(&self) {
        self.started.store(true, Ordering::Release);
    }

    pub(crate) fn paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    pub(crate) fn position(&self) -> u64 {
        self.position.load(Ordering::Acquire)
    }

    pub(crate) fn set_position(&self, samples: u64) {
        self.position.store(samples, Ordering::Release);
    }

    pub(crate) fn advance(&self, samples: u32) {
        self.position
            .fetch_add(u64::from(samples), Ordering::AcqRel);
    }

    pub(crate) fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Acquire))
    }

    /// At the end of the input: whether the track plays again, counting a repeat down.
    pub(crate) fn repeat(&self) -> bool {
        let mut looping = lock(&self.looping);
        match *looping {
            Loop::Forever => true,
            Loop::Times(left @ 1..) => {
                *looping = Loop::Times(left - 1);
                true
            }
            Loop::Times(0) | Loop::Off => false,
        }
    }

    fn info(&self) -> TrackInfo {
        let state = if self.ended() {
            PlayState::Ended
        } else if !self.started() {
            PlayState::Queued
        } else if self.paused() {
            PlayState::Paused
        } else {
            PlayState::Playing
        };
        TrackInfo {
            state,
            position: duration(self.position()),
            volume: self.volume(),
            looping: *lock(&self.looping),
        }
    }
}

/// Controls a track that [`Call::play`](crate::Call::play) or
/// [`Call::enqueue`](crate::Call::enqueue) gave the call: pause it, play it again, stop it,
/// change its volume, loop it, seek in it, read its state, follow its events.
///
/// A cheap-clone handle: clones control the same track. The controls only ask the call to
/// act, so they return at once ([`seek`](Self::seek) waits for it); they fail with
/// [`TrackError::Ended`] once the track has ended (played to its end, stopped, replaced,
/// skipped, its input failed, or its call ended). Dropping every handle does not stop the
/// track.
///
/// discord.js counterpart: `AudioPlayer#pause`, `unpause` and `stop`, and
/// `AudioResource#volume` of `@discordjs/voice` (a player plays one resource); songbird:
/// `TrackHandle`.
///
/// # Examples
///
/// ```no_run
/// # async fn run(call: djs_voice::Call) -> Result<(), Box<dyn std::error::Error>> {
/// use std::time::Duration;
///
/// use djs_voice::{Input, Loop};
///
/// let track = call.play(Input::ffmpeg("song.mp3"));
/// track.set_volume(0.5)?;
/// track.set_loop(Loop::Times(2))?;
/// track.seek(Duration::from_secs(30)).await?;
/// track.pause()?;
/// track.play()?;
/// println!("at {:?}", track.info().position);
/// track.stop()?;
/// assert!(track.play().is_err(), "a stopped track has ended");
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct TrackHandle {
    track: Arc<Track>,
}

impl fmt::Debug for TrackHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrackHandle")
            .field("id", &self.track.id)
            .field("info", &self.track.info())
            .finish()
    }
}

impl PartialEq for TrackHandle {
    /// Whether both handles control the same track.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.track, &other.track)
    }
}

impl Eq for TrackHandle {}

impl TrackHandle {
    pub(crate) fn new(track: Arc<Track>) -> Self {
        Self { track }
    }

    /// Plays the track again after [`pause`](Self::pause), from where it paused; does
    /// nothing to a track that is playing. A waiting track that was paused starts playing
    /// when its turn comes.
    ///
    /// discord.js counterpart: `AudioPlayer#unpause`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended.
    pub fn play(&self) -> Result<(), TrackError> {
        self.set_paused(false)
    }

    /// Pauses the track: it stops sending audio (after a few frames of silence, as Discord
    /// asks) and keeps its place; [`play`](Self::play) goes on from there. A waiting track
    /// starts paused when its turn comes (D-361).
    ///
    /// discord.js counterpart: `AudioPlayer#pause`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended.
    pub fn pause(&self) -> Result<(), TrackError> {
        self.set_paused(true)
    }

    fn set_paused(&self, paused: bool) -> Result<(), TrackError> {
        self.alive()?;
        self.track.paused.store(paused, Ordering::Release);
        self.wake();
        Ok(())
    }

    /// Stops the track for good ([`TrackEnd::Stopped`]): its input is no longer read. The
    /// call's queue goes on with the next track; a waiting track is removed from the queue.
    ///
    /// discord.js counterpart: `AudioPlayer#stop`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has already ended.
    pub fn stop(&self) -> Result<(), TrackError> {
        self.alive()?;
        if let Some(call) = self.track.call.upgrade() {
            lock(&call.queue).remove(self.track.id, TrackEnd::Stopped);
            call.wake();
        }
        // Not in the queue: the call ended, or it is ending the track now.
        self.track.end(TrackEnd::Stopped);
        Ok(())
    }

    /// Sets the track's volume (D-353): linear, 1.0 the original, 0.5 half as loud, 0.0
    /// silent, up to 2.0 (louder, with a soft clip against distortion); values outside
    /// `0.0..=2.0` are clamped, NaN counts as 0.0. Heard from the next frame.
    ///
    /// At 1.0 an Ogg/Opus track is sent as it is; at any other volume it is decoded and
    /// encoded again, which costs some CPU.
    ///
    /// discord.js counterpart: `AudioResource#volume.setVolume` (with `inlineVolume`);
    /// songbird: `TrackHandle::set_volume`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended.
    pub fn set_volume(&self, volume: f32) -> Result<(), TrackError> {
        self.alive()?;
        let volume = if volume.is_nan() {
            0.0
        } else {
            volume.clamp(0.0, 2.0)
        };
        self.track.volume.store(volume.to_bits(), Ordering::Release);
        Ok(())
    }

    /// Sets whether the track plays again when it reaches its end (D-358): never, forever,
    /// or a number of times more. Each repeat sends [`TrackEvent::Looped`];
    /// [`TrackInfo::looping`] shows the repeats left.
    ///
    /// discord.js counterpart: none (a bot plays the resource again); songbird:
    /// `TrackHandle::enable_loop`, `disable_loop` and `loop_for`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended, [`TrackError::Unsupported`] when its
    /// input is a reader, which cannot be opened again (D-360).
    pub fn set_loop(&self, looping: Loop) -> Result<(), TrackError> {
        self.alive()?;
        if !self.track.seekable {
            return Err(TrackError::Unsupported);
        }
        *lock(&self.track.looping) = looping;
        Ok(())
    }

    /// Plays the track from `position` (D-359); returns once it plays from there, with the
    /// position it actually plays from: the start of the Opus packet holding `position` for
    /// an Ogg/Opus file, `position` itself with FFmpeg. A waiting track will start there
    /// (the result comes at once). Seeking past the end ends the track
    /// ([`TrackEnd::Finished`]), unless it loops.
    ///
    /// The file is opened again (its packets skipped up to `position`, without decoding), or
    /// FFmpeg run again with `-ss`.
    ///
    /// discord.js counterpart: none (a resource is played again with FFmpeg's `-ss`);
    /// songbird: `TrackHandle::seek`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended (or ends while seeking: past its end,
    /// or its input failed), [`TrackError::Unsupported`] when its input is a reader, which
    /// cannot be opened again (D-360).
    pub async fn seek(&self, position: Duration) -> Result<Duration, TrackError> {
        self.alive()?;
        if !self.track.seekable {
            return Err(TrackError::Unsupported);
        }
        let target = samples(position);
        let call = self.track.call.upgrade().ok_or(TrackError::Ended)?;
        let (done, result) = oneshot::channel();
        {
            // Under the lock the call's task starts tracks with.
            let queue = lock(&call.queue);
            if !self.track.started() {
                self.track.set_position(target);
                return Ok(duration(target));
            }
            drop(queue);
        }
        let command = CallCommand::Seek {
            id: self.track.id,
            samples: target,
            done,
        };
        if call.commands.send(command).is_err() {
            return Err(TrackError::Ended);
        }
        drop(call);
        result.await.unwrap_or(Err(TrackError::Ended))
    }

    /// The track's state now: playing, paused, waiting or ended, where it is, its volume
    /// and loop (D-356). Read at once, without asking the call.
    ///
    /// discord.js counterpart: `AudioPlayer#state` and `AudioResource#playbackDuration`;
    /// songbird: `TrackHandle::get_info`.
    #[must_use]
    pub fn info(&self) -> TrackInfo {
        self.track.info()
    }

    /// Returns a stream of the track's events (D-355): it plays, pauses, loops, and ends,
    /// with why. Each stream gets every event from its creation on, and ends after
    /// [`TrackEvent::Ended`]; a stream of a track that already ended ends at once.
    ///
    /// discord.js counterpart: `AudioPlayer#on('stateChange')`; songbird:
    /// `TrackHandle::add_event`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(call: djs_voice::Call) {
    /// use djs_voice::{Input, TrackEnd, TrackEvent};
    ///
    /// let track = call.enqueue(Input::ffmpeg("song.mp3"));
    /// let mut events = track.events();
    /// while let Some(event) = events.next().await {
    ///     match event {
    ///         TrackEvent::Playing => println!("now playing"),
    ///         TrackEvent::Ended(TrackEnd::Failed(error)) => println!("failed: {error}"),
    ///         TrackEvent::Ended(reason) => println!("ended: {reason}"),
    ///         _ => {}
    ///     }
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn events(&self) -> TrackEvents {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut subscribers = lock(&self.track.subscribers);
        if !subscribers.closed {
            subscribers.senders.push(tx);
        }
        TrackEvents { rx }
    }

    fn alive(&self) -> Result<(), TrackError> {
        if self.track.ended() {
            Err(TrackError::Ended)
        } else {
            Ok(())
        }
    }

    fn wake(&self) {
        if let Some(call) = self.track.call.upgrade() {
            call.wake();
        }
    }
}

/// Whether a track plays again at its end (D-358), for [`TrackHandle::set_loop`].
///
/// songbird counterpart: `LoopState`.
///
/// discord.js counterpart: none (`@discordjs/voice` has no looping).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Loop {
    /// It ends at its end.
    #[default]
    Off,
    /// It plays again, every time.
    Forever,
    /// It plays again this many more times; [`TrackInfo::looping`] counts down.
    Times(u32),
}

/// Where a track is (D-361), in [`TrackInfo::state`].
///
/// discord.js counterpart: `AudioPlayerStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlayState {
    /// Waiting in the call's queue.
    Queued,
    /// Playing (or about to: its input is being read).
    Playing,
    /// Paused with [`TrackHandle::pause`].
    Paused,
    /// Ended; [`TrackEvent::Ended`] said why.
    Ended,
}

/// A track's state, from [`TrackHandle::info`] (D-356).
///
/// songbird counterpart: `TrackState`.
///
/// discord.js counterpart: `AudioPlayer#state` (`@discordjs/voice`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TrackInfo {
    /// Playing, paused, waiting or ended.
    pub state: PlayState,
    /// How much of it has played (from where a seek put it, rewound by a loop); for a
    /// waiting track, where it will start.
    pub position: Duration,
    /// Its volume, 1.0 the original ([`TrackHandle::set_volume`]).
    pub volume: f32,
    /// Whether it plays again at its end, and how many more times
    /// ([`TrackHandle::set_loop`]).
    pub looping: Loop,
}

/// An event of a track, from [`TrackHandle::events`] (D-355).
///
/// discord.js counterpart: the state changes of `AudioPlayer`; songbird: `TrackEvent`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TrackEvent {
    /// Its audio goes out: it started, or plays again after a pause.
    Playing,
    /// It was paused ([`TrackHandle::pause`]) while it played.
    Paused,
    /// It reached its end and plays again ([`TrackHandle::set_loop`]).
    Looped,
    /// It ended, for this reason; the stream ends after it.
    Ended(TrackEnd),
}

/// Why a track ended (D-355), in [`TrackEvent::Ended`].
///
/// songbird counterpart: `PlayMode::End`, `Stop` and `Errored`.
///
/// discord.js counterpart: none (`@discordjs/voice` reports only the player's state changes).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TrackEnd {
    /// It played to its end (or was sought past it).
    Finished,
    /// [`TrackHandle::stop`], or [`TrackQueue::clear`](crate::TrackQueue::clear) while it
    /// waited.
    Stopped,
    /// [`Call::play`](crate::Call::play) played another track instead.
    Replaced,
    /// [`TrackQueue::skip`](crate::TrackQueue::skip).
    Skipped,
    /// Its input failed: it could not be read or decoded, or FFmpeg failed.
    Failed(Arc<InputError>),
    /// Its call ended.
    CallEnded,
}

impl fmt::Display for TrackEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finished => f.write_str("the track played to its end"),
            Self::Stopped => f.write_str("the track was stopped"),
            Self::Replaced => f.write_str("another track replaced it"),
            Self::Skipped => f.write_str("the track was skipped"),
            Self::Failed(error) => write!(f, "the track's input failed: {error}"),
            Self::CallEnded => f.write_str("its call ended"),
        }
    }
}

/// The stream of a track's events. Created by [`TrackHandle::events`].
///
/// Implements [`Stream`]; [`next`](Self::next) is provided so no extension trait is needed.
///
/// discord.js counterpart: `AudioPlayer#on` (`@discordjs/voice`).
#[derive(Debug)]
pub struct TrackEvents {
    rx: mpsc::UnboundedReceiver<TrackEvent>,
}

impl TrackEvents {
    /// Waits for the next event. Returns `None` once the track has ended (after
    /// [`TrackEvent::Ended`]).
    ///
    /// discord.js counterpart: `AudioPlayer#on` (`@discordjs/voice`).
    pub async fn next(&mut self) -> Option<TrackEvent> {
        self.rx.recv().await
    }
}

impl Stream for TrackEvents {
    type Item = TrackEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<TrackEvent>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::{duration, samples};

    #[test]
    fn samples_and_durations_convert() {
        assert_eq!(duration(48_000), Duration::from_secs(1));
        assert_eq!(duration(960), Duration::from_millis(20));
        assert_eq!(samples(Duration::from_millis(500)), 24_000);
        assert_eq!(samples(Duration::from_secs(90)), 4_320_000);
        assert_eq!(samples(duration(123_456)), 123_456);
        assert_eq!(samples(Duration::MAX), u64::MAX);
    }
}
