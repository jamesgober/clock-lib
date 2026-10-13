//! A clock under the test's control.
//!
//! [`ManualClock`] is the deterministic [`Clock`] implementation: it only
//! moves when told to. Every operation is lock-free (a handful of `Relaxed`
//! atomics), so the clock can be shared between a test driver and the code
//! under test across threads.

use core::fmt;
use core::time::Duration;

use crate::clock::Clock;
use crate::platform::{self, Instant, SystemTime};
use crate::sync::{AtomicI64, AtomicU64, Ordering};
use crate::{Monotonic, Wall};

/// A clock under your control, for deterministic testing.
///
/// `ManualClock` takes its anchors at construction and then moves **only**
/// when you call [`advance`](Self::advance) (or one of the other control
/// methods). Nothing happens on its own, so a test can put time-driven code
/// at exact, repeatable instants without `thread::sleep`.
///
/// # What is deterministic
///
/// - **Deltas** are exact: `advance(d)` moves both [`now`](Self::now) and
///   [`wall`](Self::wall) by exactly `d` (wall readings to the platform's
///   resolution, see below).
/// - **Wall readings** are fully reproducible when the clock is built with
///   [`with_start`](Self::with_start): the same test produces the same
///   timestamps on every run. [`ManualClock::new`] anchors the wall reading
///   at the current OS time instead, so its absolute timestamps differ per
///   run.
/// - **Monotonic readings** are opaque by design (as with
///   [`std::time::Instant`]); only differences between them are meaningful.
///
/// # Wall time can move on its own
///
/// [`step_wall_forward`](Self::step_wall_forward),
/// [`step_wall_back`](Self::step_wall_back) and
/// [`set_wall`](Self::set_wall) move the wall reading **without** moving
/// the monotonic reading. That is how a test simulates an NTP step, a
/// manual clock change or a VM resume, which is exactly the situation the
/// `Monotonic`/`Wall` split protects against.
///
/// # Readings from this clock need this clock
///
/// A [`Monotonic`] returned by a `ManualClock` must be measured with the
/// same clock: [`Clock::elapsed`] or `clock.now().duration_since(reading)`.
/// The free function [`elapsed`](crate::elapsed) and
/// [`Monotonic::elapsed`] read the operating system clock and would ignore
/// every `advance`.
///
/// # Sharing
///
/// `ManualClock` is `Send + Sync` and every method takes `&self`. It is
/// deliberately not `Clone`: two copies would drift apart. Share one clock
/// with `&ManualClock` or `Arc<ManualClock>`; both implement [`Clock`].
///
/// # Platform resolution
///
/// On Windows, `SystemTime` counts 100 ns ticks, so wall readings move in
/// 100 ns steps: `advance(Duration::from_nanos(123))` moves `now()` by
/// 123 ns but `wall()` by 100 ns, and advances below 100 ns may not move
/// `wall()` at all. The error never accumulates, because every wall reading
/// is computed from the anchor plus the exact total offset. On Linux and
/// macOS both readings move by exactly the requested amount.
///
/// # Limits
///
/// The shared offset saturates at `u64::MAX` nanoseconds (about 584
/// years). The wall-only adjustment saturates at ±`i64::MAX` nanoseconds
/// (about ±292 years). Readings that would leave the platform's
/// representable range clamp to its edge. No method ever panics or wraps.
///
/// # Examples
///
/// ```
/// use clock_lib::{Clock, ManualClock, Monotonic};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// // A "ttl expired?" check that can be driven without sleeping.
/// fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
///     clock.elapsed(stamp) >= ttl
/// }
///
/// let clock = Arc::new(ManualClock::new());
/// let stamp = clock.now();
///
/// assert!(!expired(&clock, stamp, Duration::from_secs(60)));
/// clock.advance(Duration::from_secs(60));
/// assert!(expired(&clock, stamp, Duration::from_secs(60)));
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct ManualClock {
    monotonic_anchor: Instant,
    wall_anchor: SystemTime,
    /// Offset shared by both readings, in nanoseconds. Never decreases.
    offset_nanos: AtomicU64,
    /// Signed adjustment applied to wall readings only, in nanoseconds.
    wall_skew_nanos: AtomicI64,
    /// When non-zero, every reading advances the offset by this many
    /// nanoseconds after it is taken.
    auto_advance_nanos: AtomicU64,
}

