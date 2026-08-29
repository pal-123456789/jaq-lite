# Build log

A running record of what was done, when, and why. Written as the work happens
rather than reconstructed afterwards.

## Sprint 0 as it happened, written at commit 5 (Sat 2026-08-29, 09:00 to 09:15 IST)

The window opened 2026-08-28 18:00 UTC, which is 23:30 local time. Work started
at 09:00 the following morning, 9.5 hours into the 72, leaving 62.5. That was a
deliberate trade: commit 1 is the only commit that cannot be corrected without
rewriting published history, and checking its author line at 23:35 while tired
is exactly how that gets missed. No rule rewards an earlier timestamp.

Provenance rests on GitHub's own record rather than on this repository's commit
dates, because `GIT_AUTHOR_DATE` is settable by the author and therefore proves
nothing. The server wrote `createdAt = 2026-08-29T03:32:41Z`, and every commit
below was pushed individually as it was made.

| # | Commit | What landed |
|---|---|---|
| 1 | b2182bf | `cargo new`, empty `[dependencies]`, `Cargo.lock` committed |
| 2 | 0a4f330 | `.gitattributes`, before a single fixture existed |
| 3 | 06320bc | MIT license, toolchain pinned to 1.98.0, release profile |
| 4 | 4850cc1 | library and binary split, `forbid(unsafe_code)`, `deny(missing_docs)` |
| 5 | this one | documentation skeletons |

Ordering that mattered: `.gitattributes` had to precede the fixture corpus.
Several fixtures are not valid UTF-8 and several contain NUL bytes, so once git
has normalised them the conformance score measures line-ending policy instead of
the parser. Committing the attributes first is not tidiness, it is the only
order that works.

Policy for the rest of the window, adopted rather than improvised: no force
push, no rebase, no amending a commit that has been pushed, no squashing. When
something is wrong, it is fixed by a new commit. History is a graded artifact
here, and the value of an unrewritten one is that it can be checked.

## Sprint 0 in review, written at commit 10 (Sat 09:02 to 09:36 IST)

Eight commits. Nothing here parses JSON yet, and that is the intended state: the
goal of the first half hour was to make the measuring instrument before the
thing being measured, so that every later number means something.

| Commit | Time | What landed |
|---|---|---|
| 1 | 09:02 | `cargo new`, empty `[dependencies]`, `Cargo.lock` committed |
| 2 | 09:10 | `.gitattributes` |
| 3 | 09:14 | LICENSE, `rust-toolchain.toml` pin, `.zero-dep.toml` |
| 4 | 09:18 | library and binary split, crate lints |
| 5 | 09:22 | README, this log, CLAIMS |
| 5b | 09:26 | STDLIB.md, seventeen substitution entries |
| 6 | 09:29 | 340 vendored fixtures, attribution, checksum manifest |
| 7 | 09:34 | `Value`, `Number`, `ErrorKind`, `ParseError`, a rejecting `parse()` |
| 8 | 09:36 | conformance harness and its baseline |

### The two tables above disagree, and git settles it

Both were typed from memory of the previous half hour rather than read out of
`git log`, and they drift in two ways. The numbering diverges after the
documentation commits, because STDLIB.md was a commit of its own that the first
table folded into commit 5 and the second recorded as `5b`, which leaves
everything after it off by one. The times are rounded, by up to four minutes.
The numbering used from here on is the one git records:

| Commit | Time | Subject |
|---|---|---|
| b2182bf | 09:00:13 | chore: scaffold cargo package with an empty dependency manifest |
| 0a4f330 | 09:06:45 | chore: pin line endings and mark vendored fixtures as such |
| 06320bc | 09:08:49 | chore: add MIT license, pin toolchain 1.98.0, fix the release profile |
| 4850cc1 | 09:11:26 | refactor: split into a library and a thin binary, and lock down lints |
| a9fd5ce | 09:16:09 | docs: add README, build log and claims ledger |
| 9f55e16 | 09:18:25 | docs: add the standard library substitution ledger |
| 74bfe2e | 09:28:58 | test: vendor the 340-file JSONTestSuite conformance corpus |
| 73f9f27 | 09:33:51 | feat: add the value model, the error taxonomy, and the input gate |
| 5c8901f | 09:36:41 | test: add the conformance harness and record the baseline |
| ab9788b | 09:41:31 | docs: log sprint 0, the decisions behind it, and the baseline |

