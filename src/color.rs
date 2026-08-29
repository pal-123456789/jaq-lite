//! ANSI colour, measured against `jq` 1.8.1 rather than copied from its manual.
//!
//! Two crates normally cover this ground: one to write the escapes, one to
//! decide whether to write them at all. Neither is needed. The escapes are a
//! short table of string constants, and `std::io::IsTerminal` has been stable
//! since Rust 1.70, so the standard library can answer the question the second
//! crate exists to answer.
//!
//! The table came out of running `jq -C` through a pipe and reading the bytes
//! with `cat -v` and `od -c`. Four things fell out of that measurement which a
//! manual would not have told us, and all four are load-bearing here:
//!
//! - Punctuation is coloured, and it takes the colour of the container it
//!   belongs to. A comma between array elements is array-coloured; the colon in
//!   an object is object-coloured. Each punctuation mark is its own run.
//! - An empty container is a *single* run holding both brackets, not two runs
//!   of one bracket each.
//! - Indentation, newlines, and the one space after a colon are written
//!   *outside* the escapes.
//! - The run always ends with SGR 0, never with SGR 39.
//!
//! An object key is coloured differently from a string value, which is the one
//! distinction a reader is most likely to miss when reimplementing this.
//!
//! Two inks at the end of the table are not jq's and were not measured: the
//! gutter and caret of a parse diagnostic. jq has no caret diagnostics, so there
//! was nothing to compare against; those two follow `rustc`, which is the tool a
//! reader has most likely seen this shape of error from.

/// The sequence that ends a coloured run.
///
/// `jq` resets fully rather than returning the foreground to its default, so a
/// run cannot leave the terminal in a state the next run depends on.
const RESET: &str = "\x1b[0m";

/// What is being written, which is what selects the colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    /// `null`.
    Null,
    /// `true` and `false`.
    Bool,
    /// A number, in the source text it arrived as.
    Number,
    /// A string that is a value rather than a key.
    Str,
    /// An array's brackets, and the commas between its elements.
    Array,
    /// An object's braces and colons, and the commas between its members.
    Object,
    /// An object key. `jq` gives these their own colour.
    Key,
    /// The line number and the `|` of a diagnostic's gutter.
    ///
    /// Not a JSON value, and not something `jq` draws at all. See the note at the
    /// top of this file about where the colour comes from instead.
    Gutter,
    /// The `^` under the character a parse error points at.
    Caret,
}

impl Ink {
    /// The opening escape sequence, as `jq` 1.8.1 emits it.
    const fn sgr(self) -> &'static str {
        match self {
            Self::Null => "\x1b[0;90m",
            // Booleans and numbers share a code in jq's default scheme. Keeping
            // them as separate inks costs nothing and means a future change to
            // one does not silently move the other.
            Self::Bool | Self::Number => "\x1b[0;39m",
            Self::Str => "\x1b[0;32m",
            // Arrays and objects also share a code, for the same reason.
            Self::Array | Self::Object => "\x1b[1;39m",
            Self::Key => "\x1b[1;34m",
            // Gutter shares a code with Key by coincidence rather than by
            // meaning, which is why it is a separate variant: a change to jq's
            // key colour should not silently move the gutter with it.
            Self::Gutter => "\x1b[1;34m",
            Self::Caret => "\x1b[1;31m",
        }
    }
}

/// Whether to emit escape sequences at all.
///
/// This exists so that colour is a parameter rather than a branch scattered
/// through the serializer. With [`Paint::Never`] every sequence below is the
/// empty string, so an uncoloured run emits the same bytes a build with no
/// colour support would have emitted -- which is what keeps the conformance
/// corpus and the round-trip property unaffected by this module.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Paint {
    /// Write no escapes. The default, because a pipe is the common case.
    #[default]
    Never,
    /// Write them.
    Always,
}

impl Paint {
    /// Whether escapes are being written.
    #[must_use]
    pub const fn on(self) -> bool {
        matches!(self, Self::Always)
    }

    /// The opening sequence for `ink`, or the empty string when colour is off.
    #[must_use]
    pub const fn open(self, ink: Ink) -> &'static str {
        if self.on() { ink.sgr() } else { "" }
    }

    /// The closing sequence, or the empty string when colour is off.
    #[must_use]
    pub const fn close(self) -> &'static str {
        if self.on() { RESET } else { "" }
    }
}

#[cfg(test)]
mod tests {
    use super::{Ink, Paint};

    #[test]
    fn the_default_is_no_colour() {
        assert_eq!(Paint::default(), Paint::Never);
        assert!(!Paint::Never.on());
        assert!(Paint::Always.on());
    }

    #[test]
    fn colour_off_yields_empty_strings_not_absent_ones() {
        // The serializer writes these unconditionally in some paths, so "off"
        // has to mean zero bytes rather than a shorter sequence.
        for ink in [
            Ink::Null,
            Ink::Bool,
            Ink::Number,
            Ink::Str,
            Ink::Array,
            Ink::Object,
            Ink::Key,
            Ink::Gutter,
            Ink::Caret,
        ] {
            assert_eq!(Paint::Never.open(ink), "");
        }
        assert_eq!(Paint::Never.close(), "");
    }

    #[test]
    fn every_code_is_the_one_jq_emits() {
        // Read off `jq -C` piped through `od -c`, one case per ink.
        assert_eq!(Paint::Always.open(Ink::Null), "\x1b[0;90m");
        assert_eq!(Paint::Always.open(Ink::Bool), "\x1b[0;39m");
        assert_eq!(Paint::Always.open(Ink::Number), "\x1b[0;39m");
        assert_eq!(Paint::Always.open(Ink::Str), "\x1b[0;32m");
        assert_eq!(Paint::Always.open(Ink::Array), "\x1b[1;39m");
        assert_eq!(Paint::Always.open(Ink::Object), "\x1b[1;39m");
        assert_eq!(Paint::Always.open(Ink::Key), "\x1b[1;34m");
        // A full reset, not a foreground-default. jq emits 0m and so do we.
        assert_eq!(Paint::Always.close(), "\x1b[0m");
    }

    #[test]
    fn the_diagnostic_inks_follow_rustc_because_jq_has_none() {
        // Nothing to measure these against, so they are pinned instead. A caret
        // being red is the only reason a reader's eye lands on it; changing that
        // by accident is a regression in the thing diagnostics exist for.
        assert_eq!(Paint::Always.open(Ink::Gutter), "\x1b[1;34m");
        assert_eq!(Paint::Always.open(Ink::Caret), "\x1b[1;31m");
        assert_eq!(Paint::Never.open(Ink::Gutter), "");
        assert_eq!(Paint::Never.open(Ink::Caret), "");
    }

    #[test]
    fn every_sequence_is_a_well_formed_sgr_run() {
        for ink in [
            Ink::Null,
            Ink::Bool,
            Ink::Number,
            Ink::Str,
            Ink::Array,
            Ink::Object,
            Ink::Key,
            Ink::Gutter,
            Ink::Caret,
        ] {
            let code = Paint::Always.open(ink);
            assert!(code.starts_with("\x1b["), "{code:?} does not open an SGR");
            assert!(code.ends_with('m'), "{code:?} does not end an SGR");
            assert!(code.is_ascii(), "{code:?} is not ASCII");
        }
    }
}
