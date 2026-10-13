<h1 align="center">
    <b>clock-lib</b><br>
    <sub><sup>DEVELOPER GUIDELINES</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./API.md" title="API Reference"><b>API</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./PERFORMANCE.md" title="Performance"><b>PERFORMANCE</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <span>GUIDELINES</span>
    </sup>
</div>
<br>

This document describes the engineering standards this project is built to.

## Standards

- **MSRV:** Rust 1.85 (declared in `Cargo.toml`, built and tested in CI)
- **Edition:** 2024
- **License:** Apache-2.0 OR MIT (dual)
- **Cross-platform:** Linux, macOS, Windows; compile checks on `wasm32-unknown-unknown` and, without `std`, on `thumbv7em-none-eabihf`
- **Additive features:** a feature may add items or impls, never change the types of existing ones. `tests/clock.rs` pins the `From` impls to the `std::time` types, and CI runs it under every feature set.
- **Dependencies:** none by default. Optional dependencies live behind non-default features and are justified in `Cargo.toml` and the CHANGELOG.

## Quality Bar

- **No `unsafe`** &mdash; `#![forbid(unsafe_code)]` at the crate root.
- **No panics in the public API** &mdash; no `unwrap`, `expect`, `todo`, or `unimplemented` in library code, and no operation that panics on overflow: arithmetic is `checked_*` or saturating.
- **Layered API** &mdash; a one-line entry point covers the common case; advanced control is available but never required.
- **Documented public surface** &mdash; every public item carries documentation and at least one example. README and `docs/API.md` examples are compiled as doctests.
- **Benchmark-backed claims** &mdash; performance statements are verified by committed benchmarks, re-measured for each release, not asserted.

## Toolchain

`rust-toolchain.toml` pins the MSRV (1.85.0), as REPS requires, so a local build proves the code compiles for every supported user. CI does not inherit the pin for its other legs: every job sets `RUSTUP_TOOLCHAIN`, which overrides the file, so the stable, beta and nightly jobs really run those toolchains. Locally, use `cargo +stable <command>` to check against newer toolchains.

## Local Gate

Run before every release (RELEASE_WORKFLOW Phase 0):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo +stable clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test
cargo test --no-default-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo build --no-default-features --target thumbv7em-none-eabihf
cargo +stable clippy --target wasm32-unknown-unknown --all-features -- -D warnings
RUSTFLAGS="--cfg loom" cargo test --lib --release loom
cargo audit
cargo deny check
cargo +stable semver-checks          # needs rustc >= 1.91
cargo publish --dry-run
```

**Mutation testing.** The crate root carries `#![deny(warnings)]` (REPS). Under cargo-mutants that turns many mutants into compile errors ("unviable") instead of testing them, so `.cargo/mutants.toml` sets `cap_lints = true` (plus `all_features`, and excludes the loom-only module):

```bash
cargo mutants   # settings in .cargo/mutants.toml
```

**Benchmarks.** `cargo bench --bench clock_bench -- --save-baseline main` before a change, `-- --baseline main` after. See [PERFORMANCE](./PERFORMANCE.md).

## Contributing

Contributions are welcome under the project's dual license. Please ensure:

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --all-targets --all-features -- -D warnings` is clean.
3. `cargo test --all-features`, `cargo test` and `cargo test --no-default-features` pass.
4. New public items include documentation and examples.
5. Documentation, the API reference and the CHANGELOG are updated alongside code changes.
