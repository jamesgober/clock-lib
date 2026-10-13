//! With default features disabled, the crate builds without `std` and
//! exposes only `VERSION`. This file is the integration test that runs in
//! that configuration (`cargo test --no-default-features`); the other test
//! files are gated on the `std` feature.
#![cfg(not(feature = "std"))]

#[test]
fn test_version_is_available_without_std() {
    assert_eq!(clock_lib::VERSION, env!("CARGO_PKG_VERSION"));
}
