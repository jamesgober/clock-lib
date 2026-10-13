//! Wall-clock readings.
//!
//! Wall-clock time is calendar time, sourced from the operating system's
//! real-time clock. It is the right tool for timestamps in logs, audit
//! records, and anything that needs to line up with what a wristwatch
//! shows. It is the wrong tool for measuring elapsed time: wall-clock
//! readings can jump backwards or forwards at any moment (NTP steps, manual
//! changes, VM suspend/resume or migration, leap-second handling).
//!
//! For elapsed-time measurement, use [`Monotonic`](crate::Monotonic).

use core::fmt;
use core::time::Duration;

use crate::platform::{self, SystemTime, UNIX_EPOCH};
use crate::rfc3339;

/// A captured wall-clock instant.
///
/// `Wall` wraps a single sample of a real-time clock. Convert it to Unix
/// time with [`unix_seconds`](Wall::unix_seconds),
/// [`unix_millis`](Wall::unix_millis) or [`unix_nanos`](Wall::unix_nanos),
/// format it as RFC 3339 with [`Display`](#impl-Display-for-Wall), or build
/// one at a known time with [`from_unix_seconds`](Wall::from_unix_seconds)
/// and friends.
///
/// `Wall` and [`Monotonic`](crate::Monotonic) are deliberately distinct
/// types and cannot be mixed.
///
/// # Unix time and the epoch
///
/// Unix time is UTC and does not count leap seconds. Daylight-saving time
/// never affects it; only the local-time *presentation* changes. The
/// unsigned accessors (`unix_seconds`, `unix_millis`, `unix_nanos`) saturate
/// to `0` for moments before 1970-01-01T00:00:00Z. They never panic and
/// never wrap, but they cannot tell a pre-epoch reading from the epoch. Use
/// the `_signed` accessors when pre-epoch values matter.
///
/// # Resolution
///
/// `Wall` has the resolution of the platform's `SystemTime`: 100 ns on
/// Windows (`FILETIME`), 1 ns on Linux and macOS. Values built with
/// [`from_unix_nanos`](Wall::from_unix_nanos) or moved with
/// [`checked_add`](Wall::checked_add) are truncated to that tick, and
/// `unix_nanos()` on Windows is always a multiple of 100.
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
///
/// let stamp = Wall::now();
/// assert!(stamp.unix_seconds() > 1_704_067_200); // after 2024-01-01
///
/// let launch = Wall::from_unix_seconds(1_700_000_000).expect("in range");
/// assert_eq!(launch.to_string(), "2023-11-14T22:13:20Z");
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wall(pub(crate) SystemTime);

impl Wall {
    /// The Unix epoch, 1970-01-01T00:00:00Z.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// assert_eq!(Wall::UNIX_EPOCH.unix_nanos(), 0);
    /// assert_eq!(Wall::UNIX_EPOCH.to_string(), "1970-01-01T00:00:00Z");
    /// ```
    pub const UNIX_EPOCH: Self = Self(UNIX_EPOCH);

    /// Captures the current wall-clock time from the operating system.
    ///
    /// This always reads the OS clock. Code that receives an injected
    /// [`Clock`](crate::Clock) should call
    /// [`Clock::wall`](crate::Clock::wall) instead, so tests can control it.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let stamp = Wall::now();
    /// assert!(stamp > Wall::UNIX_EPOCH);
    /// ```
    #[inline]
    #[must_use]
    pub fn now() -> Self {
        Self(SystemTime::now())
    }

