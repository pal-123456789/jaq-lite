//! Document structure: the entry point, arrays and objects.
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
        Some(b'{') => parse_object(cursor, depth),
        Some(b't' | b'f' | b'n') => cursor.scan_literal(),
        Some(b'-' | b'0'..=b'9') => cursor.scan_number().map(Value::Number),
        None => Err(cursor.error(ErrorKind::UnexpectedEof)),
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

/// Parse an object, given that the cursor is sitting on the opening brace.
fn parse_object(cursor: &mut Cursor<'_>, depth: u32) -> Result<Value, ParseError> {
    if depth >= MAX_DEPTH {
        return Err(cursor.error(ErrorKind::DepthLimitExceeded { limit: MAX_DEPTH }));
    }
    cursor.advance();

    let mut entries: Vec<(String, Value)> = Vec::new();
    cursor.skip_whitespace();
    if cursor.peek() == Some(b'}') {
        cursor.advance();
        return Ok(Value::Object(entries));
    }

    loop {
        cursor.skip_whitespace();
        // A key can only be a string. Saying so with its own error is worth it:
        // `{a: 1}` and `{"a" 1}` are different mistakes and the corpus has
        // fixtures for both.
        if cursor.peek() != Some(b'"') {
            return Err(if cursor.is_eof() {
                cursor.error(ErrorKind::UnexpectedEof)
            } else {
                cursor.error(ErrorKind::ExpectedObjectKey)
            });
        }
        let key = cursor.scan_string()?;

        cursor.skip_whitespace();
        if cursor.peek() != Some(b':') {
            return Err(if cursor.is_eof() {
                cursor.error(ErrorKind::UnexpectedEof)
            } else {
                cursor.error(ErrorKind::ExpectedColon)
            });
        }
        cursor.advance();

        cursor.skip_whitespace();
        let value = parse_value(cursor, depth + 1)?;
        insert(&mut entries, key, value);

        cursor.skip_whitespace();
        match cursor.peek() {
            Some(b',') => cursor.advance(),
            Some(b'}') => {
                cursor.advance();
                return Ok(Value::Object(entries));
            }
            Some(_) => return Err(cursor.error(ErrorKind::ExpectedCommaOrClose { close: b'}' })),
            None => return Err(cursor.error(ErrorKind::UnexpectedEof)),
        }
    }
}

/// Add a member, letting a repeated key overwrite the earlier value in place.
///
/// RFC 8259 says names *should* be unique and leaves the rest to the
/// implementation, but three `y_` fixtures contain duplicate keys and must
/// parse, so a policy is required rather than optional. Last value wins, kept at
/// the position where the key first appeared, which is what every
/// order-preserving object model does.
///
/// The scan is linear, which makes building an object with n members O(n^2) in
/// the worst case. That is a deliberate trade: preserving insertion order rules
/// out a plain `HashMap`, and carrying a side index would cost an allocation for
/// every object in the document to speed up a case -- duplicate keys -- that
/// almost never occurs. `position` rather than `iter_mut().find` because the
/// borrow from a mutable iterator would still be live in the `else` arm.
fn insert(entries: &mut Vec<(String, Value)>, key: String, value: Value) {
    match entries.iter().position(|(existing, _)| *existing == key) {
        Some(index) => entries[index].1 = value,
        None => entries.push((key, value)),
    }
}

/// A stream of documents: one value after another, separated by nothing more
/// than optional whitespace.
///
/// Kept deliberately apart from [`parse_document`], which has to go on refusing
/// a second value so that the `n_` corpus fixtures ending in one stay rejected.
pub(crate) struct Documents<'a> {
    /// Position in the input, carried across documents.
    cursor: Cursor<'a>,
    /// Set once a document has failed to parse.
    ///
    /// After a syntax error the cursor sits on the offending byte, and there is
    /// no way to know where the next document was meant to start. Resuming would
    /// turn one mistake into a cascade of them, so the stream ends instead --
    /// which is also what `jq` does, measured.
    stopped: bool,
}

impl<'a> Documents<'a> {
    /// Start reading documents from the beginning of `input`.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(input),
            stopped: false,
        }
    }
}

