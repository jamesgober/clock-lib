//! # clock-lib
//!
//! TIME READINGS FOR RUST
//!
//! Monotonic and wall-clock time readings with a mockable clock for
//! deterministic testing. Simple Tier-1 API, zero default dependencies, no
//! `unsafe`.
//!
//! # Two kinds of time
//!
//! There are two fundamentally different kinds of time, and conflating them
//! is a common source of bugs:
//!
//! - **Monotonic time** ([`now`]) never goes backwards. Use it to measure
//!   elapsed time: rate limiting, timeouts, benchmarks. Only meaningful as a
//!   delta.
//! - **Wall-clock time** ([`wall`], [`unix`]) is calendar time (UTC). It can
//!   jump in either direction: NTP steps, manual changes, VM suspend/resume
//!   or migration, leap-second handling. Use it for timestamps and logging,
//!   never for measuring elapsed time.
//!
//! The two are returned as distinct types, [`Monotonic`] and [`Wall`], so
//! the compiler rejects any attempt to subtract one from the other. That
//! separation is the central design choice of the crate.
//!
//! # Tier-1 API (the lazy path)
//!
//! ```
//! # #[cfg(feature = "std")] {
//! use clock_lib as clock;
//! use std::time::Duration;
//!
//! let start = clock::now();              // monotonic reading
//! // ... do work ...
//! let took = clock::elapsed(start);      // Duration since `start`
//! assert!(took < Duration::from_secs(60));
//!
//! let deadline = start + Duration::from_secs(5);
//! assert!(clock::now() < deadline);
//!
//! let secs = clock::unix();              // Unix seconds, like PHP's time()
//! let stamp = clock::wall().to_string(); // "2026-10-13T08:30:00.123456789Z"
//! assert!(secs > 0 && stamp.ends_with('Z'));
//! # }
//! ```
//!
//! # Tier-2 API (the mockable clock)
//!
//! The real value is deterministic time in tests. The [`Clock`] trait has
//! two implementations, [`SystemClock`] for production and [`ManualClock`]
//! for tests, so timing-driven code can be exercised without ever calling
//! `sleep`.
//!
//! ```
//! # #[cfg(feature = "std")] {
//! use clock_lib::{Clock, ManualClock, Monotonic};
//! use std::time::Duration;
//!
//! fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
//!     clock.elapsed(stamp) >= ttl
//! }
//!
//! let clock = ManualClock::new();
//! let stamp = clock.now();
//! assert!(!expired(&clock, stamp, Duration::from_secs(60)));
//!
//! clock.advance(Duration::from_secs(60));
//! assert!(expired(&clock, stamp, Duration::from_secs(60)));
//! # }
//! ```
//!
//! [`ManualClock`] can also start at a fixed wall time
//! ([`ManualClock::with_start`]), step wall time independently of monotonic
//! time to simulate NTP corrections ([`ManualClock::step_wall_back`]), and
//! advance itself on every reading ([`ManualClock::set_auto_advance`]).
//! [`Deadline`] and [`Stopwatch`] package the two most common patterns on
//! top of any clock.
//!
//! # Which calls read the operating system clock
//!
//! These always read the OS clock, whatever clock the surrounding code was
//! given: [`now`], [`elapsed`], [`wall`], [`unix`], [`unix_ms`],
//! [`unix_ns`], [`Monotonic::now`], [`Monotonic::elapsed`], [`Wall::now`],
//! and [`Deadline::after`] / [`Stopwatch::start`] (which use
//! [`SystemClock`]).
//!
//! Code that takes an injected [`Clock`] should only use the clock's
//! methods: [`Clock::now`], [`Clock::wall`] and [`Clock::elapsed`]. In
//! particular, calling `clock_lib::elapsed(stamp)` or `stamp.elapsed()` on a
//! reading taken from a [`ManualClock`] measures real time and ignores
//! every `advance`.
//!
//! # Resolution and platforms
//!
//! Readings have the resolution of the platform clock: 100 ns on Windows
//! (`QueryPerformanceCounter` and `FILETIME`), typically 1 ns on Linux and
//! macOS. Back-to-back monotonic readings are often equal. That is normal
//! for a non-decreasing clock.
//!
//! On `wasm32-unknown-unknown`, the standard library has no clock and
//! `std::time::Instant::now()` panics. Enable the `web-time` feature there
//! to read the browser's `performance.now()` and `Date.now()` instead.
//!
//! # Feature flags
//!
//! | Feature | Default | Enables |
//! | --- | --- | --- |
//! | `std` | yes | Every reading API. Without it, only [`VERSION`] is exposed. |
//! | `serde` | no | `Serialize`/`Deserialize` for [`Wall`], in the same format as `std::time::SystemTime`. Adds the `serde` dependency. |
//! | `web-time` | no | On `wasm32-unknown-unknown` only, reads time through the `web-time` crate. A no-op with no dependency on every other target. |
//!
//! With default features disabled the crate compiles for `no_std` targets,
//! but it exposes nothing except [`VERSION`]: every reading needs an
//! operating system clock. This lets a crate that uses `clock-lib` behind
//! its own `std` feature keep building for embedded targets. It does not
//! provide a clock on bare metal.
//!
//! ```toml
//! [dependencies]
//! clock-lib = "1.1"
//! # clock-lib = { version = "1.1", features = ["serde"] }
//! ```
//!
//! # License
//!
//! Dual-licensed under Apache-2.0 OR MIT.

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