    /// Builds the reading `secs` whole seconds after the Unix epoch.
    ///
    /// Returns `None` if the moment is not representable on this platform
    /// (for example, beyond the year 30828 on Windows).
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let stamp = Wall::from_unix_seconds(1_700_000_000).expect("in range");
    /// assert_eq!(stamp.unix_seconds(), 1_700_000_000);
    /// assert!(Wall::from_unix_seconds(u64::MAX).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub fn from_unix_seconds(secs: u64) -> Option<Self> {
        Self::UNIX_EPOCH.checked_add(Duration::from_secs(secs))
    }

    /// Builds the reading `millis` milliseconds after the Unix epoch.
    ///
    /// Accepts the same type [`unix_millis`](Wall::unix_millis) returns, so
    /// the two round-trip. Returns `None` if the moment is not representable
    /// on this platform.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let stamp = Wall::from_unix_millis(1_700_000_000_123).expect("in range");
    /// assert_eq!(stamp.unix_millis(), 1_700_000_000_123);
    /// assert_eq!(stamp.to_string(), "2023-11-14T22:13:20.123Z");
    /// ```
    #[inline]
    #[must_use]
    pub fn from_unix_millis(millis: u128) -> Option<Self> {
        Self::UNIX_EPOCH.checked_add(platform::duration_from_units(millis, 1_000)?)
    }

    /// Builds the reading `nanos` nanoseconds after the Unix epoch.
    ///
    /// Accepts the same type [`unix_nanos`](Wall::unix_nanos) returns, so
    /// the two round-trip (to the platform's resolution: on Windows the
    /// value is truncated to a multiple of 100 ns). Returns `None` if the
    /// moment is not representable on this platform.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let stamp = Wall::from_unix_nanos(1_700_000_000_123_456_700).expect("in range");
    /// assert_eq!(stamp.unix_nanos(), 1_700_000_000_123_456_700);
    /// ```
    #[inline]
    #[must_use]
    pub fn from_unix_nanos(nanos: u128) -> Option<Self> {
        Self::UNIX_EPOCH.checked_add(platform::duration_from_units(nanos, 1_000_000_000)?)
    }

    /// Returns `self + duration`, or `None` if the result is not
    /// representable on this platform.
    ///
    /// Useful for computing expiry timestamps (a token's `exp`, a cookie's
    /// `Expires`). For deadlines inside the process, use
    /// [`Monotonic`](crate::Monotonic) arithmetic instead, since wall time
    /// can jump.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    /// use std::time::Duration;
    ///
    /// let issued = Wall::from_unix_seconds(1_700_000_000).expect("in range");
    /// let expires = issued.checked_add(Duration::from_secs(3_600)).expect("in range");
    /// assert_eq!(expires.unix_seconds(), 1_700_003_600);
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    /// Returns `self - duration`, or `None` if the result is not
    /// representable on this platform. The result may be before the Unix
    /// epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    /// use std::time::Duration;
    ///
    /// let before_epoch = Wall::UNIX_EPOCH.checked_sub(Duration::from_secs(1)).expect("in range");
    /// assert_eq!(before_epoch.unix_seconds_signed(), -1);
    /// assert_eq!(before_epoch.to_string(), "1969-12-31T23:59:59Z");
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        self.0.checked_sub(duration).map(Self)
    }

    /// Returns Unix time in whole seconds.
    ///
    /// Returns zero if the reading is before the Unix epoch; use
    /// [`unix_seconds_signed`](Wall::unix_seconds_signed) to see negative
    /// values. The [`u64`] range reaches far beyond any date a platform
    /// clock can represent, so this accessor cannot overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let seconds = Wall::now().unix_seconds();
    /// assert!(seconds > 1_704_067_200);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_seconds(self) -> u64 {
        self.0.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
    }

    /// Returns Unix time in whole milliseconds.
    ///
    /// Returns zero if the reading is before the Unix epoch. The return type
    /// is [`u128`], which cannot overflow for any representable time.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let millis = Wall::now().unix_millis();
    /// assert!(millis > 1_704_067_200_000);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_millis(self) -> u128 {
        self.0
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis())
    }

    /// Returns Unix time in whole nanoseconds.
    ///
    /// Returns zero if the reading is before the Unix epoch. The return type
    /// is [`u128`], which cannot overflow for any representable time. On
    /// Windows the value is always a multiple of 100 (see
    /// [Resolution](Wall#resolution)).
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let nanos = Wall::now().unix_nanos();
    /// assert!(nanos > 1_704_067_200_000_000_000);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_nanos(self) -> u128 {
        self.0
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    }

    /// Returns signed Unix time in whole seconds, rounded toward negative
    /// infinity.
    ///
    /// Pre-epoch readings return negative values: half a second before the
    /// epoch is `-1`, matching `time_t` conventions and the calendar
    /// (1969-12-31T23:59:59.5Z). Values outside the `i64` range saturate at
    /// `i64::MIN` / `i64::MAX`; no platform clock reaches them.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    /// use std::time::Duration;
    ///
    /// let half_before = Wall::UNIX_EPOCH.checked_sub(Duration::from_millis(500)).expect("in range");
    /// assert_eq!(half_before.unix_seconds_signed(), -1);
    /// assert_eq!(half_before.unix_seconds(), 0); // the unsigned accessor saturates
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_seconds_signed(self) -> i64 {
        let (secs, _) = self.unix_parts();
        platform::saturate_i64(secs)
    }

    /// Returns signed Unix time in whole milliseconds, rounded toward
    /// negative infinity.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    /// use std::time::Duration;
    ///
    /// let before = Wall::UNIX_EPOCH.checked_sub(Duration::from_micros(1_500)).expect("in range");
    /// assert_eq!(before.unix_millis_signed(), -2);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_millis_signed(self) -> i128 {
        let (secs, nanos) = self.unix_parts();
        secs * 1_000 + i128::from(nanos / 1_000_000)
    }

    /// Returns signed Unix time in nanoseconds. Exact; no rounding.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    /// use std::time::Duration;
    ///
    /// let before = Wall::UNIX_EPOCH.checked_sub(Duration::from_micros(3)).expect("in range");
    /// assert_eq!(before.unix_nanos_signed(), -3_000);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_nanos_signed(self) -> i128 {
        let (secs, nanos) = self.unix_parts();
        secs * 1_000_000_000 + i128::from(nanos)
    }

    /// Distance from the Unix epoch: `Ok` for at-or-after, `Err` holding the
    /// magnitude for before.
    #[inline]
    pub(crate) fn epoch_offset(self) -> Result<Duration, Duration> {
        self.0
            .duration_since(UNIX_EPOCH)
            .map_err(|before| before.duration())
    }

    /// Floor Unix seconds and the non-negative nanosecond remainder
    /// (`0..1_000_000_000`), the canonical signed decomposition.
    #[inline]
    fn unix_parts(self) -> (i128, u32) {
        match self.epoch_offset() {
            Ok(after) => (i128::from(after.as_secs()), after.subsec_nanos()),
            Err(before) => {
                let secs = i128::from(before.as_secs());
                match before.subsec_nanos() {
                    0 => (-secs, 0),
                    sub => (-secs - 1, 1_000_000_000 - sub),
                }
            }
        }
    }
}

