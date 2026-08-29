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
| 14 | The suite is green | `cargo test` | 87 passing, 0 failing |
| 15 | The jq comparison is against a real jq | `wsl --exec jq --version` | `jq-1.8.1` |
| 16 | jq re-renders numbers where this project preserves them | `jq -c . <<< 1e2` | `1E+2` against `1e2` here |
| 17 | A stream with one failing document exits non-zero here | `jaq-lite -c .a <<< '1 {"a":2}'` | exit 5, where jq exits 0 |

Row 9 was corrected on 2026-08-29. It recorded a binary that printed its own
version, which stopped being true the moment the CLI learned to take arguments.
This ledger's rule is that a row which no longer reproduces is a bug, so it was
re-measured rather than deleted, and the correction is noted rather than made
quietly.

Rows for round-trip fidelity, the reproducible build and performance are added as
those become measurable.