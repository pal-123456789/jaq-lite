//! End-to-end tests for the binary.
//!
//! Cargo sets `CARGO_BIN_EXE_<target>` for integration tests, which is the only
//! reliable way to find the executable: guessing at `target/debug` breaks under
//! a custom target directory, a different profile, or cross-compilation.

use std::io::Write;
use std::process::{Command, Stdio};

const EXE: &str = env!("CARGO_BIN_EXE_jaq-lite");

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
    /// Whether the whole of the input reached the child.
    ///
    /// False is not a failure. A child that rejects its arguments exits before it
    /// reads, so the read end of the pipe can be closed before the write happens.
    /// One test asserts this is false, which is what keeps that path exercised on
    /// purpose rather than by luck.
    wrote_all: bool,
}

fn run(args: &[&str], input: &str) -> Run {
    let mut child = Command::new(EXE)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("could not start the jaq-lite binary");
    // The write is setup, not an assertion, so a write that fails is not this
    // test's failure. A child that rejects its arguments exits before it reads a
    // byte, so the read end of this pipe may already be closed -- and whether it is
    // depends on scheduling. CI failed `an_unknown_option_is_a_usage_error` with a
    // broken pipe on code whose local gate had run the same test and passed.
    //
    // Nothing is hidden by tolerating it. Every fact each test cares about is read
    // below from what the child actually did, so an incomplete write can only turn
    // a real defect into a clearer failure than `Broken pipe` -- never into a pass.
    let wrote = child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(input.as_bytes());
    // `wait_with_output` closes stdin first, which is what lets the child see
    // end of input rather than blocking forever.
    let output = child
        .wait_with_output()
        .expect("could not wait for the child");
    Run {
        stdout: String::from_utf8(output.stdout).expect("stdout was not UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr was not UTF-8"),
        code: output.status.code(),
        wrote_all: wrote.is_ok(),
    }
}

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("jaq-lite-{}-{name}", std::process::id()));
    std::fs::write(&path, contents).expect("could not write the temporary file");
    path
}

#[test]
fn the_identity_filter_pretty_prints() {
    let result = run(&["."], r#"{"b":[1,{}],"a":null}"#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(
        result.stdout,
        "{\n  \"b\": [\n    1,\n    {}\n  ],\n  \"a\": null\n}\n"
    );
}

