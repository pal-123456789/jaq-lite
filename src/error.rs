//! Parse failures, and the position information a caret diagnostic needs.

use std::fmt;

/// What went wrong.
///
/// Every variant here corresponds to a rule in RFC 8259 that the corpus in
/// `tests/fixtures/` actually exercises. Variants are added as the grammar is
/// implemented rather than reserved in advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The input held no JSON value: it was empty, or only whitespace.
    EmptyInput,
    /// The input was not well-formed UTF-8.
    InvalidUtf8 {
        /// How many leading bytes decoded successfully. The failure is at this
        /// offset, which is what makes a caret possible.
        valid_up_to: usize,
    },
    /// A byte appeared where nothing legal could begin.
    UnexpectedByte {
        /// The offending byte.
        byte: u8,
    },
    /// The input ended part-way through a value.
    UnexpectedEof,
    /// A complete value was followed by something other than whitespace.
    TrailingData,
    /// Nesting went deeper than the parser allows.
    DepthLimitExceeded {
        /// The limit that was exceeded.
        limit: u32,
    },
    /// A number literal did not match the RFC 8259 grammar.
    InvalidNumber,
    /// A backslash was followed by a byte that is not a legal escape.
    InvalidEscape {
        /// The byte that followed the backslash.
        byte: u8,
    },
    /// A `\u` escape was not followed by four hexadecimal digits.
    InvalidUnicodeEscape,
    /// A surrogate code unit appeared without its pair.
    LoneSurrogate {
        /// The unpaired code unit.
        code_unit: u16,
    },
    /// A byte below 0x20 appeared unescaped inside a string literal.
    ControlCharacterInString {
        /// The offending byte.
        byte: u8,
    },
    /// An object member was missing its colon.
    ExpectedColon,
    /// A container needed a comma or its closing bracket.
    ExpectedCommaOrClose {
        /// The closing byte that was expected: `]` or `}`.
        close: u8,
    },
    /// An object key was something other than a string.
    ExpectedObjectKey,
}

/// Render a byte readably: as itself when printable, as hex otherwise.
fn byte_str(byte: u8) -> String {
    if (0x20..=0x7e).contains(&byte) {
        format!("`{}`", byte as char)
    } else {
        format!("0x{byte:02x}")
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::EmptyInput => f.write_str("no JSON value found"),
            Self::InvalidUtf8 { valid_up_to } => {
                write!(f, "invalid UTF-8 after {valid_up_to} valid bytes")
            }
            Self::UnexpectedByte { byte } => {
                write!(f, "unexpected {}", byte_str(byte))
            }
            Self::UnexpectedEof => f.write_str("unexpected end of input"),
            Self::TrailingData => f.write_str("trailing data after the value"),
            Self::DepthLimitExceeded { limit } => {
                write!(f, "nesting deeper than the limit of {limit}")
            }
            Self::InvalidNumber => f.write_str("not a valid JSON number"),
            Self::InvalidEscape { byte } => {
                write!(f, "{} is not a valid escape", byte_str(byte))
            }
            Self::InvalidUnicodeEscape => {
                f.write_str("expected four hexadecimal digits after `\\u`")
            }
            Self::LoneSurrogate { code_unit } => {
                write!(f, "unpaired surrogate U+{code_unit:04X}")
            }
            Self::ControlCharacterInString { byte } => {
                write!(
                    f,
                    "unescaped control character {} in string",
                    byte_str(byte)
                )
            }
            Self::ExpectedColon => f.write_str("expected `:`"),
            Self::ExpectedCommaOrClose { close } => {
                write!(f, "expected `,` or {}", byte_str(close))
            }
            Self::ExpectedObjectKey => f.write_str("expected a string as the object key"),
        }
    }
}

/// A parse failure together with where it happened.
///
/// The byte offset is authoritative; line and column are derived from it and
/// exist for humans. `Display` gives the one-line form. Rendering the source
/// line with a caret under the offset is the job of the diagnostic renderer,
/// which needs the input and so cannot live on the error itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: ErrorKind,
    offset: usize,
    line: usize,
    column: usize,
}

impl ParseError {
    /// Build an error for `kind` at byte `offset` within `input`, computing the
    /// line and column from the input.
    pub fn new(kind: ErrorKind, input: &[u8], offset: usize) -> Self {
        let (line, column) = locate(input, offset);
        Self {
            kind,
            offset,
            line,
            column,
        }
    }

    /// What went wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The byte offset the failure was detected at.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The 1-based line number.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The 1-based column, counted in characters rather than bytes.
    pub fn column(&self) -> usize {
        self.column
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.kind
        )
    }
}

impl std::error::Error for ParseError {}

/// Convert a byte offset into a 1-based line and column.
///
/// The column counts characters, not bytes, by skipping UTF-8 continuation
/// bytes. Working on bytes rather than on `str` is deliberate: this has to
/// produce a sensible position for input that is not valid UTF-8 at all, which
/// is a case a `char_indices` walk cannot reach.
///
/// Shared with the filter parser, which counts columns in filter text by the
/// same rule, so that one caret renderer can serve both.
pub(crate) fn locate(input: &[u8], offset: usize) -> (usize, usize) {
    let end = offset.min(input.len());
    let mut line = 1;
    let mut line_start = 0;
    for (i, &b) in input[..end].iter().enumerate() {
        if b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let column = 1 + input[line_start..end]
        .iter()
        .filter(|&&b| (b & 0xC0) != 0x80)
        .count();
    (line, column)
}
