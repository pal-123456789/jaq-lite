//! Document structure: the entry point, arrays, and later objects.
//!
//! This is the only module that recurses, which is why the depth guard lives
//! here rather than being spread across the scanners.

use crate::error::{ErrorKind, ParseError};
use crate::lexer::Cursor;
use crate::value::Value;

/// How deeply containers may nest.
///
/// A limit is not optional. Two fixtures in the corpus nest a hundred thousand
/// arrays, and a recursive-descent parser without a guard meets them as a stack
/// overflow, which is a crash and not a rejection. RFC 8259 explicitly permits
/// an implementation to set such a limit. 128 is far beyond anything that
/// appears in real documents while keeping the deepest possible call stack
/// small enough to be uninteresting.
const MAX_DEPTH: u32 = 128;

/// Parse one complete document: optional whitespace, one value, optional
/// whitespace, then nothing.
pub(crate) fn parse_document(input: &[u8]) -> Result<Value, ParseError> {
    let mut cursor = Cursor::new(input);
    cursor.skip_whitespace();
    if cursor.is_eof() {
        return Err(cursor.error(ErrorKind::EmptyInput));
    }

    let value = parse_value(&mut cursor, 0)?;

    cursor.skip_whitespace();
    if !cursor.is_eof() {
        return Err(cursor.error(ErrorKind::TrailingData));
    }
    Ok(value)
}

fn parse_value(cursor: &mut Cursor<'_>, depth: u32) -> Result<Value, ParseError> {
    match cursor.peek() {
        Some(b'"') => cursor.scan_string().map(Value::String),
        Some(b'[') => parse_array(cursor, depth),
        Some(b't' | b'f' | b'n') => cursor.scan_literal(),
        Some(b'-' | b'0'..=b'9') => cursor.scan_number().map(Value::Number),
        None => Err(cursor.error(ErrorKind::UnexpectedEof)),
        // Objects arrive in the next commit. Until then `{` cannot begin a
        // value, so reporting an unexpected byte is accurate rather than a
        // placeholder that has to be removed later.
        Some(_) => Err(cursor.unexpected()),
    }
}

/// Parse an array, given that the cursor is sitting on the opening bracket.
///
/// The depth check happens before the bracket is consumed, so the reported
/// position is the bracket that would have been one level too deep.
fn parse_array(cursor: &mut Cursor<'_>, depth: u32) -> Result<Value, ParseError> {
    if depth >= MAX_DEPTH {
        return Err(cursor.error(ErrorKind::DepthLimitExceeded { limit: MAX_DEPTH }));
    }
    cursor.advance();

    let mut items = Vec::new();
    cursor.skip_whitespace();
    if cursor.peek() == Some(b']') {
        cursor.advance();
        return Ok(Value::Array(items));
    }

    loop {
        cursor.skip_whitespace();
        items.push(parse_value(cursor, depth + 1)?);
        cursor.skip_whitespace();
        match cursor.peek() {
            Some(b',') => cursor.advance(),
            Some(b']') => {
                cursor.advance();
                return Ok(Value::Array(items));
            }
            Some(_) => return Err(cursor.error(ErrorKind::ExpectedCommaOrClose { close: b']' })),
            None => return Err(cursor.error(ErrorKind::UnexpectedEof)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_DEPTH;
    use crate::{ErrorKind, Value, parse};

    fn error_kind(src: &str) -> ErrorKind {
        parse(src.as_bytes())
            .expect_err("should not have parsed")
            .kind()
    }

    #[test]
    fn whitespace_may_surround_the_document() {
        assert!(parse(b" \t\r\n null \n").is_ok());
    }

    #[test]
    fn a_second_value_is_trailing_data() {
        let err = parse(b"null null").expect_err("two values");
        assert_eq!(err.kind(), ErrorKind::TrailingData);
    }

    #[test]
    fn arrays_hold_their_elements_in_order() {
        let parsed = parse(b"[1, \"two\", null, [true]]").expect("valid array");
        let Value::Array(items) = parsed else {
            panic!("expected an array")
        };
        assert_eq!(items.len(), 4);
        assert_eq!(items[1], Value::String("two".to_owned()));
        assert_eq!(items[3], Value::Array(vec![Value::Bool(true)]));
    }

    #[test]
    fn an_empty_array_may_contain_whitespace() {
        assert_eq!(parse(b"[ \n ]").expect("empty"), Value::Array(Vec::new()));
    }

    #[test]
    fn the_separator_rules_are_exact() {
        // A trailing comma promises an element that is not there.
        assert_eq!(error_kind("[1,]"), ErrorKind::UnexpectedByte { byte: b']' });
        assert_eq!(error_kind("[,1]"), ErrorKind::UnexpectedByte { byte: b',' });
        assert_eq!(
            error_kind("[1 2]"),
            ErrorKind::ExpectedCommaOrClose { close: b']' }
        );
        assert_eq!(error_kind("[1"), ErrorKind::UnexpectedEof);
        assert_eq!(error_kind("[1]]"), ErrorKind::TrailingData);
    }

    #[test]
    fn nesting_is_allowed_up_to_the_limit_and_no_further() {
        let limit = MAX_DEPTH as usize;
        let deepest = format!("{}{}", "[".repeat(limit), "]".repeat(limit));
        assert!(
            parse(deepest.as_bytes()).is_ok(),
            "{limit} levels should parse"
        );

        let too_deep = format!("{}{}", "[".repeat(limit + 1), "]".repeat(limit + 1));
        assert_eq!(
            error_kind(&too_deep),
            ErrorKind::DepthLimitExceeded { limit: MAX_DEPTH }
        );
    }
}
