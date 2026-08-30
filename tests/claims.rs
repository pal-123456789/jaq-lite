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
//! never falls -- and two are the substantive claims of the entry that was
//! wrong: that no number this program reads is ever reformatted, and that the
//! only numbers it builds itself are the counts a builtin answers with, in the
//! two files entry 7 permits. How many there are is deliberately not written
//! down here: a numeral in a doc comment that has to track the `#[test]` items
//! below it is one more sentence that can go stale, which is this file's own
//! subject.
//!
//! Some tests here are not about `STDLIB.md` at all. `BUILD_LOG.md` publishes a
//! sha256 a reader is invited to reproduce, and quotes the hashes the harness
//! and two CI attempts printed; those six numbers have to agree in a particular
//! pattern, and a digit transcribed wrong out of a CI log is exactly the error
//! no reviewer catches. `README.md` quotes four assertion lines out of
//! `scripts/reproducible_build.sh`, and quoted text drifts, so those phrases are
//! required to appear in both files or the test fails in the commit that moved
//! one of them. The README also accounts for all four bonuses the event scores,
//! including the one this entry declines, and names the `STDLIB.md` entry it
//! nominates as its Package Killer; that number is read back out of the document
//! rather than believed, because a nomination that moves and a README that does
//! not is this file's subject over again. All of it belongs beside the others
//! rather than in files of their own.
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

/// The only files allowed to build a `Number`. This is the substance of entry 7.
///
/// It was one file until `length` needed to answer with a count. Entry 7 said in
/// prose that the day a builtin had to print a number this claim would stop being
/// true out loud rather than quietly, and this is that day: the list grew by one
/// file, in a commit that says so, instead of the check being deleted.
const SYNTHESIS_SITES: &[&str] = &["lexer.rs", "value.rs"];

/// How a `Number` is built, spelled as it appears at a call site.
///
/// The two constructors in `value.rs` are deliberately written `Number::new(`
/// rather than `Self::new(`, which is what an idiomatic impl block would use, so
/// that this grep keeps counting them. A check that a rename can silence is not a
/// check.
const SYNTHESIS: &str = "Number::new(";

/// How many call sites there are: one in the lexer, two in `value.rs`.
const SYNTHESIS_COUNT: usize = 3;

/// The file that must never look at a number's `f64`, which is the half of entry 7
/// that decides whether a number survives.
const NEVER_REFORMATS: &str = "src/serializer.rs";

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
fn numbers_are_synthesized_only_where_entry_7_says_they_are() {
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
            count == 0 || SYNTHESIS_SITES.contains(&name.as_str()),
            "{name} builds a Number, and entry 7 of STDLIB.md says only {SYNTHESIS_SITES:?} do"
        );
        total += count;
    }
    println!("SYNTHESIS SITES: {total} (want {SYNTHESIS_COUNT}, in {SYNTHESIS_SITES:?})");
    assert_eq!(
        total, SYNTHESIS_COUNT,
        "entry 7 claims {SYNTHESIS_COUNT} call sites for {SYNTHESIS} and there are {total}"
    );
}

#[test]
fn the_serializer_never_reads_a_number_as_a_float() {
    // The other half of entry 7, and the load-bearing half now that the count of
    // call sites is three rather than one. A `Number` carries both the bytes it
    // was read from and an `f64`; the whole substitution for `ryu` is that the
    // writing path only ever touches the bytes. Counting constructors no longer
    // proves that on its own, so this asserts it directly: the one file that turns
    // values into text must not be able to see the float at all.
    let serializer = read(NEVER_REFORMATS);
    let looks = serializer.matches("as_f64").count();
    assert_eq!(
        looks, 0,
        "{NEVER_REFORMATS} reads a number's f64 {looks} time(s); entry 7 of STDLIB.md \
         claims numbers are reproduced from their bytes and never reformatted, and a \
         float reaching the writer is how that claim would quietly stop being true"
    );
    // And the bytes are what it does reach for, so this is a positive claim rather
    // than only the absence of one.
    let bytes = serializer.matches("as_str()").count();
    assert!(
        bytes > 0,
        "{NEVER_REFORMATS} no longer reads a number's literal text either, so it is \
         unclear what it writes"
    );
    println!("SERIALIZER: 0 f64 reads, {bytes} literal reads in {NEVER_REFORMATS}");
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

/// The platforms table names the compiler this project pins, twice, and both
/// targets it was built for.
///
/// The submission asks for a "tested on" line, and a line like that rots in a
/// particular way: the pin in `Cargo.toml` moves, CI keeps passing because CI
/// reads the pin rather than the prose, and the README goes on naming a compiler
/// nobody has used for days. So the version is not compared against a literal
/// here -- it is read out of `Cargo.toml` and required to appear in the table.
///
/// Requiring it exactly twice rather than at least once is the point of the
/// test. The table has one row per host, and the failure worth catching is
/// somebody updating one row and forgetting the other.
#[test]
fn the_readme_names_the_platforms_it_was_tested_on_and_the_compiler_it_pins() {
    let readme = read("README.md");
    let manifest = read("Cargo.toml");

    let marker = "rust-version = \"";
    let at = manifest
        .find(marker)
        .unwrap_or_else(|| panic!("Cargo.toml no longer declares {marker:?}"))
        + marker.len();
    let pin: String = manifest[at..].chars().take_while(|c| *c != '"').collect();
    assert!(!pin.is_empty(), "Cargo.toml declares an empty rust-version");

    let opening = "**Tested on**";
    let start = readme
        .find(opening)
        .unwrap_or_else(|| panic!("the README no longer carries a {opening} block"));
    // Up to the next heading, so a later section mentioning a target triple
    // cannot satisfy this test on the table's behalf.
    let section = readme[start..].split("\n## ").next().unwrap_or_default();

    for target in ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"] {
        assert!(
            section.contains(target),
            "the platforms table no longer names {target}, so the claim to have been \
             tested on two hosts is no longer written down anywhere"
        );
    }

    let named = section.matches(pin.as_str()).count();
    assert_eq!(
        named, 2,
        "Cargo.toml pins rustc {pin} and the platforms table names it {named} time(s) \
         rather than once per host; a row was updated without the other"
    );

    println!("README PLATFORMS: two hosts, both on the pinned rustc {pin}");
}

