//! Integration tests for `Monotonic` readings.

use std::thread::sleep;
use std::time::Duration;

use clock_lib::Monotonic;

#[test]
fn test_now_called_twice_is_non_decreasing() {
    let a = Monotonic::now();
    let b = Monotonic::now();
    assert!(b >= a);
}

#[test]
fn test_elapsed_after_construction_returns_non_negative_duration() {
    let t = Monotonic::now();
    // The call returns successfully; non-negative is guaranteed by the type.
    let _ = t.elapsed();
}

#[test]
fn test_duration_since_after_sleep_meets_sleep_floor() {
    let earlier = Monotonic::now();
    sleep(Duration::from_millis(5));
    let later = Monotonic::now();
    let delta = later.duration_since(earlier);
    assert!(delta >= Duration::from_millis(5));
}

#[test]
fn test_checked_duration_since_with_later_arg_returns_none() {
    let first = Monotonic::now();
    sleep(Duration::from_millis(1));
    let second = Monotonic::now();
    assert!(second.checked_duration_since(first).is_some());
    assert!(first.checked_duration_since(second).is_none());
}

#[test]
fn test_saturating_duration_since_with_later_arg_returns_zero() {
    let first = Monotonic::now();
    sleep(Duration::from_millis(1));
    let second = Monotonic::now();
    assert_eq!(
        first.saturating_duration_since(second),
        Duration::from_secs(0)
    );
}

#[test]
fn test_copy_of_monotonic_equals_original() {
    let a = Monotonic::now();
    let b = a;
    assert_eq!(a, b);
}