impl Iterator for Documents<'_> {
    type Item = Result<Value, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped {
            return None;
        }
        self.cursor.skip_whitespace();
        if self.cursor.is_eof() {
            return None;
        }
        match parse_value(&mut self.cursor, 0) {
            Ok(value) => Some(Ok(value)),
            Err(error) => {
                self.stopped = true;
                Some(Err(error))
            }
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

    fn members(src: &str) -> Vec<(String, Value)> {
        match parse(src.as_bytes()).expect(src) {
            Value::Object(entries) => entries,
            other => panic!("expected an object, got {other:?}"),
        }
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
        let parsed = parse(b"[1, \"two\", null, [true], {}]").expect("valid array");
        let Value::Array(items) = parsed else {
            panic!("expected an array")
        };
        assert_eq!(items.len(), 5);
        assert_eq!(items[1], Value::String("two".to_owned()));
        assert_eq!(items[3], Value::Array(vec![Value::Bool(true)]));
        assert_eq!(items[4], Value::Object(Vec::new()));
    }

    #[test]
    fn an_empty_container_may_contain_whitespace() {
        assert_eq!(parse(b"[ \n ]").expect("empty"), Value::Array(Vec::new()));
        assert_eq!(parse(b"{ \n }").expect("empty"), Value::Object(Vec::new()));
    }

    #[test]
    fn objects_keep_the_order_they_were_written_in() {
        let entries = members(r#"{"z": 1, "a": 2, "m": 3}"#);
        let keys: Vec<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, ["z", "a", "m"], "members were reordered");
    }

    #[test]
    fn a_repeated_key_keeps_its_first_position_and_its_last_value() {
        let entries = members(r#"{"b": 1, "a": 2, "b": 3}"#);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "b");
        assert_eq!(entries[1].0, "a");
        let Value::Number(last) = &entries[0].1 else {
            panic!("expected a number")
        };
        assert_eq!(last.as_str(), "3", "the earlier value should have lost");
    }

    #[test]
    fn a_key_may_contain_anything_a_string_may() {
        let entries = members(r#"{"foo\u0000bar": 42, "": 1}"#);
        assert_eq!(entries[0].0, "foo\u{0}bar");
        assert_eq!(entries[1].0, "", "an empty key is a legal key");
    }

    #[test]
    fn the_array_separator_rules_are_exact() {
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
    fn the_object_grammar_names_the_specific_mistake() {
        assert_eq!(error_kind(r#"{a: 1}"#), ErrorKind::ExpectedObjectKey);
        assert_eq!(error_kind(r#"{1: 1}"#), ErrorKind::ExpectedObjectKey);
        assert_eq!(error_kind(r#"{"a": 1,}"#), ErrorKind::ExpectedObjectKey);
        assert_eq!(error_kind(r#"{"a" 1}"#), ErrorKind::ExpectedColon);
        assert_eq!(error_kind(r#"{"a", 1}"#), ErrorKind::ExpectedColon);
        assert_eq!(
            error_kind(r#"{"a": 1 "b": 2}"#),
            ErrorKind::ExpectedCommaOrClose { close: b'}' }
        );
        assert_eq!(
            error_kind(r#"{"a": 1]"#),
            ErrorKind::ExpectedCommaOrClose { close: b'}' }
        );
        assert_eq!(error_kind(r#"{"a":"#), ErrorKind::UnexpectedEof);
        assert_eq!(error_kind(r#"{"a""#), ErrorKind::UnexpectedEof);
        assert_eq!(error_kind("{"), ErrorKind::UnexpectedEof);
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

        // The guard has to cover objects too, not just arrays.
        let deep_objects = format!("{}1{}", r#"{"a":"#.repeat(limit), "}".repeat(limit));
        assert!(parse(deep_objects.as_bytes()).is_ok());
        let deeper = format!("{}1{}", r#"{"a":"#.repeat(limit + 1), "}".repeat(limit + 1));
        assert_eq!(
            error_kind(&deeper),
            ErrorKind::DepthLimitExceeded { limit: MAX_DEPTH }
        );
    }
}