/// Wraps a [`std::time::SystemTime`] so it can be used wherever this crate
/// expects a reading, for example in a custom [`Clock`](crate::Clock)
/// implementation. Lossless, and available with every feature combination.
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
/// use std::time::{Duration, SystemTime, UNIX_EPOCH};
///
/// let time = UNIX_EPOCH + Duration::from_secs(86_400);
/// let stamp = Wall::from(time);
/// assert_eq!(stamp.to_string(), "1970-01-02T00:00:00Z");
/// assert_eq!(SystemTime::from(stamp), time);
/// ```
impl From<SystemTime> for Wall {
    #[inline]
    fn from(time: SystemTime) -> Self {
        Self(time)
    }
}

/// Unwraps a reading into a `SystemTime`, for APIs that take one
/// (`File::set_modified`, HTTP date headers, `chrono`/`time` conversions).
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
/// use std::time::SystemTime;
///
/// let modified: SystemTime = Wall::now().into();
/// assert!(modified > SystemTime::UNIX_EPOCH);
/// ```
impl From<Wall> for SystemTime {
    #[inline]
    fn from(reading: Wall) -> Self {
        reading.0
    }
}

/// Formats the reading as an RFC 3339 timestamp in UTC, for example
/// `2023-11-14T22:13:20.250Z`.
///
/// - Without a precision, the fractional part uses the shortest of 0, 3, 6
///   or 9 digits that is exact.
/// - With a precision (`{:.0}` to `{:.9}`), exactly that many digits are
///   written, truncated rather than rounded so the output never rolls into
///   the next second. Precisions above 9 are treated as 9.
/// - Width, fill and alignment are honoured (`{:>30}`).
///
/// Years outside `0000`–`9999` are written in the ISO 8601 expanded form
/// (`+10000-01-01T00:00:00Z`), which is outside RFC 3339 proper. Leap
/// seconds never appear, because Unix time does not count them.
///
/// Formatting does not allocate.
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
///
/// let stamp = Wall::from_unix_millis(1_700_000_000_250).expect("in range");
/// assert_eq!(stamp.to_string(), "2023-11-14T22:13:20.250Z");
/// assert_eq!(format!("{stamp:.0}"), "2023-11-14T22:13:20Z");
/// assert_eq!(format!("{stamp:.6}"), "2023-11-14T22:13:20.250000Z");
/// ```
impl fmt::Display for Wall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (secs, nanos) = self.unix_parts();
        rfc3339::fmt(f, secs, nanos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn before_epoch(d: Duration) -> Wall {
        Wall::UNIX_EPOCH.checked_sub(d).unwrap_or(Wall::UNIX_EPOCH)
    }

    #[test]
    fn test_unsigned_accessors_saturate_to_zero_before_epoch() {
        let w = before_epoch(Duration::from_secs(10));
        assert_eq!(w.unix_seconds(), 0);
        assert_eq!(w.unix_millis(), 0);
        assert_eq!(w.unix_nanos(), 0);
    }

    #[test]
    fn test_unix_parts_floor_before_epoch() {
        assert_eq!(Wall::UNIX_EPOCH.unix_parts(), (0, 0));
        assert_eq!(before_epoch(Duration::from_secs(2)).unix_parts(), (-2, 0));
        assert_eq!(
            before_epoch(Duration::from_millis(1_500)).unix_parts(),
            (-2, 500_000_000)
        );
    }

    #[test]
    fn test_signed_accessors_agree_with_parts() {
        let w = before_epoch(Duration::from_millis(1_500));
        assert_eq!(w.unix_seconds_signed(), -2);
        assert_eq!(w.unix_millis_signed(), -1_500);
        assert_eq!(w.unix_nanos_signed(), -1_500_000_000);

        let after = Wall::from_unix_millis(2_250).unwrap_or(Wall::UNIX_EPOCH);
        assert_eq!(after.unix_seconds_signed(), 2);
        assert_eq!(after.unix_millis_signed(), 2_250);
        assert_eq!(after.unix_nanos_signed(), 2_250_000_000);
    }

    #[test]
    fn test_epoch_offset_reports_side_of_epoch() {
        assert_eq!(Wall::UNIX_EPOCH.epoch_offset(), Ok(Duration::ZERO));
        assert_eq!(
            before_epoch(Duration::from_secs(3)).epoch_offset(),
            Err(Duration::from_secs(3))
        );
    }

    #[test]
    fn test_display_honours_width_fill_and_alignment() {
        let w = Wall::UNIX_EPOCH;
        assert_eq!(format!("{w:>22}"), "  1970-01-01T00:00:00Z");
        assert_eq!(format!("{w:<22}|"), "1970-01-01T00:00:00Z  |");
        assert_eq!(format!("{w:*^24}"), "**1970-01-01T00:00:00Z**");
        assert_eq!(format!("{w:5}"), "1970-01-01T00:00:00Z");
        assert_eq!(format!("{w:>26.3}"), "  1970-01-01T00:00:00.000Z");
    }
}
