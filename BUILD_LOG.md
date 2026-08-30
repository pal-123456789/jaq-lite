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

That transcript is from commit 34. Its digest and its byte count are a measurement of
a run rather than a constant this project promises: thirty-one commits of source
landed after it, and the same harness on the same machine at commit 65 returns
`ade5b0db` and 490336 bytes. What the four assertions below establish is
reproducibility of the recipe -- same source, same toolchain, same bytes -- and they
are re-run and re-passed on every commit. The two figures are reconciled at the foot
of this file, in the section on the pre-freeze pass that measured the newer one.

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
sha256. Gating on the constant would convert a claim this project does not
make into a red build on somebody else's patch Tuesday.

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

## A generator that cannot fail is a generator that proves nothing

`tests/roundtrip_fuzz.rs` retires three crates at once, and the middle one is the
only one worth an explanation. SplitMix64 replaces `rand` in nine lines. A value
generator replaces `proptest`. An offset and a byte window replace
`pretty_assertions`. The first and third are substitutions of a small amount of
code for a dependency and there is nothing to say about them. The second is not,
because `proptest` does something this file does not do, and saying so plainly is
worth more than a green tick.

What `proptest` adds is a shrinker. When a property fails it searches for the
smallest input that still fails, and that search is most of what the library is
for. There is none here and there will not be one: it was scoped and dropped
rather than half-built. What stands in for it is that every value comes from its
own seed, every failure prints that seed, and `a_seed_reproduces_its_value_exactly`
asserts the seed is enough to get the value back. Minimising a counterexample is
therefore a manual step rather than an automatic one, which is a real cost. It is
not guesswork, which would be a different and much larger one.

The bigger hazard in a hand-written generator is not bias. It is emptiness. A
generator that emitted `null` five hundred times would satisfy every property in
this file, and the file would report six passing tests while having exercised one
variant. So generation keeps a census of what it built and which awkward things
it reached, and `the_generator_is_not_vacuous` asserts eleven counts are above
zero: all six variants, an empty container, a duplicate key, a string carrying a
quote or a backslash, a string carrying a control byte, and a character above the
BMP. The census is printed as well as asserted, because what a fuzzer covered is
the first question anyone asks about one, and the honest answer is a table of
numbers rather than an adjective.

The fixture property needed a decision. Round-tripping the corpus against the
files themselves is the strongest-sounding claim available and it is false:
fixtures carry whitespace this tool does not reproduce, and several write a
character as an escape that resolves to something shorter than it was written. So
the property is a fixed point instead -- parse, serialize, parse, serialize, and
the two serializations must match byte for byte while the two values stay
identical by literal text rather than by numeric value. That is deliberately the
weaker claim, and the test proves the weakening was necessary rather than
convenient: it counts the fixtures whose compact form differs from their file and
fails if that count is zero. Were it ever zero, the stronger property was
available and this file chose the weaker one for no reason.

One quieter result. `Value::identical` was written earlier for a test that did not
exist yet, which is how a function becomes dead code without anyone noticing. It
now has callers, and it is the entire reason these properties can tell `1.0` from
`1`.

A note added after the fact, because the first run of this test failed and the
failure was not in the tool.

Seed 16 built an object holding the key `b` twice. The serializer wrote all three
members, the parser returned two, and `identical` reported a difference. The parser
was right: `insert` in `src/parser.rs` applies last value wins, keeping the member
at the position where the key first appeared, which is what the three `y_` fixtures
containing duplicate keys require and what every order-preserving object model
does. What was wrong was the doc comment on `Value::Object`, which said duplicate
keys were retained as they appeared, and this test, which had been written
believing it.

So the generator was left alone and the property was split instead. A value the
parser could have produced round-trips to itself; a value it could not produce
round-trips to its projection, and projecting twice changes nothing.
`has_duplicate_key` chooses the branch, the test fails if either branch never ran,
and a unit test now pins the policy by reading it back out of the parser -- the
document `{"a":1,"b":2,"a":3}` must serialize as `{"a":3,"b":2}` -- so the fact the
property rests on is checked rather than assumed.

Two things are worth keeping from that. A property test needs its generator to stay
inside the image of the function under test, or the property is false for reasons
that have nothing to do with a defect, and the repair is usually to weaken the
property on the values outside rather than to stop generating them, because those
are exactly the values the normalising path is made of. And a doc comment is not
evidence of behaviour. This one was six words long, sat on a public type,
contradicted the code directly beneath it in the call graph, and survived
forty-two commits and every reading pass in this log until something mechanical
generated a value that cared.

## Two answers, and the input nobody chose

The corpus is 318 documents somebody wrote on purpose, each with a verdict
attached. That is a strong test of the cases a person thought of. It is a weak
test of the cases nobody did, and a parser's worst failures live there: an index
that runs one past the end of a truncated escape, a length read out of a document
that no longer contains it, a position reported for a line that was cut away.

`tests/mutation_fuzz.rs` takes the ninety-five fixtures that must parse and
damages them eight ways each -- a bit flipped, a byte replaced with one that means
something to a JSON parser, the document cut short, a byte inserted, a byte
dropped, a run copied -- and asks for an answer about all 760. None of those has
an expected verdict. The claim is only that there is an answer: `Ok` or `Err`,
never a panic, never a hang, never a position that cannot be pointed at. That is a
weaker claim than conformance and a much harder one to satisfy by accident.

Three details are the whole value of the file.

The generator is seeded from each fixture's position in a **sorted** listing, so
the 760 documents are the same 760 on every machine. `read_dir` gives filename
order on NTFS and something close to hash order on ext4; without the sort, a
failure found on a laptop would not reproduce in CI, and the log entry describing
it would be useless. The sort is a no-op locally and load-bearing in the one place
a reader might click.

The answer is taken through `std::panic::catch_unwind`. A panic in a `#[test]`
already fails it, but the report names a line in the parser rather than the input
that reached it, and one loop with 760 iterations makes that the only fact worth
having. Catching it lets the failure name the fixture, the round, the kind of
damage, and print the bytes ASCII-escaped so they survive a CI log verbatim.
`Cargo.toml` sets no `panic` key, so unwinding is on and this works; if that ever
changes, this file is one of the things that changes with it.

The rejection is checked, not just counted. The reported offset has to lie inside
the input: `locate` clamps a wild offset and would hide one, while the caret
renderer slices at it and would not, so the assertion belongs on the offset rather
than on the line and column it produced. The reported line has to be one more than
the number of newlines before that offset. And the reported column has to leave
the caret on the line it points at -- at a character of that line, or the one
position past its end -- which is the renderer's precondition rather than a second
implementation of `locate`. Restating a function inside its own test proves
nothing; stating what its caller needs from it proves something. A diagnostic that
points at an impossible position is the likeliest thing in this parser to be
wrong, because no fixture is shaped to provoke it and every fixture is
hand-written enough to avoid it.

Two smaller decisions. Every mutant that *parses* is held to the round-trip
property as well, so the accepting path gets tested by documents no one designed;
the test fails if no mutant parsed at all, and fails if none was rejected, since
either would mean it had stopped measuring anything. And SplitMix64 is duplicated
from `tests/roundtrip_fuzz.rs` rather than shared: every file under `tests/` is its
own crate, sharing means a `common` module, and an item in such a module that one
importer does not use is a warning, and a warning is a failed gate here. The
duplicate carries the same published test vector, so a change to either copy that
breaks the algorithm fails a test rather than silently producing a different
corpus. Duplication with a shared oracle is cheaper than a module that has to stay
exactly as wide as its narrowest user.

Nothing was found. Eight hundred and six damaged or truncated documents went in
and every one came back as an answer, with every rejection pointing at a position
inside the document it was handed. It is worth saying plainly that this file has
not yet paid for itself the way the round-trip generator did on its first run, and
worth saying just as plainly why it is still here: a test that finds nothing today
is a claim under continuous check, and the alternative to a claim under check is a
promise. The way this file becomes worthless is not by failing to find a defect --
it is by becoming unable to find one, which is why it asserts that both answers
occurred and that all six kinds of damage happened, and why those counts are
printed where a reader can see them rather than kept inside a passing test.

## A sentence that outlived the code it described

