//! Smoke tests covering the Tier-1 surface end-to-end.

use std::thread::sleep;
use std::time::Duration;

#[test]
fn version_is_set() {
    assert!(!clock_lib::VERSION.is_empty());
}

#[test]
fn now_advances_across_a_sleep() {
    let start = clock_lib::now();
    sleep(Duration::from_millis(2));
    let took = clock_lib::elapsed(start);
    assert!(took >= Duration::from_millis(2));
}

#[test]
fn wall_returns_a_modern_timestamp() {
    // 2026-05-21 is well past 2024-01-01 (1704067200).
    let seconds = clock_lib::wall().unix_seconds();
    assert!(seconds > 1_704_067_200);
}

#[test]
fn unix_helpers_scale_consistently() {
    let secs = clock_lib::unix();
    let ms = clock_lib::unix_ms();
    let ns = clock_lib::unix_ns();

    // ms is at least seconds * 1000; ns is at least ms * 1_000_000.
    // Allow forward drift between calls but reject any going-backwards.
    assert!(ms >= u128::from(secs) * 1_000);
    assert!(ns >= ms * 1_000_000);
}
