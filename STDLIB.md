# Standard library substitutions

Every crate this project would ordinarily have reached for, what replaced it,
and why. Seventeen entries.

The `Status` line is `planned` until the replacing code exists, and is changed
to `shipped` only after the code has been found in the source tree by a check
that refuses to flip an entry it cannot evidence. An entry that still says
`planned` means the substitution did not happen and the claim is not being made.

---

1. **Normally:** `serde` + `serde_json`. **Instead:** a hand-written
   recursive-descent parser and serializer over a byte cursor.
   The headline substitution; the rest of this file is downstream of it. Being
   hand-written is also what makes RFC 8259 conformance a decision rather than
   an inherited behaviour.
   *Where:* `src/lexer.rs`, `src/parser.rs`, `src/serializer.rs` · *Status:* shipped

2. **Normally:** `clap` or `structopt`. **Instead:** a manual walk over
   `std::env::args()` with a positional query and file, a `--` terminator, and
   explicit rejection of unknown flags.
   Argument parsing for a tool with under a dozen flags is a loop, not a
   dependency. Unknown flags are rejected rather than ignored, which is the
   behaviour people actually rely on.
   *Where:* `src/main.rs` · *Status:* shipped

3. **Normally:** `rand`. **Instead:** SplitMix64 in fourteen lines.
   Deterministic by construction, so a property-test failure replays exactly
   from its seed instead of being irreproducible.
   *Where:* `tests/roundtrip_fuzz.rs` · *Status:* planned

4. **Normally:** `proptest` or `quickcheck`. **Instead:** a hand-written value
   generator driven by the SplitMix64 above.
   Stated plainly: there is no automatic shrinker. Counterexamples are
   minimised by hand and the minimised input is recorded in the test. An
   automatic shrinker was scoped and deliberately dropped for time rather than
   half-built.
   *Where:* `tests/roundtrip_fuzz.rs`, `tests/mutation_fuzz.rs` · *Status:* planned

5. **Normally:** `thiserror` or `anyhow`. **Instead:** `#[derive(Debug)]`, a
   manual `Display` impl, and `impl std::error::Error`.
   Two crates replaced by roughly three lines of boilerplate per error type.
   The error type is part of the public API, so writing it by hand also means
   its `Display` output is designed rather than generated.
   *Where:* `src/error.rs` · *Status:* shipped

6. **Normally:** `indexmap`, which is what `serde_json`'s `preserve_order`
   feature pulls in. **Instead:** `Vec<(String, Value)>`.
   Object key order is insertion order, matching `jq`, which is the behaviour
   that makes round-tripping byte-exact. The cost is O(n) key lookup, accepted
   and disclosed in the README rather than hidden.
   *Where:* `src/value.rs` · *Status:* shipped

7. **Normally:** `ryu` and `itoa` for float and integer formatting.
   **Instead:** parsed numbers are re-emitted verbatim from the exact byte span
   the grammar validated, and numbers the tool synthesizes go through
   `format_into` with `core::fmt::NumBuffer`.
   Because a parsed number is written back byte for byte, no float formatting
   happens on that path at all, which is how a thirty-digit integer survives a
   round trip. Integer formatting is therefore the only remaining place number
   text is generated, and `format_into` measured 1.68 times the speed of
   `to_string` over five million values with zero mismatches, including
   `i64::MIN`. Note for anyone following along: `NumBuffer` is not re-exported
   through `std::fmt`, so the import must be `core::fmt::NumBuffer` or the
   compiler answers with E0432. This is the nominated Package Killer.
   *Where:* `src/value.rs`, `src/serializer.rs` · *Status:* planned

8. **Normally:** `memchr`. **Instead:** plain byte-slice scanning over a
   string that has already been validated as UTF-8.
   Safe without any special care because UTF-8 continuation bytes are all
   above 0x7F and therefore can never collide with the ASCII delimiters JSON
   uses.
   *Where:* `src/lexer.rs` · *Status:* shipped

9. **Normally:** `simdutf8` or `encoding_rs`. **Instead:**
   `std::str::from_utf8`, with `Utf8Error::valid_up_to()` for the exact byte
   offset where the input stopped being valid.
   The offset is what turns "invalid UTF-8" into a diagnostic with a caret
   under the right byte. Twenty-five fixtures in the corpus are not valid
   UTF-8, so this path is exercised rather than theoretical.
   *Where:* `src/error.rs`, `src/lib.rs` · *Status:* shipped

10. **Normally:** `codespan-reporting`, `ariadne` or `miette`. **Instead:** a
    caret renderer of 160 lines in `src/diag.rs`, tests aside.
    Line, column, the source line, a caret under the offending character, and a
    reason. That is the whole feature those crates are usually pulled in for.
    What they carry besides it is multi-span layout and a Unicode display-width
    table, and neither is needed to point at one position in one line.
    One renderer serves both parsers: a document that is not JSON and a filter
    that does not compile print the same three lines, and both count columns with
    the same function, so a message and its caret cannot disagree.
    *Where:* `src/diag.rs`, `src/query.rs` · *Status:* shipped

