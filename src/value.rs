//! The JSON value model.
//!
//! Two decisions here are worth stating up front, because they are the reason
//! this is not simply an enum of Rust primitives.
//!
//! A number keeps the exact bytes it was written with. `1.0`, `1E+2` and a
//! thirty-digit integer all survive a round trip unchanged, because the
//! serializer writes the stored text back rather than reformatting an `f64`.
//! Reformatting is where most JSON tools quietly lose information, and it is
//! also what would force this crate to reimplement float printing.
//!
//! An object is a `Vec` of pairs, not a map. Insertion order is preserved,
//! which is what `jq` does and what makes byte-exact round-tripping possible.
//! The cost is an O(n) key lookup, which is stated in the README rather than
//! hidden.

use std::fmt;

/// A JSON value.
///
/// The derived `PartialEq` compares numbers numerically and objects in order.
/// For comparison by representation instead, use `Value::identical`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The literal `null`.
    Null,
    /// The literals `true` and `false`.
    Bool(bool),
    /// A number literal.
    Number(Number),
    /// A string with every escape sequence already resolved.
    String(String),
    /// An array, in document order.
    Array(Vec<Value>),
    /// An object, in insertion order.
    ///
    /// The type can hold a repeated key; `parse` never returns one. The parser's
    /// policy is last value wins, kept at the position where the key first
    /// appeared -- see `insert` in `parser.rs` -- and a round-trip property in
    /// `tests/roundtrip_fuzz.rs` rests on that. This comment claimed the opposite
    /// until that test was written believing it.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The name jq uses for this value's type.
    ///
    /// jq's spellings, not Rust's: a `Bool` is a "boolean". These words appear
    /// in error messages and are what the `type` filter will return, so there is
    /// only one place to get them wrong.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
        }
    }

    /// Compare two values by representation rather than by numeric value.
    ///
    /// This differs from `==` in exactly one place: two numbers are identical
    /// only if their literal text matches. `1.0` and `1` are equal but not
    /// identical, and `1e400` and `1e500` are equal, since both parse to
    /// infinity, while being obviously different documents. Round-trip tests
    /// want this function; a query engine wants `==`.
    pub fn identical(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a.as_str() == b.as_str(),
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
            }
            (Self::Object(a), Self::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|((ka, va), (kb, vb))| ka == kb && va.identical(vb))
            }
            _ => false,
        }
    }
}

/// A JSON number: the literal text it was parsed from, plus its `f64` value.
#[derive(Debug, Clone)]
pub struct Number {
    raw: Box<str>,
    val: f64,
}

impl Number {
    /// Build a number from its literal text and its numeric interpretation.
    pub fn new(raw: impl Into<Box<str>>, val: f64) -> Self {
        Self {
            raw: raw.into(),
            val,
        }
    }

    /// The literal text, exactly as it appeared in the input.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The `f64` interpretation, which may be infinite for a literal that
    /// overflows and may lose precision for one that exceeds 53 significant
    /// bits. The literal text is unaffected either way.
    pub fn as_f64(&self) -> f64 {
        self.val
    }

    /// A count, as a number whose text is its decimal spelling.
    ///
    /// The second of the two places in this crate that may build a number, and it
    /// exists because `length` and `keys` have to answer with one. The call below
    /// is written `Number::new` rather than the `Self::new` an impl block would
    /// normally use, on purpose: `tests/claims.rs` counts that call spelling
    /// across `src/`, and a check a rename can silence is not a check. Entry 7 of
    /// `STDLIB.md` is the claim this is holding up.
    ///
    /// The paren is left off both names here so this sentence is not itself one of
    /// the call sites the grep finds -- which it was, for about an hour.
    ///
    /// This is also the one place in the program where numeric text is generated
    /// rather than reproduced. `usize::to_string()` is the standard library's
    /// integer formatter, which is what `itoa` exists to be faster than; a count
    /// printed once per document is not where that matters. No `f64` is ever
    /// turned into text anywhere here, which is the half of the substitution that
    /// does.
    #[must_use]
    pub fn from_count(count: usize) -> Self {
        let text = count.to_string();
        let val = text
            .parse::<f64>()
            .expect("a decimal integer is valid f64 syntax");
        Number::new(text, val)
    }

    /// The same number without its sign, keeping the literal's own spelling.
    ///
    /// `length` on a number is its magnitude. Taking it by slicing the minus off
    /// the text, rather than by rendering `-val`, is what lets a thirty-digit
    /// integer keep all thirty digits; it is also where this parts company with
    /// jq, which re-renders and answers `1E+3` for `1e3 | length`.
    #[must_use]
    pub fn magnitude(&self) -> Self {
        let raw = self.as_str();
        let text = raw.strip_prefix('-').unwrap_or(raw);
        Number::new(text, self.val.abs())
    }
}

/// Numeric equality, not textual. See `Value::identical` for the other one.
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}