impl ManualClock {
    /// Constructs a manual clock anchored at the current OS time, with a
    /// zero offset. The first reading equals the anchor.
    ///
    /// Wall readings start at "now", so they differ from run to run; use
    /// [`with_start`](Self::with_start) for reproducible timestamps.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    ///
    /// let clock = ManualClock::new();
    /// assert_eq!(clock.now(), clock.now());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::with_start(Wall::now())
    }

    /// Constructs a manual clock whose wall reading starts at `start`.
    ///
    /// With a fixed start, every wall reading the clock produces is the same
    /// on every run, so snapshot or golden tests of timestamps are stable.
    /// The monotonic anchor is still taken from the OS (monotonic readings
    /// are opaque), but deltas between them are exact.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Wall};
    /// use std::time::Duration;
    ///
    /// let start = Wall::from_unix_seconds(1_700_000_000).expect("in range");
    /// let clock = ManualClock::with_start(start);
    /// assert_eq!(clock.wall().to_string(), "2023-11-14T22:13:20Z");
    ///
    /// clock.advance(Duration::from_millis(1_500));
    /// assert_eq!(clock.wall().to_string(), "2023-11-14T22:13:21.500Z");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_start(start: Wall) -> Self {
        Self {
            monotonic_anchor: Instant::now(),
            wall_anchor: start.0,
            offset_nanos: AtomicU64::new(0),
            wall_skew_nanos: AtomicI64::new(0),
            auto_advance_nanos: AtomicU64::new(0),
        }
    }

    /// Returns the current monotonic reading: the anchor plus the offset.
    ///
    /// This is the same as [`Clock::now`], available without importing the
    /// trait. With [auto-advance](Self::set_auto_advance) enabled, the
    /// clock then moves forward by the step.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let a = clock.now();
    /// clock.advance(Duration::from_secs(1));
    /// assert_eq!(clock.now() - a, Duration::from_secs(1));
    /// ```
    #[inline]
    #[must_use]
    pub fn now(&self) -> Monotonic {
        let offset = Duration::from_nanos(self.reading_offset());
        Monotonic(platform::instant_add(self.monotonic_anchor, offset))
    }

    /// Returns the current wall-clock reading: the wall anchor plus the
    /// offset plus any wall-only adjustment.
    ///
    /// This is the same as [`Clock::wall`], available without importing the
    /// trait. With [auto-advance](Self::set_auto_advance) enabled, the
    /// clock then moves forward by the step.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Wall};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
    /// clock.advance(Duration::from_secs(90));
    /// assert_eq!(clock.wall().unix_seconds(), 90);
    /// ```
    #[inline]
    #[must_use]
    pub fn wall(&self) -> Wall {
        let offset = Duration::from_nanos(self.reading_offset());
        match self.wall_skew_nanos.load(Ordering::Relaxed) {
            0 => Wall(platform::system_add(self.wall_anchor, offset)),
            skew => Wall(wall_at(self.wall_anchor, offset, skew)),
        }
    }

    /// Advances both readings forward by `by`.
    ///
    /// Successive calls accumulate. The offset saturates at
    /// [`u64::MAX`] nanoseconds (about 584 years): once saturated, further
    /// advances are no-ops, so the clock can never wrap around and go
    /// backwards. `advance(Duration::MAX)` is a safe way to "expire
    /// everything".
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let a = clock.now();
    /// clock.advance(Duration::from_secs(5));
    /// assert_eq!(clock.now().duration_since(a), Duration::from_secs(5));
    ///
    /// clock.advance(Duration::MAX);
    /// clock.advance(Duration::MAX);
    /// assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
    /// ```
    #[inline]
    pub fn advance(&self, by: Duration) {
        // The previous offset is only needed by auto-advance readings.
        let _ = self.fetch_saturating_add(saturating_nanos_u64(by));
    }

    /// Moves the offset forward to `offset` if it is currently smaller.
    ///
    /// This is the absolute counterpart to [`advance`](Self::advance):
    /// "make it 10 seconds after the start" rather than "add 10 seconds".
    /// If the clock is already at or past `offset`, nothing changes. The
    /// monotonic reading can therefore never go backwards.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// clock.advance_to(Duration::from_secs(10));
    /// assert_eq!(clock.offset(), Duration::from_secs(10));
    ///
    /// clock.advance_to(Duration::from_secs(4)); // already past: no-op
    /// assert_eq!(clock.offset(), Duration::from_secs(10));
    /// ```
    #[inline]
    pub fn advance_to(&self, offset: Duration) {
        // `fetch_max` returns the previous value, which is not needed.
        let _ = self
            .offset_nanos
            .fetch_max(saturating_nanos_u64(offset), Ordering::Relaxed);
    }

    /// Returns the total offset added to this clock since it was
    /// constructed (by `advance`, `advance_to` and auto-advance).
    ///
    /// Wall-only adjustments are not included. Reading the offset does not
    /// trigger auto-advance.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// clock.advance(Duration::from_secs(1));
    /// clock.advance(Duration::from_secs(2));
    /// assert_eq!(clock.offset(), Duration::from_secs(3));
    /// ```
    #[inline]
    #[must_use]
    pub fn offset(&self) -> Duration {
        Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed))
    }

    /// Moves the wall reading forward by `by` without moving the monotonic
    /// reading, as an NTP step or a manual clock change would.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Wall};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
    /// let before = clock.now();
    /// clock.step_wall_forward(Duration::from_secs(30));
    /// assert_eq!(clock.wall().unix_seconds(), 30);
    /// assert_eq!(clock.now(), before); // monotonic time did not move
    /// ```
    #[inline]
    pub fn step_wall_forward(&self, by: Duration) {
        let by = saturating_nanos_i64(by);
        self.update_wall_skew(|skew| skew.saturating_add(by));
    }

    /// Moves the wall reading back by `by` without moving the monotonic
    /// reading. Code that measured an interval with wall time will see it
    /// go negative; code that used [`Monotonic`] will not notice.
    ///
    /// The wall reading may move before the Unix epoch; the signed
    /// accessors such as [`Wall::unix_seconds_signed`] report it.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{Clock, ManualClock, Wall};
    /// use std::time::Duration;
    ///
    /// let start = Wall::from_unix_seconds(1_000).expect("in range");
    /// let clock = ManualClock::with_start(start);
    /// let began = clock.now();
    ///
    /// clock.advance(Duration::from_secs(5));
    /// clock.step_wall_back(Duration::from_secs(60)); // NTP correction
    ///
    /// assert_eq!(clock.wall().unix_seconds(), 945);
    /// assert_eq!(clock.elapsed(began), Duration::from_secs(5));
    /// ```
    #[inline]
    pub fn step_wall_back(&self, by: Duration) {
        let by = saturating_nanos_i64(by);
        self.update_wall_skew(|skew| skew.saturating_sub(by));
    }

    /// Sets the wall reading to `to` without moving the monotonic reading.
    ///
    /// Later [`advance`](Self::advance) calls move the wall reading on from
    /// `to`. The adjustment is computed against the offset at the time of
    /// the call, so call it from the test driver, not concurrently with
    /// other threads advancing the same clock. If `to` is more than about
    /// 292 years away from the current wall reading, the adjustment
    /// saturates.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::{ManualClock, Wall};
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// let target = Wall::from_unix_seconds(1_800_000_000).expect("in range");
    /// clock.set_wall(target);
    /// assert_eq!(clock.wall(), target);
    ///
    /// clock.advance(Duration::from_secs(1));
    /// assert_eq!(clock.wall().unix_seconds(), 1_800_000_001);
    /// ```
    pub fn set_wall(&self, to: Wall) {
        let from_anchor = match to.0.duration_since(self.wall_anchor) {
            Ok(ahead) => saturating_i128(ahead.as_nanos()),
            Err(behind) => -saturating_i128(behind.duration().as_nanos()),
        };
        let offset = i128::from(self.offset_nanos.load(Ordering::Relaxed));
        let skew = platform::saturate_i64(from_anchor - offset);
        self.wall_skew_nanos.store(skew, Ordering::Relaxed);
    }

    /// Makes every reading advance the clock by `step` after it is taken.
    ///
    /// With a non-zero step, each call to [`now`](Self::now),
    /// [`wall`](Self::wall) or [`Clock::elapsed`] returns the current
    /// reading and then moves the shared offset forward by `step`, so
    /// successive readings are `step` apart. Use it to test code that polls
    /// the clock in a loop (spin-until-deadline, self-timing operations)
    /// without a second thread driving time. `Duration::ZERO` turns it off.
    ///
    /// Each reading claims its own step atomically, so readings taken
    /// concurrently from several threads are all distinct.
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// clock.set_auto_advance(Duration::from_millis(10));
    ///
    /// let a = clock.now();
    /// let b = clock.now();
    /// assert_eq!(b - a, Duration::from_millis(10));
    ///
    /// clock.set_auto_advance(Duration::ZERO);
    /// assert_eq!(clock.now(), clock.now());
    /// ```
    #[inline]
    pub fn set_auto_advance(&self, step: Duration) {
        self.auto_advance_nanos
            .store(saturating_nanos_u64(step), Ordering::Relaxed);
    }

    /// Returns the current auto-advance step; `Duration::ZERO` when
    /// auto-advance is off (the default).
    ///
    /// # Examples
    ///
    /// ```
    /// use clock_lib::ManualClock;
    /// use std::time::Duration;
    ///
    /// let clock = ManualClock::new();
    /// assert_eq!(clock.auto_advance(), Duration::ZERO);
    /// clock.set_auto_advance(Duration::from_micros(5));
    /// assert_eq!(clock.auto_advance(), Duration::from_micros(5));
    /// ```
    #[inline]
    #[must_use]
    pub fn auto_advance(&self) -> Duration {
        Duration::from_nanos(self.auto_advance_nanos.load(Ordering::Relaxed))
    }

    /// The offset to use for one reading. With auto-advance off this is a
    /// single atomic load; with it on, the reading claims the current value
    /// and moves the shared offset forward by the step.
    #[inline]
    fn reading_offset(&self) -> u64 {
        match self.auto_advance_nanos.load(Ordering::Relaxed) {
            0 => self.offset_nanos.load(Ordering::Relaxed),
            step => self.fetch_saturating_add(step),
        }
    }

    /// Atomically replaces the offset with `offset.saturating_add(nanos)`
    /// and returns the previous offset.
    ///
    /// A compare-and-swap loop rather than `fetch_add`, because
    /// `fetch_add` wraps on overflow and a wrapped offset would move the
    /// clock backwards. When the addition would not change the value (zero
    /// advance, or already saturated) no write is issued at all.
    #[inline]
    fn fetch_saturating_add(&self, nanos: u64) -> u64 {
        let mut current = self.offset_nanos.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_add(nanos);
            if next == current {
                return current;
            }
            match self.offset_nanos.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(previous) => return previous,
                Err(actual) => current = actual,
            }
        }
    }

    /// Atomically applies `change` to the wall-only adjustment.
    fn update_wall_skew(&self, change: impl Fn(i64) -> i64) {
        let mut current = self.wall_skew_nanos.load(Ordering::Relaxed);
        loop {
            match self.wall_skew_nanos.compare_exchange_weak(
                current,
                change(current),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(actual) => current = actual,
            }
        }
    }
}

