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
| 14 | The suite is green | `cargo test` | 164 passing, 0 failing, summed across every target |
| 15 | The jq comparison is against a real jq | `wsl --exec jq --version` | `jq-1.8.1` |
| 16 | jq re-renders numbers where this project preserves them | `jq -c . <<< 1e2` | `1E+2` against `1e2` here |
| 17 | A stream with one failing document exits non-zero here | `jaq-lite -c .a <<< '1 {"a":2}'` | exit 5, where jq exits 0 |
| 18 | Parsing and serializing clear the floor a release build asserts | `cargo test --release --test throughput -- --nocapture --test-threads=1` | 3 passing, floor 5 MiB/s, one run: parse 26.5, serialize 197.9 |
| 19 | A document over a megabyte survives parse and to_string identically | `cargo test --test throughput` | 1196671 bytes in, the reparsed value identical |
| 20 | The query language behaves as its table says, and the table covers every way a filter can fail | `cargo test --test query -- --nocapture --test-threads=1` | 5 passing, 40 rows written out and 1 built, 11 named failures, 28 values round tripped |
| 21 | The README names both nesting caps the code enforces, and the lookup cost `src/value.rs` says it states | `cargo test --test claims -- --nocapture --test-threads=1` | caps 128 and 64 named, lookup cost stated |
| 22 | Two release builds of this source on one machine are byte identical, and the recorded constant belongs to the host toolchain rather than to the machine | `bash scripts/reproducible_build.sh`, then CI run 18, attempts 1 and 2 | four assertions PASS on three machines; `bbf72e72` twice on `ubuntu-latest`, `46df3c55` here |
| 23 | Documents that `cargo`, `rustc` and PowerShell really emitted round trip, and the four that arrived without whitespace come back byte for byte | `cargo test --test real_world -- --nocapture --test-threads=1` | 3 passing; 8 documents, 4 byte-identical, 104 + 48 escapes decoded, 31 raw bytes unchanged (measured 2026-08-30) |
| 24 | Every compatibility claim in the README came from running `jq` beside this binary, and the two still agree | `scripts/jq_differential.sh` | 14 comparisons, 0 disagreements against jq-1.8.1 (measured 2026-08-30); re-run by the `jq differential` CI job on every push |

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