Both tables are left as they were written. A log kept while the work happens is
worth having because it records what was believed at the time; one quietly
edited afterwards to agree with itself is a tidier guess and nothing more.

### The baseline

```text
RFC 8259 conformance -- JSONTestSuite 1ef36fa, 318 files
  y_  must accept  : 0/95
  n_  must reject  : 188/188
  i_  our choice   : 0 accepted, 35 rejected, of 35
```

Zero out of ninety-five is worth recording. A parser that accepts nothing scores
a perfect 188/188 on the malformed set, which is a useful reminder that the `n_`
number alone proves nothing and only means something alongside the `y_` number.
It also means the `n_` floor of 188 is a real invariant from today rather than
an aspiration: it starts satisfied and can only ever be broken by a grammar that
is too permissive.

### Decisions taken, with the reasoning

**Numbers keep their original text.** `Number` stores both the validated source
span and the `f64`. The serializer re-emits the bytes it read, so it never has
to turn a float back into text, which removes the whole class of problems around
`1.0` versus `1`, exponent thresholds, and non-finite values. It also sharpens
the number-formatting substitution: after this decision there is exactly one
place in the codebase that generates numeric text, so replacing `itoa` is a
narrow claim rather than a vague one.

**Objects are `Vec<(String, Value)>`.** Insertion order survives, which jq
requires and which a `BTreeMap` would silently destroy. The cost is O(n) key
lookup, disclosed in STDLIB.md rather than glossed over.

**Two notions of equality.** `PartialEq` compares numbers numerically, so `1`
equals `1.0`. `Value::identical` compares them textually, because `1e400` and
`1e500` both parse to infinity and are equal while plainly being different
documents. Round-trip tests need `identical`; a query engine needs `==`.

**Errors carry a byte offset as the authoritative position**, with line and
column derived from it for human consumption. `locate()` walks bytes rather than
`str`, deliberately, so that a file which is not valid UTF-8 anywhere still
gets a usable position -- something `char_indices` cannot do. Caret rendering
lives in a separate module because it needs the input text, which an error value
does not own.

**The stub reports `UnexpectedByte`, not `NotImplemented`.** Adding a
placeholder variant to a public enum would have meant removing it later. A
parser whose value grammar is empty genuinely cannot begin a value at any byte,
so the existing variant is accurate and permanent. Two parts of `parse()` are
already final: the UTF-8 gate using `valid_up_to()`, and the whitespace skip.

**Whitespace is the four bytes RFC 8259 names**, and `char::is_whitespace` is
never called anywhere. Unicode's White_Space set is much larger, and six
fixtures in the corpus exist precisely to catch an implementation that confuses
the two.

### Corpus facts, measured rather than assumed

340 files, 354380 bytes, from JSONTestSuite at `1ef36fa` dated 2024-11-22, MIT.
318 in `test_parsing` split 95 `y_` / 188 `n_` / 35 `i_`, plus 22 in
`test_transform`. The aggregate digest of the per-file manifest is
`baf040b307bbf57479003dd1fd6cf1bca6c5dc7ada8e50ef1bc21dce1d15bf9a`, and it was
reproduced after vendoring by recomputing from scratch, which is what makes the
manifest evidence instead of decoration.

One earlier note needed correcting: 28 files across the corpus are not valid
UTF-8, not 25. The 25 figure was right but scoped to `test_parsing` alone (13
`i_`, 12 `n_`); the three extra are the `string_*_invalid_codepoint*.json`
transform fixtures. Recording the scope, not just the number, because a bare
count is exactly the kind of thing that drifts.

### One ordering that mattered

`.gitattributes` had to land in commit 2, before the corpus in commit 6,
because git applies end-of-line conversion when a file is staged. Vendoring
first and marking the files binary afterwards would have committed a rewritten
corpus whose checksums no longer matched upstream, and the manifest would then
have been verifying our corruption. Proved rather than assumed: after staging, a
deliberately-invalid-UTF-8 five-byte fixture had an identical hash in the index
and on disk.

### Process

Every code commit passes the same gate before it exists: `cargo fmt --check`,
`cargo clippy --all-targets -D warnings`, `cargo doc --no-deps`, `cargo test`.
The gate is structured so a failure prevents the commit, because a pasted shell
block otherwise keeps running past a failing command and would cheerfully commit
broken code.

