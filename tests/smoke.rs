//! Smoke tests covering the Tier-1 surface end-to-end against the real
//! operating system clock.
#![cfg(feature = "std")]

use std::thread::sleep;
use std::time::Duration;

/// 2024-01-01T00:00:00Z. Any modern system clock is past it.
const JAN_2024: u64 = 1_704_067_200;

#[test]
fn test_version_const_matches_cargo_version() {
    assert_eq!(clock_lib::VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_now_after_sleep_advances_at_least_sleep_duration() {
    let start = clock_lib::now();
    sleep(Duration::from_millis(2));
    let took = clock_lib::elapsed(start);
    assert!(took >= Duration::from_millis(2));
}

#[test]
fn test_now_plus_duration_is_a_future_deadline() {
    let deadline = clock_lib::now() + Duration::from_secs(60);
    assert!(clock_lib::now() < deadline);
}

#[test]
fn test_wall_unix_seconds_returns_modern_timestamp() {
    assert!(clock_lib::wall().unix_seconds() > JAN_2024);
}

/// CLK-015: these thresholds kill the `unix() -> 1` style mutants.
#[test]
fn test_unix_helpers_return_modern_timestamps() {
    assert!(clock_lib::unix() > JAN_2024);
    assert!(clock_lib::unix_ms() > u128::from(JAN_2024) * 1_000);
    assert!(clock_lib::unix_ns() > u128::from(JAN_2024) * 1_000_000_000);
}

#[test]
fn test_unix_helpers_agree_within_a_minute() {
    let secs = u128::from(clock_lib::unix());
    let ms = clock_lib::unix_ms();
    let ns = clock_lib::unix_ns();
    // Three separate OS reads; allow for scheduling delay and NTP steps.
    assert!(ms / 1_000 + 60 >= secs && ms / 1_000 <= secs + 60);
    assert!(ns / 1_000_000 + 60_000 >= ms && ns / 1_000_000 <= ms + 60_000);
}

#[test]
fn test_wall_display_ends_with_utc_designator() {
    let text = clock_lib::wall().to_string();
    assert!(text.starts_with("20"));
    assert!(text.ends_with('Z'));
    assert_eq!(text.as_bytes().get(10), Some(&b'T'));
}
