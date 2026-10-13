<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>clock-lib</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <span>API</span>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./PERFORMANCE.md" title="Performance"><b>PERFORMANCE</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./GUIDELINES.md" title="Developer Guidelines"><b>GUIDELINES</b></a>
    </sup>
</div>
<br>

> **Status:** This reference tracks the public surface of `clock-lib` **1.1.0**. Every runnable Rust example on this page is compiled and executed by `cargo test --doc` (the file is included as a doctest), so the examples cannot drift from the code. Signature summaries are marked `ignore`.

`clock-lib` exposes two complementary readings: **monotonic** (for measuring elapsed time) and **wall-clock** (for timestamps), behind a one-line Tier-1 API. The two are distinct types and cannot be mixed: the compiler rejects any attempt to subtract a wall-clock reading from a monotonic one. A `Clock` trait lets time-driven code take an injected clock, so tests can drive time with `ManualClock` instead of sleeping.

## Table of Contents

- [Installation](#installation)
  - [Feature Flags](#feature-flags)
  - [Platforms and Resolution](#platforms-and-resolution)
- [Quick Start](#quick-start)
- [Tier-1 Functions](#tier-1-functions)
  - [`now`](#now) &middot; [`elapsed`](#elapsed) &middot; [`wall`](#wall) &middot; [`unix`](#unix) &middot; [`unix_ms`](#unix_ms) &middot; [`unix_ns`](#unix_ns)
- [Types](#types)
  - [`Monotonic`](#monotonic)
    - [Reading and measuring](#monotonic-reading-and-measuring)
    - [Deadline arithmetic](#monotonic-deadline-arithmetic)
    - [Conversions](#monotonic-conversions)
  - [`Wall`](#wall-1)
    - [Reading and constructing](#wall-reading-and-constructing)
    - [Unix accessors](#wall-unix-accessors)
    - [Arithmetic](#wall-arithmetic)
    - [Conversions](#wall-conversions)
    - [RFC 3339 formatting](#wall-rfc-3339-formatting)
    - [Serde](#wall-serde)
- [Traits](#traits)
  - [`Clock`](#clock)
- [Clock Implementations](#clock-implementations)
  - [`SystemClock`](#systemclock)
  - [`ManualClock`](#manualclock)
    - [Construction](#manualclock-construction)
    - [Readings](#manualclock-readings)
    - [Moving time](#manualclock-moving-time)
    - [Moving wall time only](#manualclock-moving-wall-time-only)
    - [Auto-advance](#manualclock-auto-advance)
- [Helpers](#helpers)
  - [`Deadline`](#deadline)
  - [`Stopwatch`](#stopwatch)
- [Testing Patterns](#testing-patterns)
- [Constants](#constants)
  - [`VERSION`](#version)

<br>

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
clock-lib = "1.1"
```

By default `clock-lib` has zero runtime dependencies and contains no `unsafe` code (`#![forbid(unsafe_code)]`).

### Feature Flags

| Feature | Default | What it enables | Dependencies |
| --- | --- | --- | --- |
| `std` | yes | Every reading API: `Monotonic`, `Wall`, `Clock`, `SystemClock`, `ManualClock`, `Deadline`, `Stopwatch`, and the Tier-1 free functions. | none |
| `serde` | no | `Serialize` / `Deserialize` for [`Wall`](#wall-serde), in the same format as `std::time::SystemTime`. Implies `std`. | `serde` |

```toml
[dependencies]
clock-lib = { version = "1.1", features = ["serde"] }
```

**Without `std`.** With `default-features = false` the crate compiles for `no_std` targets but exposes only [`VERSION`](#version): every reading needs an operating-system clock. This lets a crate that depends on `clock-lib` behind its own `std` feature keep building for embedded targets. It does not provide a clock on bare metal.

```toml
[dependencies]
clock-lib = { version = "1.1", default-features = false }
```

### Platforms and Resolution

| Platform | Monotonic source | Wall source | Tick |
| --- | --- | --- | --- |
| Windows | `QueryPerformanceCounter` | `GetSystemTimePreciseAsFileTime` | 100 ns |
| Linux | `clock_gettime(CLOCK_MONOTONIC)` (vDSO) | `clock_gettime(CLOCK_REALTIME)` (vDSO) | 1 ns nominal |
| macOS | `clock_gettime(CLOCK_UPTIME_RAW)` | `clock_gettime(CLOCK_REALTIME)` | 1 ns nominal |

Back-to-back monotonic readings are often equal, especially on Windows. That is correct for a non-decreasing clock. On Windows, `Wall::unix_nanos()` is always a multiple of 100.

On `wasm32-unknown-unknown` the standard library has no clock: `std::time::Instant::now()` and `SystemTime::now()` panic, so every function that reads the OS clock panics there. `ManualClock` readings, conversions and arithmetic still work. Browser time support is not part of 1.1.

<br>

## Quick Start

```rust
use clock_lib as clock;
use std::time::Duration;

// Measure elapsed time (monotonic: safe even across NTP corrections).
let start = clock::now();
// ... do work ...
let took = clock::elapsed(start);

// Build a deadline.
let deadline = start + Duration::from_secs(5);
assert!(clock::now() < deadline);

// Stamp an event (wall-clock).
let seconds = clock::unix();
let millis = clock::unix_ms();
let rfc3339 = clock::wall().to_string(); // e.g. "2026-10-13T08:30:00.123456700Z"

assert!(took < Duration::from_secs(60));
assert!(millis / 1_000 >= u128::from(seconds) - 60);
assert!(rfc3339.ends_with('Z'));
```

<br>

## Tier-1 Functions

The Tier-1 surface is the one-line entry point per common operation. Every Tier-1 function reads the **operating-system clock**. Code that takes an injected [`Clock`](#clock) should use the clock's methods instead, so tests can control time.

### `now`

```rust,ignore
pub fn now() -> Monotonic
```

Captures the current monotonic time from the operating system. Pair with [`elapsed`](#elapsed) to measure how long an operation took, or add a `Duration` to build a deadline.

**Example: tracking checkpoints**

```rust
use clock_lib as clock;

fn parse_input() -> u32 { 7 }
fn process(n: u32) -> u32 { n * 2 }

let t0 = clock::now();
let parsed = parse_input();
let t1 = clock::now();
let processed = process(parsed);
let t2 = clock::now();

let parse_time = t1 - t0;
let process_time = t2 - t1;
assert_eq!(processed, 14);
assert!(parse_time + process_time == t2 - t0);
```

<br>

### `elapsed`

```rust,ignore
pub fn elapsed(earlier: Monotonic) -> Duration
```

Returns the time between `earlier` and the operating system's current monotonic time. Equivalent to `Monotonic::now().saturating_duration_since(earlier)`.

> **Only for OS readings.** `elapsed` always reads the OS clock. On a reading taken from a `ManualClock` it would measure real time and ignore every `advance`. For injected clocks, use [`Clock::elapsed`](#clock).

| Parameter | Type | Description |
| --- | --- | --- |
| `earlier` | [`Monotonic`](#monotonic) | A reading from the OS clock, taken at the start of the interval. |

**Example: rate limiting**

```rust
use clock_lib as clock;
use std::time::Duration;

let last_call = clock::now();
// ... later ...
let too_soon = clock::elapsed(last_call) < Duration::from_secs(1);
assert!(too_soon);
```

<br>

### `wall`

```rust,ignore
pub fn wall() -> Wall
```

Captures the current wall-clock time from the operating system. Use it for timestamps. Do **not** use it to measure elapsed time: wall-clock readings can jump in either direction (NTP steps, manual changes, VM suspend/resume or migration, leap-second handling). Daylight-saving time does not affect it; Unix time is UTC.

```rust
use clock_lib as clock;

let stamp = clock::wall();
assert!(stamp.unix_seconds() > 1_704_067_200);
println!("event at {stamp}"); // RFC 3339, UTC
```

<br>

### `unix`

```rust,ignore
pub fn unix() -> u64
```

Returns the current Unix time in whole seconds, like C's `time(NULL)` or PHP's `time()`. Returns `0` if the system clock is set before the Unix epoch.

```rust
let now = clock_lib::unix();
assert!(now > 1_704_067_200); // after 2024-01-01
```

<br>

### `unix_ms`

```rust,ignore
pub fn unix_ms() -> u128
```

Returns the current Unix time in whole milliseconds. Returns `0` if the system clock is set before the Unix epoch.

```rust
let event_id = format!("evt-{}", clock_lib::unix_ms());
assert!(event_id.starts_with("evt-1"));
```

<br>

### `unix_ns`

```rust,ignore
pub fn unix_ns() -> u128
```

Returns the current Unix time in whole nanoseconds. Returns `0` if the system clock is set before the Unix epoch. On Windows the value is always a multiple of 100.

```rust
let stamp = clock_lib::unix_ns();
assert!(stamp > 1_704_067_200_000_000_000);
```

<br>

## Types

### `Monotonic`

```rust,ignore
pub struct Monotonic(/* opaque */);
```

A captured monotonic instant. Monotonic time never goes backwards, which makes it the right choice for measuring elapsed time and building deadlines: rate limiting, timeouts, benchmarks, retry backoff. The absolute value carries no calendar meaning; it is only useful relative to another `Monotonic` from the same clock in the same process.

**Traits:** `Debug`, `Copy`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Add<Duration>`, `AddAssign<Duration>`, `Sub<Duration>`, `SubAssign<Duration>`, `Sub<Monotonic>` (output `Duration`), `From<Instant>`, and `Instant: From<Monotonic>`.

**No panics.** Unlike `std::time::Instant`, no `Monotonic` operation panics. Methods either return `Option` or saturate at the platform's representable range.

<br>

#### Monotonic: reading and measuring

```rust,ignore
pub fn now() -> Monotonic
pub fn elapsed(self) -> Duration
pub fn duration_since(self, earlier: Monotonic) -> Duration
pub fn checked_duration_since(self, earlier: Monotonic) -> Option<Duration>
pub fn saturating_duration_since(self, earlier: Monotonic) -> Duration
```

| Method | Behaviour |
| --- | --- |
| `now()` | Reads the OS monotonic clock. |
| `elapsed(self)` | Time from `self` to the **OS** clock's now. Only for OS readings; see [`elapsed`](#elapsed). |
| `duration_since(earlier)` | `self - earlier`, **zero** if `earlier` is later. Never panics. |
| `checked_duration_since(earlier)` | `Some(self - earlier)`, or `None` if `earlier` is later. |
| `saturating_duration_since(earlier)` | Same as `duration_since`. |

`duration_since` is pinned to saturating behaviour. `std::time::Instant::duration_since` also saturates today, but the standard library reserves the right to make it panic again; `clock-lib` does not.

```rust
use clock_lib::{Clock, ManualClock};
use std::time::Duration;

let clock = ManualClock::new();
let a = clock.now();
clock.advance(Duration::from_millis(5));
let b = clock.now();

assert_eq!(b.duration_since(a), Duration::from_millis(5));
assert_eq!(a.duration_since(b), Duration::ZERO);
assert_eq!(b.checked_duration_since(a), Some(Duration::from_millis(5)));
assert_eq!(a.checked_duration_since(b), None);
```

<br>

#### Monotonic: deadline arithmetic

```rust,ignore
pub fn checked_add(self, duration: Duration) -> Option<Monotonic>
pub fn checked_sub(self, duration: Duration) -> Option<Monotonic>
pub fn saturating_add(self, duration: Duration) -> Monotonic
pub fn saturating_sub(self, duration: Duration) -> Monotonic

// Operators (all saturating, never panic)
Monotonic + Duration -> Monotonic
Monotonic - Duration -> Monotonic
Monotonic - Monotonic -> Duration
Monotonic += Duration
Monotonic -= Duration
```

The `+` and `-` operators saturate rather than panic, so `now + Duration::MAX` is a valid "never" deadline. The first overflow in a process locates the platform's limit (about 1 µs); later ones reuse it and cost a few nanoseconds. Use `checked_add` / `checked_sub` when overflow should be an error.

```rust
use clock_lib::Monotonic;
use std::time::Duration;

let start = Monotonic::now();
let deadline = start + Duration::from_secs(30);
assert_eq!(deadline - start, Duration::from_secs(30));
assert_eq!(start - deadline, Duration::ZERO); // saturates

let never = start + Duration::MAX;            // no panic
assert!(never > deadline);
assert!(start.checked_add(Duration::MAX).is_none());

let mut t = start;
t += Duration::from_millis(10);
t -= Duration::from_millis(10);
assert_eq!(t, start);
```

<br>

#### Monotonic: conversions

```rust,ignore
impl From<Instant> for Monotonic
impl From<Monotonic> for Instant
```

Lossless in both directions. Use them to pass readings to APIs that take an `Instant` (`Receiver::recv_deadline`, `tokio::time::Instant::from_std`) or to build readings in a custom [`Clock`](#clock). The impls always use `std::time::Instant`; no feature changes them.

```rust
use clock_lib::Monotonic;
use std::time::{Duration, Instant};

let deadline: Instant = (Monotonic::now() + Duration::from_millis(50)).into();
let reading = Monotonic::from(deadline);
assert_eq!(Instant::from(reading), deadline);
```

<br>

### `Wall`

```rust,ignore
pub struct Wall(/* opaque */);
```

A captured wall-clock instant: calendar time in UTC. `Wall` is the right tool for timestamps in logs, audit records, and anything that has to line up with a calendar. It is the wrong tool for measuring elapsed time, because wall-clock readings can jump. For that, use [`Monotonic`](#monotonic).

**Traits:** `Debug`, `Display` (RFC 3339), `Copy`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `From<SystemTime>`, and `SystemTime: From<Wall>`; `Serialize` / `Deserialize` with the `serde` feature.

**Unix time** is UTC and does not count leap seconds. Daylight-saving time never changes it.

**Resolution.** `Wall` has the resolution of the platform `SystemTime`: 100 ns on Windows, 1 ns on Linux and macOS. Values built or moved at a finer resolution are truncated to the platform tick.

<br>

#### Wall: reading and constructing

```rust,ignore
pub const UNIX_EPOCH: Wall
pub fn now() -> Wall
pub fn from_unix_seconds(secs: u64) -> Option<Wall>
pub fn from_unix_millis(millis: u128) -> Option<Wall>
pub fn from_unix_nanos(nanos: u128) -> Option<Wall>
```

The constructors return `None` when the moment is not representable on the current platform (for example, beyond the year 30828 on Windows). They take the same types the `unix_*` accessors return, so values round-trip.

```rust
use clock_lib::Wall;

let launch = Wall::from_unix_seconds(1_700_000_000).expect("representable");
assert_eq!(launch.to_string(), "2023-11-14T22:13:20Z");

let precise = Wall::from_unix_millis(1_700_000_000_250).expect("representable");
assert_eq!(precise.unix_millis(), 1_700_000_000_250);

assert_eq!(Wall::UNIX_EPOCH.unix_seconds(), 0);
assert!(Wall::from_unix_seconds(u64::MAX).is_none());
```

<br>

#### Wall: Unix accessors

```rust,ignore
pub fn unix_seconds(self) -> u64
pub fn unix_millis(self) -> u128
pub fn unix_nanos(self) -> u128

pub fn unix_seconds_signed(self) -> i64
pub fn unix_millis_signed(self) -> i128
pub fn unix_nanos_signed(self) -> i128
```

| Accessor | Before the epoch | Rounding |
| --- | --- | --- |
| `unix_seconds` / `unix_millis` / `unix_nanos` | saturate to `0` | truncation |
| `unix_seconds_signed` / `unix_millis_signed` | negative values | toward negative infinity (`time_t` convention) |
| `unix_nanos_signed` | negative values | exact |

The unsigned accessors cannot tell a pre-epoch reading from the epoch itself; use the signed ones when that matters. None of them can overflow for any time a platform clock can represent.

```rust
use clock_lib::Wall;
use std::time::Duration;

let half_second_before = Wall::UNIX_EPOCH
    .checked_sub(Duration::from_millis(500))
    .expect("representable");

assert_eq!(half_second_before.unix_seconds(), 0);          // saturated
assert_eq!(half_second_before.unix_seconds_signed(), -1);  // floor
assert_eq!(half_second_before.unix_millis_signed(), -500);
assert_eq!(half_second_before.unix_nanos_signed(), -500_000_000);
```

<br>

#### Wall: arithmetic

```rust,ignore
pub fn checked_add(self, duration: Duration) -> Option<Wall>
pub fn checked_sub(self, duration: Duration) -> Option<Wall>
```

For computing timestamps such as a token's `exp` or a cookie's `Expires`. For deadlines inside the process, use [`Monotonic`](#monotonic-deadline-arithmetic) arithmetic; wall time can jump.

```rust
use clock_lib::Wall;
use std::time::Duration;

let issued = Wall::from_unix_seconds(1_700_000_000).expect("representable");
let expires = issued.checked_add(Duration::from_secs(3_600)).expect("representable");
assert_eq!(expires.unix_seconds(), 1_700_003_600);
```

<br>

#### Wall: conversions

```rust,ignore
impl From<SystemTime> for Wall
impl From<Wall> for SystemTime
```

Lossless in both directions. Use them for `File::set_modified`, HTTP date handling, or conversion into `chrono` / `time` types. The impls always use `std::time::SystemTime`; no feature changes them.

```rust
use clock_lib::Wall;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

let time = UNIX_EPOCH + Duration::from_secs(86_400);
let stamp = Wall::from(time);
assert_eq!(stamp.to_string(), "1970-01-02T00:00:00Z");
assert_eq!(SystemTime::from(stamp), time);
```

<br>

#### Wall: RFC 3339 formatting

```rust,ignore
impl Display for Wall
```

`Wall` formats as an RFC 3339 timestamp in UTC, with no dependencies and no allocation:

- No precision: the fractional part uses the shortest of 0, 3, 6 or 9 digits that is exact.
- `{:.N}` (0 to 9): exactly `N` fractional digits, **truncated** (never rounded into the next second). Precisions above 9 act as 9.
- Width, fill and alignment are honoured.
- Years outside `0000`–`9999` use the ISO 8601 expanded form (`+10000-01-01T00:00:00Z`), which is outside RFC 3339 proper.
- Leap seconds never appear (Unix time does not count them).

Parsing is out of scope; use `chrono`, `time` or `jiff` for that, and for time zones.

```rust
use clock_lib::Wall;

let stamp = Wall::from_unix_millis(1_700_000_000_250).expect("representable");
assert_eq!(stamp.to_string(), "2023-11-14T22:13:20.250Z");
assert_eq!(format!("{stamp:.0}"), "2023-11-14T22:13:20Z");
assert_eq!(format!("{stamp:.6}"), "2023-11-14T22:13:20.250000Z");
assert_eq!(format!("[{:>26}]", Wall::UNIX_EPOCH), "[      1970-01-01T00:00:00Z]");
```

<br>

#### Wall: serde

Requires the `serde` feature.

`Wall` serializes exactly like `std::time::SystemTime` under serde: a struct with `secs_since_epoch` (`u64`) and `nanos_since_epoch` (`u32`). A `Wall` field can replace a `SystemTime` field without changing the wire format, and either side reads data written by the other. As with `SystemTime`, readings before the Unix epoch cannot be serialized.

`Monotonic` deliberately has no serde support: a monotonic reading only means something inside the process that took it.

```rust,ignore
// Requires `features = ["serde"]` (and serde_json for this example).
// Compiled and run by the crate's rustdoc tests and tests/serde.rs.
use clock_lib::Wall;

let stamp = Wall::from_unix_millis(1_700_000_000_250).expect("representable");
let json = serde_json::to_string(&stamp)?;
assert_eq!(json, r#"{"secs_since_epoch":1700000000,"nanos_since_epoch":250000000}"#);
let back: Wall = serde_json::from_str(&json)?;
assert_eq!(back, stamp);
```

<br>

## Traits

### `Clock`

```rust,ignore
pub trait Clock: Send + Sync {
    fn now(&self) -> Monotonic;
    fn wall(&self) -> Wall;

    // Provided
    fn elapsed(&self, since: Monotonic) -> Duration {
        self.now().saturating_duration_since(since)
    }
}
```

A source of time. Time-driven code takes a `Clock` instead of calling the OS directly; production passes [`SystemClock`](#systemclock), tests pass [`ManualClock`](#manualclock).

| Method | Returns | Description |
| --- | --- | --- |
| `now(&self)` | [`Monotonic`](#monotonic) | Current monotonic reading. Must never go backwards. |
| `wall(&self)` | [`Wall`](#wall-1) | Current wall-clock reading. May jump. |
| `elapsed(&self, since)` | `Duration` | Time from `since` to this clock's now, zero if `since` is in the future. **Use this, not the free [`elapsed`](#elapsed), for readings from an injected clock.** |

**Forwarding implementations.** `Clock` is implemented for `&C`, `Box<C>` and `Arc<C>` where `C: Clock + ?Sized`. A borrowed, boxed or shared clock, including `&dyn Clock`, `Box<dyn Clock>` and `Arc<dyn Clock>`, can be passed wherever `C: Clock` is expected.

**Example: a time-driven primitive**

```rust
use clock_lib::{Clock, ManualClock, Monotonic};
use std::time::Duration;

struct RateLimiter<C: Clock> {
    clock: C,
    last: Option<Monotonic>,
    interval: Duration,
}

impl<C: Clock> RateLimiter<C> {
    fn try_acquire(&mut self) -> bool {
        let now = self.clock.now();
        match self.last {
            Some(last) if now - last < self.interval => false,
            _ => {
                self.last = Some(now);
                true
            }
        }
    }
}

let clock = ManualClock::new();
let mut limiter = RateLimiter { clock: &clock, last: None, interval: Duration::from_secs(1) };
assert!(limiter.try_acquire());
assert!(!limiter.try_acquire());
clock.advance(Duration::from_secs(1));
assert!(limiter.try_acquire());
```

**Example: implementing `Clock`**

Build readings from your own source with `Monotonic::from(Instant)` and `Wall::from(SystemTime)`:

```rust
use clock_lib::{Clock, Monotonic, Wall};
use std::time::{Duration, Instant, SystemTime};

/// Runs a fixed amount ahead of the system clock.
struct Ahead(Duration);

impl Clock for Ahead {
    fn now(&self) -> Monotonic {
        Monotonic::from(Instant::now()) + self.0
    }
    fn wall(&self) -> Wall {
        Wall::from(SystemTime::now() + self.0)
    }
}

let clock = Ahead(Duration::from_secs(10));
let start = clock.now();
assert!(clock.elapsed(start) < Duration::from_secs(10));
```

<br>

## Clock Implementations

### `SystemClock`

```rust,ignore
pub struct SystemClock;
pub const fn new() -> SystemClock
```

A zero-sized, `Copy` clock backed by the operating system: `now()` forwards to `Monotonic::now`, `wall()` to `Wall::now`. The constructor is `const`, so it can live in a `const` or `static`. It costs nothing over the free functions.

**Traits:** `Debug`, `Default`, `Copy`, `Clone`, `Clock`.

```rust
use clock_lib::{Clock, SystemClock};

const CLOCK: SystemClock = SystemClock::new();
let start = CLOCK.now();
let _took = CLOCK.elapsed(start);
```

<br>

### `ManualClock`

```rust,ignore
pub struct ManualClock { /* opaque */ }
```

A clock under the test's control. It takes its anchors at construction and then moves **only** when told to, so time-driven code can be exercised at exact, repeatable instants without `sleep`. Every method takes `&self` and is lock-free (relaxed atomics), so one clock can be shared between the test driver and the code under test, across threads.

**Traits:** `Debug`, `Default`, `Send`, `Sync`, `Clock`. Deliberately **not** `Clone`: two copies would drift apart. Share it with `&ManualClock` or `Arc<ManualClock>`, both of which implement `Clock`.

**What is deterministic.** Deltas are exact. Wall readings are fully reproducible when the clock starts from a fixed time ([`with_start`](#manualclock-construction)); `new()` anchors at the current OS time, so its absolute timestamps differ per run. Monotonic readings are opaque by design; only differences between them mean anything.

**Limits.** The shared offset saturates at `u64::MAX` ns (about 584 years) and never wraps, so the clock can never go backwards. The wall-only adjustment saturates at ±`i64::MAX` ns (about ±292 years). No method panics.

**Windows resolution.** Wall readings move in 100 ns ticks on Windows: `advance(123 ns)` moves `now()` by 123 ns but `wall()` by 100 ns. The error stays below one tick and never accumulates. Monotonic deltas are exact on every platform.

<br>

#### ManualClock: construction

```rust,ignore
pub fn new() -> ManualClock
pub fn with_start(start: Wall) -> ManualClock
impl Default for ManualClock // same as new()
```

```rust
use clock_lib::{ManualClock, Wall};
use std::time::Duration;

let start = Wall::from_unix_seconds(1_700_000_000).expect("representable");
let clock = ManualClock::with_start(start);
assert_eq!(clock.wall().to_string(), "2023-11-14T22:13:20Z");

clock.advance(Duration::from_millis(1_500));
assert_eq!(clock.wall().to_string(), "2023-11-14T22:13:21.500Z"); // same on every run
```

<br>

#### ManualClock: readings

```rust,ignore
pub fn now(&self) -> Monotonic
pub fn wall(&self) -> Wall
pub fn offset(&self) -> Duration
```

`now` and `wall` are inherent methods, so they work without importing `Clock`; the `Clock` impl forwards to them. `offset` is the total forward motion since construction (wall-only adjustments excluded). Reading the offset never triggers auto-advance.

```rust
use clock_lib::ManualClock;
use std::time::Duration;

let clock = ManualClock::new();
assert_eq!(clock.now(), clock.now());
clock.advance(Duration::from_secs(1));
clock.advance(Duration::from_secs(2));
assert_eq!(clock.offset(), Duration::from_secs(3));
```

<br>

#### ManualClock: moving time

```rust,ignore
pub fn advance(&self, by: Duration)
pub fn advance_to(&self, offset: Duration)
```

| Method | Effect |
| --- | --- |
| `advance(by)` | Moves both readings forward by `by`. Accumulates and saturates at `u64::MAX` ns, so `advance(Duration::MAX)` is a safe "expire everything". |
| `advance_to(offset)` | Moves the offset forward to `offset` if it is currently smaller; otherwise does nothing. The absolute counterpart to `advance`, and it can never move time backwards. |

```rust
use clock_lib::ManualClock;
use std::time::Duration;

let clock = ManualClock::new();
let start = clock.now();

clock.advance(Duration::from_secs(5));
assert_eq!(clock.now() - start, Duration::from_secs(5));

clock.advance_to(Duration::from_secs(10));
assert_eq!(clock.offset(), Duration::from_secs(10));
clock.advance_to(Duration::from_secs(3)); // already past: no-op
assert_eq!(clock.offset(), Duration::from_secs(10));

clock.advance(Duration::MAX);
clock.advance(Duration::MAX); // saturated, never wraps
assert_eq!(clock.offset(), Duration::from_nanos(u64::MAX));
```

<br>

#### ManualClock: moving wall time only

```rust,ignore
pub fn step_wall_forward(&self, by: Duration)
pub fn step_wall_back(&self, by: Duration)
pub fn set_wall(&self, to: Wall)
```

These move the wall reading **without** moving the monotonic reading, the way an NTP step, a manual clock change or a VM resume does. Use them to prove that code measures intervals with `Monotonic` and is not fooled by wall-clock jumps. The wall reading may cross the Unix epoch; the signed accessors report it.

`set_wall` computes the adjustment against the offset at the time of the call, so call it from the test driver, not concurrently with other threads advancing the same clock.

```rust
use clock_lib::{Clock, ManualClock, Wall};
use std::time::Duration;

let clock = ManualClock::with_start(Wall::from_unix_seconds(1_000).expect("representable"));
let began = clock.now();

clock.advance(Duration::from_secs(5));
clock.step_wall_back(Duration::from_secs(60)); // NTP correction

assert_eq!(clock.wall().unix_seconds(), 945);              // wall jumped back
assert_eq!(clock.elapsed(began), Duration::from_secs(5));  // monotonic did not

clock.set_wall(Wall::from_unix_seconds(1_800_000_000).expect("representable"));
assert_eq!(clock.wall().unix_seconds(), 1_800_000_000);
assert_eq!(clock.elapsed(began), Duration::from_secs(5));
```

<br>

#### ManualClock: auto-advance

```rust,ignore
pub fn set_auto_advance(&self, step: Duration)
pub fn auto_advance(&self) -> Duration
```

With a non-zero step, every reading (`now`, `wall`, `Clock::elapsed`) returns the current time and then moves the clock forward by `step`. Use it for code that polls the clock in a loop (spin-until-deadline, self-timing operations) when no second thread drives time. Each reading claims its own step atomically, so concurrent readings are all distinct. `Duration::ZERO` turns it off (the default).

```rust
use clock_lib::ManualClock;
use std::time::Duration;

let clock = ManualClock::new();
clock.set_auto_advance(Duration::from_millis(10));
let a = clock.now();
let b = clock.now();
assert_eq!(b - a, Duration::from_millis(10));

clock.set_auto_advance(Duration::ZERO);
assert_eq!(clock.now(), clock.now());
```

<br>

## Helpers

### `Deadline`

```rust,ignore
pub struct Deadline<C = SystemClock> { /* clock + expiry reading */ }

impl Deadline<SystemClock> {
    pub fn after(ttl: Duration) -> Self
}
impl<C: Clock> Deadline<C> {
    pub fn after_with(clock: C, ttl: Duration) -> Self
    pub const fn at(clock: C, expires_at: Monotonic) -> Self
    pub const fn expires_at(&self) -> Monotonic
    pub fn is_expired(&self) -> bool
    pub fn remaining(&self) -> Duration
    pub const fn clock(&self) -> &C
}
```

A monotonic deadline bound to the clock that measures it. It is a reading plus a clock, not a timer: nothing happens on expiry, you ask it. `after` uses the OS clock; `after_with` takes any `C: Clock` (`&ManualClock`, `Arc<dyn Clock>`, ...) so the code using it stays testable. The expiry saturates, so `Deadline::after(Duration::MAX)` never expires. A zero TTL is expired immediately. `remaining()` is zero once expired, so it can be passed straight to anything that takes a timeout.

**Traits:** `Debug`, `Clone`, and `Copy` when `C: Copy`.

```rust
use clock_lib::{Clock, Deadline, ManualClock};
use std::time::Duration;

fn wait_for<C: Clock>(clock: C, ttl: Duration, mut ready: impl FnMut() -> bool) -> bool {
    let deadline = Deadline::after_with(clock, ttl);
    while !deadline.is_expired() {
        if ready() {
            return true;
        }
    }
    false
}

// Production: Deadline::after(Duration::from_secs(5)).
// Test: every reading advances the clock one second, no sleeping.
let clock = ManualClock::new();
clock.set_auto_advance(Duration::from_secs(1));
assert!(!wait_for(&clock, Duration::from_secs(3), || false));

let clock = ManualClock::new();
let deadline = Deadline::after_with(&clock, Duration::from_secs(10));
clock.advance(Duration::from_secs(4));
assert_eq!(deadline.remaining(), Duration::from_secs(6));
```

<br>

### `Stopwatch`

```rust,ignore
pub struct Stopwatch<C = SystemClock> { /* clock + start + lap readings */ }

impl Stopwatch<SystemClock> {
    pub fn start() -> Self
}
impl<C: Clock> Stopwatch<C> {
    pub fn start_with(clock: C) -> Self
    pub fn elapsed(&self) -> Duration
    pub fn lap(&mut self) -> Duration
    pub fn restart(&mut self) -> Duration
    pub const fn started_at(&self) -> Monotonic
    pub const fn clock(&self) -> &C
}
```

Measures elapsed time with optional laps. `elapsed` is the total since the start (laps do not reset it). `lap` returns the time since the previous lap and starts a new one. `restart` returns the total and starts again from zero. `start` uses the OS clock; `start_with` takes any `C: Clock`.

**Traits:** `Debug`, `Clone`, and `Copy` when `C: Copy`.

```rust
use clock_lib::{ManualClock, Stopwatch};
use std::time::Duration;

let clock = ManualClock::new();
let mut watch = Stopwatch::start_with(&clock);

clock.advance(Duration::from_millis(120));
assert_eq!(watch.lap(), Duration::from_millis(120));
clock.advance(Duration::from_millis(80));
assert_eq!(watch.lap(), Duration::from_millis(80));
assert_eq!(watch.elapsed(), Duration::from_millis(200));

assert_eq!(watch.restart(), Duration::from_millis(200));
assert_eq!(watch.elapsed(), Duration::ZERO);
```

<br>

## Testing Patterns

**Take the clock as a parameter.** Generic `C: Clock` costs nothing at runtime. Use `&dyn Clock` or `Arc<dyn Clock>` where a concrete type parameter is inconvenient.

**Measure through the clock.** Inside code that takes a `Clock`, use `clock.now()`, `clock.wall()` and `clock.elapsed(reading)`. Never call `clock_lib::now()`, `clock_lib::elapsed()`, `Monotonic::now()`, `Monotonic::elapsed()` or `Wall::now()` there: they read the OS clock and bypass the injected one.

**Pin wall time for snapshot tests.** `ManualClock::with_start(Wall::from_unix_seconds(..))` makes every timestamp reproducible.

**Test the NTP case.** `step_wall_back` / `step_wall_forward` / `set_wall` let a test prove that interval logic uses monotonic time.

```rust
use clock_lib::{Clock, ManualClock, Monotonic};
use std::time::Duration;

/// Code under test: measures with the injected clock.
fn session_expired<C: Clock>(clock: &C, started: Monotonic) -> bool {
    clock.elapsed(started) >= Duration::from_secs(30 * 60)
}

let clock = ManualClock::new();
let started = clock.now();

clock.step_wall_forward(Duration::from_secs(24 * 3_600)); // wall clock jumps a day
assert!(!session_expired(&clock, started));                // not fooled

clock.advance(Duration::from_secs(30 * 60));
assert!(session_expired(&clock, started));
```

<br>

## Constants

### `VERSION`

```rust,ignore
pub const VERSION: &str
```

The crate version string, set by Cargo at build time. Available with and without the `std` feature.

```rust
assert!(clock_lib::VERSION.starts_with("1."));
```
