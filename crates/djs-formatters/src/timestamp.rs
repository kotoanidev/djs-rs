//! Timestamps shown in each reader's time zone and language: [`time`].
//!
//! See Discord's [timestamp styles](https://docs.discord.com/developers/reference#message-formatting-timestamp-styles).

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use djs_model::util::Timestamp;

/// How Discord shows a [`time`]; the examples are for 20 April 2021, 16:20:30, in an English
/// client.
///
/// discord.js counterpart: `TimestampStyles` (whose deprecated aliases `LongTime`,
/// `ShortDateTime` and `LongDateTime` are [`MediumTime`](Self::MediumTime),
/// [`LongDateShortTime`](Self::LongDateShortTime) and
/// [`FullDateShortTime`](Self::FullDateShortTime)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimestampStyle {
    /// `t`: 16:20.
    ShortTime,
    /// `T`: 16:20:30.
    MediumTime,
    /// `d`: 20/04/2021.
    ShortDate,
    /// `D`: April 20, 2021.
    LongDate,
    /// `f`: April 20, 2021 at 16:20 — what Discord shows without a style.
    LongDateShortTime,
    /// `F`: Tuesday, April 20, 2021 at 16:20.
    FullDateShortTime,
    /// `s`: 20/04/2021, 16:20.
    ShortDateShortTime,
    /// `S`: 20/04/2021, 16:20:30.
    ShortDateMediumTime,
    /// `R`: 4 years ago — kept up to date by the client.
    RelativeTime,
}

impl TimestampStyle {
    /// The style's letter, such as `R`.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::ShortTime => 't',
            Self::MediumTime => 'T',
            Self::ShortDate => 'd',
            Self::LongDate => 'D',
            Self::LongDateShortTime => 'f',
            Self::FullDateShortTime => 'F',
            Self::ShortDateShortTime => 's',
            Self::ShortDateMediumTime => 'S',
            Self::RelativeTime => 'R',
        }
    }
}

/// A point in time that [`time`] can show: seconds since the Unix epoch.
///
/// Implemented for `i64` (seconds), [`Timestamp`] and [`SystemTime`]; implement it for a
/// time type of another crate to pass it directly.
pub trait UnixSeconds {
    /// Whole seconds since 1970-01-01 00:00:00 UTC, rounded down (negative before it).
    fn unix_seconds(&self) -> i64;
}

impl UnixSeconds for i64 {
    fn unix_seconds(&self) -> i64 {
        *self
    }
}

impl UnixSeconds for Timestamp {
    fn unix_seconds(&self) -> i64 {
        self.as_unix_secs()
    }
}

impl UnixSeconds for SystemTime {
    /// Saturates at `i64::MIN` / `i64::MAX` for times billions of years away.
    fn unix_seconds(&self) -> i64 {
        match self.duration_since(UNIX_EPOCH) {
            Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
            Err(before) => {
                let before = before.duration();
                // Round down: 0.5 s before the epoch is second -1.
                let seconds = before.as_secs() + u64::from(before.subsec_nanos() > 0);
                i64::try_from(seconds).map_or(i64::MIN, |seconds| -seconds)
            }
        }
    }
}

impl<T: UnixSeconds + ?Sized> UnixSeconds for &T {
    fn unix_seconds(&self) -> i64 {
        (**self).unix_seconds()
    }
}

/// A timestamp: `<t:SECONDS>` or `<t:SECONDS:STYLE>`. Returned by [`time`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct Time {
    seconds: i64,
    style: Option<TimestampStyle>,
}

/// Shows `at` in each reader's time zone and language: `<t:1618953630>`.
///
/// Without a [`style`](Time::style) Discord uses
/// [`LongDateShortTime`](TimestampStyle::LongDateShortTime) (D-213). Discord counts whole
/// seconds, so a fraction is dropped (rounding down).
///
/// discord.js counterpart: `time`.
///
/// # Examples
///
/// ```
/// use djs_formatters::{TimestampStyle, time};
/// use djs_model::util::Timestamp;
///
/// assert_eq!(time(1_618_953_630).to_string(), "<t:1618953630>");
/// assert_eq!(time(1_618_953_630).style(TimestampStyle::RelativeTime).to_string(), "<t:1618953630:R>");
///
/// let sent = Timestamp::parse("2021-04-20T21:20:30.250+00:00").unwrap();
/// assert_eq!(time(sent).style(TimestampStyle::ShortDate).to_string(), "<t:1618953630:d>");
///
/// // Now, as "in a few seconds" / "a few seconds ago":
/// let now = time(std::time::SystemTime::now()).style(TimestampStyle::RelativeTime);
/// # let _ = now;
/// ```
pub fn time(at: impl UnixSeconds) -> Time {
    Time {
        seconds: at.unix_seconds(),
        style: None,
    }
}

impl Time {
    /// Sets how the time is shown.
    pub const fn style(mut self, style: TimestampStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// The seconds since the Unix epoch this timestamp shows.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        self.seconds
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.style {
            Some(style) => write!(f, "<t:{}:{}>", self.seconds, style.as_char()),
            None => write!(f, "<t:{}>", self.seconds),
        }
    }
}
