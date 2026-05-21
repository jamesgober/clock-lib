//! Integration tests for `Wall` readings.

use clock_lib::Wall;

#[test]
fn test_wall_now_unix_seconds_returns_modern_timestamp() {
    // Any modern system clock is after 2024-01-01 UTC.
    assert!(Wall::now().unix_seconds() > 1_704_067_200);
}

#[test]
fn test_unix_seconds_and_unix_millis_are_within_one_second() {
    let stamp = Wall::now();
    let secs = stamp.unix_seconds();
    let ms = stamp.unix_millis();
    let floor = u128::from(secs) * 1_000;
    assert!(ms >= floor);
    assert!(ms < floor + 1_000);
}

#[test]
fn test_unix_millis_and_unix_nanos_are_within_one_millisecond() {
    let stamp = Wall::now();
    let ms = stamp.unix_millis();
    let ns = stamp.unix_nanos();
    let floor = ms * 1_000_000;
    assert!(ns >= floor);
    assert!(ns < floor + 1_000_000);
}

#[test]
fn test_wall_consecutive_reads_are_non_decreasing() {
    let a = Wall::now();
    let b = Wall::now();
    // Wall is not strictly monotonic (NTP can step it back), but for two
    // back-to-back reads on a sane system this should hold.
    assert!(b >= a);
}
