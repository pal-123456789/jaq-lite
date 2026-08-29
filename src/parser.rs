//! Document structure: the entry point, and later arrays and objects.
//!
//! This is the only module that will recurse, which is why the depth guard will
//! live here rather than being spread across the scanners.

use crate::error::{ErrorKind, ParseError};
use crate::lexer::Cursor;
use crate::value::Value;

/// Parse one complete document: optional whitespace, one value, optional
/// whitespace, then nothing.
pub(crate) fn parse_document(input: &[u8]) -> Result<Value, ParseError> {
    let mut cursor = Cursor::new(input);
    cursor.skip_whitespace();
    if cursor.is_eof() {
        return Err(cursor.error(ErrorKind::EmptyInput));
    }

    let value = parse_value(&mut cursor)?;

    cursor.skip_whitespace();
    if !cursor.is_eof() {
        return Err(cursor.error(ErrorKind::TrailingData));
    }
    Ok(value)
}

fn parse_value(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    match cursor.peek() {
        Some(b't' | b'f' | b'n') => cursor.scan_literal(),
        Some(b'-' | b'0'..=b'9') => cursor.scan_number().map(Value::Number),
        None => Err(cursor.error(ErrorKind::UnexpectedEof)),
        // Strings, arrays and objects arrive in the next two commits. Until
        // they do, no byte can begin one, so reporting an unexpected byte here
        // is accurate rather than a placeholder that has to be removed later.
        Some(_) => Err(cursor.unexpected()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{ErrorKind, parse};

    #[test]
    fn whitespace_may_surround_the_document() {
        assert!(parse(b" \t\r\n null \n").is_ok());
    }

    #[test]
    fn a_second_value_is_trailing_data() {
        let err = parse(b"null null").expect_err("two values");
        assert_eq!(err.kind(), ErrorKind::TrailingData);
    }
}