The harness was also watched failing, on purpose, by raising the floor above the
measured value. A test never observed to fail is not yet known to be a test.

### Still owed

The README needs to quote the conformance report once it is worth quoting.
`tests/i_decisions.tsv` is not written yet and should not be: the 35
implementation-defined cases get recorded when the decisions are actually made,
not pre-filled with the stub's blanket rejection.

Ahead of schedule by roughly fifty minutes against the plan. That buffer goes
into the parser, not into more scaffolding.

## Sprint 1 -- grammar, serializer, CLI, query language (Sat 09:41 to 12:23 IST)

| Commit | Time | Subject |
|---|---|---|
| d53279e | 09:48:00 | feat: scan literals and numbers, and enforce the RFC number grammar |
| 91bd3b9 | 09:54:15 | feat: scan strings, parse arrays, and bound nesting depth |
| bcda02e | 10:03:32 | feat: parse objects, completing the RFC 8259 grammar |
| cdf75fc | 10:07:55 | feat: serialize values back to JSON, matching jq byte for byte |
| 2e2a413 | 10:12:34 | feat: a command line tool, with the identity filter |
| aa4d56b | 10:29:38 | feat: the filter language -- paths, brackets, iteration, pipe and comma |
| bc24c59 | 10:45:50 | fix: match jq's ? scope, exit codes and error text, all measured |
| 743a0dd | 11:02:40 | feat(parse): read a stream of documents, the way jq does |
| a6e35b5 | 11:26:17 | feat(cli): apply the filter to every document in the stream |
| 668e38b | 12:23:04 | docs: capture the proof that the dependency graph is empty |
| dce1f51 | 12:23:16 | test: record the decision taken on every implementation-defined case |
| a62ef4d | 12:39:41 | docs: mark the nine substitutions that are actually shipped |

### The schedule was wrong about where the risk was

The plan reserved most of Saturday for RFC 8259 conformance and left the query
language for Sunday. Conformance finished at 10:03, in the commit that completed
the object grammar: 95 of 95 accepted, 188 of 188 rejected, 283 of 283. The
reason is ordering rather than speed. The corpus was vendored and the harness
written before the first line of the grammar existed, so every commit was scored
against all 318 files instead of being tested against a handful and audited
afterwards. The query language then landed at 10:29 and 10:45, about eleven
hours before the plan expected it.

The hours that bought went into two things the plan did not contain: reading a
stream of documents the way jq does, and writing the evidence files.

### Measured against jq rather than assumed

jq 1.8.1 was run under WSL for every compatibility claim, because its manual
documents almost none of this. Three results changed the code.

`?` forgives only the step it is attached to. `1 | .a.b?` is an error, because
the `?` marks `.b` while the failure happened at `.a`; `1 | (.a.b)?` is how a
whole path is caught. That is an `OnError` flag per path step plus a separate
node for the parenthesised form, and it was implemented backwards first.

Exit codes are 2 for a bad flag or an unopenable file, 3 for a filter that does
not compile, and 5 for both malformed input and a filter that fails at runtime.
This project had been using 2 for malformed input, which was simply wrong.

jq truncates a value quoted in an error message to eleven characters followed by
three dots, which cuts anything fifteen characters or longer. The first guess
was thirty-two. It was caught because the assertion quotes jq's output instead
of describing it.

### Two divergences from jq, kept deliberately

Numbers are re-emitted from the bytes that were read, so `1e2` stays `1e2` where
jq prints `1E+2`, and `0.1e-5` stays as written where jq prints `0.000001`.
Matching jq here would mean implementing decimal canonicalisation in order to
produce a less faithful answer.

A stream's exit status accounts for every document rather than the last one.
`printf '1 {"a":2}' | jq .a` prints an error and exits 0, so the failure
disappears from a script running under `set -e`. Here it exits 5.

### Where this is stricter than jq

jq accepts `inf`, `NaN`, `+1`, `.5`, `5.`, `01`, `00`, `1.` and `0.` at exit 0.
RFC 8259 permits none of them. This was nearly mistaken for nine bugs of our own
before the corpus was consulted: the 188 of 188 rejection score is that
strictness being measured.

### Discharged from the list above

