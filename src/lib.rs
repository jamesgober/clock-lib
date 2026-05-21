//! # clock-lib
//!
//! TIME READINGS FOR RUST
//!
//! Monotonic and wall-clock time readings with a mockable clock for deterministic
//! testing. Simple Tier-1 API, zero dependencies, no `unsafe`.
//!
//! # Two kinds of time
//!
//! There are two fundamentally different kinds of time, and conflating them is a
//! common source of bugs:
//!
//! - **Monotonic time** ([`now`]) never goes backwards. Use it to measure elapsed
//!   time: rate limiting, timeouts, benchmarks. Only meaningful as a delta.
//! - **Wall-clock time** ([`wall`], [`unix`]) is calendar time. It can jump (NTP,
//!   DST, manual changes). Use it for timestamps and logging, never for measuring
//!   elapsed time.
//!
//! The two are returned as distinct types — [`Monotonic`] and [`Wall`] — so the
//! compiler rejects any attempt to subtract one from the other. That separation
//! is the central design choice of the crate.
//!
//! # Tier-1 API (the lazy path)
//!
//! ```
//! use clock_lib as clock;
//!
//! let start = clock::now();              // monotonic reading
//! // ... do work ...
//! let took = clock::elapsed(start);      // Duration since `start`
//!
//! let secs = clock::unix();              // unix seconds (like PHP time())
//! # let _ = (took, secs);
//! ```
//!
//! # Tier-2 API (the mockable clock)
//!
//! The real value is deterministic time in tests. The [`Clock`] trait has two
//! implementations &mdash; [`SystemClock`] for production and [`ManualClock`]
//! for tests &mdash; so timing-driven code can be exercised without ever
//! calling `sleep`.
//!
//! ```
//! use clock_lib::{Clock, ManualClock, Monotonic};
//! use std::time::Duration;
//!
//! fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
//!     clock.now().duration_since(stamp) >= ttl
//! }
//!
//! let clock = ManualClock::new();
//! let stamp = clock.now();
//! assert!(!expired(&clock, stamp, Duration::from_secs(60)));
//!
//! clock.advance(Duration::from_secs(60));
//! assert!(expired(&clock, stamp, Duration::from_secs(60)));
//! ```
//!
//! # License
//!
//! Dual-licensed under Apache-2.0 OR MIT.

#![doc(html_root_url = "https://docs.rs/clock-lib")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_must_use)]
#![deny(unused_results)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unreachable)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::missing_safety_doc)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod clock;
mod monotonic;
mod wall;

#[cfg(feature = "std")]
pub use clock::{Clock, ManualClock, SystemClock};
#[cfg(feature = "std")]
pub use monotonic::Monotonic;
#[cfg(feature = "std")]
pub use wall::Wall;

/// Crate version string, populated by Cargo at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Captures the current monotonic time.
///
/// Shortcut for [`Monotonic::now`]. Pair it with [`elapsed`] to measure how
/// long an operation took.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let start = clock::now();
/// // ... work ...
/// let took = clock::elapsed(start);
/// # let _ = took;
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn now() -> Monotonic {
    Monotonic::now()
}

/// Returns the [`Duration`](core::time::Duration) elapsed since `earlier`.
///
/// Shortcut for [`Monotonic::elapsed`]. The argument is the
/// [`Monotonic`] captured at the start of the interval; the return value
/// is the time from then until now.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let start = clock::now();
/// // ... work ...
/// let took = clock::elapsed(start);
/// # let _ = took;
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn elapsed(earlier: Monotonic) -> core::time::Duration {
    earlier.elapsed()
}

/// Captures the current wall-clock time.
///
/// Shortcut for [`Wall::now`]. Use it for timestamps. For elapsed-time
/// measurement, use [`now`] instead — wall-clock readings can jump.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let stamp = clock::wall();
/// let secs = stamp.unix_seconds();
/// assert!(secs > 0);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn wall() -> Wall {
    Wall::now()
}

/// Returns the current Unix time in whole seconds.
///
/// Shortcut for `wall().unix_seconds()`. Equivalent in spirit to C's
/// `time(NULL)` or PHP's `time()`.
///
/// Returns zero if the system clock is set to a moment before the Unix
/// epoch.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let now = clock::unix();
/// assert!(now > 0);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn unix() -> u64 {
    Wall::now().unix_seconds()
}

/// Returns the current Unix time in whole milliseconds.
///
/// Shortcut for `wall().unix_millis()`. Returns zero if the system clock is
/// set to a moment before the Unix epoch.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let now = clock::unix_ms();
/// assert!(now > 0);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn unix_ms() -> u128 {
    Wall::now().unix_millis()
}

/// Returns the current Unix time in whole nanoseconds.
///
/// Shortcut for `wall().unix_nanos()`. Returns zero if the system clock is
/// set to a moment before the Unix epoch.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let now = clock::unix_ns();
/// assert!(now > 0);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn unix_ns() -> u128 {
    Wall::now().unix_nanos()
}
