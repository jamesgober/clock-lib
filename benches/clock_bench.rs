//! Reading-latency benchmarks for `clock-lib`.
//!
//! Each group benchmarks the `clock-lib` surface alongside the raw
//! `std::time` call it ultimately performs. The wrappers should add no
//! measurable overhead; if one does, that is a regression.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::{Duration, Instant, SystemTime};

use clock_lib::{Clock, Deadline, ManualClock, Monotonic, Stopwatch, SystemClock, Wall};
use criterion::{Criterion, criterion_group, criterion_main};

/// Monotonic reading latency: `Instant::now` vs every wrapper that calls it.
fn bench_monotonic_now(c: &mut Criterion) {
    let mut group = c.benchmark_group("monotonic_now");
    group.bench_function("std::time::Instant::now", |b| {
        b.iter(|| black_box(Instant::now()));
    });
    group.bench_function("clock_lib::now", |b| {
        b.iter(|| black_box(clock_lib::now()));
    });
    group.bench_function("Monotonic::now", |b| {
        b.iter(|| black_box(Monotonic::now()));
    });
    group.bench_function("SystemClock::now", |b| {
        let clock = SystemClock::new();
        b.iter(|| black_box(clock.now()));
    });
    group.finish();
}

/// Wall-clock reading latency: `SystemTime::now` vs every wrapper.
fn bench_wall_now(c: &mut Criterion) {
    let mut group = c.benchmark_group("wall_now");
    group.bench_function("std::time::SystemTime::now", |b| {
        b.iter(|| black_box(SystemTime::now()));
    });
    group.bench_function("clock_lib::wall", |b| {
        b.iter(|| black_box(clock_lib::wall()));
    });
    group.bench_function("Wall::now", |b| {
        b.iter(|| black_box(Wall::now()));
    });
    group.finish();
}

/// Unix-time conversion cost on a pre-captured `Wall` reading.
fn bench_unix_conversions(c: &mut Criterion) {
    let stamp = Wall::now();
    let mut group = c.benchmark_group("unix_conversions");
    group.bench_function("Wall::unix_seconds", |b| {
        b.iter(|| black_box(black_box(stamp).unix_seconds()));
    });
    group.bench_function("Wall::unix_millis", |b| {
        b.iter(|| black_box(black_box(stamp).unix_millis()));
    });
    group.bench_function("Wall::unix_nanos", |b| {
        b.iter(|| black_box(black_box(stamp).unix_nanos()));
    });
    group.bench_function("Wall::unix_seconds_signed", |b| {
        b.iter(|| black_box(black_box(stamp).unix_seconds_signed()));
    });
    group.bench_function("Wall::unix_millis_signed", |b| {
        b.iter(|| black_box(black_box(stamp).unix_millis_signed()));
    });
    group.bench_function("Wall::unix_nanos_signed", |b| {
        b.iter(|| black_box(black_box(stamp).unix_nanos_signed()));
    });
    group.finish();
}

/// Unix-time end-to-end (capture + convert).
fn bench_unix_helpers(c: &mut Criterion) {
    let mut group = c.benchmark_group("unix_helpers");
    group.bench_function("clock_lib::unix", |b| {
        b.iter(|| black_box(clock_lib::unix()));
    });
    group.bench_function("clock_lib::unix_ms", |b| {
        b.iter(|| black_box(clock_lib::unix_ms()));
    });
    group.bench_function("clock_lib::unix_ns", |b| {
        b.iter(|| black_box(clock_lib::unix_ns()));
    });
    group.finish();
}

/// Elapsed-time measurement: `Instant::elapsed` vs the wrappers.
fn bench_elapsed(c: &mut Criterion) {
    let std_start = Instant::now();
    let clock_start = Monotonic::now();
    let clock = SystemClock::new();
    let mut group = c.benchmark_group("elapsed");
    group.bench_function("std::time::Instant::elapsed", |b| {
        b.iter(|| black_box(black_box(std_start).elapsed()));
    });
    group.bench_function("Monotonic::elapsed", |b| {
        b.iter(|| black_box(black_box(clock_start).elapsed()));
    });
    group.bench_function("clock_lib::elapsed", |b| {
        b.iter(|| black_box(clock_lib::elapsed(black_box(clock_start))));
    });
    group.bench_function("Clock::elapsed (SystemClock)", |b| {
        b.iter(|| black_box(clock.elapsed(black_box(clock_start))));
    });
    group.finish();
}

