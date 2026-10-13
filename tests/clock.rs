//! Integration tests for the `Clock` trait, `SystemClock`, and the
//! forwarding implementations for `&C`, `Box<C>` and `Arc<C>`.
#![cfg(feature = "std")]

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use clock_lib::{Clock, ManualClock, Monotonic, SystemClock, Wall};

fn reading_via<C: Clock>(clock: C) -> Monotonic {
    clock.now()
}

#[test]
fn test_system_clock_now_is_bracketed_by_monotonic_now() {
    let before = Monotonic::now();
    let middle = SystemClock.now();
    let after = Monotonic::now();
    assert!(before <= middle);
    assert!(middle <= after);
}

#[test]
fn test_system_clock_wall_is_a_modern_timestamp() {
    assert!(SystemClock::new().wall().unix_seconds() > 1_704_067_200);
}

#[test]
fn test_system_clock_new_is_const_constructible() {
    const CLOCK: SystemClock = SystemClock::new();
    let start = CLOCK.now();
    assert!(CLOCK.elapsed(start) < Duration::from_secs(60));
}

#[test]
fn test_system_clock_default_is_usable() {
    let clock = SystemClock;
    let start = clock.now();
    assert!(clock.now() >= start);
}

#[test]
fn test_clock_elapsed_on_manual_clock_honours_advance() {
    let clock = ManualClock::new();
    let start = clock.now();
    clock.advance(Duration::from_secs(3_600));
    assert_eq!(clock.elapsed(start), Duration::from_secs(3_600));
}

/// Audit repro r05 (CLK-002): the free `elapsed` reads the OS clock, so it
/// ignores a `ManualClock`'s offset. This test documents that contract;
/// `Clock::elapsed` is the right call for injected clocks.
#[test]
fn test_free_elapsed_on_manual_reading_measures_real_time_not_offset() {
    let clock = ManualClock::new();
    let stamp = clock.now();
    clock.advance(Duration::from_secs(3_600));
    assert!(clock_lib::elapsed(stamp) < Duration::from_secs(3_600));
    assert_eq!(clock.elapsed(stamp), Duration::from_secs(3_600));
}

#[test]
fn test_clock_elapsed_for_future_reading_saturates_to_zero() {
    let clock = ManualClock::new();
    let future = clock.now() + Duration::from_secs(5);
    assert_eq!(clock.elapsed(future), Duration::ZERO);
}

#[test]
fn test_reference_clock_dispatches_to_inner_clock() {
    let clock = ManualClock::new();
    let before = reading_via(&clock);
    clock.advance(Duration::from_secs(1));
    assert_eq!(reading_via(&clock) - before, Duration::from_secs(1));
    assert_eq!(Clock::elapsed(&&clock, before), Duration::from_secs(1));
    assert_eq!(Clock::wall(&&clock), clock.wall());
}

#[test]
fn test_arc_clock_dispatches_to_inner_clock() {
    let clock = Arc::new(ManualClock::new());
    let before = reading_via(Arc::clone(&clock));
    clock.advance(Duration::from_secs(1));
    assert_eq!(clock.elapsed(before), Duration::from_secs(1));
    assert_eq!(Clock::wall(&clock), clock.wall());
}

#[test]
fn test_box_clock_dispatches_to_inner_clock() {
    let clock = Box::new(ManualClock::new());
    let before = Clock::now(&clock);
    clock.advance(Duration::from_secs(2));
    assert_eq!(Clock::elapsed(&clock, before), Duration::from_secs(2));
    assert_eq!(Clock::wall(&clock), clock.wall());
}

/// Audit compile probe `cf_box` (CLK-018): `Box<dyn Clock>` now satisfies a
/// `C: Clock` bound.
#[test]
fn test_box_dyn_clock_is_accepted_by_generic_code() {
    let manual = Arc::new(ManualClock::new());
    let boxed: Box<dyn Clock> = Box::new(Arc::clone(&manual));
    let before = boxed.now();
    manual.advance(Duration::from_millis(5));
    assert_eq!(reading_via(boxed) - before, Duration::from_millis(5));
}

#[test]
fn test_arc_dyn_clock_is_accepted_by_generic_code() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let start = reading_via(Arc::clone(&clock));
    assert!(clock.elapsed(start) < Duration::from_secs(60));
    assert!(clock.wall().unix_seconds() > 0);
}

#[test]
fn test_dyn_reference_clock_is_accepted_by_generic_code() {
    let manual = ManualClock::new();
    let clock: &dyn Clock = &manual;
    let start = reading_via(clock);
    manual.advance(Duration::from_secs(9));
    assert_eq!(clock.elapsed(start), Duration::from_secs(9));
}

/// Feature additivity: the conversion impls take and return the `std::time`
/// types under every feature combination CI builds (all, default, none of
/// the optional ones). No feature may change these signatures.
#[test]
fn test_conversions_use_std_types_under_every_feature_set() {
    fn to_readings(instant: Instant, time: SystemTime) -> (Monotonic, Wall) {
        (Monotonic::from(instant), Wall::from(time))
    }
    fn from_readings(reading: Monotonic, stamp: Wall) -> (Instant, SystemTime) {
        (reading.into(), stamp.into())
    }
    let instant = Instant::now();
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    let (reading, stamp) = to_readings(instant, time);
    assert_eq!(from_readings(reading, stamp), (instant, time));
}

/// CLK-003: a custom clock can be implemented from std types alone.
#[test]
fn test_custom_clock_built_from_std_conversions() {
    struct Frozen {
        mono: Instant,
        wall: SystemTime,
    }
    impl Clock for Frozen {
        fn now(&self) -> Monotonic {
            Monotonic::from(self.mono)
        }
        fn wall(&self) -> Wall {
            Wall::from(self.wall)
        }
    }

    let frozen = Frozen {
        mono: Instant::now(),
        wall: SystemTime::UNIX_EPOCH + Duration::from_secs(60),
    };
    let start = frozen.now();
    assert_eq!(frozen.elapsed(start), Duration::ZERO);
    assert_eq!(frozen.wall().to_string(), "1970-01-01T00:01:00Z");
    assert_eq!(Instant::from(frozen.now()), frozen.mono);
}
