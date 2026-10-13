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

> **Status:** Numbers below are the **1.1.0** measurements, taken on 2026-10-13 together with a fresh **1.0.0** baseline on the same machine, toolchain and harness (see [Methodology](#methodology)). Re-run the suite locally to compare on your hardware.

`clock-lib` is a thin, inlined layer over `std::time`. The headline claim is that the wrappers cost nothing measurable: `Monotonic::now()` runs in the time of `Instant::now()`, and `Wall::now()` in the time of `SystemTime::now()`. This document records the measurements behind that claim, the cost of the test-only `ManualClock`, and the methodology.

## Table of Contents

- [Methodology](#methodology)
- [Results](#results)
  - [Monotonic Reading Latency](#monotonic-reading-latency)
  - [Wall-Clock Reading Latency](#wall-clock-reading-latency)
  - [Elapsed-Time Measurement](#elapsed-time-measurement)
  - [Deadline Arithmetic](#deadline-arithmetic)
  - [Deadline and Stopwatch](#deadline-and-stopwatch)
  - [Unix Conversions](#unix-conversions)
  - [Unix Helpers (capture + convert)](#unix-helpers-capture--convert)
  - [RFC 3339 Formatting](#rfc-3339-formatting)
  - [`ManualClock` Operations](#manualclock-operations)
  - [Trait-Object Dispatch](#trait-object-dispatch)
- [Running the Benchmarks](#running-the-benchmarks)
- [Interpreting the Numbers](#interpreting-the-numbers)

<br>

## Methodology

- **Harness:** [`criterion`](https://crates.io/crates/criterion) 0.7, default features off (text output only).
- **Profile:** `bench` (inherits `release`: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`).
- **Settings:** `--warm-up-time 2 --measurement-time 5`, 100 samples. The value reported is criterion's **median**.
- **Optimizer barriers:** `std::hint::black_box` on every input and every result. Inputs are black-boxed too, so the compiler cannot hoist a loop-invariant conversion out of the timed loop. The 1.0.0 numbers in the previous version of this page did not do that, which is why `Wall::unix_seconds` used to show ~180 ps.
- **Before/after:** the 1.0.0 column comes from the `v1.0.0`-equivalent source (commit `0b3ef42`) running the same benchmark bodies, restricted to the 1.0 API, in the same session.

**Reference platform**

| Property | Value |
| --- | --- |
| CPU | AMD Ryzen 9 9950X3D (16 cores) |
| OS | Windows 11 Pro (build 10.0.26200) |
| Rust toolchain | 1.85.0 (`rust-toolchain.toml`) |
| Machine state | Desktop in normal use, not isolated. Expect run-to-run drift of a few percent on ~20 ns reads and more on sub-nanosecond ones. |

Your numbers will differ: clock-read latency is platform-dependent. What does not depend on the platform is the relative shape: every wrapper should sit on top of the raw `std::time` call it forwards to, **within the same run**.

<br>

## Results

### Monotonic Reading Latency

Capturing a monotonic instant. Every wrapper reduces to `Instant::now()` after inlining.

| Bench | 1.0.0 | 1.1.0 | Δ vs std (1.1.0 run) |
| --- | --- | --- | --- |
| `std::time::Instant::now` | 23.74 ns | **25.09 ns** | &mdash; |
| `clock_lib::now` | 23.71 ns | 23.63 ns | &minus;5.8% |
| `Monotonic::now` | 23.80 ns | 24.33 ns | &minus;3.0% |
| `SystemClock::now` | 23.77 ns | 23.60 ns | &minus;5.9% |

Within noise of the raw call in both releases. (The negative deltas are noise, not a speed-up.)

### Wall-Clock Reading Latency

| Bench | 1.0.0 | 1.1.0 | Δ vs std (1.1.0 run) |
| --- | --- | --- | --- |
| `std::time::SystemTime::now` | 20.82 ns | **22.86 ns** | &mdash; |
| `clock_lib::wall` | 21.94 ns | 22.19 ns | &minus;2.9% |
| `Wall::now` | 21.18 ns | 23.44 ns | +2.5% |

The raw `SystemTime::now` moved by +9.8% between the two runs on unchanged code, which sets the noise floor for this group. `Wall::now` is unchanged source.

### Elapsed-Time Measurement

One clock read plus a subtraction.

| Bench | 1.0.0 | 1.1.0 |
| --- | --- | --- |
| `std::time::Instant::elapsed` | 28.64 ns | **26.40 ns** |
| `Monotonic::elapsed` | 25.30 ns | 25.60 ns |
| `clock_lib::elapsed` | 27.14 ns | 25.87 ns |
| `Clock::elapsed` (`SystemClock`) | &mdash; | 26.34 ns |

### Deadline Arithmetic

New in 1.1. `Monotonic` arithmetic saturates instead of panicking; on the in-range path it costs the same as `std`.

| Bench | 1.1.0 |
| --- | --- |
| `std Instant + Duration` (panics on overflow) | **790 ps** |
| `Monotonic + Duration` (saturates) | 777 ps |
| `Monotonic::checked_add` | 955 ps |
| `std Instant - Instant` | **2.12 ns** |
| `Monotonic - Monotonic` | 2.08 ns |

**Overflow ("never" deadlines).** `now + Duration::MAX` saturates. The clamp point is found by a binary search the first time a process overflows (about 1 µs). It is then cached, because it is verified to be the platform's absolute limit, so later overflows cost one atomic load. Measured in one session on a loaded machine (about 77 concurrent compiler processes, so the absolute numbers are inflated):

| Bench | 1.1.0 as first committed | 1.1.0 (cached limit) |
| --- | --- | --- |
| `Monotonic + Duration::MAX` | 504 ns | 3.4–4.0 ns |
| `Deadline::after(Duration::MAX)` (includes the OS clock read) | 792 ns | 36.7 ns |

The same two runs put `Monotonic + Duration` at 1.67 / 1.55 ns against `std` `Instant + Duration` at 2.00 / 1.56 ns: parity. An independent review on a saturated host had measured the wrapper slower; on this machine it is not. Both paths are one `checked_add`, and the overflow branch is out of line.

### Deadline and Stopwatch

New in 1.1, on the system clock. One clock read each; the wrappers add nothing over writing the comparison by hand.

| Bench | 1.1.0 |
| --- | --- |
| `Instant::now() >= deadline` (hand-written) | **26.78 ns** |
| `Deadline::is_expired` | 25.37 ns |
| `Deadline::remaining` | 26.57 ns |
| `Stopwatch::elapsed` | 25.87 ns |

### Unix Conversions

Converting a captured `Wall` to a number. No clock read; pure arithmetic on the embedded `SystemTime`.

| Bench | 1.0.0 | 1.1.0 |
| --- | --- | --- |
| `Wall::unix_seconds` | 790 ps | 936 ps |
| `Wall::unix_millis` | 3.31 ns | 3.37 ns |
| `Wall::unix_nanos` | 3.58 ns | 3.31 ns |
| `Wall::unix_seconds_signed` | &mdash; | 1.10 ns |
| `Wall::unix_millis_signed` | &mdash; | 3.86 ns |
| `Wall::unix_nanos_signed` | &mdash; | 3.70 ns |

The unsigned accessors' source is identical in both releases; the `unix_seconds` difference is sub-nanosecond code-layout noise. The millisecond and nanosecond accessors pay for 128-bit arithmetic.

### Unix Helpers (capture + convert)

| Bench | 1.0.0 | 1.1.0 |
| --- | --- | --- |
| `clock_lib::unix` | 22.34 ns | **23.01 ns** |
| `clock_lib::unix_ms` | 27.90 ns | 26.87 ns |
| `clock_lib::unix_ns` | 26.84 ns | 26.94 ns |

### RFC 3339 Formatting

New in 1.1. `Display for Wall`, writing into a reused `String` (the formatter itself never allocates).

| Bench | 1.1.0 |
| --- | --- |
| `format!("{wall}")` (auto precision) | **132.4 ns** |
| `format!("{wall:.3}")` | 131.0 ns |

### `ManualClock` Operations

`ManualClock` is the inner loop of test suites, so it has to stay cheap. All operations are a few relaxed atomic operations plus `Instant`/`SystemTime` arithmetic.

| Bench | 1.0.0 | 1.1.0 | Notes |
| --- | --- | --- | --- |
| `now` | 777 ps | **854 ps** | +1 relaxed load (auto-advance check) |
| `wall` | 839 ps | 3.39 ns | +1 relaxed load and branch (wall-only adjustment) |
| `offset` | 599 ps | 599 ps | unchanged |
| `advance` (1 ns) | 4.18 ns | 4.16 ns | `fetch_add` → saturating CAS loop: no measurable cost |
| `advance_to` | &mdash; | 4.67 ns | `fetch_max` |
| `Clock::elapsed` | &mdash; | 5.41 ns | `now` + subtraction |
| `wall` with an active wall step | &mdash; | 5.52 ns | out-of-line path |
| `now` with auto-advance | &mdash; | 5.66 ns | one CAS per reading |

**The `wall` regression is real and deliberate.** 1.0's `wall()` was a single load feeding one addition, which the CPU overlaps almost perfectly across loop iterations. Supporting `step_wall_*`/`set_wall` adds a second atomic load and a branch, and that costs about 2.5 ns per call. The alternatives were to drop wall-only stepping, or to fold the adjustment into a single atomic, which would cap it to the `i64` nanosecond range around 1970 and break `with_start` for most dates. Every arrangement of the extra load and branch measured the same (3.4 ns ± 0.2). In absolute terms, a test that reads `wall()` a million times spends about 3.4 ms on it.

The saturating `advance` (CLK-001 fix) costs nothing measurable over the old wrapping `fetch_add`: both are one locked read-modify-write.

### Trait-Object Dispatch

| Bench | 1.0.0 | 1.1.0 |
| --- | --- | --- |
| `SystemClock` (concrete) | 24.60 ns | **23.94 ns** |
| `&dyn Clock` | 24.85 ns | 24.99 ns |
| `Box<dyn Clock>` | &mdash; | 24.16 ns |

The vtable call is lost in the clock read's noise.

<br>

## Running the Benchmarks

```bash
cargo bench --bench clock_bench
```

Criterion writes estimates to `target/criterion/`. To compare against a baseline:

```bash
# Before a change: record a named baseline.
cargo bench --bench clock_bench -- --save-baseline main

# After the change: compare against it.
cargo bench --bench clock_bench -- --baseline main
```

REPS sets a 5% regression budget on tracked metrics. It is enforced at release time by running the comparison above on a quiet machine and recording the result in the release notes. CI compiles the benchmarks on every push (`clippy --all-targets`) but does not time them: shared CI runners vary by far more than 5% between runs, so a timing gate there would only produce noise.

<br>

## Interpreting the Numbers

- **The clock read dominates.** `Instant::now()` and `SystemTime::now()` call `QueryPerformanceCounter` and `GetSystemTimePreciseAsFileTime` on Windows. On Linux they call `clock_gettime`, which the vDSO serves in user space without a system call. Either way, the read is the cost, and the wrappers inline away.
- **Watch the relative gap within one run, not absolute numbers across runs.** Between the two runs recorded here, raw `SystemTime::now` itself moved by ~10% on unchanged code.
- **Sub-nanosecond results are fragile.** At a few cycles per iteration, code layout and the CPU's ability to overlap iterations decide the number. Treat a ±20% swing there as noise unless the source changed.
- **`ManualClock` is cheap enough for any test.** Even the slowest path (auto-advance, ~5.7 ns) costs under 6 ms per million readings.
- **`&dyn Clock` is effectively free** next to a real clock read. Prefer a generic `C: Clock` anyway in code that reads a `ManualClock` in a hot loop.
