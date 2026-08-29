//! One measured figure, and a plain account of what it is not.
//!
//! `criterion` is the crate this file replaces, and the replacement is not a
//! smaller `criterion`: it is one `std::time::Instant` around one workload. What
//! that crate does and this does not is worth naming rather than glossing over.
//! There are no percentiles here, no outlier rejection, no warm-up policy, no
//! statistical comparison between runs and no verdict on whether two figures
//! differ. What is here is a number a reader can reproduce with one command, and
//! a floor that fails the build if throughput collapses.
//!
//! Two things are borrowed from how a benchmark harness has to work, because
//! without them the number would be a lie in an optimized build.
//! `std::hint::black_box` stands between the optimizer and a result that is
//! otherwise dead, since a parse whose value is dropped unread can legally be
//! deleted outright. And the timed region includes that drop, because a parse
//! whose result is leaked is not a parse anyone performs.
//!
//! The workload is deliberately not the conformance corpus. Those 95 documents
//! total 1190 bytes between them, so timing them measures the cost of calling a
//! function 95 times. This file builds one document of rather more than a
//! megabyte instead, the same bytes on every machine, holding every branch of the
//! value model: objects, arrays, three spellings of number, an escaped quote, a
//! `\u` escape, a control-character escape, `true` and `null`.
//!
//! The loop is bounded by time rather than by a round count, so one source file
//! yields a usable sample in an unoptimized build and in an optimized one. A
//! floor is asserted and the figure never is: a floor catches the regression that
//! changes the shape of the algorithm, while asserting a figure would fail on a
//! busy laptop and prove nothing on a fast one.

use jaq_lite::{Style, parse, to_string};
use std::hint::black_box;
use std::time::{Duration, Instant};

/// How long each measurement runs for. Not how long the work takes: the loop
/// repeats the same document until this much time has gone by.
const BUDGET: Duration = Duration::from_millis(300);

/// A ceiling on the sample, so a machine fast enough to make `BUDGET` cheap still
/// finishes. Reaching it is not a failure.
const MAX_ROUNDS: u32 = 100_000;

/// Records in the generated document.
const RECORDS: usize = 8_000;

/// The smallest document worth timing, so that per-call overhead is not what gets
/// measured.
const MIN_BYTES: usize = 1 << 20;

/// The floor for this build, in whole MiB/s.
///
/// An unoptimized build is the slowest thing that has to pass, and it is roughly
/// an order of magnitude above one; the optimized floor is set the same way, far
/// enough below the measured figure that no machine trips it and near enough that
/// a collapse does.
fn committed_floor() -> usize {
    if cfg!(debug_assertions) { 1 } else { 20 }
}

/// Read a floor from the environment, which may raise it but never lower it.
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

/// The workload: one document, built the same way on every machine.
///
/// No random number generator and no input from the environment, so two runs
/// anywhere time the same bytes. The harness formats integers to build its input;
/// the program under test never formats a number at all, which is the substance of
/// entry 7 of `STDLIB.md`.
fn workload() -> String {
    let mut out = String::with_capacity(MIN_BYTES * 2);
    out.push('[');
    for index in 0..RECORDS {
        if index > 0 {
            out.push(',');
        }
        let count = index.to_string();
        out.push_str(r#"{"id":"#);
        out.push_str(&count);
        out.push_str(r#","name":"row "#);
        out.push_str(&count);
        out.push_str(r#"","tags":["alpha","b\"c","caf\u00e9"],"ratio":"#);
        out.push_str(&count);
        out.push_str(r#".5e-2,"deep":{"a":[1,2,3],"b":{"c":"line\nbreak"}},"ok":true,"nil":null}"#);
    }
    out.push(']');
    out
}

/// MiB/s from the bytes moved and the time it took.
fn rate(bytes: usize, rounds: u32, elapsed: Duration) -> f64 {
    let moved = bytes as f64 * f64::from(rounds);
    moved / elapsed.as_secs_f64() / (1024.0 * 1024.0)
}

/// Run `work` until `BUDGET` is spent, at least once, and report the sample.
fn measure(mut work: impl FnMut()) -> (u32, Duration) {
    let start = Instant::now();
    let mut rounds = 0u32;
    loop {
        work();
        rounds += 1;
        if start.elapsed() >= BUDGET || rounds >= MAX_ROUNDS {
            break;
        }
    }
    (rounds, start.elapsed())
}

/// How this build is named in the printed lines.
fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[test]
fn the_workload_is_one_fixed_document() {
    let document = workload();
    assert_eq!(document, workload(), "two builds of the workload differ");
    assert!(
        document.len() >= MIN_BYTES,
        "the workload is {} bytes and {MIN_BYTES} is the smallest worth timing",
        document.len()
    );
    let value = parse(document.as_bytes()).expect("the workload is valid JSON");
    let printed = to_string(&value, Style::Compact);
    let again = parse(printed.as_bytes()).expect("what the serializer wrote parses");
    assert!(
        again.identical(&value),
        "a megabyte survives parse and to_string but comes back different"
    );
    println!(
        "WORKLOAD: {} bytes, {RECORDS} records, serialized {} bytes, round trip identical",
        document.len(),
        printed.len()
    );
}

#[test]
fn parse_throughput_clears_its_floor() {
    let document = workload();
    let bytes = document.as_bytes();
    let (rounds, elapsed) = measure(|| {
        black_box(parse(black_box(bytes)).expect("the workload is valid JSON"));
    });
    let mibs = rate(document.len(), rounds, elapsed);
    let want = floor("JAQ_PARSE_FLOOR", committed_floor());
    println!(
        "PARSE: {mibs:.1} MiB/s ({} bytes x {rounds} rounds in {:.0} ms, {} build, floor {want})",
        document.len(),
        elapsed.as_secs_f64() * 1000.0,
        profile()
    );
    assert!(
        mibs >= want as f64,
        "parsing ran at {mibs:.1} MiB/s and the floor is {want}"
    );
}

#[test]
fn serialize_throughput_clears_its_floor() {
    let document = workload();
    let value = parse(document.as_bytes()).expect("the workload is valid JSON");
    let written = to_string(&value, Style::Compact).len();
    let (rounds, elapsed) = measure(|| {
        black_box(to_string(black_box(&value), Style::Compact));
    });
    let mibs = rate(written, rounds, elapsed);
    let want = floor("JAQ_SERIALIZE_FLOOR", committed_floor());
    println!(
        "SERIALIZE: {mibs:.1} MiB/s ({written} bytes x {rounds} rounds in {:.0} ms, {} build, floor {want})",
        elapsed.as_secs_f64() * 1000.0,
        profile()
    );
    assert!(
        mibs >= want as f64,
        "serializing ran at {mibs:.1} MiB/s and the floor is {want}"
    );
}
