//! Tests for the `serde` feature: `Wall` must be wire-compatible with
//! `std::time::SystemTime`.
#![cfg(feature = "serde")]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clock_lib::Wall;

#[test]
fn test_wall_serializes_like_system_time() {
    let time = UNIX_EPOCH + Duration::new(1_700_000_000, 250_000_000);
    let from_wall = serde_json::to_string(&Wall::from(time));
    let from_std = serde_json::to_string(&time);
    assert!(from_wall.is_ok());
    assert_eq!(from_wall.ok(), from_std.ok());
}

#[test]
fn test_wall_json_shape_is_stable() {
    let wall = Wall::from_unix_millis(1_700_000_000_250).unwrap_or(Wall::UNIX_EPOCH);
    assert_eq!(
        serde_json::to_string(&wall).ok().as_deref(),
        Some(r#"{"secs_since_epoch":1700000000,"nanos_since_epoch":250000000}"#)
    );
}

#[test]
fn test_wall_round_trips_through_json() {
    let wall = Wall::from_unix_nanos(1_234_567_890_123_456_700).unwrap_or(Wall::UNIX_EPOCH);
    let json = serde_json::to_string(&wall).unwrap_or_default();
    let back: Result<Wall, _> = serde_json::from_str(&json);
    assert_eq!(back.ok(), Some(wall));
}

#[test]
fn test_wall_deserializes_data_written_as_system_time() {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400);
    let json = serde_json::to_string(&time).unwrap_or_default();
    let wall: Result<Wall, _> = serde_json::from_str(&json);
    assert_eq!(
        wall.ok().map(|w| w.to_string()).as_deref(),
        Some("1970-01-02T00:00:00Z")
    );
}

#[test]
fn test_system_time_deserializes_data_written_as_wall() {
    let wall = Wall::from_unix_seconds(42).unwrap_or(Wall::UNIX_EPOCH);
    let json = serde_json::to_string(&wall).unwrap_or_default();
    let time: Result<SystemTime, _> = serde_json::from_str(&json);
    assert_eq!(time.ok(), Some(UNIX_EPOCH + Duration::from_secs(42)));
}

#[test]
fn test_wall_epoch_serializes() {
    assert_eq!(
        serde_json::to_string(&Wall::UNIX_EPOCH).ok().as_deref(),
        Some(r#"{"secs_since_epoch":0,"nanos_since_epoch":0}"#)
    );
}

#[test]
fn test_wall_before_epoch_fails_to_serialize() {
    let before = Wall::UNIX_EPOCH
        .checked_sub(Duration::from_secs(1))
        .unwrap_or(Wall::UNIX_EPOCH);
    let err = serde_json::to_string(&before);
    assert!(err.is_err());
    assert!(
        err.err()
            .is_some_and(|e| e.to_string().contains("UNIX_EPOCH"))
    );
}

#[test]
fn test_wall_rejects_malformed_input() {
    assert!(serde_json::from_str::<Wall>("12").is_err());
    assert!(serde_json::from_str::<Wall>(r#"{"secs_since_epoch":1}"#).is_err());
    assert!(
        serde_json::from_str::<Wall>(r#"{"secs_since_epoch":"1","nanos_since_epoch":0}"#).is_err()
    );
}

/// Edge-case parity: serde's `SystemTime` carries an out-of-range
/// nanosecond field into the seconds; `Wall` must do exactly the same.
#[test]
fn test_wall_matches_system_time_on_nanosecond_carry() {
    let json = r#"{"secs_since_epoch":1,"nanos_since_epoch":1000000000}"#;
    let wall = serde_json::from_str::<Wall>(json)
        .ok()
        .map(SystemTime::from);
    let time = serde_json::from_str::<SystemTime>(json).ok();
    assert_eq!(wall, time);
}

#[test]
fn test_wall_out_of_platform_range_fails_to_deserialize() {
    // u64::MAX seconds is beyond every platform's SystemTime.
    let json = format!(
        r#"{{"secs_since_epoch":{},"nanos_since_epoch":0}}"#,
        u64::MAX
    );
    assert!(serde_json::from_str::<Wall>(&json).is_err());
}