Entry 7 of `STDLIB.md` is the nominated Package Killer, which makes it the entry
a judge is most likely to read closely. Until this commit it said that numbers
the tool synthesizes go through `format_into` with `core::fmt::NumBuffer`, that
integer formatting is therefore the only remaining place number text is
generated, and that `format_into` measured 1.68 times the speed of `to_string`
over five million values with zero mismatches including `i64::MIN`.

Three sentences, one of them carrying a figure to two decimal places, and none of
them true of this program. `grep -rn 'NumBuffer\|format_into' src/` returns
nothing, and never returned anything. The measurement is real; it was taken in a
throwaway warm-up crate that is not part of this repository, and the entry was
written from the measurement rather than from the tree. The premise was wrong as
well: integer formatting is
not the only remaining place number text is generated here, it is not a place at
all. `Number::new` has exactly one call site and it is in the lexer. Nothing in
jaq-lite synthesizes a number. There was no code for a fast path to be in.

Worth recording is which safeguard held and which was never there. The `Status`
field read `planned`, and that field is governed by a rule in the preamble of
that same file: planned until the replacing code exists, flipped only by a check
that refuses an entry it cannot evidence. That rule worked exactly as written,
and it is why the audit found this immediately. The body of the entry was
governed by nothing, and a body can say anything at all, including a figure to
two decimal places.

The obvious fix is the wrong one. The obvious fix is to write the code so the
sentence becomes true: find somewhere a number gets synthesized, put
`format_into` there, flip the status. That is choosing a feature to make a
document true, which is the same error facing the other way, and in practice it
meant shipping a builtin on a Saturday evening to justify one sentence. The true
statement is stronger than the false one anyway. No number is
formatted in this program at all. A parsed number is written back from the byte
span the grammar validated, and there is no other kind of number in the value
model. `ryu` exists to solve shortest-round-trip float formatting, an algorithm
with papers behind it and a decade of bugs in the implementations that predate
it; the substitution here is not a faster version of that work, it is the absence
of any need for it. Entry 7 now says that, names the one call site, and reads
`shipped`.

Saying it is not enough, which is this project's own thesis pointed at its own
documentation, so `tests/claims.rs` now checks on every `cargo test` what a
program can check about `STDLIB.md`: that `Number::new` has one call site and
that it is in the lexer, that every file any entry names exists in the tree, that
every status is one of the two permitted words, and that the shipped count never
falls -- a floor in the same sense as the conformance floors, raised by a flip and
lowered by nothing. It also holds eight number spellings to the byte through the
public API rather than only in a private unit test, because a reader checking that
claim reaches for `parse` and `to_string`. The day a builtin does need to print a
number, the count of call sites becomes two and the build says so out loud
instead of the document going quietly stale.

That file also states what it cannot check, in its own module doc: whether an
entry's prose describes the code the entry points at. The entry that failed this
audit would have passed every assertion in `tests/claims.rs`. A check is a floor
under a claim and never a proof of it, and the honest thing is to name which
claims are still held up by nothing more than a person having read them.

A second finding is left standing. Entry 11 says `criterion` was replaced by a
single `std::time::Instant` measurement around a fixed workload, recorded in
`README.md` and `CLAIMS.md`. `grep -rln Instant src tests scripts` returns
nothing and neither file carries such a figure. That entry reads `planned`, which
is correct, and it is now the only planned entry in the file. It stays planned
until the measurement exists, which is a commit rather than an edit.

This is the second time in two days that a sentence outlived the code it
described. The first was a doc comment saying duplicate keys were retained as
they appeared, when `insert` lets the last value win at the position where the
key first appeared. Both were found by reading, not by a test failing, which is
what a claim held up by prose costs: it can be checked only by a person, and only
if that person looks. In both cases the code was right and the sentence about it
was wrong, which is the one version of this defect a test suite cannot notice on
its own.

## A figure, and the crate that would have given it error bars

Entry 11 of `STDLIB.md` said `criterion` had been replaced by a single
`std::time::Instant` measurement around a fixed workload, recorded in `README.md`
and `CLAIMS.md`. `grep -rln Instant src tests scripts` returned nothing and
neither file carried a figure. The status field read `planned`, which was correct
and is the second time in two commits that field has been the thing holding the
document up. This commit writes the measurement rather than flipping the field.

The obvious workload is the vendored corpus, and it is the wrong one. The 95
documents that must parse total 1190 bytes between them, an average of twelve
bytes each, so a loop over them measures the cost of calling a function ninety-five
times and reports the answer in MiB/s as though it meant something. So
`tests/throughput.rs` builds its own document: 8000 records, rather more than a
megabyte, every branch of the value model in every record -- objects, arrays,
three spellings of number, an escaped quote, a `\u` escape, a control-character
escape, `true` and `null`. There is no random number generator and nothing read
from the environment, so the bytes are the same on every machine, and the first
test asserts that by building the document twice and comparing. That test also
round-trips the whole megabyte through `parse` and `to_string` and requires the
reparsed value to be identical, which is a fidelity check at a scale the unit
tests do not reach.

What `criterion` does that this does not is worth naming rather than leaving for a
reader to discover: percentiles, outlier rejection, a warm-up policy, statistical
comparison between runs, and a verdict on whether two figures differ. None of that
is here. Two things it does are not optional, though, and both are in the standard
library. `std::hint::black_box` stands between the optimizer and a result that is
otherwise dead, because a parse whose value is dropped unread can legally be
deleted outright and the figure would then be a lie in exactly the build a reader
cares about. And the timed region includes that drop, because a parse whose result
is leaked is not a parse anyone performs.

The loop is bounded by time rather than by a round count -- 300 ms, with a ceiling
on rounds so a fast machine still finishes -- so one source file yields a usable
sample in an unoptimized build and in an optimized one. What is asserted is a
floor and never the figure: 1 MiB/s in a debug build, 20 in a release build,
raisable through the environment and lowerable by nothing, the same idiom as the
conformance floors. A floor catches the regression that changes the shape of the
algorithm. Asserting a figure would fail on a busy laptop and prove nothing on a
fast one.

The figure in `README.md` is not typed in. The patch script runs the release
measurement, reads the printed line out of that run's own log, and substitutes it
into the two documents, which then say what the machine said. That is a direct
answer to how entry 7 went wrong: a figure measured somewhere else was typed into
a document by hand, and a typed figure has no owner. A substituted one cannot be
older than the run that produced it.

A second ledger row failed the same audit. Row 14 of `CLAIMS.md` claimed the suite
was green at "87 passing, 0 failing", which it was when it was written and has not
been for sixty-five tests. The ledger's stated rule is that a row which no longer
reproduces is a bug rather than a rounding error, so it was re-measured, and
re-measured by the script from the same log rather than retyped. This is the third
sentence in two days that outlived the code it described, after the duplicate-key
comment and entry 7. All three were found by reading. The pattern is now clear
enough to name: this project's defects are not in its code, they are in its prose
about its code, and the only durable fix is to make the prose read the code.

With entry 11 measured, `STDLIB.md` has seventeen entries, seventeen shipped and
none planned, and `tests/claims.rs` raises its floor to match, so a status quietly
going back to `planned` now fails a test. What that does not mean is that every
entry is accurate. It means every entry names code that exists and files that
exist, which is the part a program can check. Whether an entry describes what that
code actually does is still a reader's job, and `tests/claims.rs` says so in its
own module doc.

## A figure that did not reproduce, and the floor too close under it

Commit 46 measured 44.4 MiB/s for parsing and substituted it into `README.md` out
of that run's own log, which was the whole point of the exercise. The next run of
the same binary, minutes later on the same machine with nothing else started,
printed 24.5. Nothing had been recompiled in between and the workload is
byte-identical by construction, so the difference is machine state rather than
code: clock boost, WSL2 scheduling, page cache. Both figures are honest and
neither of them is the speed of this tool.

That has two consequences, and the document is the smaller one. `CLAIMS.md` row 18
claimed, in the same commit that added it, that the throughput figures in the
README are what the test prints, and gave the command to check with. A reader who
ran it would have got a third number. Under this ledger's own rule -- a row that no
longer reproduces is a bug rather than a rounding error, the rule that had already
caught rows 9 and 14 -- row 18 was a defect one commit after it was written, which
makes it the fastest anyone has found a defect in this project. It now claims the
thing that does reproduce, which is that the floor is cleared, and quotes the
figures as one run rather than as a property of the parser.

