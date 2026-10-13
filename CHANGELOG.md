# Changelog

All notable changes to `clock-lib` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [1.1.0] - 2026-10-13

Fixes the `ManualClock::advance` overflow wrap and makes the mock clock and
the reading types usable for real-world code: conversions, deadline
arithmetic, a fixed-start and NTP-step-capable `ManualClock`,
`Clock::elapsed`, `Deadline`/`Stopwatch`, RFC 3339 formatting, and an
opt-in `serde` feature. Every change is additive; no public item was
removed or changed signature. MSRV stays 1.85.

### Added

- **`Monotonic` deadline arithmetic.** `checked_add`, `checked_sub`,
  `saturating_add`, `saturating_sub`, and the `+`, `-`, `+=`, `-=`
  operators with `Duration`, plus `Monotonic - Monotonic -> Duration`.
  Unlike `std::time::Instant`, none of them panic: operators saturate at
  the platform's representable range, so `now + Duration::MAX` is a valid
  "never" deadline.
- **Conversions.** `From<Instant> for Monotonic`, `From<Monotonic> for
  Instant`, `From<SystemTime> for Wall`, `From<Wall> for SystemTime`. The
  `Clock` trait can now be implemented from any time source, and readings
  can be handed to std/tokio APIs that take `Instant` or `SystemTime`.
- **`Wall` constructors and arithmetic.** `Wall::UNIX_EPOCH`,
  `Wall::from_unix_seconds` / `from_unix_millis` / `from_unix_nanos`
  (returning `Option`; they accept the same types the accessors return),
  `Wall::checked_add`, `Wall::checked_sub`.
- **Signed Unix accessors.** `Wall::unix_seconds_signed` (`i64`),
  `unix_millis_signed` and `unix_nanos_signed` (`i128`), rounding toward
  negative infinity, so pre-epoch readings are no longer indistinguishable
  from the epoch.
- **RFC 3339 formatting.** `impl Display for Wall`: UTC, no dependencies,
  no allocation; `{:.N}` selects fractional digits (truncated), and width,
  fill and alignment are honoured.
- **`Clock::elapsed(&self, since)`**, a provided method that measures with
  the injected clock. The `&C`, `Box<C>` and `Arc<C>` impls forward it.
- **`impl Clock for Box<C>`** (`C: Clock + ?Sized`), so `Box<dyn Clock>`
  satisfies `C: Clock` bounds.
- **`ManualClock` control surface.** `with_start(Wall)` for reproducible
  wall timestamps; inherent `now()` / `wall()` (no trait import needed);
  `advance_to(offset)` (forward-only absolute set); `step_wall_forward`,
  `step_wall_back` and `set_wall` to move wall time without moving
  monotonic time (simulated NTP steps); `set_auto_advance` /
  `auto_advance` to advance the clock on every reading.
- **`Deadline<C = SystemClock>`**: `after`, `after_with`, `at`,
  `expires_at`, `is_expired`, `remaining`, `clock`.
- **`Stopwatch<C = SystemClock>`**: `start`, `start_with`, `elapsed`,
  `lap`, `restart`, `started_at`, `clock`.
- **Feature `serde`** (off by default): `Serialize` / `Deserialize` for
  `Wall`, in exactly the format serde uses for `std::time::SystemTime`, so
  the two are wire-compatible. New optional dependency: `serde` 1.0 with
  only its `std` feature (no derive macros). Justification: the de-facto
  serialization framework, mature and maintained, MSRV 1.56, and
  timestamps routinely end up in logs, events and database rows.
- **Tests.** Unit tests in every module; new integration suites for
  `ManualClock`, `Deadline`, `Stopwatch`, concurrency (contended
  `advance`, saturation, auto-advance uniqueness, wall steps), property
  tests (proptest: offset arithmetic, accessor consistency, signed
  rounding, RFC 3339 against a naive reference over the years 1601–9999,
  `Monotonic` arithmetic), `serde` parity with `SystemTime`, and a
  `--no-default-features` test. Loom models for every lock-free
  `ManualClock` path. README and `docs/API.md` are compiled as doctests.
  New dev-dependencies: `proptest` 1 (std only), `serde_json` 1, and
  `loom` 0.7 (only under `--cfg loom`).
- **Benchmarks** for deadline arithmetic, `Deadline` / `Stopwatch`,
  RFC 3339 formatting, the signed accessors, the new `ManualClock` paths,
  and `Box<dyn Clock>`.
