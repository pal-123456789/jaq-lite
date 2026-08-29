//! The filter language: compiling a jq-style program and running it on a value.
//!
//! # The shape of the thing
//!
//! A jq filter is not a function from one value to one value. It is a function
//! from one value to a *stream* of values: `.a` produces one output, `.[]`
//! produces as many as the array is long, and a filter can produce none at all.
//! Every design decision here follows from that. `eval` appends to a caller's
//! `Vec` instead of returning a value, `|` feeds each of the left side's outputs
//! through the right side, and `,` simply concatenates two streams.
//!
//! Building the stream into a `Vec` rather than a lazy iterator is a deliberate
//! trade. A borrowing iterator over a recursive tree needs either boxed dynamic
//! iterators at every node or a hand-written state machine, and both cost far
//! more in complexity than the intermediate allocations cost in time for
//! documents that fit in memory -- which is the only kind this tool reads.
//!
//! # Where the semantics come from
//!
//! Not from the jq manual. Every rule in here was run against jq 1.8.1 and the
//! answer copied down, because the manual is silent or misleading on exactly the
//! cases that matter. Two examples, both of which this module got wrong before
//! the measurement: null is indexable but not iterable, and a trailing `?`
//! forgives only the step it follows rather than the whole path to its left.
//!
//! # Depth
//!
//! Parenthesis nesting is capped at 64, for the same reason the JSON parser caps
//! container nesting: recursive descent meets unbounded nesting as a stack
//! overflow. The cap is lower here because no human writes a filter anywhere
//! near that deep. Note that the length of a *path* is not capped: `.a.a.a...`
//! makes `eval` recurse once per step. That is acceptable where unbounded JSON
//! nesting is not, because a filter is program text the user typed, while a
//! document is untrusted input that arrived from somewhere else.

use crate::lexer::Cursor;
use crate::value::Value;
use core::fmt;

/// How deeply parentheses may nest.
const MAX_DEPTH: u32 = 64;

/// The longest offending value a message prints whole.
///
/// jq's number, measured rather than guessed: fourteen characters print whole
/// and fifteen or more come back as eleven characters and three dots. That is
/// jq formatting the value into a fifteen-byte buffer and overwriting the last
/// three characters, and both boundaries are pinned by tests below.
const SHOWN_FULL: usize = 14;

/// How many characters survive the cut, which is the same arithmetic.
const SHOWN_KEEP: usize = 11;

/// A compiled filter.
///
/// Opaque on purpose: the tree inside is an implementation detail, and keeping
/// it private means the language can grow without breaking anyone.
#[derive(Debug, Clone)]
pub struct Filter {
    root: Node,
}

impl Filter {
    /// Compile a jq-style filter.
    ///
    /// An empty filter is the identity, which is what jq does with `jq ''`.
    ///
    /// # Errors
    ///
    /// Returns the reason, the byte offset, and the line and column that offset
    /// falls on, if the filter does not parse.
    pub fn compile(source: &str) -> Result<Self, FilterError> {
        Self::parse(source).map_err(|error| error.at(source))
    }

    /// The body of [`Filter::compile`].
    ///
    /// Split out so that every error leaves through the one `map_err` above,
    /// which is the only place the offset and the source text are both in hand.
    fn parse(source: &str) -> Result<Self, FilterError> {
        let tokens = tokenize(source)?;
        if tokens.is_empty() {
            return Ok(Self {
                root: Node::Identity,
            });
        }
        let mut parser = Parser {
            tokens,
            at: 0,
            end: source.len(),
            depth: 0,
        };
        let root = parser.parse_pipe()?;
        if let Some(extra) = parser.tokens.get(parser.at) {
            return Err(FilterError::new(
                FilterErrorKind::Unexpected {
                    found: extra.token.describe(),
                },
                extra.offset,
            ));
        }
        Ok(Self { root })
    }

    /// Run the filter on one input, collecting every output it produces.
    ///
    /// # Errors
    ///
    /// Returns the first runtime error, for example a field name applied to a
    /// number. Outputs produced before the error are discarded, because a
    /// half-written stream is worse than none.
    pub fn run(&self, input: &Value) -> Result<Vec<Value>, EvalError> {
        let mut out = Vec::new();
        eval(&self.root, input, &mut out)?;
        Ok(out)
    }
}

