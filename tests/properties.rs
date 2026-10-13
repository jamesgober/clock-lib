//! Property-based tests (proptest) for invariants that must hold for all
//! inputs: `ManualClock` offset arithmetic, Unix accessor consistency,
//! signed rounding, RFC 3339 formatting, and `Monotonic` arithmetic.
#![cfg(feature = "std")]

use std::time::Duration;

use clock_lib::{Clock, Deadline, ManualClock, Monotonic, Stopwatch, Wall};
use proptest::prelude::*;

const NANOS_PER_SEC: i128 = 1_000_000_000;
/// 1601-01-01, the earliest moment Windows `SystemTime` can represent.
const MIN_PORTABLE_SECS: i64 = -11_644_473_600;
/// 9999-12-31T23:59:59Z, the last four-digit-year second.
const MAX_FOUR_DIGIT_SECS: i64 = 253_402_300_799;

/// A mix of small steps and huge ones, so sequences both stay in range and
/// cross `u64::MAX`.
fn advance_step() -> impl Strategy<Value = u64> {
    prop_oneof![0_u64..1_000_000, 0_u64..u64::MAX / 4, any::<u64>()]
}

/// Builds a `Wall` from signed Unix nanoseconds.
fn wall_from_signed_nanos(nanos: i128) -> Option<Wall> {
    let abs = nanos.unsigned_abs();
    let secs = u64::try_from(abs / 1_000_000_000).ok()?;
    let sub = u32::try_from(abs % 1_000_000_000).ok()?;
    let magnitude = Duration::new(secs, sub);
    if nanos >= 0 {
        Wall::UNIX_EPOCH.checked_add(magnitude)
    } else {
        Wall::UNIX_EPOCH.checked_sub(magnitude)
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Deliberately naive reference: walk years and months one at a time.
fn reference_rfc3339(secs: i64, nanos: u32) -> String {
    let mut days = secs.div_euclid(86_400);
    let second_of_day = secs.rem_euclid(86_400);
    let mut year = 1970_i64;
    while days < 0 {
        year -= 1;
        days += if is_leap(year) { 366 } else { 365 };
    }
    loop {
        let len = if is_leap(year) { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    let february = if is_leap(year) { 29 } else { 28 };
    let mut month = 1;
    for len in [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
        if days < len {
            break;
        }
        days -= len;
        month += 1;
    }
    format!(
        "{year:04}-{month:02}-{:02}T{:02}:{:02}:{:02}.{nanos:09}Z",
        days + 1,
        second_of_day / 3_600,
        second_of_day % 3_600 / 60,
        second_of_day % 60
    )
}

proptest! {
    // Failing cases are printed with their minimal input; no regression
    // files are written into the source tree.
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// The offset is the saturating sum of all advances, readings never go
    /// backwards, and `Clock::elapsed` reports exactly the offset.
    #[test]
    fn prop_manual_clock_offset_is_saturating_sum(steps in prop::collection::vec(advance_step(), 0..24)) {
        let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
        let start = clock.now();
        let mut expected: u64 = 0;
        let mut last = start;
        for step in steps {
            clock.advance(Duration::from_nanos(step));
            expected = expected.saturating_add(step);
            let now = clock.now();
            prop_assert!(now >= last);
            last = now;
            prop_assert_eq!(clock.offset(), Duration::from_nanos(expected));
            prop_assert_eq!(clock.elapsed(start), Duration::from_nanos(expected));
        }
    }

    /// Wall readings track the offset to within one platform tick (exactly
    /// on Linux/macOS, 100 ns ticks on Windows) and the error never grows.
    #[test]
    fn prop_manual_clock_wall_tracks_offset_within_one_tick(steps in prop::collection::vec(0_u64..10_000_000_000, 0..24)) {
        let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
        let mut expected: u128 = 0;
        for step in steps {
            clock.advance(Duration::from_nanos(step));
            expected += u128::from(step);
            let actual = clock.wall().unix_nanos();
            prop_assert!(actual <= expected);
            prop_assert!(expected - actual < 100);
        }
    }

    /// `advance_to` behaves as a running maximum and composes with
    /// `advance`.
    #[test]
    fn prop_advance_to_is_running_max(ops in prop::collection::vec((any::<bool>(), 0_u64..1_000_000), 0..32)) {
        let clock = ManualClock::new();
        let mut model: u64 = 0;
        for (absolute, nanos) in ops {
            if absolute {
                clock.advance_to(Duration::from_nanos(nanos));
                model = model.max(nanos);
            } else {
                clock.advance(Duration::from_nanos(nanos));
                model = model.saturating_add(nanos);
            }
            prop_assert_eq!(clock.offset(), Duration::from_nanos(model));
        }
    }

    /// Wall-only steps move the wall reading by their net amount and never
    /// touch the monotonic reading.
    #[test]
    fn prop_wall_steps_move_wall_only(steps in prop::collection::vec((any::<bool>(), 0_u64..1_000_000_000), 0..16)) {
        let start = Wall::from_unix_seconds(1_000_000_000).unwrap_or(Wall::UNIX_EPOCH);
        let clock = ManualClock::with_start(start);
        let mono = clock.now();
        let mut net: i128 = 1_000_000_000 * NANOS_PER_SEC;
        for (forward, micros) in steps {
            let by = Duration::from_micros(micros);
            if forward {
                clock.step_wall_forward(by);
                net += i128::from(micros) * 1_000;
            } else {
                clock.step_wall_back(by);
                net -= i128::from(micros) * 1_000;
            }
            prop_assert_eq!(clock.wall().unix_nanos_signed(), net);
            prop_assert_eq!(clock.now(), mono);
        }
    }

    /// Unsigned and signed accessors agree after the epoch and are
    /// truncations of the nanosecond value.
    #[test]
    fn prop_unix_accessors_are_consistent_after_epoch(hundreds in 0_u64..(1_u64 << 56)) {
        let nanos = u128::from(hundreds) * 100;
        let w = Wall::from_unix_nanos(nanos);
        prop_assert!(w.is_some());
        let w = w.unwrap_or(Wall::UNIX_EPOCH);
        prop_assert_eq!(w.unix_nanos(), nanos);
        prop_assert_eq!(w.unix_millis(), nanos / 1_000_000);
        prop_assert_eq!(u128::from(w.unix_seconds()), nanos / 1_000_000_000);
        prop_assert_eq!(w.unix_nanos_signed(), i128::try_from(nanos).unwrap_or(i128::MAX));
        prop_assert_eq!(w.unix_millis_signed(), i128::try_from(nanos / 1_000_000).unwrap_or(i128::MAX));
        prop_assert_eq!(i128::from(w.unix_seconds_signed()), i128::try_from(nanos / 1_000_000_000).unwrap_or(i128::MAX));
    }

    /// Before the epoch, signed accessors round toward negative infinity
    /// and unsigned accessors saturate at zero.
    #[test]
    fn prop_signed_accessors_floor_before_epoch(hundreds in 1_u64..(1_u64 << 56)) {
        let nanos = -i128::from(hundreds) * 100;
        let w = wall_from_signed_nanos(nanos);
        prop_assert!(w.is_some());
        let w = w.unwrap_or(Wall::UNIX_EPOCH);
        prop_assert_eq!(w.unix_nanos_signed(), nanos);
        prop_assert_eq!(w.unix_millis_signed(), nanos.div_euclid(1_000_000));
        prop_assert_eq!(i128::from(w.unix_seconds_signed()), nanos.div_euclid(NANOS_PER_SEC));
        prop_assert_eq!(w.unix_nanos(), 0);
        prop_assert_eq!(w.unix_seconds(), 0);
    }

    /// RFC 3339 output matches a naive year-by-year reference across the
    /// whole portable four-digit range (1601..=9999).
    #[test]
    fn prop_rfc3339_matches_naive_reference(secs in MIN_PORTABLE_SECS..=MAX_FOUR_DIGIT_SECS, hundreds in 0_u32..10_000_000) {
        let nanos = hundreds * 100;
        let total = i128::from(secs) * NANOS_PER_SEC + i128::from(nanos);
        let w = wall_from_signed_nanos(total);
        prop_assert!(w.is_some());
        let w = w.unwrap_or(Wall::UNIX_EPOCH);
        prop_assert_eq!(format!("{w:.9}"), reference_rfc3339(secs, nanos));
    }

    /// The fractional part has exactly the requested number of digits.
    #[test]
    fn prop_rfc3339_precision_controls_length(secs in 0_u64..253_402_300_799, digits in 0_usize..=9) {
        let w = Wall::from_unix_seconds(secs).unwrap_or(Wall::UNIX_EPOCH);
        let text = format!("{w:.digits$}");
        let expected = 20 + if digits == 0 { 0 } else { digits + 1 };
        prop_assert_eq!(text.len(), expected);
    }

    /// `Monotonic` arithmetic is exact inside the representable range.
    #[test]
    fn prop_monotonic_add_sub_round_trip(nanos in 0_u64..(1_u64 << 60)) {
        let base = Monotonic::now();
        let d = Duration::from_nanos(nanos);
        let later = base + d;
        prop_assert_eq!(base.checked_add(d), Some(later));
        prop_assert_eq!(later - base, d);
        prop_assert_eq!(later - d, base);
        prop_assert_eq!(base - later, Duration::ZERO);
    }

    /// A deadline expires exactly when the clock reaches it, and the
    /// remaining time plus the elapsed time is the TTL until then.
    #[test]
    fn prop_deadline_and_stopwatch_agree_with_model(ttl in 0_u64..1_000_000_000, advances in prop::collection::vec(0_u64..300_000_000, 0..8)) {
        let clock = ManualClock::new();
        let deadline = Deadline::after_with(&clock, Duration::from_nanos(ttl));
        let watch = Stopwatch::start_with(&clock);
        let mut elapsed: u64 = 0;
        for step in advances {
            clock.advance(Duration::from_nanos(step));
            elapsed += step;
            prop_assert_eq!(deadline.is_expired(), elapsed >= ttl);
            prop_assert_eq!(deadline.remaining(), Duration::from_nanos(ttl.saturating_sub(elapsed)));
            prop_assert_eq!(watch.elapsed(), Duration::from_nanos(elapsed));
        }
    }
}
