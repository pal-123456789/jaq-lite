# jaq-lite

[![ci](https://github.com/pal-123456789/jaq-lite/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pal-123456789/jaq-lite/actions/workflows/ci.yml)

A JSON parser, serializer and jq-style query CLI written against the Rust
standard library and nothing else. No `serde`, no `serde_json`, no `clap`, no
crates at all: `Cargo.toml` declares an empty `[dependencies]` table and
`Cargo.lock` is committed so that you can check the claim instead of believing
it.

Built during the Zero Dependency hackathon window, 2026-08-28 18:00 UTC to
2026-08-31 18:00 UTC. Sections below are filled in as each claim becomes
measurable, and every number in this file is produced by a command recorded
beside it in [CLAIMS.md](CLAIMS.md). Nothing here is asserted without a way to
check it.

## Proof of an empty dependency graph

`cargo tree` prints one crate, because there is only one:

    jaq-lite v0.1.0 (D:\zero-dep\jaq-lite)

`Cargo.lock` holds exactly 1 `[[package]]` entry, this crate, which is what
an empty dependency graph looks like in a lock file. The whole capture is
committed as `deps-proof.txt`: toolchain versions, both manifests, the tree, and
a release build run as

    cargo build --release --offline --locked

where `--offline` rules out a fetch and `--locked` rules out the lock file being
rewritten to make the build succeed. Those are the two ways an accidental
dependency could hide.

## Install and run

    git clone https://github.com/pal-123456789/jaq-lite
    cd jaq-lite
    cargo build --release

One command, no flags, no feature selection, no network access after the clone.

**Tested on** two hosts, both running the compiler `Cargo.toml` pins:

| host | target | rustc |
|---|---|---|
| Windows 11 Home Single Language build 26200 | `x86_64-pc-windows-msvc` | 1.98.0, committed 2026-08-18 |
| Ubuntu26.04LTS under WSL2 | `x86_64-unknown-linux-gnu` | 1.98.0, committed 2026-08-18 |

That table is a claim about two machines one person owns, which is the weaker
half of the evidence. The stronger half is the badge at the top: CI runs the same
gate on `windows-latest` and `ubuntu-latest`, on hardware nobody here has touched.

The split is not symmetric and it is worth saying which way. Tests, `clippy` and
`rustfmt` run on both. The reproducible-build proof and the jq differential are
Linux only and say so where each is described: the first because the claim is
about ELF bytes and deliberately excludes MSVC, the second because comparing
against two different jq builds is not twice the information. `tests/claims.rs`
reads the version out of `Cargo.toml` and fails if either row here drifts from it.
## Usage

Reads standard input, or a file named as the last argument; a bare `-` names
standard input among file arguments. `-c` compacts the output, `-r` prints a
top-level string as its contents, `-h` prints usage, `-V` prints the version,
and `--` stops option parsing.

    $ jaq-lite . users.json
    {
      "users": [
        {
          "name": "ada",
          "admin": true
        },
        {
          "name": "linus",
          "admin": false
        }
      ],
      "count": 2
    }

    $ jaq-lite -c '.users[] | .name' users.json
    "ada"
    "linus"

    $ jaq-lite -r '.users[] | .name' users.json
    ada
    linus

    $ jaq-lite -c '.count, .users[0].admin' users.json
    2
    true

    $ printf '1 2 3' | jaq-lite -c .
    1
    2
    3

`-r` applies to the value being printed, not to strings inside it: the second and
third commands above run the same filter and differ only in the quoting of the
result, while a string nested in an array stays quoted because the array is what
is printed.

Exit codes follow jq: 2 for a bad flag or a file that will not open, 3 for a
filter that does not compile, 5 for input that is not JSON or a filter that
fails on a document, and 0 otherwise.

## Diagnostics

A parse failure prints the line, the column and the reason, then the offending
source line with a caret under the character that was wrong -- the shape `rustc`
uses. The first line is the whole message when it is read by a program; the
snippet under it is for the person who has to fix the file.

    $ printf '{1:2}' | jaq-lite .
    jaq-lite: <stdin>: line 1, column 2: expected a string as the object key
      |
    1 | {1:2}
      |  ^
    [exit 5]

    $ jaq-lite . broken.json
    jaq-lite: broken.json: line 3, column 8: unexpected `t`
      |
    3 |   "b": tru
      |        ^
    [exit 5]

The renderer works on the raw input bytes rather than on a `str`, because input
that is not valid UTF-8 is itself one of the failures it has to draw. An invalid
byte becomes one replacement character, so the caret still lands exactly where
the column number says it does.

    $ jaq-lite . broken.json
    jaq-lite: broken.json: line 1, column 4: invalid UTF-8 after 3 valid bytes
      |
    1 | ["a�"]
      |    ^
    [exit 5]

A column is one non-continuation byte together with the continuation bytes after
it, which is the same unit the column number counts in, so a multi-byte
character earlier in the line cannot shift the caret. A tab is expanded to four
spaces before the caret is placed, or the caret would sit three places short of
its character. A long line is cut around the caret with an ellipsis on either
side: minified JSON is one line and can be megabytes, and a failure late in a
large document must not print the document to standard error.

A filter that does not compile is drawn by the same renderer, because a filter is
also source text with a position in it:

    $ printf 'null' | jaq-lite .a%
    jaq-lite: filter, column 3: `%` has no meaning here
      |
    1 | .a%
      |   ^
    [exit 3]

One renderer, one column rule, and two callers that each have a byte offset and
the bytes it points into. The filter parser counts its columns with the function
the JSON parser uses, so the number in the message and the position of the caret
cannot drift apart in either of them.

## jq compatibility

Output is byte-compatible with `jq` wherever a choice exists. Every claim in this
section was produced by running `jq-1.8.1` beside this binary, not by reading its
manual, which documents almost none of it. Two divergences are deliberate.

**Numbers are re-emitted from the bytes that were read.** jq re-renders them
through its own decimal formatter, so it prints a canonical form rather than the
one you wrote:

| input | jaq-lite | jq |
|---|---|---|
| `1e2` | `1e2` | `1E+2` |
| `0.1e-5` | `0.1e-5` | `0.000001` |
| `1e1000` | `1e1000` | `1E+1000` |

Matching jq would mean implementing decimal canonicalisation in order to return
a less faithful answer, so the original text is kept instead.

**Strings are not.** A string is decoded on the way in and re-escaped minimally
on the way out, which is the opposite rule and is also jq's. PowerShell's
`ConvertTo-Json` makes the difference visible, because it escapes characters that
never needed escaping: in `tests/fixtures/real_world/culture.json`, 104
apostrophes and 48 solidi arrive as escape sequences and come back as bare
characters, while the 31 raw non-ASCII bytes in the same document pass through
untouched. Numbers keep their spelling; strings keep their meaning. All three
counts are asserted in `tests/real_world.rs`, and `tests/claims.rs` fails if this
paragraph and those constants stop agreeing.

**A stream's exit status accounts for every document, not just the last.** A
failure followed by a success exits 0 under jq, which hides it from a script
running under `set -e`:

    $ printf '1 {"a":2}' | jq -c .a
    jq: error (at <stdin>:0): Cannot index number with string "a"
    2
    [exit 0]

    $ printf '1 {"a":2}' | jaq-lite -c .a
    jaq-lite: <stdin>: Cannot index number with string "a"
    2
    [exit 5]

Both transcripts print the failure before the document that followed it, because
output is flushed before anything is written to standard error. That ordering is
asserted where it is captured, so it cannot quietly stop being true.

Where the input is malformed, the reported position is the byte that is wrong
rather than the end of the token that contains it:

    $ printf '{1:2}' | jq -c .
    jq: parse error: Object keys must be strings at line 1, column 3
    [exit 5]

    $ printf '{1:2}' | jaq-lite -c .
    jaq-lite: <stdin>: line 1, column 2: expected a string as the object key
    [exit 5]

jq is also more permissive than RFC 8259 allows: it accepts `inf`, `NaN`, `+1`,
`.5`, `5.`, `01`, `00`, `1.` and `0.` at exit 0. This project rejects all nine,
which is a large part of what the rejection score below measures.

## Conformance

Scored on every run against the vendored JSONTestSuite corpus, printed by the
harness itself:

    RFC 8259 conformance -- JSONTestSuite 1ef36fa, 318 files
      y_  must accept  : 95/95
      n_  must reject  : 188/188
      i_  our choice   : 10 accepted, 25 rejected, of 35 (implementation-defined)

95 documents that must parse, 188 that must be rejected, and 35 that RFC 8259
leaves to the implementation. The first two numbers are asserted as floors, so a
regression fails a test rather than quietly lowering a number in this file. For
the third group a count would prove little, since a different ten accepted would
print the same line, so the decision taken on each file is recorded with its
reason in `tests/i_decisions.tsv`.

JSONTestSuite is adversarial by construction, and passing it says nothing about
the JSON a build system emits. A second corpus sits beside it:
`tests/fixtures/real_world/` holds documents that `cargo metadata`, rustc's
`--message-format=json` and PowerShell's `ConvertTo-Json` really produced, kept
byte for byte. Every one of them survives a compact reprint and a second parse,
and the ones their producer emitted without whitespace come back byte for byte --
the same bytes cargo and rustc wrote rather than an equivalent document. The
commands, the substitutions applied before they were kept, and what is asserted
about them are in that directory's `PROVENANCE.md`. It found no behavioural
defect, which is worth stating plainly: what it corrected was the paragraph on
strings above.

## Speed

    PARSE:     26.5 MiB/s
    SERIALIZE: 197.9 MiB/s

Measured by `tests/throughput.rs` over a document of 1196671 bytes that the test
builds for itself, on AMD Ryzen 5 4600H with Radeon Graphics, Ubuntu on WSL2.

The loop is bounded by time rather than by a round count, so the same file yields
a usable sample in a debug build and in a release build. Both figures above are
substituted into this file out of the output of the run that produced them, so
the number here is the number the machine printed:

    cargo test --release --test throughput -- --nocapture --test-threads=1

There are no percentiles here, no outlier rejection and no statistical comparison
between runs. `criterion` is the crate that provides those, and what replaced it
is one `std::time::Instant` rather than a smaller version of that crate; the
difference is spelled out in [STDLIB.md](STDLIB.md). What the test asserts is a
floor and never the figure -- 5 MiB/s in a release build, 1 in a debug build,
raisable through `JAQ_PARSE_FLOOR` and `JAQ_SERIALIZE_FLOOR` and lowerable by
nothing -- so a regression that changes the shape of the algorithm fails a test,
while an unlucky sample on a busy machine does not.

One sample is one sample. Two runs of this binary on this machine, minutes apart
with nothing else started, differed by nearly a factor of two; both figures were
honest and neither of them was the speed of this tool. So the floor sits at a
fifth of the slower of those two runs rather than just under either, the figure
above is one run rather than a property of the parser, and the floor is the only
thing the suite asserts. What gets printed is the mean over the window and not
the fastest round in it: interference only ever slows a round down, so the fastest
round is the flattering one to report and it is not what a caller gets. Both runs
are in [BUILD_LOG.md](BUILD_LOG.md).

The document is built by the test rather than read from `tests/fixtures`, because
the 95 corpus documents that must parse total 1190 bytes between them: timing
those would measure the cost of calling a function 95 times.

## Reproducible build

Two `cargo build --release` runs on this source produce byte-identical
binaries on one machine, and `scripts/reproducible_build.sh` is the check
rather than the claim. It builds the crate twice into directories whose paths
differ in length, hashes both binaries, builds a third with the determinism
settings inverted, and prints:

    1  two builds, unequal path lengths, same hash
    2  control with debug=2 strip=none must differ
    3  no build path, home, rustup or cargo in it
    4  the two sizes are equal

It exits non-zero unless all four hold, so CI can gate on it, and the
`byte-identical rebuild` job does.

Assertion 2 is the one worth explaining. Two builds of one source agreeing is
weak evidence by itself, because a build that ignored its own settings would
agree too. The control inverts `debug` and `strip` -- the two settings that
decide whether a binary carries facts about the machine that built it -- and
it has to differ. A checker that cannot fail is not a checker.

The unequal path lengths matter for the same reason. An earlier version of
this harness built in `/tmp/a` and `/tmp/b`, and its control matched when it
should have differed: an absolute path leaked into a binary shifts nothing
downstream if the path it replaces is the same length, so the scan came back
clean because the leak was invisible rather than absent.

Determinism here comes from three keys in `[profile.release]` --
`codegen-units = 1`, `debug = 0`, `strip = "symbols"` -- and deliberately not
from `RUSTFLAGS`. A hash that only reproduces when the reader remembers to
export a long environment variable is a hash that quietly stops matching, and
a bare `cargo build --release` has to be the command that produces it.

What does not travel is the number. The sha256 measured on the author's
laptop is recorded in [BUILD_LOG.md](BUILD_LOG.md); a GitHub-hosted
`ubuntu-latest` runner produced a different one, twice, forty minutes apart
on two separate virtual machines. So the published constant is a function of
the host toolchain, you should not expect to reproduce it, and nothing should
be gated on it. What reproduces is the property above, and it has now held on
three machines.

## Design notes

The modules split along the problem rather than along a crate layout. `lexer.rs`
turns bytes into tokens and is the only place that validates UTF-8 or resolves an
escape; `parser.rs` turns tokens into a `Value` and is the only place that counts
document depth; `serializer.rs` turns a `Value` back into bytes and is the only
place that knows what two-space indentation looks like. `query.rs` is a second
front end over the same value model, with its own scanner, its own recursive
descent and its own depth counter, because a filter is a different language from a
document and one tokeniser pretending to be two would have to know which it was
being. `diag.rs` draws the caret snippet for both of them, `color.rs` decides
whether anything is painted, `error.rs` and `value.rs` hold the two types every
other module names, `lib.rs` is the public surface -- `parse`, `parse_stream`,
`write`, `to_string`, `Filter` -- and `main.rs` is argument parsing and exit codes
and nothing else.

**A number keeps the bytes it was written with.** `Number` stores the original
text beside an `f64`, and the serializer writes the text back, which is why the
compatibility table above shows `1e2` surviving as `1e2`. Reformatting is where
JSON tools quietly lose information, and it is also the decision that keeps float
printing -- a genuinely hard algorithm, and one of the larger things a
dependency would have been carrying -- out of a project that is not allowed to
depend on one.

**An object is a `Vec` of pairs, not a map.** Insertion order is what jq
preserves and what makes a byte-exact round trip possible: a `HashMap` would have
made the round-trip property untestable and a `BTreeMap` would have sorted the
keys. The cost is a linear key lookup, stated among the limits below rather than
hidden.

**A position is a byte offset; line and column are computed from it.**
`ParseError` and `FilterError` each carry an offset and a kind, and count lines
only when asked, so nothing pays for a line table it never prints. The kind is a
separate public enum from the message, which is what lets `main.rs` choose an exit
code and `tests/query.rs` name eleven distinct failures without matching on
English.

**A depth cap is an explicit counter, never the stack.** Both recursive descents
carry a `u32` and refuse before recursing, so deeply nested input is a diagnostic
rather than a stack overflow. Both caps are private, and both are readable back
out of the error variant that reports them: the limit is a field rather than prose
inside a message, which is the only way a dependent can learn a number the crate
does not export.

**Anything measured is asserted as a floor, never as a figure.** The throughput
test, the conformance counts and the substitution ledger share one idiom: read a
committed number, take the maximum of it and whatever the environment asks for,
and fail below the result. A floor can be raised by a passing run and lowered by
nothing, which is the opposite of a number typed into a document.

## Honest limits

Stated plainly rather than omitted.

The caret counts one column per character, not one per terminal cell. A
character a terminal draws two cells wide -- most CJK, and most emoji -- shifts
the caret one cell to the left of its target for each such character earlier on
the same line. Correcting for that needs the Unicode East Asian Width table,
which is the larger half of what a diagnostics crate carries, and the line and
column in the message above the snippet are right either way.

There are two nesting limits, neither of them configurable, and there is no
`--max-depth` for either: 128 levels for a document, 64 for the parentheses
in a filter. A limit is what turns deeply nested input into a diagnostic rather
than a stack overflow, and both numbers are far past anything written by hand.
Both are private constants, so this section is the only place a reader can see
either number; `tests/claims.rs` reads them back out of the source and fails if
the numbers here stop matching.

An object is a list of pairs, so looking up a key is linear -- O(n) in the number
of keys in that object. For what this tool is pointed at, which is configuration
files, API responses and log lines, that beats hashing outright; for an object
with thousands of keys it does not, and nothing here switches representation to
find out. The trade is the one in the design notes above: insertion order and a
byte-exact round trip are worth more here than a constant-time lookup.

The filter language has no functions. `.a`, `."a b"`, `.[0]`, `.[-1]`, `.[]`,
`.["a b"]`, `|`, `,`, `?` and parentheses are the whole grammar. `length`, `map`,
`select` and every other builtin is refused at compile time with its own name in
the message rather than silently ignored, because a tool that accepted `length`
and returned nothing would be worse than one that will not compile it. What is
here is the path-and-stream core that most jq one-liners are made of.

`JQ_COLORS` is not read. Whether anything is painted follows `-M`, then `-C`, then
`NO_COLOR`, then whether the stream is a terminal, which is the precedence jq
1.8.1 was measured to use; but the palette those rules switch on is fixed. jq's
variable takes a colon-separated list of SGR parameter strings, and reading it
would mean validating a small language whose entire effect is to change six
colours.

## Standard library substitutions

Every crate this project would ordinarily have used, and what replaced it, is
listed in [STDLIB.md](STDLIB.md).

## Attribution

Test fixtures are vendored third-party data, not code, and are attributed in
`tests/fixtures/ATTRIBUTION.md`. They are never compiled into the binary.

## License

MIT. See [LICENSE](LICENSE).