/// What a single step does when the value it was handed is the wrong type.
///
/// This is what a trailing `?` sets, and it belongs on the step rather than on a
/// wrapper because jq scopes it that narrowly: `.a.b?` still fails at `.a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnError {
    /// Report it.
    Fail,
    /// Produce nothing and carry on.
    Skip,
}

/// One node of the compiled tree.
#[derive(Debug, Clone)]
enum Node {
    /// `.`
    Identity,
    /// `a | b`
    Pipe(Box<Node>, Box<Node>),
    /// `a, b`
    Both(Box<Node>, Box<Node>),
    /// `a.name`, `a["name"]`
    Field(Box<Node>, String, OnError),
    /// `a[0]`, `a[-1]`
    Index(Box<Node>, i64, OnError),
    /// `a[]`
    Iterate(Box<Node>, OnError),
    /// `(a)?` -- a `?` with no single step to attach to, so it catches whatever
    /// the parenthesised filter raises.
    Optional(Box<Node>),
}

/// Make the outermost path step forgiving.
///
/// Only ever called when the outermost node *is* a step, so the last arm is a
/// fallback rather than a case that happens.
fn forgive(node: Node) -> Node {
    match node {
        Node::Field(source, name, _) => Node::Field(source, name, OnError::Skip),
        Node::Index(source, index, _) => Node::Index(source, index, OnError::Skip),
        Node::Iterate(source, _) => Node::Iterate(source, OnError::Skip),
        other => Node::Optional(Box::new(other)),
    }
}

// -- errors -----------------------------------------------------------------

/// Why a filter could not be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterError {
    kind: FilterErrorKind,
    offset: usize,
    line: usize,
    column: usize,
}

impl FilterError {
    /// Build an error at a byte offset in the filter text.
    ///
    /// The line and column start out as the values for a single-line filter,
    /// which is what almost every filter is, and are corrected by [`Self::at`]
    /// on the way out of [`Filter::compile`]. The parser tracks offsets and does
    /// not carry the source, so that boundary is the one place both are in hand.
    fn new(kind: FilterErrorKind, offset: usize) -> Self {
        Self {
            kind,
            offset,
            line: 1,
            column: offset + 1,
        }
    }

    /// Measure where this error's offset actually falls within `source`.
    fn at(mut self, source: &str) -> Self {
        let (line, column) = crate::error::locate(source.as_bytes(), self.offset);
        self.line = line;
        self.column = column;
        self
    }

    /// What went wrong.
    #[must_use]
    pub fn kind(&self) -> &FilterErrorKind {
        &self.kind
    }

    /// Where it went wrong, as a byte offset into the filter text.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }
    /// The 1-based line of the filter text the failure is on.
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// The 1-based column, counted in characters rather than bytes.
    ///
    /// Characters rather than bytes so that this number and the caret drawn
    /// under the filter cannot disagree.
    #[must_use]
    pub fn column(&self) -> usize {
        self.column
    }
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A filter is one line unless someone went out of their way, so naming
        // line 1 on every message would be noise.
        if self.line == 1 {
            write!(f, "filter, column {}: {}", self.column, self.kind)
        } else {
            write!(
                f,
                "filter, line {}, column {}: {}",
                self.line, self.column, self.kind
            )
        }
    }
}

impl std::error::Error for FilterError {}

/// The specific reason a filter did not compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterErrorKind {
    /// A character that means nothing in this language.
    UnexpectedByte {
        /// The offending byte.
        byte: u8,
    },
    /// The filter stopped in the middle of something.
    UnexpectedEnd,
    /// A token turned up where no token of that kind can appear.
    Unexpected {
        /// A description of what was found, for the message.
        found: String,
    },
    /// A `.` was not followed by a name or a bracket.
    ExpectedFieldName,
    /// Something specific was required and was not there.
    Expected {
        /// The text that was required, such as `]`.
        what: &'static str,
    },
    /// A quoted name that is not a well-formed JSON string.
    InvalidString,
    /// Brackets that hold neither a whole number nor a quoted name.
    InvalidIndex,
    /// Parentheses nested deeper than the limit, which is 64.
    DepthLimitExceeded {
        /// The limit that was reached.
        limit: u32,
    },
}

