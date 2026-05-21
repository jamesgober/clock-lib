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

[Unreleased]: https://github.com/jamesgober/clock-lib/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/jamesgober/clock-lib/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/jamesgober/clock-lib/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/clock-lib/releases/tag/v0.1.0
