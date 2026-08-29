//! Turning a `Value` back into JSON text.
//!
//! The target is byte-for-byte agreement with `jq` 1.8.1, because a tool that
//! claims to be a drop-in replacement can be checked against the original and
//! this one is. The layout rules were measured rather than assumed: two-space
//! indentation, one element per line for a non-empty container, `[]` and `{}`
//! printed inline, a space after the colon in an object but none before it, and
//! no trailing newline (the caller adds that).
//!
//! Escaping is the part people get wrong. `/` is left alone, since escaping it
//! is legal but not required and `jq` does not. Bytes at or above `0x80` are
//! written through untouched, so text that arrived as UTF-8 leaves as UTF-8
//! rather than as a wall of `\u` escapes. The seven characters with a
//! two-character escape get it; everything else below `0x20`, plus `0x7F`, gets
//! a lowercase four-digit `\u` escape.

use crate::{Ink, Paint, Style, Value};
use std::io::{self, Write};

/// One level of pretty-printed indentation.
const INDENT: &[u8] = b"  ";

/// Write one value at the given nesting depth.
///
/// Recursion here mirrors the shape of the value. Anything this crate parsed is
/// bounded by the parser's depth limit; a `Value` assembled by hand is not, so a
/// caller who builds one thousands of levels deep is responsible for it.
pub(crate) fn write_value<W: Write>(
    out: &mut W,
    value: &Value,
    style: Style,
    paint: Paint,
    depth: usize,
) -> io::Result<()> {
    match value {
        Value::Null => tinted(out, paint, Ink::Null, b"null"),
        Value::Bool(true) => tinted(out, paint, Ink::Bool, b"true"),
        Value::Bool(false) => tinted(out, paint, Ink::Bool, b"false"),
        // The source text, not a reformatting of the f64. This is the whole
        // reason the number keeps its raw form: 1e2 goes out as 1e2, and an
        // integer too large for i64 or too precise for f64 survives intact.
        Value::Number(number) => tinted(out, paint, Ink::Number, number.as_str().as_bytes()),
        Value::String(text) => write_string(out, text, paint, Ink::Str),
        Value::Array(items) => {
            // An empty container is one run holding both brackets rather than
            // two runs of one bracket each. Measured, not assumed.
            if items.is_empty() {
                return tinted(out, paint, Ink::Array, b"[]");
            }
            tinted(out, paint, Ink::Array, b"[")?;
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    tinted(out, paint, Ink::Array, b",")?;
                }
                break_line(out, style, depth + 1)?;
                write_value(out, item, style, paint, depth + 1)?;
            }
            break_line(out, style, depth)?;
            tinted(out, paint, Ink::Array, b"]")
        }
        Value::Object(entries) => {
            if entries.is_empty() {
                return tinted(out, paint, Ink::Object, b"{}");
            }
            tinted(out, paint, Ink::Object, b"{")?;
            for (index, (key, member)) in entries.iter().enumerate() {
                if index > 0 {
                    tinted(out, paint, Ink::Object, b",")?;
                }
                break_line(out, style, depth + 1)?;
                write_string(out, key, paint, Ink::Key)?;
                // The colon belongs inside the coloured run and the space after
                // it does not, which is where jq draws the line. With colour off
                // these two writes still put exactly `: ` on the wire.
                tinted(out, paint, Ink::Object, b":")?;
                if style == Style::Pretty {
                    out.write_all(b" ")?;
                }
                write_value(out, member, style, paint, depth + 1)?;
            }
            break_line(out, style, depth)?;
            tinted(out, paint, Ink::Object, b"}")
        }
    }
}

/// Write `text` wrapped in `ink`, or exactly `text` when colour is off.
///
/// The escapes are skipped rather than written as empty slices, so an uncoloured
/// run makes the same calls into `out` that it made before this module existed.
fn tinted<W: Write>(out: &mut W, paint: Paint, ink: Ink, text: &[u8]) -> io::Result<()> {
    if !paint.on() {
        return out.write_all(text);
    }
    out.write_all(paint.open(ink).as_bytes())?;
    out.write_all(text)?;
    out.write_all(paint.close().as_bytes())
}

/// End a line and indent, or do nothing at all in compact style.
fn break_line<W: Write>(out: &mut W, style: Style, depth: usize) -> io::Result<()> {
    if style == Style::Compact {
        return Ok(());
    }
    out.write_all(b"\n")?;
    for _ in 0..depth {
        out.write_all(INDENT)?;
    }
    Ok(())
}

