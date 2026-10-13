//! Elapsed-time measurement with laps, against any clock.

use core::time::Duration;

use crate::{Clock, Monotonic, SystemClock};

/// Measures elapsed time, with optional laps, against a [`Clock`].
///
/// A `Stopwatch` remembers when it started and when the last lap ended.
/// [`elapsed`](Stopwatch::elapsed) is the total since the start;
/// [`lap`](Stopwatch::lap) returns the time since the previous lap and
/// begins a new one. Like [`Deadline`](crate::Deadline), it is a pair of
/// readings plus a clock, not a running timer.
///
/// The clock type defaults to [`SystemClock`]. Build it with
/// [`start_with`](Stopwatch::start_with) to measure against an injected
/// clock, so a test can use `&ManualClock` and assert exact durations.
///
/// # Examples
///
/// ```
/// use clock_lib::{ManualClock, Stopwatch};
/// use std::time::Duration;
///
/// let clock = ManualClock::new();
/// let mut watch = Stopwatch::start_with(&clock);
///
/// clock.advance(Duration::from_millis(120));
/// assert_eq!(watch.lap(), Duration::from_millis(120));
///
/// clock.advance(Duration::from_millis(80));
/// assert_eq!(watch.lap(), Duration::from_millis(80));
/// assert_eq!(watch.elapsed(), Duration::from_millis(200));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Clone, Copy)]
pub struct Stopwatch<C = SystemClock> {
    clock: C,
    started_at: Monotonic,
    lap_started_at: Monotonic,
}

impl Stopwatch<SystemClock> {
    /// Starts a stopwatch on the operating system clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Stopwatch;
    /// use std::time::Duration;
    ///
    /// let watch = Stopwatch::start();
    /// // ... work ...
    /// assert!(watch.elapsed() < Duration::from_secs(60));
    /// ```
    #[inline]
    #[must_use]
    pub fn start() -> Self {
        Self::start_with(SystemClock)
    }
}

impl<C: Clock> Stopwatch<C> {
    /// Starts a stopwatch on `clock`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Stopwatch};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let watch = Stopwatch::start_with(&clock);
    /// clock.advance(Duration::from_secs(2));
    /// assert_eq!(watch.elapsed(), Duration::from_secs(2));
    /// ```
    #[inline]
    #[must_use]
    pub fn start_with(clock: C) -> Self {
        let now = clock.now();
        Self {
            clock,
            started_at: now,
            lap_started_at: now,
        }
    }

    /// Returns the time since the stopwatch started (or was last
    /// [restarted](Stopwatch::restart)). Laps do not reset it.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Stopwatch};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let mut watch = Stopwatch::start_with(&clock);
    /// clock.advance(Duration::from_secs(1));
    /// let _ = watch.lap();
    /// clock.advance(Duration::from_secs(1));
    /// assert_eq!(watch.elapsed(), Duration::from_secs(2));
    /// ```
    #[inline]
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed(self.started_at)
    }

    /// Returns the time since the previous lap (or the start) and begins a
    /// new lap.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Stopwatch};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let mut watch = Stopwatch::start_with(&clock);
    /// clock.advance(Duration::from_millis(30));
    /// assert_eq!(watch.lap(), Duration::from_millis(30));
    /// assert_eq!(watch.lap(), Duration::ZERO);
    /// ```
    #[inline]
    pub fn lap(&mut self) -> Duration {
        let now = self.clock.now();
        let lap = now.saturating_duration_since(self.lap_started_at);
        self.lap_started_at = now;
        lap
    }

    /// Returns the total elapsed time and starts again from zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Stopwatch};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let mut watch = Stopwatch::start_with(&clock);
    /// clock.advance(Duration::from_secs(4));
    /// assert_eq!(watch.restart(), Duration::from_secs(4));
    /// assert_eq!(watch.elapsed(), Duration::ZERO);
    /// ```
    #[inline]
    pub fn restart(&mut self) -> Duration {
        let now = self.clock.now();
        let total = now.saturating_duration_since(self.started_at);
        self.started_at = now;
        self.lap_started_at = now;
        total
    }

    /// Returns the reading taken when the stopwatch started (or was last
    /// restarted).
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, ManualClock, Stopwatch};
    ///
    /// let clock = ManualClock::new();
    /// let watch = Stopwatch::start_with(&clock);
    /// assert_eq!(watch.started_at(), clock.now());
    /// ```
    #[inline]
    #[must_use]
    pub const fn started_at(&self) -> Monotonic {
        self.started_at
    }

    /// Returns the clock this stopwatch reads.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Stopwatch};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let watch = Stopwatch::start_with(&clock);
    /// watch.clock().advance(Duration::from_secs(1));
    /// assert_eq!(watch.elapsed(), Duration::from_secs(1));
    /// ```
    #[inline]
    #[must_use]
    pub const fn clock(&self) -> &C {
        &self.clock
    }
}
