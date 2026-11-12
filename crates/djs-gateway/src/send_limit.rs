//! The gateway send limit (D-142).
//!
//! Discord accepts 120 payloads per connection every 60 seconds and closes the connection of
//! a client that sends more ([`CloseCode::RateLimited`]). Commands queued through a
//! [`MessageSender`] are therefore paced by a [`SendLimit`]; heartbeats, identify and resume
//! are never held back and use the room left over.
//!
//! [`CloseCode::RateLimited`]: djs_model::gateway::CloseCode::RateLimited
//! [`MessageSender`]: crate::MessageSender

use core::time::Duration;
use std::collections::VecDeque;

use tokio::time::Instant;

/// The span the limit is counted over: Discord's 60 seconds plus a one-second margin, so
/// network jitter cannot push two windows' worth of commands into one of Discord's.
pub(crate) const WINDOW: Duration = Duration::from_secs(61);

/// How many queued commands may be sent in any [`WINDOW`]. Discord's limit is 120; the other
/// five payloads are left for heartbeats (at most two per minute at Discord's usual
/// 41.25-second interval), heartbeats Discord asks for, and identify or resume.
pub(crate) const COMMANDS_PER_WINDOW: usize = 115;

/// A sliding window over the send times of one connection's recent commands.
///
/// A sliding window (rather than a counter reset every minute) never allows more than
/// [`COMMANDS_PER_WINDOW`] commands in *any* span of [`WINDOW`], so it stays under Discord's
/// limit however Discord aligns its own window.
#[derive(Debug, Default)]
pub(crate) struct SendLimit {
    /// The send times of the commands still inside the window, oldest first. Never longer
    /// than [`COMMANDS_PER_WINDOW`].
    sent: VecDeque<Instant>,
}

impl SendLimit {
    /// Takes a slot for a command sent at `now`, if the window has one.
    ///
    /// # Errors
    ///
    /// Returns the instant at which the next slot frees up if the window is full.
    pub(crate) fn try_acquire(&mut self, now: Instant) -> Result<(), Instant> {
        while self.sent.front().is_some_and(|&at| at + WINDOW <= now) {
            self.sent.pop_front();
        }
        match self.sent.front() {
            Some(&oldest) if self.sent.len() >= COMMANDS_PER_WINDOW => Err(oldest + WINDOW),
            _ => {
                self.sent.push_back(now);
                Ok(())
            }
        }
    }

    /// Forgets every send: a new connection starts with a fresh limit.
    pub(crate) fn clear(&mut self) {
        self.sent.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS_PER_WINDOW, Duration, Instant, SendLimit, WINDOW};

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn allows_a_full_window_then_waits_for_the_oldest_send() {
        let start = Instant::now();
        let mut limit = SendLimit::default();
        for i in 0..COMMANDS_PER_WINDOW {
            let at = start + MS * u32::try_from(i).unwrap();
            assert_eq!(limit.try_acquire(at), Ok(()), "command {i}");
        }
        let now = start + Duration::from_secs(1);
        assert_eq!(limit.try_acquire(now), Err(start + WINDOW));
        // A rejected attempt takes no slot.
        assert_eq!(limit.sent.len(), COMMANDS_PER_WINDOW);
    }

    #[test]
    fn slots_free_up_one_by_one() {
        let start = Instant::now();
        let mut limit = SendLimit::default();
        for i in 0..COMMANDS_PER_WINDOW {
            let at = start + MS * u32::try_from(i).unwrap();
            limit.try_acquire(at).unwrap();
        }
        // The first send leaves the window; the second is still inside it.
        assert_eq!(limit.try_acquire(start + WINDOW), Ok(()));
        assert_eq!(limit.try_acquire(start + WINDOW), Err(start + MS + WINDOW));
        assert_eq!(limit.try_acquire(start + MS + WINDOW), Ok(()));
    }

    #[test]
    fn never_exceeds_the_limit_in_any_window() {
        let start = Instant::now();
        let mut limit = SendLimit::default();
        let mut sent = Vec::new();
        let mut now = start;
        // Try to send as fast as possible for five windows.
        while now < start + WINDOW * 5 {
            match limit.try_acquire(now) {
                Ok(()) => sent.push(now),
                Err(at) => now = at,
            }
        }
        for (i, &first) in sent.iter().enumerate() {
            let in_window = sent[i..].iter().take_while(|&&at| at < first + WINDOW);
            assert!(in_window.count() <= COMMANDS_PER_WINDOW);
        }
        assert_eq!(sent.len(), COMMANDS_PER_WINDOW * 5);
    }

    #[test]
    fn clear_starts_a_fresh_window() {
        let now = Instant::now();
        let mut limit = SendLimit::default();
        for _ in 0..COMMANDS_PER_WINDOW {
            limit.try_acquire(now).unwrap();
        }
        assert!(limit.try_acquire(now).is_err());
        limit.clear();
        assert_eq!(limit.try_acquire(now), Ok(()));
    }
}
