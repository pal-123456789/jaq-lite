//! Bytes nobody chose, and the two answers the parser is allowed to give.
//!
//! The corpus under `tests/fixtures` is a set of documents someone wrote on
//! purpose, each with an expected verdict. This file takes the ones that must
//! parse and damages them: a bit flipped, a byte replaced with one that means
//! something to a JSON parser, a document cut short, a byte inserted or dropped,
//! a run copied. None of the results has an expected verdict, and that is the
//! point. The claim is weaker than conformance and much harder to satisfy by
//! accident: whatever the bytes are, `parse` answers `Ok` or `Err`, and never
//! panics, never indexes past the end, never returns a position that cannot be
//! pointed at.
//!
//! The answer is taken through `std::panic::catch_unwind` rather than left to the
//! test runner. A panic inside a `#[test]` already fails it, but the report names
//! a line in the parser and not the input that reached it, and with 760 mutants in
//! one loop that is the only fact worth having. Catching it means the failure can
//! say which fixture, which round, which kind of damage, and print the bytes
//! ASCII-escaped so they survive a CI log.
//!
//! Two invariants on the rejection, because `Ok` or `Err` is a low bar. The
//! reported offset must lie within the input, since `locate` clamps a wild offset
//! and would hide it, while the caret renderer slices at it and would not. The
//! reported line must be one more than the number of newlines before that offset,
//! and the reported column must leave the caret on the line it points at rather
//! than one character off the end of it. A diagnostic that points somewhere
//! impossible is the failure this file is most likely to find, because no fixture
//! is shaped to provoke it.
//!
//! SplitMix64 is duplicated from `tests/roundtrip_fuzz.rs` instead of shared. Each
//! file under `tests/` is its own crate, so sharing means a `tests/common` module,
//! and every item in such a module must be used by every file that imports it or
//! the build warns and the gate fails. The published test vector is asserted in
//! both copies, so a change to either that breaks the algorithm fails a test
//! rather than quietly generating a different corpus.
//!
//! This file is what lets entry 4 of `STDLIB.md` stop saying planned.

use jaq_lite::{Style, parse, to_string};
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// Mutations per fixture. Ninety-five fixtures, so 760 documents nobody wrote.
const MUTATIONS: usize = 8;

/// How many `y_` fixtures the corpus holds. The same count `tests/roundtrip_fuzz.rs`
/// walks, and a mismatch here means the corpus moved under both of them.
const Y_FIXTURES: usize = 95;

/// Bytes that mean something to a JSON parser: structure, the start of a literal,
/// the parts of a number, and the ones that may not appear in a string unescaped.
/// A uniformly random byte is almost always uninteresting; these are the ones that
/// turn a valid document into an ambiguous one.
const INTERESTING: &[u8] = &[
    b'"', b'\\', b'{', b'}', b'[', b']', b',', b':', b'0', b'9', b'-', b'+', b'.', b'e', b'E',
    b't', b'f', b'n', b'u', b' ', b'\t', b'\n', b'\r', 0x00, 0x1f, 0x7f, 0x80, 0xc3, 0xff,
];

/// The kinds of damage, named for the failure messages. The length is the number
/// of kinds, so adding one here is all it takes to widen the search.
const KIND_NAMES: [&str; 6] = [
    "a bit flipped",
    "a byte replaced",
    "cut short",
    "a byte inserted",
    "a byte dropped",
    "a run copied",
];

/// SplitMix64, duplicated from `tests/roundtrip_fuzz.rs` for the reason given at
/// the top of this file. Both copies assert the same published vector.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n`. Plain modulo, so low values are favoured by about
    /// `n / 2^64`, which for the values used here is not measurable.
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    fn pick<'a, T>(&mut self, from: &'a [T]) -> &'a T {
        &from[self.below(from.len() as u64) as usize]
    }
}

/// Damage `source` one way, returning which way and the result.
///
/// Every kind is total: it produces bytes for any non-empty input, including the
/// empty document that cutting at offset zero gives, which is itself a case worth
/// asking about.
fn mutate(rng: &mut Rng, source: &[u8]) -> (usize, Vec<u8>) {
    assert!(!source.is_empty(), "there is nothing to mutate");
    let kind = rng.below(KIND_NAMES.len() as u64) as usize;
    let at = rng.below(source.len() as u64) as usize;
    let mut bytes = source.to_vec();
    match kind {
        0 => {
            let bit = 1u8 << rng.below(8);
            bytes[at] ^= bit;
        }
        1 => bytes[at] = *rng.pick(INTERESTING),
        2 => bytes.truncate(at),
        3 => bytes.insert(at, *rng.pick(INTERESTING)),
        4 => {
            bytes.remove(at);
        }
        _ => {
            let run = 1 + rng.below(4) as usize;
            let end = (at + run).min(bytes.len());
            let mut copy = bytes[..end].to_vec();
            copy.extend_from_slice(&bytes[at..end]);
            copy.extend_from_slice(&bytes[end..]);
            bytes = copy;
        }
    }
    (kind, bytes)
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_parsing")
}

/// The documents that must parse, in a fixed order.
///
/// Sorted, because `read_dir` returns whatever order the filesystem keeps and this
/// harness seeds each fixture's generator from its position in the list.
/// Alphabetical on NTFS, effectively hash order on ext4: without the sort the same
/// commit would mutate different documents on a laptop and in CI, and a failure
/// found in one would not reproduce in the other.
fn y_fixtures() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(fixtures_dir())
        .expect("tests/fixtures/test_parsing is missing")
        .map(|entry| entry.expect("cannot read a directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("y_") && name.ends_with(".json"))
        })
        .collect();
    found.sort();
    found
}

