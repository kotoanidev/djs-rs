//! ISO 8601 timestamps as used by the Discord API.

use core::{fmt, str::FromStr};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Visitor},
};

/// A point in time, as sent by Discord in ISO 8601 format.
///
/// Discord timestamps look like `2026-11-10T19:42:07.123000+00:00`. They are stored here as a
/// single `i64` of **microseconds since the Unix epoch** (8 bytes, `Copy`), which is the
/// precision Discord uses. Parsing and formatting are implemented without any date/time
/// dependency.
///
/// Timestamps are ordered chronologically.
///
/// # Examples
///
/// ```
/// use djs_model::util::Timestamp;
///
/// let ts: Timestamp = "2021-01-01T12:30:00.5+00:00".parse().unwrap();
/// assert_eq!(ts.as_unix_millis(), 1_609_504_200_500);
/// assert_eq!(ts.to_string(), "2021-01-01T12:30:00.500000+00:00");
///
/// // Offsets are normalised to UTC.
/// let same: Timestamp = "2021-01-01T14:30:00.500+02:00".parse().unwrap();
/// assert_eq!(ts, same);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    unix_micros: i64,
}

const MICROS_PER_SECOND: i64 = 1_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

impl Timestamp {
    /// Creates a timestamp from microseconds since the Unix epoch.
    #[must_use]
    pub const fn from_unix_micros(unix_micros: i64) -> Self {
        Self { unix_micros }
    }

    /// Creates a timestamp from milliseconds since the Unix epoch.
    ///
    /// Returns `None` if the value overflows the microsecond representation.
    #[must_use]
    pub const fn from_unix_millis(unix_millis: i64) -> Option<Self> {
        match unix_millis.checked_mul(1_000) {
            Some(unix_micros) => Some(Self { unix_micros }),
            None => None,
        }
    }

    /// Creates a timestamp from seconds since the Unix epoch.
    ///
    /// Returns `None` if the value overflows the microsecond representation.
    #[must_use]
    pub const fn from_unix_secs(unix_secs: i64) -> Option<Self> {
        match unix_secs.checked_mul(MICROS_PER_SECOND) {
            Some(unix_micros) => Some(Self { unix_micros }),
            None => None,
        }
    }

    /// Returns the current system time.
    ///
    /// # Panics
    ///
    /// Panics if the system clock is set before 1970 or after the year 294 247.
    #[must_use]
    pub fn now() -> Self {
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is set before the Unix epoch");
        Self {
            unix_micros: i64::try_from(since_epoch.as_micros())
                .expect("system clock is too far in the future"),
        }
    }

    /// Returns the number of microseconds since the Unix epoch.
    #[must_use]
    pub const fn as_unix_micros(self) -> i64 {
        self.unix_micros
    }

    /// Returns the number of whole milliseconds since the Unix epoch (rounded towards negative
    /// infinity).
    #[must_use]
    pub const fn as_unix_millis(self) -> i64 {
        self.unix_micros.div_euclid(1_000)
    }

    /// Returns the number of whole seconds since the Unix epoch (rounded towards negative
    /// infinity).
    ///
    /// This is the value expected by Discord's `<t:…>` timestamp markdown.
    #[must_use]
    pub const fn as_unix_secs(self) -> i64 {
        self.unix_micros.div_euclid(MICROS_PER_SECOND)
    }

    /// Parses an ISO 8601 / RFC 3339 timestamp such as `2026-11-10T19:42:07.123000+00:00`.
    ///
    /// Accepted format: `YYYY-MM-DDTHH:MM:SS`, an optional fraction of 1–9 digits (digits
    /// beyond microseconds are truncated), and a `Z` or `±HH:MM` offset. A space is accepted in
    /// place of `T`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseTimestampError`] if the input is not in that format or contains an
    /// out-of-range component (for example month `13` or `February 30`).
    pub fn parse(input: &str) -> Result<Self, ParseTimestampError> {
        parse(input.as_bytes()).ok_or(ParseTimestampError(()))
    }
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Formats as `YYYY-MM-DDTHH:MM:SS.ffffff+00:00`, the format Discord itself uses.
impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self
            .unix_micros
            .div_euclid(SECONDS_PER_DAY * MICROS_PER_SECOND);
        let micros_of_day = self
            .unix_micros
            .rem_euclid(SECONDS_PER_DAY * MICROS_PER_SECOND);
        let (year, month, day) = civil_from_days(days);
        let seconds_of_day = micros_of_day / MICROS_PER_SECOND;
        let micros = micros_of_day % MICROS_PER_SECOND;
        write!(
            f,
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{micros:06}+00:00",
            seconds_of_day / 3_600,
            seconds_of_day / 60 % 60,
            seconds_of_day % 60,
        )
    }
}

/// The error returned when parsing a [`Timestamp`] fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseTimestampError(());

impl fmt::Display for ParseTimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid ISO 8601 timestamp")
    }
}

impl core::error::Error for ParseTimestampError {}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TimestampVisitor;

        impl Visitor<'_> for TimestampVisitor {
            type Value = Timestamp;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an ISO 8601 timestamp string")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<Self::Value, E> {
                Timestamp::parse(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(TimestampVisitor)
    }
}

