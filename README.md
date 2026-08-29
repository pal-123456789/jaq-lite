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

## Design notes

Filled in as the modules land.

## Honest limits

Stated plainly rather than omitted.

The caret counts one column per character, not one per terminal cell. A
character a terminal draws two cells wide -- most CJK, and most emoji -- shifts
the caret one cell to the left of its target for each such character earlier on
the same line. Correcting for that needs the Unicode East Asian Width table,
which is the larger half of what a diagnostics crate carries, and the line and
column in the message above the snippet are right either way.

The nesting limit is 128 and is not configurable: there is no `--max-depth`. The
limit is what turns deeply nested input into a diagnostic instead of a stack
overflow, and 128 is far past anything written by hand.

## Standard library substitutions

Every crate this project would ordinarily have used, and what replaced it, is
listed in [STDLIB.md](STDLIB.md).

## Attribution

Test fixtures are vendored third-party data, not code, and are attributed in
`tests/fixtures/ATTRIBUTION.md`. They are never compiled into the binary.

## License

MIT. See [LICENSE](LICENSE).