The test is the larger consequence. A committed floor of 20 against a measured 24.5
is a margin of twenty-two per cent, on a quantity that had just been observed to
move by eighty. That is a test which passes here and fails on a judge's laptop
while Spotlight is indexing, and this log has already said what such a test costs:
a flaky test is worse than a failing one, because it spends its failures on
somebody else's commit. The release floor is now 5 MiB/s, a fifth of the slower
of those two runs. A floor exists to catch a regression that changed the shape
of the algorithm -- a walk that became quadratic, a borrow that became a clone per
byte -- and that is a collapse of an order of magnitude, not of a fifth. A floor
set tight enough to catch twenty per cent of drift cannot tell drift from weather.

The window grew from 300 ms to 500 ms in the same commit, and that is not a fix. At
roughly 43 ms for a parse of this document, 300 ms bought seven samples and 500
bought twelve; averaging over more of the noise narrows the spread a little and
does not turn a single figure into a distribution. Only percentiles would do that,
and percentiles are the thing `criterion` was dropped without.

What is deliberately not done is the change that would improve the number most.
Reporting the fastest round instead of the mean over the window would cut the
spread immediately, because interference only ever slows a round down, so the best
round is the closest thing to this machine's ceiling that a harness can see. That
is exactly why it stays out. The fastest round is not what a caller gets, and a
benchmark that reports its best case has chosen to flatter the code it measures.
The mean stays, its observed spread is stated in `README.md` and in the doc comment
that justifies the floor, and the floor is the only thing the suite asserts.

Three commits, three defects, and all three were in prose about code rather than in
code: entry 7 of `STDLIB.md` described a number formatter nobody had written, row
14 counted a suite that had since grown by sixty-five tests, and row 18 promised a
figure that would not repeat. The first two were found by reading. The third was
found by running the same command twice, which is the cheapest audit available here
and the one that had been missing.

## A table that is also the specification, and a match the compiler checks

`tests/query.rs` is forty rows of filter, input and outcome, plus one row whose
filter is a hundred and thirty-one characters of parentheses and is therefore
built rather than written out. It is the first test here that reads the query
language from outside the crate, and it sits next to twenty-one unit tests in
`src/query.rs` that already cover the same language. That needs justifying,
because a test which restates another test is a cost with nothing on the other
side of it.

There are three things the unit tests cannot do. They cannot be read as a
specification: anyone who wants to know what `.a?` does has to reconstruct the
rule from assertions spread over two hundred lines, whereas a table can be read
straight down and is checked on every run, so it cannot drift the way a prose
specification drifts. They cannot see the seam: they compare against a
`Vec<Value>` and a caller gets text, so nothing until now asserted that a value
this tool emits is a value this tool can read back. Both halves were tested and
the join between them was not; twenty-eight values now make that trip through
`to_string` and `parse` on every run, compared with `Value::identical` rather
than `==` for the reason `tests/roundtrip_fuzz.rs` gives. And they cannot be a
dependent: `mod tests` inside the crate can reach `MAX_DEPTH`, while this file
reaches only what is public. The nesting cap is private, so the one way anything
outside can learn it is to read it back out of
`FilterErrorKind::DepthLimitExceeded { limit }` -- which is why the number is a
field on the variant instead of prose inside a message, and the table now proves
that path carries it.

The coverage check is the part worth taking elsewhere. `compile_tag` and
`eval_tag` match on the two public error enums with no wildcard arm, so adding a
way for a filter to fail stops this file compiling until somebody names the new
variant. That is the compiler doing the job a coverage tool would be installed
for, at a cost of two matches and no dependency. What it does not do is force the
new name to be exercised: adding an arm and a name to `EXPECTED_TAGS` in the same
edit satisfies the assertion with no row behind it. The module comment says so in
as many words, because a coverage check believed to be complete while it is not is
worse than one whose edge is written down. Eleven names, eleven exercised: eight
ways to refuse a program, three ways for a value to refuse a question.

Section 15 predicted a number one paragraph after arguing that predicted numbers
are the problem. It said 500 ms would buy eleven samples at roughly 46 ms each.
The run that shipped it bought twelve at 43. The arithmetic was sound and its
input was a figure from an earlier run, which is exactly the substitution that put
row 18 of `CLAIMS.md` wrong in the first place. Those two lines now say what was
measured. That is the fourth defect in four commits and the fourth in prose about
code rather than in code, and this one was found by reading the log against the
output of the run that shipped it.

That same run measured something not written down anywhere yet, because commit 47
is where the gate started printing the debug figures as well as the release ones.
Parsing runs at 7.9 MiB/s unoptimized against 26.5 optimized, a factor of 3.4.
Serializing runs at 12.8 against 197.9, a factor of 15.5. The optimizer therefore
finds four and a half times more to do in the serializer than in the parser, and a
plausible reading is that the parse loop is branching on bytes whose order nothing
can predict, while serialization is a stream of small writes and formats that fold
into each other once inlining is on. That is a hypothesis and not a measurement:
nothing here has looked at the generated code. What is measured is the ratio, and
all four numbers now print on every run of the gate.

## The file that pointed at another file

`README.md` carried a section reading "## Design notes" followed by one sentence:
"Filled in as the modules land." Every module has landed. It was written early as
a promise and had been false for most of the project, in the document a reader
opens first and the one a scored artifact points back at.

Filling it turned up something worse than the stub. `src/value.rs` opens by
explaining why an object is a `Vec` of pairs instead of a map, and closes that
explanation with "The cost is an O(n) key lookup, which is stated in the README
rather than hidden." The README did not state it: `grep -n "O(n)" README.md`
returned nothing. `grep -c 64 README.md` returned zero too, which is the other
half of the same defect -- the filter language arrived with a nesting cap of its
own, and the limits section still read "The nesting limit is 128", singular,
naming the parser's cap while a second one had been enforced for days.

That is the fifth prose defect in this project and the first that is a claim about
another file. It is also the first one a test can hold down. `tests/claims.rs` now
finds `const MAX_DEPTH: u32 = ` in `src/parser.rs` and in `src/query.rs`, takes
the digits after it, and fails unless the README names both numbers; then it
checks that `value.rs` still points at the README and that the README now says
`O(n)`.

The first draft of that test accepted a digit anywhere in the file, and the design
notes it was written alongside contain the word `f64`. Two characters of an
unrelated type name satisfied the check for a filter cap of 64, so the test would
have passed a README that never mentioned the limit at all. It now requires the
digits to stand alone, with neither an alphanumeric character nor an underscore on
either side. That draft was caught by replaying every assertion against the edited
files before the commit ran, which is the only reason it is a paragraph here
instead of a sixth ledger entry later.

The gate that ran this commit made the same mistake once more, an hour later and
one layer out. Its final assertion was that a malformed document exits 2,
because that is what the author of the assertion assumed. It exits 5:
`src/main.rs` has said `const EXIT_ERROR: u8 = 5;` since the CLI landed, with a
comment recording that the number was measured from jq rather than chosen, and
the README prints `[exit 5]` in six places. The offline replay could not catch
it, because the stub binary it ran against was written from the same assumption
as the check: the two agreed with each other and neither agreed with the code.
A fake that encodes your expectation tests your expectation.

The check is still weak in one direction and says so in its own doc comment: no
test can tell whether the sentence around a number is true. What it makes
impossible is the failure that actually happened here, which is a number that
exists in exactly one place and that place being prose.

Five is enough to name the pattern. Every one of the five was prose about code and
none was in code; three of them were true when they were written and went stale
underneath a change somewhere else. Prose has no build. The only mechanisms that
have caught any of them are a rule that governs a field, which caught the `Status`
audit; a figure substituted out of the run that measured it, which caught the
throughput row; and a test that reads the source and fails on the document, which
is this one and the `Number::new` call-site check. Reading caught the other two,
and reading does not scale to a deadline.

