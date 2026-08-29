//! The colour flags, end to end through the binary.
//!
//! Every case here runs with standard output on a pipe, which is the only thing
//! a test can arrange without a pseudo-terminal. That is deliberate rather than
//! a compromise: the pipe cases are the ones where a regression would be silent,
//! because escape bytes in a redirected file corrupt it without ever being seen.
//!
//! The terminal cases -- colour by default, and NO_COLOR overriding that default
//! -- cannot be reached from here. The standard library does not open a
//! pseudo-terminal, and shelling out to `script` would make this suite depend on
//! a tool that does not exist on Windows. They were measured against jq on a pty
//! and are covered by inspection of `choose_paint`, which is four branches long.
//!
//! `Run` and `run` are duplicated from `cli.rs` rather than shared: each file in
//! `tests/` is its own crate, so the alternative is a `common` module, and one
//! twenty-line helper is not worth the indirection.

use std::io::Write;
use std::process::{Command, Stdio};

const EXE: &str = env!("CARGO_BIN_EXE_jaq-lite");

/// The escape byte that starts every SGR sequence.
const ESC: u8 = 0x1b;

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

/// Run the binary with `args`, feeding `input` on standard input.
///
/// `NO_COLOR` is removed from the child's environment rather than assumed absent.
/// A developer with it exported would otherwise get different results from CI,
/// and a test that only passes on some machines is not a test.
fn run(args: &[&str], input: &str) -> Run {
    spawn(args, input, None)
}

/// The same, with `NO_COLOR` set to `value`.
fn run_with_no_color(args: &[&str], input: &str, value: &str) -> Run {
    spawn(args, input, Some(value))
}

fn spawn(args: &[&str], input: &str, no_color: Option<&str>) -> Run {
    let mut command = Command::new(EXE);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(value) = no_color {
        command.env("NO_COLOR", value);
    } else {
        command.env_remove("NO_COLOR");
    }
    let mut child = command
        .spawn()
        .expect("could not start the jaq-lite binary");
    // An empty input never touches the pipe, so `--help` -- which exits before
    // reading -- cannot fail here with a broken pipe.
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(input.as_bytes())
        .expect("could not write to the child");
    let output = child
        .wait_with_output()
        .expect("could not wait for the child");
    Run {
        stdout: String::from_utf8(output.stdout).expect("stdout was not UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr was not UTF-8"),
        code: output.status.code(),
    }
}

/// The document most cases use. Two members, one string and one number, so a key
/// and a value of different colours both appear.
const DOC: &str = r#"{"k":"s","n":1}"#;

#[test]
fn a_pipe_gets_no_escapes_by_default() {
    let result = run(&["-c", "."], DOC);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "{\"k\":\"s\",\"n\":1}\n");
    assert!(
        !result.stdout.as_bytes().contains(&ESC),
        "a redirected stream must not carry escapes, got {:?}",
        result.stdout
    );
}

#[test]
fn colour_output_is_available_under_both_spellings() {
    for flag in ["-C", "--color-output"] {
        let result = run(&["-c", flag, "."], DOC);
        assert_eq!(
            result.code,
            Some(0),
            "{flag}: stderr was: {}",
            result.stderr
        );
        assert!(
            result.stdout.as_bytes().contains(&ESC),
            "{flag} produced no escapes"
        );
    }
}

#[test]
fn monochrome_output_is_available_under_both_spellings() {
    for flag in ["-M", "--monochrome-output"] {
        let result = run(&["-c", flag, "."], DOC);
        assert_eq!(
            result.code,
            Some(0),
            "{flag}: stderr was: {}",
            result.stderr
        );
        assert_eq!(result.stdout, "{\"k\":\"s\",\"n\":1}\n");
    }
}

/// Measured in both orders, because this is where last-one-wins would be wrong.
#[test]
fn monochrome_beats_colour_in_either_order() {
    for args in [["-c", "-C", "-M", "."], ["-c", "-M", "-C", "."]] {
        let result = run(&args, DOC);
        assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
        assert!(
            !result.stdout.as_bytes().contains(&ESC),
            "-M did not win in {args:?}"
        );
    }
}

/// The surprising one: an explicit flag outranks the environment.
#[test]
fn an_explicit_colour_flag_beats_no_color() {
    let result = run_with_no_color(&["-c", "-C", "."], DOC, "1");
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert!(
        result.stdout.as_bytes().contains(&ESC),
        "NO_COLOR must not override -C, got {:?}",
        result.stdout
    );
}

#[test]
fn no_color_never_turns_colour_on() {
    // Weak, and better said than dressed up: on a pipe there is no colour to
    // suppress, so this only shows NO_COLOR cannot somehow enable it. The case
    // where NO_COLOR does the real work needs a terminal, and is measured there.
    for value in ["1", "", "anything"] {
        let result = run_with_no_color(&["-c", "."], DOC, value);
        assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
        assert!(!result.stdout.as_bytes().contains(&ESC), "NO_COLOR={value}");
    }
}

#[test]
fn raw_output_is_never_coloured() {
    let result = run(&["-C", "-r", "."], r#""hello""#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert_eq!(result.stdout, "hello\n");
}

#[test]
fn raw_output_still_colours_a_container() {
    // -r is not a colour flag. It replaces the encoding of a top-level string,
    // and a string inside an array is still JSON, so it is still quoted and green.
    let result = run(&["-c", "-C", "-r", "."], r#"["s"]"#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    assert!(
        result.stdout.contains("\x1b[0;32m\"s\"\x1b[0m"),
        "expected a green quoted string inside the array, got {:?}",
        result.stdout
    );
}

/// The whole table in one assertion, against bytes read out of `jq -C -c`.
///
/// One run per token: colour, text, reset. The object owns its braces, its comma
/// and its colon and paints all three `1;39`; a key is `1;34`, a string value
/// `0;32`, a number `0;39`. The reset is a full `0m`, never `39m`.
#[test]
fn the_bytes_are_the_ones_jq_writes() {
    let result = run(&["-c", "-C", "."], DOC);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    let expected = concat!(
        "\x1b[1;39m{\x1b[0m",
        "\x1b[1;34m\"k\"\x1b[0m",
        "\x1b[1;39m:\x1b[0m",
        "\x1b[0;32m\"s\"\x1b[0m",
        "\x1b[1;39m,\x1b[0m",
        "\x1b[1;34m\"n\"\x1b[0m",
        "\x1b[1;39m:\x1b[0m",
        "\x1b[0;39m1\x1b[0m",
        "\x1b[1;39m}\x1b[0m",
        "\n"
    );
    assert_eq!(result.stdout, expected);
}

/// Pretty output puts the indentation, the newlines and the space after a colon
/// outside every run, and the colon itself inside one. That asymmetry is measured,
/// and it is the detail a reimplementation gets wrong first.
#[test]
fn indentation_and_the_colon_space_stay_outside_the_runs() {
    let result = run(&["-C", "."], r#"{"k":1}"#);
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    let expected = concat!(
        "\x1b[1;39m{\x1b[0m\n",
        "  \x1b[1;34m\"k\"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m\n",
        "\x1b[1;39m}\x1b[0m\n"
    );
    assert_eq!(result.stdout, expected);
}

#[test]
fn the_help_text_documents_both_flags() {
    let result = run(&["--help"], "");
    assert_eq!(result.code, Some(0), "stderr was: {}", result.stderr);
    for needle in ["-C, --color-output", "-M, --monochrome-output", "NO_COLOR"] {
        assert!(
            result.stdout.contains(needle),
            "--help does not mention {needle}"
        );
    }
}