#[test]
fn compact_output_is_available_under_both_spellings() {
    for flag in ["-c", "--compact-output"] {
        let result = run(&[flag, "."], r#"{ "a" : [ 1 , 2 ] }"#);
        assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
        assert_eq!(result.stdout, "{\"a\":[1,2]}\n", "flag was {flag}");
    }
}

#[test]
fn there_is_exactly_one_trailing_newline() {
    let result = run(&["."], "null");
    assert_eq!(result.stdout, "null\n");
}

#[test]
fn help_goes_to_stdout_and_exits_zero() {
    let result = run(&["--help"], "");
    assert_eq!(result.code, Some(0));
    assert!(result.stdout.contains("Usage:"), "got: {}", result.stdout);
    assert!(result.stderr.is_empty(), "help should not warn");
}

#[test]
fn version_reports_the_crate_version() {
    let result = run(&["--version"], "");
    assert_eq!(result.code, Some(0));
    assert_eq!(
        result.stdout.trim(),
        format!("jaq-lite {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn malformed_input_is_reported_with_a_position() {
    let result = run(&["."], "{\"a\": }");
    assert_eq!(
        result.code,
        Some(5),
        "jq exits 5 for input that is not JSON, stderr was: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("line 1, column"),
        "the error should locate itself, got: {}",
        result.stderr
    );
    assert!(result.stdout.is_empty(), "nothing should have been printed");
}

#[test]
fn an_unknown_option_is_a_usage_error() {
    let result = run(&["--nope", "."], "null");
    assert_eq!(result.code, Some(2));
    assert!(
        result.stderr.contains("unknown option"),
        "got: {}",
        result.stderr
    );
}

#[test]
fn a_rejected_invocation_is_unaffected_by_how_much_input_it_was_sent() {
    // A megabyte, which no pipe buffers, so this child's read end is certainly
    // closed before the last byte is written. That is the point of the size: the
    // tolerance in `run` is otherwise reached only when the race above is lost,
    // which is seldom, which is how a green suite hid it until CI lost it.
    //
    // This cannot hang. `--nope` is rejected in `parse_args` before stdin is
    // touched, and the two lines the child writes to stderr are far short of that
    // pipe's buffer, so the child always reaches its exit and always closes the
    // read end -- which is what ends the write, with an error rather than a wait.
    let big = "null ".repeat(200_000);
    let result = run(&["--nope", "."], &big);
    assert!(
        !result.wrote_all,
        "a megabyte reached a child that never reads"
    );
    // The same three facts the four-byte case asserts. How much input the caller
    // sent is not something a rejected invocation is allowed to depend on.
    assert_eq!(result.code, Some(2));
    assert!(
        result.stderr.contains("unknown option"),
        "got: {}",
        result.stderr
    );
    assert!(result.stdout.is_empty(), "got: {}", result.stdout);
}

#[test]
fn no_filter_at_all_is_a_usage_error() {
    let result = run(&[], "null");
    assert_eq!(result.code, Some(2));
}

#[test]
fn a_filter_that_does_not_compile_has_its_own_exit_code() {
    // `lenght` and not `length`. This test was written with `length` itself, back
    // when no bare name compiled at all; the misspelling is the better case
    // anyway, since a typo in one of the four is how anybody arrives here.
    let result = run(&["lenght"], "{}");
    assert_eq!(
        result.code,
        Some(3),
        "a bad filter is not a usage error, stderr was: {}",
        result.stderr
    );
    assert!(result.stderr.contains("column"), "got: {}", result.stderr);
}

/// The filter gets the same treatment a document gets: a summary line, then the
/// source with a caret under the character that was wrong.
#[test]
fn a_filter_that_does_not_compile_draws_a_caret_under_it() {
    let result = run(&[".a %"], "null");
    assert_eq!(result.code, Some(3), "stderr was: {}", result.stderr);
    let lines: Vec<&str> = result.stderr.lines().collect();
    assert_eq!(
        lines.len(),
        4,
        "a summary line and a three-line snippet: {lines:?}"
    );
    assert_eq!(
        lines[0],
        "jaq-lite: filter, column 4: `%` has no meaning here"
    );
    assert_eq!(lines[2], "1 | .a %");
    assert_eq!(lines[3], "  |    ^");
}

#[test]
fn a_filter_that_cannot_run_has_its_own_exit_code() {
    let result = run(&[".a"], "[1]");
    assert_eq!(
        result.code,
        Some(5),
        "a runtime error is neither a usage nor a compile error, stderr was: {}",
        result.stderr
    );
    assert!(
        result
            .stderr
            .contains("Cannot index array with string \"a\""),
        "got: {}",
        result.stderr
    );
}

#[test]
fn a_path_reaches_into_the_document() {
    let result = run(&["-c", ".users[0].name"], r#"{"users":[{"name":"ada"}]}"#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "\"ada\"\n");
}

#[test]
fn a_filter_with_several_outputs_prints_one_per_line() {
    let result = run(&["-c", ".[] | .id"], r#"[{"id":1},{"id":2}]"#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "1\n2\n");
}
#[test]
fn a_file_is_read_instead_of_standard_input() {
    let path = temp_file("one.json", "[1,2]");
    let result = run(&["-c", ".", path.to_str().expect("utf-8 path")], "");
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "[1,2]\n");
    std::fs::remove_file(&path).ok();
}

#[test]
fn several_files_are_printed_in_the_order_given() {
    let first = temp_file("first.json", "1");
    let second = temp_file("second.json", "2");
    let result = run(
        &[
            "-c",
            ".",
            first.to_str().expect("utf-8 path"),
            second.to_str().expect("utf-8 path"),
        ],
        "",
    );
    assert_eq!(result.stdout, "1\n2\n");
    std::fs::remove_file(&first).ok();
    std::fs::remove_file(&second).ok();
}

#[test]
fn a_dash_means_standard_input() {
    let result = run(&["-c", ".", "-"], "[3]");
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "[3]\n");
}

#[test]
fn a_missing_file_is_named_in_the_error() {
    let result = run(&[".", "no-such-file-4d2.json"], "");
    assert_eq!(result.code, Some(2));
    assert!(
        result.stderr.contains("no-such-file-4d2.json"),
        "the error should name the file, got: {}",
        result.stderr
    );
}

#[test]
fn a_stream_of_documents_is_read_the_way_jq_reads_one() {
    // Newline-delimited JSON is the common case, but jq does not require the
    // newline: whitespace between documents is enough, and none at all is too.
    assert_eq!(run(&["-c", "."], "1 2").stdout, "1\n2\n");
    assert_eq!(run(&["-c", "."], "1\n2\n").stdout, "1\n2\n");
    assert_eq!(run(&["-c", ".a"], r#"{"a":1}{"a":2}"#).stdout, "1\n2\n");
    assert_eq!(run(&["-c", ".[]"], "[1,2] [3]").stdout, "1\n2\n3\n");
}

#[test]
fn an_empty_stream_says_nothing_and_exits_zero() {
    // One empty input is an error, because there is no document in it. A stream
    // of no documents is simply empty, which is what an empty file is.
    let got = run(&["-c", "."], "   \n\t\n");
    assert_eq!(got.stdout, "");
    assert_eq!(got.stderr, "");
    assert_eq!(got.code, Some(0));
}

#[test]
fn output_before_a_syntax_error_is_still_written() {
    // Writing as documents are parsed, rather than collecting first, is what
    // makes this true. It is also what jq does.
    let got = run(&["-c", "."], "1 2 [");
    assert_eq!(got.stdout, "1\n2\n");
    assert_eq!(got.code, Some(5));
}

#[test]
fn a_document_the_filter_cannot_handle_does_not_stop_the_stream() {
    let got = run(&["-c", ".a"], r#"1 {"a":2}"#);
    assert_eq!(got.stdout, "2\n");
    assert_eq!(got.code, Some(5));
    assert!(
        got.stderr
            .contains(r#"Cannot index number with string "a""#),
        "{}",
        got.stderr
    );
}

#[test]
fn every_failing_document_is_named_not_only_the_first() {
    // jq reports the status of the last document only, so this exits 0 there
    // and the failures disappear from a script running under `set -e`.
    let got = run(&["-c", ".a"], "1 2 3");
    let lines: Vec<&str> = got.stderr.lines().collect();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(got.code, Some(5));
}

#[test]
fn raw_output_prints_a_string_without_its_quotes() {
    let result = run(&["-r", ".s"], "{\"s\":\"hi there\"}");
    assert_eq!(result.stdout, "hi there\n");
    assert_eq!(result.code, Some(0));
}

#[test]
fn raw_output_is_available_under_both_spellings() {
    let short = run(&["-r", ".s"], "{\"s\":\"x\"}");
    let long = run(&["--raw-output", ".s"], "{\"s\":\"x\"}");
    assert_eq!(short.stdout, "x\n");
    assert_eq!(long.stdout, short.stdout);
}

#[test]
fn raw_output_leaves_a_nested_string_quoted() {
    // The value being printed is the array, so -r does not reach inside it.
    let result = run(&["-c", "-r", ".n"], "{\"n\":[\"a\",\"b\"]}");
    assert_eq!(result.stdout, "[\"a\",\"b\"]\n");
}

#[test]
fn raw_output_changes_nothing_that_is_not_a_string() {
    let result = run(&["-c", "-r", "."], "{\"a\":1}");
    assert_eq!(result.stdout, "{\"a\":1}\n");
}

#[test]
fn raw_output_writes_the_value_not_the_source_text() {
    // A tab written as an escape in the input is a real tab in the value, and
    // raw output writes the value. Without -r it is escaped again on the way out.
    let escaped = run(&["-r", ".s"], "{\"s\":\"a\\tb\"}");
    assert_eq!(escaped.stdout, "a\tb\n");
    let quoted = run(&["-c", ".s"], "{\"s\":\"a\\tb\"}");
    assert_eq!(quoted.stdout, "\"a\\tb\"\n");
}

/// The filter the test above used to be written with, now that it compiles.
///
/// `tests/query.rs` is where the four builtins' answers are pinned; what is worth
/// asserting out here is only that one reaches the command line at all -- on
/// stdout, as JSON, with the exit code of a filter that ran and nothing on stderr.
#[test]
fn a_builtin_answers_on_the_command_line() {
    let result = run(&["length"], r#"{"a":1,"b":2}"#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "2\n");
    assert!(result.stderr.is_empty(), "got: {}", result.stderr);
    let keys = run(&["-c", "keys"], r#"{"b":1,"a":2}"#);
    assert_eq!(keys.code, Some(0), "stderr was: {}", keys.stderr);
    assert_eq!(keys.stdout, "[\"a\",\"b\"]\n");
}
