//! Mockable clock abstraction.
//!
//! The [`Clock`] trait lets time-driven code (rate limiters, TTL caches,
//! timeouts, retry backoff) depend on an injected source of time instead of
//! calling the OS directly. Production code uses [`SystemClock`], which
//! forwards to the OS. Tests use [`ManualClock`](crate::ManualClock), which
//! advances on demand, so timing-driven behaviour can be exercised
//! deterministically without ever calling [`std::thread::sleep`].

use core::time::Duration;
use std::sync::Arc;

use crate::{Monotonic, Wall};

/// A source of time.
///
/// `Clock` exposes both kinds of reading this crate keeps separate: a
/// monotonic reading via [`Clock::now`] and a wall-clock reading via
/// [`Clock::wall`]. Implementations must be safe to share across threads.
///
/// In production, take a `Clock` (generic `C: Clock`, `&dyn Clock`,
/// `Box<dyn Clock>` or `Arc<dyn Clock>`) as a dependency and pass
/// [`SystemClock`] at the top of your call graph. In tests, pass a
/// [`ManualClock`](crate::ManualClock) and drive time forward by calling
/// [`advance`](crate::ManualClock::advance).
///
/// `Clock` is implemented for `&C`, `Box<C>` and `Arc<C>` where
/// `C: Clock + ?Sized`, so a clock can be borrowed, boxed or shared and
/// still be passed to any `C: Clock` parameter.
///
/// # Measuring elapsed time through the clock
///
/// Use [`Clock::elapsed`] for readings that came from a clock. The free
/// function [`elapsed`](crate::elapsed) and [`Monotonic::elapsed`] always
/// read the operating system clock, so on a reading taken from a
/// `ManualClock` they would measure real time and ignore every `advance`.
///
/// # Implementing `Clock`
///
/// Implement [`now`](Clock::now) and [`wall`](Clock::wall). Build the
/// readings from your own source with `Monotonic::from(Instant)` and
/// `Wall::from(SystemTime)`. `now` must never return a reading earlier than
/// one it returned before.
///
/// ```
/// use clock_lib::{Clock, Monotonic, Wall};
/// use std::time::{Duration, Instant, SystemTime};
///
/// /// A clock that runs a fixed amount ahead of the system clock.
/// struct Ahead(Duration);
///
/// impl Clock for Ahead {
///     fn now(&self) -> Monotonic {
///         Monotonic::from(Instant::now()) + self.0
///     }
///     fn wall(&self) -> Wall {
///         Wall::from(SystemTime::now() + self.0)
///     }
/// }
///
/// let clock = Ahead(Duration::from_secs(10));
/// let start = clock.now();
/// assert!(clock.elapsed(start) < Duration::from_secs(10));
/// ```
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, ManualClock, Monotonic, SystemClock};
/// use std::time::Duration;
///
/// fn took_at_least<C: Clock>(clock: &C, start: Monotonic, target: Duration) -> bool {
///     clock.elapsed(start) >= target
/// }
///
/// // Production
/// let sys = SystemClock::new();
/// let start = sys.now();
/// assert!(!took_at_least(&sys, start, Duration::from_secs(60 * 60)));
///
/// // Test: no sleep, exact durations
/// let test = ManualClock::new();
/// let start = test.now();
/// test.advance(Duration::from_secs(60 * 60));
/// assert!(took_at_least(&test, start, Duration::from_secs(60 * 60)));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub trait Clock: Send + Sync {
    /// Returns the current monotonic reading.
    ///
    /// Successive calls on the same clock must never go backwards.
    fn now(&self) -> Monotonic;

    /// Returns the current wall-clock reading.
    ///
    /// Wall-clock readings may jump in either direction.
    fn wall(&self) -> Wall;

    /// Returns the time elapsed between `since` and this clock's current
    /// monotonic reading, saturating at zero if `since` is in the future.
    ///
    /// This is the injected-clock counterpart to the free function
    /// [`elapsed`](crate::elapsed): it reads *this* clock, so on a
    /// [`ManualClock`](crate::ManualClock) it reflects every `advance`.
    /// The provided implementation is
    /// `self.now().saturating_duration_since(since)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let start = clock.now();
    /// clock.advance(Duration::from_secs(3_600));
    /// assert_eq!(clock.elapsed(start), Duration::from_secs(3_600));
    /// ```
    #[inline]
    fn elapsed(&self, since: Monotonic) -> Duration {
        self.now().saturating_duration_since(since)
    }
}

