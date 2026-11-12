//! Monotonic clock readings.
//!
//! Monotonic time never goes backwards. It is the right tool for measuring
//! elapsed time: rate limiting, timeouts, benchmarks, retry backoff. The
//! absolute value of a [`Monotonic`] reading carries no calendar meaning —
//! it is only useful as a delta against another [`Monotonic`] from the same
//! process.
//!
//! For calendar timestamps, use [`Wall`](crate::Wall) instead.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
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
/// # Readings from different clocks
///
/// A reading from a [`ManualClock`](crate::ManualClock) has the same type
/// as one from the operating system clock, but the two do not measure the
/// same thing: the manual clock moves only when it is told to. Measure a
/// reading with the clock it came from, for example
/// [`Clock::elapsed`](crate::Clock::elapsed). In debug builds every reading
/// remembers its clock, and [`elapsed`](Monotonic::elapsed),
/// [`duration_since`](Monotonic::duration_since) and the other
/// `*_duration_since` methods, and `reading - reading`, panic when they are
/// given readings of different clocks (a `ManualClock` and the operating
/// system clock, or two `ManualClock`s), instead of returning a plausible
/// but wrong duration. Release builds keep only the instant and check
/// nothing. Comparisons (`==`, `<`) and conversions to
/// [`std::time::Instant`] are never checked.
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
#[derive(Copy, Clone)]
pub struct Monotonic(
    pub(crate) Instant,
    /// The clock the reading came from (debug builds only).
    #[cfg(debug_assertions)]
    pub(crate) Source,
);

/// Which clock a reading came from: [`Source::OS`] for the operating
/// system clock, a distinct value for each [`ManualClock`](crate::ManualClock).
/// Only tracked in debug builds, to catch readings of different clocks
/// being measured against each other.
#[cfg(debug_assertions)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Source(pub(crate) u32);

#[cfg(debug_assertions)]
impl Source {
    /// The operating system clock, and readings built from an `Instant`.
    pub(crate) const OS: Self = Self(0);
}

#[cfg(debug_assertions)]
const MIXED: &str = "clock-lib: a reading was measured against a reading from a \
                     different clock (a ManualClock and the operating system clock, or two \
                     ManualClocks); measure a reading with the clock it came from, for \
                     example `clock.elapsed(reading)`";

#[cfg(debug_assertions)]
const NOT_OS: &str = "clock-lib: Monotonic::elapsed and clock_lib::elapsed read the \
                      operating system clock, but this reading came from a ManualClock, so \
                      every advance would be ignored; use `clock.elapsed(reading)`";

impl Monotonic {
    /// A reading of the operating system clock.
    #[inline]
    pub(crate) const fn from_os(instant: Instant) -> Self {
        Self(
            instant,
            #[cfg(debug_assertions)]
            Source::OS,
        )
    }

    /// A reading at `instant` from the same clock as `self`.
    #[inline]
    #[cfg_attr(not(debug_assertions), allow(clippy::unused_self))]
    const fn moved_to(self, instant: Instant) -> Self {
        Self(
            instant,
            #[cfg(debug_assertions)]
            self.1,
        )
    }

    /// Debug builds: panics unless both readings came from the same clock.
    #[inline]
    #[track_caller]
    #[cfg_attr(not(debug_assertions), allow(clippy::unused_self))]
    fn same_clock(self, other: Self) {
        #[cfg(debug_assertions)]
        debug_assert!(self.1 == other.1, "{MIXED}");
        #[cfg(not(debug_assertions))]
        let _ = other;
    }

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
        Self::from_os(Instant::now())
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
    #[track_caller]
    pub fn elapsed(self) -> Duration {
        #[cfg(debug_assertions)]
        debug_assert!(self.1 == Source::OS, "{NOT_OS}");
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
    #[track_caller]
    pub fn duration_since(self, earlier: Self) -> Duration {
        self.same_clock(earlier);
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
    #[track_caller]
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        self.same_clock(earlier);
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
    #[track_caller]
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.same_clock(earlier);
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
        self.0.checked_add(duration).map(|i| self.moved_to(i))
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
        self.0.checked_sub(duration).map(|i| self.moved_to(i))
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
        self.moved_to(platform::instant_add(self.0, duration))
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
        self.moved_to(platform::instant_sub(self.0, duration))
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
        Self::from_os(instant)
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

/// Shows the instant only, as `Monotonic(Instant { .. })`.
impl fmt::Debug for Monotonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Monotonic").field(&self.0).finish()
    }
}

/// Compares the instants only; the clock a reading came from is not part
/// of its value.
impl PartialEq for Monotonic {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Monotonic {}

impl PartialOrd for Monotonic {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Monotonic {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Hash for Monotonic {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
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