impl fmt::Display for FilterErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedByte { byte } if byte.is_ascii_graphic() => {
                write!(f, "`{}` has no meaning here", char::from(*byte))
            }
            Self::UnexpectedByte { byte } => write!(f, "byte 0x{byte:02x} has no meaning here"),
            Self::UnexpectedEnd => write!(f, "the filter ends too soon"),
            Self::Unexpected { found } => write!(f, "{found} cannot appear here"),
            Self::ExpectedFieldName => write!(f, "expected a field name or `[` after `.`"),
            Self::Expected { what } => write!(f, "expected `{what}`"),
            Self::InvalidString => write!(f, "this is not a well-formed string"),
            Self::InvalidIndex => write!(f, "expected a whole number or a quoted name"),
            Self::DepthLimitExceeded { limit } => {
                write!(f, "parentheses nested deeper than {limit}")
            }
        }
    }
}

/// A filter asked a value for something that value cannot do.
///
/// The wording is jq 1.8.1's, capital letter and all, because a tool that claims
/// to stand in for jq should fail the way jq fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// A field name used on something other than an object or null.
    NotIndexableByName {
        /// The type that was indexed.
        found: &'static str,
        /// The name that was asked for.
        name: String,
    },
    /// A number used on something other than an array or null.
    NotIndexableByNumber {
        /// The type that was indexed.
        found: &'static str,
    },
    /// `.[]` used on something with no elements.
    NotIterable {
        /// The type that was iterated.
        found: &'static str,
        /// The value itself, cut short exactly where jq cuts it short.
        shown: String,
    },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotIndexableByName { found, name } => {
                write!(f, "Cannot index {found} with string \"{name}\"")
            }
            Self::NotIndexableByNumber { found } => write!(f, "Cannot index {found} with number"),
            Self::NotIterable { found, shown } => {
                write!(f, "Cannot iterate over {found} ({shown})")
            }
        }
    }
}

impl std::error::Error for EvalError {}

/// How a value appears inside an error message.
fn shown(value: &Value) -> String {
    let text = crate::to_string(value, crate::Style::Compact);
    if text.chars().count() <= SHOWN_FULL {
        return text;
    }
    // Characters and not bytes. jq truncates with `strncpy` and will happily
    // cut a UTF-8 sequence in half; matching that bug is not worth matching.
    let mut short: String = text.chars().take(SHOWN_KEEP).collect();
    short.push_str("...");
    short
}

// -- tokens -----------------------------------------------------------------

/// One token of filter text.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Dot,
    Ident(String),
    Text(String),
    Integer(i64),
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    Pipe,
    Comma,
    Question,
}

impl Token {
    /// How to name this token in an error message.
    fn describe(&self) -> String {
        match self {
            Self::Dot => "`.`".to_owned(),
            Self::Ident(name) => format!("`{name}`"),
            Self::Text(_) => "a quoted name".to_owned(),
            Self::Integer(value) => format!("`{value}`"),
            Self::OpenBracket => "`[`".to_owned(),
            Self::CloseBracket => "`]`".to_owned(),
            Self::OpenParen => "`(`".to_owned(),
            Self::CloseParen => "`)`".to_owned(),
            Self::Pipe => "`|`".to_owned(),
            Self::Comma => "`,`".to_owned(),
            Self::Question => "`?`".to_owned(),
        }
    }
}

/// A token and where it started, so an error can point at it.
#[derive(Debug, Clone)]
struct Spanned {
    token: Token,
    offset: usize,
}

/// Find the offset just past the closing quote of the string starting at `start`.
///
/// This locates the end without decoding anything, so that the JSON string
/// scanner can be handed the exact slice and do the actual escape handling.
/// Writing a second escape decoder here would be two places to get `\uD83D`
/// wrong instead of one.
fn end_of_text(bytes: &[u8], start: usize) -> Option<usize> {
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => return Some(at + 1),
            _ => at += 1,
        }
    }
    None
}

