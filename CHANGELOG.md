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

[Unreleased]: https://github.com/jamesgober/clock-lib/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/jamesgober/clock-lib/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/jamesgober/clock-lib/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/jamesgober/clock-lib/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/jamesgober/clock-lib/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/clock-lib/releases/tag/v0.1.0
