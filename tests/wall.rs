//! Integration tests for `Wall` readings.

use clock_lib::Wall;

#[test]
fn wall_now_returns_a_recent_timestamp() {
    // Sanity floor: any modern system clock is after 2024-01-01 UTC.
    assert!(Wall::now().unix_seconds() > 1_704_067_200);
}

#[test]
fn unix_seconds_and_unix_millis_agree() {
    let stamp = Wall::now();
    let secs = stamp.unix_seconds();
    let ms = stamp.unix_millis();
    // ms must be in the same second-window as `secs`.
    let floor = u128::from(secs) * 1_000;
    assert!(ms >= floor);
    assert!(ms < floor + 1_000);
}

#[test]
fn unix_millis_and_unix_nanos_agree() {
    let stamp = Wall::now();
    let ms = stamp.unix_millis();
    let ns = stamp.unix_nanos();
    let floor = ms * 1_000_000;
    assert!(ns >= floor);
    assert!(ns < floor + 1_000_000);
}

#[test]
fn wall_ordering_is_consistent_with_capture_order() {
    let a = Wall::now();
    let b = Wall::now();
    // The wall clock may not be strictly increasing (NTP can step it back), but
    // for two back-to-back reads on a sane system this should hold.
    assert!(b >= a);
}