/// A clock backed by the operating system.
///
/// `SystemClock` forwards [`Clock::now`] to [`Monotonic::now`] and
/// [`Clock::wall`] to [`Wall::now`]. It is zero-sized, `Copy`, and the
/// constructor is `const`, so it costs nothing over calling the free
/// functions directly.
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, SystemClock};
///
/// const CLOCK: SystemClock = SystemClock::new();
/// let start = CLOCK.now();
/// let _took = CLOCK.elapsed(start);
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Default, Copy, Clone)]
pub struct SystemClock;

impl SystemClock {
    /// Constructs a new system clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, SystemClock};
    ///
    /// let clock = SystemClock::new();
    /// assert!(clock.wall().unix_seconds() > 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

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

impl<C: Clock + ?Sized> Clock for &C {
    #[inline]
    fn now(&self) -> Monotonic {
        (**self).now()
    }

    #[inline]
    fn wall(&self) -> Wall {
        (**self).wall()
    }

    #[inline]
    fn elapsed(&self, since: Monotonic) -> Duration {
        (**self).elapsed(since)
    }
}

impl<C: Clock + ?Sized> Clock for Box<C> {
    #[inline]
    fn now(&self) -> Monotonic {
        (**self).now()
    }

    #[inline]
    fn wall(&self) -> Wall {
        (**self).wall()
    }

    #[inline]
    fn elapsed(&self, since: Monotonic) -> Duration {
        (**self).elapsed(since)
    }
}

impl<C: Clock + ?Sized> Clock for Arc<C> {
    #[inline]
    fn now(&self) -> Monotonic {
        (**self).now()
    }

    #[inline]
    fn wall(&self) -> Wall {
        (**self).wall()
    }

    #[inline]
    fn elapsed(&self, since: Monotonic) -> Duration {
        (**self).elapsed(since)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clock that overrides `elapsed`, to prove the forwarding impls
    /// dispatch to the inner implementation rather than the default.
    struct FixedElapsed;

    impl Clock for FixedElapsed {
        fn now(&self) -> Monotonic {
            Monotonic::now()
        }
        fn wall(&self) -> Wall {
            Wall::UNIX_EPOCH
        }
        fn elapsed(&self, _since: Monotonic) -> Duration {
            Duration::from_secs(42)
        }
    }

    fn elapsed_via<C: Clock>(clock: &C) -> Duration {
        clock.elapsed(Monotonic::now())
    }

    #[test]
    fn test_forwarding_impls_forward_overridden_elapsed() {
        let expected = Duration::from_secs(42);
        assert_eq!(elapsed_via(&FixedElapsed), expected);
        assert_eq!(elapsed_via(&&FixedElapsed), expected);
        assert_eq!(elapsed_via(&Box::new(FixedElapsed)), expected);
        assert_eq!(elapsed_via(&Arc::new(FixedElapsed)), expected);
        let boxed: Box<dyn Clock> = Box::new(FixedElapsed);
        assert_eq!(elapsed_via(&boxed), expected);
    }

    #[test]
    fn test_forwarding_impls_forward_wall() {
        let boxed = Box::new(FixedElapsed);
        let shared = Arc::new(FixedElapsed);
        assert_eq!(Clock::wall(&&FixedElapsed), Wall::UNIX_EPOCH);
        assert_eq!(Clock::wall(&boxed), Wall::UNIX_EPOCH);
        assert_eq!(Clock::wall(&shared), Wall::UNIX_EPOCH);
    }

    #[test]
    fn test_default_elapsed_saturates_for_future_reading() {
        let future = Monotonic::now() + Duration::from_secs(60);
        assert_eq!(SystemClock.elapsed(future), Duration::ZERO);
    }
}
