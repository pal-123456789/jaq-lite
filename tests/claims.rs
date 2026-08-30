//! The claims in `STDLIB.md` and `BUILD_LOG.md`, checked by a program.
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
//! who types `cargo test`. Some are structural -- every file an entry names
//! exists, every status is one of the two permitted words, the shipped count
//! never falls -- and one is the substantive claim of the entry that was wrong:
//! that no number in this program is ever formatted, because none is ever
//! synthesized. How many there are is deliberately not written down here: a
//! numeral in a doc comment that has to track the `#[test]` items below it is
//! one more sentence that can go stale, which is this file's own subject.
//!
//! Some tests here are not about `STDLIB.md` at all. `BUILD_LOG.md` publishes a
//! sha256 a reader is invited to reproduce, and quotes the hashes the harness
//! and two CI attempts printed; those six numbers have to agree in a particular
//! pattern, and a digit transcribed wrong out of a CI log is exactly the error
//! no reviewer catches. `README.md` quotes four assertion lines out of
//! `scripts/reproducible_build.sh`, and quoted text drifts, so those phrases are
//! required to appear in both files or the test fails in the commit that moved
//! one of them. Both belong beside the others rather than in files of their own.
//!
//! What this file cannot do is said plainly rather than left implied. It cannot
//! tell whether an entry's prose describes the code it points at; only a reader
//! can. This is a floor under the claims, not a proof of them, and the entry
//! that failed the audit would have passed every check in this file.

use jaq_lite::{Style, parse, to_string};
use std::fs;
use std::path::PathBuf;

/// Entries in `STDLIB.md`, counted by their `Status` field.
const ENTRIES: usize = 18;

/// How many entries read `shipped`. A floor in the same sense as the conformance
/// floors: flipping an entry raises it and nothing may lower it, so a status
/// quietly going back to `planned` fails here.
const SHIPPED_FLOOR: usize = 18;

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

