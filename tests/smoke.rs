//! Smoke tests covering the Tier-1 surface end-to-end.

use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_version_const_is_non_empty() {
    assert!(!clock_lib::VERSION.is_empty());
}

#[test]
fn test_now_after_sleep_advances_at_least_sleep_duration() {
    let start = clock_lib::now();
    sleep(Duration::from_millis(2));
    let took = clock_lib::elapsed(start);
    assert!(took >= Duration::from_millis(2));
}

#[test]
fn test_wall_unix_seconds_returns_modern_timestamp() {
    // Any modern system clock is after 2024-01-01 UTC (1704067200).
    let seconds = clock_lib::wall().unix_seconds();
    assert!(seconds > 1_704_067_200);
}

#[test]
fn test_unix_helpers_scale_consistently_across_units() {
    let secs = clock_lib::unix();
    let ms = clock_lib::unix_ms();
    let ns = clock_lib::unix_ns();

    // ms is at least seconds * 1000; ns is at least ms * 1_000_000.
    // Forward drift between calls is allowed; going backwards is not.
    assert!(ms >= u128::from(secs) * 1_000);
    assert!(ns >= ms * 1_000_000);
}
