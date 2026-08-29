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
//! # Depth
//!
//! Parenthesis nesting is capped, for the same reason the JSON parser caps
//! container nesting: recursive descent meets unbounded nesting as a stack
//! overflow. The cap is lower here (64) because no human writes a filter
//! anywhere near that deep. Note that the length of a *path* is not capped:
//! `.a.a.a...` makes `eval` recurse once per step. That is acceptable where
//! unbounded JSON nesting is not, because a filter is program text the user
//! typed, while a document is untrusted input that arrived from somewhere else.

use crate::lexer::Cursor;
use crate::value::Value;
use core::fmt;

/// How deeply parentheses may nest.
const MAX_DEPTH: u32 = 64;

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
    /// Returns the reason and the byte offset if the filter does not parse.
    pub fn compile(source: &str) -> Result<Self, FilterError> {
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
    Field(Box<Node>, String),
    /// `a[0]`, `a[-1]`
    Index(Box<Node>, i64),
    /// `a[]`
    Iterate(Box<Node>),
    /// `a?`
    Optional(Box<Node>),
}

// -- errors -----------------------------------------------------------------

/// Why a filter could not be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterError {
    kind: FilterErrorKind,
    offset: usize,
}

impl FilterError {
    fn new(kind: FilterErrorKind, offset: usize) -> Self {
        Self { kind, offset }
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
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "filter, column {}: {}", self.offset + 1, self.kind)
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
    /// Parentheses nested past [`MAX_DEPTH`].
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
/// These are jq's runtime errors. They carry the type name rather than the
/// value, because a message that quotes a whole document back at you is not a
/// message anyone reads.
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
    },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotIndexableByName { found, name } => {
                write!(f, "cannot index {found} with \"{name}\"")
            }
            Self::NotIndexableByNumber { found } => write!(f, "cannot index {found} with a number"),
            Self::NotIterable { found } => write!(f, "cannot iterate over {found}"),
        }
    }
}

impl std::error::Error for EvalError {}

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
        loop {
            if matches!(self.peek(), Some(Token::Dot)) {
                self.at += 1;
                node = self.parse_step(node)?;
            } else if matches!(self.peek(), Some(Token::OpenBracket)) {
                self.at += 1;
                node = self.parse_bracket(node)?;
            } else if matches!(self.peek(), Some(Token::Question)) {
                // `?` covers the whole chain to its left, which is what jq does:
                // `.a.b?` swallows a failure at `.a` as well as at `.b`.
                self.at += 1;
                node = Node::Optional(Box::new(node));
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
        /// Read out of the token before touching `self` again, so the borrow the
        /// name comes from is over before the cursor moves.
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
            Step::Name(name) => Ok(Node::Field(Box::new(source), name)),
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
            Inside::Iterate => Ok(Node::Iterate(Box::new(source))),
            Inside::Name(name) => {
                self.expect(&Token::CloseBracket, "]")?;
                Ok(Node::Field(Box::new(source), name))
            }
            Inside::At(value) => {
                self.expect(&Token::CloseBracket, "]")?;
                Ok(Node::Index(Box::new(source), value))
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
        Node::Field(source, name) => {
            let mut values = Vec::new();
            eval(source, input, &mut values)?;
            for value in &values {
                out.push(field(value, name)?);
            }
            Ok(())
        }
        Node::Index(source, index) => {
            let mut values = Vec::new();
            eval(source, input, &mut values)?;
            for value in &values {
                out.push(at(value, *index)?);
            }
            Ok(())
        }
        Node::Iterate(source) => {
            let mut values = Vec::new();
            eval(source, input, &mut values)?;
            for value in &values {
                iterate(value, out)?;
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
    fn a_question_mark_turns_an_error_into_nothing() {
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
        // The error is inside the chain rather than at its end, and `?` still
        // covers it, because it applies to everything to its left.
        assert!(run(".a.b?", "1").is_empty());
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
        assert_eq!(
            fails(".a", "1"),
            EvalError::NotIndexableByName {
                found: "number",
                name: "a".to_owned()
            }
        );
        assert_eq!(
            fails(".[0]", r#"{"a":1}"#),
            EvalError::NotIndexableByNumber { found: "object" }
        );
        assert_eq!(
            fails(".[]", "null"),
            EvalError::NotIterable { found: "null" }
        );
        assert_eq!(
            fails(".[]", r#""s""#),
            EvalError::NotIterable { found: "string" }
        );
    }

    #[test]
    fn an_error_inside_a_pipeline_stops_the_pipeline() {
        assert_eq!(
            fails(".[] | .a", "[{},1]"),
            EvalError::NotIndexableByName {
                found: "number",
                name: "a".to_owned()
            }
        );
    }

    #[test]
    fn the_message_names_the_type_the_way_jq_does() {
        assert_eq!(
            fails(".a", "true").to_string(),
            "cannot index boolean with \"a\""
        );
        assert_eq!(fails(".[]", "1").to_string(), "cannot iterate over number");
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
        assert_eq!(
            error.to_string(),
            "filter, column 4: `%` has no meaning here"
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
