//! A JSON parser, serializer and jq-style query engine written against the
//! Rust standard library and nothing else.
//!
//! The library is deliberately separate from the `jaq-lite` binary. Integration
//! tests under `tests/` can only link against a library target, and the
//! conformance harness that scores this crate against the JSONTestSuite corpus
//! is such a test, so a binary-only crate would not have been testable in the
//! way this project needs.
//!
//! Design commitments, stated here because they constrain every module:
//!
//! - No third-party code. `Cargo.toml` declares an empty `[dependencies]`
//!   table and `Cargo.lock` is committed so that the claim is checkable.
//! - Parsing follows RFC 8259 exactly, including the places where it is
//!   stricter than `f64::from_str`.
//! - Output is byte-compatible with `jq` wherever a choice exists, and every
//!   deliberate divergence is named in the README.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The crate version, read from `Cargo.toml` at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
