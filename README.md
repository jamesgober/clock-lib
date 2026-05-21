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
    <strong>clock-lib</strong> provides clean, correct access to the two kinds of time every program needs: <b>monotonic time</b> for measuring durations, and <b>wall-clock time</b> for timestamps. It draws a hard line between them at the API level, so you can never accidentally measure elapsed time with a clock that jumps backwards during an NTP sync.
</p>

<p>
    The headline feature is the <b>mockable clock</b>. In production, time comes from the operating system. In tests, you inject a <code>ManualClock</code> and advance time instantly &mdash; no <code>sleep</code>, no flaky timing, fully deterministic. Every time-driven system built on <strong>clock-lib</strong> (rate limiters, TTL caches, timeouts, schedulers) becomes trivially testable.
</p>

<p>
    Built for performance and correctness: a one-line Tier-1 API for the common case, zero runtime dependencies, no <code>unsafe</code> code, and full <code>no_std</code> support. <strong>clock-lib</strong> wraps <code>std::time</code> primitives with a thin, well-inlined layer that costs nothing and forces the right usage.
</p>

## Features

### Two Kinds of Time, Clearly Separated
- **Monotonic readings** &mdash; for measuring elapsed time; never goes backwards
- **Wall-clock readings** &mdash; for timestamps and calendar time
- **Type-level separation** &mdash; the API makes it impossible to subtract a wall-clock time from a monotonic one

### Mockable Clock (the killer feature)
- **`SystemClock`** &mdash; reads the OS in production
- **`ManualClock`** &mdash; advance time instantly in tests, fully deterministic
- **`Clock` trait** &mdash; inject time into any system for testability

### Simple Tier-1 API
- **`now()`** &mdash; monotonic reading
- **`elapsed(earlier)`** &mdash; duration since a monotonic reading
- **`wall()`** &mdash; wall-clock reading
- **`unix()` / `unix_ms()` / `unix_ns()`** &mdash; unix time conveniences

### Lean & Correct
- **Zero runtime dependencies** &mdash; wraps `std::time`
- **No `unsafe` code**
- **`no_std` support** &mdash; via the default-off path
- **Cross-platform** &mdash; Linux, macOS, Windows

> This crate deliberately does **not** do calendar math, date formatting, or timezones. For that, use [`chrono`](https://crates.io/crates/chrono) or [`time`](https://crates.io/crates/time). clock-lib is the lean primitive layer beneath them.

<hr>

## Installation

```toml
[dependencies]
clock-lib = "0.4"
```

<hr>

## Quick Start

```rust
use clock_lib as clock;

// Measure elapsed time (monotonic — safe)
let start = clock::now();
// ... do work ...
let took = clock::elapsed(start);

// Get a timestamp (wall-clock)
let unix_seconds = clock::unix();
```

Reach for the typed surface when you need it &mdash; [`Monotonic`](https://docs.rs/clock-lib/latest/clock_lib/struct.Monotonic.html) for elapsed-time math, [`Wall`](https://docs.rs/clock-lib/latest/clock_lib/struct.Wall.html) for timestamps. The compiler refuses to mix the two, so you cannot accidentally measure an interval with a clock that can step backwards.

### Deterministic Time in Tests

```rust
use clock_lib::{Clock, ManualClock, Monotonic};
use std::time::Duration;

fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
    clock.now().duration_since(stamp) >= ttl
}

let clock = ManualClock::new();
let stamp = clock.now();
assert!(!expired(&clock, stamp, Duration::from_secs(60)));

clock.advance(Duration::from_secs(60));   // instant, no sleep
assert!(expired(&clock, stamp, Duration::from_secs(60)));
```

Inject [`Clock`](https://docs.rs/clock-lib/latest/clock_lib/trait.Clock.html) into anything time-driven (rate limiters, TTL caches, timeouts) and your test suite drops every `thread::sleep` it had.

<hr>

## Documentation

- [API reference](docs/API.md) &mdash; every public type, trait, and function with examples.
- [Performance](docs/PERFORMANCE.md) &mdash; benchmark methodology and the zero-overhead claim, verified.
- [Developer guidelines](docs/GUIDELINES.md) &mdash; the engineering bar this project is built to.
- [Release notes](docs/release/) &mdash; what shipped, when, and why.

<br>

## Contributing

Contributions are welcome under the project's dual license. Before opening a pull request, please make sure:

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --all-targets --all-features -- -D warnings` is clean.
3. `cargo test --all-features` passes.
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