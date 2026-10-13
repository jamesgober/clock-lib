//! Integration tests for `ManualClock`: advancing, saturation, fixed starts,
//! wall-only steps and auto-advance.
#![cfg(feature = "std")]

use std::time::Duration;

use clock_lib::{Clock, ManualClock, Wall};

fn wall_at(secs: u64) -> Wall {
    Wall::from_unix_seconds(secs).unwrap_or(Wall::UNIX_EPOCH)
}

// --- advance -------------------------------------------------------------

#[test]
fn test_manual_clock_without_advance_returns_same_reading() {
    let clock = ManualClock::new();
    assert_eq!(clock.now(), clock.now());
    assert_eq!(clock.wall(), clock.wall());
}

#[test]
fn test_manual_clock_advance_moves_monotonic_by_exact_amount() {
    let clock = ManualClock::new();
    let before = clock.now();
    clock.advance(Duration::from_nanos(1_234_567_891));
    assert_eq!(
        clock.now().duration_since(before),
        Duration::from_nanos(1_234_567_891)
    );
}

#[test]
fn test_manual_clock_advance_moves_wall_by_exact_amount() {
    let clock = ManualClock::with_start(wall_at(1_000));
    clock.advance(Duration::from_secs(60));
    assert_eq!(clock.wall().unix_seconds(), 1_060);
    clock.advance(Duration::from_millis(250));
    assert_eq!(clock.wall().unix_millis(), 1_060_250);
}

#[test]
fn test_manual_clock_advance_zero_changes_nothing() {
    let clock = ManualClock::new();
    let (now, wall) = (clock.now(), clock.wall());
    clock.advance(Duration::ZERO);
    assert_eq!(clock.now(), now);
    assert_eq!(clock.wall(), wall);
    assert_eq!(clock.offset(), Duration::ZERO);
}

#[test]
fn test_manual_clock_offset_accumulates_across_advances() {
    let clock = ManualClock::new();
    clock.advance(Duration::from_secs(1));
    clock.advance(Duration::from_secs(2));
    clock.advance(Duration::from_millis(500));
    assert_eq!(clock.offset(), Duration::from_millis(3_500));
}

/// Audit repro r01 (CLK-001): the offset used to wrap with `fetch_add`.
#[test]
fn test_manual_clock_advance_past_u64_max_saturates_instead_of_wrapping() {
    let clock = ManualClock::new();
    let t0 = clock.now();
    clock.advance(Duration::from_nanos(u64::MAX - 5));
    let t1 = clock.now();
    clock.advance(Duration::from_nanos(10));
    let t2 = clock.now();
    assert!(t1 > t0);
    assert!(t2 >= t1, "clock went backwards after overflow");
    assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
}

/// Audit repro r02 (CLK-001): `advance(Duration::MAX)` twice used to
/// decrease the offset.
#[test]
fn test_manual_clock_advance_duration_max_twice_stays_saturated() {
    let clock = ManualClock::new();
    clock.advance(Duration::MAX);
    let once = clock.offset();
    clock.advance(Duration::MAX);
    assert_eq!(clock.offset(), once);
    assert_eq!(once, Duration::from_nanos(u64::MAX));
}

/// Audit repro r03: readings at the maximum offset must not panic.
#[test]
fn test_manual_clock_readings_at_max_offset_do_not_panic() {
    let clock = ManualClock::new();
    let before = clock.now();
    clock.advance(Duration::MAX);
    assert!(clock.now() >= before);
    assert!(clock.wall().unix_seconds() > 0);
    // Further advances are no-ops; readings stay put.
    let saturated = clock.now();
    clock.advance(Duration::from_secs(1));
    assert_eq!(clock.now(), saturated);
}

