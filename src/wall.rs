//! Wall-clock readings.
//!
//! Wall-clock time is calendar time, sourced from the operating system's
//! real-time clock. It is the right tool for timestamps in logs, audit
//! records, and anything that needs to line up with what a wristwatch
//! shows. It is the wrong tool for measuring elapsed time — wall-clock
//! readings can jump backwards or forwards at any moment (NTP corrections,
//! DST changes, manual adjustments).
//!
//! For elapsed-time measurement, use [`Monotonic`](crate::Monotonic).

#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

/// A captured wall-clock instant.
///
/// `Wall` wraps a single sample of the operating system's real-time clock.
/// Convert it to Unix time with [`unix_seconds`](Wall::unix_seconds),
/// [`unix_millis`](Wall::unix_millis), or [`unix_nanos`](Wall::unix_nanos).
///
/// `Wall` and [`Monotonic`](crate::Monotonic) are deliberately distinct
/// types and cannot be mixed. If your system clock predates the Unix
/// epoch (1970-01-01 UTC), the `unix_*` accessors saturate at zero — they
/// never panic and never silently wrap.
///
/// Construct one with [`Wall::now`] or the crate-level
/// [`wall`](crate::wall) shortcut.
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
///
/// let stamp = Wall::now();
/// let seconds = stamp.unix_seconds();
/// assert!(seconds > 0);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wall(SystemTime);

#[cfg(feature = "std")]
impl Wall {
    /// Captures the current wall-clock time from the operating system.
    ///
    /// This is the constructor for [`Wall`]. The crate-level
    /// [`wall`](crate::wall) function is a one-line shortcut for the same
    /// thing.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let stamp = Wall::now();
    /// # let _ = stamp;
    /// ```
    #[inline]
    #[must_use]
    pub fn now() -> Self {
        Self(SystemTime::now())
    }

    /// Returns Unix time in whole seconds.
    ///
    /// Returns zero if the system clock is set to a moment before the Unix
    /// epoch. The return type is [`u64`], which is sufficient to represent
    /// any plausible wall-clock value through year 584,942,417,355 — Unix
    /// time will not overflow this accessor.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let seconds = Wall::now().unix_seconds();
    /// assert!(seconds > 0);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_seconds(self) -> u64 {
        self.0.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
    }

    /// Returns Unix time in whole milliseconds.
    ///
    /// Returns zero if the system clock is set to a moment before the Unix
    /// epoch. The return type is [`u128`], which cannot overflow for any
    /// representable [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let millis = Wall::now().unix_millis();
    /// assert!(millis > 0);
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
    /// Returns zero if the system clock is set to a moment before the Unix
    /// epoch. The return type is [`u128`], which cannot overflow for any
    /// representable [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Wall;
    ///
    /// let nanos = Wall::now().unix_nanos();
    /// assert!(nanos > 0);
    /// ```
    #[inline]
    #[must_use]
    pub fn unix_nanos(self) -> u128 {
        self.0
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    }
}