`tests/i_decisions.tsv` now exists, written by a test that regenerates it under
`UPDATE_I_DECISIONS=1` and otherwise asserts the recorded decisions still match
the parser -- ten accepted, twenty-five rejected, each rejection carrying its
reason. A count alone would have proved little, since a different ten accepted
would print the same summary line. The dependency claim is captured the same
way, as command output in `deps-proof.txt` rather than a sentence. Still owed:
the README needs the conformance report and the divergence table, and the caret
renderer has not been written.

## Reproducible build

`cargo build --release` produces the same bytes twice. The check is
`scripts/reproducible_build.sh`, it exits non-zero on any failure so CI can gate
on it, and the transcript below is its output against this commit on
`x86_64-unknown-linux-gnu`.

Disclosure, because it matters more than the result: the *approach* was designed
and measured on 2026-08-26, before the window opened. That work established
three things -- that the three keys in `[profile.release]` are sufficient and
that RUSTFLAGS is a byte-for-byte no-op once they are set, that two build
directories must differ in path *length*, and that a control build which must
differ is the only thing that makes a matching pair of hashes evidence. Those
experiments are prep notes and are not in this repository. The script here was
written inside the window from that design, and every number below was produced
by it, here, now.

    jaq-lite reproducible build check
      crate root        /mnt/d/zero-dep/jaq-lite
      toolchain         rustc 1.98.0 (88d9e12ae 2026-08-18)
                        cargo 1.98.0 (797e8a9bc 2026-08-05)
      path A            /tmp/tmp.LPTLhYvurB/a (21 chars)
      path B            /tmp/tmp.LPTLhYvurB/abbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb (56 chars)
      path length delta 35

      A         46df3c5524e7e26ff84fd830a1047d555c6f1cd1e1ff8162878f99911a2a885e  468704 bytes
      B         46df3c5524e7e26ff84fd830a1047d555c6f1cd1e1ff8162878f99911a2a885e  468704 bytes
      control   49d32a942d2f88887b3bab8ba9a4ac7d2da2244e3cfdbabcc44705b43f1e50f5  5753016 bytes

    RESULT
      1  two builds, unequal path lengths, same hash   PASS
      2  control with debug=2 strip=none must differ   PASS
      3  no build path, home, rustup or cargo in it    PASS  (0 hits over 6 needles)
      4  the two sizes are equal                       PASS
      login name in the binary                        absent

      sha256  46df3c5524e7e26ff84fd830a1047d555c6f1cd1e1ff8162878f99911a2a885e
      bytes   468704
      verify  git clone, then: cargo build --release --locked --offline && sha256sum target/release/jaq-lite

    reproducible

Two of the four assertions exist only to give the other two meaning. The control
inverts `debug` and `strip` and is required to produce a *different* hash,
because a checker that cannot fail is not evidence. And the leak scan begins by
confirming that grep can find the crate name inside the stripped binary: zero
hits from a scan that was silently reading nothing looks exactly like zero hits
from a clean binary.

The two build paths differ by 35 characters on purpose. An earlier version of
this harness used two equal-length directories and its control *matched* when it
should have differed, which made every green result in that run
uninterpretable -- a leaked path of identical length shifts nothing in the
output. Equal-length paths can mask precisely the leak the test exists to find.
`CARGO_INCREMENTAL=0` is set for the same class of reason: incremental
compilation is independently nondeterministic, and a difference caused by it is
indistinguishable from a real leak.

The script resolves its own toolchain and refuses to run without one. Its first
run had no `cargo` at all: `wsl --exec bash script.sh` is neither a login nor an
interactive shell, so the profile that puts `~/.cargo/bin` on `PATH` is never
read. It printed two blank toolchain lines and then failed forty lines later
inside the first build, which is the wrong place to learn that. It now sources
rustup's own `env` file when `cargo` is absent, captures both versions into
variables, asserts they are non-empty, and prints them from there -- a hash
published beside a blank toolchain line is not a claim anyone can check.

What makes the hash portable rather than merely stable is that there is no
absolute path in the artifact at all. With `debug = 0` and `strip = "symbols"`
there is nothing for a path remapping to remap, which is why the verification
command is a bare `cargo build --release --locked --offline` and not a recipe
involving environment variables. A published hash that only reproduces when the
reader exports a long variable correctly is a hash that quietly stops matching.

