<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>clock-lib</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./GUIDELINES.md" title="Developer Guidelines"><b>GUIDELINES</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <span>API</span>
    </sup>
</div>
<br>

> **Status:** This reference tracks the public surface of `clock-lib` **0.4.0**. Every example is verified against the current codebase.

`clock-lib` exposes two complementary readings &mdash; **monotonic** (for measuring elapsed time) and **wall-clock** (for timestamps) &mdash; behind a one-line Tier-1 API. The two are distinct types and cannot be mixed: the compiler rejects any attempt to subtract a wall-clock reading from a monotonic one, eliminating an entire class of subtle timing bugs.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Tier-1 Functions](#tier-1-functions)
  - [`now`](#now)
  - [`elapsed`](#elapsed)
  - [`wall`](#wall)
  - [`unix`](#unix)
  - [`unix_ms`](#unix_ms)
  - [`unix_ns`](#unix_ns)
- [Types](#types)
  - [`Monotonic`](#monotonic)
    - [`Monotonic::now`](#monotonicnow)
    - [`Monotonic::elapsed`](#monotonicelapsed)
    - [`Monotonic::duration_since`](#monotonicduration_since)
    - [`Monotonic::checked_duration_since`](#monotonicchecked_duration_since)
    - [`Monotonic::saturating_duration_since`](#monotonicsaturating_duration_since)
  - [`Wall`](#wall-1)
    - [`Wall::now`](#wallnow)
    - [`Wall::unix_seconds`](#wallunix_seconds)
    - [`Wall::unix_millis`](#wallunix_millis)
    - [`Wall::unix_nanos`](#wallunix_nanos)
- [Traits](#traits)
  - [`Clock`](#clock)
- [Clock Implementations](#clock-implementations)
  - [`SystemClock`](#systemclock)
    - [`SystemClock::new`](#systemclocknew)
  - [`ManualClock`](#manualclock)
    - [`ManualClock::new`](#manualclocknew)
    - [`ManualClock::advance`](#manualclockadvance)
    - [`ManualClock::offset`](#manualclockoffset)
- [Constants](#constants)
  - [`VERSION`](#version)

<br>

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
clock-lib = "0.4"
```

`clock-lib` has zero runtime dependencies, supports `no_std` builds via `default-features = false`, and contains no `unsafe` code.

<br>

## Quick Start

```rust
use clock_lib as clock;

// Measure elapsed time (monotonic — safe even across NTP corrections).
let start = clock::now();
// ... do work ...
let took = clock::elapsed(start);

// Stamp an event (wall-clock).
let seconds = clock::unix();
let millis = clock::unix_ms();
let nanos = clock::unix_ns();
# let _ = (took, seconds, millis, nanos);
```

<br>

## Tier-1 Functions

The Tier-1 surface is the one-line entry point per common operation. It is the recommended path unless you have a reason to hold the typed values directly.

### `now`

```rust
pub fn now() -> Monotonic
```

Captures the current monotonic time from the operating system. Pair with [`elapsed`](#elapsed) to measure how long an operation took.

**Returns:** a [`Monotonic`](#monotonic) reading suitable for delta math against another `Monotonic` from the same process.

**Example &mdash; measuring a block of work:**

```rust
use clock_lib as clock;

let start = clock::now();
// ... do work ...
let took = clock::elapsed(start);
println!("operation took {took:?}");
```

**Example &mdash; tracking multiple checkpoints:**

```rust
use clock_lib as clock;

let t0 = clock::now();
let parsed = parse_input();
let t1 = clock::now();
let processed = process(parsed);
let t2 = clock::now();

let parse_time = t1.duration_since(t0);
let process_time = t2.duration_since(t1);
# fn parse_input() {}
# fn process(_: ()) {}
# let _ = (parse_time, process_time);
```

<br>

### `elapsed`

```rust
pub fn elapsed(earlier: Monotonic) -> Duration
```

Returns the [`Duration`](https://doc.rust-lang.org/core/time/struct.Duration.html) between `earlier` and the current monotonic time. Equivalent to `Monotonic::now().duration_since(earlier)`.

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `earlier` | [`Monotonic`](#monotonic) | The reading captured at the start of the interval. |

**Returns:** the time elapsed since `earlier`, as a `Duration`.

**Example &mdash; rate limiting:**

```rust
use clock_lib as clock;
use std::time::Duration;

let last_call = clock::now();
// ... later ...
if clock::elapsed(last_call) < Duration::from_secs(1) {
    // too soon — skip
}
```

<br>

### `wall`

```rust
pub fn wall() -> Wall
```

Captures the current wall-clock time from the operating system's real-time clock. Use this for timestamps. Do **not** use it to measure elapsed time &mdash; wall-clock readings can jump forwards or backwards (NTP, DST, manual changes).

**Returns:** a [`Wall`](#wall-1) reading that can be converted to Unix time at second, millisecond, or nanosecond resolution.

**Example:**

```rust
use clock_lib as clock;

let stamp = clock::wall();
let event_seconds = stamp.unix_seconds();
let event_millis = stamp.unix_millis();
# let _ = (event_seconds, event_millis);
```

<br>

### `unix`

```rust
pub fn unix() -> u64
```

Returns the current Unix time in whole seconds. Equivalent in spirit to C's `time(NULL)` or PHP's `time()`.

**Returns:** seconds elapsed since `1970-01-01 00:00:00 UTC`, as `u64`. Returns `0` if the system clock is set to a moment before the Unix epoch.

**Example:**

```rust
use clock_lib as clock;

let now = clock::unix();
println!("event at {now}");
```

<br>

### `unix_ms`

```rust
pub fn unix_ms() -> u128
```

Returns the current Unix time in whole milliseconds.

**Returns:** milliseconds elapsed since `1970-01-01 00:00:00 UTC`, as `u128`. Returns `0` if the system clock is set to a moment before the Unix epoch.

**Example:**

```rust
use clock_lib as clock;

let event_id = format!("evt-{}", clock::unix_ms());
# let _ = event_id;
```

<br>

### `unix_ns`

```rust
pub fn unix_ns() -> u128
```

Returns the current Unix time in whole nanoseconds.

**Returns:** nanoseconds elapsed since `1970-01-01 00:00:00 UTC`, as `u128`. Returns `0` if the system clock is set to a moment before the Unix epoch.

**Example:**

```rust
use clock_lib as clock;

let high_res_stamp = clock::unix_ns();
# let _ = high_res_stamp;
```

<br>

## Types

### `Monotonic`

```rust
pub struct Monotonic(/* opaque */);
```

A captured monotonic instant. `Monotonic` wraps a single sample of the operating system's monotonic clock. Monotonic time never goes backwards, which makes it the correct choice for measuring elapsed time: rate limiting, timeouts, benchmarks, retry backoff.

The absolute value of a `Monotonic` reading carries no calendar meaning &mdash; it is only useful as a delta against another `Monotonic` from the same process. For calendar timestamps, use [`Wall`](#wall-1) instead.

**Derived traits:** `Debug`, `Copy`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`.

**Type-level separation.** `Monotonic` and [`Wall`](#wall-1) are deliberately distinct types with no cross-type arithmetic. The compiler will refuse to mix them &mdash; that is the central design choice of this crate, and it eliminates a class of bugs at the source.

<br>

#### `Monotonic::now`

```rust
pub fn now() -> Monotonic
```

Captures the current monotonic time from the operating system. The crate-level [`now`](#now) function is a one-line shortcut for the same thing.

**Example:**

```rust
use clock_lib::Monotonic;

let t = Monotonic::now();
# let _ = t;
```

<br>

#### `Monotonic::elapsed`

```rust
pub fn elapsed(self) -> Duration
```

Returns the [`Duration`](https://doc.rust-lang.org/core/time/struct.Duration.html) elapsed since this reading was captured. Equivalent to `Monotonic::now().duration_since(self)`.

**Example:**

```rust
use clock_lib::Monotonic;

let t = Monotonic::now();
// ... work ...
let took = t.elapsed();
# let _ = took;
```

<br>

#### `Monotonic::duration_since`

```rust
pub fn duration_since(self, earlier: Monotonic) -> Duration
```

Returns the `Duration` between two readings.

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `earlier` | [`Monotonic`](#monotonic) | A reading that must be at or before `self`. |

**Panics:** if `earlier` is later than `self`. Prefer [`checked_duration_since`](#monotonicchecked_duration_since) or [`saturating_duration_since`](#monotonicsaturating_duration_since) when the ordering is not guaranteed by construction.

**Example:**

```rust
use clock_lib::Monotonic;

let earlier = Monotonic::now();
let later = Monotonic::now();
let delta = later.duration_since(earlier);
# let _ = delta;
```

<br>

#### `Monotonic::checked_duration_since`

```rust
pub fn checked_duration_since(self, earlier: Monotonic) -> Option<Duration>
```

The non-panicking counterpart to [`duration_since`](#monotonicduration_since). Returns `Some(duration)` if `self >= earlier`, otherwise `None`.

**Example &mdash; tolerating out-of-order timestamps:**

```rust
use clock_lib::Monotonic;

let a = Monotonic::now();
let b = Monotonic::now();
match b.checked_duration_since(a) {
    Some(delta) => println!("forward {delta:?}"),
    None => println!("a was after b — skipping"),
}
```

<br>

#### `Monotonic::saturating_duration_since`

```rust
pub fn saturating_duration_since(self, earlier: Monotonic) -> Duration
```

Returns the duration since `earlier`, saturating at zero when `earlier` is later than `self`. Never panics.

**Example:**

```rust
use clock_lib::Monotonic;
use std::time::Duration;

let a = Monotonic::now();
let b = Monotonic::now();
let delta = a.saturating_duration_since(b);
assert!(delta == Duration::ZERO || delta > Duration::ZERO);
```

<br>

### `Wall`

```rust
pub struct Wall(/* opaque */);
```

A captured wall-clock instant. `Wall` wraps a single sample of the operating system's real-time clock. Convert it to Unix time with [`unix_seconds`](#wallunix_seconds), [`unix_millis`](#wallunix_millis), or [`unix_nanos`](#wallunix_nanos).

`Wall` is the right tool for timestamps in logs, audit records, and anything that needs to line up with what a wristwatch shows. It is the wrong tool for measuring elapsed time &mdash; wall-clock readings can jump. For that, use [`Monotonic`](#monotonic).

**Derived traits:** `Debug`, `Copy`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`.

**Pre-epoch behavior.** If the system clock is set to a moment before `1970-01-01 00:00:00 UTC`, all `unix_*` accessors saturate to `0`. They never panic and never silently wrap.

<br>

#### `Wall::now`

```rust
pub fn now() -> Wall
```

Captures the current wall-clock time from the operating system. The crate-level [`wall`](#wall) function is a one-line shortcut for the same thing.

**Example:**

```rust
use clock_lib::Wall;

let stamp = Wall::now();
# let _ = stamp;
```

<br>

#### `Wall::unix_seconds`

```rust
pub fn unix_seconds(self) -> u64
```

Returns Unix time in whole seconds.

**Returns:** seconds since `1970-01-01 00:00:00 UTC`. Returns `0` for pre-epoch system clocks. The `u64` range is sufficient to represent any plausible wall-clock value through year 584,942,417,355 &mdash; this accessor cannot overflow.

**Example:**

```rust
use clock_lib::Wall;

let secs = Wall::now().unix_seconds();
assert!(secs > 0);
```

<br>

#### `Wall::unix_millis`

```rust
pub fn unix_millis(self) -> u128
```

Returns Unix time in whole milliseconds.

**Returns:** milliseconds since `1970-01-01 00:00:00 UTC`. Returns `0` for pre-epoch system clocks. The `u128` return type cannot overflow for any representable `SystemTime`.

**Example:**

```rust
use clock_lib::Wall;

let millis = Wall::now().unix_millis();
assert!(millis > 0);
```

<br>

#### `Wall::unix_nanos`

```rust
pub fn unix_nanos(self) -> u128
```

Returns Unix time in whole nanoseconds.

**Returns:** nanoseconds since `1970-01-01 00:00:00 UTC`. Returns `0` for pre-epoch system clocks. The `u128` return type cannot overflow for any representable `SystemTime`.

**Example:**

```rust
use clock_lib::Wall;

let nanos = Wall::now().unix_nanos();
assert!(nanos > 0);
```

<br>

## Traits

### `Clock`

```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> Monotonic;
    fn wall(&self) -> Wall;
}
```

A source of time. `Clock` lets time-driven code depend on an injected reading source instead of calling the OS directly. Production code passes [`SystemClock`](#systemclock); tests pass [`ManualClock`](#manualclock) and advance time without sleeping.

**Required methods:**

| Method | Returns | Description |
| --- | --- | --- |
| `now(&self)` | [`Monotonic`](#monotonic) | Captures a monotonic reading at the time of the call. |
| `wall(&self)` | [`Wall`](#wall-1) | Captures a wall-clock reading at the time of the call. |

**Bounds.** `Clock: Send + Sync` so instances can be shared across threads &mdash; typically via [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html).

**Blanket implementations.** `Clock` is implemented for `Arc<C>` and `&C` where `C: Clock`, so the same value can be shared and reused without re-implementing the trait.

**Example &mdash; defining a time-driven primitive:**

```rust
use clock_lib::{Clock, Monotonic};
use std::time::Duration;

struct Deadline<C: Clock> {
    clock: C,
    started: Monotonic,
    ttl: Duration,
}

impl<C: Clock> Deadline<C> {
    fn new(clock: C, ttl: Duration) -> Self {
        let started = clock.now();
        Self { clock, started, ttl }
    }

    fn is_expired(&self) -> bool {
        self.clock.now().duration_since(self.started) >= self.ttl
    }
}
```

**Example &mdash; production vs test:**

```rust
use clock_lib::{Clock, ManualClock, Monotonic, SystemClock};
use std::time::Duration;

fn measure_then_check<C: Clock>(clock: &C, target: Duration) -> bool {
    let start = clock.now();
    // ... work ...
    clock.now().duration_since(start) >= target
}

// Production
let sys = SystemClock::new();
let _ok = measure_then_check(&sys, Duration::from_millis(1));

// Test — fully deterministic
let test = ManualClock::new();
test.advance(Duration::from_secs(1));
assert!(measure_then_check(&test, Duration::ZERO));
```

<br>

## Clock Implementations

### `SystemClock`

```rust
pub struct SystemClock;
```

A zero-sized, `Copy` clock backed by the operating system. `SystemClock::now()` forwards to [`Monotonic::now`](#monotonicnow); `SystemClock::wall()` forwards to [`Wall::now`](#wallnow). The constructor is `const`, so `SystemClock` can be embedded in `const` items.

**Derived traits:** `Debug`, `Default`, `Copy`, `Clone`.

<br>

#### `SystemClock::new`

```rust
pub const fn new() -> SystemClock
```

Constructs a new system clock.

**Example:**

```rust
use clock_lib::{Clock, SystemClock};

const CLOCK: SystemClock = SystemClock::new();
let t = CLOCK.now();
# let _ = t;
```

<br>

### `ManualClock`

```rust
pub struct ManualClock { /* opaque */ }
```

A clock under your control, for deterministic testing. `ManualClock` captures the OS monotonic and wall-clock anchors at construction, then advances **only** when you call [`advance`](#manualclockadvance). The clock never moves on its own, so timing-driven code can be exercised at exact, repeatable instants.

`ManualClock` is `Send + Sync`. Share it across the test driver and the code under test by wrapping in an [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html); both ends observe the same readings.

**Derived traits:** `Debug`, `Default`.

**Example &mdash; sleep-free TTL test:**

```rust
use clock_lib::{Clock, ManualClock, Monotonic};
use std::sync::Arc;
use std::time::Duration;

fn expired<C: Clock>(clock: &C, stamp: Monotonic, ttl: Duration) -> bool {
    clock.now().duration_since(stamp) >= ttl
}

let clock = Arc::new(ManualClock::new());
let stamp = clock.now();
let ttl = Duration::from_secs(60);

assert!(!expired(&*clock, stamp, ttl));
clock.advance(Duration::from_secs(30));
assert!(!expired(&*clock, stamp, ttl));
clock.advance(Duration::from_secs(30));
assert!(expired(&*clock, stamp, ttl));
```

<br>

#### `ManualClock::new`

```rust
pub fn new() -> ManualClock
```

Constructs a new manual clock anchored at the current OS time. The offset starts at zero, so the first reading equals the anchor.

**Example:**

```rust
use clock_lib::ManualClock;

let clock = ManualClock::new();
# let _ = clock;
```

<br>

#### `ManualClock::advance`

```rust
pub fn advance(&self, by: Duration)
```

Advances the clock forward by `by`. Successive calls accumulate. If the cumulative offset would exceed [`u64::MAX`] nanoseconds (≈584 years), the offset saturates &mdash; well outside any plausible test scenario.

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `by` | [`Duration`](https://doc.rust-lang.org/core/time/struct.Duration.html) | The amount of time to advance the clock by. |

**Note.** `advance` takes `&self` so it can be called from any reference, including through an [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html). Internally, the offset is an [`AtomicU64`](https://doc.rust-lang.org/core/sync/atomic/struct.AtomicU64.html) that accumulates lock-free.

**Example:**

```rust
use clock_lib::{Clock, ManualClock};
use std::time::Duration;

let clock = ManualClock::new();
let before = clock.now();
clock.advance(Duration::from_secs(5));
let after = clock.now();
assert_eq!(after.duration_since(before), Duration::from_secs(5));
```

<br>

#### `ManualClock::offset`

```rust
pub fn offset(&self) -> Duration
```

Returns the cumulative offset that has been added to this clock since it was constructed.

**Example:**

```rust
use clock_lib::ManualClock;
use std::time::Duration;

let clock = ManualClock::new();
clock.advance(Duration::from_secs(1));
clock.advance(Duration::from_secs(2));
assert_eq!(clock.offset(), Duration::from_secs(3));
```

<br>

## Constants

### `VERSION`

```rust
pub const VERSION: &str
```

The crate version string, populated by Cargo at build time. Useful for embedding the running clock-lib version in logs or diagnostics.

**Example:**

```rust
println!("clock-lib {}", clock_lib::VERSION);
```