/// Parses `len` ASCII digits starting at `at`.
fn digits(input: &[u8], at: usize, len: usize) -> Option<i64> {
    let slice = input.get(at..at + len)?;
    let mut value = 0_i64;
    for &byte in slice {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value * 10 + i64::from(byte - b'0');
    }
    Some(value)
}

fn expect(input: &[u8], at: usize, byte: u8) -> Option<()> {
    (input.get(at) == Some(&byte)).then_some(())
}

fn parse(input: &[u8]) -> Option<Timestamp> {
    // YYYY-MM-DDTHH:MM:SS
    let year = digits(input, 0, 4)?;
    expect(input, 4, b'-')?;
    let month = digits(input, 5, 2)?;
    expect(input, 7, b'-')?;
    let day = digits(input, 8, 2)?;
    if !matches!(input.get(10), Some(b'T' | b't' | b' ')) {
        return None;
    }
    let hour = digits(input, 11, 2)?;
    expect(input, 13, b':')?;
    let minute = digits(input, 14, 2)?;
    expect(input, 16, b':')?;
    let second = digits(input, 17, 2)?;

    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    // Optional fraction: 1–9 digits, truncated to microseconds.
    let mut at = 19;
    let mut micros = 0_i64;
    if input.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while let Some(byte) = input.get(at).filter(|byte| byte.is_ascii_digit()) {
            if at - start < 6 {
                micros = micros * 10 + i64::from(byte - b'0');
            }
            at += 1;
        }
        let len = at - start;
        if len == 0 || len > 9 {
            return None;
        }
        for _ in len..6 {
            micros *= 10;
        }
    }

    // Offset: `Z` or `±HH:MM`.
    let offset_seconds = match input.get(at)? {
        b'Z' | b'z' => {
            at += 1;
            0
        }
        sign @ (b'+' | b'-') => {
            let offset_hour = digits(input, at + 1, 2)?;
            expect(input, at + 3, b':')?;
            let offset_minute = digits(input, at + 4, 2)?;
            if offset_hour > 23 || offset_minute > 59 {
                return None;
            }
            at += 6;
            let offset = offset_hour * 3_600 + offset_minute * 60;
            if *sign == b'+' { offset } else { -offset }
        }
        _ => return None,
    };
    if at != input.len() {
        return None;
    }

    let days = days_from_civil(year, month, day);
    let seconds = days * SECONDS_PER_DAY + hour * 3_600 + minute * 60 + second - offset_seconds;
    Some(Timestamp::from_unix_micros(
        seconds * MICROS_PER_SECOND + micros,
    ))
}

const fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
///
/// Algorithm by Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms".
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12; // March = 0
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Inverse of [`days_from_civil`].
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_discord_formats() {
        let with_fraction: Timestamp = "2017-07-11T17:27:07.299000+00:00".parse().unwrap();
        assert_eq!(with_fraction.as_unix_millis(), 1_499_794_027_299);

        let without_fraction: Timestamp = "2017-07-11T17:27:07+00:00".parse().unwrap();
        assert_eq!(without_fraction.as_unix_secs(), 1_499_794_027);

        let zulu: Timestamp = "2017-07-11T17:27:07.299Z".parse().unwrap();
        assert_eq!(zulu, with_fraction);
    }

    #[test]
    fn nanosecond_fraction_is_truncated() {
        let ts: Timestamp = "1970-01-01T00:00:00.123456789Z".parse().unwrap();
        assert_eq!(ts.as_unix_micros(), 123_456);
    }

    #[test]
    fn display_round_trip() {
        for input in [
            "1970-01-01T00:00:00.000000+00:00",
            "2000-02-29T23:59:59.999999+00:00",
            "2026-11-10T19:42:07.123000+00:00",
            "1969-12-31T23:59:59.500000+00:00",
        ] {
            let ts: Timestamp = input.parse().unwrap();
            assert_eq!(ts.to_string(), input);
        }
    }

    #[test]
    fn negative_offset() {
        let ts: Timestamp = "2021-01-01T00:00:00-05:30".parse().unwrap();
        assert_eq!(ts.to_string(), "2021-01-01T05:30:00.000000+00:00");
    }

    #[test]
    fn rejects_invalid() {
        for input in [
            "",
            "2021-13-01T00:00:00Z",
            "2021-02-29T00:00:00Z",
            "2021-01-01T24:00:00Z",
            "2021-01-01T00:00:00",
            "2021-01-01T00:00:00.Z",
            "2021-01-01T00:00:00.1234567890Z",
            "2021-01-01T00:00:00+0000",
            "2021-01-01T00:00:00Zjunk",
            "2021-1-01T00:00:00Z",
        ] {
            assert!(
                input.parse::<Timestamp>().is_err(),
                "{input:?} should be rejected"
            );
        }
    }

    #[test]
    fn civil_round_trip() {
        for days in (-800_000..800_000).step_by(997) {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
        }
    }
}
