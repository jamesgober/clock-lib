//! Integration tests for the `Clock` trait and its `SystemClock` /
//! `ManualClock` implementations.

use std::sync::Arc;
use std::time::Duration;

use clock_lib::{Clock, ManualClock, Monotonic, SystemClock};

#[test]
fn test_system_clock_now_is_bracketed_by_monotonic_now() {
    let before = Monotonic::now();
    let middle = SystemClock.now();
    let after = Monotonic::now();
    assert!(before <= middle);
    assert!(middle <= after);
}

#[test]
fn test_system_clock_new_is_const_constructible() {
    const CLOCK: SystemClock = SystemClock::new();
    let _ = CLOCK.now();
}

#[test]
fn test_manual_clock_without_advance_returns_same_reading() {
    let clock = ManualClock::new();
    let a = clock.now();
    let b = clock.now();
    assert_eq!(a, b);
}

#[test]
fn test_manual_clock_advance_moves_monotonic_by_exact_amount() {
    let clock = ManualClock::new();
    let before = clock.now();
    clock.advance(Duration::from_secs(10));
    let after = clock.now();
    assert_eq!(after.duration_since(before), Duration::from_secs(10));
}

#[test]
fn test_manual_clock_advance_moves_wall_by_exact_amount() {
    let clock = ManualClock::new();
    let before = clock.wall();
    clock.advance(Duration::from_secs(60));
    let after = clock.wall();
    assert_eq!(after.unix_seconds() - before.unix_seconds(), 60);
}

#[test]
fn test_manual_clock_offset_accumulates_across_advances() {
    let clock = ManualClock::new();
    clock.advance(Duration::from_secs(1));
    clock.advance(Duration::from_secs(2));
    clock.advance(Duration::from_millis(500));
    assert_eq!(clock.offset(), Duration::from_millis(3_500));
}

#[test]
fn test_manual_clock_drives_ttl_check_without_sleeping() {
    fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
        clock.now().duration_since(stamp) >= ttl
    }

    let clock = ManualClock::new();
    let stamp = clock.now();
    let ttl = Duration::from_secs(60);

    assert!(!expired(&clock, stamp, ttl));

    clock.advance(Duration::from_secs(30));
    assert!(!expired(&clock, stamp, ttl));

    clock.advance(Duration::from_secs(30));
    assert!(expired(&clock, stamp, ttl));
}

#[test]
fn test_arc_clock_dispatches_through_inner_clock() {
    let clock: Arc<ManualClock> = Arc::new(ManualClock::new());
    let before = clock.now();
    clock.advance(Duration::from_secs(1));
    let after = clock.now();
    assert_eq!(after.duration_since(before), Duration::from_secs(1));
}

#[test]
fn test_clock_is_usable_as_arc_dyn_trait_object() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let _ = clock.now();
    let _ = clock.wall();
}
