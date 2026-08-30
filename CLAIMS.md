# Claims ledger

Every factual claim this project makes, the command that proves it, and the
result that command produced. Re-run top to bottom before submission; a row
that no longer reproduces is a bug in the README, not a rounding error.

Verified 2026-08-29 09:15 IST on Windows 11, rustc 1.98.0 (88d9e12ae), unless a
row says otherwise.

| # | Claim | Command | Result |
|---|---|---|---|
| 1 | The manifest declares no dependencies | `Get-Content Cargo.toml` | `[dependencies]` table present and empty |
| 2 | The lockfile contains exactly one package, this crate | `Select-String -Path Cargo.lock -Pattern "^\[\[package\]\]"` | count = 1 |
| 3 | The release profile keys are recognised, not ignored | `cargo build --release` | no `unused manifest key` warning |
| 4 | The toolchain is pinned | `Get-Content rust-toolchain.toml` | `channel = "1.98.0"` |
| 5 | Formatting is clean | `cargo fmt --all -- --check` | no output |
| 6 | Clippy is clean at deny level | `cargo clippy --all-targets -- -D warnings` | no output |
| 7 | Documentation builds with no warnings | `cargo doc --no-deps` | no output beyond `Generated` |
| 8 | No unsafe code exists | `Select-String -Path src\*.rs -Pattern unsafe` | only the two `forbid(unsafe_code)` attributes |
| 9 | The binary links the library and refuses to guess | `cargo run --quiet` | usage on standard error, exit 2 |

| 10 | Every document that must parse, parses | `cargo test --test conformance -- --nocapture` | `y_  must accept  : 95/95` |
| 11 | Every malformed document is rejected | the same run | `n_  must reject  : 188/188` |
| 12 | Each implementation-defined case has a recorded decision | `cargo test --test conformance` | `tests/i_decisions.tsv`, 35 rows, 10 accept, 25 reject |
| 13 | The release build needs no network and no lockfile rewrite | `cargo build --release --offline --locked` | succeeds; captured in `deps-proof.txt` |
| 14 | The suite is green | `cargo test` | 191 `#[test]` functions across every target, all passing, 0 failing; the figure is grepped out of `src/` and `tests/` by `the_ledger_counts_the_tests_the_tree_actually_has` rather than typed here, because it had already gone stale twice |
| 15 | The jq comparison is against a real jq | `wsl --exec jq --version` | `jq-1.8.1` |
| 16 | jq re-renders numbers where this project preserves them | `jq -c . <<< 1e2` | `1E+2` against `1e2` here |
| 17 | A stream with one failing document exits non-zero here | `jaq-lite -c .a <<< '1 {"a":2}'` | exit 5, where jq exits 0 |
| 18 | Parsing and serializing clear the floor a release build asserts | `cargo test --release --test throughput -- --nocapture --test-threads=1` | 3 passing; five runs printed parse 17.7 to 41.8 MiB/s and serialize 143.5 to 228.5, every one clearing the committed floor of 5 |
| 19 | A document over a megabyte survives parse and to_string identically | `cargo test --test throughput` | 1196671 bytes in, the reparsed value identical |
| 20 | The query language behaves as its table says, and the table covers every way a filter can fail | `cargo test --test query -- --nocapture --test-threads=1` | 6 passing, 101 rows written out and 1 built, 15 named failures, 74 values round tripped, and 59 of those rows calling one of the eleven builtins by name |
| 21 | The README names both nesting caps the code enforces, and the lookup cost `src/value.rs` says it states | `cargo test --test claims -- --nocapture --test-threads=1` | caps 128 and 64 named, lookup cost stated |
| 22 | Two release builds of this source on one machine are byte identical, and the recorded constant belongs to the host toolchain rather than to the machine | `bash scripts/reproducible_build.sh`, then CI run 18, attempts 1 and 2 | four assertions PASS on three machines; `bbf72e72` twice on `ubuntu-latest`, `46df3c55` here |
| 23 | Documents that `cargo`, `rustc` and PowerShell really emitted round trip, and the four that arrived without whitespace come back byte for byte | `cargo test --test real_world -- --nocapture --test-threads=1` | 3 passing; 8 documents, 4 byte-identical, 104 + 48 escapes decoded, 31 raw bytes unchanged (measured 2026-08-30) |
| 24 | Every compatibility claim in the README came from running `jq` beside this binary, and the two still agree | `scripts/jq_differential.sh` | 62 comparisons, 0 disagreements against jq-1.8.1 (measured 2026-08-30); every builtin is reached by at least one of them and the script fails on one that is not; the count is held to the script, to this row and to the README by `the_differential_compares_every_builtin_this_build_has`; re-run by the `jq differential` CI job on every push |
| 25 | The platforms table names both targets this was built on, and the compiler version in it is the one `Cargo.toml` pins | `cargo test --test claims -- --nocapture --test-threads=1` | 14 passing; two hosts, both rustc 1.98.0 (measured 2026-08-30); the count is the one this file holds, read back out of it by `the_ledger_counts_the_tests_the_tree_actually_has` |
| 26 | The eleven builtins answer the way jq 1.8.1 answers, including where they refuse | `cargo test --test query -- --nocapture --test-threads=1` | 59 of the 101 table rows call a builtin by name, and every value in them was measured against jq 1.8.1 before any of it was implemented; the roster the test iterates is read out of the rejection message rather than repeated, so a twelfth builtin with no rows of its own fails here |
| 27 | Numbers are still never reformatted, now that a builtin has to print one | `cargo test --test claims -- --nocapture --test-threads=1` | 3 `Number::new` call sites in the 2 files entry 7 names, and 0 reads of `as_f64` in `src/serializer.rs` |
| 28 | Every bonus the event scores is accounted for in the README, including the one declined | `cargo test --test claims -- --nocapture --test-threads=1` | `+11 of a possible +16`, entry 7 nominated as the Package Killer, 4 paths named and all of them present |
| 29 | The README names every builtin the code has, and this ledger states the test count the tree has | `cargo test --test claims -- --nocapture --test-threads=1` | 11 names read as text out of the private `Builtin::name`, every one of them found between backticks in the limits section of `README.md` with the count spelled out; 15 failure tags counted out of `tests/query.rs`; row 14 equal to the `#[test]` count of the tree |

