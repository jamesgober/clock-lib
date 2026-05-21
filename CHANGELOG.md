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

[Unreleased]: https://github.com/jamesgober/clock-lib/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jamesgober/clock-lib/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/clock-lib/releases/tag/v0.1.0