fn tokenize(source: &str) -> Result<Vec<Spanned>, FilterError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let byte = bytes[at];
        let token = match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                at += 1;
                continue;
            }
            b'.' => {
                at += 1;
                Token::Dot
            }
            b'[' => {
                at += 1;
                Token::OpenBracket
            }
            b']' => {
                at += 1;
                Token::CloseBracket
            }
            b'(' => {
                at += 1;
                Token::OpenParen
            }
            b')' => {
                at += 1;
                Token::CloseParen
            }
            b'|' => {
                at += 1;
                Token::Pipe
            }
            b',' => {
                at += 1;
                Token::Comma
            }
            b'?' => {
                at += 1;
                Token::Question
            }
            b'"' => {
                let end = end_of_text(bytes, at)
                    .ok_or_else(|| FilterError::new(FilterErrorKind::InvalidString, start))?;
                let text = Cursor::new(&bytes[at..end])
                    .scan_string()
                    .map_err(|_| FilterError::new(FilterErrorKind::InvalidString, start))?;
                at = end;
                Token::Text(text)
            }
            b'-' | b'0'..=b'9' => {
                let mut end = at + usize::from(byte == b'-');
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                let value: i64 = source[at..end]
                    .parse()
                    .map_err(|_| FilterError::new(FilterErrorKind::InvalidIndex, start))?;
                at = end;
                Token::Integer(value)
            }
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => {
                let mut end = at;
                while end < bytes.len()
                    && (bytes[end] == b'_' || bytes[end].is_ascii_alphanumeric())
                {
                    end += 1;
                }
                let name = source[at..end].to_owned();
                at = end;
                Token::Ident(name)
            }
            _ => {
                return Err(FilterError::new(
                    FilterErrorKind::UnexpectedByte { byte },
                    start,
                ));
            }
        };
        tokens.push(Spanned {
            token,
            offset: start,
        });
    }
    Ok(tokens)
}

// -- parsing ----------------------------------------------------------------

struct Parser {
    tokens: Vec<Spanned>,
    at: usize,
    end: usize,
    depth: u32,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at).map(|spanned| &spanned.token)
    }

    /// The offset to blame for a problem found at the current position.
    fn here(&self) -> usize {
        self.tokens
            .get(self.at)
            .map_or(self.end, |spanned| spanned.offset)
    }

    fn expect(&mut self, token: &Token, what: &'static str) -> Result<(), FilterError> {
        if self.peek() == Some(token) {
            self.at += 1;
            Ok(())
        } else {
            Err(FilterError::new(
                FilterErrorKind::Expected { what },
                self.here(),
            ))
        }
    }

    /// `a | b | c`, the loosest binding thing there is.
    fn parse_pipe(&mut self) -> Result<Node, FilterError> {
        let mut node = self.parse_comma()?;
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.at += 1;
            let right = self.parse_comma()?;
            node = Node::Pipe(Box::new(node), Box::new(right));
        }
        Ok(node)
    }

    /// `a, b`, which binds tighter than `|`: `.a, .b | .c` pipes both sides.
    fn parse_comma(&mut self) -> Result<Node, FilterError> {
        let mut node = self.parse_postfix()?;
        while matches!(self.peek(), Some(Token::Comma)) {
            self.at += 1;
            let right = self.parse_postfix()?;
            node = Node::Both(Box::new(node), Box::new(right));
        }
        Ok(node)
    }

    /// A term followed by any number of steps: `.a[0][]?`.
    fn parse_postfix(&mut self) -> Result<Node, FilterError> {
        let mut node = self.parse_term()?;
        // Whether a step has been appended since the term was parsed, which is
        // what decides where a `?` attaches. With a step, it marks that step and
        // errors from the rest of the path still escape -- jq's rule, measured.
        // Without one, as in `(.a.b)?`, there is nothing narrower to mark, so it
        // catches whatever the term raises.
        let mut steps = 0_usize;
        loop {
            if matches!(self.peek(), Some(Token::Dot)) {
                self.at += 1;
                node = self.parse_step(node)?;
                steps += 1;
            } else if matches!(self.peek(), Some(Token::OpenBracket)) {
                self.at += 1;
                node = self.parse_bracket(node)?;
                steps += 1;
            } else if matches!(self.peek(), Some(Token::Question)) {
                self.at += 1;
                node = if steps > 0 {
                    forgive(node)
                } else {
                    Node::Optional(Box::new(node))
                };
            } else {
                return Ok(node);
            }
        }
    }

    fn parse_term(&mut self) -> Result<Node, FilterError> {
        if matches!(self.peek(), Some(Token::Dot)) {
            self.at += 1;
            // A leading `.` is either the identity or the start of a path. The
            // difference is only visible in the token after it.
            if matches!(
                self.peek(),
                Some(Token::Ident(_) | Token::Text(_) | Token::OpenBracket)
            ) {
                return self.parse_step(Node::Identity);
            }
            return Ok(Node::Identity);
        }
        if matches!(self.peek(), Some(Token::OpenParen)) {
            self.at += 1;
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err(FilterError::new(
                    FilterErrorKind::DepthLimitExceeded { limit: MAX_DEPTH },
                    self.here(),
                ));
            }
            let inner = self.parse_pipe()?;
            self.expect(&Token::CloseParen, ")")?;
            self.depth -= 1;
            return Ok(inner);
        }
        match self.tokens.get(self.at) {
            Some(spanned) => Err(FilterError::new(
                FilterErrorKind::Unexpected {
                    found: spanned.token.describe(),
                },
                spanned.offset,
            )),
            None => Err(FilterError::new(FilterErrorKind::UnexpectedEnd, self.end)),
        }
    }

    /// What follows a `.`: a name, a quoted name, or a bracket.
    fn parse_step(&mut self, source: Node) -> Result<Node, FilterError> {
        // Read out of the token before touching `self` again, so the borrow the
        // name comes from is over before the cursor moves.
        enum Step {
            Name(String),
            Bracket,
        }
        let step = match self.peek() {
            Some(Token::Ident(name)) => Step::Name(name.clone()),
            Some(Token::Text(text)) => Step::Name(text.clone()),
            Some(Token::OpenBracket) => Step::Bracket,
            Some(_) | None => {
                return Err(FilterError::new(
                    FilterErrorKind::ExpectedFieldName,
                    self.here(),
                ));
            }
        };
        self.at += 1;
        match step {
            Step::Name(name) => Ok(Node::Field(Box::new(source), name, OnError::Fail)),
            Step::Bracket => self.parse_bracket(source),
        }
    }

    /// What sits inside `[...]`, with the `[` already consumed.
    fn parse_bracket(&mut self, source: Node) -> Result<Node, FilterError> {
        enum Inside {
            Iterate,
            Name(String),
            At(i64),
        }
        let inside = match self.peek() {
            Some(Token::CloseBracket) => Inside::Iterate,
            Some(Token::Text(text)) => Inside::Name(text.clone()),
            Some(Token::Integer(value)) => Inside::At(*value),
            Some(_) | None => {
                return Err(FilterError::new(FilterErrorKind::InvalidIndex, self.here()));
            }
        };
        self.at += 1;
        match inside {
            Inside::Iterate => Ok(Node::Iterate(Box::new(source), OnError::Fail)),
            Inside::Name(name) => {
                self.expect(&Token::CloseBracket, "]")?;
                Ok(Node::Field(Box::new(source), name, OnError::Fail))
            }
            Inside::At(value) => {
                self.expect(&Token::CloseBracket, "]")?;
                Ok(Node::Index(Box::new(source), value, OnError::Fail))
            }
        }
    }
}

