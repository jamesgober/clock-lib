//! Integration tests for `Deadline`.
#![cfg(feature = "std")]

use std::sync::Arc;
use std::time::Duration;

use clock_lib::{Clock, Deadline, ManualClock};

#[test]
fn test_deadline_after_system_clock_is_not_expired_immediately() {
    let deadline = Deadline::after(Duration::from_secs(60));
    assert!(!deadline.is_expired());
    assert!(deadline.remaining() > Duration::from_secs(50));
    assert!(deadline.remaining() <= Duration::from_secs(60));
}

#[test]
fn test_deadline_after_zero_is_expired_immediately() {
    assert!(Deadline::after(Duration::ZERO).is_expired());
    assert_eq!(Deadline::after(Duration::ZERO).remaining(), Duration::ZERO);
}

#[test]
fn test_deadline_expires_exactly_at_ttl() {
    let clock = ManualClock::new();
    let deadline = Deadline::after_with(&clock, Duration::from_secs(10));
    clock.advance(Duration::from_nanos(9_999_999_999));
    assert!(!deadline.is_expired());
    assert_eq!(deadline.remaining(), Duration::from_nanos(1));
    clock.advance(Duration::from_nanos(1));
    assert!(deadline.is_expired());
    assert_eq!(deadline.remaining(), Duration::ZERO);
}

#[test]
fn test_deadline_remaining_saturates_after_expiry() {
    let clock = ManualClock::new();
    let deadline = Deadline::after_with(&clock, Duration::from_secs(1));
    clock.advance(Duration::from_secs(100));
    assert_eq!(deadline.remaining(), Duration::ZERO);
}

#[test]
fn test_deadline_with_max_ttl_never_expires() {
    let clock = ManualClock::new();
    let deadline = Deadline::after_with(&clock, Duration::MAX);
    clock.advance(Duration::from_secs(365 * 86_400 * 100));
    assert!(!deadline.is_expired());
    assert!(deadline.remaining() > Duration::ZERO);
}

#[test]
fn test_deadline_at_uses_given_reading() {
    let clock = ManualClock::new();
    let at = clock.now() + Duration::from_millis(250);
    let deadline = Deadline::at(&clock, at);
    assert_eq!(deadline.expires_at(), at);
    assert_eq!(deadline.remaining(), Duration::from_millis(250));
}

#[test]
fn test_deadline_expires_at_is_start_plus_ttl() {
    let clock = ManualClock::new();
    let start = clock.now();
    let deadline = Deadline::after_with(&clock, Duration::from_secs(7));
    assert_eq!(deadline.expires_at() - start, Duration::from_secs(7));
}

#[test]
fn test_deadline_works_with_shared_and_boxed_clocks() {
    let clock = Arc::new(ManualClock::new());
    let shared = Deadline::after_with(Arc::clone(&clock), Duration::from_secs(1));
    let boxed: Box<dyn Clock> = Box::new(Arc::clone(&clock));
    let dynamic = Deadline::after_with(boxed, Duration::from_secs(2));
    clock.advance(Duration::from_secs(1));
    assert!(shared.is_expired());
    assert!(!dynamic.is_expired());
    assert_eq!(dynamic.remaining(), Duration::from_secs(1));
}

#[test]
fn test_deadline_clock_accessor_returns_bound_clock() {
    let clock = ManualClock::new();
    let deadline = Deadline::after_with(&clock, Duration::from_secs(1));
    deadline.clock().advance(Duration::from_secs(1));
    assert!(deadline.is_expired());
}

#[test]
fn test_deadline_over_dyn_clock_is_debug() {
    let boxed: Box<dyn Clock> = Box::new(ManualClock::new());
    let deadline = Deadline::after_with(boxed, Duration::from_secs(1));
    let text = format!("{deadline:?}");
    assert!(text.starts_with("Deadline { clock: "));
    assert!(text.contains("Box<dyn "));
    assert!(text.contains("expires_at: Monotonic("));
}

#[test]
fn test_deadline_is_copy_with_copy_clock() {
    let clock = ManualClock::new();
    let a = Deadline::after_with(&clock, Duration::from_secs(1));
    let b = a;
    assert_eq!(a.expires_at(), b.expires_at());
    assert!(format!("{a:?}").starts_with("Deadline"));
}
