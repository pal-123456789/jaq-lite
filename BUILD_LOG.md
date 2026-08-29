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
