//! The claims in `STDLIB.md`, checked by a program.
//!
//! `STDLIB.md` is a scored artifact. It names each crate this project does not
//! use and the standard-library machinery that took its place, and it is the
//! document a reader opens to see whether the zero-dependency story is real. It
//! is also prose, and prose does not fail a build.
//!
//! One entry described a `core::fmt::NumBuffer` fast path, with a measured
//! figure attached, for code that was never written: the measurement came from a
//! throwaway warm-up crate and the sentence was written from the measurement
//! rather than from this tree. The entry's `Status` field said `planned` the
//! whole time and was never wrong, because the preamble's rule governs that
//! field. Nothing governed the body.
//!
//! So the claims a program can check are checked here, on every run, by anyone
//! who types `cargo test`. Three are structural -- every file an entry names
//! exists, every status is one of the two permitted words, the shipped count
//! never falls -- and one is the substantive claim of the entry that was wrong:
//! that no number in this program is ever formatted, because none is ever
//! synthesized.
//!
//! What this file cannot do is said plainly rather than left implied. It cannot
//! tell whether an entry's prose describes the code it points at; only a reader
//! can. This is a floor under the claims, not a proof of them, and the entry
//! that failed the audit would have passed every check in this file.

use jaq_lite::{Style, parse, to_string};
use std::fs;
use std::path::PathBuf;

/// Entries in `STDLIB.md`, counted by their `Status` field.
const ENTRIES: usize = 17;

/// How many entries read `shipped`. A floor in the same sense as the conformance
/// floors: flipping an entry raises it and nothing may lower it, so a status
/// quietly going back to `planned` fails here.
const SHIPPED_FLOOR: usize = 17;

/// The one file allowed to build a `Number`. This is the substance of entry 7.
const SYNTHESIS_SITE: &str = "lexer.rs";

/// How a `Number` is built, spelled as it appears at a call site.
const SYNTHESIS: &str = "Number::new(";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"))
}

/// Every `.rs` file directly under `src/`, sorted.
///
/// Not recursive, and it asserts there is nothing to recurse into: a new
/// subdirectory has to be noticed here rather than skipped in silence.
fn source_files() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(root().join("src")).expect("src is missing") {
        let entry = entry.expect("cannot read a directory entry");
        let kind = entry.file_type().expect("cannot stat a directory entry");
        assert!(
            kind.is_file(),
            "src contains {:?}, which is not a file, and this walk is not recursive",
            entry.path()
        );
        let path = entry.path();
        if path.extension().and_then(|part| part.to_str()) == Some("rs") {
            found.push(path);
        }
    }
    found.sort();
    assert!(found.len() > 1, "src holds {} rust files", found.len());
    found
}

/// The `Where` field and the status of every entry, one pair per entry.
///
/// The two fields share one line, separated by a character this file has no
/// reason to name: everything before `*Status:*` is the `Where` field and
/// everything after it is the status.
fn entries(stdlib: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for line in stdlib.lines() {
        if let Some(cut) = line.find("*Status:*") {
            let (before, after) = line.split_at(cut);
            let status = after["*Status:*".len()..].trim().to_owned();
            found.push((before.to_owned(), status));
        }
    }
    found
}

/// The files a `Where` field names, which are exactly its backtick spans.
///
/// Everything in that field is a path, so anything else written between
/// backticks there fails the check that calls this.
fn backtick_spans(field: &str) -> Vec<String> {
    field
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn no_number_is_synthesized_outside_the_lexer() {
    let mut total = 0usize;
    for path in source_files() {
        let name = path
            .file_name()
            .and_then(|part| part.to_str())
            .expect("a source name is UTF-8")
            .to_owned();
        let text = fs::read_to_string(&path).expect("cannot read a source file");
        let count = text.matches(SYNTHESIS).count();
        if count > 0 {
            println!("  {count}  {name}");
        }
        assert!(
            count == 0 || name == SYNTHESIS_SITE,
            "{name} builds a Number, and entry 7 of STDLIB.md says only {SYNTHESIS_SITE} does"
        );
        total += count;
    }
    println!("SYNTHESIS SITES: {total} (want 1, in {SYNTHESIS_SITE})");
    assert_eq!(
        total, 1,
        "entry 7 claims one call site for {SYNTHESIS} and there are {total}"
    );
}

#[test]
fn a_number_keeps_its_bytes_through_the_public_api() {
    const KEPT: [&str; 8] = [
        "0",
        "-0",
        "1e2",
        "1E+2",
        "0.10",
        "1.0",
        "1e999",
        "123456789012345678901234567890",
    ];
    for source in KEPT {
        let value = parse(source.as_bytes())
            .unwrap_or_else(|error| panic!("{source} will not parse: {error}"));
        let printed = to_string(&value, Style::Compact);
        assert_eq!(printed, source, "{source} came back as {printed}");
    }
    println!("NUMBERS KEPT: {} spellings, byte for byte", KEPT.len());
}

#[test]
fn every_status_is_one_of_the_two_permitted_words() {
    let stdlib = read("STDLIB.md");
    let found = entries(&stdlib);
    assert_eq!(
        found.len(),
        ENTRIES,
        "STDLIB.md holds {} entries, not {ENTRIES}",
        found.len()
    );
    let mut planned = 0usize;
    let mut shipped = 0usize;
    for (_, status) in &found {
        match status.as_str() {
            "planned" => planned += 1,
            "shipped" => shipped += 1,
            other => panic!("an entry reads Status: {other}, which is neither planned nor shipped"),
        }
    }
    println!(
        "STDLIB: {} entries, {planned} planned, {shipped} shipped (floor {SHIPPED_FLOOR})",
        found.len()
    );
    assert!(
        shipped >= SHIPPED_FLOOR,
        "{shipped} entries read shipped and the floor is {SHIPPED_FLOOR}"
    );
}

#[test]
fn every_file_a_stdlib_entry_names_exists() {
    let stdlib = read("STDLIB.md");
    let mut named = 0usize;
    for (field, status) in entries(&stdlib) {
        let paths = backtick_spans(&field);
        assert!(
            !paths.is_empty(),
            "an entry marked {status} names no file: {field}"
        );
        for path in paths {
            assert!(
                root().join(&path).is_file(),
                "STDLIB.md names {path}, which is not a file in this tree"
            );
            named += 1;
        }
    }
    println!("STDLIB PATHS: {named} named, all present");
    assert!(
        named >= ENTRIES,
        "{named} paths named across {ENTRIES} entries"
    );
}
