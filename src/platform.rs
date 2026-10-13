//! Platform time sources and saturating arithmetic on them.
//!
//! Everything in the crate that names the operating system's time types goes
//! through the re-exports in this module, so the `web-time` feature can swap
//! the source on `wasm32-unknown-unknown` in exactly one place. On every
//! other target, and whenever `web-time` is off, these are the `std::time`
//! types.
//!
//! The saturating helpers exist because `Instant + Duration` and
//! `SystemTime + Duration` panic on overflow, and the representable range of
//! both types is platform-specific (`i64` seconds on Unix, 100 ns ticks since
//! 1601 on Windows). Library code must never panic, so every addition the
//! crate performs on its own goes through `checked_*` and, on the rare
//! overflow, clamps to the furthest point reachable from the starting value.

use core::time::Duration;

#[cfg(not(all(feature = "web-time", target_family = "wasm", target_os = "unknown")))]
pub(crate) use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[cfg(all(feature = "web-time", target_family = "wasm", target_os = "unknown"))]
pub(crate) use web_time::{Instant, SystemTime, UNIX_EPOCH};

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Builds a [`Duration`] from `value` units, where one second is
/// `units_per_sec` units. `units_per_sec` must divide one billion
/// (1, 1 000, 1 000 000 or 1 000 000 000).
///
/// Returns `None` when the whole-second part does not fit in a `u64`, which
/// is the limit of `Duration` itself.
#[inline]
pub(crate) fn duration_from_units(value: u128, units_per_sec: u32) -> Option<Duration> {
    let per_sec = u128::from(units_per_sec);
    let secs = u64::try_from(value / per_sec).ok()?;
    let sub_nanos = (value % per_sec) * (NANOS_PER_SEC / per_sec);
    // `sub_nanos` is below one billion by construction, so this never fails.
    let sub_nanos = u32::try_from(sub_nanos).ok()?;
    Some(Duration::new(secs, sub_nanos))
}

