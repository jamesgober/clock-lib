//! Reading-latency benchmarks for `clock-lib`.
//!
//! Each group benchmarks the `clock-lib` surface alongside the raw
//! `std::time` call it ultimately performs. The wrapper should add zero
//! measurable overhead; if it doesn't, that's a regression.

use std::time::{Duration, Instant, SystemTime};

use clock_lib::{Clock, ManualClock, Monotonic, SystemClock, Wall};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Monotonic reading latency — `Instant::now` vs every wrapper that calls it.
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

/// Wall-clock reading latency — `SystemTime::now` vs every wrapper.
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
        b.iter(|| black_box(stamp.unix_seconds()));
    });
    group.bench_function("Wall::unix_millis", |b| {
        b.iter(|| black_box(stamp.unix_millis()));
    });
    group.bench_function("Wall::unix_nanos", |b| {
        b.iter(|| black_box(stamp.unix_nanos()));
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

/// Elapsed-time measurement — `Instant::elapsed` vs `Monotonic::elapsed`.
fn bench_elapsed(c: &mut Criterion) {
    let std_start = Instant::now();
    let clock_start = Monotonic::now();
    let mut group = c.benchmark_group("elapsed");
    group.bench_function("std::time::Instant::elapsed", |b| {
        b.iter(|| black_box(std_start.elapsed()));
    });
    group.bench_function("Monotonic::elapsed", |b| {
        b.iter(|| black_box(clock_start.elapsed()));
    });
    group.bench_function("clock_lib::elapsed", |b| {
        b.iter(|| black_box(clock_lib::elapsed(clock_start)));
    });
    group.finish();
}

/// `ManualClock` is the hot path in test suites — keep it cheap.
fn bench_manual_clock(c: &mut Criterion) {
    let clock = ManualClock::new();
    let mut group = c.benchmark_group("manual_clock");
    group.bench_function("now", |b| {
        b.iter(|| black_box(clock.now()));
    });
    group.bench_function("wall", |b| {
        b.iter(|| black_box(clock.wall()));
    });
    group.bench_function("offset", |b| {
        b.iter(|| black_box(clock.offset()));
    });
    group.bench_function("advance_1ns", |b| {
        b.iter(|| clock.advance(black_box(Duration::from_nanos(1))));
    });
    group.finish();
}

/// Trait-object dispatch — `&dyn Clock` must not regress vs concrete.
fn bench_dyn_dispatch(c: &mut Criterion) {
    let concrete = SystemClock::new();
    let dynamic: &dyn Clock = &SystemClock::new();
    let mut group = c.benchmark_group("dyn_dispatch");
    group.bench_function("SystemClock (concrete)", |b| {
        b.iter(|| black_box(concrete.now()));
    });
    group.bench_function("&dyn Clock", |b| {
        b.iter(|| black_box(dynamic.now()));
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
    bench_manual_clock,
    bench_dyn_dispatch,
);
criterion_main!(benches);