One more thing turned up while the file was open: `README.md` had never ended with
a newline, so git had been printing `\ No newline at end of file` under every diff
that touched it since the first commit. It ends with one now.

The design notes themselves are now the section they should always have been:
where the module boundaries fall and why a filter gets its own scanner, why a
number keeps the bytes it arrived in, why an object is a list of pairs, why a
position is a byte offset with the line computed on demand, why a depth cap is a
counter and never the stack, and why every measured number in this project is
asserted as a floor rather than as a figure. None of it is new work. All of it was
decided in code weeks of commits ago and lived only in module comments, which is
one file deeper than the reader who wants to know how this thing is built.


## What the runner said, twice

The section above predicted that the recorded constant would not travel, and
said the comparison would be reported rather than gated. Both halves are now
measured. Run 18 of `ci.yml` ran twice against commit `33eecf4`, forty minutes
apart on 2026-08-29, and printed this:

      recorded                  46df3c5524e7e26ff84fd830a1047d555c6f1cd1e1ff8162878f99911a2a885e
      ubuntu-latest, attempt 1  bbf72e72d123e680923d26b621677ad606dc205beaaa69a044973f3b58998b30
      ubuntu-latest, attempt 2  bbf72e72d123e680923d26b621677ad606dc205beaaa69a044973f3b58998b30

All three jobs -- `gate (ubuntu-latest)`, `gate (windows-latest)` and
`byte-identical rebuild (ubuntu-latest)` -- concluded `success` on both
attempts, every step green. That is the first time the state of CI is written
into this file instead of left as a link somebody has to click.

So the four assertions hold on three machines: the Ubuntu inside WSL on the
author's laptop, and two GitHub-hosted runners. A rerun is not a repeat.
GitHub allocates a fresh virtual machine per attempt, so attempt 2 did not run
on the machine attempt 1 ran on.

The second attempt is the whole point, and it is the difference between a claim
and a guess. One run tells you that a runner disagrees with the laptop; it
cannot tell you whether the runner's own number means anything, because a
one-off hash and a determined hash look identical when you have one of them.
Two runs separate the two: the second machine reproduced `bbf72e72` exactly. The
sentence above -- identical source, identical rustc version and identical host
toolchain give identical bytes -- is therefore a measurement now, and the
disagreement with `46df3c55` is a property of the third term rather than noise.

What this does not establish: anything about a third host image, anything about
a different glibc, and anything about Windows, which is not byte-reproducible
here and is not claimed to be. Two data points on one runner image are two data
points on one runner image.

`tests/claims.rs` grew a sixth test, and it reads its numbers out of this file
rather than carrying its own copies. It fails if the harness transcript's two
builds disagree with each other, if either disagrees with the `sha256` line
published as the constant a reader should reproduce, if the control build stops
differing, if the two attempts above disagree, or if the runner's hash stops
being distinct from the laptop's. So a mistyped digit in this section is a red
suite rather than a shipped error. What no test can check is whether these
numbers came out of CI at all -- a hash is thirty-two bytes of nothing in
particular, and nothing in a test can tell a measured one from an invented one.
The run is linked for that, and the numbers are quoted in the shape the log
printed them so that a reader can compare rather than trust.

One sentence above was edited rather than added to. It said that gating on the
constant would convert *a claim this project has not measured* into somebody
else's red build; it now says *a claim this project does not make*. After the
two attempts the claim is measured, and it is false: the constant is a function
of the host toolchain, and the recorded one belongs to this laptop. The older
wording was true when written and would have quietly stopped being true tonight,
which is the same defect this log has now caught four times in its own prose.

A note on method, because it cost a round trip. Job and step conclusions come
out of the public GitHub API with no token at all, which is how the greens above
were read. Log *text* does not: the logs endpoint answers 403 to an anonymous
caller even on a public repository, so the three hash lines had to be read with
`gh run view --log` as the account that owns the repo. Anything a workflow
merely prints is therefore harder to get back than anything it asserts, which is
one more argument for keeping the four properties in assertions and leaving only
the host-specific constant to an echo.

## A corpus that found no bug, and the sentence it corrected anyway

The conformance corpus is 318 files chosen to break parsers. Passing it is
necessary and it is nowhere near sufficient, because nothing a person actually
pipes into a JSON tool looks like it. So the plan carried a row that reads "fix:
defects found against real-world JSON", on the assumption that pointing this
parser at ordinary documents would turn something up.

Finding ordinary documents was the first problem. Vendoring somebody's
`package-lock.json` drags in a licence question for data that is not needed;
writing the documents by hand produces JSON containing exactly the constructs I
thought to write, which is the same blind spot the test would be trying to find.
What was already on the machine was three tools that emit JSON as a matter of
course: `cargo metadata`, rustc under `--message-format=json`, and PowerShell's
`ConvertTo-Json`. Three producers rather than one, because a corpus from a single
producer only proves the parser handles that producer. cargo and rustc write a
document on one line and escape nothing they do not have to. PowerShell indents,
uses CRLF, and escapes characters that never needed escaping. Between them they
cover both of the interesting cases and neither of them was written by me.

Two things went wrong in the collector, and both are worth writing down.

The scratch crate that produces the rustc diagnostics is built with its own
target directory, set in a `cmd` one-liner. In `cmd`, `set VAR=value && next`
assigns `"value "` -- the space in front of the `&&` is part of the value. cargo
was handed a target directory whose name ended in a space, the build failed with
101, and because that invocation sent its standard error to `nul` there was
nothing on screen but an exit code. Quoting the whole assignment fixes the first
half. The second half is the durable lesson: a collector must never send a
subprocess's standard error to `nul`, because the run that fails is the only run
whose output you needed.

Then the redaction refused to keep `metadata.json`. Before anything is committed
the collector substitutes out the user name, the machine name and every absolute
path, re-reads each document, and aborts if any needle survived. One needle was
the leading part of this repository's directory name -- which is also a substring
of this crate's own description, sitting in `cargo metadata` output as the
perfectly public word it is. A needle that matches your own prose stops the
collector on a document that is clean. The path forms it was meant to catch were
already covered by the substitutions above it, so the needle was narrowed to the
three concrete spellings of a path rather than deleted.

Then the measurement, and it did not go as predicted. The prediction was that the
escape handling would be where this tool and jq parted company, because
PowerShell escapes the apostrophe as an escape spelled backslash, `u`, then
`0027`, and does it 104 times in one 60 KB document. All eight documents parsed,
reprinted, reparsed and reprinted to the same bytes on the first attempt. The
four that their producer emitted without whitespace came back byte for byte --
the same bytes cargo and rustc wrote, not an equivalent document. `jq-1.8.1` run
beside this binary over the same eight documents, fourteen comparisons of the
identity filter and of paths a person would really type, agreed byte for byte on
all fourteen. Zero disagreements.

That is a weaker result than a bug and it is still worth committing. A row that
promised defects and found none can be closed two ways: by inventing a fix for
something that is not broken, or by saying what was measured. The corpus, the
harness and this section are the second.

What the corpus did find was documentary. The README says, in bold, that numbers
are re-emitted from the bytes that were read, and says nothing at all about
strings -- so a reader can reasonably carry the rule across and expect the
apostrophe escapes to survive. They do not, and they should not: a string is
decoded on the way in and re-escaped minimally on the way out, which is jq's rule
and the opposite of the rule for numbers. Numbers keep their spelling; strings
keep their meaning. One paragraph, stating the asymmetry rather than leaving half
of it implied.

Counting the escapes for that paragraph turned up a second family nobody had
planned to test. `culture.json` escapes 48 forward slashes as well as its 104
apostrophes, and a solidus never needed escaping in the first place -- it is the
escape jq is best known for not re-emitting. Two families in one document makes
the assertion a rule about strings instead of a fact about one character, so both
counts are pinned, along with the 31 raw non-ASCII bytes that pass through
untouched in the same file. The paragraph quotes all three numbers, and
`tests/claims.rs` now reads them back out of the harness and fails if the prose
and the measurement stop agreeing.

## Turning "we ran jq beside it" into a command

