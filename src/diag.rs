//! Rendering a parse failure the way `rustc` does: the offending source line,
//! and a caret under the character that was wrong.
//!
//! This is what a project would normally reach for `annotate-snippets` or
//! `codespan-reporting` to do. What those buy is multi-span layout and a
//! Unicode display-width table; neither is needed to point at one position in
//! one line, and both are larger than this file.
//!
//! One rule holds the module together: a column of the source line is one caret
//! position. That is the unit [`ParseError::column`] counts in, so the caret
//! cannot drift away from the number in the summary line printed above it.

use crate::color::{Ink, Paint};
use crate::error::ParseError;

/// How many columns of the source line to show.
///
/// JSON is very often minified onto a single line, so a failure late in a large
/// document must not print the document to standard error.
const SHOWN_COLUMNS: usize = 96;

/// How much of the line to keep to the left of the caret when the line is too
/// long to show in full.
const LEAD_COLUMNS: usize = 40;

/// What a tab expands to in the shown line. Four is what `rustc` uses.
const TAB_WIDTH: usize = 4;

/// Marks a line that has been cut. ASCII, so its width is its length.
const ELLIPSIS: &str = "...";

/// Render the source line `error` points into, with a caret beneath it.
///
/// The summary line is [`ParseError`]'s `Display` and belongs to the caller;
/// this is the snippet alone, and carries no trailing newline.
#[must_use]
pub fn snippet(input: &[u8], error: &ParseError) -> String {
    snippet_painted(input, error, Paint::Never)
}

/// The same, coloured the way `rustc` colours its own: a blue gutter and a red
/// caret.
///
/// Colour is a parameter rather than a decision made here, and the reason is worth
/// stating: diagnostics go to standard error, and whether *that* is a terminal is
/// a different question from whether standard output is. `jaq-lite . big.json >
/// out.json` should still draw a red caret on the console it is being watched
/// from.
///
/// With [`Paint::Never`] the bytes are exactly the ones [`snippet()`] produces. A
/// test asserts that by stripping every escape from a coloured snippet and
/// comparing what is left, rather than by reading two format strings and hoping
/// they agree.
#[must_use]
pub fn snippet_painted(input: &[u8], error: &ParseError, paint: Paint) -> String {
    snippet_at_painted(input, error.offset(), error.line(), error.column(), paint)
}

/// Render a snippet for any position in `input`, given the byte `offset` it is
/// at and the 1-based `line` and `column` derived from that offset.
#[must_use]
pub fn snippet_at(input: &[u8], offset: usize, line: usize, column: usize) -> String {
    snippet_at_painted(input, offset, line, column, Paint::Never)
}

/// The same, coloured.
///
/// Five parameters is one more than this wants, but the alternative is a struct
/// that would exist only to be destructured on the first line.
#[must_use]
pub fn snippet_at_painted(
    input: &[u8],
    offset: usize,
    line: usize,
    column: usize,
    paint: Paint,
) -> String {
    let cols = columns(line_of(input, offset));
    let caret = column.saturating_sub(1).min(cols.len());
    let (from, to) = window(cols.len(), caret);

    let mut shown = String::new();
    let mut pad = 0;
    if from > 0 {
        shown.push_str(ELLIPSIS);
        pad += ELLIPSIS.len();
    }
    for (i, col) in cols[from..to].iter().enumerate() {
        let width = render(col, &mut shown);
        if from + i < caret {
            pad += width;
        }
    }
    if to < cols.len() {
        shown.push_str(ELLIPSIS);
    }

    // Layout whitespace -- the space after each bar, and the run of spaces before
    // the caret -- stays outside every escape run. That is the rule the serializer
    // follows, and here it is what keeps `trim_end` working: with colour on the
    // run has already closed before the space it would otherwise strand.
    let number = line.to_string();
    let gutter = " ".repeat(number.len());
    let rule = paint.open(Ink::Gutter);
    let tip = paint.open(Ink::Caret);
    let off = paint.close();
    let bar = format!("{rule}{gutter} |{off}");
    let text = format!("{rule}{number} |{off} {shown}");
    let mark = format!("{rule}{gutter} |{off} {}{tip}^{off}", " ".repeat(pad));
    [bar.as_str(), text.trim_end(), mark.as_str()].join("\n")
}

/// The bytes of the line `offset` falls in, without its line ending.
fn line_of(input: &[u8], offset: usize) -> &[u8] {
    let at = offset.min(input.len());
    let start = match input[..at].iter().rposition(|&b| b == b'\n') {
        Some(i) => i + 1,
        None => 0,
    };
    let end = match input[start..].iter().position(|&b| b == b'\n') {
        Some(i) => start + i,
        None => input.len(),
    };
    // A CRLF ending leaves the carriage return at the end of the line, where it
    // would draw as a stray column and push everything after it one place right.
    if end > start && input[end - 1] == b'\r' {
        &input[start..end - 1]
    } else {
        &input[start..end]
    }
}

