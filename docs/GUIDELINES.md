<h1 align="center">
    <b>clock-lib</b><br>
    <sub><sup>DEVELOPER GUIDELINES</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;&mdash;&nbsp;</span>
        <span>GUIDELINES</span>
        <span>&nbsp;&mdash;&nbsp;</span>
        <a href="./API.md" title="API Reference"><b>API</b></a>
    </sup>
</div>
<br>

This document describes the engineering standards this project is built to.

## Standards

- **MSRV:** Rust 1.85
- **Edition:** 2024
- **License:** Apache-2.0 OR MIT (dual)
- **Cross-platform:** Linux, macOS, Windows

## Quality Bar

- **Zero `unsafe`** in the public API.
- **No panics on the hot path** &mdash; no `unwrap`, `expect`, `todo`, or `unimplemented` in library code.
- **Layered API** &mdash; a one-line entry point covers the common case; advanced control is available but never required.
- **Documented public surface** &mdash; every public item carries documentation and at least one example.
- **Benchmark-backed claims** &mdash; performance statements are verified by committed benchmarks, not asserted.

## Contributing

Contributions are welcome under the project's dual license. Please ensure:

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --all-targets --all-features -- -D warnings` is clean.
3. `cargo test --all-features` passes.
4. New public items include documentation and examples.
5. Documentation and the API reference are updated alongside code changes.