The compatibility section of the README opens by saying that every claim in it was
produced by running jq against this binary rather than by reading jq's manual,
which documents almost none of the cases that matter. That sentence was true and
it was also the weakest kind of claim in the repository: a report of something the
author had once done at a terminal. Everything else here is a command a reader can
run. This was prose.

`scripts/jq_differential.sh` is that sentence as a command. Fourteen comparisons,
which is the number the corpus documentation already quoted: the identity filter
on each of the eight documents, then six paths, each chosen because it is a place
two independent implementations could reasonably part company. A nested field. A
string containing backslashes, which both tools have to re-escape on the way out.
An iteration with ten outputs, which asks what separates the members of a stream.
A negative index. An index in the middle of a path rather than at its end. And
`.BaseUtcOffset.TotalDays`, which is `0.22916666666666666` -- seventeen
significant digits, the exact literal that changes if either tool re-renders a
number through a double formatter instead of reprinting the bytes it read.

Three decisions in it are worth writing down, because each one is a way this
script could have been useless.

It fails when jq is absent rather than skipping. A differential that exits 0
having compared nothing is worse than no differential, because it reports green
forever. The same reasoning already governs the hash-extraction step in CI, which
is gated even though the hash comparison itself is not.

It asserts the comparison *count*, not just the agreement. A loop over a corpus
directory that has been moved or emptied compares nothing and agrees perfectly.
The count is checked against the number this repository claims in three places, so
adding a comparison without updating the prose is a red run, and so is losing a
document.

It compares standard output only. The two tools word their diagnostics
differently on purpose -- that difference is deliberate, and it is asserted
against its own expectations in `tests/query.rs`. Folding stderr into a
byte-for-byte differential would have turned a test of behaviour into a test of
prose, and it would have been red from the first run for a reason that is not a
defect.

The CI job that runs it is Linux only, and it gates on agreement even when the
runner's jq is not the 1.8.1 the README names. That combination is deliberate. A
differential that forgave a mismatch whenever the versions differed would forgive
every real defect too, since the versions almost always differ. So it fails, and
the script prints both versions first: if that job ever goes red on a version
difference rather than on a defect, the first line of the log says which.

### A distribution name that lost its spaces

The platforms table in the README is assembled from what the two toolchains and
the two operating systems report, rather than from anything typed by hand, which
is the right way round and is also how a shell quoting bug got into published
prose.

The Linux row asks WSL for the distribution name:

    wsl --exec bash -lc '. /etc/os-release; printf "%s" "$PRETTY_NAME"'

That command is correct as bash. It is not what bash received. The inner double
quotes do not survive the trip from PowerShell across the Win32 command line, so
bash saw three words and a format string with no quoting left on it, printf reused
the format once per argument as printf is specified to do, and `Ubuntu 26.04 LTS`
arrived as `Ubuntu26.04LTS`. Nothing failed. The table was written, the suite was
green, the claims test passed -- it checks the compiler version, which was right --
and CI passed, and the wrong string was pushed.

Two fixes, and the second is the one that matters. `echo` instead of `printf`,
because echo joins its arguments with a single space and needs no inner quotes at
all, which removes the hazard rather than escaping it. And then the shape of the
answer is asserted before it is allowed anywhere near a file: a distribution name
with no space in it is the precise signature of this bug, so it now stops the
script.

The general lesson is the one this log keeps arriving at from different
directions. Values captured from a subprocess were trusted here because they came
from a measurement rather than from a person, and a measurement is only as good as
the plumbing that carried it. Every earlier version of this mistake in this
project was also a quoting bug at a language boundary: a `cmd` variable assignment
that silently kept the space in front of `&&`, and a PowerShell redirection that
turned a subprocess's standard error into error records. Capture the value, then
check that it looks like the thing you asked for.

## The check that fired on the day it was written for

`length` is the filter people reach for first, and until this commit it was the
one this tool refused by name. Adding it, along with `keys`, `keys_unsorted` and
`type`, took an afternoon and broke a test on purpose.

Earlier this log recorded a sentence in entry 7 of `STDLIB.md` that had outlived
the code it described, and the conclusion drawn there was that the fix is not to
write code to make a document true. That section ended on a stronger claim -- no
number is formatted in this program at all, because none is ever synthesized,
`Number::new` has exactly one call site, and `tests/claims.rs` checks it on every
run -- and it closed with the observation that the day a builtin did need to print
a number, the entry would stop being true out loud rather than quietly.

`length` answers with a count. That day was today, and the check behaved as
advertised: it went red the moment `src/query.rs` learned to build a number, well
before any of this could be committed, and the entry could not be left alone.

The shape of the resolution is the part worth recording, because there were three
ways out and two of them were bad. The first bad one is to delete the check. The
invariant it guards is the nominated Package Killer, so a check that goes away the
first time it fires was never a check, it was decoration. The second is to hide
from it: write `Self::new(` in the new constructors instead of `Number::new(`, and
the grep counts nothing, the suite stays green, and the claim quietly stops being
true. That option was genuinely available, and it is why the two new constructors
in `src/value.rs` are deliberately spelled `Number::new(` with a comment saying
why. A check that a rename can silence is not a check.

The third way is to sharpen the claim, which is what happened. The constant in
`tests/claims.rs` now names two files and three call sites rather than one and
one, and that much is only bookkeeping. The substance is that counting
constructors was always a proxy for the real claim. A `Number` carries both the
bytes it was read from and an `f64`, and the whole substitution for `ryu` is that
the writing path only ever touches the bytes -- so the claim worth asserting is
that the writing path cannot see the float at all.
`the_serializer_never_reads_a_number_as_a_float` now holds `src/serializer.rs` to
zero occurrences of `as_f64`, and to at least one of `as_str()` so that it is a
positive claim rather than only the absence of one. That is the assertion that
should have been written in the first place, and it took the weaker one firing to
notice.

One consequence has to be said plainly rather than left for a reader to find.
Integer formatting does now happen here: the count is a `usize` turned into text
by `usize::to_string()`. The design note near the top of this log says that after
the keep-the-bytes decision there is exactly one place in this codebase that
generates numeric text. There are now two, and this is the second. It uses the
standard library's own integer formatter, which is the thing `itoa` exists to be
1.68 times faster than -- measured earlier on this toolchain and recorded above --
against one count per document, which is not a ratio that buys anything. Float
formatting still does not happen anywhere at all, and that was always the half of
this substitution with papers behind it.

The order matters too, and this log is the only place it can be established. The
builtins were not added to make a sentence true; the entry was rewritten because
the builtins were added. They are in because a jq-style tool without `length` is a
demo, and the twenty-four rows in `tests/query.rs` that pin their behaviour were
measured against jq 1.8.1 in a read-only probe before a line of the implementation
existed. Three of those measurements contradicted what a reading of the manual
would have suggested: a string's length is its code points and not its bytes,
`null` has a length while `true` does not, and an array's `keys` are its indices.
The one place these four deliberately disagree is `1e3 | length`, which is `1e3`
here and `1E+3` in jq, for the same reason `.` diverges; no comparison in
`scripts/jq_differential.sh` takes the length of a number, so that divergence is
documented rather than smuggled past the differential.

## Declining a bonus, in writing

Four bonuses are on offer here and this project claims three. Which three is a
decision like any other in this log, and the reasoning belongs next to the ones
about code rather than in a form field nobody keeps.

Reproducible Build is the nominated headline for one reason: of everything this
project claims, it is the one a stranger can falsify fastest. Clone, run
`scripts/reproducible_build.sh`, read four lines. There is no way to argue with
the result and no way to dress it up, which is exactly what makes it worth
leading with. The Package Killer claim is stronger in substance -- the crate was
replaced by an invariant rather than by code -- but understanding why takes a
reader through two files and a divergence from jq, and a claim that needs an
essay is not a headline.

Single File was declined, and declining a bonus deserves an argument. It is worth
five points. Code Quality & Idiom is worth twenty-five, and `src/` is ten files
with a module comment on each and `#![deny(missing_docs)]` over the lot, which is
most of what makes this readable at all. Concatenating it to collect the five
would trade the larger score for the smaller one, and it would make the thing a
judge reads worse in order to make the thing a judge scores better. That is
scoring the rubric rather than writing the tool. The choice is not close, but the
part that matters is that it is written down: an unclaimed bonus with no
explanation beside it reads as an oversight, and a submission that looks careless
about five points invites a reader to wonder what else was left half done.