/// Deadline arithmetic: `Instant + Duration` vs `Monotonic` operations.
fn bench_monotonic_arith(c: &mut Criterion) {
    let instant = Instant::now();
    let reading = Monotonic::now();
    let later_instant = instant + Duration::from_secs(1);
    let later_reading = reading + Duration::from_secs(1);
    let ttl = Duration::from_millis(250);
    let mut group = c.benchmark_group("monotonic_arith");
    group.bench_function("std Instant + Duration", |b| {
        b.iter(|| black_box(black_box(instant) + black_box(ttl)));
    });
    group.bench_function("Monotonic + Duration", |b| {
        b.iter(|| black_box(black_box(reading) + black_box(ttl)));
    });
    group.bench_function("Monotonic::checked_add", |b| {
        b.iter(|| black_box(black_box(reading).checked_add(black_box(ttl))));
    });
    group.bench_function("Monotonic + Duration::MAX (saturating)", |b| {
        b.iter(|| black_box(black_box(reading) + black_box(Duration::MAX)));
    });
    group.bench_function("std Instant - Instant", |b| {
        b.iter(|| black_box(black_box(later_instant) - black_box(instant)));
    });
    group.bench_function("Monotonic - Monotonic", |b| {
        b.iter(|| black_box(black_box(later_reading) - black_box(reading)));
    });
    group.finish();
}

/// `ManualClock` is the hot path in test suites; keep it cheap.
fn bench_manual_clock(c: &mut Criterion) {
    let clock = ManualClock::new();
    let stepped = ManualClock::new();
    stepped.step_wall_back(Duration::from_secs(3));
    let auto = ManualClock::new();
    auto.set_auto_advance(Duration::from_nanos(1));
    let start = clock.now();
    let mut group = c.benchmark_group("manual_clock");
    group.bench_function("now", |b| {
        b.iter(|| black_box(black_box(&clock).now()));
    });
    group.bench_function("wall", |b| {
        b.iter(|| black_box(black_box(&clock).wall()));
    });
    group.bench_function("offset", |b| {
        b.iter(|| black_box(black_box(&clock).offset()));
    });
    group.bench_function("advance_1ns", |b| {
        b.iter(|| clock.advance(black_box(Duration::from_nanos(1))));
    });
    group.bench_function("advance_to", |b| {
        b.iter(|| clock.advance_to(black_box(Duration::from_nanos(1))));
    });
    group.bench_function("Clock::elapsed", |b| {
        b.iter(|| black_box(black_box(&clock).elapsed(black_box(start))));
    });
    group.bench_function("wall (stepped)", |b| {
        b.iter(|| black_box(black_box(&stepped).wall()));
    });
    group.bench_function("now (auto-advance)", |b| {
        b.iter(|| black_box(black_box(&auto).now()));
    });
    group.finish();
}

/// `Deadline` and `Stopwatch` on the system clock vs the raw std pattern.
fn bench_helpers(c: &mut Criterion) {
    let std_deadline = Instant::now() + Duration::from_secs(60);
    let deadline = Deadline::after(Duration::from_secs(60));
    let watch = Stopwatch::start();
    let mut group = c.benchmark_group("helpers");
    group.bench_function("std Instant::now() >= deadline", |b| {
        b.iter(|| black_box(Instant::now() >= black_box(std_deadline)));
    });
    group.bench_function("Deadline::is_expired", |b| {
        b.iter(|| black_box(black_box(&deadline).is_expired()));
    });
    group.bench_function("Deadline::remaining", |b| {
        b.iter(|| black_box(black_box(&deadline).remaining()));
    });
    group.bench_function("Deadline::after(Duration::MAX)", |b| {
        b.iter(|| black_box(Deadline::after(black_box(Duration::MAX))));
    });
    group.bench_function("Stopwatch::elapsed", |b| {
        b.iter(|| black_box(black_box(&watch).elapsed()));
    });
    group.finish();
}

/// RFC 3339 formatting into a reused buffer (no allocation per call).
fn bench_wall_format(c: &mut Criterion) {
    let stamp = Wall::from_unix_nanos(1_760_000_000_123_456_700).unwrap_or(Wall::UNIX_EPOCH);
    let mut out = String::with_capacity(64);
    let mut group = c.benchmark_group("wall_format");
    group.bench_function("Display (auto precision)", |b| {
        b.iter(|| {
            out.clear();
            let _ = write!(out, "{}", black_box(stamp));
            black_box(out.len())
        });
    });
    group.bench_function("Display {:.3}", |b| {
        b.iter(|| {
            out.clear();
            let _ = write!(out, "{:.3}", black_box(stamp));
            black_box(out.len())
        });
    });
    group.finish();
}

/// Trait-object dispatch: `&dyn Clock` and `Box<dyn Clock>` vs concrete.
fn bench_dyn_dispatch(c: &mut Criterion) {
    let concrete = SystemClock::new();
    let dynamic: &dyn Clock = &SystemClock::new();
    let boxed: Box<dyn Clock> = Box::new(SystemClock::new());
    let mut group = c.benchmark_group("dyn_dispatch");
    group.bench_function("SystemClock (concrete)", |b| {
        b.iter(|| black_box(concrete.now()));
    });
    group.bench_function("&dyn Clock", |b| {
        b.iter(|| black_box(dynamic.now()));
    });
    group.bench_function("Box<dyn Clock>", |b| {
        b.iter(|| black_box(boxed.now()));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_monotonic_now,
    bench_wall_now,
    bench_unix_conversions,
    bench_unix_helpers,
    bench_elapsed,
    bench_monotonic_arith,
    bench_manual_clock,
    bench_helpers,
    bench_wall_format,
    bench_dyn_dispatch,
);
criterion_main!(benches);
