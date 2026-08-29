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

pub mod color;
pub mod diag;
pub mod error;
pub mod value;

pub use color::{Ink, Paint};
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
    check_utf8(input)?;
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
    write_painted(out, value, style, Paint::Never)
}

/// Write `value` to `out` as JSON text, coloured the way `jq -C` colours it.
///
/// With [`Paint::Never`] the bytes are exactly those [`write()`] produces. That is
/// asserted rather than asserted-in-a-comment: a test strips every escape from a
/// coloured run and requires the remainder to equal the uncoloured run.
///
/// # Errors
///
/// Passes through whatever `out` returns.
pub fn write_painted<W: std::io::Write>(
    out: &mut W,
    value: &Value,
    style: Style,
    paint: Paint,
) -> std::io::Result<()> {
    serializer::write_value(out, value, style, paint, 0)
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
/// Reject input that is not UTF-8, before any scanning starts.
///
/// Validating once, here, is what allows every scanner downstream to treat a
/// byte at or above `0x80` as part of a well-formed sequence without re-checking
/// it. Both entry points need that guarantee, so the check lives in one place.
fn check_utf8(input: &[u8]) -> Result<(), ParseError> {
    if let Err(e) = core::str::from_utf8(input) {
        return Err(ParseError::new(
            ErrorKind::InvalidUtf8 {
                valid_up_to: e.valid_up_to(),
            },
            input,
            e.valid_up_to(),
        ));
    }
    Ok(())
}

/// Parse a stream of JSON documents, which is what `jq` reads from its input.
///
/// Documents need no separator beyond optional whitespace, so `{"a":1}{"a":2}`
/// is two of them and so is a file of newline-delimited JSON. Both readings were
/// measured against `jq` 1.8.1 rather than assumed.
///
/// Whitespace-only input yields nothing, and that is not an error. The stricter
/// [`parse`] calls the same input `EmptyInput`, and both are right: an empty
/// *document* is malformed, an empty *stream* is merely empty. Keeping the two
/// apart is also what keeps the `n_` corpus fixtures rejected -- `[][]` is one
/// of them, and it has to stay a failure for [`parse`] while being two
/// documents here.
///
/// Values are yielded as they are parsed, so a caller that prints them gets
/// output for every document preceding a syntax error, which is again what `jq`
/// does.
///
/// # Errors
///
/// Yields `Err` once for input that is not UTF-8, before any document is looked
/// at, and otherwise for the first document that fails to parse. Iteration stops
/// at that point.
pub fn parse_stream(input: &[u8]) -> impl Iterator<Item = Result<Value, ParseError>> + '_ {
    let (bad_utf8, documents) = match check_utf8(input) {
        Ok(()) => (None, Some(parser::Documents::new(input))),
        Err(error) => (Some(Err(error)), None),
    };
    bad_utf8.into_iter().chain(documents.into_iter().flatten())
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

#[cfg(test)]
mod stream_tests {
    use super::*;

    /// Render a whole stream, stopping at the first error.
    fn documents(input: &str) -> Result<Vec<String>, String> {
        let mut rendered = Vec::new();
        for document in parse_stream(input.as_bytes()) {
            match document {
                Ok(value) => rendered.push(to_string(&value, Style::Compact)),
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(rendered)
    }

    #[test]
    fn one_document_is_still_one_document() {
        assert_eq!(documents("[1,2]").unwrap(), vec!["[1,2]"]);
    }

    #[test]
    fn documents_need_no_separator_at_all() {
        // Every case here was run through jq 1.8.1 first, which reads each of
        // them as two inputs and prints two lines.
        assert_eq!(
            documents(r#"{"a":1}{"a":2}"#).unwrap(),
            vec![r#"{"a":1}"#, r#"{"a":2}"#]
        );
        assert_eq!(documents("1 2").unwrap(), vec!["1", "2"]);
        assert_eq!(documents("1\n2\n").unwrap(), vec!["1", "2"]);
    }

    #[test]
    fn an_empty_stream_is_fine_even_though_an_empty_document_is_not() {
        assert!(documents("").unwrap().is_empty());
        assert!(documents(" \t\r\n ").unwrap().is_empty());
        // jq exits 0 and says nothing for both of those. The single-document
        // entry point still refuses, which is what keeps
        // `n_structure_no_data.json` rejected by the conformance harness.
        assert!(parse(b"").is_err());
    }

    #[test]
    fn a_second_value_still_defeats_the_single_document_parser() {
        // `n_structure_double_array.json` is exactly this text, and the corpus
        // requires it to be rejected -- as a document. As a stream it is two.
        assert!(parse(b"[][]").is_err());
        assert_eq!(documents("[][]").unwrap(), vec!["[]", "[]"]);
    }

    #[test]
    fn everything_before_a_syntax_error_still_arrives() {
        // jq prints 1 and 2 before failing on the unfinished array, so values
        // have to be yielded as they are parsed rather than collected first.
        let mut seen = Vec::new();
        let mut failure = None;
        for document in parse_stream(b"1 2 [") {
            match document {
                Ok(value) => seen.push(to_string(&value, Style::Compact)),
                Err(error) => failure = Some(error),
            }
        }
        assert_eq!(seen, vec!["1", "2"]);
        assert!(failure.is_some(), "the unfinished array has to be reported");
    }

    #[test]
    fn a_syntax_error_ends_the_stream() {
        // Guessing where the next document was meant to begin would turn one
        // mistake into a cascade, so there is one error and then nothing.
        assert_eq!(parse_stream(b"[ 1 2 3").filter(Result::is_err).count(), 1);
    }

    #[test]
    fn input_that_is_not_utf8_is_rejected_before_any_document() {
        let bytes = [b'1', b' ', 0xff];
        let results: Vec<_> = parse_stream(&bytes).collect();
        assert_eq!(results.len(), 1, "not even the leading 1 should come back");
        assert!(results[0].is_err());
    }
}
