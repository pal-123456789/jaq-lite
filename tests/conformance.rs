//! The RFC 8259 conformance harness.
//!
//! Every file in `tests/fixtures/test_parsing` goes through `jaq_lite::parse`
//! and is scored against what its name promises: `y_` must be accepted, `n_`
//! must be rejected, `i_` is implementation-defined and is reported rather than
//! scored.
//!
//! To see the report:
//!
//! ```text
//! cargo test --test conformance -- --nocapture --test-threads=1
//! ```
//!
//! Without `--nocapture` the report is invisible, because `cargo test` captures
//! stdout from tests that pass and only shows it for tests that fail.

use std::fs;
use std::path::{Path, PathBuf};

/// The minimum number of `y_` fixtures that must parse.
///
/// This is raised in the same commit as any grammar work that improves it. The
/// point is that a later change which breaks something already working fails
/// here instead of being discovered by reading the report carefully.
const Y_FLOOR: usize = 5;

/// Every `n_` fixture must be rejected, and that is true from the first commit
/// because a parser that accepts nothing rejects all of them. So this floor is
/// a real invariant immediately: it can only ever be broken by a grammar that
/// is too permissive.
const N_FLOOR: usize = 188;

/// The size of the vendored corpus. Checked so that a partial or damaged
/// checkout produces a failure rather than a flattering score over whatever
/// files happen to be present.
const TEST_PARSING_FILES: usize = 318;

/// How many failing fixtures to name in the report before summarising.
const SHOW: usize = 10;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_parsing")
}

/// Collect the fixture paths in a deterministic order.
///
/// The sort is not cosmetic. `read_dir` yields entries in an order the
/// filesystem chooses: alphabetical on NTFS, hash order on ext4. Sorting here
/// is what makes the report byte-identical on a developer machine and on a
/// Linux CI runner, which matters because the report is quoted in the README.
fn sorted_fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .expect("tests/fixtures/test_parsing is missing; see tests/fixtures/ATTRIBUTION.md")
        .map(|entry| entry.expect("unreadable directory entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
}

/// Read a floor from the environment, which may raise it but never lower it.
///
/// Allowing a floor to be lowered would defeat its purpose, so the committed
/// constant is always a hard minimum and the variable is only useful for
/// tightening the check, for instance in CI.
fn floor(var: &str, committed: usize) -> usize {
    match std::env::var(var) {
        Ok(raw) => {
            let parsed: usize = raw
                .parse()
                .unwrap_or_else(|_| panic!("{var} must be a whole number, got {raw:?}"));
            committed.max(parsed)
        }
        Err(_) => committed,
    }
}

#[test]
fn rfc8259_conformance() {
    let dir = fixtures_dir();
    let paths = sorted_fixtures(&dir);
    assert_eq!(
        paths.len(),
        TEST_PARSING_FILES,
        "expected {TEST_PARSING_FILES} fixtures in {}, found {}",
        dir.display(),
        paths.len()
    );

    let mut y_pass = 0usize;
    let mut n_pass = 0usize;
    let mut i_accept = 0usize;
    let mut i_reject = 0usize;
    let mut y_fail: Vec<String> = Vec::new();
    let mut n_fail: Vec<String> = Vec::new();

    for path in &paths {
        let name = path
            .file_name()
            .expect("fixture path has no file name")
            .to_string_lossy()
            .into_owned();
        let bytes = fs::read(path).expect("fixture unreadable");
        match (name.as_bytes()[0], jaq_lite::parse(&bytes)) {
            (b'y', Ok(_)) => y_pass += 1,
            (b'y', Err(e)) => y_fail.push(format!("{name}: {e}")),
            (b'n', Err(_)) => n_pass += 1,
            (b'n', Ok(_)) => n_fail.push(name),
            (b'i', Ok(_)) => i_accept += 1,
            (b'i', Err(_)) => i_reject += 1,
            _ => panic!("fixture name starts with none of y_, n_, i_: {name}"),
        }
    }

    let y_total = y_pass + y_fail.len();
    let n_total = n_pass + n_fail.len();
    let i_total = i_accept + i_reject;

    println!();
    println!(
        "RFC 8259 conformance -- JSONTestSuite 1ef36fa, {} files",
        paths.len()
    );
    println!("  y_  must accept  : {y_pass}/{y_total}");
    println!("  n_  must reject  : {n_pass}/{n_total}");
    println!(
        "  i_  our choice   : {i_accept} accepted, {i_reject} rejected, of {i_total} (implementation-defined)"
    );

    if !y_fail.is_empty() {
        println!();
        println!(
            "  y_ not yet accepted -- {} total, first {} shown:",
            y_fail.len(),
            SHOW.min(y_fail.len())
        );
        for line in y_fail.iter().take(SHOW) {
            println!("    {line}");
        }
    }

    if !n_fail.is_empty() {
        println!();
        println!("  n_ WRONGLY ACCEPTED -- every line here is a bug:");
        for name in &n_fail {
            println!("    {name}");
        }
    }

    let y_floor = floor("JAQ_Y_FLOOR", Y_FLOOR);
    let n_floor = floor("JAQ_N_FLOOR", N_FLOOR);
    println!();
    println!("  floors: y_ >= {y_floor}, n_ >= {n_floor}");
    println!();

    assert!(
        y_pass >= y_floor,
        "y_ conformance is {y_pass}, below the floor of {y_floor}"
    );
    assert!(
        n_pass >= n_floor,
        "n_ conformance is {n_pass}, below the floor of {n_floor}; wrongly accepted: {n_fail:?}"
    );
}