// -- evaluation -------------------------------------------------------------

fn eval(node: &Node, input: &Value, out: &mut Vec<Value>) -> Result<(), EvalError> {
    match node {
        Node::Identity => {
            out.push(input.clone());
            Ok(())
        }
        Node::Pipe(first, second) => {
            let mut middle = Vec::new();
            eval(first, input, &mut middle)?;
            for value in &middle {
                eval(second, value, out)?;
            }
            Ok(())
        }
        Node::Both(left, right) => {
            eval(left, input, out)?;
            eval(right, input, out)
        }
        Node::Field(source, name, on_error) => {
            let mut values = Vec::new();
            eval(source, input, &mut values)?;
            for value in &values {
                match field(value, name) {
                    Ok(found) => out.push(found),
                    Err(error) if *on_error == OnError::Fail => return Err(error),
                    Err(_) => {}
                }
            }
            Ok(())
        }
        Node::Index(source, index, on_error) => {
            let mut values = Vec::new();
            eval(source, input, &mut values)?;
            for value in &values {
                match at(value, *index) {
                    Ok(found) => out.push(found),
                    Err(error) if *on_error == OnError::Fail => return Err(error),
                    Err(_) => {}
                }
            }
            Ok(())
        }
        Node::Iterate(source, on_error) => {
            let mut values = Vec::new();
            eval(source, input, &mut values)?;
            for value in &values {
                // `iterate` reports the wrong type before appending anything, so
                // a skipped element leaves no half-written output behind.
                match iterate(value, out) {
                    Ok(()) => {}
                    Err(error) if *on_error == OnError::Fail => return Err(error),
                    Err(_) => {}
                }
            }
            Ok(())
        }
        Node::Optional(inner) => {
            // Into a scratch buffer, so a filter that produces two values and
            // then fails contributes nothing rather than half of itself.
            let mut scratch = Vec::new();
            if eval(inner, input, &mut scratch).is_ok() {
                out.append(&mut scratch);
            }
            Ok(())
        }
    }
}

