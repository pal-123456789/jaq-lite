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
| 9 | The binary links the library | `cargo run --quiet` | prints `jaq-lite 0.1.0` |

Rows for conformance, round-trip fidelity, jq differential, reproducible build
and performance are added as those become measurable.