11. **Normally:** `criterion`. **Instead:** a single `std::time::Instant`
    measurement around a fixed workload.
    Honest about scope: there are no percentiles, no outlier rejection and no
    statistical modelling. A percentile harness was planned and dropped, so
    the README quotes one measured figure rather than implying a distribution.
    *Where:* recorded in `README.md` and `CLAIMS.md` · *Status:* planned

12. **Normally:** `insta`. **Instead:** a recorded file, compared on every run,
    with an environment variable that rewrites it.
    Snapshot testing is a file comparison and a way to regenerate the file. The
    first of these is `tests/i_decisions.tsv`, which records the decision taken
    on each implementation-defined fixture and is rewritten by setting
    `UPDATE_I_DECISIONS=1`. The second is `tests/diagnostics.txt`, which
    records every diagnostic the binary can print, captured from the binary
    rather than from the renderer, and rewritten by `UPDATE_DIAGNOSTICS=1`.
    What a snapshot crate adds beyond a file and a switch is an interactive
    review command. What it cannot add is teeth: a record only says the tool
    does what it does today, so two of the three tests around this one never
    open it.
    *Where:* `tests/conformance.rs`, `tests/i_decisions.tsv`, `tests/diagnostics.rs`, `tests/diagnostics.txt` · *Status:* shipped

13. **Normally:** `walkdir` or `glob`. **Instead:** `std::fs::read_dir` with an
    explicit sort of the results.
    The sort is load-bearing rather than tidy: `read_dir` returns entries in
    alphabetical order on NTFS but in hash order on ext4, so without it the
    conformance report would be ordered differently on a developer machine and
    on the CI runner.
    *Where:* `tests/conformance.rs` · *Status:* shipped

14. **Normally:** `pretty_assertions`. **Instead:** a small helper that prints
    the first differing byte offset with the surrounding context.
    For byte-exact round-trip failures, the offset of the first difference is
    more useful than a coloured diff of two long lines.
    *Where:* `tests/roundtrip_fuzz.rs` · *Status:* planned

15. **Normally:** `jq` itself, invoked as an external binary. **Instead:** an
    in-process query engine, so the tool shells out to nothing.
    This one is about the spirit of the rule as much as the letter: an empty
    dependency manifest in a program that requires a separately installed
    binary at runtime has simply moved the dependency somewhere the manifest
    cannot see. `jq` is used during development to verify output compatibility,
    and is not required to run this tool.
    *Where:* `src/query.rs` · *Status:* shipped

16. **Normally:** `unicode-segmentation`. **Instead:** counting UTF-8 lead
    bytes, which is the unit a column number is already in, and byte iteration
    everywhere else.
    A column is one non-continuation byte together with the continuation bytes
    after it. That is deliberately not `char` iteration: a position has to be
    reportable for input that is not valid UTF-8 at all, and a `char_indices`
    walk cannot reach that case. Grapheme clusters would be a third unit again,
    and a caret under one code point is what `rustc` prints.
    Also the reason `char::is_whitespace()` is never used to skip JSON
    whitespace: Unicode `White_Space` is a much larger set than the four bytes
    RFC 8259 permits, and six fixtures in the corpus exist to catch exactly
    that mistake.
    *Where:* `src/error.rs`, `src/diag.rs`, `src/lexer.rs` · *Status:* shipped

17. **Normally:** `owo-colors` or `colored` for ANSI output, plus
    `is-terminal` or `atty` to decide whether to emit it. **Instead:** a short
    table of ANSI escape constants in `src/color.rs`, and `std::io::IsTerminal`
    to decide, which has been stable since Rust 1.70.
    This entry said the opposite from commit 6 to commit 38. It claimed the
    standard library cannot detect a terminal without going through `libc`, so
    detection was unavailable here and colour had to be opt-in. That was simply
    wrong. The correction is left visible rather than quietly rewritten: a log
    of what the standard library can do instead of a crate is worth less if the
    places it got that wrong are edited out of it. `is-terminal` exists for the
    years before 1.70 and `atty` has been unmaintained since 2021; on 1.98
    neither is needed for one method call on `Stdout`.
    The precedence -- `-M`, then `-C`, then `NO_COLOR`, then whether standard
    output is a terminal -- was measured against jq 1.8.1 on a pseudo-terminal
    rather than assumed, because two of those four are invisible through a
    pipe. Piped output contains no escape bytes at all, which `tests/color.rs`
    asserts on every run. The same table paints the caret diagnostics, with two
    colours that are `rustc`'s rather than jq's -- jq has no caret to draw, so
    there was nothing to measure -- and with the terminal question asked of
    standard error instead of standard output.
    *Where:* `src/color.rs`, `src/diag.rs`, `src/main.rs` · *Status:* shipped

---

## Vendored third-party material, disclosed

The conformance corpus in `tests/fixtures/` is vendored from JSONTestSuite by
Nicolas Seriot, MIT licensed. It is third-party **data**, not code: it is never
compiled into the binary, it does not appear in `Cargo.toml` or `Cargo.lock`,
and it is marked in `.gitattributes` as vendored so that it is excluded from the
repository language statistics. The upstream commit, the license text and a
per-file SHA-256 manifest are in `tests/fixtures/ATTRIBUTION.md` and
`tests/fixtures/FIXTURES_MANIFEST.sha256`.

This disclosure is here as well as in the README because the rule about
vendoring names this file specifically.