/// Split a line into columns: one non-continuation byte, plus any continuation
/// bytes that follow it.
///
/// This is deliberately the rule [`ParseError`] counts columns by rather than a
/// `char_indices` walk, because the line may not be valid UTF-8 at all -- that
/// is one of the failures this renderer has to draw.
fn columns(line: &[u8]) -> Vec<&[u8]> {
    let starts: Vec<usize> = line
        .iter()
        .enumerate()
        .filter(|&(_, &b)| (b & 0xC0) != 0x80)
        .map(|(i, _)| i)
        .collect();
    if starts.is_empty() {
        // Either nothing at all, or continuation bytes with no character to
        // belong to, which draw as the one column they cannot be part of.
        return if line.is_empty() {
            Vec::new()
        } else {
            vec![line]
        };
    }
    starts
        .iter()
        .enumerate()
        .map(|(k, &s)| {
            // The first column absorbs any continuation bytes ahead of it: they
            // belong to no character and the column count skips them, so the
            // caret must not be shifted by them either.
            let from = if k == 0 { 0 } else { s };
            let to = starts.get(k + 1).copied().unwrap_or(line.len());
            &line[from..to]
        })
        .collect()
}

/// Append one column's printable form to `into`, and report how many caret
/// positions it took.
fn render(column: &[u8], into: &mut String) -> usize {
    if column.is_empty() {
        return 0;
    }
    if column == b"\t" {
        for _ in 0..TAB_WIDTH {
            into.push(' ');
        }
        return TAB_WIDTH;
    }
    // A control byte moves the cursor instead of printing, and invalid UTF-8 has
    // no character to print. Both become one replacement character, which keeps
    // every column exactly one caret position wide.
    let control = column.len() == 1 && (column[0] < 0x20 || column[0] == 0x7f);
    match std::str::from_utf8(column) {
        Ok(text) if !control => into.push_str(text),
        _ => into.push(char::REPLACEMENT_CHARACTER),
    }
    1
}

