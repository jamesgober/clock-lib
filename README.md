<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>clock-lib</b>
    <br><sub><sup>RUST TIME LIBRARY</sup></sub>
</h1>

<p align="center">
    <a href="https://crates.io/crates/clock-lib"><img src="https://img.shields.io/crates/v/clock-lib.svg" alt="Crates.io"></a>
    <a href="https://crates.io/crates/clock-lib" alt="Download"><img alt="Crates.io Downloads" src="https://img.shields.io/crates/d/clock-lib?color=%230099ff"></a>
    <a href="https://docs.rs/clock-lib"><img src="https://docs.rs/clock-lib/badge.svg" alt="Documentation"></a>
    <a href="https://github.com/jamesgober/clock-lib/actions"><img alt="GitHub CI" src="https://github.com/jamesgober/clock-lib/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md" title="MSRV"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</p>

<p align="center">
    <b>Monotonic & Wall-Clock Time, Done Right</b>
    <br>
    <i>Simple, fast, zero-dependency time readings with a mockable clock for deterministic testing</i>
</p>

<p>
    <strong>clock-lib</strong> provides clean, correct access to the two kinds of time every program needs: <b>monotonic time</b> for measuring durations and deadlines, and <b>wall-clock time</b> for timestamps. It draws a hard line between them at the type level, so you can never accidentally measure elapsed time with a clock that jumps backwards during an NTP sync.
</p>

<p>
    The headline feature is the <b>mockable clock</b>. In production, time comes from the operating system. In tests, you inject a <code>ManualClock</code> and move time instantly: no <code>sleep</code>, no flaky timing, exact durations, and reproducible timestamps when you pin the start time. You can even step wall time on its own to prove your code survives an NTP correction. Every time-driven system built on <strong>clock-lib</strong> (rate limiters, TTL caches, timeouts, schedulers) becomes straightforward to test.
</p>

<p>
    Built for performance and correctness: a one-line Tier-1 API for the common case, zero default dependencies, <code>#![forbid(unsafe_code)]</code> at the crate root, and no panics anywhere in the public API. <strong>clock-lib</strong> wraps <code>std::time</code> with a thin, inlined layer that <a href="docs/PERFORMANCE.md">benchmarks within noise of raw <code>std::time</code></a>: you pay for the clock read and nothing more.
</p>

## Features

### Two Kinds of Time, Clearly Separated
- **Monotonic readings** &mdash; for elapsed time and deadlines; never go backwards
- **Wall-clock readings** &mdash; for timestamps; UTC, RFC 3339 formatting built in
- **Type-level separation** &mdash; the compiler refuses to subtract a wall-clock time from a monotonic one
- **Arithmetic that never panics** &mdash; `start + ttl` saturates instead of overflowing; `checked_*` when you want an `Option`

### Mockable Clock (the killer feature)
- **`Clock` trait** &mdash; inject time into any system; works through `&C`, `Box<C>`, `Arc<C>` and `dyn Clock`
- **`SystemClock`** &mdash; reads the OS in production; zero-sized and `const`
- **`ManualClock`** &mdash; lock-free test clock: `advance`, `advance_to`, fixed start time, wall-only steps (simulate NTP jumps), auto-advance per reading
- **`Deadline` / `Stopwatch`** &mdash; the two most common timing patterns, on any clock

### Simple Tier-1 API
- **`now()`** &mdash; monotonic reading
- **`elapsed(earlier)`** &mdash; duration since a monotonic reading
- **`wall()`** &mdash; wall-clock reading
- **`unix()` / `unix_ms()` / `unix_ns()`** &mdash; Unix time conveniences

### Lean & Correct
- **Zero default dependencies** &mdash; wraps `std::time`; `serde` is an opt-in feature
- **`#![forbid(unsafe_code)]`** &mdash; enforced by the compiler
- **Interop** &mdash; lossless `From` conversions to and from `Instant` and `SystemTime`
- **Cross-platform** &mdash; Linux, macOS, Windows