/// Every bonus the event scores is accounted for, including the one declined.
///
/// A bonus claim is unlike the other claims in the README. It is read out of the
/// document and scored somewhere else, by somebody who cannot see the commit that
/// changed it, so the failure worth catching here is not a false claim but a stale
/// one. A category dropped from the list reads as an oversight; a total that no
/// longer equals the parts it is made of reads as arithmetic nobody redid; and a
/// nomination that moves inside `STDLIB.md` while the README goes on naming the
/// old entry number is the defect this whole file was written for.
///
/// So all four names have to be present, the total has to equal the sum of the
/// ones not marked declined, the two counts the section states in words are read
/// back out of the tree, the nominated entry number is read out of `STDLIB.md`
/// rather than believed, and every file the section points at has to exist.
///
/// What this cannot check is whether the reasoning in the section is any good, or
/// whether the bonuses named here are the ones the event actually offers.
#[test]
fn the_readme_accounts_for_every_bonus_the_event_scores() {
    // Small numbers as the section spells them, which is in words.
    const WORDS: &[&str] = &[
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];

    // A number spelled in prose has to stand on its own: digits and words alike
    // are substrings of other words, and `ten` inside `written` is not a claim.
    fn stands_alone(text: &str, word: &str) -> bool {
        let attached = |c: char| c.is_ascii_alphanumeric() || c == '_';
        text.match_indices(word).any(|(at, found)| {
            !text[..at].chars().next_back().is_some_and(attached)
                && !text[at + found.len()..]
                    .chars()
                    .next()
                    .is_some_and(attached)
        })
    }

    let readme = read("README.md");
    let opening = "## Bonus claims";
    let start = readme
        .find(opening)
        .unwrap_or_else(|| panic!("the README no longer carries a {opening} section"));
    // Up to the next heading, so no later section can satisfy these assertions on
    // the bonus section's behalf.
    let section = readme[start + opening.len()..]
        .split("\n## ")
        .next()
        .unwrap_or_default();

    let mut claimed = 0usize;
    let mut offered = 0usize;
    for name in [
        "Reproducible Build",
        "Package Killer",
        "STDLIB Log",
        "Single File",
    ] {
        let marker = format!("**{name}, +");
        let at = section
            .find(&marker)
            .unwrap_or_else(|| panic!("the bonus section no longer names {name}"))
            + marker.len();
        let digits: String = section[at..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let points: usize = digits
            .parse()
            .unwrap_or_else(|_| panic!("{name} no longer states its points as a number"));
        // Whether this one is claimed is decided by the word, not by a list here:
        // a category moving between claimed and declined is a prose edit, and the
        // arithmetic below has to follow it without anybody remembering to.
        let head: String = section[at..].chars().take(digits.len() + 40).collect();
        if !head.contains("Declined") {
            claimed += points;
        }
        offered += points;
    }
    let arithmetic = format!("+{claimed} of a possible +{offered}");
    assert!(
        section.contains(&arithmetic),
        "the four categories in the bonus section add up to {arithmetic}, which is not \
         the total it states"
    );

    // Two counts the section states in words. Both are read out of the tree, so
    // adding a source file or an entry fails here rather than leaving a number in
    // the README that was true last week.
    for (count, what) in [
        (source_files().len(), "files under src/"),
        (ENTRIES, "entries in STDLIB.md"),
    ] {
        let word = WORDS
            .get(count)
            .copied()
            .unwrap_or_else(|| panic!("{count} {what} is past the end of this test's words"));
        assert!(
            stands_alone(section, word),
            "there are {count} {what} and the bonus section does not say {word}"
        );
    }

    // The nominated entry number, read out of the document that nominates it.
    let stdlib = read("STDLIB.md");
    let nomination = "This is the nominated Package Killer.";
    let nominated = stdlib.matches(nomination).count();
    assert_eq!(
        nominated, 1,
        "{nominated} entries of STDLIB.md claim to be the nominated Package Killer"
    );
    let at = stdlib.find(nomination).unwrap_or_default();
    let entry = stdlib[..at]
        .lines()
        .rev()
        .find_map(|line| line.split_once(". **Normally:**"))
        .and_then(|(number, _)| number.parse::<usize>().ok())
        .expect("the nomination is not inside a numbered entry of STDLIB.md");
    assert!(
        section.contains(&format!("Entry {entry} of ")),
        "STDLIB.md nominates entry {entry} and the bonus section names a different one"
    );

    // And every file it points at, because a claim that cites its own evidence is
    // only worth as much as the citation.
    let mut paths = 0;
    for span in section.split('`').skip(1).step_by(2) {
        if span.contains(' ')
            || ![".md", ".rs", ".sh", ".toml"]
                .iter()
                .any(|e| span.ends_with(*e))
        {
            continue;
        }
        assert!(
            root().join(span).exists(),
            "the bonus section points at {span}, which does not exist"
        );
        paths += 1;
    }
    assert!(
        paths >= 3,
        "the bonus section names {paths} file(s), so it has stopped pointing at its evidence"
    );

    println!(
        "README BONUSES: +{claimed} of a possible +{offered}, entry {entry} nominated, \
         {paths} paths named"
    );
}
