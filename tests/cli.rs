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
}

fn run(args: &[&str], input: &str) -> Run {
    let mut child = Command::new(EXE)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("could not start the jaq-lite binary");
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(input.as_bytes())
        .expect("could not write to the child");
    // `wait_with_output` closes stdin first, which is what lets the child see
    // end of input rather than blocking forever.
    let output = child
        .wait_with_output()
        .expect("could not wait for the child");
    Run {
        stdout: String::from_utf8(output.stdout).expect("stdout was not UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr was not UTF-8"),
        code: output.status.code(),
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
    assert_eq!(result.code, Some(2));
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
fn no_filter_at_all_is_a_usage_error() {
    let result = run(&[], "null");
    assert_eq!(result.code, Some(2));
}

#[test]
fn an_unsupported_filter_has_its_own_exit_code() {
    let result = run(&[".foo"], "{}");
    assert_eq!(
        result.code,
        Some(3),
        "an unsupported filter is not a usage error, stderr was: {}",
        result.stderr
    );
    assert!(
        result.stderr.contains("unsupported filter"),
        "got: {}",
        result.stderr
    );
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
