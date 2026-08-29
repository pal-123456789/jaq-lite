//! Byte-level scanning: the cursor, the three literals, numbers and strings.
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

    /// Consume one byte that the caller has already inspected with `peek`.
    pub(crate) fn advance(&mut self) {
        self.at += 1;
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

    /// An error at a position the caller remembered, used when the interesting
    /// place is where a construct started rather than where scanning stopped.
    pub(crate) fn error_at(&self, kind: ErrorKind, offset: usize) -> ParseError {
        ParseError::new(kind, self.input, offset)
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

    /// Scan a string, decoding escapes.
    ///
    /// Ordinary bytes are copied in runs rather than one character at a time.
    /// That is safe without any UTF-8 bookkeeping because the three bytes that
    /// end a run -- `"`, `\` and anything below `0x20` -- are all ASCII, and a
    /// UTF-8 continuation byte is always `0x80` or above. So a run boundary can
    /// never fall inside a multi-byte character, which is the same argument that
    /// makes byte scanning a safe replacement for `memchr` here.
    pub(crate) fn scan_string(&mut self) -> Result<String, ParseError> {
        self.at += 1; // the opening quote the caller peeked
        let mut out = String::new();
        loop {
            let run = self.at;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.at += 1;
            }
            if self.at > run {
                let text = core::str::from_utf8(&self.input[run..self.at])
                    .expect("run boundaries are ASCII, so this is a character boundary");
                out.push_str(text);
            }
            match self.peek() {
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    self.scan_escape(&mut out)?;
                }
                // RFC 8259 requires every byte below 0x20 to be escaped. Note
                // that 0x7F is not in that range and is allowed raw, which is a
                // divergence from what many parsers do and is deliberate.
                Some(byte) => return Err(self.error(ErrorKind::ControlCharacterInString { byte })),
                None => return Err(self.error(ErrorKind::UnexpectedEof)),
            }
        }
    }

    fn scan_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        let Some(byte) = self.peek() else {
            return Err(self.error(ErrorKind::UnexpectedEof));
        };
        let decoded = match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                self.at += 1;
                return self.scan_unicode_escape(out);
            }
            // Reported before advancing, so the caret lands on the offending
            // byte rather than after it.
            _ => return Err(self.error(ErrorKind::InvalidEscape { byte })),
        };
        self.at += 1;
        out.push(decoded);
        Ok(())
    }

    /// Decode a `\uXXXX` escape, and the surrogate pair it may be half of.
    ///
    /// Surrogates exist only as a UTF-16 encoding artefact and are not
    /// characters. Rust's `String` cannot hold one, so an unpaired surrogate is
    /// rejected rather than replaced: substituting U+FFFD would silently change
    /// the document, and this parser reports rather than repairs.
    fn scan_unicode_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        let escape_start = self.at - 2;
        let first = self.scan_hex4()?;

        let decoded = if (0xD800..=0xDBFF).contains(&first) {
            if !self.input[self.at..].starts_with(b"\\u") {
                return Err(
                    self.error_at(ErrorKind::LoneSurrogate { code_unit: first }, escape_start)
                );
            }
            self.at += 2;
            let second = self.scan_hex4()?;
            if !(0xDC00..=0xDFFF).contains(&second) {
                return Err(
                    self.error_at(ErrorKind::LoneSurrogate { code_unit: first }, escape_start)
                );
            }
            let combined =
                0x1_0000 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00);
            char::from_u32(combined)
                .ok_or_else(|| self.error_at(ErrorKind::InvalidUnicodeEscape, escape_start))?
        } else if (0xDC00..=0xDFFF).contains(&first) {
            return Err(self.error_at(ErrorKind::LoneSurrogate { code_unit: first }, escape_start));
        } else {
            char::from_u32(u32::from(first))
                .ok_or_else(|| self.error_at(ErrorKind::InvalidUnicodeEscape, escape_start))?
        };

        out.push(decoded);
        Ok(())
    }

    fn scan_hex4(&mut self) -> Result<u16, ParseError> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let Some(byte) = self.peek() else {
                return Err(self.error(ErrorKind::UnexpectedEof));
            };
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.error(ErrorKind::InvalidUnicodeEscape)),
            };
            value = value * 16 + u16::from(digit);
            self.at += 1;
        }
        Ok(value)
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

    fn string_of(src: &str) -> String {
        match parse(src.as_bytes()).expect(src) {
            Value::String(text) => text,
            other => panic!("expected a string, got {other:?}"),
        }
    }

    fn error_kind(src: &str) -> ErrorKind {
        parse(src.as_bytes())
            .expect_err("should not have parsed")
            .kind()
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

    #[test]
    fn every_escape_the_rfc_defines_is_decoded() {
        assert_eq!(string_of(r#""\"\\\/\b\f\n\r\t""#), "\"\\/\u{8}\u{c}\n\r\t");
    }

    #[test]
    fn unicode_escapes_and_surrogate_pairs_are_decoded() {
        assert_eq!(string_of(r#""\u0041\u00e9\u20ac""#), "A\u{e9}\u{20ac}");
        // A pair, which is the only legal way to write a character above the BMP.
        assert_eq!(string_of(r#""\ud83d\ude00""#), "\u{1f600}");
    }

    #[test]
    fn unpaired_surrogates_are_rejected_not_replaced() {
        assert_eq!(
            error_kind(r#""\ud800""#),
            ErrorKind::LoneSurrogate { code_unit: 0xd800 }
        );
        assert_eq!(
            error_kind(r#""\udc00""#),
            ErrorKind::LoneSurrogate { code_unit: 0xdc00 }
        );
        // A high surrogate followed by an escape that is not a low surrogate.
        assert_eq!(
            error_kind(r#""\ud800\u0041""#),
            ErrorKind::LoneSurrogate { code_unit: 0xd800 }
        );
    }

    #[test]
    fn a_lone_surrogate_is_reported_at_the_start_of_its_escape() {
        let err = parse(r#""ab\ud800""#.as_bytes()).expect_err("lone surrogate");
        assert_eq!(err.column(), 4, "the caret should sit on the backslash");
    }

    #[test]
    fn bad_escapes_and_bad_hex_are_distinguished() {
        assert_eq!(
            error_kind(r#""\x""#),
            ErrorKind::InvalidEscape { byte: b'x' }
        );
        assert_eq!(
            error_kind(r#""\U0041""#),
            ErrorKind::InvalidEscape { byte: b'U' }
        );
        assert_eq!(error_kind(r#""\uqqqq""#), ErrorKind::InvalidUnicodeEscape);
        assert_eq!(error_kind(r#""\u00A""#), ErrorKind::InvalidUnicodeEscape);
    }

    #[test]
    fn raw_control_bytes_must_be_escaped_but_delete_need_not_be() {
        assert_eq!(
            error_kind("\"a\tb\""),
            ErrorKind::ControlCharacterInString { byte: b'\t' }
        );
        assert_eq!(
            error_kind("\"a\nb\""),
            ErrorKind::ControlCharacterInString { byte: b'\n' }
        );
        // 0x7F is not below 0x20, so the RFC does not require escaping it.
        assert_eq!(string_of("\"a\u{7f}b\""), "a\u{7f}b");
    }

    #[test]
    fn multibyte_characters_pass_through_untouched() {
        assert_eq!(
            string_of("\"h\u{e9}llo \u{1f600}\""),
            "h\u{e9}llo \u{1f600}"
        );
    }

    #[test]
    fn an_unterminated_string_runs_out_of_input() {
        assert_eq!(error_kind("\"abc"), ErrorKind::UnexpectedEof);
        assert_eq!(error_kind("\"abc\\"), ErrorKind::UnexpectedEof);
    }
}
