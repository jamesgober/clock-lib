//! Integration tests for `Wall` readings: constructors, Unix accessors,
//! signed accessors, arithmetic, conversions and RFC 3339 formatting.
#![cfg(feature = "std")]

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clock_lib::Wall;

fn at_secs(secs: u64) -> Wall {
    Wall::from_unix_seconds(secs).unwrap_or(Wall::UNIX_EPOCH)
}

fn before_epoch(d: Duration) -> Wall {
    Wall::UNIX_EPOCH.checked_sub(d).unwrap_or(Wall::UNIX_EPOCH)
}

#[test]
fn test_wall_now_unix_seconds_returns_modern_timestamp() {
    // Any modern system clock is after 2024-01-01 UTC.
    assert!(Wall::now().unix_seconds() > 1_704_067_200);
}

#[test]
fn test_unix_seconds_and_unix_millis_are_within_one_second() {
    let stamp = Wall::now();
    let floor = u128::from(stamp.unix_seconds()) * 1_000;
    let ms = stamp.unix_millis();
    assert!(ms >= floor);
    assert!(ms < floor + 1_000);
}

#[test]
fn test_unix_millis_and_unix_nanos_are_within_one_millisecond() {
    let stamp = Wall::now();
    let floor = stamp.unix_millis() * 1_000_000;
    let ns = stamp.unix_nanos();
    assert!(ns >= floor);
    assert!(ns < floor + 1_000_000);
}

#[test]
fn test_unix_epoch_constant_is_zero() {
    assert_eq!(Wall::UNIX_EPOCH.unix_seconds(), 0);
    assert_eq!(Wall::UNIX_EPOCH.unix_millis(), 0);
    assert_eq!(Wall::UNIX_EPOCH.unix_nanos(), 0);
    assert_eq!(Wall::UNIX_EPOCH.unix_nanos_signed(), 0);
    assert_eq!(SystemTime::from(Wall::UNIX_EPOCH), UNIX_EPOCH);
}

#[test]
fn test_from_unix_constructors_round_trip() {
    assert_eq!(
        Wall::from_unix_seconds(1_700_000_000).map(Wall::unix_seconds),
        Some(1_700_000_000)
    );
    assert_eq!(
        Wall::from_unix_millis(1_700_000_000_123).map(Wall::unix_millis),
        Some(1_700_000_000_123)
    );
    assert_eq!(
        Wall::from_unix_nanos(1_700_000_000_123_456_700).map(Wall::unix_nanos),
        Some(1_700_000_000_123_456_700)
    );
}

#[test]
fn test_from_unix_constructors_out_of_range_return_none() {
    assert_eq!(Wall::from_unix_seconds(u64::MAX), None);
    assert_eq!(Wall::from_unix_millis(u128::MAX), None);
    assert_eq!(Wall::from_unix_nanos(u128::MAX), None);
}

#[test]
fn test_unsigned_accessors_saturate_before_epoch() {
    let w = before_epoch(Duration::from_secs(5));
    assert_eq!(w.unix_seconds(), 0);
    assert_eq!(w.unix_millis(), 0);
    assert_eq!(w.unix_nanos(), 0);
}

#[test]
fn test_signed_accessors_report_pre_epoch_values() {
    let w = before_epoch(Duration::from_millis(2_500));
    assert_eq!(w.unix_seconds_signed(), -3);
    assert_eq!(w.unix_millis_signed(), -2_500);
    assert_eq!(w.unix_nanos_signed(), -2_500_000_000);
}

#[test]
fn test_signed_accessors_round_toward_negative_infinity() {
    let w = before_epoch(Duration::from_micros(1));
    assert_eq!(w.unix_seconds_signed(), -1);
    assert_eq!(w.unix_millis_signed(), -1);
    assert_eq!(w.unix_nanos_signed(), -1_000);
}

