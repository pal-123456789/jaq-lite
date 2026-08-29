//! Every diagnostic this tool can print, recorded byte for byte.
//!
//! A caret diagnostic is the one piece of output a user reads under stress, and it
//! is assembled from three places at once: the summary line from `ParseError`'s
//! `Display`, the snippet from `src/diag.rs`, and the prefix and the exit code
//! from `src/main.rs`. Nothing in the unit tests sees all three together. This
//! file does, because it runs the binary.
//!
//! The record is `tests/diagnostics.txt`, rewritten by setting
//! `UPDATE_DIAGNOSTICS=1` -- the arrangement `tests/i_decisions.tsv` already uses.
//!
//! A recorded file asserts nothing on its own. It says the tool does what it
//! currently does, which is worth having, because drift becomes a failing test
//! instead of a quiet change of behaviour; but it cannot say the behaviour is
//! right. So two of the three tests below never open it. One checks that the
//! column the summary line reports is the column the caret is drawn under, which
//! is two independently written code paths agreeing about the same position. The
//! other checks that `-C` adds exactly three gutter runs, one caret run and eight
//! escape bytes, and changes nothing else.
//!
//! `strip_sgr` is duplicated from the unit tests in `src/diag.rs` rather than
//! shared, for the reason `tests/color.rs` gives about its own helpers: each file
//! under `tests/` is its own crate, so sharing means a `common` module, and ten
//! lines are not worth the indirection.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EXE: &str = env!("CARGO_BIN_EXE_jaq-lite");

/// The byte that opens every SGR escape sequence.
const ESC: u8 = 0x1b;

/// What the record says about itself, so a reader who opens it first is not left
/// guessing where it came from or how to change it.
const HEADER: &str = "\
# Every diagnostic jaq-lite can print, captured from the binary by
# tests/diagnostics.rs and compared on every run. Regenerate after an intended
# change with UPDATE_DIAGNOSTICS=1, then read the diff: this file is the review
# surface for what a user sees when their JSON is wrong.
#
# Both streams are recorded, because the split matters -- a stream keeps the
# documents it printed before the bad one. `stdin` is shown ASCII-escaped, so the
# case that is deliberately not UTF-8 still reads as text.

";

/// One invocation of the binary.
struct Case {
    /// What the case demonstrates. Becomes its heading in the record.
    name: &'static str,
    /// The filter argument.
    filter: &'static str,
    /// The exact bytes fed to standard input.
    ///
    /// Bytes rather than a `&str`, because one case is deliberately not UTF-8 and
    /// a `&str` cannot hold it.
    stdin: Vec<u8>,
    /// The exit code this must produce. Predicted here rather than recorded, so it
    /// is an assertion and not a snapshot: 5 for a document that is not JSON, 3
    /// for a filter that does not compile, 0 for a stream that held no documents.
    exit: i32,
    /// Whether the caret's column can be compared against the summary line's.
    ///
    /// True only where the offending line is one line, ASCII, and short enough not
    /// to be truncated. Outside those bounds the caret is placed by display width
    /// and the byte arithmetic in the test would be asserting the wrong thing.
    aligned: bool,
}

fn case(name: &'static str, filter: &'static str, stdin: &[u8], exit: i32, aligned: bool) -> Case {
    Case {
        name,
        filter,
        stdin: stdin.to_vec(),
        exit,
        aligned,
    }
}

/// The table, in the order the record lists it.
///
/// Deliberately weighted towards the awkward cases rather than the predictable
/// ones: a tab ahead of the caret, a multi-byte character ahead of it, a line long
/// enough to be truncated, bytes that are not UTF-8 at all, and a stream whose
/// second document is the bad one. None of these had to be guessed in advance,
/// which is the whole reason a generated record earns its place here.
///
/// The last entry produces no diagnostic. A stream that holds no documents is not
/// an error, and recording that is how the claim stays checked.
fn cases() -> Vec<Case> {
    let long = format!("[{}x]", "1,".repeat(60)).into_bytes();
    vec![
        case("an unclosed array", ".", b"[1,", 5, true),
        case("a trailing comma", ".", b"[1, 2, 3,]", 5, true),
        case("a key that is not a string", ".", b"{1:2}", 5, true),
        case("a member with no value", ".", b"{\"a\":}", 5, true),
        case("a truncated literal", ".", b"tru", 5, true),
        case("an unterminated string", ".", b"[\"abc", 5, true),
        case(
            "a stream whose second document is bad",
            ".",
            b"1 2 [",
            5,
            true,
        ),
        case(
            "an error that is not on line one",
            ".",
            b"{\n  \"a\": 1,\n  \"b\": ,\n}",
            5,
            false,
        ),
        case("a tab ahead of the caret", ".", b"[\n\t1 2\n]", 5, false),
        case(
            "a multi-byte character ahead of it",
            ".",
            b"[\"\xc3\xa9\", x]",
            5,
            false,
        ),
        case("bytes that are not utf-8", ".", b"[\"\xff\"]", 5, false),
        case("a line long enough to truncate", ".", &long, 5, false),
        case("a filter that does not compile", ".foo[", b"", 3, false),
        case("no documents at all", ".", b"", 0, false),
    ]
}

/// One run of the binary, captured.
struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