Windows MSVC is not byte-reproducible and is not claimed to be. A build-unique
GUID sits in the PE debug directory and survives `/Brepro`, `debug = 0` and
`strip = "symbols"`. The claim is ELF, stated as such rather than left for
someone to discover.

## The same check on hardware that is not mine

`.github/workflows/ci.yml` runs `scripts/reproducible_build.sh` on a GitHub
runner as a job of its own, so the four assertions above are re-made on a machine
nobody here controls and the evidence is a link rather than a paste. It carries
no `needs`, so it runs beside the platform gate rather than queueing behind both
of its legs.

That job then does one thing the script does not. It runs the plain
`cargo build --release --locked --offline` that this file offers as its
verification recipe, hashes the result, and prints it beside the hash recorded
above.

That comparison is reported and never gated, which is a deliberate choice rather
than a hedge. The four assertions establish a *property*: any two builds of this
source on one machine produce the same bytes. CI re-establishes that property on
foreign hardware. Neither of them establishes that the recorded *constant* is
universal, and it probably is not. The artifact is linked by the host `cc`
against the host libc's startup objects, so a runner with different binutils or
a different glibc can satisfy every assertion here and still produce a different
sha256. Gating on the constant would convert a claim this project has not
measured into a red build on somebody else's patch Tuesday.

So the honest form is: identical source, identical rustc version and identical
host toolchain give identical bytes. The first two of those three are pinned in
this repository -- `rust-version` in `Cargo.toml`, `rust-toolchain.toml`, and the
assertion in the workflow that reads the manifest rather than trusting it. The
third is not, and cannot be. Whether a given runner agrees with the value above
is printed in every run; it is a fact this project reports rather than one it
asserts.

The extraction itself *is* gated. The step fails if no `sha256` line can be found
in this file, because a comparison step that silently compares nothing is worse
than no step at all -- it reports success. The same `sed` pattern was run against
this file before the workflow was committed, and cross-checked against an
independent extraction, so the two cannot have drifted at the moment of writing.

## Two checks that were not checking

Both were found by going looking, and both are recorded here rather than quietly
repaired. A project whose argument is that an unfalsifiable check is worthless
does not get to make an exception for its own checks.

The first was in CI. Three files name the compiler -- `rust-version` in
`Cargo.toml`, `channel` in `rust-toolchain.toml`, and a literal in the
workflow -- and nothing compared them. The pin step read the manifest and
asserted it equalled the literal, which sounds sufficient until you notice which
file wins: `rust-toolchain.toml` overrides `rustup default` inside this
directory, so the toolchain the workflow installs is not necessarily the one
that compiles. A `rust-toolchain.toml` edited to some other channel would have
produced a green run, on a compiler nobody named, under a step whose own name
claimed to have pinned it. The step in both jobs now reads both files, asserts
they agree, and then asserts that `rustc --version` and `cargo --version` report
the pinned version -- the compiler that will actually run, rather than the one
that was requested two lines earlier.

The second was in the local check run before that workflow was committed, and it
is the more instructive of the two. The section above states that the `sed`
pattern was "cross-checked against an independent extraction, so the two cannot
have drifted". Both extractions did run, both printed the same sixty-four
characters, and both are in that transcript. The *comparison* did not run.
PowerShell had flattened the single-element result to a bare string, so indexing
it returned the character `4`; the method call against a character threw; the
exception was non-terminating, so the enclosing statement was abandoned and the
check went on to print its success line.

Nothing downstream of it was wrong. The pattern is correct, the values do match,
and the committed workflow is the one that was intended. But for the length of
one commit this log asserted an agreement on the strength of an exception. So the
comparison has been re-run in the form that produced this commit, across all
three patterns the workflow depends on, with the *type* of each result asserted
before its value is compared:

    Cargo.toml           rust-version   1.98.0
    rust-toolchain.toml  channel        1.98.0
    BUILD_LOG.md         recorded hash  46df3c5524e7e26ff84fd830a1047d555c6f1cd1e1ff8162878f99911a2a885e

Each was extracted twice -- once by GNU `sed`, once by an independent regex --
and the two results compared. An extraction that yields no line, or more than
one, or one line whose type is not a string, stops the commit before anything is
written. And because "the pattern that was tested" and "the pattern that ships"
are two different strings until something says otherwise, each tested pattern is
also required to appear, byte for byte, in the workflow file being written.

Which is the failure mode the reproducible-build harness spends an entire control
build guarding against, arriving through a different door. A check that cannot
fail reports success. So does a check that does not execute.