/// The caret renderer slices the failing line and puts a marker at `column`, so
/// the column has to name a character on that line or the one position after it.
/// Returns how many characters the line holding `offset` has.
fn characters_on_line(bytes: &[u8], offset: usize) -> usize {
    let end = offset.min(bytes.len());
    let start = bytes[..end]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |at| at + 1);
    let stop = bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |at| start + at);
    bytes[start..stop]
        .iter()
        .filter(|byte| (**byte & 0xC0) != 0x80)
        .count()
}

#[test]
fn splitmix64_matches_its_published_test_vector() {
    let mut rng = Rng::new(0);
    let expected = [
        0xE220_A839_7B1D_CDAF,
        0x6E78_9E6A_A1B9_65F4,
        0x06C4_5D18_8009_454F,
        0xF88B_B8A8_724C_81EC,
    ];
    for (step, want) in expected.iter().enumerate() {
        assert_eq!(rng.next_u64(), *want, "SplitMix64 diverges at step {step}");
    }
}

#[test]
fn every_mutation_is_answered_and_never_a_crash() {
    let files = y_fixtures();
    assert_eq!(files.len(), Y_FIXTURES, "the corpus changed size");
    let mut census = [0usize; KIND_NAMES.len()];
    let mut parsed = 0usize;
    let mut rejected = 0usize;
    for (index, path) in files.iter().enumerate() {
        let name = path
            .file_name()
            .and_then(|part| part.to_str())
            .expect("a fixture name is UTF-8")
            .to_string();
        let source = fs::read(path).expect("cannot read a fixture");
        let mut rng = Rng::new(index as u64);
        for round in 0..MUTATIONS {
            let (kind, bytes) = mutate(&mut rng, &source);
            census[kind] += 1;
            let label = format!("{name}, round {round}, {}", KIND_NAMES[kind]);
            let Ok(answer) = panic::catch_unwind(AssertUnwindSafe(|| parse(&bytes))) else {
                panic!("{label} made the parser panic on {}", bytes.escape_ascii());
            };
            match answer {
                Ok(value) => {
                    parsed += 1;
                    let text = to_string(&value, Style::Compact);
                    let reparsed = parse(text.as_bytes()).unwrap_or_else(|error| {
                        panic!("{label} produced {text}, which will not parse: {error}")
                    });
                    assert!(
                        reparsed.identical(&value),
                        "{label} produced {text}, which is not a fixed point"
                    );
                }
                Err(error) => {
                    rejected += 1;
                    assert!(
                        error.offset() <= bytes.len(),
                        "{label}: offset {} is past the end of {} bytes",
                        error.offset(),
                        bytes.len()
                    );
                    let newlines = bytes[..error.offset()]
                        .iter()
                        .filter(|byte| **byte == b'\n')
                        .count();
                    assert_eq!(
                        error.line(),
                        newlines + 1,
                        "{label}: line {} after {newlines} newlines",
                        error.line()
                    );
                    let room = characters_on_line(&bytes, error.offset()) + 1;
                    assert!(
                        error.column() >= 1 && error.column() <= room,
                        "{label}: column {} on a line with room for {room}",
                        error.column()
                    );
                }
            }
        }
    }
    println!(
        "MUTANTS: {} answered, {parsed} parsed, {rejected} rejected",
        parsed + rejected
    );
    for (kind, count) in census.iter().enumerate() {
        println!("  {count:6}  {}", KIND_NAMES[kind]);
    }
    assert_eq!(
        parsed + rejected,
        Y_FIXTURES * MUTATIONS,
        "a mutant went unanswered"
    );
    assert!(
        parsed > 0,
        "no mutant parsed, so nothing tested the accepting path"
    );
    assert!(
        rejected > 0,
        "no mutant was rejected, so nothing was really damaged"
    );
    for (kind, count) in census.iter().enumerate() {
        assert!(*count > 0, "{} never happened", KIND_NAMES[kind]);
    }
}

#[test]
fn a_document_cut_at_every_offset_is_answered() {
    // Exhaustive, not sampled: a prefix is the failure a stream reader meets
    // first, and every offset in this document is a different way to run out of
    // input -- between a key and its colon, inside an escape, after a minus sign,
    // after the plus in an exponent. The empty prefix is included, and is an error
    // as well: a stream with no documents in it is the reader's business, not this
    // function's.
    let source = br#"{"a":[1,2,{"b":"c\nd"},true,null],"e":-1.5e+3}"#;
    parse(source).expect("the whole document is valid");
    let mut rejected = 0usize;
    for cut in 0..source.len() {
        let bytes = &source[..cut];
        let Ok(answer) = panic::catch_unwind(AssertUnwindSafe(|| parse(bytes))) else {
            panic!("cutting at {cut} made the parser panic");
        };
        if let Err(error) = answer {
            rejected += 1;
            assert!(
                error.offset() <= cut,
                "cutting at {cut} reported offset {}",
                error.offset()
            );
        }
    }
    println!("PREFIXES: {} cut, {rejected} rejected", source.len());
    assert_eq!(rejected, source.len(), "a prefix of one document parsed");
}

#[test]
fn a_mutant_replays_from_its_seed() {
    let source = br#"{"a":[1,2,3],"b":"c","d":true}"#;
    let run = || {
        let mut rng = Rng::new(7);
        (0..32)
            .map(|_| mutate(&mut rng, source))
            .collect::<Vec<_>>()
    };
    let first = run();
    assert_eq!(first, run(), "the same seed produced different mutants");
    let distinct: std::collections::BTreeSet<&Vec<u8>> =
        first.iter().map(|(_, bytes)| bytes).collect();
    assert!(
        distinct.len() >= 16,
        "32 mutations produced only {} distinct documents",
        distinct.len()
    );
}