/// Narrows an `i128` to `i64`, saturating at the `i64` bounds.
#[inline]
#[allow(clippy::cast_possible_truncation)] // clamped into the i64 range first, so the cast is exact
pub(crate) fn saturate_i64(value: i128) -> i64 {
    value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// Converts a nanosecond count to a [`Duration`], clamping at
/// [`Duration::MAX`].
#[inline]
pub(crate) fn duration_from_nanos_saturating(nanos: u128) -> Duration {
    duration_from_units(nanos, 1_000_000_000).unwrap_or(Duration::MAX)
}

/// `base + by`, clamped to the furthest instant reachable from `base`.
#[inline]
pub(crate) fn instant_add(base: Instant, by: Duration) -> Instant {
    match base.checked_add(by) {
        Some(sum) => sum,
        None => saturate(base, by, |b, d| b.checked_add(d)),
    }
}

/// `base - by`, clamped to the earliest instant reachable from `base`.
#[inline]
pub(crate) fn instant_sub(base: Instant, by: Duration) -> Instant {
    match base.checked_sub(by) {
        Some(diff) => diff,
        None => saturate(base, by, |b, d| b.checked_sub(d)),
    }
}

/// `base + by`, clamped to the furthest time reachable from `base`.
#[inline]
pub(crate) fn system_add(base: SystemTime, by: Duration) -> SystemTime {
    match base.checked_add(by) {
        Some(sum) => sum,
        None => saturate(base, by, |b, d| b.checked_add(d)),
    }
}

/// `base - by`, clamped to the earliest time reachable from `base`.
#[inline]
pub(crate) fn system_sub(base: SystemTime, by: Duration) -> SystemTime {
    match base.checked_sub(by) {
        Some(diff) => diff,
        None => saturate(base, by, |b, d| b.checked_sub(d)),
    }
}

/// Finds the furthest point reachable from `base` by applying `step` with a
/// duration no larger than `limit`.
///
/// `step` must be monotone in its duration argument: if it succeeds for some
/// `d`, it succeeds for every smaller `d`. That holds for `checked_add` and
/// `checked_sub` on both `Instant` and `SystemTime`. The search is a plain
/// binary search over nanoseconds, so it costs at most ~95 `step` calls. It
/// only runs after the fast `checked_*` call has already failed, which in
/// practice means a test deliberately pushed a clock to the edge of the
/// platform's range.
#[cold]
#[inline(never)]
fn saturate<T: Copy>(base: T, limit: Duration, step: impl Fn(T, Duration) -> Option<T>) -> T {
    let mut best = base;
    let mut lo: u128 = 0;
    let mut hi: u128 = limit.as_nanos();
    // Invariant: `step(base, lo)` succeeds (it is `base` itself at 0) and
    // every value above `hi` fails.
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        match step(base, duration_from_nanos_saturating(mid)) {
            Some(reached) => {
                best = reached;
                lo = mid;
            }
            None => hi = mid - 1,
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_from_units_seconds_millis_nanos_are_exact() {
        assert_eq!(duration_from_units(7, 1), Some(Duration::from_secs(7)));
        assert_eq!(
            duration_from_units(1_234, 1_000),
            Some(Duration::from_millis(1_234))
        );
        assert_eq!(
            duration_from_units(1_234_567, 1_000_000),
            Some(Duration::from_micros(1_234_567))
        );
        assert_eq!(
            duration_from_units(9_876_543_210, 1_000_000_000),
            Some(Duration::from_nanos(9_876_543_210))
        );
    }

    #[test]
    fn test_duration_from_units_beyond_duration_max_returns_none() {
        let too_many_secs = u128::from(u64::MAX) + 1;
        assert_eq!(duration_from_units(too_many_secs, 1), None);
        assert_eq!(
            duration_from_units(Duration::MAX.as_nanos(), 1_000_000_000),
            Some(Duration::MAX)
        );
        assert_eq!(
            duration_from_units(Duration::MAX.as_nanos() + 1, 1_000_000_000),
            None
        );
    }

    #[test]
    fn test_saturate_i64_clamps_both_ends_and_preserves_in_range() {
        assert_eq!(saturate_i64(i128::MAX), i64::MAX);
        assert_eq!(saturate_i64(i128::MIN), i64::MIN);
        assert_eq!(saturate_i64(i128::from(i64::MAX) + 1), i64::MAX);
        assert_eq!(saturate_i64(i128::from(i64::MIN) - 1), i64::MIN);
        assert_eq!(saturate_i64(-42), -42);
        assert_eq!(saturate_i64(42), 42);
    }

    #[test]
    fn test_duration_from_nanos_saturating_clamps_at_duration_max() {
        assert_eq!(duration_from_nanos_saturating(0), Duration::ZERO);
        assert_eq!(duration_from_nanos_saturating(u128::MAX), Duration::MAX);
    }

    #[test]
    fn test_saturate_finds_exact_threshold() {
        // A synthetic "type" that can only move forward 1_000 ns.
        let step = |base: u64, d: Duration| {
            let n = u64::try_from(d.as_nanos()).ok()?;
            let sum = base.checked_add(n)?;
            (sum <= 1_000).then_some(sum)
        };
        assert_eq!(saturate(0_u64, Duration::from_secs(5), step), 1_000);
        assert_eq!(saturate(400_u64, Duration::MAX, step), 1_000);
        assert_eq!(saturate(1_000_u64, Duration::MAX, step), 1_000);
    }

    #[test]
    fn test_saturate_with_zero_limit_returns_base() {
        assert_eq!(saturate(9_u8, Duration::ZERO, |_, _| None), 9);
    }

    #[test]
    fn test_instant_add_overflow_saturates_at_fixed_point() {
        let base = Instant::now();
        let top = instant_add(base, Duration::MAX);
        assert!(top >= base);
        // Saturated: no further forward motion is representable.
        assert_eq!(top.checked_add(Duration::from_nanos(1)), None);
        assert_eq!(instant_add(top, Duration::from_secs(1)), top);
    }

    #[test]
    fn test_instant_sub_overflow_saturates_at_fixed_point() {
        let base = Instant::now();
        let bottom = instant_sub(base, Duration::MAX);
        assert!(bottom <= base);
        assert_eq!(bottom.checked_sub(Duration::from_nanos(1)), None);
        assert_eq!(instant_sub(bottom, Duration::from_secs(1)), bottom);
    }

    #[test]
    fn test_system_add_and_sub_overflow_clamp_monotonically() {
        // `SystemTime` limits differ by platform: on Windows the limit is on
        // the size of the duration in 100 ns ticks, not only on the result,
        // so the clamped value is not necessarily the platform's absolute
        // extreme. What holds everywhere: no panic, and clamping never moves
        // a result backwards relative to a smaller in-range step.
        let century = Duration::from_secs(100 * 365 * 86_400);
        let top = system_add(UNIX_EPOCH, Duration::MAX);
        assert!(top >= system_add(UNIX_EPOCH, century));
        let bottom = system_sub(UNIX_EPOCH, Duration::MAX);
        assert!(bottom <= system_sub(UNIX_EPOCH, century));
        assert!(bottom < UNIX_EPOCH);
    }

    #[test]
    fn test_in_range_arithmetic_is_exact() {
        let base = Instant::now();
        let d = Duration::from_millis(1_500);
        assert_eq!(instant_add(base, d).duration_since(base), d);
        assert_eq!(base.duration_since(instant_sub(base, d)), d);
        assert_eq!(
            system_add(UNIX_EPOCH, d).duration_since(UNIX_EPOCH).ok(),
            Some(d)
        );
    }
}