> This crate deliberately does **not** do calendar math, parsing, or time zones. For that, use [`chrono`](https://crates.io/crates/chrono), [`time`](https://crates.io/crates/time) or [`jiff`](https://crates.io/crates/jiff). clock-lib is the lean primitive layer beneath them.

<hr>

## Installation

```toml
[dependencies]
clock-lib = "1.1"
```

### Optional features

| Feature | Default | Enables |
| --- | --- | --- |
| `std` | yes | Every reading API. |
| `serde` | no | `Serialize`/`Deserialize` for `Wall`, wire-compatible with `std::time::SystemTime`. |

```toml
[dependencies]
clock-lib = { version = "1.1", features = ["serde"] }
```

With `default-features = false`, the crate compiles for `no_std` targets but exposes only the `VERSION` constant: every reading needs an operating-system clock. That keeps a dependent crate's own `no_std` build working; it does not provide a clock on bare metal.

<hr>

## Quick Start

```rust
use clock_lib as clock;
use std::time::Duration;

// Measure elapsed time (monotonic: safe across NTP corrections)
let start = clock::now();
// ... do work ...
let took = clock::elapsed(start);

// Build a deadline
let deadline = start + Duration::from_secs(5);
assert!(clock::now() < deadline);

// Get a timestamp (wall-clock)
let unix_seconds = clock::unix();
let rfc3339 = clock::wall().to_string(); // "2026-10-13T08:30:00.123456700Z"
```

Reach for the typed surface when you need it &mdash; [`Monotonic`](https://docs.rs/clock-lib/latest/clock_lib/struct.Monotonic.html) for elapsed-time math and deadlines, [`Wall`](https://docs.rs/clock-lib/latest/clock_lib/struct.Wall.html) for timestamps. The compiler refuses to mix the two.

### Deterministic Time in Tests

```rust
use clock_lib::{Clock, ManualClock, Monotonic};
use std::time::Duration;

fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
    clock.elapsed(stamp) >= ttl
}

let clock = ManualClock::new();
let stamp = clock.now();
assert!(!expired(&clock, stamp, Duration::from_secs(60)));

clock.advance(Duration::from_secs(60));   // instant, no sleep
assert!(expired(&clock, stamp, Duration::from_secs(60)));
```

Inject [`Clock`](https://docs.rs/clock-lib/latest/clock_lib/trait.Clock.html) into anything time-driven and your test suite drops every `thread::sleep` it had. Inside code that takes a clock, measure with `clock.elapsed(stamp)`: the free `clock_lib::elapsed` always reads the OS clock.

### Reproducible Timestamps and NTP Jumps

```rust
use clock_lib::{Clock, ManualClock, Wall};
use std::time::Duration;

let start = Wall::from_unix_seconds(1_700_000_000).expect("representable");
let clock = ManualClock::with_start(start);
let began = clock.now();

clock.advance(Duration::from_secs(90));
assert_eq!(clock.wall().to_string(), "2023-11-14T22:14:50Z"); // same on every run

clock.step_wall_back(Duration::from_secs(3_600));             // NTP steps the wall clock back
assert_eq!(clock.elapsed(began), Duration::from_secs(90));    // monotonic time is unaffected
```

### Deadlines and Stopwatches

```rust
use clock_lib::{Deadline, ManualClock, Stopwatch};
use std::time::Duration;

let clock = ManualClock::new();
let deadline = Deadline::after_with(&clock, Duration::from_secs(5));
let mut watch = Stopwatch::start_with(&clock);

clock.advance(Duration::from_secs(2));
assert_eq!(deadline.remaining(), Duration::from_secs(3));
assert_eq!(watch.lap(), Duration::from_secs(2));

clock.advance(Duration::from_secs(3));
assert!(deadline.is_expired());

// In production: Deadline::after(ttl) and Stopwatch::start() use the OS clock.
```

<hr>

## Documentation

- [API reference](docs/API.md) &mdash; every public type, trait, and function with examples.
- [Performance](docs/PERFORMANCE.md) &mdash; benchmark methodology and measured overhead against raw `std::time`.
- [Developer guidelines](docs/GUIDELINES.md) &mdash; the engineering bar this project is built to.
- [Release notes](docs/release/) &mdash; what shipped, when, and why.

<br>

## Contributing

Contributions are welcome under the project's dual license. Before opening a pull request, please make sure:

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --all-targets --all-features -- -D warnings` is clean.
3. `cargo test --all-features`, `cargo test` and `cargo test --no-default-features` pass.
4. New public items include documentation and at least one example.
5. The [API reference](docs/API.md) and [CHANGELOG](CHANGELOG.md) are updated alongside code changes.

<br>

<!-- LICENSE
############################################# -->
<div id="license">
    <h2>License</h2>
    <p><b>clock-lib</b> is <b>dual-licensed</b> under either of:</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; see <a href="./LICENSE-APACHE" title="Apache-2.0 License">LICENSE-APACHE</a> or <a href="http://www.apache.org/licenses/LICENSE-2.0" title="Apache-2.0 License" target="_blank">apache.org/licenses/LICENSE-2.0</a></li>
        <li><b>MIT License</b> &mdash; see <a href="./LICENSE-MIT" title="MIT License">LICENSE-MIT</a> or <a href="https://opensource.org/licenses/MIT" title="MIT License" target="_blank">opensource.org/licenses/MIT</a></li>
    </ul>
    <p>at your option.</p>
    <br>
    <h3>Contribution</h3>
    <p>Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in <b>clock-lib</b> by you, as defined in the <b>Apache-2.0 license</b>, shall be <b>dual-licensed</b> as above, without any additional terms or conditions.</p>
    <p>Unless required by applicable law or agreed to in writing, software distributed under the Licenses is distributed on an <b>"AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND</b>, either express or implied.</p>
</div>

<!-- FOOT COPYRIGHT
################################################# -->
<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>JAMES GOBER.</strong></sup>
</div>