# Build log

A running record of what was done, when, and why. Written as the work happens
rather than reconstructed afterwards.

## 2026-08-29, 09:00-09:15 IST - Sprint 0, commits 1 to 5

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

## Sprint 0 -- scaffold, corpus, baseline (Sat 09:02 to 09:36 IST)

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