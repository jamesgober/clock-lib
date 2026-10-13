//! Integration tests for `Stopwatch`.
#![cfg(feature = "std")]

use std::thread::sleep;
use std::time::Duration;

use clock_lib::{ManualClock, Stopwatch};

#[test]
fn test_stopwatch_on_system_clock_measures_real_time() {
    let watch = Stopwatch::start();
    sleep(Duration::from_millis(2));
    assert!(watch.elapsed() >= Duration::from_millis(2));
}

#[test]
fn test_stopwatch_elapsed_tracks_manual_clock_exactly() {
    let clock = ManualClock::new();
    let watch = Stopwatch::start_with(&clock);
    assert_eq!(watch.elapsed(), Duration::ZERO);
    clock.advance(Duration::from_millis(1_234));
    assert_eq!(watch.elapsed(), Duration::from_millis(1_234));
}

#[test]
fn test_stopwatch_laps_partition_total_elapsed() {
    let clock = ManualClock::new();
    let mut watch = Stopwatch::start_with(&clock);
    let mut laps = Vec::new();
    for ms in [10_u64, 0, 25, 5] {
        clock.advance(Duration::from_millis(ms));
        laps.push(watch.lap());
    }
    assert_eq!(laps, [10_u64, 0, 25, 5].map(Duration::from_millis).to_vec());
    assert_eq!(watch.elapsed(), Duration::from_millis(40));
}

#[test]
fn test_stopwatch_restart_returns_total_and_resets() {
    let clock = ManualClock::new();
    let mut watch = Stopwatch::start_with(&clock);
    clock.advance(Duration::from_secs(3));
    let _ = watch.lap();
    clock.advance(Duration::from_secs(1));
    assert_eq!(watch.restart(), Duration::from_secs(4));
    assert_eq!(watch.elapsed(), Duration::ZERO);
    assert_eq!(watch.lap(), Duration::ZERO);
    assert_eq!(watch.started_at(), clock.now());
}

#[test]
fn test_stopwatch_started_at_and_clock_accessors() {
    let clock = ManualClock::new();
    let start = clock.now();
    let watch = Stopwatch::start_with(&clock);
    assert_eq!(watch.started_at(), start);
    watch.clock().advance(Duration::from_secs(1));
    assert_eq!(watch.elapsed(), Duration::from_secs(1));
}

#[test]
fn test_stopwatch_with_auto_advance_counts_each_reading() {
    let clock = ManualClock::new();
    clock.set_auto_advance(Duration::from_millis(1));
    let mut watch = Stopwatch::start_with(&clock);
    // start took offset 0; lap reads 1ms.
    assert_eq!(watch.lap(), Duration::from_millis(1));
    // elapsed reads 2ms.
    assert_eq!(watch.elapsed(), Duration::from_millis(2));
}

#[test]
fn test_stopwatch_is_copy_with_copy_clock() {
    let clock = ManualClock::new();
    let a = Stopwatch::start_with(&clock);
    let b = a;
    assert_eq!(a.started_at(), b.started_at());
    assert!(format!("{a:?}").starts_with("Stopwatch"));
}
