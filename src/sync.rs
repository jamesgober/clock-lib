//! Atomic types used by [`ManualClock`](crate::ManualClock).
//!
//! Under `--cfg loom` the crate's unit tests swap these for loom's model
//! checked atomics, so the lock-free paths in `ManualClock` are explored
//! under every legal interleaving. Normal builds use `core` atomics with no
//! indirection.

#[cfg(not(loom))]
pub(crate) use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
#[cfg(loom)]
pub(crate) use loom::sync::atomic::{AtomicI64, AtomicU64, Ordering};
