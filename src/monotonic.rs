//! Monotonic clock readings.
//!
//! Monotonic time never goes backwards. It is the right tool for measuring
//! elapsed time: rate limiting, timeouts, benchmarks, retry backoff. The
//! absolute value of a [`Monotonic`] reading carries no calendar meaning —
//! it is only useful as a delta against another [`Monotonic`] from the same
//! process.
//!
//! For calendar timestamps, use [`Wall`](crate::Wall) instead.

use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration;

use crate::platform::{self, Instant};

/// A captured monotonic instant.
///
/// `Monotonic` wraps a single sample of a monotonic clock. Two readings can
/// be compared to recover the [`Duration`] between them with
/// [`duration_since`](Monotonic::duration_since), and a reading can be moved
/// by a `Duration` to build a deadline (`start + ttl`).
///
/// `Monotonic` and [`Wall`](crate::Wall) are deliberately distinct types
/// with no cross-type arithmetic. The compiler rejects any attempt to mix
/// them; that separation is the central design choice of this crate.
///
/// Get one from [`Monotonic::now`], the crate-level [`now`](crate::now)
/// shortcut, an injected [`Clock`](crate::Clock), or by converting a
/// [`std::time::Instant`] with [`From`].
///
/// # Arithmetic never panics
///
/// `std::time::Instant + Duration` panics when the result is not
/// representable. Every arithmetic operation on `Monotonic` either returns
/// an `Option` ([`checked_add`](Monotonic::checked_add),
/// [`checked_sub`](Monotonic::checked_sub)) or saturates at the
/// platform's representable range (the `+` and `-` operators,
/// [`saturating_add`](Monotonic::saturating_add),
/// [`saturating_sub`](Monotonic::saturating_sub)). `now + Duration::MAX` is
/// therefore a usable "never" deadline. The first overflow in a process
/// locates the platform's limit (about 1 µs); every later one reuses it and
/// costs a few nanoseconds.
///
/// # Resolution
///
/// The tick size is the platform's: 100 ns on Windows
/// (`QueryPerformanceCounter`), typically 1 ns on Linux and macOS. Two
/// back-to-back readings are often equal, which is correct behaviour for a
/// monotonic (non-decreasing, not strictly increasing) clock.
///
/// # Examples
///
/// ```
/// use clock_lib::Monotonic;
/// use std::time::Duration;
///
/// let start = Monotonic::now();
/// let deadline = start + Duration::from_secs(30);
/// assert!(deadline > start);
/// assert_eq!(deadline - start, Duration::from_secs(30));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Monotonic(pub(crate) Instant);

impl Monotonic {
    /// Captures the current monotonic time from the operating system.
    ///
    /// This always reads the OS clock. Code that receives an injected
    /// [`Clock`](crate::Clock) should call [`Clock::now`](crate::Clock::now)
    /// instead, so tests can control it.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    ///
    /// let a = Monotonic::now();
    /// let b = Monotonic::now();
    /// assert!(b >= a);
    /// ```
    #[inline]
    #[must_use]
    pub fn now() -> Self {
        Self(Instant::now())
    }

    /// Returns the time elapsed between this reading and **the operating
    /// system's** current monotonic time.
    ///
    /// Equivalent to `Monotonic::now().saturating_duration_since(self)`.
    ///
    /// Only call this on readings that came from the OS clock
    /// ([`Monotonic::now`], [`now`](crate::now),
    /// [`SystemClock`](crate::SystemClock)). For a reading taken from an
    /// injected clock such as [`ManualClock`](crate::ManualClock), use
    /// [`Clock::elapsed`](crate::Clock::elapsed). This method ignores the
    /// injected clock: it would measure real time and silently ignore every
    /// `advance`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let t = Monotonic::now();
    /// assert!(t.elapsed() < Duration::from_secs(60));
    /// ```
    #[inline]
    #[must_use]
    pub fn elapsed(self) -> Duration {
        self.0.elapsed()
    }

    /// Returns the [`Duration`] from `earlier` to `self`, or zero if
    /// `earlier` is later than `self`.
    ///
    /// This is the same operation as
    /// [`saturating_duration_since`](Monotonic::saturating_duration_since)
    /// and never panics. (`std::time::Instant::duration_since` currently
    /// saturates too, but the standard library reserves the right to make it
    /// panic again; this crate pins the saturating behaviour.) Use
    /// [`checked_duration_since`](Monotonic::checked_duration_since) when
    /// you need to tell "zero elapsed" apart from "wrong order".
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let earlier = Monotonic::now();
    /// let later = earlier + Duration::from_millis(5);
    /// assert_eq!(later.duration_since(earlier), Duration::from_millis(5));
    /// assert_eq!(earlier.duration_since(later), Duration::ZERO);
    /// ```
    #[inline]
    #[must_use]
    pub fn duration_since(self, earlier: Self) -> Duration {
        self.0.saturating_duration_since(earlier.0)
    }