#[cfg(feature = "std")]
mod clock;
#[cfg(feature = "std")]
mod deadline;
#[cfg(feature = "std")]
mod manual;
#[cfg(feature = "std")]
mod monotonic;
#[cfg(feature = "std")]
mod platform;
#[cfg(feature = "std")]
mod rfc3339;
#[cfg(feature = "serde")]
mod serde_impl;
#[cfg(feature = "std")]
mod stopwatch;
#[cfg(feature = "std")]
mod sync;
#[cfg(feature = "std")]
mod wall;

#[cfg(feature = "std")]
pub use clock::{Clock, SystemClock};
#[cfg(feature = "std")]
pub use deadline::Deadline;
#[cfg(feature = "std")]
pub use manual::ManualClock;
#[cfg(feature = "std")]
pub use monotonic::Monotonic;
#[cfg(feature = "std")]
pub use stopwatch::Stopwatch;
#[cfg(feature = "std")]
pub use wall::Wall;

/// Crate version string, populated by Cargo at build time.
///
/// Available with and without the `std` feature.
///
/// # Examples
///
/// ```
/// assert!(clock_lib::VERSION.starts_with("1."));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Captures the current monotonic time from the operating system.
///
/// Shortcut for [`Monotonic::now`]. Pair it with [`elapsed`] to measure how
/// long an operation took, or add a `Duration` to build a deadline. Code
/// that takes an injected [`Clock`] should call [`Clock::now`] instead.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
/// use std::time::Duration;
///
/// let start = clock::now();
/// let deadline = start + Duration::from_millis(100);
/// assert!(deadline > start);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn now() -> Monotonic {
    Monotonic::now()
}

/// Returns the time elapsed between `earlier` and the operating system's
/// current monotonic time.
///
/// Shortcut for [`Monotonic::elapsed`]. This always reads the **OS clock**.
/// Only pass it readings that came from the OS clock ([`now`],
/// [`Monotonic::now`], [`SystemClock`]). For a reading taken from an
/// injected clock such as [`ManualClock`], call
/// [`Clock::elapsed`] on that clock: this function would
/// measure real time and ignore every `advance`.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
/// use std::time::Duration;
///
/// let start = clock::now();
/// // ... work ...
/// let took = clock::elapsed(start);
/// assert!(took < Duration::from_secs(60));
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn elapsed(earlier: Monotonic) -> core::time::Duration {
    earlier.elapsed()
}

/// Captures the current wall-clock time from the operating system.
///
/// Shortcut for [`Wall::now`]. Use it for timestamps. For elapsed-time
/// measurement, use [`now`] instead, because wall-clock readings can jump.
/// Code that takes an injected [`Clock`] should call [`Clock::wall`].
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let stamp = clock::wall();
/// assert!(stamp.unix_seconds() > 1_704_067_200);
/// assert!(stamp.to_string().ends_with('Z'));
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn wall() -> Wall {
    Wall::now()
}

/// Returns the current Unix time in whole seconds, from the operating
/// system clock.
///
/// Shortcut for `wall().unix_seconds()`. Equivalent in spirit to C's
/// `time(NULL)` or PHP's `time()`. Returns zero if the system clock is set
/// to a moment before the Unix epoch.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let now = clock::unix();
/// assert!(now > 1_704_067_200); // after 2024-01-01
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn unix() -> u64 {
    Wall::now().unix_seconds()
}

/// Returns the current Unix time in whole milliseconds, from the operating
/// system clock.
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
/// assert!(now > 1_704_067_200_000);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn unix_ms() -> u128 {
    Wall::now().unix_millis()
}

/// Returns the current Unix time in whole nanoseconds, from the operating
/// system clock.
///
/// Shortcut for `wall().unix_nanos()`. Returns zero if the system clock is
/// set to a moment before the Unix epoch. On Windows the value is always a
/// multiple of 100.
///
/// # Examples
///
/// ```
/// use clock_lib as clock;
///
/// let now = clock::unix_ns();
/// assert!(now > 1_704_067_200_000_000_000);
/// ```
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[inline]
#[must_use]
pub fn unix_ns() -> u128 {
    Wall::now().unix_nanos()
}
