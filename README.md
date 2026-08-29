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

Three commands, no interpretation required:

    cargo tree
    cargo metadata --format-version 1 --no-deps
    grep -c "^\[\[package\]\]" Cargo.lock

Their captured output is committed as `deps-proof.txt`, and the four-line
excerpt showing a package count of exactly 1 is reproduced here once that file
exists.

## Install and run

    git clone https://github.com/pal-123456789/jaq-lite
    cd jaq-lite
    cargo build --release

One command, no flags, no feature selection, no network access after the clone.

## Usage

Filled in when the CLI accepts arguments.

## Diagnostics

jaq-lite reports a parse error with the offending line, a caret under the exact
byte, and the reason, in the style rustc uses. Example goes here once the
renderer exists.

## jq compatibility

Output is byte-compatible with `jq` wherever a choice exists. The differential
comparison and the table of deliberate divergences go here.

## Conformance

Scored against the JSONTestSuite corpus: 95 documents that must parse, 188 that
must be rejected, and 35 whose behaviour is implementation-defined and for which
this project publishes a decision and a reason for each one.

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