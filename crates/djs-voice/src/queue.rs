//! A call's queue of tracks (D-352, C-126): the current track and the waiting ones, with
//! their inputs, shared by the call's handles and its task under one lock (never held across
//! an `await`). The handles change it at once; the task starts whatever is current, and
//! moves on when a track ends.

use core::fmt;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc;

use crate::{
    Input, TrackHandle,
    call::{CallCommand, lock},
    track::{Track, TrackEnd},
};

/// What a call's handles share with its task.
pub(crate) struct Shared {
    pub(crate) commands: mpsc::UnboundedSender<CallCommand>,
    pub(crate) queue: Mutex<Queue>,
}

impl Shared {
    /// Tells the task the queue or a track changed.
    pub(crate) fn wake(&self) {
        // The task may be gone: the call ended, and its tracks with it.
        let _ = self.commands.send(CallCommand::Wake);
    }
}

/// A track in the queue, and its input until the call's task starts it.
pub(crate) struct Slot {
    pub(crate) track: Arc<Track>,
    pub(crate) input: Option<Input>,
}

/// The tracks of a call.
#[derive(Default)]
pub(crate) struct Queue {
    /// The track playing (or paused), or about to be started by the task.
    pub(crate) current: Option<Slot>,
    pub(crate) waiting: VecDeque<Slot>,
    /// The call ended: tracks given to it end at once.
    pub(crate) closed: bool,
}

impl Queue {
    /// The next waiting track becomes the current one.
    pub(crate) fn advance(&mut self) {
        self.current = self.waiting.pop_front();
    }

    /// Ends the track `id` for `reason` and takes it out of the queue: the next one plays if
    /// it was the current one. Whether it was in the queue.
    pub(crate) fn remove(&mut self, id: u64, reason: TrackEnd) -> bool {
        if self
            .current
            .as_ref()
            .is_some_and(|slot| slot.track.id == id)
        {
            if let Some(slot) = self.current.take() {
                slot.track.end(reason);
            }
            self.advance();
            return true;
        }
        if let Some(index) = self.waiting.iter().position(|slot| slot.track.id == id) {
            if let Some(slot) = self.waiting.remove(index) {
                slot.track.end(reason);
            }
            return true;
        }
        false
    }

    /// The call ended: so do its tracks.
    pub(crate) fn close(&mut self) {
        self.closed = true;
        for slot in self
            .current
            .take()
            .into_iter()
            .chain(self.waiting.drain(..))
        {
            slot.track.end(TrackEnd::CallEnded);
        }
    }
}

/// The queue of a [`Call`](crate::Call), from [`Call::queue`](crate::Call::queue): skip the
/// current track, clear the waiting ones, see what plays (D-352).
///
/// [`Call::enqueue`](crate::Call::enqueue) adds tracks to it; each plays when the one
/// before it ends. [`Call::play`](crate::Call::play) replaces the current track, and the
/// waiting ones play after it. A cheap-clone handle; its methods act at once.
///
/// discord.js counterpart: none (a bot keeps its own queue and plays the next resource on
/// `AudioPlayerStatus.Idle`); songbird: `TrackQueue`.
///
/// # Examples
///
/// ```no_run
/// # fn run(call: djs_voice::Call) {
/// use djs_voice::Input;
///
/// call.enqueue(Input::ffmpeg("one.mp3"));
/// call.enqueue(Input::ffmpeg("two.mp3"));
/// let queue = call.queue();
/// assert_eq!(queue.len(), 2);
/// queue.skip(); // "two.mp3" plays.
/// queue.clear(); // Nothing waits any more; "two.mp3" goes on.
/// # }
/// ```
#[derive(Clone)]
pub struct TrackQueue {
    pub(crate) shared: Arc<Shared>,
}

impl fmt::Debug for TrackQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrackQueue")
            .field("len", &self.len())
            .finish()
    }
}

impl TrackQueue {
    /// Ends the current track ([`TrackEnd::Skipped`]) and plays the next one; whether there
    /// was a current track.
    ///
    /// songbird counterpart: `TrackQueue::skip`.
    #[allow(
        clippy::must_use_candidate,
        reason = "skipping without looking at the result is fine"
    )]
    pub fn skip(&self) -> bool {
        let mut queue = lock(&self.shared.queue);
        let Some(id) = queue.current.as_ref().map(|slot| slot.track.id) else {
            return false;
        };
        queue.remove(id, TrackEnd::Skipped);
        drop(queue);
        self.shared.wake();
        true
    }

    /// Removes the waiting tracks ([`TrackEnd::Stopped`]); the current one goes on.
    ///
    /// songbird counterpart: `TrackQueue::stop` (which stops the current one too).
    ///
    /// discord.js counterpart: none (`@discordjs/voice` has no queue).
    pub fn clear(&self) {
        let waiting: Vec<Slot> = lock(&self.shared.queue).waiting.drain(..).collect();
        for slot in waiting {
            slot.track.end(TrackEnd::Stopped);
        }
    }

    /// The tracks in the queue: the current one and the waiting ones.
    ///
    /// songbird counterpart: `TrackQueue::len`.
    ///
    /// discord.js counterpart: none (`@discordjs/voice` has no queue).
    #[must_use]
    pub fn len(&self) -> usize {
        let queue = lock(&self.shared.queue);
        usize::from(queue.current.is_some()) + queue.waiting.len()
    }

    /// Whether nothing plays and nothing waits.
    ///
    /// discord.js counterpart: none (`@discordjs/voice` has no queue).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The track playing (or paused), if any.
    ///
    /// songbird counterpart: `TrackQueue::current`.
    ///
    /// discord.js counterpart: the resource of `AudioPlayer#state` (`@discordjs/voice`).
    #[must_use]
    pub fn current(&self) -> Option<TrackHandle> {
        lock(&self.shared.queue)
            .current
            .as_ref()
            .map(|slot| TrackHandle::new(Arc::clone(&slot.track)))
    }
}