#[test]
fn test_signed_accessors_match_unsigned_after_epoch() {
    let w = Wall::from_unix_millis(1_700_000_000_999).unwrap_or(Wall::UNIX_EPOCH);
    assert_eq!(w.unix_seconds_signed(), 1_700_000_000);
    assert_eq!(w.unix_millis_signed(), 1_700_000_000_999);
    assert_eq!(w.unix_nanos_signed(), 1_700_000_000_999_000_000);
}

#[test]
fn test_checked_add_and_sub() {
    let w = at_secs(1_000);
    assert_eq!(
        w.checked_add(Duration::from_secs(1))
            .map(Wall::unix_seconds),
        Some(1_001)
    );
    assert_eq!(
        w.checked_sub(Duration::from_secs(1))
            .map(Wall::unix_seconds),
        Some(999)
    );
    assert_eq!(w.checked_add(Duration::MAX), None);
    assert_eq!(w.checked_sub(Duration::MAX), None);
}

#[test]
fn test_conversions_to_and_from_system_time_are_lossless() {
    let now = SystemTime::now();
    let stamp = Wall::from(now);
    let back: SystemTime = stamp.into();
    assert_eq!(back, now);
    let via_into: Wall = now.into();
    assert_eq!(via_into, stamp);
}

#[test]
fn test_display_formats_rfc3339_utc() {
    assert_eq!(Wall::UNIX_EPOCH.to_string(), "1970-01-01T00:00:00Z");
    assert_eq!(at_secs(1_700_000_000).to_string(), "2023-11-14T22:13:20Z");
    assert_eq!(at_secs(951_782_400).to_string(), "2000-02-29T00:00:00Z");
    assert_eq!(at_secs(253_402_300_799).to_string(), "9999-12-31T23:59:59Z");
}

#[test]
fn test_display_auto_precision_uses_shortest_exact_width() {
    let ms = Wall::from_unix_millis(1_500).unwrap_or(Wall::UNIX_EPOCH);
    assert_eq!(ms.to_string(), "1970-01-01T00:00:01.500Z");
    let us = Wall::from_unix_nanos(1_000_001_000).unwrap_or(Wall::UNIX_EPOCH);
    assert_eq!(us.to_string(), "1970-01-01T00:00:01.000001Z");
    let ns = Wall::from_unix_nanos(1_000_000_100).unwrap_or(Wall::UNIX_EPOCH);
    assert_eq!(ns.to_string(), "1970-01-01T00:00:01.000000100Z");
}

#[test]
fn test_display_explicit_precision_truncates() {
    let w = Wall::from_unix_nanos(1_999_999_900).unwrap_or(Wall::UNIX_EPOCH);
    assert_eq!(format!("{w:.0}"), "1970-01-01T00:00:01Z");
    assert_eq!(format!("{w:.3}"), "1970-01-01T00:00:01.999Z");
    assert_eq!(format!("{w:.9}"), "1970-01-01T00:00:01.999999900Z");
    assert_eq!(format!("{w:.12}"), "1970-01-01T00:00:01.999999900Z");
}

#[test]
fn test_display_before_epoch() {
    let w = before_epoch(Duration::from_millis(500));
    assert_eq!(w.to_string(), "1969-12-31T23:59:59.500Z");
    let a_day = before_epoch(Duration::from_secs(86_400));
    assert_eq!(a_day.to_string(), "1969-12-31T00:00:00Z");
}

#[test]
fn test_display_width_and_alignment() {
    let w = Wall::UNIX_EPOCH;
    assert_eq!(format!("[{w:>21}]"), "[ 1970-01-01T00:00:00Z]");
    assert_eq!(format!("[{w:-<21}]"), "[1970-01-01T00:00:00Z-]");
}

#[test]
fn test_wall_ordering() {
    let set: BTreeSet<Wall> = [at_secs(3), at_secs(1), at_secs(2), at_secs(1)]
        .into_iter()
        .collect();
    let ordered: Vec<u64> = set.into_iter().map(Wall::unix_seconds).collect();
    assert_eq!(ordered, [1, 2, 3]);
}
