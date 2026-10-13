//! A point in monotonic time after which something counts as expired.

use core::fmt;
use core::time::Duration;

use crate::{Clock, Monotonic, SystemClock};

/// A monotonic deadline bound to the clock that measures it.
///
/// `Deadline` answers "has the time run out?" and "how long is left?"
/// against a [`Clock`]. It is a reading plus a clock, not a timer: nothing
/// happens when it expires, you ask it.
///
/// The clock type defaults to [`SystemClock`], so the common case needs no
/// type annotations. To make the code testable, take the clock as a
/// parameter and build the deadline with
/// [`after_with`](Deadline::after_with); a test can then pass
/// `&ManualClock`. Any `C: Clock` works, including `&C`, `Box<dyn Clock>`
/// and `Arc<dyn Clock>`.
///
/// Deadline arithmetic saturates: `Deadline::after(Duration::MAX)` never
/// expires instead of overflowing.
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, Deadline, ManualClock};
/// use std::time::Duration;
///
/// fn poll_until_done<C: Clock>(clock: C, mut step: impl FnMut() -> bool) -> bool {
///     let deadline = Deadline::after_with(clock, Duration::from_secs(5));
///     while !deadline.is_expired() {
///         if step() {
///             return true;
///         }
///     }
///     false
/// }
///
/// // In a test, every clock reading moves time forward one second. The
/// // deadline takes the first reading, then four checks see 1..=4 seconds
/// // elapsed and the fifth sees 5 seconds: expired.
/// let clock = ManualClock::new();
/// clock.set_auto_advance(Duration::from_secs(1));
/// let mut polls = 0;
/// assert!(!poll_until_done(&clock, || { polls += 1; false }));
/// assert_eq!(polls, 4);
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Clone, Copy)]
pub struct Deadline<C = SystemClock> {
    clock: C,
    expires_at: Monotonic,
}

/// Prints the expiry reading and the clock's type name. The clock itself is
/// not printed, so `Deadline<Box<dyn Clock>>` is `Debug` too.
impl<C> fmt::Debug for Deadline<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deadline")
            .field("clock", &core::any::type_name::<C>())
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl Deadline<SystemClock> {
    /// Creates a deadline `ttl` from now on the operating system clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Deadline;
    /// use std::time::Duration;
    ///
    /// let deadline = Deadline::after(Duration::from_secs(30));
    /// assert!(!deadline.is_expired());
    /// assert!(deadline.remaining() <= Duration::from_secs(30));
    /// ```
    #[inline]
    #[must_use]
    pub fn after(ttl: Duration) -> Self {
        Self::after_with(SystemClock, ttl)
    }
}

impl<C: Clock> Deadline<C> {
    /// Creates a deadline `ttl` after `clock`'s current reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Deadline, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let deadline = Deadline::after_with(&clock, Duration::from_secs(10));
    /// clock.advance(Duration::from_secs(9));
    /// assert_eq!(deadline.remaining(), Duration::from_secs(1));
    /// clock.advance(Duration::from_secs(1));
    /// assert!(deadline.is_expired());
    /// ```
    #[inline]
    #[must_use]
    pub fn after_with(clock: C, ttl: Duration) -> Self {
        let expires_at = clock.now().saturating_add(ttl);
        Self { clock, expires_at }
    }

    /// Creates a deadline at an explicit reading, for example one computed
    /// earlier or restored from a cache entry.
    ///
    /// `expires_at` must come from `clock` (or from the OS clock when
    /// `clock` is a [`SystemClock`]); readings from different clocks are
    /// not comparable.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, Deadline, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let at = clock.now() + Duration::from_millis(250);
    /// let deadline = Deadline::at(&clock, at);
    /// assert_eq!(deadline.expires_at(), at);
    /// ```
    #[inline]
    #[must_use]
    pub const fn at(clock: C, expires_at: Monotonic) -> Self {
        Self { clock, expires_at }
    }

    /// Returns the reading at which the deadline expires.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, Deadline, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let deadline = Deadline::after_with(&clock, Duration::from_secs(2));
    /// assert_eq!(deadline.expires_at() - clock.now(), Duration::from_secs(2));
    /// ```
    #[inline]
    #[must_use]
    pub const fn expires_at(&self) -> Monotonic {
        self.expires_at
    }

    /// Returns `true` once the clock has reached the deadline.
    ///
    /// A deadline with a zero TTL is expired immediately.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Deadline, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// assert!(Deadline::after_with(&clock, Duration::ZERO).is_expired());
    /// assert!(!Deadline::after_with(&clock, Duration::from_nanos(1)).is_expired());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.clock.now() >= self.expires_at
    }

    /// Returns the time left before the deadline, or zero once it has
    /// expired.
    ///
    /// Pass it to anything that takes a timeout, such as
    /// `Receiver::recv_timeout` or a socket read timeout.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Deadline, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let deadline = Deadline::after_with(&clock, Duration::from_secs(3));
    /// clock.advance(Duration::from_secs(5));
    /// assert_eq!(deadline.remaining(), Duration::ZERO);
    /// ```
    #[inline]
    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.expires_at.saturating_duration_since(self.clock.now())
    }

    /// Returns the clock this deadline is measured against.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Deadline, ManualClock};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let deadline = Deadline::after_with(&clock, Duration::from_secs(1));
    /// deadline.clock().advance(Duration::from_secs(1));
    /// assert!(deadline.is_expired());
    /// ```
    #[inline]
    #[must_use]
    pub const fn clock(&self) -> &C {
        &self.clock
    }
}