/// Write a quoted, escaped string.
///
/// Ordinary bytes are written in runs rather than one at a time. Every byte that
/// interrupts a run is ASCII, so a run boundary can never land inside a
/// multi-byte character and the slices stay valid UTF-8 without any bookkeeping.
fn write_string<W: Write>(out: &mut W, text: &str, paint: Paint, ink: Ink) -> io::Result<()> {
    // The quotes go inside the coloured run, which is where jq puts them.
    if paint.on() {
        out.write_all(paint.open(ink).as_bytes())?;
    }
    out.write_all(b"\"")?;
    let bytes = text.as_bytes();
    let mut run = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        let short: Option<&[u8]> = match byte {
            b'"' => Some(b"\\\""),
            b'\\' => Some(b"\\\\"),
            0x08 => Some(b"\\b"),
            0x0c => Some(b"\\f"),
            b'\n' => Some(b"\\n"),
            b'\r' => Some(b"\\r"),
            b'\t' => Some(b"\\t"),
            // No two-character form exists for these, so they need \u. 0x7F is
            // in the list even though the RFC does not require escaping it,
            // because jq escapes it and matching jq is the point.
            0x00..=0x1f | 0x7f => None,
            // Everything else, including `/` and every byte of a multi-byte
            // character, goes out as it came in.
            _ => continue,
        };
        out.write_all(&bytes[run..index])?;
        match short {
            Some(sequence) => out.write_all(sequence)?,
            None => write!(out, "\\u{byte:04x}")?,
        }
        run = index + 1;
    }
    out.write_all(&bytes[run..])?;
    out.write_all(b"\"")?;
    if paint.on() {
        out.write_all(paint.close().as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{Style, Value, parse, to_string};

    fn pretty(src: &str) -> String {
        to_string(&parse(src.as_bytes()).expect(src), Style::Pretty)
    }

    fn compact(src: &str) -> String {
        to_string(&parse(src.as_bytes()).expect(src), Style::Compact)
    }

    #[test]
    fn the_pretty_layout_matches_jq() {
        let out = pretty(r#"{"a":[1,{"b":null}],"c":{},"d":[]}"#);
        assert_eq!(
            out,
            "{\n  \"a\": [\n    1,\n    {\n      \"b\": null\n    }\n  ],\n  \"c\": {},\n  \"d\": []\n}"
        );
    }

    #[test]
    fn empty_containers_stay_on_one_line() {
        assert_eq!(pretty("[]"), "[]");
        assert_eq!(pretty("{}"), "{}");
        assert_eq!(pretty("[[],{}]"), "[\n  [],\n  {}\n]");
    }

    #[test]
    fn compact_output_contains_no_whitespace() {
        let out = compact(r#" { "a" : [ 1 , 2 ] , "b" : { "c" : true } } "#);
        assert_eq!(out, r#"{"a":[1,2],"b":{"c":true}}"#);
        assert!(
            !out.contains(' ') && !out.contains('\n'),
            "compact output must have no whitespace: {out}"
        );
    }

    #[test]
    fn the_seven_short_escapes_are_preferred_over_u_escapes() {
        assert_eq!(compact(r#""\"\\\b\f\n\r\t""#), r#""\"\\\b\f\n\r\t""#);
    }

    #[test]
    fn everything_else_below_0x20_and_delete_get_lowercase_u_escapes() {
        assert_eq!(
            compact(r#""\u0000\u001F\u007F""#),
            r#""\u0000\u001f\u007f""#
        );
    }

    #[test]
    fn the_solidus_is_not_escaped_on_the_way_out() {
        assert_eq!(compact(r#""a\/b""#), r#""a/b""#);
        assert_eq!(compact(r#""a/b""#), r#""a/b""#);
    }

    #[test]
    fn multibyte_text_is_written_through_not_escaped() {
        assert_eq!(
            compact("\"h\u{e9}llo \u{1f600}\""),
            "\"h\u{e9}llo \u{1f600}\""
        );
        // The escaped and literal spellings of a character converge on output.
        assert_eq!(compact(r#""\u00e9""#), "\"\u{e9}\"");
    }

    #[test]
    fn number_text_is_reproduced_not_reformatted() {
        for src in ["0", "-0", "1e2", "1E+2", "0.10", "1.0", "1e999"] {
            assert_eq!(compact(src), src, "{src} was rewritten");
        }
        // Values that no f64 can hold still print exactly as written.
        assert_eq!(
            compact("123456789012345678901234567890"),
            "123456789012345678901234567890"
        );
    }

    #[test]
    fn member_order_survives_the_round_trip() {
        assert_eq!(compact(r#"{"z":1,"a":2,"m":3}"#), r#"{"z":1,"a":2,"m":3}"#);
    }

    #[test]
    fn a_repeated_key_is_collapsed_the_way_jq_collapses_it() {
        // Verified against jq 1.8.1: first position, last value.
        assert_eq!(compact(r#"{"b":1,"a":2,"b":3}"#), r#"{"b":3,"a":2}"#);
    }

    #[test]
    fn there_is_no_trailing_newline() {
        let out = to_string(&Value::Null, Style::Pretty);
        assert_eq!(out, "null", "the caller owns the trailing newline");
    }
}

#[cfg(test)]
mod colour_tests {
    use crate::{Paint, Style, parse, to_string, write_painted};

    /// A spread wide enough to reach every arm of the serializer.
    const SAMPLES: [&str; 8] = [
        "null",
        "true",
        "1e2",
        r#""s""#,
        "[]",
        "{}",
        r#"[1,"x",null]"#,
        r#"{"a":[1,{"b":{}}],"c":[]}"#,
    ];

    /// Serialize with colour on.
    fn lit(src: &str, style: Style) -> String {
        let value = parse(src.as_bytes()).expect(src);
        let mut bytes = Vec::new();
        write_painted(&mut bytes, &value, style, Paint::Always).expect("a Vec cannot fail");
        String::from_utf8(bytes).expect("the serializer only emits valid UTF-8")
    }

    /// Remove every SGR sequence, leaving the JSON that carried them.
    ///
    /// SGR parameters are digits and semicolons, so the first `m` after the
    /// escape is always the terminator and this needs no real parser.
    fn stripped(text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find('\x1b') {
            out.push_str(&rest[..start]);
            let tail = &rest[start..];
            let end = tail.find('m').expect("an SGR sequence with no terminator");
            rest = &tail[end + 1..];
        }
        out.push_str(rest);
        out
    }

    #[test]
    fn every_scalar_gets_the_code_jq_gives_it() {
        // Each of these was read off `jq -C -c .` under jq 1.8.1 with `od -c`.
        assert_eq!(lit("null", Style::Compact), "\x1b[0;90mnull\x1b[0m");
        assert_eq!(lit("true", Style::Compact), "\x1b[0;39mtrue\x1b[0m");
        assert_eq!(lit("false", Style::Compact), "\x1b[0;39mfalse\x1b[0m");
        assert_eq!(lit("1", Style::Compact), "\x1b[0;39m1\x1b[0m");
        assert_eq!(lit(r#""s""#, Style::Compact), "\x1b[0;32m\"s\"\x1b[0m");
    }

    #[test]
    fn an_empty_container_is_a_single_run_not_two() {
        assert_eq!(lit("[]", Style::Compact), "\x1b[1;39m[]\x1b[0m");
        assert_eq!(lit("{}", Style::Compact), "\x1b[1;39m{}\x1b[0m");
    }

    #[test]
    fn punctuation_takes_the_colour_of_its_container() {
        // The brace, the colon and the closing brace are object-coloured; the
        // key has its own colour; each mark is a separate run.
        assert_eq!(
            lit(r#"{"a":1}"#, Style::Compact),
            "\x1b[1;39m{\x1b[0m\x1b[1;34m\"a\"\x1b[0m\x1b[1;39m:\x1b[0m\x1b[0;39m1\x1b[0m\x1b[1;39m}\x1b[0m"
        );
        // And the comma between array elements is array-coloured.
        assert_eq!(
            lit("[1,2]", Style::Compact),
            "\x1b[1;39m[\x1b[0m\x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m\x1b[0;39m2\x1b[0m\x1b[1;39m]\x1b[0m"
        );
    }

    #[test]
    fn indentation_and_the_colon_space_sit_outside_the_runs() {
        // One line of this test per line of the `jq -C . | cat -v` transcript it
        // was taken from. The two leading spaces and the space after the colon
        // are outside every escape, which is the detail a reimplementation is
        // most likely to get wrong.
        let want = concat!(
            "\x1b[1;39m{\x1b[0m\n",
            "  \x1b[1;34m\"a\"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[1;39m[\x1b[0m\n",
            "    \x1b[0;39m1\x1b[0m\n",
            "  \x1b[1;39m]\x1b[0m\n",
            "\x1b[1;39m}\x1b[0m",
        );
        assert_eq!(lit(r#"{"a":[1]}"#, Style::Pretty), want);
    }

    #[test]
    fn colour_off_is_byte_identical_to_no_colour_at_all() {
        // This is the assertion that lets the conformance corpus and the
        // round-trip property stay indifferent to this module existing.
        for src in SAMPLES {
            let value = parse(src.as_bytes()).expect(src);
            for style in [Style::Pretty, Style::Compact] {
                let mut bytes = Vec::new();
                write_painted(&mut bytes, &value, style, Paint::Never).expect("a Vec cannot fail");
                let plain = String::from_utf8(bytes).expect("valid UTF-8");
                assert_eq!(
                    plain,
                    to_string(&value, style),
                    "{src} changed with colour off"
                );
                assert!(
                    !plain.contains('\x1b'),
                    "{src} leaked an escape with colour off"
                );
            }
        }
    }

    #[test]
    fn removing_the_escapes_gives_back_exactly_the_plain_bytes() {
        // Stated as a property rather than as a promise: colour may add escapes
        // and may add nothing else.
        for src in SAMPLES {
            let value = parse(src.as_bytes()).expect(src);
            for style in [Style::Pretty, Style::Compact] {
                let coloured = lit(src, style);
                assert!(coloured.contains('\x1b'), "{src} came back with no colour");
                assert_eq!(
                    stripped(&coloured),
                    to_string(&value, style),
                    "{src}: colour changed the JSON and not just its escapes"
                );
            }
        }
    }
}
