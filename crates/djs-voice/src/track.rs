//! The handle of a playing track (D-323, D-339).

use core::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::{TrackError, call::CallCommand};

/// What a [`TrackHandle`] asks of its track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Play,
    Pause,
    Stop,
}

/// What a track's handles and its call share: whether it ended.
#[derive(Debug, Default)]
pub(crate) struct TrackState {
    ended: AtomicBool,
}

impl TrackState {
    pub(crate) fn end(&self) {
        self.ended.store(true, Ordering::Release);
    }

    pub(crate) fn ended(&self) -> bool {
        self.ended.load(Ordering::Acquire)
    }
}

/// Controls a track that [`Call::play`](crate::Call::play) started: pause it, play it again,
/// stop it.
///
/// A cheap-clone handle: clones control the same track. The controls only ask the call to
/// act, so they return at once; they fail with [`TrackError::Ended`] once the track has
/// ended (played to its end, stopped, replaced by another, its input failed, or its call
/// ended). Dropping every handle does not stop the track.
///
/// discord.js counterpart: `AudioPlayer#pause`, `unpause` and `stop` of `@discordjs/voice`
/// (a player plays one resource); songbird: `TrackHandle`.
///
/// # Examples
///
/// ```no_run
/// # async fn run(call: djs_voice::Call) -> Result<(), Box<dyn std::error::Error>> {
/// use djs_voice::Input;
///
/// let track = call.play(Input::ogg_file("song.opus").await?);
/// track.pause()?;
/// track.play()?;
/// track.stop()?;
/// assert!(track.play().is_err(), "a stopped track has ended");
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct TrackHandle {
    inner: Arc<Inner>,
}

struct Inner {
    id: u64,
    state: Arc<TrackState>,
    commands: mpsc::UnboundedSender<CallCommand>,
}

impl fmt::Debug for TrackHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrackHandle")
            .field("id", &self.inner.id)
            .field("ended", &self.inner.state.ended())
            .finish()
    }
}

impl TrackHandle {
    pub(crate) fn new(
        id: u64,
        state: Arc<TrackState>,
        commands: mpsc::UnboundedSender<CallCommand>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                id,
                state,
                commands,
            }),
        }
    }

    /// Plays the track again after [`pause`](Self::pause), from where it paused; does
    /// nothing to a track that is playing.
    ///
    /// discord.js counterpart: `AudioPlayer#unpause`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended.
    pub fn play(&self) -> Result<(), TrackError> {
        self.control(Control::Play)
    }

    /// Pauses the track: it stops sending audio (after a few frames of silence, as Discord
    /// asks) and keeps its place; [`play`](Self::play) goes on from there.
    ///
    /// discord.js counterpart: `AudioPlayer#pause`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has ended.
    pub fn pause(&self) -> Result<(), TrackError> {
        self.control(Control::Pause)
    }

    /// Stops the track for good: it ends, and its input is no longer read.
    ///
    /// discord.js counterpart: `AudioPlayer#stop`.
    ///
    /// # Errors
    ///
    /// [`TrackError::Ended`] when the track has already ended.
    pub fn stop(&self) -> Result<(), TrackError> {
        self.control(Control::Stop)
    }

    fn control(&self, control: Control) -> Result<(), TrackError> {
        if self.inner.state.ended() {
            return Err(TrackError::Ended);
        }
        let command = CallCommand::Track {
            id: self.inner.id,
            control,
        };
        if self.inner.commands.send(command).is_err() {
            // The call is gone, and the track with it.
            self.inner.state.end();
            return Err(TrackError::Ended);
        }
        Ok(())
    }
}