- **CI**: format; clippy and tests with all, default and no default
  features on Linux, macOS and Windows for both stable and 1.85.0; beta
  (allowed to fail); a docs.rs build on nightly with `--cfg docsrs`;
  `wasm32-unknown-unknown` compile checks on stable and 1.85;
  a bare-metal `no_std` build; loom; cargo-semver-checks; audit; deny.
  `.github/dependabot.yml` for Cargo and GitHub Actions.

### Changed

- `Monotonic::duration_since` is now implemented as
  `saturating_duration_since` instead of delegating to
  `Instant::duration_since`. The result is identical on every supported
  toolchain (std saturates since 1.60); the difference is that a future
  std change back to panicking can no longer reach users.
- `ManualClock` readings saturate at the platform's representable range
  instead of panicking (the 1.0 docs listed a theoretical panic).
- Saturating `Monotonic` arithmetic caches the platform's `Instant` limits
  after the first overflow, so `now + Duration::MAX` and
  `Deadline::after(Duration::MAX)` cost a few nanoseconds instead of a
  ~0.5–0.8 µs binary search each time.
- `Deadline` and `Stopwatch` implement `Debug` for every clock type,
  including `Box<dyn Clock>` and `Arc<dyn Clock>` (the clock prints as its
  type name).
- **Not included: a `web-time` feature.** A pre-release review found the
  draft design non-additive: on `wasm32-unknown-unknown` it changed the
  types of the public `From` impls, so enabling it anywhere in a dependency
  graph broke other crates' conversions. A lossless `From<std::time::Instant>`
  cannot exist when readings are backed by the browser clock, so an additive
  design needs a different internal representation for `Monotonic`. It is
  deferred to a later release; on `wasm32-unknown-unknown` the OS-clock
  reads still panic (documented). The `From` impls always use the
  `std::time` types, and a test pins that under every feature set.
- Performance: the OS-clock paths are unchanged and within noise of raw
  `std::time`. `ManualClock::wall` went from ~0.8 ns to ~3.4 ns and
  `ManualClock::now` from ~0.78 ns to ~0.85 ns, the cost of the
  wall-only adjustment and auto-advance checks (one extra relaxed load
  each). The saturating `advance` costs the same as the old `fetch_add`.
  Numbers and methodology are in `docs/PERFORMANCE.md`.
- `ManualClock` lives in its own module (`src/manual.rs`); its public path
  is unchanged. Its `Debug` output now prints the offsets as integers and
  includes the wall-skew and auto-advance fields.
- The `Clock`, free-function and `Monotonic::elapsed` docs now state which
  calls read the operating-system clock, and that readings from an
  injected clock must be measured with `Clock::elapsed`.
- Removed the `no-std` crates.io category and corrected the `no_std`
  wording everywhere: without `std` the crate compiles but exposes only
  `VERSION`.
- The published package now uses an `include` list: source, tests,
  benches, `README.md`, `CHANGELOG.md`, `docs/API.md` and the licenses.
  Repository tooling (REPS, CI and lint configs, release notes) is no
  longer shipped.
- Benchmarks: criterion 0.5 → 0.7 (the newest line that supports MSRV
  1.85) with default features off, and `std::hint::black_box` instead of
  `criterion::black_box`. Bench inputs now pass through `black_box`, so
  loop-invariant conversions are no longer hoisted out of the measurement.
- `rust-toolchain.toml` keeps the REPS-mandated 1.85.0 pin for local
  builds, but every CI job now sets `RUSTUP_TOOLCHAIN`, so the stable legs
  run real stable instead of silently running 1.85.0.
- CI actions: `actions/checkout@v7`; `dtolnay/rust-toolchain` pinned to a
  commit SHA instead of `@master`; workflow-level
  `permissions: contents: read`; a concurrency group.
- `rustfmt.toml` drops the nightly-only `group_imports` and
  `imports_granularity` keys that stable rustfmt ignored with warnings.
- `deny.toml` no longer warns about allowlisted licenses that are absent
  from the graph.
- Removed the obsolete `#![doc(html_root_url)]` and the redundant `[lib]`
  section of `Cargo.toml`.

### Fixed

- **`ManualClock::advance` wrapped on cumulative overflow.** It used
  `AtomicU64::fetch_add`, so a large total (for example
  `advance(Duration::MAX)` twice) wrapped the offset and moved `now()` and
  `wall()` backwards, breaking the monotonic contract and contradicting
  the documented saturation. It is now a lock-free compare-and-swap loop
  with `saturating_add`.
