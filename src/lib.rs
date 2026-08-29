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

pub mod error;
pub mod value;

pub use error::{ErrorKind, ParseError};
pub use value::{Number, Value};

mod lexer;
mod parser;
mod query;
mod serializer;

/// Parse a JSON document.
///
/// The input is bytes rather than `&str` so that a file which is not valid
/// UTF-8 gets a diagnostic pointing at the offending byte, instead of failing
/// before parsing begins.
///
/// # Errors
///
/// Returns the first problem found, with the byte offset where it was found.
/// Parsing stops there; this is not an error-recovering parser.
pub fn parse(input: &[u8]) -> Result<Value, ParseError> {
    // Validating UTF-8 once, here, is what allows every scanner downstream to
    // treat a byte at or above 0x80 as part of a well-formed sequence without
    // re-checking it.
    if let Err(e) = core::str::from_utf8(input) {
        return Err(ParseError::new(
            ErrorKind::InvalidUtf8 {
                valid_up_to: e.valid_up_to(),
            },
            input,
            e.valid_up_to(),
        ));
    }
    parser::parse_document(input)
}
/// How to lay out serialized output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Two-space indentation, one element per line: `jq`'s default.
    Pretty,
    /// No whitespace at all: `jq --compact-output`.
    Compact,
}

/// Write `value` to `out` as JSON text.
///
/// No trailing newline is written, so a caller printing several values decides
/// how to separate them.
///
/// # Errors
///
/// Passes through whatever `out` returns.
pub fn write<W: std::io::Write>(out: &mut W, value: &Value, style: Style) -> std::io::Result<()> {
    serializer::write_value(out, value, style, 0)
}

/// Render `value` as a JSON string.
#[must_use]
pub fn to_string(value: &Value, style: Style) -> String {
    let mut bytes = Vec::new();
    write(&mut bytes, value, style).expect("writing into a Vec cannot fail");
    String::from_utf8(bytes).expect("the serializer only ever emits valid UTF-8")
}
/// The filter language: [`Filter::compile`] turns jq-style text into something
/// [`Filter::run`] can apply to a [`Value`].
pub use query::{EvalError, Filter, FilterError, FilterErrorKind};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_whitespace_only_input_is_empty_input() {
        assert_eq!(parse(b"").unwrap_err().kind(), ErrorKind::EmptyInput);
        assert_eq!(
            parse(b" \t\r\n ").unwrap_err().kind(),
            ErrorKind::EmptyInput
        );
    }

    #[test]
    fn invalid_utf8_reports_where_decoding_stopped() {
        let e = parse(b"\"\xff\"").unwrap_err();
        assert_eq!(e.kind(), ErrorKind::InvalidUtf8 { valid_up_to: 1 });
        assert_eq!(e.offset(), 1);
    }

    #[test]
    fn position_advances_across_newlines() {
        let e = ParseError::new(ErrorKind::UnexpectedEof, b"[\n  1", 5);
        assert_eq!((e.line(), e.column()), (2, 4));
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        let e = ParseError::new(ErrorKind::UnexpectedEof, "\"\u{e9}\"".as_bytes(), 4);
        assert_eq!(e.column(), 4);
    }
}
