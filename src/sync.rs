//! Atomic types used by [`ManualClock`](crate::ManualClock).
//!
//! Under `--cfg clock_lib_loom` the crate's unit tests swap these for loom's
//! model checked atomics, so the lock-free paths in `ManualClock` are
//! explored under every legal interleaving. Normal builds use `core` atomics
//! with no indirection.
//!
//! The cfg is not the conventional `loom`. A crate that model-checks its own
//! code sets `--cfg loom` through `RUSTFLAGS`, which applies to every crate in
//! its build. Keying off `loom` here would swap in loom atomics for all of its
//! dependents' `ManualClock`s, which then panic when used outside a
//! `loom::model`, and it broke the build outright in 1.1.0, where loom was only
//! a dev-dependency.

#[cfg(not(clock_lib_loom))]
pub(crate) use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
#[cfg(clock_lib_loom)]
pub(crate) use loom::sync::atomic::{AtomicI64, AtomicU64, Ordering};
