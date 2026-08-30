//! Documents that real tools emitted, and what has to stay true about them.
//!
//! `tests/fixtures/test_parsing` is JSONTestSuite: files chosen to break
//! parsers, several of which are not valid UTF-8 and two of which are a hundred
//! thousand levels deep. Passing it says nothing about whether this parser
//! handles the JSON that comes out of a build system, because no build system
//! emits anything like it.
//!
//! `tests/fixtures/real_world` is the other half of that question. Every file in
//! it was produced by a tool -- `cargo metadata`, rustc's
//! `--message-format=json`, and PowerShell's `ConvertTo-Json` -- and kept byte
//! for byte, with only the machine substituted out. The commands and the
//! substitutions are in `PROVENANCE.md` beside them.
//!
//! Three producers rather than one, because each has habits the others do not.
//! cargo and rustc write a whole document on one line with no whitespace in it,
//! which makes a byte-for-byte reprint testable. PowerShell indents four spaces,
//! ends its lines with CRLF, and escapes the apostrophe as an ASCII `\u` escape
//! even though it never has to, which is what turns the round trip into a real
//! question about strings rather than a formality.
//!
//! To see what it measured:
//!
//! ```text
//! cargo test --test real_world -- --nocapture --test-threads=1
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use jaq_lite::{Style, parse, to_string};

/// How many documents the corpus holds.
///
/// Checked for the same reason the conformance corpus size is checked: a partial
/// or damaged checkout should fail here rather than pass quietly over whichever
/// files happen to be present.
const DOCUMENTS: usize = 8;

/// The documents whose producer emitted them with no whitespace at all.
///
/// For these the compact reprint has to equal the input exactly, which is the
/// strongest thing this harness can say -- not "an equivalent document" but "the
/// same bytes cargo and rustc wrote". The PowerShell documents arrive indented,
/// so their bytes cannot come back and they are not listed here.
///
/// Worth knowing what this does not prove: these four also contain no escape
/// sequence of any kind, because their producer only escapes what it has to. So
/// they pin whitespace, key order and number spelling, and say nothing about
/// string escapes. That is what the third test is for.
const BYTE_EXACT: [&str; 4] = [
    "diagnostic1.json",
    "diagnostic2.json",
    "diagnostic3.json",
    "metadata.json",
];

/// How many apostrophes `culture.json` spells as an escape instead of as a
/// bare `'`.
///
/// The count is here to give the next assertion something to bite on. "The
/// output contains no `\u` escape" is satisfied trivially by a document that had
/// none going in, so the input is required to be full of them first. Every
/// escape of that form in this file is an apostrophe and there are none of any
/// other kind, which is what lets the assertion look for the prefix alone.
const APOSTROPHE_ESCAPES: usize = 104;

/// How many forward slashes `culture.json` writes with a backslash in front of
/// them.
///
/// A second escape family, and the one jq is best known for not re-emitting: a
/// solidus never needs escaping, PowerShell escapes it anyway, and both tools
/// print it bare. Two families rather than one is what makes the rule below a
/// rule about strings instead of a fact about one character.
const SOLIDUS_ESCAPES: usize = 48;

/// The raw non-ASCII bytes in `culture.json`: the currency sign, the per-mille
/// sign and the rest of the invariant culture's symbols.
///
/// They arrive unescaped and must leave unescaped, and the count must match on
/// both sides. Re-encoding them as `\u` escapes would still be valid JSON and
/// would still round trip, so only a byte count catches it.
const RAW_NON_ASCII: usize = 31;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("real_world")
}