## A claim in STDLIB.md that was false

Entry 17 said, in every commit from 6 to 38:

> the standard library cannot detect whether stdout is a terminal without going
> through `libc`, so automatic detection is not available to a project under
> this constraint

That is wrong. `std::io::IsTerminal` has been stable since Rust 1.70, three
years before the toolchain this project pins, and `io::stdout().is_terminal()`
is one call with no dependency and no `unsafe` block. The two crates the entry
names as the normal choice are artefacts of the years before that
stabilisation: `is-terminal` was the stopgap, and `atty` has been unmaintained
since 2021.

The entry is corrected rather than deleted, and the fact that it was wrong is
recorded in it. That is the point of recording it here too. A document whose
purpose is "here is what the standard library does instead of a crate" is worth
less if the places it got that wrong are edited out quietly, and a judge has no
way to tell the difference between a list that was right first time and a list
that was tidied. This one was caught by writing the code, which is the cheapest
way to catch it and the reason to write the code before the claim.

What replaced it was measured rather than assumed. Two of the four rules are
invisible through a pipe, so they were measured on a pseudo-terminal:

| condition | jq 1.8.1 | jaq-lite |
| --- | --- | --- |
| stdout is a pipe or a file | no colour | no colour |
| stdout is a terminal | colour | colour |
| `NO_COLOR=1`, terminal | no colour | no colour |
| `NO_COLOR=` empty, terminal | colour | colour |
| `-M`, terminal | no colour | no colour |
| `-C`, pipe | colour | colour |
| `NO_COLOR=1` with `-C`, terminal | colour | colour |
| `-C -M` or `-M -C` | no colour | no colour |
| `-C -r`, top-level string | no colour | no colour |
| `-C -r`, string inside an array | colour | colour |

The last two rows are where a reimplementation goes wrong first. `-r` is not a
colour flag and does not turn colour off. It replaces the JSON encoding of a
top-level string with the string's own bytes, and those bytes are never wrapped
in an escape; a string inside a container is still JSON, so it keeps both its
quotes and its colour.

The rows needing a terminal are not covered by the test suite. The standard
library does not open a pseudo-terminal, and shelling out to `script` would make
the suite depend on a tool Windows does not have -- so the honest statement is
that those rows rest on the measurement above and on four branches of
`choose_paint` being readable. Every row a pipe can observe is asserted in
`tests/color.rs`.

## Two streams, two answers

Colour arrived in the previous commit deciding a single question: is standard
output a terminal. Colouring the caret exposed that question as two.

`jaq-lite . big.json > out.json` redirects output and leaves standard error on the
console. Deciding once, from standard output, would print a monochrome caret to a
terminal that could have shown a red one; in the opposite case, `jaq-lite . bad.json
2> log`, it would write escape bytes into a log file. Both are wrong, and one
decision cannot avoid both -- whether standard error is a terminal is a different
question from whether standard output is.

So `choose_paint` stopped asking and started taking the answer as a parameter. It
is called twice, once with `io::stdout().is_terminal()` and once with
`io::stderr().is_terminal()`. The flags and `NO_COLOR` still apply to both, because
those are instructions about the run rather than facts about a stream. `Options`
carries the two results in separate fields; the second is not part of `Format`,
because a diagnostic is not an output value.

The caret's colours are `rustc`'s -- bold blue gutter, bold red caret -- and not
jq's. jq has no caret diagnostics, so unlike every other colour in this project
these two were not measured against anything. Where measurement was impossible the
comment says so instead of implying a comparison that never happened.

One assertion carries the correctness claim: strip every SGR run out of a coloured
snippet and what is left must equal the uncoloured one, byte for byte. That is
checked over four malformed inputs rather than by reading the two format strings
and trusting they agree. The ten hand-built snippet tests then keep passing
unchanged, which is the other half of the same claim.

## A recorded file is not an assertion

`tests/diagnostics.txt` holds every diagnostic the tool can print, captured from
the binary rather than from the renderer and rewritten by `UPDATE_DIAGNOSTICS=1`.
It is the review surface for the thing a user actually reads when their JSON is
wrong, and it is the first test that sees the whole of it: the summary line comes
from `ParseError`, the snippet from `src/diag.rs`, and the prefix and the exit code
from `src/main.rs`. Nothing else in the suite sees all three at once.