    /// Returns `Some(duration)` if `self >= earlier`, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let earlier = Monotonic::now();
    /// let later = earlier + Duration::from_secs(2);
    /// assert_eq!(later.checked_duration_since(earlier), Some(Duration::from_secs(2)));
    /// assert_eq!(earlier.checked_duration_since(later), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        self.0.checked_duration_since(earlier.0)
    }

    /// Returns the duration since `earlier`, saturating at zero when
    /// `earlier` is later than `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let a = Monotonic::now();
    /// let b = a + Duration::from_secs(1);
    /// assert_eq!(b.saturating_duration_since(a), Duration::from_secs(1));
    /// assert_eq!(a.saturating_duration_since(b), Duration::ZERO);
    /// ```
    #[inline]
    #[must_use]
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.0.saturating_duration_since(earlier.0)
    }

    /// Returns `self + duration`, or `None` if the result is not
    /// representable on this platform.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let start = Monotonic::now();
    /// assert!(start.checked_add(Duration::from_secs(5)).is_some());
    /// assert!(start.checked_add(Duration::MAX).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    /// Returns `self - duration`, or `None` if the result is not
    /// representable on this platform.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let now = Monotonic::now();
    /// assert!(now.checked_sub(Duration::from_nanos(1)).is_some());
    /// assert!(now.checked_sub(Duration::MAX).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        self.0.checked_sub(duration).map(Self)
    }

    /// Returns `self + duration`, clamped to the latest instant this
    /// platform can represent.
    ///
    /// Use it for deadlines where "far enough in the future to never fire"
    /// is an acceptable answer to overflow, such as a TTL of
    /// [`Duration::MAX`]. The `+` operator does the same thing.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let start = Monotonic::now();
    /// let never = start.saturating_add(Duration::MAX);
    /// assert!(never > start);
    /// assert_eq!(never.saturating_add(Duration::from_secs(1)), never);
    /// ```
    #[inline]
    #[must_use]
    pub fn saturating_add(self, duration: Duration) -> Self {
        Self(platform::instant_add(self.0, duration))
    }

    /// Returns `self - duration`, clamped to the earliest instant this
    /// platform can represent. The `-` operator does the same thing.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    /// use std::time::Duration;
    ///
    /// let now = Monotonic::now();
    /// let floor = now.saturating_sub(Duration::MAX);
    /// assert!(floor < now);
    /// assert_eq!(floor.saturating_sub(Duration::from_secs(1)), floor);
    /// ```
    #[inline]
    #[must_use]
    pub fn saturating_sub(self, duration: Duration) -> Self {
        Self(platform::instant_sub(self.0, duration))
    }
}

/// Wraps a [`std::time::Instant`] so it can be used wherever this crate
/// expects a reading, for example in a custom [`Clock`](crate::Clock)
/// implementation. Lossless, and available with every feature combination.
///
/// # Examples
///
/// ```
/// use clock_lib::Monotonic;
/// use std::time::Instant;
///
/// let instant = Instant::now();
/// let reading = Monotonic::from(instant);
/// assert_eq!(Instant::from(reading), instant);
/// ```
impl From<Instant> for Monotonic {
    #[inline]
    fn from(instant: Instant) -> Self {
        Self(instant)
    }
}

/// Unwraps a reading into an `Instant`, for APIs that take one
/// (`Condvar::wait_timeout_while` deadlines, `Receiver::recv_deadline`,
/// `tokio::time::Instant::from_std`, and so on).
///
/// # Examples
///
/// ```
/// use clock_lib::Monotonic;
/// use std::time::{Duration, Instant};
///
/// let before = Instant::now();
/// let deadline: Instant = (Monotonic::now() + Duration::from_millis(10)).into();
/// assert!(deadline > before);
/// ```
impl From<Monotonic> for Instant {
    #[inline]
    fn from(reading: Monotonic) -> Self {
        reading.0
    }
}

/// `reading + duration`, saturating at the latest representable instant.
///
/// Unlike `Instant + Duration`, this never panics. See
/// [`Monotonic::saturating_add`].
impl Add<Duration> for Monotonic {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self {
        self.saturating_add(rhs)
    }
}

/// `reading += duration`, saturating at the latest representable instant.
impl AddAssign<Duration> for Monotonic {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.saturating_add(rhs);
    }
}

/// `reading - duration`, saturating at the earliest representable instant.
///
/// Unlike `Instant - Duration`, this never panics. See
/// [`Monotonic::saturating_sub`].
impl Sub<Duration> for Monotonic {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Duration) -> Self {
        self.saturating_sub(rhs)
    }
}

/// `reading -= duration`, saturating at the earliest representable instant.
impl SubAssign<Duration> for Monotonic {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.saturating_sub(rhs);
    }
}

/// `later - earlier`: the [`Duration`] between two readings, zero if
/// `earlier` is actually later. See [`Monotonic::saturating_duration_since`].
impl Sub for Monotonic {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Self) -> Duration {
        self.saturating_duration_since(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_since_reversed_returns_zero_not_panic() {
        let a = Monotonic::now();
        let b = a + Duration::from_secs(1);
        assert_eq!(a.duration_since(b), Duration::ZERO);
        assert_eq!(b.duration_since(a), Duration::from_secs(1));
    }

    #[test]
    fn test_operators_match_named_methods() {
        let a = Monotonic::now();
        let d = Duration::from_millis(250);
        assert_eq!(a + d, a.saturating_add(d));
        assert_eq!(a - d, a.saturating_sub(d));
        assert_eq!((a + d) - a, d);
        assert_eq!(a - (a + d), Duration::ZERO);

        let mut m = a;
        m += d;
        assert_eq!(m, a + d);
        m -= d;
        assert_eq!(m, a);
    }

    #[test]
    fn test_add_duration_max_saturates_without_panic() {
        let a = Monotonic::now();
        let top = a + Duration::MAX;
        assert!(top > a);
        assert_eq!(top + Duration::from_secs(1), top);
        assert_eq!(top.checked_add(Duration::from_nanos(1)), None);
    }

    #[test]
    fn test_sub_duration_max_saturates_without_panic() {
        let a = Monotonic::now();
        let bottom = a - Duration::MAX;
        assert!(bottom < a);
        assert_eq!(bottom - Duration::from_secs(1), bottom);
    }

    #[test]
    fn test_instant_round_trip_is_lossless() {
        let instant = Instant::now();
        let reading: Monotonic = instant.into();
        let back: Instant = reading.into();
        assert_eq!(back, instant);
    }
}