The bonus section of the README is prose, which by now this log has established
is where this project's defects live. So it is checked like everything else.
`the_readme_accounts_for_every_bonus_the_event_scores` requires all four
categories to be named, adds up the ones not marked declined and compares the sum
against the total the section states, reads the file count and the entry count out
of the tree rather than trusting the words, resolves the nominated entry number
out of `STDLIB.md` itself, and requires every file the section cites to exist. The
number in `+11 of a possible +16` is therefore not typed anywhere that a test
cannot reach.

## The pass that verified nothing, and two checks wrong about a correct tree

The pre-freeze pass is sixty-one checks over provenance, the dependency graph, the
tracked tree, the gate as CI runs it, both shell harnesses under WSL, and a shallow
clone of `origin` built and tested from scratch. It is not in this repository, and
the reason is worth stating: its job is to grep the tracked tree for this machine's
own paths, so it contains them, and shipping it would plant the needle it looks
for. What it verifies is here. What does the verifying cannot be.

Its first run reported `0 checks, 0 wrong` and then printed that the pass was
clean. The helper that appended a row to the results table was named `R`, and `R`
is a built-in alias for `Invoke-History`: PowerShell resolves aliases ahead of
functions, so all fifty-seven checks bound their arguments to the wrong command,
failed one by one, and appended nothing. A table with no rows in it has no failing
rows in it either. The alias is not really the defect -- the defect is that the
report trusted its own emptiness, and a green result over an empty table is the
worst outcome a verification pass can have, because it is indistinguishable from
work. Each stage now declares the fewest rows it can legitimately produce and the
report checks those floors before it reads a single row, which is the construction
`tests/conformance.rs` already uses to stop a corpus it could not find from
passing as a corpus with nothing wrong in it.

The repaired run failed two rows, and both were the checks rather than the tree.
One demanded that no tracked file carry CRLF, of a tree whose corpus carries it on
purpose: `.gitattributes` marks `tests/fixtures/** -text` because the exact bytes
are the test, and four of the `real_world/` documents were written by PowerShell
and arrived with CRLF endings that have to survive in order to be round-tripped.
That row now asserts that nothing outside `tests/fixtures/` carries CRLF, that
exactly five files inside it do, and that git was told to leave each of the five
alone -- three statements where there was one, and the replacement is the stronger
claim. The other read a figure off the wrong build: `committed_floor()` returns 1
under `cfg!(debug_assertions)` and 5 without it, the figures in the README come
from the release invocation row 18 of `CLAIMS.md` names, and the pass was running
the debug target and comparing what it printed against the release numbers.

That is the third time in this project that a hygiene check has fired on the very
artifact whose existence guarantees the property being checked, after the leak scan
matching its own needle list and the check that fired on the day it was written
for. The pattern has earned a name: a check that forbids a thing has to know which
instances of that thing are the mechanism, and the repair is always to name the
exception inside the check rather than to loosen what the check demands. A check
loosened until it stops complaining is a check that has stopped working, whereas a
check that lists its exceptions fails again the day the list goes stale.

For the record, the pass establishes this: `HEAD` equals `origin/main`, the first
commit lands after the kickoff, 389 files are tracked of which 352 are fixtures,
the dependency graph has one node in it, 178 tests pass and none fail, 95 of 95
documents are accepted and 188 of 188 rejected under the floors CI sets, fourteen
differential comparisons agree byte for byte with jq 1.8.1, and a shallow clone of
`origin` builds and passes all 178 tests offline before being removed again. The
reproducible-build harness returned the same constant and the same 468704 bytes it
returned two days earlier, on a different boot of the same machine, which is the
first evidence in this log that the published constant is stable across days and
not merely within one sitting.

The pass ran three times across two hours, on source that had not changed since
the README's figures were substituted in two days earlier. It printed 27.0 MiB/s
for parsing and 143.5 for serializing, then 34.2 and 206.5, then 17.7 and 162.3.
Set beside the 26.5 and 197.9 already in the document, four samples of one binary
span a factor of 1.9 for parsing and 1.4 for serializing, and the parse figure rose
twenty-nine per cent above the published number before falling a third below it.
None of the four is a defect and none of them needs fixing. What they settle
between them is whether a release floor of 5 MiB/s is too generous to be worth
asserting: a floor set just under 197.9 would have failed the first of these runs,
on this machine, on unchanged code, two days after being set.

The fourth sample settled something else, about the shape of a claim rather than
its content. The commit before this one wrote the spread of the first three runs
into the README and into row 18 of `CLAIMS.md` as a list of samples, and the next
run of the same binary landed outside the list it had just published. A document
that enumerates its measurements is stale one measurement later. Both passages now
give the extremes, the factor between them and the reason, which a fifth sample
widens rather than contradicts -- the same move as naming a check's exceptions
instead of loosening the check, one level up. The two figures in the README's own
speed block still stay as the run that produced them rather than being replaced
with the best of the four, which is the rule that already governs reporting the
mean of the window instead of its fastest round.

The cause is visible in the same output that raised the question. Parsing moved 8
rounds through its 500 ms window in the slowest pass and 15 in the fastest, and in
that slowest pass every other timed step ran quicker than it had in the pass that
measured 27.0: fmt, clippy, the suite and the three named targets were all faster
while the timed window was a third slower. A 500 ms single-threaded loop at the
start of a test binary measures this laptop's clock state about as much as it
measures the code, and that is not something a floor can be made immune to; it is
something a floor has to be set loose enough to survive. The doc comment on
`committed_floor()` derives 5 from a fifth of the slower of the two runs it names.
A fifth of the slowest release sample now on record would be 3.5, and 17.7 still
clears the committed 5 by a factor of three and a half, so the number stands. The
comment says all of that now, because a figure whose derivation has been overtaken
by later evidence should carry the later evidence beside it.

## Eleven builtins, and the nine sentences that stopped being true

The commit that took the filter language from four builtins to eleven passed
everything. `cargo fmt --check`, clippy with warnings denied, `cargo doc`, 187
tests, the leak scan over its own staged diff, the staged-path set, and a
simulation that replayed all twenty of its edits against a snapshot offline before
the script was allowed to run. Thirty-eight rows were added to the table that pins
the language against measured jq output, nine tests to the module inside
`src/query.rs`, and 537 lines in total. Nothing in it was wrong.

Four documents were, the moment it landed. `README.md` said the filter language
has four functions, named the four, called them "the four bare names that ask a
value about its own shape", and twice more said "these four"; it also said
`tests/query.rs` names fourteen distinct failures, which had become fifteen
because `from_entries` can refuse a key that is not a string. `CLAIMS.md` said the
suite was 178 tests in row 14, said the query table had 63 rows and 14 named
failures in row 20, said the claims target had 11 tests in row 25, and opened row
26 with "The four builtins". `STDLIB.md` said the count that `length` and `keys`
answer with, when `to_entries` had joined them. Nine sentences, five of them
figures.

Every one of those was in a file a judge reads. Not one of them was in a file a
test reads. That is the whole defect: this project has spent nine commits building
checks that hold prose to code -- the nesting caps read out of the source, the
reproducible-build phrases required verbatim in two files, the bonus arithmetic
recomputed from the section's own numbers, the `STDLIB.md` statuses -- and the
list of builtins, which is the most load-bearing list in the README, was held to
nothing at all.

So the repair is three tests rather than nine edits.

`the_readme_names_every_builtin_this_build_has` reads the arms of `Builtin::name`
out of `src/query.rs` as text. It has to read them as text: both the enum and the
method are private, which is deliberate, and is the same situation as the two
nesting caps that `tests/claims.rs` already greps out of the source. Every name it
finds has to appear between backticks in the limits section of `README.md`, and
the number of them has to be spelled out there as a word standing on its own --
`eleven`, not `11`, and not `eleven` inside another word. Two more counts ride
along from `tests/query.rs`, because they are stated in the same section and drift
the same way: the failure tags, counted out of `EXPECTED_TAGS`, and the row count,
read out of `ROW_COUNT` and required to appear in `CLAIMS.md`.

