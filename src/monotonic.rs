//! Monotonic clock readings.
//!
//! Monotonic time never goes backwards. It is the right tool for measuring
//! elapsed time: rate limiting, timeouts, benchmarks, retry backoff. The
//! absolute value of a [`Monotonic`] reading carries no calendar meaning —
//! it is only useful as a delta against another [`Monotonic`] from the same
//! process.
//!
//! For calendar timestamps, use [`Wall`](crate::Wall) instead.

#[cfg(feature = "std")]
use std::time::Instant;

/// A captured monotonic instant.
///
/// `Monotonic` wraps a single sample of the operating system's monotonic
/// clock. Two readings can be compared to recover the [`Duration`] between
/// them with [`duration_since`](Monotonic::duration_since) or the safer
/// [`checked_duration_since`](Monotonic::checked_duration_since).
///
/// `Monotonic` and [`Wall`](crate::Wall) are deliberately distinct types
/// with no cross-type arithmetic. The compiler will reject any attempt to
/// mix them — that separation is the central design choice of this crate.
///
/// Construct one with [`Monotonic::now`] or the crate-level
/// [`now`](crate::now) shortcut.
///
/// [`Duration`]: core::time::Duration
///
/// # Examples
///
/// ```
/// use clock_lib::Monotonic;
///
/// let start = Monotonic::now();
/// // ... do some work ...
/// let took = start.elapsed();
/// assert!(took.as_nanos() < u128::MAX);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Monotonic(pub(crate) Instant);

#[cfg(feature = "std")]
impl Monotonic {
    /// Captures the current monotonic time from the operating system.
    ///
    /// This is the constructor for [`Monotonic`]. The crate-level
    /// [`now`](crate::now) function is a one-line shortcut for the same
    /// thing.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    ///
    /// let t = Monotonic::now();
    /// # let _ = t;
    /// ```
    #[inline]
    #[must_use]
    pub fn now() -> Self {
        Self(Instant::now())
    }

    /// Returns the [`Duration`](core::time::Duration) elapsed since this
    /// reading was captured.
    ///
    /// Equivalent to `Monotonic::now().duration_since(self)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    ///
    /// let t = Monotonic::now();
    /// let _ = t.elapsed();
    /// ```
    #[inline]
    #[must_use]
    pub fn elapsed(self) -> core::time::Duration {
        self.0.elapsed()
    }

    /// Returns the [`Duration`](core::time::Duration) between two readings.
    ///
    /// # Panics
    ///
    /// Panics if `earlier` is later than `self`. Prefer
    /// [`checked_duration_since`](Monotonic::checked_duration_since) or
    /// [`saturating_duration_since`](Monotonic::saturating_duration_since)
    /// when the ordering is not guaranteed by construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    ///
    /// let earlier = Monotonic::now();
    /// let later = Monotonic::now();
    /// let _delta = later.duration_since(earlier);
    /// ```
    #[inline]
    #[must_use]
    pub fn duration_since(self, earlier: Self) -> core::time::Duration {
        self.0.duration_since(earlier.0)
    }

    /// Returns `Some(duration)` if `self >= earlier`, otherwise `None`.
    ///
    /// The non-panicking counterpart to
    /// [`duration_since`](Monotonic::duration_since).
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    ///
    /// let earlier = Monotonic::now();
    /// let later = Monotonic::now();
    /// assert!(later.checked_duration_since(earlier).is_some());
    /// ```
    #[inline]
    #[must_use]
    pub fn checked_duration_since(self, earlier: Self) -> Option<core::time::Duration> {
        self.0.checked_duration_since(earlier.0)
    }

    /// Returns the duration since `earlier`, saturating at zero when
    /// `earlier` is later than `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::Monotonic;
    ///
    /// let a = Monotonic::now();
    /// let b = Monotonic::now();
    /// // Regardless of which is later, this never panics.
    /// let _ = a.saturating_duration_since(b);
    /// ```
    #[inline]
    #[must_use]
    pub fn saturating_duration_since(self, earlier: Self) -> core::time::Duration {
        self.0.saturating_duration_since(earlier.0)
    }
}