/// CLK-006: on Windows, wall readings move in 100 ns ticks. The error is
/// bounded below one tick and never accumulates.
#[test]
fn test_manual_clock_sub_tick_advances_bound_wall_error_below_one_tick() {
    let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
    let mut expected: u128 = 0;
    for step in [1_u64, 23, 99, 123, 777, 1_001] {
        clock.advance(Duration::from_nanos(step));
        expected += u128::from(step);
        let actual = clock.wall().unix_nanos();
        assert!(actual <= expected);
        assert!(expected - actual < 100, "wall error {expected} - {actual}");
    }
    // Monotonic deltas are exact on every platform.
    assert_eq!(clock.offset(), Duration::from_nanos(2_024));
}

// --- advance_to ----------------------------------------------------------

#[test]
fn test_manual_clock_advance_to_moves_forward_only() {
    let clock = ManualClock::new();
    let start = clock.now();
    clock.advance_to(Duration::from_secs(10));
    assert_eq!(clock.now() - start, Duration::from_secs(10));
    clock.advance_to(Duration::from_secs(3));
    assert_eq!(clock.offset(), Duration::from_secs(10));
    clock.advance(Duration::from_secs(1));
    clock.advance_to(Duration::from_secs(11));
    assert_eq!(clock.offset(), Duration::from_secs(11));
    clock.advance_to(Duration::MAX);
    assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
}

// --- fixed start (deterministic wall time) -------------------------------

#[test]
fn test_manual_clock_with_start_produces_reproducible_timestamps() {
    let render = || {
        let clock = ManualClock::with_start(wall_at(1_700_000_000));
        clock.advance(Duration::from_millis(1_500));
        clock.wall().to_string()
    };
    assert_eq!(render(), "2023-11-14T22:13:21.500Z");
    assert_eq!(render(), render());
}

#[test]
fn test_manual_clock_with_start_before_epoch_reports_signed_time() {
    let start = Wall::UNIX_EPOCH
        .checked_sub(Duration::from_secs(10))
        .unwrap_or(Wall::UNIX_EPOCH);
    let clock = ManualClock::with_start(start);
    assert_eq!(clock.wall().unix_seconds_signed(), -10);
    clock.advance(Duration::from_secs(15));
    assert_eq!(clock.wall().unix_seconds_signed(), 5);
}

#[test]
fn test_manual_clock_new_starts_at_current_wall_time() {
    let before = Wall::now();
    let clock = ManualClock::new();
    let after = Wall::now();
    // Wall time may be stepped by the OS between reads; this check only
    // needs the anchor to be a modern timestamp near "now".
    let anchor = clock.wall().unix_seconds();
    assert!(anchor + 60 >= before.unix_seconds());
    assert!(anchor <= after.unix_seconds() + 60);
}

#[test]
fn test_manual_clock_default_matches_new() {
    let clock = ManualClock::default();
    assert_eq!(clock.offset(), Duration::ZERO);
    assert_eq!(clock.auto_advance(), Duration::ZERO);
    assert!(clock.wall().unix_seconds() > 1_704_067_200);
}

// --- wall-only steps -----------------------------------------------------

#[test]
fn test_manual_clock_step_wall_back_leaves_monotonic_untouched() {
    let clock = ManualClock::with_start(wall_at(1_000));
    let start = clock.now();
    clock.advance(Duration::from_secs(5));
    clock.step_wall_back(Duration::from_secs(60));
    assert_eq!(clock.wall().unix_seconds(), 945);
    assert_eq!(clock.elapsed(start), Duration::from_secs(5));
    assert_eq!(clock.offset(), Duration::from_secs(5));
}

#[test]
fn test_manual_clock_step_wall_forward_then_advance_compose() {
    let clock = ManualClock::with_start(wall_at(1_000));
    let start = clock.now();
    clock.step_wall_forward(Duration::from_secs(30));
    assert_eq!(clock.now(), start);
    clock.advance(Duration::from_secs(1));
    assert_eq!(clock.wall().unix_seconds(), 1_031);
}

#[test]
fn test_manual_clock_step_wall_back_can_cross_the_epoch() {
    let clock = ManualClock::with_start(wall_at(5));
    clock.step_wall_back(Duration::from_secs(8));
    assert_eq!(clock.wall().unix_seconds_signed(), -3);
    assert_eq!(clock.wall().unix_seconds(), 0);
    clock.advance(Duration::from_secs(4));
    assert_eq!(clock.wall().unix_seconds_signed(), 1);
}