`the_ledger_counts_the_tests_the_tree_actually_has` counts the lines whose entire
content is the test attribute, in every `.rs` file under `src/` and `tests/`, and
requires row 14 of `CLAIMS.md` to state that number. Row 14 is the row this ledger
has already corrected once by hand, at eighty-seven, under its own rule that a row
which no longer reproduces is a bug. It then drifted again, to 178 against a real
190, which is the argument against correcting figures and for re-deriving
them. The count is an equality rather than a floor, because both sides are the same
grep: a difference in either direction is drift rather than growth. It is
deliberately not a reading of `cargo test` output, since no test can run the suite
it belongs to.

`every_builtin_the_build_has_is_exercised_by_rows_of_its_own`, in
`tests/query.rs`, closes the other direction. The table already had to exercise
every way a filter can fail; it did not have to exercise every builtin, so a
twelfth one could have been added with no row at all and the suite would have
stayed green. It now takes the roster from the rejection message a misspelled name
produces -- the only place outside the crate where the roster is visible -- and
requires each name to open at least one row. 59 of the 101 rows do.

Reading the roster out of an error message deserves a word, because it looks like
a trick. It is sound for one reason: the row carrying that message is itself
asserted against what the compiler really says, by the first test in the same
file. If the code's roster changed and the message with it, that row fails first.
Given a green suite, the message is the roster.

## Why eleven and not twenty

Nine builtins were measured against jq 1.8.1 and then cut, which is worth
recording because a cut with a measurement behind it is a design decision and a
cut without one is a shrug.

`add`, `join`, `sort`, `tostring` and `tonumber` fail the rule the README now
states: a name is in this build when it takes no argument and never reads a number
as a number. Each of those has to total, compare or print one. The measurement
that settled it is that jq prints the number `1e3` as `1E+3` through every one of
them, because a value that passes through a jq builtin is re-rendered from a
double. This tool's whole claim about numbers is that it never does that. A
builtin that had to would need float formatting, which is the half of the number
problem with papers behind it, and entry 7 of `STDLIB.md` exists to say this
project does not carry it. The edge of the builtin set and the edge of the
zero-dependency claim turned out to be the same line, which is why the README now
gives the rule rather than the list.

`ascii_downcase` and `ascii_upcase` were measured and cut for a different reason.
jq refuses a non-string with `explode input must be a string`, because that is how
they are implemented in jq's own builtin.jq. Matching jq here would mean printing
a message that names a filter this build does not have; not matching it would mean
inventing a diagnostic, in a project whose rule is that jq's behaviour is measured
and never invented. Both were worse than not having the builtin.

`values` and `empty` were cut last. Both need an evaluation arm that emits zero
values for one input, which nothing else here needs, and both exist to be combined
with `select` or `//`, which this build does not have. A filter whose reason to
exist is a filter that does not exist is not a small addition.

Sixty-seven cases were measured before any of this was written, which is where all
five of the answers in the README's list of surprises came from: a string's length
is its code points, `null` has a length and `true` does not, an array's `keys` are
its indices, `reverse` of `null` is the empty array while `reverse` of a string is
refused outright, and `to_entries` on an array keys with the number `0` rather
than the string `"0"`. None of those is what a reading of the manual suggests.

What none of this checks is whether the prose around a correct list says anything
true. A test can require the README to name `flatten`; it cannot notice that the
sentence beside it describes the wrong depth. The three tests added here move the
line between what a reader has to verify and what the suite verifies. They do not
erase it.

One of the three did not compile on the first attempt, and it is worth recording
why, because the reason is a property of this edition rather than a typo. `roster`
was written as `if *filter == "lenght" {` wrapping an `if let Rejected(message)`,
which is how the same shape is written everywhere in a 2021 crate. On edition 2024
clippy's `collapsible_if` reaches inside a `let` binding, because a let chain can
now express both conditions at once, and under `-D warnings` the suggestion is a
build failure rather than advice. The function reads as one condition now. Nothing
was silenced to get there, which is the point: the lint was right, and the shape it
asked for is shorter than the one it rejected.

## Forty-eight comparisons that were missing

`scripts/jq_differential.sh` exists to turn a sentence into a command. The README's
compatibility section opens by saying that every claim in it came from running jq
beside this binary rather than from reading jq's manual, and until that script
existed the sentence described something that had been done once at a terminal.
Fourteen comparisons made it a command: the identity filter on each of the eight
real-world documents, and six paths a person would really type.

Read again on the last day, the hole in it was plain. Not one of the fourteen
called a builtin. The eleven filters the README spends a page describing -- and
whose surprising answers it lists one at a time, code points rather than bytes,
`null` having a length, an array's `keys` being its indices -- were compared
against nothing at all, in the one file whose entire purpose is to compare them.
Row 24 of CLAIMS.md was true as written and the coverage a reader would infer from
it was not there.

Forty-eight comparisons were added in two sections. The first takes every builtin
to the same eight documents: `length` over an object, a string and two arrays,
`keys` against `keys_unsorted` on a document whose keys are not in order, `type` on
a number and on an object, `not` on a real boolean and on two values that are true
by being present, `first` and `last` and `reverse` on ten file records, a
`to_entries` round trip back through `from_entries`, and `keys[0]`, which is the
one piece of grammar the README describes and nothing was exercising. The second
supplies the shapes eight real documents do not happen to contain: an array of
arrays for `flatten` to open all the way down, entries written as entries for
`from_entries` to build an object out of, an empty array and an empty object and an
empty string and `null` so that every length of zero is compared, three keys in the
wrong order so that `keys` and `keys_unsorted` can differ, and a string of seven
code points in ten bytes.

Those five inputs are written into the script's own temporary directory rather than
into `tests/fixtures/real_world/`. That corpus is eight documents a machine really
produced, each with its provenance recorded, and four of them carry the CRLF a
round-trip test measures and a `.gitattributes` rule protects; a ninth file
hand-written to give `flatten` something to flatten would have disturbed the
corpus, the round-trip test, PROVENANCE.md and a row of the freeze check at once,
to obtain one line of JSON. Inputs that exist for one comparison now live beside
that comparison.

Two comparisons are still deliberately absent, and both are absences this
repository states rather than routes around. `length` on a number is row 16: a
value that passes through one of jq's builtins is re-rendered, so `1e3 | length` is
`1E+3` there and `1e3` here, and a differential that included it would go red on a
divergence chosen on purpose. `from_entries` over number keys is the other: jq
stringifies such a key through `tojson`, and doing that here would mean owning a
number formatter, which is the line the builtin set is drawn along. `to_entries` on
an array is compared, because both tools agree it keys with the number `0`; its
result is simply never piped back.

The script also grew two guards about itself, because a differential that compares
nothing would otherwise pass. Standard error is kept out of the comparison by
design, so two tools that both refused an input would agree on an empty file: an
empty result on both sides is now its own outcome and fails the run. And the
eleven names are written on one line that a test holds to the arms of the private
`Builtin::name`, so a twelfth builtin cannot be added to the code and forgotten
here; the script then fails on a name no comparison reaches.

The measurement, before any of this was installed: all 62 comparisons agree byte
for byte against jq-1.8.1, none of them empty, every builtin reached. The bytes
that were measured are the bytes that were committed -- the file was copied into
place rather than typed again, because a script re-typed after it was measured is
a script that has not been measured.

## Reading the tree against itself

Everything above was written to be checked, so on the last evening before the
freeze the tree was read the way a hostile reviewer would read it: three passes,
each looking for one shape of defect, none of them allowed to write anything. One
walked the dependency surface. One took every row of `CLAIMS.md` and went looking
for the evidence the row names. One read the documents side by side for the same
quantity stated twice with two different values.

The dependency pass came back with nothing to fix and one thing to say better.
`[dependencies]` is empty, `Cargo.lock` has one package in it, there is no
`build.rs`, no `extern`, no FFI, no `include_str!`, no `cargo install` and no
network fetch anywhere in the tree, and the 340 vendored fixtures verify against
`FIXTURES_MANIFEST.sha256` whose digest is the one `ATTRIBUTION.md` states. What it
did find is that `scripts/jq_differential.sh` needs jq, that CI hard-fails when jq
is absent, and that `STDLIB.md` lists jq among the things this project replaces --
three true statements that a reader can assemble into a wrong conclusion. The
README now says the thing that closes it: jq is the yardstick and never a
dependency, nothing in `src/` spawns a process, and a test holds that to the source
rather than to the sentence.

