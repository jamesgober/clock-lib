//! Integration tests for `Monotonic` readings, arithmetic and conversions.
#![cfg(feature = "std")]

use std::collections::HashSet;
use std::time::{Duration, Instant};

use clock_lib::{ManualClock, Monotonic};

#[test]
fn test_now_called_twice_is_non_decreasing() {
    let a = Monotonic::now();
    let b = Monotonic::now();
    assert!(b >= a);
}

#[test]
fn test_elapsed_on_os_reading_is_bounded_by_bracketing_reads() {
    let start = Monotonic::now();
    let mid = Monotonic::now();
    let took = start.elapsed();
    let end = Monotonic::now();
    assert!(took >= mid - start);
    assert!(took <= end - start);
}

#[test]
fn test_duration_since_forward_returns_exact_delta() {
    let clock = ManualClock::new();
    let earlier = clock.now();
    clock.advance(Duration::from_micros(1_500));
    let later = clock.now();
    assert_eq!(later.duration_since(earlier), Duration::from_micros(1_500));
}

/// Audit repro r04 (CLK-005): reversed `duration_since` saturates.
#[test]
fn test_duration_since_reversed_returns_zero() {
    let clock = ManualClock::new();
    let first = clock.now();
    clock.advance(Duration::from_millis(1));
    let second = clock.now();
    assert_eq!(first.duration_since(second), Duration::ZERO);
}

#[test]
fn test_checked_duration_since_returns_exact_or_none() {
    let clock = ManualClock::new();
    let first = clock.now();
    clock.advance(Duration::from_millis(7));
    let second = clock.now();
    assert_eq!(
        second.checked_duration_since(first),
        Some(Duration::from_millis(7))
    );
    assert_eq!(first.checked_duration_since(second), None);
    assert_eq!(first.checked_duration_since(first), Some(Duration::ZERO));
}

/// CLK-015: the forward case must return the real delta, not zero.
#[test]
fn test_saturating_duration_since_forward_returns_exact_delta() {
    let clock = ManualClock::new();
    let first = clock.now();
    clock.advance(Duration::from_millis(3));
    let second = clock.now();
    assert_eq!(
        second.saturating_duration_since(first),
        Duration::from_millis(3)
    );
    assert_eq!(first.saturating_duration_since(second), Duration::ZERO);
}

#[test]
fn test_checked_add_and_sub_round_trip() {
    let base = Monotonic::now();
    let d = Duration::from_secs(90);
    let later = base.checked_add(d);
    assert!(later.is_some_and(|t| t - base == d));
    assert_eq!(later.and_then(|t| t.checked_sub(d)), Some(base));
}

#[test]
fn test_checked_add_overflow_returns_none() {
    assert_eq!(Monotonic::now().checked_add(Duration::MAX), None);
    assert_eq!(Monotonic::now().checked_sub(Duration::MAX), None);
}

#[test]
fn test_saturating_add_overflow_is_a_fixed_point() {
    let base = Monotonic::now();
    let never = base.saturating_add(Duration::MAX);
    assert!(never > base);
    assert_eq!(never.saturating_add(Duration::from_secs(1)), never);
    assert_eq!(never + Duration::MAX, never);
}

#[test]
fn test_saturating_sub_overflow_is_a_fixed_point() {
    let base = Monotonic::now();
    let floor = base.saturating_sub(Duration::MAX);
    assert!(floor < base);
    assert_eq!(floor.saturating_sub(Duration::from_secs(1)), floor);
    assert_eq!(floor - Duration::MAX, floor);
}

/// Audit compile probe `cf_add` (CLK-004): `Monotonic + Duration` compiles
/// and behaves like a deadline.
#[test]
fn test_add_assign_and_sub_assign_operators() {
    let base = Monotonic::now();
    let mut t = base;
    t += Duration::from_secs(10);
    assert_eq!(t - base, Duration::from_secs(10));
    t -= Duration::from_secs(4);
    assert_eq!(t - base, Duration::from_secs(6));
    assert_eq!(t - Duration::from_secs(6), base);
}

#[test]
fn test_sub_of_two_readings_saturates_when_reversed() {
    let base = Monotonic::now();
    let later = base + Duration::from_millis(2);
    assert_eq!(later - base, Duration::from_millis(2));
    assert_eq!(base - later, Duration::ZERO);
}

#[test]
fn test_deadline_comparison_with_manual_clock() {
    let clock = ManualClock::new();
    let deadline = clock.now() + Duration::from_secs(5);
    clock.advance(Duration::from_secs(4));
    assert!(clock.now() < deadline);
    clock.advance(Duration::from_secs(1));
    assert!(clock.now() >= deadline);
}

/// Audit compile probe `cf_into_instant` (CLK-003).
#[test]
fn test_conversions_to_and_from_instant_are_lossless() {
    let instant = Instant::now();
    let reading = Monotonic::from(instant);
    let back: Instant = reading.into();
    assert_eq!(back, instant);
    let reading2: Monotonic = instant.into();
    assert_eq!(reading2, reading);
}

#[test]
fn test_converted_instant_works_with_std_apis() {
    let deadline: Instant = (Monotonic::now() + Duration::from_secs(1)).into();
    assert!(deadline.checked_duration_since(Instant::now()).is_some());
}

#[test]
fn test_copy_hash_and_ordering_traits() {
    let a = Monotonic::now();
    let b = a;
    let c = a + Duration::from_nanos(1);
    assert_eq!(a, b);
    assert!(c > a);
    let set: HashSet<Monotonic> = [a, b, c].into_iter().collect();
    assert_eq!(set.len(), 2);
    assert!(format!("{a:?}").starts_with("Monotonic("));
}
