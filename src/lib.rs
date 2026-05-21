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
//! # Tier-1 API (the lazy path)
//!
//! ```
//! use clock_lib as clock;
//!
//! let start = clock::now();              // monotonic reading
//! // ... do work ...
//! let elapsed = clock::elapsed(start);   // Duration since `start`
//!
//! let secs = clock::unix();              // unix seconds (like PHP time())
//! ```
//!
//! # Tier-2 API (the mockable clock)
//!
//! The real value: deterministic time in tests. Inject a [`Clock`] and tests can
//! advance time instantly with no `sleep`.
//!
//! ```ignore
//! // Production
//! let limiter = RateLimiter::new(SystemClock);
//!
//! // Tests — instant, deterministic
//! let clock = ManualClock::new();
//! let limiter = RateLimiter::new(clock.clone());
//! clock.advance(Duration::from_secs(5));
//! ```
//!
//! # License
//!
//! Dual-licensed under Apache-2.0 OR MIT.

#![doc(html_root_url = "https://docs.rs/clock-lib")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
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
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::missing_safety_doc)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// Crate version string, populated by Cargo at build time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");