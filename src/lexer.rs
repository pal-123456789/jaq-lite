//! Byte-level scanning: the cursor, the three literals, and numbers.
//!
//! Structure and recursion belong to `parser`; everything that reads bytes one
//! at a time lives here. The split keeps the only recursive code in the crate
//! in one small file that can be reasoned about for depth safety on its own.
//!
//! One invariant holds throughout this module: `crate::parse` has already
//! validated the input as UTF-8, so any byte at or above `0x80` is part of a
//! well-formed multi-byte sequence and nothing here needs to check that again.

use crate::error::{ErrorKind, ParseError};
use crate::value::{Number, Value};

/// A position in the input, carrying the input so errors can locate themselves.
pub(crate) struct Cursor<'a> {
    input: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    /// Start at the first byte.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input, at: 0 }
    }

    /// True when there is nothing left to read.
    pub(crate) fn is_eof(&self) -> bool {
        self.at >= self.input.len()
    }

    /// The next byte without consuming it.
    pub(crate) fn peek(&self) -> Option<u8> {
        self.input.get(self.at).copied()
    }

    /// Advance past the four bytes RFC 8259 permits between tokens.
    ///
    /// `char::is_whitespace` is deliberately not used: Unicode's White_Space
    /// property covers a much larger set, and six fixtures in the corpus exist
    /// to catch a parser that confuses the two.
    pub(crate) fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// An error at the current position.
    pub(crate) fn error(&self, kind: ErrorKind) -> ParseError {
        ParseError::new(kind, self.input, self.at)
    }

    /// The error for a byte that cannot appear where the cursor is standing,
    /// distinguishing a wrong byte from running out of input.
    pub(crate) fn unexpected(&self) -> ParseError {
        match self.peek() {
            Some(byte) => self.error(ErrorKind::UnexpectedByte { byte }),
            None => self.error(ErrorKind::UnexpectedEof),
        }
    }

    /// Scan `true`, `false` or `null`.
    ///
    /// Matching the whole word at once is what rejects `tru` and `nul`: there is
    /// no state in which a prefix of a literal is acceptable.
    pub(crate) fn scan_literal(&mut self) -> Result<Value, ParseError> {
        let rest = &self.input[self.at..];
        if rest.starts_with(b"true") {
            self.at += 4;
            return Ok(Value::Bool(true));
        }
        if rest.starts_with(b"false") {
            self.at += 5;
            return Ok(Value::Bool(false));
        }
        if rest.starts_with(b"null") {
            self.at += 4;
            return Ok(Value::Null);
        }
        Err(self.unexpected())
    }

    /// Scan a number, keeping its source text.
    ///
    /// The grammar implemented here is RFC 8259's, which is narrower than the
    /// one `str::parse::<f64>()` accepts in exactly the places the corpus
    /// probes: a leading zero may not be followed by another digit, a fraction
    /// needs at least one digit after the point, and an exponent needs at least
    /// one digit after the optional sign. Nineteen of the 47 single-literal
    /// `n_number_*` fixtures parse cleanly as `f64`, so deferring to the
    /// standard library here would silently accept nineteen malformed
    /// documents.
    ///
    /// The scanner stops at the first byte that cannot extend the number and
    /// does not consume it, which is what turns `1,2` into trailing data rather
    /// than one unreadable number.
    pub(crate) fn scan_number(&mut self) -> Result<Number, ParseError> {
        let start = self.at;

        if self.peek() == Some(b'-') {
            self.at += 1;
        }

        match self.peek() {
            Some(b'0') => {
                self.at += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.error(ErrorKind::InvalidNumber));
                }
            }
            Some(b'1'..=b'9') => self.skip_digits(),
            _ => return Err(self.error(ErrorKind::InvalidNumber)),
        }

        if self.peek() == Some(b'.') {
            self.at += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error(ErrorKind::InvalidNumber));
            }
            self.skip_digits();
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error(ErrorKind::InvalidNumber));
            }
            self.skip_digits();
        }

        let raw = &self.input[start..self.at];
        // Every byte accepted above is ASCII, so this cannot fail.
        let text = core::str::from_utf8(raw).expect("number bytes are ASCII");
        // The grammar above is a strict subset of the one f64 accepts. Overflow
        // is not an error: 1e999 becomes infinity, and the preserved text is
        // what gets printed, so no information is lost on the way out.
        let val: f64 = text.parse().expect("scanned text is valid f64 syntax");
        Ok(Number::new(text, val))
    }

    fn skip_digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Number, ParseError};
    use crate::{ErrorKind, Value, parse};

    fn number(src: &str) -> Result<Number, ParseError> {
        let mut cursor = Cursor::new(src.as_bytes());
        cursor.scan_number()
    }

    #[test]
    fn number_text_survives_the_round_trip() {
        for src in ["0", "-0", "1E+2", "1e-2", "0.10", "-1.5e10"] {
            let scanned = number(src).expect(src);
            assert_eq!(scanned.as_str(), src, "text was rewritten");
        }
    }

    #[test]
    fn the_rfc_number_grammar_is_narrower_than_rusts() {
        // Several of these parse cleanly as f64 and must still be rejected,
        // which is the whole reason this scanner exists.
        for src in [
            "01", "-01", "1.", ".5", "1e", "1e+", "2.e3", "inf", "NaN", "+1", "-",
        ] {
            assert!(number(src).is_err(), "{src} should have been rejected");
        }
    }

    #[test]
    fn huge_exponents_go_infinite_but_keep_their_text() {
        let scanned = number("1e999").expect("grammatically valid");
        assert!(scanned.as_f64().is_infinite());
        assert_eq!(scanned.as_str(), "1e999");
    }

    #[test]
    fn literals_match_whole_words_only() {
        assert_eq!(parse(b"true").expect("true"), Value::Bool(true));
        assert_eq!(parse(b"false").expect("false"), Value::Bool(false));
        assert_eq!(parse(b"null").expect("null"), Value::Null);
        for src in [&b"tru"[..], b"nul", b"fals", b"truex"] {
            assert!(parse(src).is_err(), "prefix or suffix wrongly accepted");
        }
    }

    #[test]
    fn a_number_stops_at_the_first_byte_it_cannot_use() {
        let err = parse(b"1,2").expect_err("two values");
        assert_eq!(err.kind(), ErrorKind::TrailingData);
        assert_eq!(err.column(), 2, "the comma should not have been consumed");
    }
}
