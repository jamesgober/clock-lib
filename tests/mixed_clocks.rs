//! Readings from different clocks share the `Monotonic` type. In debug
//! builds, measuring one against another panics instead of returning a
//! plausible but wrong duration; same-clock use is unaffected.
#![cfg(all(feature = "std", debug_assertions))]

use std::time::{Duration, Instant};

use clock_lib::{Clock, Deadline, ManualClock, Monotonic, SystemClock};

#[test]
#[should_panic(expected = "different clock")]
fn test_os_reading_minus_manual_reading_panics() {
    let manual = ManualClock::new();
    let stamp = manual.now();
    let _ = Monotonic::now().duration_since(stamp);
}

#[test]
#[should_panic(expected = "different clock")]
fn test_manual_clock_elapsed_of_an_os_reading_panics() {
    let manual = ManualClock::new();
    let stamp = Monotonic::now();
    let _ = manual.elapsed(stamp);
}

#[test]
#[should_panic(expected = "operating system clock")]
fn test_monotonic_elapsed_of_a_manual_reading_panics() {
    let manual = ManualClock::new();
    let stamp = manual.now();
    manual.advance(Duration::from_secs(3600));
    let _ = stamp.elapsed();
}

#[test]
#[should_panic(expected = "operating system clock")]
fn test_free_elapsed_of_a_manual_reading_panics() {
    let manual = ManualClock::new();
    let _ = clock_lib::elapsed(manual.now());
}

#[test]
#[should_panic(expected = "different clock")]
fn test_two_manual_clocks_do_not_mix() {
    let a = ManualClock::new();
    let b = ManualClock::new();
    let _ = a.now() - b.now();
}

#[test]
#[should_panic(expected = "different clock")]
fn test_deadline_on_the_os_clock_with_a_manual_reading_panics_when_used() {
    let manual = ManualClock::new();
    let deadline = Deadline::at(SystemClock, manual.now() + Duration::from_secs(60));
    let _ = deadline.remaining();
}

#[test]
#[should_panic(expected = "different clock")]
fn test_checked_and_saturating_forms_are_checked_too() {
    let manual = ManualClock::new();
    let _ = Monotonic::now().checked_duration_since(manual.now());
}

#[test]
fn test_same_clock_use_is_unaffected() {
    let manual = ManualClock::new();
    let start = manual.now();
    manual.advance(Duration::from_secs(5));
    assert_eq!(manual.elapsed(start), Duration::from_secs(5));
    assert_eq!(manual.now() - start, Duration::from_secs(5));
    let deadline = Deadline::at(&manual, start + Duration::from_secs(10));
    assert_eq!(deadline.remaining(), Duration::from_secs(5));
    // Arithmetic keeps the clock: a moved reading is still this clock's.
    let later = (start + Duration::from_secs(1)).saturating_sub(Duration::from_secs(1));
    assert_eq!(manual.elapsed(later), Duration::from_secs(5));

    let os = Monotonic::now();
    assert!(os.elapsed() < Duration::from_secs(60));
    assert!(SystemClock.elapsed(os) < Duration::from_secs(60));
}

#[test]
fn test_comparisons_and_conversions_are_not_checked() {
    let manual = ManualClock::new();
    let reading = manual.now();
    // Equality and order look at the instant only.
    let round_trip = Monotonic::from(Instant::from(reading));
    assert_eq!(round_trip, reading);
    let _ordered = Monotonic::now() >= reading;
    // A reading rebuilt from an Instant counts as the operating system's.
    assert!(round_trip.elapsed() < Duration::from_secs(3600));
}
