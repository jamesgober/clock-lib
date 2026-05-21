//! Mockable clock abstraction.
//!
//! The [`Clock`] trait lets time-driven code &mdash; rate limiters, TTL caches,
//! timeouts, retry backoff &mdash; depend on an injected source of time instead
//! of calling the OS directly. Production code uses [`SystemClock`], which
//! simply forwards to the OS. Tests use [`ManualClock`], which advances on
//! demand so timing-driven behavior can be exercised deterministically
//! without ever calling [`std::thread::sleep`].

#[cfg(feature = "std")]
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "std")]
use core::time::Duration;
#[cfg(feature = "std")]
use std::sync::Arc;
#[cfg(feature = "std")]
use std::time::{Instant, SystemTime};

#[cfg(feature = "std")]
use crate::{Monotonic, Wall};

/// A source of time.
///
/// `Clock` exposes both kinds of reading this crate keeps separate:
/// a monotonic reading via [`Clock::now`] and a wall-clock reading via
/// [`Clock::wall`]. Implementations must be safe to share across threads.
///
/// In production, take a `Clock` (or `&dyn Clock`, or `Arc<dyn Clock>`) as
/// a dependency and use [`SystemClock`] at the top of your call graph. In
/// tests, substitute [`ManualClock`] and drive time forward by calling
/// [`ManualClock::advance`].
///
/// `Clock` is also implemented for `Arc<C>` and `&C` where `C: Clock`,
/// so the same value can be shared and reused freely.
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, ManualClock, SystemClock};
/// use std::time::Duration;
///
/// fn took_at_least<C: Clock>(clock: &C, start: clock_lib::Monotonic, target: Duration) -> bool {
///     clock.now().duration_since(start) >= target
/// }
///
/// // Production
/// let sys = SystemClock::new();
/// let start = sys.now();
/// assert!(!took_at_least(&sys, start, Duration::from_secs(60 * 60)));
///
/// // Test — no sleep, fully deterministic
/// let test = ManualClock::new();
/// let start = test.now();
/// test.advance(Duration::from_secs(60 * 60));
/// assert!(took_at_least(&test, start, Duration::from_secs(60 * 60)));
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub trait Clock: Send + Sync {
    /// Returns the current monotonic reading.
    fn now(&self) -> Monotonic;

    /// Returns the current wall-clock reading.
    fn wall(&self) -> Wall;
}

/// A clock backed by the operating system.
///
/// `SystemClock` forwards [`Clock::now`] to [`Monotonic::now`] and
/// [`Clock::wall`] to [`Wall::now`]. It is zero-sized, `Copy`, and the
/// constructor is `const`, so it imposes no runtime cost over calling the
/// free functions directly.
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, SystemClock};
///
/// const CLOCK: SystemClock = SystemClock::new();
/// let t = CLOCK.now();
/// # let _ = t;
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Default, Copy, Clone)]
pub struct SystemClock;

#[cfg(feature = "std")]
impl SystemClock {
    /// Constructs a new system clock.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "std")]
impl Clock for SystemClock {
    #[inline]
    fn now(&self) -> Monotonic {
        Monotonic::now()
    }

    #[inline]
    fn wall(&self) -> Wall {
        Wall::now()
    }
}

/// A clock under your control, for deterministic testing.
///
/// `ManualClock` captures the OS monotonic and wall-clock anchors at
/// construction, then advances **only** when you call
/// [`advance`](Self::advance). The clock never moves on its own, so tests
/// against time-driven code become point-in-time deterministic.
///
/// `ManualClock` is `Send + Sync`. Wrap it in an [`Arc`] to share with the
/// code under test; both the test driver and the production code can hold
/// references to the same clock and observe consistent readings.
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, ManualClock, Monotonic};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// // A "ttl expired?" check that can be driven without sleeping.
/// fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
///     clock.now().duration_since(stamp) >= ttl
/// }
///
/// let clock = Arc::new(ManualClock::new());
/// let stamp = clock.now();
///
/// assert!(!expired(&*clock, stamp, Duration::from_secs(60)));
///
/// clock.advance(Duration::from_secs(60));
/// assert!(expired(&*clock, stamp, Duration::from_secs(60)));
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug)]
pub struct ManualClock {
    monotonic_anchor: Instant,
    wall_anchor: SystemTime,
    offset_nanos: AtomicU64,
}

#[cfg(feature = "std")]
impl ManualClock {
    /// Constructs a new manual clock anchored at the current OS time.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    ///
    /// let clock = ManualClock::new();
    /// # let _ = clock;
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            monotonic_anchor: Instant::now(),
            wall_anchor: SystemTime::now(),
            offset_nanos: AtomicU64::new(0),
        }
    }

    /// Advances the clock forward by `by`.
    ///
    /// Successive calls accumulate. If the cumulative offset would exceed
    /// [`u64::MAX`] nanoseconds (≈584 years), the offset saturates &mdash;
    /// well outside any plausible test scenario.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let a = clock.now();
    /// clock.advance(Duration::from_secs(5));
    /// let b = clock.now();
    /// assert_eq!(b.duration_since(a), Duration::from_secs(5));
    /// ```
    #[inline]
    pub fn advance(&self, by: Duration) {
        let nanos = u64::try_from(by.as_nanos()).unwrap_or(u64::MAX);
        let _ = self.offset_nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    /// Returns the cumulative offset that has been added to this clock
    /// since it was constructed.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// clock.advance(Duration::from_secs(1));
    /// clock.advance(Duration::from_secs(2));
    /// assert_eq!(clock.offset(), Duration::from_secs(3));
    /// ```
    #[inline]
    #[must_use]
    pub fn offset(&self) -> Duration {
        Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed))
    }
}

#[cfg(feature = "std")]
impl Default for ManualClock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl Clock for ManualClock {
    /// Returns the monotonic anchor plus the accumulated offset.
    ///
    /// # Panics
    ///
    /// Panics if the anchor plus offset is not representable as an
    /// [`Instant`] on the current platform. This requires a cumulative
    /// offset measured in centuries and never occurs in realistic tests.
    fn now(&self) -> Monotonic {
        let offset = Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed));
        Monotonic(self.monotonic_anchor + offset)
    }

    /// Returns the wall-clock anchor plus the accumulated offset.
    ///
    /// # Panics
    ///
    /// Panics if the anchor plus offset is not representable as a
    /// [`SystemTime`] on the current platform. This requires a cumulative
    /// offset measured in centuries and never occurs in realistic tests.
    fn wall(&self) -> Wall {
        let offset = Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed));
        Wall(self.wall_anchor + offset)
    }
}

#[cfg(feature = "std")]
impl<C: Clock + ?Sized> Clock for Arc<C> {
    #[inline]
    fn now(&self) -> Monotonic {
        (**self).now()
    }

    #[inline]
    fn wall(&self) -> Wall {
        (**self).wall()
    }
}

#[cfg(feature = "std")]
impl<C: Clock + ?Sized> Clock for &C {
    #[inline]
    fn now(&self) -> Monotonic {
        (**self).now()
    }

    #[inline]
    fn wall(&self) -> Wall {
        (**self).wall()
    }
}
