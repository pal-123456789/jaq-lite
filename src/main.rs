//! Command-line front end for the `jaq_lite` library.
//!
//! This file stays thin on purpose: argument handling and process exit codes
//! live here, and everything that can be unit tested lives in the library.

#![forbid(unsafe_code)]

use jaq_lite::{Style, parse};
use std::io::{self, Read, Write};
use std::process::ExitCode;

/// Anything the caller got wrong about the invocation, the input file, or the
/// JSON inside it. This is the code jq uses for all three.
const EXIT_USAGE: u8 = 2;

/// A filter this build cannot compile, which is jq's code for the same thing.
const EXIT_FILTER: u8 = 3;

/// A closed pipe, which is not a failure at all. See `write_error`.
const EXIT_FINE: u8 = 0;

const USAGE: &str = "\
jaq-lite -- a JSON processor with no dependencies

Usage:
  jaq-lite [options] <filter> [file...]

The filter is applied to each input document and the result is written to
standard output. With no file arguments, or with `-`, input is read from
standard input.

Options:
  -c, --compact-output   Print with no newlines or indentation.
  -h, --help             Print this help and exit.
  -V, --version          Print the version and exit.
  --                     Stop reading options; later arguments are positional.

Exit codes:
  0   the filter ran
  2   a problem with the invocation, an input file, or the JSON in it
  3   a filter this build does not support
";

/// What went wrong, and what to exit with.
struct Failure {
    code: u8,
    message: String,
}

impl Failure {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_USAGE,
            message: message.into(),
        }
    }

    fn filter(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_FILTER,
            message: message.into(),
        }
    }
}

/// The command line after parsing.
struct Options {
    filter: String,
    files: Vec<String>,
    style: Style,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            if failure.code == EXIT_FINE {
                return ExitCode::SUCCESS;
            }
            eprintln!("jaq-lite: {}", failure.message);
            if failure.code == EXIT_USAGE {
                eprintln!("jaq-lite: run `jaq-lite --help` for usage");
            }
            ExitCode::from(failure.code)
        }
    }
}

fn run() -> Result<(), Failure> {
    let Some(options) = parse_args(std::env::args().skip(1).collect())? else {
        return Ok(());
    };

    // The query language arrives in a later commit. Refusing an unsupported
    // filter with its own exit code, rather than ignoring it and printing the
    // input unchanged, is the difference between a limitation and a lie.
    if options.filter.trim() != "." {
        return Err(Failure::filter(format!(
            "unsupported filter `{}`; this build understands `.` only",
            options.filter
        )));
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    if options.files.is_empty() {
        let bytes = read_stdin()?;
        emit(&mut out, &bytes, "<stdin>", options.style)?;
    } else {
        for path in &options.files {
            if path == "-" {
                let bytes = read_stdin()?;
                emit(&mut out, &bytes, "<stdin>", options.style)?;
            } else {
                let bytes = std::fs::read(path)
                    .map_err(|error| Failure::usage(format!("{path}: {error}")))?;
                emit(&mut out, &bytes, path, options.style)?;
            }
        }
    }

    out.flush().map_err(|error| write_error(&error))
}

/// Parse the arguments. `Ok(None)` means the work is already done, which is what
/// `--help` and `--version` do.
fn parse_args(args: Vec<String>) -> Result<Option<Options>, Failure> {
    let mut filter: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut style = Style::Pretty;
    let mut options_ended = false;

    for arg in args {
        // A bare `-` names standard input, so it is a positional argument
        // rather than an option despite the leading dash.
        let is_option = !options_ended && arg.starts_with('-') && arg != "-";
        if is_option {
            if arg == "--" {
                options_ended = true;
            } else if arg == "-h" || arg == "--help" {
                print!("{USAGE}");
                return Ok(None);
            } else if arg == "-V" || arg == "--version" {
                println!("jaq-lite {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            } else if arg == "-c" || arg == "--compact-output" {
                style = Style::Compact;
            } else {
                return Err(Failure::usage(format!("unknown option `{arg}`")));
            }
        } else if filter.is_none() {
            filter = Some(arg);
        } else {
            files.push(arg);
        }
    }

    let filter = filter.ok_or_else(|| Failure::usage("no filter given"))?;
    Ok(Some(Options {
        filter,
        files,
        style,
    }))
}

fn read_stdin() -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::usage(format!("standard input: {error}")))?;
    Ok(bytes)
}

/// Parse one document and write the result, followed by a newline.
///
/// Bytes rather than text, so that a file which is not valid UTF-8 is reported
/// with a position instead of failing before parsing starts.
fn emit<W: Write>(out: &mut W, bytes: &[u8], origin: &str, style: Style) -> Result<(), Failure> {
    let value = parse(bytes).map_err(|error| Failure::usage(format!("{origin}: {error}")))?;
    jaq_lite::write(out, &value, style).map_err(|error| write_error(&error))?;
    out.write_all(b"\n").map_err(|error| write_error(&error))
}

/// Classify an output error.
///
/// A broken pipe is how `jaq-lite . big.json | head` is supposed to end. Exiting
/// zero and saying nothing is what every well-behaved filter does; printing an
/// error there would make the tool noisy in exactly the pipeline it belongs in.
fn write_error(error: &io::Error) -> Failure {
    if error.kind() == io::ErrorKind::BrokenPipe {
        Failure {
            code: EXIT_FINE,
            message: String::new(),
        }
    } else {
        Failure::usage(format!("standard output: {error}"))
    }
}
