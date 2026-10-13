//! Multi-threaded tests for `ManualClock`. The loom models in
//! `src/manual.rs` cover every interleaving of small cases; these tests run
//! the real atomics under heavy contention.
#![cfg(feature = "std")]

use std::collections::HashSet;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use clock_lib::{Clock, ManualClock, Monotonic};

const WRITERS: usize = 8;
const READERS: usize = 4;
const ADVANCES_PER_WRITER: u64 = 50_000;
const READS_PER_READER: usize = 100_000;

/// Audit repro r06, as a regression test: concurrent advances sum exactly
/// and no reader ever sees time go backwards.
#[test]
fn test_concurrent_advance_sums_exactly_and_readers_never_regress() {
    let clock = Arc::new(ManualClock::new());
    let start = clock.now();
    let barrier = Arc::new(Barrier::new(WRITERS + READERS));

    let mut handles = Vec::new();
    for _ in 0..WRITERS {
        let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
        handles.push(thread::spawn(move || {
            let _ = barrier.wait();
            for _ in 0..ADVANCES_PER_WRITER {
                clock.advance(Duration::from_nanos(1));
            }
            true
        }));
    }
    for _ in 0..READERS {
        let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
        handles.push(thread::spawn(move || {
            let _ = barrier.wait();
            let mut last = clock.now();
            for _ in 0..READS_PER_READER {
                let next = clock.now();
                if next < last {
                    return false;
                }
                last = next;
            }
            true
        }));
    }
    for handle in handles {
        assert!(
            matches!(handle.join(), Ok(true)),
            "a thread saw a regression"
        );
    }
    let total = ADVANCES_PER_WRITER * WRITERS as u64;
    assert_eq!(clock.elapsed(start), Duration::from_nanos(total));
}

/// CLK-001 under contention: many threads pushing past `u64::MAX` must
/// leave the offset saturated, never wrapped.
#[test]
fn test_concurrent_overflowing_advances_saturate() {
    let clock = Arc::new(ManualClock::new());
    let barrier = Arc::new(Barrier::new(WRITERS));
    let handles: Vec<_> = (0..WRITERS)
        .map(|_| {
            let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
            thread::spawn(move || {
                let _ = barrier.wait();
                for _ in 0..1_000 {
                    clock.advance(Duration::from_nanos(u64::MAX / 512));
                }
            })
        })
        .collect();
    for handle in handles {
        assert!(handle.join().is_ok());
    }
    assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
}

/// Every auto-advanced reading claims its own step, so readings from all
/// threads are distinct and the final offset counts every reading.
#[test]
fn test_concurrent_auto_advance_readings_are_unique() {
    const THREADS: usize = 6;
    const READS: usize = 5_000;

    let clock = Arc::new(ManualClock::new());
    clock.set_auto_advance(Duration::from_nanos(1));
    let barrier = Arc::new(Barrier::new(THREADS));
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
            thread::spawn(move || {
                let _ = barrier.wait();
                (0..READS).map(|_| clock.now()).collect::<Vec<Monotonic>>()
            })
        })
        .collect();

    let mut seen = HashSet::new();
    for handle in handles {
        let readings = handle.join().unwrap_or_default();
        assert_eq!(readings.len(), READS);
        assert!(readings.windows(2).all(|w| w[1] > w[0]));
        seen.extend(readings);
    }
    assert_eq!(seen.len(), THREADS * READS);
    assert_eq!(
        clock.offset(),
        Duration::from_nanos((THREADS * READS) as u64)
    );
}

/// `advance_to` racing `advance`: the result is always one of the two
/// legal serialisations, and never below the `advance_to` target.
#[test]
fn test_concurrent_advance_to_and_advance_compose() {
    for _ in 0..200 {
        let clock = Arc::new(ManualClock::new());
        let barrier = Arc::new(Barrier::new(2));
        let a = {
            let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
            thread::spawn(move || {
                let _ = barrier.wait();
                clock.advance_to(Duration::from_nanos(1_000));
            })
        };
        let b = {
            let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
            thread::spawn(move || {
                let _ = barrier.wait();
                clock.advance(Duration::from_nanos(300));
            })
        };
        assert!(a.join().is_ok());
        assert!(b.join().is_ok());
        let offset = clock.offset().as_nanos();
        assert!(offset == 1_000 || offset == 1_300, "offset {offset}");
    }
}

/// Wall-only steps from several threads commute with each other and with
/// advances.
#[test]
fn test_concurrent_wall_steps_and_advances_commute() {
    let clock = Arc::new(ManualClock::with_start(clock_lib::Wall::UNIX_EPOCH));
    let barrier = Arc::new(Barrier::new(3));
    let spawn = |f: fn(&ManualClock)| {
        let (clock, barrier) = (Arc::clone(&clock), Arc::clone(&barrier));
        thread::spawn(move || {
            let _ = barrier.wait();
            for _ in 0..10_000 {
                f(&clock);
            }
        })
    };
    let handles = [
        spawn(|c| c.step_wall_forward(Duration::from_micros(3))),
        spawn(|c| c.step_wall_back(Duration::from_micros(1))),
        spawn(|c| c.advance(Duration::from_micros(2))),
    ];
    for handle in handles {
        assert!(handle.join().is_ok());
    }
    // 10_000 * (3 - 1 + 2) µs = 40 ms of wall movement, 20 ms monotonic.
    assert_eq!(clock.wall().unix_millis(), 40);
    assert_eq!(clock.offset(), Duration::from_millis(20));
}

/// Cross-thread visibility: an advance made in one thread is observed by
/// another thread after `join`.
#[test]
fn test_advance_in_one_thread_is_visible_after_join() {
    let clock = Arc::new(ManualClock::new());
    let start = clock.now();
    let writer = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || clock.advance(Duration::from_secs(7)))
    };
    assert!(writer.join().is_ok());
    let reader = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || clock.elapsed(start))
    };
    assert_eq!(reader.join().ok(), Some(Duration::from_secs(7)));
}