Row 9 was corrected on 2026-08-29. It recorded a binary that printed its own
version, which stopped being true the moment the CLI learned to take arguments.
This ledger's rule is that a row which no longer reproduces is a bug, so it was
re-measured rather than deleted, and the correction is noted rather than made
quietly.

Row 14 was re-measured on 2026-08-29 under the same rule. It recorded a suite of
87 tests, which stopped being true sixty-five tests later, and a row that no
longer reproduces is a bug however small the number. Performance and round-trip
fidelity became measurable in commit 46 and are rows 18 and 19. The last claim
unrecorded here was the reproducible build, which waited on the hash comparison
in CI having run twice against the same commit. It ran twice on 2026-08-29,
printed the same runner hash on two separate machines, and is row 22.

Row 18 was rewritten on 2026-08-29, one commit after it was added and by the rule
this file already stated. As first written it claimed the figures in the README
were what the command prints. They are not repeatable to three digits: the same
release binary, minutes apart on an idle machine, differed by nearly a factor of
two. The row now claims the thing that does reproduce, which is that the floor is
cleared, and the digits are given as one run. The floor moved with it, from twenty
to five, because a floor that cannot tell drift from weather fails on somebody
else's machine rather than on mine.

Rows 14, 20, 25 and 26 were all re-measured on 2026-08-30, in the evening, when
the builtins went from four to eleven. Not one of them was wrong about anything
the commit changed on purpose: they were wrong about how many tests the suite has,
how many rows the query table has, how many ways a filter can fail, and how many
builtins there are. Every gate passed while all four were false, which is the
argument for row 29 -- a figure no program re-derives is a figure that is true on
the day it is typed. Three of the four are now re-derived, and the fourth is the
count of a table that a test compares against `ROW_COUNT`.

Row 25 was corrected the next day, and it is the one worth reading twice. The
paragraph above says three of the four figures are re-derived. Row 25's was not one
of them, because the number in it is not the claim: the claim is the platforms
table, and that is checked. The pass count beside it was decoration, and decoration
is where a stale figure hides. It said thirteen while `tests/claims.rs` held
fourteen tests, it survived the commit that added the fourteenth, and it was found
by reading this file against the tree rather than by any gate. It is now read back
out of that file, so the decoration is held to the standard of the claim.

Two figures elsewhere were wrong in the same reading and are corrected in the same
commit. A paragraph of `BUILD_LOG.md` stated a size for the published binary that
disagreed with the harness transcript printed a thousand lines above it, and one
sha256 cannot have two sizes; every size that log states for its own binary is now
held to the one it publishes, by the test that already read its hashes. Entry 9 of
`STDLIB.md` still scoped a count of invalid-UTF-8 fixtures to the whole corpus
after the log had already narrowed it to `test_parsing`; that one is a sentence, and
no program here checks it. Saying so is the rule this file opens with.

Rows 26 and 27 were added earlier the same day, with the first four builtins. Row
27 replaces a
stricter claim that named exactly one `Number::new` call site: `length` has to
answer with a count, the test guarding the old wording went red before the commit
could be made, and the wording was sharpened instead of the test being removed.
`BUILD_LOG.md` has the reasoning, including the two worse ways out that were
available.

Row 28 was added on 2026-08-30 with the bonus section of the README. An earlier
draft of this paragraph called it the last row this ledger would gain before
submission; row 29 arrived the same evening, and a prediction this document got
wrong is corrected in place rather than quietly deleted, under the same rule as
the rows themselves. Three bonuses are claimed --
Reproducible Build, nominated as the headline, the Package Killer entry
`STDLIB.md` nominates, and the log itself -- and Single File is declined in
writing rather than by omission. The row exists because a bonus claim is the one
part of a submission that is read out of the README and scored somewhere else, by
somebody who cannot see the commit that changed it; a sentence with a score
attached to it is worth a test.
