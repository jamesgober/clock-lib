//! Integration tests for `Monotonic` readings.

use std::thread::sleep;
use std::time::Duration;

use clock_lib::Monotonic;

#[test]
fn now_is_monotonic_non_decreasing() {
    let a = Monotonic::now();
    let b = Monotonic::now();
    assert!(b >= a);
}

#[test]
fn elapsed_is_non_negative() {
    let t = Monotonic::now();
    let _ = t.elapsed();
}

#[test]
fn duration_since_matches_real_sleep() {
    let earlier = Monotonic::now();
    sleep(Duration::from_millis(5));
    let later = Monotonic::now();
    let delta = later.duration_since(earlier);
    assert!(delta >= Duration::from_millis(5));
}

#[test]
fn checked_duration_since_returns_none_when_earlier_is_later() {
    let first = Monotonic::now();
    sleep(Duration::from_millis(1));
    let second = Monotonic::now();
    // Reversed argument order: first < second, so second.checked_duration_since(first) = Some.
    assert!(second.checked_duration_since(first).is_some());
    // Whereas first.checked_duration_since(second) = None.
    assert!(first.checked_duration_since(second).is_none());
}

#[test]
fn saturating_duration_since_floors_at_zero() {
    let first = Monotonic::now();
    sleep(Duration::from_millis(1));
    let second = Monotonic::now();
    assert_eq!(
        first.saturating_duration_since(second),
        Duration::from_secs(0)
    );
}

#[test]
fn copy_and_eq_work_as_expected() {
    let a = Monotonic::now();
    let b = a;
    assert_eq!(a, b);
}