- Docs: `Monotonic::duration_since` was documented to panic when the
  arguments are reversed; it saturates to zero.
- Docs: daylight-saving time was listed as a cause of wall-clock jumps.
  Unix time is UTC; the real causes (NTP steps, manual changes, VM
  suspend/migration, leap-second handling) are now listed.
- Docs: `ManualClock` was described as "fully deterministic". Deltas are;
  absolute wall readings now are too, with `with_start`.
- Docs: Windows' 100 ns wall-clock granularity (including in
  `ManualClock::wall`) and per-platform clock resolution are documented,
  and tests cover the bounded, non-accumulating `ManualClock` wall error.
- Docs: `wasm32-unknown-unknown` panics (std has no clock there) are
  documented.
- Docs: `docs/PERFORMANCE.md` no longer claims an enforced CI benchmark
  gate or identical assembly, and describes the Linux vDSO path correctly.
- `cargo test --no-default-features` did not compile. Every test file is
  now gated on the features it needs, and CI runs that configuration.
- `docs/API.md` signature blocks are marked `ignore`, and its examples no
  longer show `# ` hidden lines on GitHub.
- CHANGELOG comparison links pointed at `v0.1.0` and `v0.2.0` tags that
  were never created; they now point at the corresponding commits.

### Security

- Recorded: the dev-only lockfile refresh for RUSTSEC-2026-0204
  (`crossbeam-epoch` 0.9.18 → 0.9.21, reached only through criterion)
  landed after 1.0.0. With criterion's default features now off,
  `crossbeam-*` is no longer in the dependency graph at all. Downstream
  users were never affected: the crate has no runtime dependencies by
  default, and a library's lockfile is not used by dependents.
- CI follows least privilege (`permissions: contents: read`), pins the
  toolchain action to a commit, and dependabot now watches Cargo and
  GitHub Actions dependencies.

---

## [1.0.0] - 2026-05-21

The stable release. The public API is now committed for the 1.x line: no
breaking changes without a major version bump.

### Added

- Release notes at `docs/release/v1.0.0.md` documenting the stable
  surface, performance posture, engineering posture, and out-of-scope
  boundaries.

### Changed

- All install snippets, version references, and the lib.rs crate
  documentation updated to `1.0`.
- `docs/PERFORMANCE.md` baseline label set to `1.0.0`; the
  performance-critical paths have not changed since the 0.4.0
  measurement run, so the recorded numbers carry forward.
- README opening paragraph reflects `#![forbid(unsafe_code)]` and links
  to the performance document directly.
- Minor prose tightening across crate-level documentation.

### Public API

The 1.0 surface is the union of every prior 0.x addition:

- Tier-1 free functions: `now`, `elapsed`, `wall`, `unix`, `unix_ms`,
  `unix_ns`.
- Types: `Monotonic`, `Wall`.
- Trait: `Clock` (with blanket impls for `Arc<C>` and `&C`).
- Implementations: `SystemClock`, `ManualClock`.
- Constants: `VERSION`.

---

## [0.5.0] - 2026-05-21

### Added

- CI job that builds the crate against `thumbv7em-none-eabihf` (and the
  host with `--no-default-features`) to keep the `no_std` path honest.
- Feature-flag documentation in `src/lib.rs`, `README.md`, and
  `docs/API.md` describing the `std`/`no_std` split and what's available
  in each mode.
- Cross-doc navigation: every doc page now links to `README`, `API.md`,
  `PERFORMANCE.md`, and `GUIDELINES.md`.

### Changed

- `docs/PERFORMANCE.md` reproduction commands use a generic `main`
  baseline name rather than a version-coupled one, and the
  REPS-mandated 5%-regression gate is called out explicitly.
- README "Lean & Correct" section reflects `#![forbid(unsafe_code)]`
  and the verified bare-metal `no_std` build.

---

## [0.4.0] - 2026-05-21

### Added

- Real benchmark suite at `benches/clock_bench.rs` (criterion).
  Seven groups covering monotonic reads, wall reads, unix conversions,
  unix helpers, elapsed-time measurement, `ManualClock` operations, and
  trait-object dispatch.
- `docs/PERFORMANCE.md` documenting the methodology, baseline numbers,
  and the verified zero-overhead claim against raw `std::time`.
- Documentation link from `README.md` to the performance page.

### Changed

- Removed the placeholder bench function — the suite is now real.

---

## [0.3.0] - 2026-05-21

### Added

- `Clock` trait (`Send + Sync`) with `now() -> Monotonic` and
  `wall() -> Wall` methods. Lets time-driven code take an injected source
  of time and become deterministically testable.