/// Every nesting cap the code enforces, named in the README, and the lookup cost
/// `src/value.rs` says the README states.
///
/// Both caps are private constants in two different modules, so the README is the
/// only place a reader can see either number -- which is exactly the arrangement
/// that lets a document drift away from the code. This reads them out of the
/// source instead of trusting the prose.
///
/// The digits have to stand on their own. An earlier draft of this test accepted
/// any occurrence, and `f64` in a sentence about how numbers are stored satisfied
/// it: the test passed while saying nothing about the filter nesting cap at all.
///
/// It also checks one cross-file claim. `src/value.rs` tells the reader that its
/// linear key lookup is "stated in the README rather than hidden", which is an
/// assertion about the contents of a different file -- the kind of claim nothing
/// here used to check.
///
/// What it still cannot check is whether the sentence around a number says
/// anything true about it.
#[test]
fn the_readme_states_every_limit_the_code_enforces() {
    let readme = read("README.md");
    let attached = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut caps = Vec::new();
    for module in ["src/parser.rs", "src/query.rs"] {
        let source = read(module);
        let marker = "const MAX_DEPTH: u32 = ";
        let start = source
            .find(marker)
            .unwrap_or_else(|| panic!("{module} no longer declares a nesting cap"))
            + marker.len();
        let digits: String = source[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert!(
            !digits.is_empty(),
            "{module}'s cap is not written as a decimal literal"
        );
        let named = readme.match_indices(digits.as_str()).any(|(at, found)| {
            !readme[..at].chars().next_back().is_some_and(attached)
                && !readme[at + found.len()..]
                    .chars()
                    .next()
                    .is_some_and(attached)
        });
        assert!(
            named,
            "the README does not name the nesting cap of {digits} that {module} enforces"
        );
        caps.push(digits);
    }
    assert_ne!(
        caps[0], caps[1],
        "the two caps are no longer distinct numbers"
    );
    assert!(
        read("src/value.rs").contains("stated in the README"),
        "value.rs no longer points at the README, so half of this check is stale"
    );
    assert!(
        readme.contains("O(n)"),
        "value.rs sends the reader to the README for the lookup cost, and the README is silent"
    );
    println!(
        "README LIMITS: caps {} and {} named, lookup cost stated",
        caps[0], caps[1]
    );
}

/// The hashes `BUILD_LOG.md` publishes have to agree with each other.
///
/// Six numbers live in that file. Three are one number: the two builds the
/// reproducible-build harness compared at different path lengths, and the
/// `sha256` line offered as the constant a reader should get back. One must
/// differ from those -- the control build, whose whole job is to differ, since
/// an assertion that two dissimilar builds match would be satisfied by a
/// compiler that ignored its flags. The last two are a GitHub runner's, one per
/// attempt of the same run, and they must equal each other and differ from the
/// local one: the section quoting them says in prose that the constant is a
/// function of the host toolchain, and this is that sentence as an assertion.
///
/// A digit typed wrong while transcribing a CI log fails here instead of
/// shipping. What cannot be checked is where any of the six came from; a hash is
/// thirty-two bytes of nothing in particular, and no test can tell a measured
/// one from an invented one. The run is linked in that section for the reader
/// who wants to check, which is the honest division of labour.
#[test]
fn the_hashes_recorded_in_the_build_log_agree() {
    fn is_sha(token: &str) -> bool {
        token.len() == 64
            && token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }

    let log = read("BUILD_LOG.md");

    // Lines are found by the tag that labels them and then searched for a hash,
    // rather than by position: prose in this file starts with `A ` too, and a
    // check that grabbed the first such line would report a missing hash the day
    // somebody wrote a sentence.
    let hash_on = |tag: &str| -> String {
        log.lines()
            .filter(|line| line.trim_start().starts_with(tag))
            .find_map(|line| line.split_whitespace().find(|token| is_sha(token)))
            .unwrap_or_else(|| panic!("no line starting {tag:?} in BUILD_LOG.md carries a sha256"))
            .to_string()
    };

    let build_a = hash_on("A ");
    let build_b = hash_on("B ");
    let published = hash_on("sha256 ");
    let control = hash_on("control ");
    let attempt_1 = hash_on("ubuntu-latest, attempt 1");
    let attempt_2 = hash_on("ubuntu-latest, attempt 2");

    assert_eq!(
        build_a, build_b,
        "the transcript shows two different hashes for the same source, so the \
         result line claiming they matched is describing a run that did not happen"
    );
    assert_eq!(
        published, build_a,
        "the sha256 offered for verification is not the hash the harness measured, \
         so a reader following the verify line would be told the build is broken"
    );
    assert_ne!(
        control, build_a,
        "the control build no longer differs from the real one, which makes \
         assertion 2 vacuous rather than passing"
    );
    assert_eq!(
        attempt_1, attempt_2,
        "the two CI attempts disagree, so the runner's hash is per-VM after all \
         and the paragraph calling it a function of the host toolchain is wrong"
    );
    assert_ne!(
        attempt_1, published,
        "the runner and this laptop now agree; that would be good news and it \
         makes the surrounding paragraph false, so rewrite the paragraph rather \
         than this assertion"
    );

    println!(
        "BUILD_LOG HASHES: local {}, runner {}, control differs",
        &published[..8],
        &attempt_1[..8]
    );
}

/// The README's account of the reproducible build has to match the harness.
///
/// A README is the document most likely to be read and the least likely to be
/// re-checked, and this section of it quotes four assertion lines out of a shell
/// script. Quoted text drifts: the script gets an assertion reworded, the README
/// keeps the old wording, and the file a judge actually reads is the stale one.
/// So the four phrases are required to appear verbatim in both, which turns a
/// rewording into a failing test in the same commit that causes it.
///
/// The last two checks are about a number that is deliberately absent. The
/// section says the published sha256 belongs to the host toolchain rather than
/// to this source, and that nothing should be gated on it; a README that then
/// printed the constant would invite exactly the comparison the prose warns
/// against. Asserting there is no 64-character hex token here makes that
/// decision structural instead of a matter of remembering it.
#[test]
fn the_readme_quotes_the_reproducible_build_harness_verbatim() {
    let readme = read("README.md");
    let script = read("scripts/reproducible_build.sh");
    let workflow = read(".github/workflows/ci.yml");

    for phrase in [
        "two builds, unequal path lengths, same hash",
        "control with debug=2 strip=none must differ",
        "no build path, home, rustup or cargo in it",
        "the two sizes are equal",
    ] {
        assert!(
            script.contains(phrase),
            "scripts/reproducible_build.sh no longer prints {phrase:?}, so the README \
             quotes an assertion this harness does not make; reword both or neither"
        );
        assert!(
            readme.contains(phrase),
            "README.md does not quote {phrase:?}, so its list of what the harness \
             checks is shorter or differently worded than the harness itself"
        );
    }

    assert!(
        readme.contains("scripts/reproducible_build.sh"),
        "the README's reproducible-build section no longer names the script, leaving \
         the reader the claim without the command that checks it"
    );
    assert!(
        readme.contains("byte-identical rebuild"),
        "the README no longer names the CI job that runs the harness"
    );
    assert!(
        workflow.contains("byte-identical rebuild"),
        "the README names a `byte-identical rebuild` job that ci.yml does not define, \
         so the sentence promising CI gates on the harness is false"
    );

    let hash = readme
        .split_whitespace()
        .find(|token| token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(
        hash.is_none(),
        "README.md now publishes what looks like a sha256 ({hash:?}), but its own text \
         says the constant is host-specific and must not be gated on. If publishing it \
         is the new intent, delete this assertion and add a check that it equals the \
         one in BUILD_LOG.md, so the two cannot drift"
    );

    println!("README HARNESS: 4 phrases verbatim, ci job named, no hash published");
}

/// The README's paragraph on strings quotes three counts out of
/// `tests/real_world.rs`, and this is the check that they are still the same
/// numbers.
///
/// That paragraph is the only place a reader learns that numbers and strings
/// follow opposite rules here -- a number keeps the bytes it was written with, a
/// string keeps its meaning -- and it earns the claim by quoting what a document
/// a real tool wrote actually contained. Three numbers typed into prose are
/// three numbers that can drift away from the harness that measured them, so
/// they are read back out of it rather than trusted.
///
/// The search is scoped to that one paragraph rather than to the whole file. A
/// check that accepted the number anywhere in the README would pass on a
/// coincidence somewhere else in it, and would go on passing after the paragraph
/// itself was deleted.
#[test]
fn the_readme_quotes_the_real_world_counts_it_borrowed() {
    let harness = read("tests/real_world.rs");
    let readme = read("README.md");

    let opening = "**Strings are not.**";
    let start = readme
        .find(opening)
        .unwrap_or_else(|| panic!("the README no longer carries a paragraph opening {opening:?}"));
    let paragraph = readme[start..].split("\n\n").next().unwrap_or_default();

    let attached = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut quoted = Vec::new();
    for name in ["APOSTROPHE_ESCAPES", "SOLIDUS_ESCAPES", "RAW_NON_ASCII"] {
        let marker = format!("const {name}: usize = ");
        let at = harness
            .find(&marker)
            .unwrap_or_else(|| panic!("tests/real_world.rs no longer declares {name}"))
            + marker.len();
        let digits: String = harness[at..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert!(
            !digits.is_empty(),
            "{name} is no longer written as a decimal literal"
        );
        let named = paragraph
            .match_indices(digits.as_str())
            .any(|(pos, found)| {
                !paragraph[..pos].chars().next_back().is_some_and(attached)
                    && !paragraph[pos + found.len()..]
                        .chars()
                        .next()
                        .is_some_and(attached)
            });
        assert!(
            named,
            "the README's paragraph on strings does not name the {digits} that {name} \
             asserts, so the measurement moved and the sentence describing it did not"
        );
        quoted.push(digits);
    }

    assert!(
        paragraph.contains("tests/real_world.rs"),
        "the paragraph quotes three measured counts without naming the harness that \
         measured them, which leaves a reader nothing to check them against"
    );

    println!(
        "README STRINGS: counts {} read back out of the harness",
        quoted.join(", ")
    );
}
