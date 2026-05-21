<h1 align="center">
    <b>clock-lib</b><br>
    <sub><sup>PERFORMANCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./API.md" title="API Reference"><b>API</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./GUIDELINES.md" title="Developer Guidelines"><b>GUIDELINES</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <span>PERFORMANCE</span>
    </sup>
</div>
<br>

> **Status:** Numbers below are the **1.0.0** baseline. The performance-critical paths have not changed since the 0.4.0 measurement run &mdash; the same numbers carry forward. Re-run the suite locally to compare on your hardware.

`clock-lib` is a thin, well-inlined layer over `std::time`. The headline performance claim is that the wrapper is free &mdash; `Monotonic::now()` runs in the same time as `Instant::now()`, and `Wall::now()` in the same time as `SystemTime::now()`. This document records the measurements behind that claim and the methodology used to take them.

## Table of Contents

- [Methodology](#methodology)
- [Results](#results)
  - [Monotonic Reading Latency](#monotonic-reading-latency)
  - [Wall-Clock Reading Latency](#wall-clock-reading-latency)
  - [Unix Conversions](#unix-conversions)
  - [Unix Helpers (capture + convert)](#unix-helpers-capture--convert)
  - [Elapsed-Time Measurement](#elapsed-time-measurement)
  - [`ManualClock` Operations](#manualclock-operations)
  - [Trait-Object Dispatch](#trait-object-dispatch)
- [Running the Benchmarks](#running-the-benchmarks)
- [Interpreting the Numbers](#interpreting-the-numbers)

<br>

## Methodology

- **Harness:** [`criterion`](https://crates.io/crates/criterion) 0.5.
- **Profile:** `release` (`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`), as declared in `Cargo.toml`.
- **Inlining:** Every reading function is `#[inline]`. The wrapper is a thin newtype around the underlying `std::time` type with no extra work on the hot path.
- **Sampling:** Criterion defaults &mdash; 100 samples, automatic warm-up, statistical outlier rejection.
- **Compiler hint suppression:** Hot values are passed through `criterion::black_box` so the compiler cannot fold the bench away.

**Reference platform for the recorded numbers**

| Property | Value |
| --- | --- |
| OS | Windows 11 Pro (build 10.0.26200) |
| Rust toolchain | 1.85.0 (pinned via `rust-toolchain.toml`) |
| Profile | `bench` (release) |

Your numbers will differ &mdash; clock syscall latency is platform-dependent. What is **not** platform-dependent is the relative shape: every wrapper number should sit on top of the raw `std::time` number it forwards to.

<br>

## Results

### Monotonic Reading Latency

How long it takes to capture a monotonic instant from the OS. The clock-lib wrappers all reduce to `Instant::now()` after inlining.

| Bench | Median | Δ vs std |
| --- | --- | --- |
| `std::time::Instant::now` | **21.63 ns** | &mdash; |
| `clock_lib::now` | 21.59 ns | &minus;0.2% |
| `Monotonic::now` | 21.48 ns | &minus;0.7% |
| `SystemClock::now` | 21.43 ns | &minus;0.9% |

**Zero-overhead claim, verified.** All four values sit within criterion's noise band of each other; the wrapper paths are indistinguishable from the raw syscall.

### Wall-Clock Reading Latency

How long it takes to capture a wall-clock instant from the OS. The clock-lib wrappers all reduce to `SystemTime::now()`.

| Bench | Median | Δ vs std |
| --- | --- | --- |
| `std::time::SystemTime::now` | **20.50 ns** | &mdash; |
| `clock_lib::wall` | 20.35 ns | &minus;0.7% |
| `Wall::now` | 20.38 ns | &minus;0.6% |

### Unix Conversions

Cost of converting a pre-captured `Wall` reading to a numeric Unix timestamp. No system call involved &mdash; this is pure arithmetic on the embedded `SystemTime`. Sub-nanosecond on all three.

| Bench | Median |
| --- | --- |
| `Wall::unix_seconds` | **181.6 ps** |
| `Wall::unix_millis` | 364.2 ps |
| `Wall::unix_nanos` | 373.4 ps |

### Unix Helpers (capture + convert)

End-to-end cost of the one-line Tier-1 helpers: capture a wall reading and convert it. The capture dominates &mdash; the conversion adds picoseconds.

| Bench | Median |
| --- | --- |
| `clock_lib::unix` | **21.18 ns** |
| `clock_lib::unix_ms` | 21.86 ns |
| `clock_lib::unix_ns` | 22.21 ns |

### Elapsed-Time Measurement

Cost of computing the elapsed duration since a captured monotonic reading. Includes one `Instant::now()` call plus a subtraction.

| Bench | Median | Δ vs std |
| --- | --- | --- |
| `std::time::Instant::elapsed` | **23.22 ns** | &mdash; |
| `Monotonic::elapsed` | 23.56 ns | +1.5% |
| `clock_lib::elapsed` | 23.18 ns | &minus;0.2% |

### `ManualClock` Operations

`ManualClock` is the hot path in test suites &mdash; thousands of `now()` and `advance()` calls per test are common. Cost is one atomic load plus an `Instant + Duration`. All operations land in **sub-nanosecond to low single-digit nanosecond** territory &mdash; a test that calls `advance` and `now` a million times spends about 4 ms on the clock.

| Bench | Median |
| --- | --- |
| `ManualClock::now` | **752 ps** |
| `ManualClock::wall` | 915 ps |
| `ManualClock::offset` | 596 ps |
| `ManualClock::advance` (1ns) | 3.75 ns |

### Trait-Object Dispatch

Cost of going through `&dyn Clock` instead of a concrete type. One extra vtable lookup per call &mdash; small but measurable.

| Bench | Median | Δ vs concrete |
| --- | --- | --- |
| `SystemClock` (concrete) | **21.40 ns** | &mdash; |
| `&dyn Clock` | 21.91 ns | +2.4% |

<br>

## Running the Benchmarks

```bash
cargo bench --bench clock_bench
```

Criterion writes detailed reports to `target/criterion/`. For a quick text summary, watch the terminal output during the run.

To compare against this baseline:

```bash
# First run — establishes a named baseline.
cargo bench --bench clock_bench -- --save-baseline main

# Later — compares the current code against the saved baseline.
cargo bench --bench clock_bench -- --baseline main
```

A regression exceeding 5% on any tracked metric is treated as a build failure per the [REPS performance gate](../REPS.md#cicd--pre-merge-checklist).

<br>

## Interpreting the Numbers

- **Clock syscall latency dominates.** `Instant::now()` and `SystemTime::now()` on Windows go through `QueryPerformanceCounter` and `GetSystemTimePreciseAsFileTime`. On Linux they reach `clock_gettime`. Either way the syscall is the cost. The clock-lib wrappers add literally nothing on top &mdash; the inlining is complete and the generated assembly is identical.
- **Watch the relative gap, not the absolute number.** If `clock_lib::now` is more than a few percent over `Instant::now`, something has regressed.
- **`ManualClock` is intentionally cheap.** It is the inner loop of test suites. One `AtomicU64::load(Relaxed)` plus `Instant + Duration` is all it does.
- **`&dyn Clock` adds a vtable indirection.** The cost is small but measurable. Use a concrete generic (`fn foo<C: Clock>(c: &C)`) in performance-sensitive code; use `&dyn Clock` when ergonomic flexibility matters more.