/// `.name`.
///
/// Null is indexable and yields null, which is what makes `.a.b.c` on a missing
/// branch return null instead of failing. A missing key on a real object is the
/// same story. Anything else is a type error.
fn field(value: &Value, name: &str) -> Result<Value, EvalError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Object(entries) => Ok(entries
            .iter()
            .find(|(key, _)| key == name)
            .map_or(Value::Null, |(_, found)| found.clone())),
        other => Err(EvalError::NotIndexableByName {
            found: other.type_name(),
            name: name.to_owned(),
        }),
    }
}

/// `.[n]`, where a negative `n` counts back from the end.
fn at(value: &Value, index: i64) -> Result<Value, EvalError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Array(items) => {
            // `checked_neg` and not `-index`: negating `i64::MIN` overflows, and
            // `.[-9223372036854775808]` is a filter someone can actually type.
            let resolved = if index < 0 {
                index
                    .checked_neg()
                    .and_then(|back| usize::try_from(back).ok())
                    .and_then(|back| items.len().checked_sub(back))
            } else {
                usize::try_from(index).ok()
            };
            Ok(resolved
                .and_then(|position| items.get(position))
                .cloned()
                .unwrap_or(Value::Null))
        }
        other => Err(EvalError::NotIndexableByNumber {
            found: other.type_name(),
        }),
    }
}