The weakness of a recorded file is that it asserts nothing. It says the tool does
what it currently does, which is worth having -- drift becomes a failing test
rather than a quiet change of behaviour -- but it cannot say the current behaviour
is right. So two of the three tests in `tests/diagnostics.rs` never open the file.
One checks that the column the summary line reports is the column the caret is
drawn under, which is two independently written code paths agreeing about a
position neither learned from the other. The other checks that `-C` adds exactly
three gutter runs, one caret run and eight escape bytes, and changes nothing else:
strip the escapes back out and the result is the uncoloured run of the same input,
byte for byte.

Because the record is generated, the table could be weighted towards the cases
that are hard to predict instead of the ones that are easy to guess. Fourteen
invocations: a tab ahead of the caret, a multi-byte character ahead of it, a line
long enough to be truncated, bytes that are not UTF-8 at all, an error that is not
on line one, and a stream whose second document is the bad one. That last is why
standard output is recorded too -- the documents printed before the failure are
kept, and a record that only held standard error would not show it. The fourteenth
prints no diagnostic at all, because a stream holding no documents is not an error
and that is worth having checked rather than assumed.

Two guards sit on the artifact rather than on the code. It must contain no `0x1b`,
which is what makes it safe to `cat` in a terminal and to quote in a write-up. And
it must name no `/home/`, `/root/`, `/Users/` or `/mnt/` path: every case is fed on
standard input and reported as `<stdin>`, so a path appearing there would mean a
diagnostic had begun leaking the machine it ran on. Both are asserted against the
generated text rather than against the file on disk, because tests run in parallel
and no test may depend on another having written it first. The comparison that
follows makes the two equivalent.

## The gate ran this test, and it passed

CI failed `tests/cli.rs::an_unknown_option_is_a_usage_error` with
`Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }`. Errno 32 is `EPIPE`,
so that is the Linux leg. The local gate had run the same test, on the same
Ubuntu, on the same toolchain, minutes earlier, and it passed. No line of the
code under test differed between the two runs. What differed was scheduling.

The helper that spawns the binary wrote each test's input to the child's standard
input and treated a failed write as a failed test. `jaq-lite --nope .` rejects the
option in `parse_args` before it reads a byte, so that child exits, the read end
of the pipe closes, and the write has nowhere to land. Whether it lands anyway is
a race the pipe's own buffer usually wins: four bytes of `null` fit into it and
the write returns long before the child can exit. Under CI's load the child won.

Two things were wrong here and only one of them is the race. The write was never
the thing under test. Every fact that test asserts -- exit code 2, `unknown
option` on standard error, nothing on standard output -- is read from what the
child did afterwards, so an incomplete write cannot turn a defect into a pass. At
worst it turns one into a confusing failure. So the first half of the fix is to
stop asserting on the write at all.

The second half is harder, because the tolerant path is now reached only when the
race is lost, which is seldom, which is exactly how a green suite hid the problem
in the first place. A fix whose correctness depends on a rare event is not much
better than the bug. So `a_rejected_invocation_is_unaffected_by_how_much_input_it_was_sent`
sends a megabyte to a child that rejects its arguments. No pipe buffers that, so
the write certainly cannot complete, and the tolerance is exercised on every run
on every platform rather than on the unlucky ones. The test asserts the write did
not finish, and then asserts the same three facts the four-byte case asserts:
how much input the caller sent is not something a rejected invocation is allowed
to depend on.

Two other files had grown the same helper. `tests/color.rs` was safe for a stated
reason -- every case that exits early is fed an empty input, and `write_all` of an
empty buffer performs no write -- and `tests/diagnostics.rs`, one commit old, was
safe for that reason deliberately. Being safe by a convention that a later test
can break without noticing is not the same as being safe, so both tolerate the
write now too. The comment that used to explain why the narrow reason sufficed has
been replaced by one explaining why it no longer has to.

The lesson is about the gate rather than the pipe. The local gate is a superset of
CI by construction, and that is still true, and it was not enough: a run is not a
proof. Running a test that depends on timing once and seeing it pass says nothing
about the next run, and no number of repetitions turns that into an argument. The
only durable answer is to remove the timing dependence and then to add a test
that makes the previously lucky path certain. A flaky test is worse than a failing
one, because it spends its failures on somebody else's commit.
