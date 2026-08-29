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

/// Parse a JSON document from raw bytes.
///
/// The input is bytes rather than `&str` on purpose. RFC 8259 defines a JSON
/// text as a stream of octets, and an ill-formed encoding is a parse error this
/// crate wants to report itself, with an offset, rather than have the caller
/// reject beforehand with no position information.
///
/// # Errors
///
/// Returns a [`ParseError`] describing the first violation found, with the byte
/// offset it was found at.
pub fn parse(input: &[u8]) -> Result<Value, ParseError> {
    let text = std::str::from_utf8(input).map_err(|e| {
        ParseError::new(
            ErrorKind::InvalidUtf8 {
                valid_up_to: e.valid_up_to(),
            },
            input,
            e.valid_up_to(),
        )
    })?;

    // RFC 8259 permits exactly four whitespace bytes between tokens. Note that
    // `char::is_whitespace` is a much larger set and would wrongly accept
    // several fixtures in the corpus, so it is not used here or anywhere else.
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() && matches!(bytes[at], b' ' | b'\t' | b'\n' | b'\r') {
        at += 1;
    }
    if at == bytes.len() {
        return Err(ParseError::new(ErrorKind::EmptyInput, input, at));
    }

    // The value grammar is not implemented at this commit, so no byte can begin
    // a value and this report is accurate rather than a placeholder.
    Err(ParseError::new(
        ErrorKind::UnexpectedByte { byte: bytes[at] },
        input,
        at,
    ))
}

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