/// The documents in an order the filesystem does not get to choose.
///
/// `read_dir` yields entries alphabetically on NTFS and in hash order on ext4.
/// Sorting is what makes a failure name the same file on a laptop and on a CI
/// runner.
fn sorted_documents() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(corpus_dir())
        .expect("tests/fixtures/real_world is missing; see its PROVENANCE.md")
        .map(|entry| entry.expect("unreadable directory entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Parse, print compact, parse that, print again, and require the two prints to
/// agree.
///
/// The second parse is the part that matters. A serializer can emit something
/// this parser accepts and something no other parser accepts; making the output
/// go back through the front door is what rules that out.
#[test]
fn every_document_survives_a_compact_reprint_and_a_second_parse() {
    let documents = sorted_documents();
    assert_eq!(
        documents.len(),
        DOCUMENTS,
        "the corpus should hold {DOCUMENTS} documents and holds {}; if one was added \
         or removed on purpose, move the constant and say so in PROVENANCE.md",
        documents.len()
    );

    for path in &documents {
        let bytes = read(path);
        let value = parse(&bytes).unwrap_or_else(|err| {
            panic!(
                "{} came out of a real tool and has to parse: {err}",
                name(path)
            )
        });
        let once = to_string(&value, Style::Compact);
        let reparsed = parse(once.as_bytes()).unwrap_or_else(|err| {
            panic!(
                "{}: this serializer's own output has to parse: {err}",
                name(path)
            )
        });
        assert_eq!(
            once,
            to_string(&reparsed, Style::Compact),
            "{} does not survive a second trip, so one of the two directions is losing \
             or inventing something",
            name(path)
        );
    }

    println!("REAL WORLD: {DOCUMENTS} documents parsed, reprinted compact and reparsed");
}

/// The documents that arrived compact come back as the same bytes.
#[test]
fn a_document_a_tool_emitted_compact_comes_back_byte_for_byte() {
    for file in BYTE_EXACT {
        let bytes = read(&corpus_dir().join(file));
        let text = std::str::from_utf8(&bytes)
            .unwrap_or_else(|err| panic!("{file} should be valid UTF-8: {err}"));
        let value = parse(&bytes).unwrap_or_else(|err| panic!("{file} has to parse: {err}"));
        assert_eq!(
            to_string(&value, Style::Compact),
            text,
            "{file} arrived from its producer with no whitespace in it, so a compact \
             reprint has to be the same bytes. A difference here is a serializer \
             change; if it is a deliberate one, this list and the README's account of \
             number and string handling move in the same commit"
        );
    }

    println!(
        "REAL WORLD: {} documents reprinted to their producer's own bytes",
        BYTE_EXACT.len()
    );
}

/// Escapes are decoded, raw UTF-8 is left alone, and the asymmetry with numbers
/// is deliberate.
///
/// Numbers are re-emitted from the bytes that were read, so `1e2` stays `1e2`.
/// Strings are not: they are decoded on the way in and minimally re-escaped on
/// the way out, so an escaped apostrophe comes back as a bare `'`. Both rules
/// are jq's, and a document a real tool wrote is what makes the difference
/// visible: PowerShell escapes every apostrophe and every solidus it emits, and
/// this parser keeps the meaning rather than the spelling.
#[test]
fn escapes_are_decoded_and_raw_utf8_is_passed_through_untouched() {
    let bytes = read(&corpus_dir().join("culture.json"));
    let text = std::str::from_utf8(&bytes).expect("culture.json should be valid UTF-8");

    assert_eq!(
        text.matches("\\u0027").count(),
        APOSTROPHE_ESCAPES,
        "culture.json should carry {APOSTROPHE_ESCAPES} escaped apostrophes; without \
         them the assertion below proves nothing"
    );
    assert_eq!(
        text.matches("\\/").count(),
        SOLIDUS_ESCAPES,
        "culture.json should carry {SOLIDUS_ESCAPES} escaped solidi"
    );
    assert_eq!(
        bytes.iter().filter(|byte| **byte >= 0x80).count(),
        RAW_NON_ASCII,
        "culture.json should carry {RAW_NON_ASCII} raw non-ASCII bytes"
    );

    let printed = to_string(
        &parse(&bytes).expect("culture.json has to parse"),
        Style::Compact,
    );

    assert!(
        !printed.contains("\\u"),
        "an escape survived into the output; strings are supposed to be re-escaped \
         minimally, the way jq does it, and a `\\u` sequence means the input's \
         spelling was kept instead of its meaning"
    );
    assert!(
        !printed.contains("\\/"),
        "a solidus came back escaped; it never needed to be, and jq prints it bare"
    );
    assert!(
        printed.contains('\''),
        "the escaped apostrophes decoded to nothing recognisable"
    );
    assert_eq!(
        printed.bytes().filter(|byte| *byte >= 0x80).count(),
        RAW_NON_ASCII,
        "the raw non-ASCII bytes changed count on the way out, so they were escaped, \
         dropped or re-encoded rather than passed through"
    );

    println!(
        "REAL WORLD: {APOSTROPHE_ESCAPES} apostrophe and {SOLIDUS_ESCAPES} solidus escapes \
         decoded, {RAW_NON_ASCII} raw bytes untouched"
    );
}