/// Run the binary on `stdin` with `filter`, capturing both streams.
///
/// `NO_COLOR` is removed rather than assumed absent. A developer with it exported
/// would otherwise get a different answer from CI, and colour is exactly what one
/// of these tests is about.
///
/// Standard output is a pipe here, so the tool leaves it uncoloured on its own.
/// That is what makes `colour` mean `-C` and nothing else.
fn invoke(filter: &str, stdin: &[u8], colour: bool) -> Run {
    let mut command = Command::new(EXE);
    if colour {
        command.arg("-C");
    }
    let mut child = command
        .arg(filter)
        .env_remove("NO_COLOR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("could not start the jaq-lite binary");
    // The write is setup, not an assertion. The case with a filter that does not
    // compile is still fed nothing, because that child exits before it reads -- but
    // that is no longer what keeps this helper safe. An incomplete write is
    // tolerated, and every fact the tests care about is read from what the child
    // did, so a case added later cannot make this line the reason CI is red.
    let _ = child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(stdin);
    let output = child
        .wait_with_output()
        .expect("could not wait for the child");
    Run {
        stdout: String::from_utf8(output.stdout).expect("stdout was not UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr was not UTF-8"),
        code: output.status.code(),
    }
}

fn record_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("diagnostics.txt")
}

/// One captured stream, labelled.
///
/// An empty stream is spelled out rather than left as a blank line, so that
/// "nothing was written" and "the record is missing a line" cannot look the same
/// in a diff.
fn block(label: &str, text: &str) -> String {
    if text.is_empty() {
        return format!("{label}  : (nothing)\n");
    }
    format!("{label}  :\n{text}")
}

/// Render one case the way the record holds it.
fn render(case: &Case, run: &Run) -> String {
    let exit = run.code.expect("the child was killed by a signal");
    let head = format!(
        "== {} ==\nfilter  : {}\nstdin   : {}\nexit    : {exit}\n",
        case.name,
        case.filter,
        case.stdin.escape_ascii()
    );
    head + &block("stdout", &run.stdout) + &block("stderr", &run.stderr) + "\n"
}

/// Pull the column out of a summary line, which reads
/// `jaq-lite: <stdin>: line 1, column 6: ...`.
///
/// Parsed rather than assumed. The point of the test that uses it is that the
/// error's own `Display` and the caret renderer agree about a position neither of
/// them learned from the other.
fn reported_column(summary: &str) -> usize {
    let tail = summary
        .split_once(", column ")
        .expect("the summary line names a column")
        .1;
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().expect("the column is a number")
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
fn caret_diagnostics_are_recorded() {
    let cases = cases();
    // Paired with the prose in BUILD_LOG.md and STDLIB.md, so the table cannot
    // quietly stop being the thing they describe.
    assert_eq!(cases.len(), 14, "the case table changed size");

    let mut rows = String::from(HEADER);
    for case in &cases {
        let got = invoke(case.filter, &case.stdin, false);
        assert_eq!(
            got.code,
            Some(case.exit),
            "wrong exit code for `{}`; stderr was {}",
            case.name,
            got.stderr
        );
        assert!(
            !got.stderr.as_bytes().contains(&ESC),
            "`{}` coloured a pipe",
            case.name
        );
        rows.push_str(&render(case, &got));
    }

    // The two guards below are on the text about to be written rather than on the
    // file, because tests run in parallel and no test here may depend on another
    // having written the record first. The comparison at the end makes them
    // equivalent to guarding the file.
    assert!(
        !rows.as_bytes().contains(&ESC),
        "the record holds escape bytes"
    );
    // Every case is fed on standard input and reported as `<stdin>`, so a path in
    // the record would mean a diagnostic had begun naming the machine it ran on.
    // This file is quoted in the write-up, and a home directory is not ours to
    // publish.
    for probe in ["/home/", "/root/", "/Users/", "/mnt/"] {
        assert!(!rows.contains(probe), "the record names {probe}");
    }

    let record = record_path();
    if std::env::var_os("UPDATE_DIAGNOSTICS").is_some() {
        fs::write(&record, &rows).expect("cannot write the record");
    }
    let found = fs::read_to_string(&record)
        .expect("tests/diagnostics.txt is missing; regenerate with UPDATE_DIAGNOSTICS=1")
        .replace("\r\n", "\n");
    assert_eq!(
        found, rows,
        "the recorded diagnostics no longer match this tool"
    );
}

#[test]
fn the_caret_agrees_with_the_reported_column() {
    for case in cases() {
        if !case.aligned {
            continue;
        }
        let got = invoke(case.filter, &case.stdin, false);
        let lines: Vec<&str> = got.stderr.lines().collect();
        assert_eq!(lines.len(), 4, "`{}` did not print four lines", case.name);
        let column = reported_column(lines[0]);
        let caret = lines[3];
        assert_eq!(
            caret.matches('^').count(),
            1,
            "`{}` drew more than one caret",
            case.name
        );
        let bar = caret.find('|').expect("the caret line has a gutter");
        let tip = caret.find('^').expect("the caret line has a caret");
        // One space follows the bar before the source line begins, so what is left
        // after removing the bar and that space is the caret's column.
        assert_eq!(
            tip - bar - 1,
            column,
            "the caret missed for `{}`",
            case.name
        );
    }
}

#[test]
fn colour_is_four_runs_and_moves_nothing() {
    let input = b"{\"a\":}";
    let plain = invoke(".", input, false);
    let painted = invoke(".", input, true);
    // One gutter run per snippet line, and one caret run. Eight escape bytes,
    // because every run opens and closes. The summary line above the snippet is
    // not coloured at all, which is the other half of the count: four runs, not
    // five.
    assert_eq!(painted.stderr.matches("\x1b[1;34m").count(), 3);
    assert_eq!(painted.stderr.matches("\x1b[1;31m").count(), 1);
    assert_eq!(painted.stderr.matches("\x1b[0m").count(), 4);
    assert_eq!(painted.stderr.bytes().filter(|b| *b == ESC).count(), 8);
    assert_eq!(strip_sgr(&painted.stderr), plain.stderr);
    assert_eq!(painted.code, plain.code);
}