- `SystemClock` &mdash; zero-sized, `Copy`, `const`-constructible
  production implementation backed by the OS.
- `ManualClock` &mdash; lock-free, atomic-offset test implementation. Tests
  advance time forward in arbitrary increments without calling
  `thread::sleep`. Methods: `new`, `advance`, `offset`.
- Blanket `Clock` implementations for `Arc<C: Clock>` and `&C: Clock` so
  shared and borrowed clocks work everywhere.
- Integration tests covering deterministic-time test patterns
  (`tests/clock.rs`).
- Doctests on every public item in the new module.
- API reference section for the `Clock` trait, `SystemClock`, and
  `ManualClock` in `docs/API.md`.
- Release notes at `docs/release/v0.3.0.md`.

### Changed

- `Monotonic.0` and `Wall.0` fields raised from private to `pub(crate)`
  so the new `ManualClock` can construct synthetic readings. The fields
  remain inaccessible outside the crate.

---

## [0.2.1] - 2026-05-21

### Added

- `#![forbid(unsafe_code)]` at the crate root, promoting the no-`unsafe`
  promise to a compiler-enforced guarantee that downstream code cannot
  override.
- Lint denies for `warnings` and `clippy::unreachable`, completing the
  REPS-required lint stack.
- Pinned Rust toolchain via `rust-toolchain.toml`.
- Committed `Cargo.lock` for reproducible builds across environments.
- `deny.toml` with license allowlist, yanked-crate ban, and wildcard ban.
- `cargo audit` and `cargo deny check` jobs in CI.
- `.gitattributes` to normalize line endings across platforms (fixes a
  CRLF/LF mismatch that broke `cargo fmt --check` on Windows runners).

### Changed

- CI: `actions/checkout` bumped to `v5` (Node.js 20 deprecation).
- CI: Security audit job now uses pre-built `cargo-audit` via
  `taiki-e/install-action`, sidestepping both the Node.js 20 deprecation
  and the audit-tool MSRV mismatch with the pinned 1.85 toolchain.
- Integration tests renamed to the REPS
  `test_<subject>_<condition>_<expected>` convention.

### Security

- `cargo audit` and `cargo deny` now run on every push, blocking merges on
  any RustSec advisory or license/source policy violation.

---

## [0.2.0] - 2026-05-21

### Added

- `Monotonic` reading type wrapping the OS monotonic clock, with `now`,
  `elapsed`, `duration_since`, `checked_duration_since`, and
  `saturating_duration_since`.
- `Wall` reading type wrapping the OS real-time clock, with `now`,
  `unix_seconds`, `unix_millis`, and `unix_nanos`. Pre-epoch system clocks
  saturate to `0` rather than panicking or wrapping.
- Tier-1 free functions: `now`, `elapsed`, `wall`, `unix`, `unix_ms`,
  `unix_ns`.
- Integration tests (`tests/monotonic.rs`, `tests/wall.rs`, `tests/smoke.rs`)
  and doctests on every public item.
- Full API reference at `docs/API.md` covering every public type and
  function with multiple examples.
- Release notes at `docs/release/v0.2.0.md`.

### Changed

- **Raised MSRV to 1.85** to match the Rust 2024 edition declared in
  `Cargo.toml`. The previous `rust-version = "1.75"` was inconsistent with
  `edition = "2024"` and prevented the crate from compiling.
- Bumped `clippy.toml` `msrv` and the CI matrix to `1.85` accordingly.

---

## [0.1.0] - 2026-05-21

### Added

- Initial scaffold and repository bootstrap.
- REPS compliance baseline.
- CI for Linux/macOS/Windows on stable and MSRV.
- Project documentation framework.

[Unreleased]: https://github.com/jamesgober/clock-lib/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/jamesgober/clock-lib/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/jamesgober/clock-lib/compare/v0.5.0...v1.0.0
[0.5.0]: https://github.com/jamesgober/clock-lib/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/jamesgober/clock-lib/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/jamesgober/clock-lib/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/jamesgober/clock-lib/compare/8a20c338c23d3671b72fc5cd5f1bbb3d97758df4...v0.2.1
[0.2.0]: https://github.com/jamesgober/clock-lib/compare/10a92d34e13afcd3d5faea194090d74803f78667...8a20c338c23d3671b72fc5cd5f1bbb3d97758df4
[0.1.0]: https://github.com/jamesgober/clock-lib/commit/10a92d34e13afcd3d5faea194090d74803f78667
