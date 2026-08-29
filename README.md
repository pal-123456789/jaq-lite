# jaq-lite

<!-- The CI status badge belongs on this line. It is copied from the Actions
     page at the commit that adds the workflow, never hand-typed. -->

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

jaq-lite reports a parse error with the offending line, a caret under the exact
byte, and the reason, in the style rustc uses. Example goes here once the
renderer exists.

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

## Design notes

Filled in as the modules land.

## Honest limits

Stated plainly rather than omitted. Filled in as they are measured.

## Standard library substitutions

Every crate this project would ordinarily have used, and what replaced it, is
listed in [STDLIB.md](STDLIB.md).

## Attribution

Test fixtures are vendored third-party data, not code, and are attributed in
`tests/fixtures/ATTRIBUTION.md`. They are never compiled into the binary.

## License

MIT. See [LICENSE](LICENSE).