/// `.[]`. An object iterates its values, in the order they were written.
///
/// Null is *not* iterable, unlike indexing. That asymmetry is jq's, and it is
/// the right one: asking for a missing field is ordinary, but iterating nothing
/// is almost always a mistake worth reporting.
fn iterate(value: &Value, out: &mut Vec<Value>) -> Result<(), EvalError> {
    match value {
        Value::Array(items) => {
            out.extend(items.iter().cloned());
            Ok(())
        }
        Value::Object(entries) => {
            out.extend(entries.iter().map(|(_, found)| found.clone()));
            Ok(())
        }
        other => Err(EvalError::NotIterable {
            found: other.type_name(),
            shown: shown(other),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{EvalError, Filter, FilterError, FilterErrorKind};
    use crate::Style;

    /// Compile, run, and render every output compactly, so a test can state its
    /// expectation as a list of strings.
    fn run(filter: &str, json: &str) -> Vec<String> {
        let value = crate::parse(json.as_bytes()).expect("the test input should be valid JSON");
        Filter::compile(filter)
            .expect("the test filter should compile")
            .run(&value)
            .expect("the test filter should not fail")
            .iter()
            .map(|output| crate::to_string(output, Style::Compact))
            .collect()
    }

    fn fails(filter: &str, json: &str) -> EvalError {
        let value = crate::parse(json.as_bytes()).expect("the test input should be valid JSON");
        Filter::compile(filter)
            .expect("the test filter should compile")
            .run(&value)
            .expect_err("the test filter should have failed")
    }

    fn wont_compile(filter: &str) -> FilterError {
        Filter::compile(filter).expect_err("the test filter should not have compiled")
    }

    fn cannot_index_number_with_a() -> EvalError {
        EvalError::NotIndexableByName {
            found: "number",
            name: "a".to_owned(),
        }
    }

    #[test]
    fn the_identity_returns_its_input_unchanged() {
        assert_eq!(run(".", r#"{"a":[1,2]}"#), vec![r#"{"a":[1,2]}"#]);
    }

    #[test]
    fn an_empty_filter_is_the_identity() {
        assert_eq!(run("", "1"), vec!["1"]);
        assert_eq!(run("   ", "1"), vec!["1"]);
    }

    #[test]
    fn a_field_reaches_into_an_object() {
        assert_eq!(run(".a", r#"{"a":1}"#), vec!["1"]);
        assert_eq!(run(".a.b", r#"{"a":{"b":2}}"#), vec!["2"]);
        assert_eq!(run(".missing", "{}"), vec!["null"]);
    }

    #[test]
    fn a_field_name_may_be_quoted_or_bracketed() {
        assert_eq!(run(r#"."a b""#, r#"{"a b":7}"#), vec!["7"]);
        assert_eq!(run(r#".["a b"]"#, r#"{"a b":7}"#), vec!["7"]);
        assert_eq!(run(r#".a["b"]"#, r#"{"a":{"b":8}}"#), vec!["8"]);
        // The escape decoder is the JSON one, so this is a name with a quote in
        // it rather than a syntax error.
        assert_eq!(run(r#".["q\"q"]"#, r#"{"q\"q":9}"#), vec!["9"]);
    }

    #[test]
    fn null_absorbs_a_whole_path() {
        assert_eq!(run(".a.b.c", "null"), vec!["null"]);
        assert_eq!(run(".a.b.c", r#"{"a":null}"#), vec!["null"]);
        assert_eq!(run(".[3]", "null"), vec!["null"]);
    }

    #[test]
    fn indexing_counts_from_either_end() {
        assert_eq!(run(".[0]", "[1,2,3]"), vec!["1"]);
        assert_eq!(run(".[-1]", "[1,2,3]"), vec!["3"]);
        assert_eq!(run(".[9]", "[1,2,3]"), vec!["null"]);
        assert_eq!(run(".[-9]", "[1,2,3]"), vec!["null"]);
    }

    #[test]
    fn the_most_negative_index_does_not_panic() {
        assert_eq!(run(".[-9223372036854775808]", "[1]"), vec!["null"]);
    }

    #[test]
    fn iterating_yields_elements_and_then_values() {
        assert_eq!(run(".[]", "[1,2]"), vec!["1", "2"]);
        assert_eq!(run(".[]", r#"{"b":1,"a":2}"#), vec!["1", "2"]);
        assert!(run(".[]", "[]").is_empty(), "an empty array yields nothing");
    }

    #[test]
    fn the_pipe_feeds_every_output_into_the_next_stage() {
        assert_eq!(run(".[] | .id", r#"[{"id":1},{"id":2}]"#), vec!["1", "2"]);
        assert_eq!(run(".a | .b | .c", r#"{"a":{"b":{"c":4}}}"#), vec!["4"]);
    }

    #[test]
    fn the_comma_concatenates_two_streams() {
        assert_eq!(run(".a, .b", r#"{"a":1,"b":2}"#), vec!["1", "2"]);
        // `,` binds tighter than `|`, so both sides go through `.x`.
        assert_eq!(
            run(".a, .b | .x", r#"{"a":{"x":1},"b":{"x":2}}"#),
            vec!["1", "2"]
        );
        assert_eq!(run(".[] | .a, .b", r#"[{"a":1,"b":2}]"#), vec!["1", "2"]);
    }

    #[test]
    fn parentheses_group_a_whole_pipeline() {
        assert_eq!(
            run("(.a | .b), .c", r#"{"a":{"b":1},"c":2}"#),
            vec!["1", "2"]
        );
    }

    #[test]
    fn a_question_mark_forgives_only_the_step_it_follows() {
        assert!(
            run(".a?", "1").is_empty(),
            "a number has no fields, and `?` forgives it"
        );
        assert!(run(".[]?", "1").is_empty());
        assert_eq!(
            run(".a?", r#"{"a":1}"#),
            vec!["1"],
            "`?` does not change a success"
        );
        // Measured against jq 1.8.1, which errors here. `?` marks `.b`; the
        // failure happens earlier, at `.a`, and is none of its business.
        assert_eq!(fails(".a.b?", "1"), cannot_index_number_with_a());
        // Parentheses are how you catch the whole path, and there `?` has no
        // single step to attach to.
        assert!(run("(.a.b)?", "1").is_empty());
    }

    #[test]
    fn asking_the_wrong_type_is_a_runtime_error() {
        assert_eq!(
            fails(".a", "[1]"),
            EvalError::NotIndexableByName {
                found: "array",
                name: "a".to_owned()
            }
        );
        assert_eq!(fails(".a", "1"), cannot_index_number_with_a());
        assert_eq!(
            fails(".[0]", r#"{"a":1}"#),
            EvalError::NotIndexableByNumber { found: "object" }
        );
        assert_eq!(
            fails(".[]", "null"),
            EvalError::NotIterable {
                found: "null",
                shown: "null".to_owned()
            }
        );
        assert_eq!(
            fails(".[]", r#""s""#),
            EvalError::NotIterable {
                found: "string",
                shown: "\"s\"".to_owned()
            }
        );
    }

    #[test]
    fn an_error_inside_a_pipeline_stops_the_pipeline() {
        assert_eq!(fails(".[] | .a", "[{},1]"), cannot_index_number_with_a());
    }

    #[test]
    fn the_message_is_word_for_word_what_jq_prints() {
        assert_eq!(
            fails(".a", "true").to_string(),
            "Cannot index boolean with string \"a\""
        );
        assert_eq!(
            fails(".[0]", "{}").to_string(),
            "Cannot index object with number"
        );
        assert_eq!(
            fails(".[]", "null").to_string(),
            "Cannot iterate over null (null)"
        );
        assert_eq!(
            fails(".[]", r#""s""#).to_string(),
            "Cannot iterate over string (\"s\")"
        );
    }

    #[test]
    fn a_long_value_is_cut_short_in_the_message() {
        // Every string below was copied out of jq 1.8.1's own output for the
        // same input. jq keeps eleven characters and cuts at fifteen, so the
        // fourteen-character cases are the last ones that print whole.
        let long = format!("\"{}\"", "a".repeat(100));
        assert_eq!(
            fails(".[]", &long).to_string(),
            "Cannot iterate over string (\"aaaaaaaaaa...)"
        );
        assert_eq!(
            fails(".[]", "\"aaaaaaaaaaaa\"").to_string(),
            "Cannot iterate over string (\"aaaaaaaaaaaa\")"
        );
        assert_eq!(
            fails(".[]", "12345678901234").to_string(),
            "Cannot iterate over number (12345678901234)"
        );
        assert_eq!(
            fails(".[]", "123456789012345").to_string(),
            "Cannot iterate over number (12345678901...)"
        );
    }

    #[test]
    fn nonsense_does_not_compile() {
        assert_eq!(
            *wont_compile("|").kind(),
            FilterErrorKind::Unexpected {
                found: "`|`".to_owned()
            }
        );
        assert_eq!(*wont_compile(".a |").kind(), FilterErrorKind::UnexpectedEnd);
        assert_eq!(
            *wont_compile(".a.").kind(),
            FilterErrorKind::ExpectedFieldName
        );
        assert_eq!(
            *wont_compile(".[0").kind(),
            FilterErrorKind::Expected { what: "]" }
        );
        assert_eq!(
            *wont_compile("(.a").kind(),
            FilterErrorKind::Expected { what: ")" }
        );
        assert_eq!(
            *wont_compile(".[1.5]").kind(),
            FilterErrorKind::Expected { what: "]" }
        );
        assert_eq!(*wont_compile(".[x]").kind(), FilterErrorKind::InvalidIndex);
        assert_eq!(
            *wont_compile(".a %").kind(),
            FilterErrorKind::UnexpectedByte { byte: b'%' }
        );
        assert_eq!(
            *wont_compile(r#".["unclosed"#).kind(),
            FilterErrorKind::InvalidString
        );
        // A name with no leading dot is a function call, and no functions exist
        // yet -- so this has to fail rather than quietly do nothing.
        assert_eq!(
            *wont_compile("length").kind(),
            FilterErrorKind::Unexpected {
                found: "`length`".to_owned()
            }
        );
    }

    #[test]
    fn a_filter_error_points_at_the_column_that_is_wrong() {
        let error = wont_compile(".a %");
        assert_eq!(error.offset(), 3);
        assert_eq!((error.line(), error.column()), (1, 4));
        assert_eq!(
            error.to_string(),
            "filter, column 4: `%` has no meaning here"
        );
    }

    #[test]
    fn a_filter_column_counts_characters_and_not_bytes() {
        // The quoted name is one character written as two bytes. Counting bytes
        // would say column 9 and draw the caret one place right of the `%`.
        let filter = String::from_utf8(vec![b'.', b'[', b'"', 0xc3, 0xa9, b'"', b']', b' ', b'%'])
            .expect("valid UTF-8");
        let error = wont_compile(&filter);
        assert_eq!(error.offset(), 8);
        assert_eq!((error.line(), error.column()), (1, 8));
    }

    #[test]
    fn a_filter_spanning_lines_reports_the_line_it_failed_on() {
        let error = wont_compile(".a |\n.b %");
        assert_eq!((error.line(), error.column()), (2, 4));
        assert_eq!(
            error.to_string(),
            "filter, line 2, column 4: `%` has no meaning here"
        );
    }

    #[test]
    fn parentheses_may_not_nest_without_end() {
        let deep = "(".repeat(200) + "." + &")".repeat(200);
        assert_eq!(
            *wont_compile(&deep).kind(),
            FilterErrorKind::DepthLimitExceeded {
                limit: super::MAX_DEPTH
            }
        );
    }
}