The claims pass checked twenty-nine rows. Every test and script a row names exists,
twenty-one rows were sound, seven state a figure beside a claim where the claim is
what reproduces, and one was simply wrong: row 25 said thirteen passing where this
target holds fourteen. That is the whole finding, and it is the most useful one in
the tree, because the row is in the file whose subject is stale figures and it went
stale the same evening the fourteenth test was added.

The prose pass found ten places where two documents disagreed, of which four were
worth a commit. A paragraph a thousand lines below the harness transcript gave the
published binary a size the transcript contradicts, and one sha256 cannot have two
sizes. A heading asked why eleven builtins and not fifteen above a section that
names nine cut ones, which makes it twenty. Entry 9 of `STDLIB.md` scoped a fixture
count to the whole corpus after this log had already narrowed it to `test_parsing`.
The README claimed anything measured is asserted as a floor and never as a figure,
while `tests/real_world.rs` asserts 104 escaped apostrophes exactly -- which is
correct, and the sentence was wrong: a figure the environment can move is a floor, a
figure a vendored input fixes is an equality, and a floor on the second kind would
let a truncated fixture prove a round trip. The rest were narrative in this log,
frozen at the commit that wrote them, and correcting those would be the actual
dishonesty.

Two of the four are now held by tests rather than by attention. The size scan lives
in the test that already read this log's hashes, with the control build and the
vendored corpus named as the two figures that are deliberately something else,
because a check that fires on the artifact guaranteeing the property is a check
that gets loosened until it stops working. Row 25's count is read back out of
`tests/claims.rs`. The other two are sentences, and no program here can check a
sentence; what it can do is say which is which, which is what the closing note of
`CLAIMS.md` now does.

The general finding is worth more than any of the ten. Every figure that went stale
this week was decoration rather than claim -- a pass count beside a row about a
platforms table, a size beside a hash, a heading above a list. The claims were
checked because they looked like claims. Nothing looks less like a claim than a
number in an aside, and nothing rots faster.

## Two defects that hid each other

The pre-freeze pass read six checks wrong. Five were stale equalities in the gate
itself -- a suite of 191 against a pin of 178, and four more like it -- and the
sixth was `scripts/jq_differential.sh` exiting 1 with 48 of its 62 comparisons
disagreeing. Every disagreement was a builtin, every message was the same `cannot
appear here`, and the 14 comparisons that agreed were exactly the 14 that call no
builtin at all. A parser that rejects a bare `type` at column 1 is not this source:
`tests/query.rs` has 101 rows over that language and they were green in the same
run.

So it was the binary. This checkout sits on a drive both toolchains can see -- the
Windows one writes `jaq-lite.exe`, a Linux one writes `jaq-lite` -- and the script
decided whether to build with `if [ ! -x "$bin" ]`. The file at that path was a
471824-byte ELF from the previous evening: executable, therefore trusted. What the
current source builds is 490336 bytes, which is also the size
`reproducible_build.sh` had hashed hours earlier the same morning. Both numbers
were in one transcript, and nothing read them together. Rebuilding took the count
to 62 of 62 agreeing.

That 490336 is not the 468704 published further up this file, and neither figure is
a typo. The harness transcript above is older than the code: it recorded what the
source built on the day it ran, and the source has grown since. The section quoting
it already says the constant belongs to the host toolchain and that nothing should
be gated on it, which is why nothing is -- but somebody running the harness on this
machine today gets 490336 bytes and a different sha256, so the pairing is written
down here instead of left to be found. `tests/claims.rs` holds every size this log
states for its own binary to the one it publishes, and now names these two as the
deliberate exceptions, so a third cannot arrive quietly.

The first probe did not establish that. It tried to rebuild through
`wsl --exec bash -c "cd ... && cargo build --release"`, got `cargo: command not
found` and exit 127, left the ELF untouched at its old size and mtime -- and then
announced that the diagnosis was wrong, because its closing test was the
differential's exit code and never its own rebuild's. A probe that does not check
its precondition reports a failed experiment as a result, which is worse than
reporting nothing.

That failure was the second defect, and it was in the script too.
`scripts/reproducible_build.sh` has carried the note since the first run it lost to
this: rustup installs cargo into `~/.cargo/bin`, a login profile is what puts that
on PATH, and `wsl --exec bash script.sh` is neither login nor interactive. It
resolves cargo for itself. `jq_differential.sh` never inherited the preamble, so
its one `cargo build` line could not have run from WSL at all. Neither defect could
surface while the other stood: the stale binary was executable, so the build line
was never reached, and on a CI runner cargo is already on PATH, so the missing
preamble never cost anything there.

Both are fixed in the script rather than in a test about the script. It builds on
the default path every time and lets cargo decide whether there is work; it
resolves cargo the way its sibling does; and before comparing anything it asks the
binary in front of it for each of the eleven names on its own `BUILTINS` line,
reading exit 3 -- EXIT_FILTER, a filter that does not compile -- as the answer no.
A binary that does not know the roster now fails with one line naming the cause
instead of forty-eight naming the symptom, which also covers the one case this
script cannot rebuild for the caller: a `JAQ_LITE_BIN` that has gone stale.

The same pass measured parse at 41.8 MiB/s, above all four samples the README's
speed paragraph enumerated, so it now reports five and spans a factor of 2.4 rather
than 1.9. The fifth sample widened the spread that paragraph exists to warn about
instead of contradicting it, which is the argument it was already making. The
published figure stays the run that produced it, and that rule costs something only
on a day when the newest sample is the fastest, which is what this day was.

## The pre-freeze pass at commit 65, and what the published digest is worth

The gate ran clean at commit 65: 61 checks, none wrong, no stage short, in 1.4
minutes. Every figure a submission quotes came out of that one run -- 191 tests
passing and none failing, 95 of 95 documents accepted and 188 of 188 rejected, 62
differential comparisons agreeing byte for byte with jq 1.8.1 and none disagreeing,
35 stdlib substitutions all present, and `+11 of a possible +16` in bonuses -- and a
shallow clone of `origin` built and passed all 191 tests offline before being
removed again.

One figure came out different, and it is the one this file publishes. The
reproducible-build harness returned
`ade5b0dba26b707495c8f3098b5bde748e7b0891602e48e0f3ba42fb76ff9558` at 490336 bytes,
where the transcript near the top recorded
`46df3c5524e7e26ff84fd830a1047d555c6f1cd1e1ff8162878f99911a2a885e` at 468704
instead. All four assertions passed either way: the two unequal-length build paths
agreed with each other, the inverted control differed as it must, the leak scan
found nothing over six needles, and the two sizes were equal. Nothing about
reproducibility failed. Thirty-one commits of source landed between the two runs,
and a larger program compiles to a larger binary.

That is worth stating plainly, because this file has already drawn a stronger
conclusion from a weaker run. The earlier pre-freeze section records that the
harness "returned the same constant and the same 468704 bytes it returned two days
earlier, on a different boot of the same machine, which is the first evidence in
this log that the published constant is stable across days and not merely within one
sitting." That is true of the runs it describes, and the inference is narrower than
it reads: the constant is stable across days for source that has not changed. No
commit had intervened, so it was never evidence about a constant surviving one.

So the recipe is the claim and the digest is a dated sample. Identical source plus
identical rustc version plus identical host toolchain give identical bytes; the first
is what a clone pins, the second is pinned by `rust-toolchain.toml`, and the third is
neither pinned nor pinnable, which is why the recorded digest has never been gated
and why CI reports it rather than asserting it. Somebody who clones this tree today
and runs the verify line gets `ade5b0db` from the same four assertions that gave
`46df3c55` on the day that one was measured, and both runs are the same result.

`tests/claims.rs` now also requires every digest row 22 of `CLAIMS.md` cites to be
recorded in this file, so the ledger cannot quote a hash from a run this log never
saw. Row 22 carries both digests and the commit each belongs to.
