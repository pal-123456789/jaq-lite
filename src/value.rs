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
    /// An object, in insertion order, duplicate keys retained as they appeared.
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
