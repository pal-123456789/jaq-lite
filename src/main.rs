//! Command-line front end for the `jaq_lite` library.
//!
//! This file stays thin on purpose: argument handling and process exit codes
//! live here, and everything that can be unit tested lives in the library.

#![forbid(unsafe_code)]

use jaq_lite::{Filter, Style};
use std::io::{self, Read, Write};
use std::process::ExitCode;

/// Anything the caller got wrong about the invocation, the input file, or the
/// JSON inside it. This is the code jq uses for all three.
const EXIT_USAGE: u8 = 2;

/// A filter this build cannot compile, which is jq's code for the same thing.
const EXIT_FILTER: u8 = 3;

/// A document that is not JSON, or a filter that could not run on it.
///
/// jq uses one code for both, which is not what this tool did until the
/// behaviour was measured: invalid input and a runtime error both exit 5, and
/// 2 is reserved for getting the invocation wrong.
const EXIT_ERROR: u8 = 5;

/// A closed pipe, which is not a failure at all. See `write_error`.
const EXIT_FINE: u8 = 0;

/// The name this tool answers to, in front of everything it says on standard
/// error.
const BINARY: &str = "jaq-lite";

/// Report a problem on standard error, prefixed the way every other line is.
///
/// One stream can produce several problems, so reporting cannot wait for the
/// process to exit carrying a single message.
fn report(message: impl std::fmt::Display) {
    eprintln!("{BINARY}: {message}");
}

/// Print a diagnostic snippet, unprefixed.
///
/// The summary line above it already names the binary and the input; repeating
/// that on the caret lines would break the alignment the snippet exists for.
fn show(block: &str) {
    eprintln!("{block}");
}

const USAGE: &str = "\
jaq-lite -- a JSON processor with no dependencies

Usage:
  jaq-lite [options] <filter> [file...]

The filter is applied to each input document and the result is written to
standard output. With no file arguments, or with `-`, input is read from
standard input.

Options:
  -c, --compact-output   Print with no newlines or indentation.
  -r, --raw-output       Print a top-level string as its contents, not as JSON.
  -h, --help             Print this help and exit.
  -V, --version          Print the version and exit.
  --                     Stop reading options; later arguments are positional.

Exit codes:
  0   the filter ran
  2   a problem with the invocation, or with opening a file
  3   a filter that does not compile
  5   input that is not JSON, or a filter that could not run on it
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
}

/// The command line after parsing.
struct Options {
    filter: String,
    files: Vec<String>,
    style: Style,
    raw: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            if failure.code == EXIT_FINE {
                return ExitCode::SUCCESS;
            }
            // A failure that already reported itself carries no message. That is
            // how a stream with several bad documents names each one as it is
            // reached and still exits once, with one code.
            if !failure.message.is_empty() {
                report(&failure.message);
            }
            if failure.code == EXIT_USAGE {
                report("run `jaq-lite --help` for usage");
            }
            ExitCode::from(failure.code)
        }
    }
}

fn run() -> Result<(), Failure> {
    let Some(options) = parse_args(std::env::args().skip(1).collect())? else {
        return Ok(());
    };

    // Reported here rather than through `?`: the caret needs the filter text as
    // well as the error, and this is the scope that holds both.
    let filter = match Filter::compile(&options.filter) {
        Ok(filter) => filter,
        Err(error) => {
            report(&error);
            show(&jaq_lite::diag::snippet_at(
                options.filter.as_bytes(),
                error.offset(),
                error.line(),
                error.column(),
            ));
            return Err(Failure {
                code: EXIT_FILTER,
                message: String::new(),
            });
        }
    };
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    if options.files.is_empty() {
        let bytes = read_stdin()?;
        emit(
            &mut out,
            &filter,
            &bytes,
            "<stdin>",
            options.style,
            options.raw,
        )?;
    } else {
        for path in &options.files {
            if path == "-" {
                let bytes = read_stdin()?;
                emit(
                    &mut out,
                    &filter,
                    &bytes,
                    "<stdin>",
                    options.style,
                    options.raw,
                )?;
            } else {
                let bytes = std::fs::read(path)
                    .map_err(|error| Failure::usage(format!("{path}: {error}")))?;
                emit(&mut out, &filter, &bytes, path, options.style, options.raw)?;
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
    let mut raw = false;
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
            } else if arg == "-r" || arg == "--raw-output" {
                raw = true;
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
        raw,
    }))
}

fn read_stdin() -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::usage(format!("standard input: {error}")))?;
    Ok(bytes)
}

/// Run the filter over every document in one input, writing as it goes.
///
/// Bytes rather than text, so that a file which is not valid UTF-8 is reported
/// with a position instead of failing before parsing starts.
///
/// A filter produces a stream, not a value: none for a path that misses under
/// `?`, several for `.[]`. So this writes a loop, inside the loop over
/// documents.
///
/// Output is written per document rather than collected, so everything produced
/// before a syntax error still reaches standard output. A document the filter
/// cannot handle is named and the stream continues: jq reports the status of
/// the last document only, which hides an earlier failure from a script running
/// under `set -e`.
fn emit<W: Write>(
    out: &mut W,
    filter: &Filter,
    bytes: &[u8],
    origin: &str,
    style: Style,
    raw: bool,
) -> Result<(), Failure> {
    let mut failed = false;
    for document in jaq_lite::parse_stream(bytes) {
        let value = match document {
            Ok(value) => value,
            Err(error) => {
                // Flush first: the documents already written belong on standard
                // output before this line appears on standard error.
                out.flush().map_err(|io_error| write_error(&io_error))?;
                report(format!("{origin}: {error}"));
                show(&jaq_lite::diag::snippet(bytes, &error));
                return Err(Failure {
                    code: EXIT_ERROR,
                    message: String::new(),
                });
            }
        };
        match filter.run(&value) {
            Ok(outputs) => {
                for output in &outputs {
                    // `-r` prints a top-level string as its contents. A string
                    // inside an array or an object stays quoted, because the
                    // value being printed is the container. That is jq's rule.
                    match output {
                        jaq_lite::Value::String(text) if raw => {
                            out.write_all(text.as_bytes())
                                .map_err(|error| write_error(&error))?;
                        }
                        _ => {
                            jaq_lite::write(out, output, style)
                                .map_err(|error| write_error(&error))?;
                        }
                    }
                    out.write_all(b"\n").map_err(|error| write_error(&error))?;
                }
            }
            Err(error) => {
                out.flush().map_err(|io_error| write_error(&io_error))?;
                report(format!("{origin}: {error}"));
                failed = true;
            }
        }
    }
    if failed {
        // Every failure has already been reported, so there is no message left
        // to carry -- only the code.
        out.flush().map_err(|io_error| write_error(&io_error))?;
        return Err(Failure {
            code: EXIT_ERROR,
            message: String::new(),
        });
    }
    Ok(())
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