impl Default for ManualClock {
    /// Same as [`ManualClock::new`].
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ManualClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManualClock")
            .field("monotonic_anchor", &self.monotonic_anchor)
            .field("wall_anchor", &self.wall_anchor)
            .field("offset_nanos", &self.offset_nanos.load(Ordering::Relaxed))
            .field(
                "wall_skew_nanos",
                &self.wall_skew_nanos.load(Ordering::Relaxed),
            )
            .field(
                "auto_advance_nanos",
                &self.auto_advance_nanos.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl Clock for ManualClock {
    /// Same as [`ManualClock::now`].
    #[inline]
    fn now(&self) -> Monotonic {
        Self::now(self)
    }

    /// Same as [`ManualClock::wall`].
    #[inline]
    fn wall(&self) -> Wall {
        Self::wall(self)
    }
}

/// `anchor + forward + skew` without overflow, panics or 128-bit division.
/// Kept out of line: it only runs while a wall-only adjustment is active,
/// and inlining it into `wall()` measurably slows the common path.
#[inline(never)]
fn wall_at(anchor: SystemTime, forward: Duration, skew: i64) -> SystemTime {
    let adjust = Duration::from_nanos(skew.unsigned_abs());
    if skew >= 0 {
        platform::system_add(anchor, forward.saturating_add(adjust))
    } else {
        match forward.checked_sub(adjust) {
            Some(net) => platform::system_add(anchor, net),
            None => platform::system_sub(anchor, adjust.saturating_sub(forward)),
        }
    }
}

#[inline]
fn saturating_nanos_u64(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

#[inline]
fn saturating_nanos_i64(d: Duration) -> i64 {
    i64::try_from(d.as_nanos()).unwrap_or(i64::MAX)
}

#[inline]
fn saturating_i128(v: u128) -> i128 {
    i128::try_from(v).unwrap_or(i128::MAX)
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::platform::UNIX_EPOCH;

    #[test]
    fn test_fetch_saturating_add_returns_previous_and_saturates() {
        let clock = ManualClock::new();
        assert_eq!(clock.fetch_saturating_add(5), 0);
        assert_eq!(clock.fetch_saturating_add(u64::MAX), 5);
        assert_eq!(clock.fetch_saturating_add(1), u64::MAX);
        assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
    }

    #[test]
    fn test_wall_at_combines_offset_and_skew() {
        let at = |offset, skew| {
            wall_at(UNIX_EPOCH, Duration::from_nanos(offset), skew)
                .duration_since(UNIX_EPOCH)
                .map_or(-1, |d| i128::try_from(d.as_nanos()).unwrap_or(-1))
        };
        assert_eq!(at(1_000, 0), 1_000);
        assert_eq!(at(1_000, 500), 1_500);
        assert_eq!(at(1_000, -400), 600);
        assert_eq!(at(1_000, -1_000), 0);
    }

    #[test]
    fn test_wall_at_negative_net_goes_before_anchor() {
        let w = wall_at(UNIX_EPOCH, Duration::from_secs(1), -3_000_000_000);
        assert_eq!(
            UNIX_EPOCH.duration_since(w).ok(),
            Some(Duration::from_secs(2))
        );
    }

    #[test]
    fn test_wall_at_extremes_do_not_panic() {
        let _ = wall_at(UNIX_EPOCH, Duration::from_nanos(u64::MAX), i64::MAX);
        let _ = wall_at(UNIX_EPOCH, Duration::ZERO, i64::MIN);
        let _ = wall_at(UNIX_EPOCH, Duration::from_nanos(u64::MAX), i64::MIN);
    }

    #[test]
    fn test_set_wall_saturates_skew_far_from_anchor() {
        let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
        let far = Wall::UNIX_EPOCH.checked_add(Duration::from_secs(400 * 366 * 86_400));
        if let Some(far) = far {
            clock.set_wall(far);
            assert_eq!(clock.wall_skew_nanos.load(Ordering::Relaxed), i64::MAX);
        }
        clock.set_wall(Wall::UNIX_EPOCH);
        assert_eq!(clock.wall_skew_nanos.load(Ordering::Relaxed), 0);

        let late = Wall::UNIX_EPOCH
            .checked_add(Duration::from_secs(400 * 366 * 86_400))
            .unwrap_or(Wall::UNIX_EPOCH);
        let clock = ManualClock::with_start(late);
        clock.set_wall(Wall::UNIX_EPOCH);
        assert_eq!(clock.wall_skew_nanos.load(Ordering::Relaxed), i64::MIN);
    }

    #[test]
    fn test_saturating_conversions() {
        assert_eq!(saturating_nanos_u64(Duration::MAX), u64::MAX);
        assert_eq!(saturating_nanos_u64(Duration::from_nanos(7)), 7);
        assert_eq!(saturating_nanos_i64(Duration::MAX), i64::MAX);
        assert_eq!(saturating_nanos_i64(Duration::from_nanos(7)), 7);
        assert_eq!(saturating_i128(u128::MAX), i128::MAX);
        assert_eq!(saturating_i128(7), 7);
    }

    #[test]
    fn test_debug_reports_control_state() {
        let clock = ManualClock::with_start(Wall::UNIX_EPOCH);
        clock.advance(Duration::from_secs(2));
        clock.step_wall_back(Duration::from_nanos(5));
        let text = format!("{clock:?}");
        assert!(text.starts_with("ManualClock {"));
        assert!(text.contains("offset_nanos: 2000000000"));
        assert!(text.contains("wall_skew_nanos: -5"));
        assert!(text.contains("auto_advance_nanos: 0"));
    }
}

/// Model-checked concurrency tests. Run with
/// `RUSTFLAGS="--cfg loom" cargo test --lib --release`.
#[cfg(all(test, loom))]
mod loom_tests {
    use super::*;
    use loom::sync::Arc;
    use loom::thread;

    fn spawn_with(clock: &Arc<ManualClock>, f: fn(&ManualClock)) -> thread::JoinHandle<()> {
        let clock = Arc::clone(clock);
        thread::spawn(move || f(&clock))
    }

    #[test]
    fn loom_concurrent_advances_sum_exactly_and_never_regress() {
        loom::model(|| {
            let clock = Arc::new(ManualClock::new());
            let start = clock.now();
            let a = spawn_with(&clock, |c| c.advance(Duration::from_nanos(300)));
            let b = spawn_with(&clock, |c| c.advance(Duration::from_nanos(500)));
            let first = clock.now();
            let second = clock.now();
            assert!(first >= start);
            assert!(second >= first);
            assert!(a.join().is_ok());
            assert!(b.join().is_ok());
            assert_eq!(clock.offset(), Duration::from_nanos(800));
        });
    }

    #[test]
    fn loom_concurrent_overflowing_advances_saturate() {
        loom::model(|| {
            let clock = Arc::new(ManualClock::new());
            let a = spawn_with(&clock, |c| c.advance(Duration::from_nanos(u64::MAX - 1)));
            let b = spawn_with(&clock, |c| c.advance(Duration::from_nanos(10)));
            assert!(a.join().is_ok());
            assert!(b.join().is_ok());
            assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
        });
    }

    #[test]
    fn loom_advance_to_and_advance_compose_in_either_order() {
        loom::model(|| {
            let clock = Arc::new(ManualClock::new());
            let a = spawn_with(&clock, |c| c.advance_to(Duration::from_nanos(1_000)));
            let b = spawn_with(&clock, |c| c.advance(Duration::from_nanos(300)));
            assert!(a.join().is_ok());
            assert!(b.join().is_ok());
            let offset = clock.offset();
            assert!(
                offset == Duration::from_nanos(1_000) || offset == Duration::from_nanos(1_300),
                "unexpected offset {offset:?}"
            );
        });
    }

    #[test]
    fn loom_auto_advance_readings_are_distinct() {
        loom::model(|| {
            let clock = Arc::new(ManualClock::new());
            clock.set_auto_advance(Duration::from_nanos(100));
            let start = clock.offset();
            let other = {
                let clock = Arc::clone(&clock);
                thread::spawn(move || clock.now())
            };
            let mine = clock.now();
            let theirs = other.join();
            assert!(theirs.is_ok_and(|theirs| theirs != mine));
            assert_eq!(clock.offset() - start, Duration::from_nanos(200));
        });
    }

    #[test]
    fn loom_wall_steps_and_advances_commute() {
        loom::model(|| {
            let clock = Arc::new(ManualClock::with_start(Wall::UNIX_EPOCH));
            let a = spawn_with(&clock, |c| c.step_wall_back(Duration::from_nanos(500)));
            let b = spawn_with(&clock, |c| c.advance(Duration::from_nanos(700)));
            let c = spawn_with(&clock, |c| c.step_wall_forward(Duration::from_nanos(100)));
            assert!(a.join().is_ok());
            assert!(b.join().is_ok());
            assert!(c.join().is_ok());
            assert_eq!(clock.wall().unix_nanos_signed(), 300);
        });
    }
}