/// Which columns of a line of `total` columns to show, as a half-open range,
/// keeping the caret inside the result.
fn window(total: usize, caret: usize) -> (usize, usize) {
    if total <= SHOWN_COLUMNS {
        return (0, total);
    }
    let to = (caret.saturating_sub(LEAD_COLUMNS) + SHOWN_COLUMNS).min(total);
    (to.saturating_sub(SHOWN_COLUMNS), to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    fn at(kind: ErrorKind, input: &[u8], offset: usize) -> ParseError {
        ParseError::new(kind, input, offset)
    }

    /// The character the caret points at, which is what every test here is
    /// really about. The gutter is the same width on both lines, and the caret
    /// line is all ASCII, so its byte index is a character index into the other.
    fn pointed_at(snippet: &str) -> char {
        let lines: Vec<&str> = snippet.lines().collect();
        assert_eq!(lines.len(), 3, "a snippet is three lines:\n{snippet}");
        let caret = lines[2].find('^').expect("a caret");
        lines[1].chars().nth(caret).unwrap_or(' ')
    }

    #[test]
    fn the_caret_lands_under_the_reported_column() {
        let input = b"{1:2}";
        let drawn = snippet(input, &at(ErrorKind::ExpectedObjectKey, input, 1));
        assert_eq!(drawn, "  |\n1 | {1:2}\n  |  ^");
        assert_eq!(pointed_at(&drawn), '1');
    }

    #[test]
    fn the_gutter_widens_with_the_line_number() {
        let mut input = vec![b'\n'; 11];
        input.push(b'x');
        let kind = ErrorKind::UnexpectedByte { byte: b'x' };
        let drawn = snippet(&input, &at(kind, &input, 11));
        assert_eq!(drawn, "   |\n12 | x\n   | ^");
    }

    #[test]
    fn a_tab_is_expanded_and_the_caret_follows_it() {
        let input = b"[\n\tx]";
        let kind = ErrorKind::UnexpectedByte { byte: b'x' };
        let drawn = snippet(input, &at(kind, input, 3));
        assert_eq!(drawn, "  |\n2 |     x]\n  |     ^");
        assert_eq!(pointed_at(&drawn), 'x');
    }

    #[test]
    fn a_multibyte_character_does_not_shift_the_caret() {
        // `["<e-acute>",x]` -- the x is character 6 but byte 7, and the caret
        // has to land on it. Counting bytes would point at the comma.
        let input = b"[\"\xc3\xa9\",x]";
        let kind = ErrorKind::UnexpectedByte { byte: b'x' };
        let drawn = snippet(input, &at(kind, input, 6));
        assert_eq!(pointed_at(&drawn), 'x');
    }

    #[test]
    fn invalid_utf8_draws_as_one_replacement_character() {
        // The renderer has to survive this: input that is not UTF-8 is one of
        // the errors it exists to report, so it cannot decode before drawing.
        let input = b"[\xff]";
        let kind = ErrorKind::InvalidUtf8 { valid_up_to: 1 };
        let drawn = snippet(input, &at(kind, input, 1));
        assert_eq!(pointed_at(&drawn), char::REPLACEMENT_CHARACTER);
        assert_eq!(drawn.lines().nth(1).unwrap().chars().count(), 7);
    }

    #[test]
    fn a_control_byte_draws_without_moving_the_cursor() {
        let input = b"[\"a\x01b\"]";
        let kind = ErrorKind::ControlCharacterInString { byte: 1 };
        let drawn = snippet(input, &at(kind, input, 3));
        assert_eq!(pointed_at(&drawn), char::REPLACEMENT_CHARACTER);
    }

    #[test]
    fn a_failure_at_end_of_input_points_just_past_the_last_character() {
        let input = b"[1,";
        let drawn = snippet(input, &at(ErrorKind::UnexpectedEof, input, 3));
        let lines: Vec<&str> = drawn.lines().collect();
        assert_eq!(lines[1], "1 | [1,");
        assert_eq!(lines[2].find('^'), Some(lines[1].chars().count()));
    }

    #[test]
    fn a_long_line_is_cut_around_the_caret() {
        // Minified JSON is one line, and it can be a very long one. What must
        // not happen is the whole document arriving on standard error.
        let mut input = vec![b'0'; 500];
        input[400] = b'!';
        let drawn = snippet(&input, &at(ErrorKind::InvalidNumber, &input, 400));
        let shown = drawn.lines().nth(1).unwrap();
        assert!(shown.starts_with("1 | ..."), "got {shown}");
        assert!(shown.ends_with("..."), "got {shown}");
        assert!(
            shown.chars().count() < 110,
            "{} columns",
            shown.chars().count()
        );
        assert_eq!(pointed_at(&drawn), '!');
    }

    #[test]
    fn a_carriage_return_before_the_newline_is_not_a_column() {
        let input = b"[1,\r\n2]";
        let kind = ErrorKind::UnexpectedByte { byte: b'2' };
        let drawn = snippet(input, &at(kind, input, 5));
        assert_eq!(drawn.lines().nth(1).unwrap(), "2 | 2]");
    }

    #[test]
    fn an_empty_line_still_draws_a_caret() {
        let drawn = snippet(b"", &at(ErrorKind::EmptyInput, b"", 0));
        assert_eq!(drawn, "  |\n1 |\n  | ^");
    }

    /// The error the real parser reports for a malformed input.
    ///
    /// The other tests here build a `ParseError` by hand so they can aim the caret
    /// anywhere. The two below do not care where it lands, only that colour does
    /// not move it, so they take whatever the parser says.
    fn first_error(input: &[u8]) -> ParseError {
        crate::parse(input).unwrap_err()
    }

    /// Remove every SGR escape run, leaving the text they wrapped.
    fn strip_sgr(text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find('\x1b') {
            out.push_str(&rest[..start]);
            let end = rest[start..].find('m').expect("an SGR run ends with m") + start;
            rest = &rest[end + 1..];
        }
        out.push_str(rest);
        out
    }

    #[test]
    fn colour_off_is_byte_for_byte_the_uncoloured_snippet() {
        let input = b"[1,";
        let error = first_error(input);
        assert_eq!(
            snippet_painted(input, &error, Paint::Never),
            snippet(input, &error)
        );
    }

    #[test]
    fn stripping_the_escapes_gives_back_the_plain_snippet() {
        // The claim this module rests on: colour adds bytes and moves nothing.
        for input in [&b"{1:2}"[..], &b"[\n\tx]"[..], &b"[1,"[..], &b"tru"[..]] {
            let error = first_error(input);
            let plain = snippet(input, &error);
            let painted = snippet_painted(input, &error, Paint::Always);
            assert_ne!(painted, plain, "nothing was coloured for {input:?}");
            assert_eq!(strip_sgr(&painted), plain, "input was {input:?}");
        }
    }

    #[test]
    fn the_runs_land_exactly_where_they_belong() {
        // The same hand-built case as `the_caret_lands_under_the_reported_column`,
        // so the coloured and uncoloured expectations can be read side by side.
        let input = b"{1:2}";
        let error = at(ErrorKind::ExpectedObjectKey, input, 1);
        // Two spaces before the caret run and both outside it: one belongs to the
        // gutter's layout, the other is the indent that aims the caret.
        let expected = concat!(
            "\x1b[1;34m  |\x1b[0m\n",
            "\x1b[1;34m1 |\x1b[0m {1:2}\n",
            "\x1b[1;34m  |\x1b[0m  \x1b[1;31m^\x1b[0m",
        );
        assert_eq!(snippet_painted(input, &error, Paint::Always), expected);
    }
}