#[test]
fn test_manual_clock_step_wall_saturates_without_panicking() {
    let clock = ManualClock::new();
    clock.step_wall_forward(Duration::MAX);
    clock.step_wall_forward(Duration::MAX);
    let _far_future = clock.wall();
    clock.step_wall_back(Duration::MAX);
    clock.step_wall_back(Duration::MAX);
    clock.step_wall_back(Duration::MAX);
    let _far_past = clock.wall();
}

#[test]
fn test_manual_clock_set_wall_then_advance_moves_from_target() {
    let clock = ManualClock::with_start(wall_at(1_000));
    clock.advance(Duration::from_secs(10));
    let start = clock.now();
    clock.set_wall(wall_at(1_800_000_000));
    assert_eq!(clock.wall().unix_seconds(), 1_800_000_000);
    assert_eq!(clock.now(), start);
    clock.advance(Duration::from_secs(2));
    assert_eq!(clock.wall().unix_seconds(), 1_800_000_002);
    clock.set_wall(wall_at(500));
    assert_eq!(clock.wall().unix_seconds(), 500);
}

#[test]
fn test_manual_clock_set_wall_is_exact_at_full_resolution() {
    let clock = ManualClock::new();
    let target = Wall::from_unix_nanos(1_234_567_890_123_456_700).unwrap_or(Wall::UNIX_EPOCH);
    clock.set_wall(target);
    assert_eq!(clock.wall(), target);
}

// --- auto-advance --------------------------------------------------------

#[test]
fn test_manual_clock_auto_advance_moves_after_each_reading() {
    let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
    clock.set_auto_advance(Duration::from_millis(10));
    let a = clock.now();
    let b = clock.now();
    assert_eq!(b - a, Duration::from_millis(10));
    // The wall reading also claims a step; it sees the offset as of the read.
    assert_eq!(clock.wall().unix_millis(), 20);
    assert_eq!(clock.offset(), Duration::from_millis(30));
}

#[test]
fn test_manual_clock_auto_advance_applies_to_clock_elapsed() {
    let clock = ManualClock::new();
    let start = clock.now();
    clock.set_auto_advance(Duration::from_secs(1));
    assert_eq!(clock.elapsed(start), Duration::ZERO);
    assert_eq!(clock.elapsed(start), Duration::from_secs(1));
}

#[test]
fn test_manual_clock_auto_advance_off_restores_fixed_readings() {
    let clock = ManualClock::new();
    clock.set_auto_advance(Duration::from_nanos(1));
    let _ = clock.now();
    clock.set_auto_advance(Duration::ZERO);
    assert_eq!(clock.auto_advance(), Duration::ZERO);
    assert_eq!(clock.now(), clock.now());
    assert_eq!(clock.offset(), Duration::from_nanos(1));
}

#[test]
fn test_manual_clock_offset_read_does_not_auto_advance() {
    let clock = ManualClock::new();
    clock.set_auto_advance(Duration::from_secs(1));
    assert_eq!(clock.offset(), Duration::ZERO);
    assert_eq!(clock.offset(), Duration::ZERO);
}

#[test]
fn test_manual_clock_auto_advance_saturates_at_max() {
    let clock = ManualClock::new();
    clock.advance(Duration::from_nanos(u64::MAX - 1));
    clock.set_auto_advance(Duration::MAX);
    assert_eq!(clock.auto_advance(), Duration::from_nanos(u64::MAX));
    let a = clock.now();
    let b = clock.now();
    let c = clock.now();
    assert!(b >= a && c >= b);
    assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
}

// --- inherent methods match the trait --------------------------------------

#[test]
fn test_manual_clock_inherent_and_trait_readings_agree() {
    let clock = ManualClock::with_start(wall_at(42));
    clock.advance(Duration::from_secs(1));
    assert_eq!(ManualClock::now(&clock), Clock::now(&clock));
    assert_eq!(ManualClock::wall(&clock), Clock::wall(&clock));
}
