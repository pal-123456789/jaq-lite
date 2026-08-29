//! The filter language as one table, read through the public API only.
//!
//! `src/query.rs` already tests this language from the inside, twenty-one tests
//! deep, and this file does not repeat them. It is here for three things none of
//! them can do.
//!
//! The first is being readable as a specification. Anyone who wants to know what
//! this tool's query language does currently has to reconstruct it from
//! twenty-one assertions spread over two hundred lines. Below it is forty rows of
//! filter, input and outcome, and the rows are checked on every run, so this
//! specification cannot drift away from the implementation the way a prose one
//! does.
//!
//! The second is the seam. The unit tests compare against a `Vec<Value>`; a
//! caller gets text. Every value the table emits is serialized and parsed back
//! here and compared with `Value::identical`, which asserts something nothing
//! asserted before: that the query layer's output lies inside the parser's input.
//! Both halves were tested and the join between them was not.
//!
//! The third is that this file is a dependent rather than a module. It can reach
//! `Filter`, `FilterError`, `EvalError` and their accessors and nothing else --
//! `MAX_DEPTH` is private, so the only way anything out here can learn the
//! nesting cap is to read it back out of the error, which is why the limit is a
//! field on the variant rather than a number in a message.
//!
//! # What the coverage test does and does not guarantee
//!
//! `compile_tag` and `eval_tag` match on the two public error enums with no
//! wildcard arm, so a new way for a filter to fail cannot be added to the library
//! without this file refusing to compile. That much the compiler enforces. It
//! does not enforce that the new arm gets a row: adding a tag and a name to
//! `EXPECTED_TAGS` together would satisfy the assertion without exercising
//! anything. That is worth saying plainly, because a coverage check believed to be
//! complete and not being so is worse than one whose edges are written down.

use jaq_lite::{EvalError, Filter, FilterErrorKind, Style, Value};

/// What a row expects to happen.
enum Outcome {
    /// It compiles, it runs, and it emits exactly these values as compact JSON.
    /// An empty slice means the filter produced no output at all, which is a
    /// success and not a failure.
    Yields(&'static [&'static str]),
    /// It compiles and then the value refuses the question. The text is
    /// `EvalError`'s `Display`, which is jq 1.8.1's wording character for
    /// character.
    Fails(&'static str),
    /// It does not compile. The text is the *kind*'s `Display` alone and not the
    /// whole message, because the position belongs to the input rather than to the
    /// language; the two full messages, column and all, are pinned by
    /// `the_public_surface_is_enough_to_report_a_failure` below.
    Rejected(&'static str),
}

use Outcome::{Fails, Rejected, Yields};

/// The language. Filter, input, outcome.
const ROWS: &[(&str, &str, Outcome)] = &[
    // The identity, and the two ways of writing it.
    (".", r#"{"a":[1,2]}"#, Yields(&[r#"{"a":[1,2]}"#])),
    ("", "1", Yields(&["1"])),
    ("   ", "[]", Yields(&["[]"])),
    // Fields, by dot, by quoted name and by bracket.
    (".a", r#"{"a":1}"#, Yields(&["1"])),
    (".a.b", r#"{"a":{"b":2}}"#, Yields(&["2"])),
    (".missing", "{}", Yields(&["null"])),
    (r#"."a b""#, r#"{"a b":7}"#, Yields(&["7"])),
    (r#".["a b"]"#, r#"{"a b":7}"#, Yields(&["7"])),
    (".a.b.c", "null", Yields(&["null"])),
    // Indices count from either end and run off both of them into null.
    (".[0]", "[1,2,3]", Yields(&["1"])),
    (".[-1]", "[1,2,3]", Yields(&["3"])),
    (".[9]", "[1,2,3]", Yields(&["null"])),
    (".a[0]", r#"{"a":[10,20]}"#, Yields(&["10"])),
    // Iteration yields elements, then values, then nothing.
    (".[]", "[1,2]", Yields(&["1", "2"])),
    (".[]", r#"{"b":1,"a":2}"#, Yields(&["1", "2"])),
    (".[]", "[]", Yields(&[])),
    (".a[]", r#"{"a":[1,2]}"#, Yields(&["1", "2"])),
    // Streams: `|` feeds, `,` concatenates, `,` binds tighter, `()` groups.
    (".[] | .id", r#"[{"id":1},{"id":2}]"#, Yields(&["1", "2"])),
    (".a, .b", r#"{"a":1,"b":2}"#, Yields(&["1", "2"])),
    (
        ".a, .b | .x",
        r#"{"a":{"x":1},"b":{"x":2}}"#,
        Yields(&["1", "2"]),
    ),
    (
        "(.a | .b), .c",
        r#"{"a":{"b":1},"c":2}"#,
        Yields(&["1", "2"]),
    ),
    // `?` forgives the step it follows and no more. Row four is jq 1.8.1's
    // answer, not a guess: the `?` marks `.b`, the failure happens at `.a`, and
    // parentheses are how a whole path is caught instead.
    (".a?", "1", Yields(&[])),
    (".a?", r#"{"a":1}"#, Yields(&["1"])),
    ("(.a.b)?", "1", Yields(&[])),
    (
        ".a.b?",
        "1",
        Fails(r#"Cannot index number with string "a""#),
    ),
    // The three ways a value refuses.
    (".a", "[1]", Fails(r#"Cannot index array with string "a""#)),
    (".[0]", "{}", Fails("Cannot index object with number")),
    (".[]", "null", Fails("Cannot iterate over null (null)")),
    // Fourteen characters print whole and fifteen come back as eleven and three
    // dots, which is jq formatting into a fifteen-byte buffer.
    (
        ".[]",
        r#""aaaaaaaaaaaa""#,
        Fails(r#"Cannot iterate over string ("aaaaaaaaaaaa")"#),
    ),
    (
        ".[]",
        "123456789012345",
        Fails("Cannot iterate over number (12345678901...)"),
    ),
    // One failure ends the whole stream; the second element is never reached.
    (
        ".[] | .a",
        "[{},1]",
        Fails(r#"Cannot index number with string "a""#),
    ),
    // Every way the language can refuse the program itself, except the nesting
    // cap, whose filter is too long to write out and is built below.
    ("|", "null", Rejected("`|` cannot appear here")),
    (".a |", "null", Rejected("the filter ends too soon")),
    (
        ".a.",
        "null",
        Rejected("expected a field name or `[` after `.`"),
    ),
    (".[0", "null", Rejected("expected `]`")),
    ("(.a", "null", Rejected("expected `)`")),
    (
        ".[x]",
        "null",
        Rejected("expected a whole number or a quoted name"),
    ),
    (".a %", "null", Rejected("`%` has no meaning here")),
    (
        r#".["unclosed"#,
        "null",
        Rejected("this is not a well-formed string"),
    ),
    // A bare name is a function call, and no functions exist yet, so it has to
    // fail rather than quietly do nothing.
    ("length", "null", Rejected("`length` cannot appear here")),
];

/// How many rows the table above has, so that deleting one is a failure.
const ROW_COUNT: usize = 40;

/// The row whose filter is 131 characters of parentheses, built rather than
/// written out. Sixty-five of them clears a cap of sixty-four.
fn built_rows() -> Vec<(String, &'static str, Outcome)> {
    vec![(
        "(".repeat(65) + "." + &")".repeat(65),
        "null",
        Rejected("parentheses nested deeper than 64"),
    )]
}

/// Every name `compile_tag` and `eval_tag` can return, sorted.
const EXPECTED_TAGS: &[&str] = &[
    "cannot index by name",
    "cannot index by number",
    "cannot iterate",
    "depth limit exceeded",
    "expected a field name",
    "expected specific text",
    "invalid index",
    "invalid string",
    "unexpected byte",
    "unexpected end",
    "unexpected token",
];

/// Name the way a filter failed to compile.
///
/// No wildcard arm, deliberately: a new variant on the public enum stops this
/// file compiling until someone decides what to call it, and the test below then
/// asks the table to exercise it.
fn compile_tag(kind: &FilterErrorKind) -> &'static str {
    match kind {
        FilterErrorKind::UnexpectedByte { .. } => "unexpected byte",
        FilterErrorKind::UnexpectedEnd => "unexpected end",
        FilterErrorKind::Unexpected { .. } => "unexpected token",
        FilterErrorKind::ExpectedFieldName => "expected a field name",
        FilterErrorKind::Expected { .. } => "expected specific text",
        FilterErrorKind::InvalidString => "invalid string",
        FilterErrorKind::InvalidIndex => "invalid index",
        FilterErrorKind::DepthLimitExceeded { .. } => "depth limit exceeded",
    }
}

/// Name the way a value refused, under the same rule.
fn eval_tag(error: &EvalError) -> &'static str {
    match error {
        EvalError::NotIndexableByName { .. } => "cannot index by name",
        EvalError::NotIndexableByNumber { .. } => "cannot index by number",
        EvalError::NotIterable { .. } => "cannot iterate",
    }
}

/// A row's input, which is a test bug rather than a failure if it is not JSON.
fn parse_json(input: &str) -> Value {
    jaq_lite::parse(input.as_bytes())
        .unwrap_or_else(|error| panic!("the row input `{input}` is not JSON: {error}"))
}

/// Check one row, and name the failure it produced if it produced one.
fn check(filter: &str, input: &str, expected: &Outcome) -> Option<&'static str> {
    let compiled = match Filter::compile(filter) {
        Ok(compiled) => compiled,
        Err(error) => {
            let Rejected(text) = expected else {
                panic!(
                    "`{filter}` was expected to compile and was rejected: {}",
                    error.kind()
                );
            };
            assert_eq!(
                error.kind().to_string(),
                *text,
                "`{filter}` was rejected for the wrong reason"
            );
            return Some(compile_tag(error.kind()));
        }
    };
    let value = parse_json(input);
    match (compiled.run(&value), expected) {
        (Ok(values), Yields(want)) => {
            let got: Vec<String> = values
                .iter()
                .map(|value| jaq_lite::to_string(value, Style::Compact))
                .collect();
            assert_eq!(
                got.as_slice(),
                *want,
                "`{filter}` on `{input}` emitted the wrong stream"
            );
            None
        }
        (Err(error), Fails(text)) => {
            assert_eq!(
                error.to_string(),
                *text,
                "`{filter}` on `{input}` failed for the wrong reason"
            );
            Some(eval_tag(&error))
        }
        (Ok(values), _) => panic!(
            "`{filter}` on `{input}` was expected to fail and emitted {} values",
            values.len()
        ),
        (Err(error), _) => {
            panic!("`{filter}` on `{input}` was expected to succeed and failed: {error}")
        }
    }
}

#[test]
fn every_row_behaves_as_the_table_says() {
    assert_eq!(
        ROWS.len(),
        ROW_COUNT,
        "a row was added or removed without saying so here"
    );
    for row in ROWS {
        let _ = check(row.0, row.1, &row.2);
    }
    let built = built_rows();
    for row in &built {
        let _ = check(&row.0, row.1, &row.2);
    }
    println!(
        "QUERY ROWS: {} written out, {} built",
        ROWS.len(),
        built.len()
    );
}

#[test]
fn the_table_exercises_every_way_a_filter_can_fail() {
    let mut seen: Vec<&str> = ROWS
        .iter()
        .filter_map(|row| check(row.0, row.1, &row.2))
        .collect();
    seen.extend(
        built_rows()
            .iter()
            .filter_map(|row| check(&row.0, row.1, &row.2)),
    );
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.as_slice(),
        EXPECTED_TAGS,
        "the table no longer exercises one failure for each name, or exercises one it has no name for"
    );
    println!("QUERY FAILURES: {} named, all exercised", seen.len());
}

#[test]
fn every_value_a_filter_emits_is_json() {
    let mut checked = 0;
    for row in ROWS {
        if !matches!(row.2, Yields(_)) {
            continue;
        }
        let value = parse_json(row.1);
        let outputs = Filter::compile(row.0)
            .expect("a row that yields should compile")
            .run(&value)
            .expect("a row that yields should run");
        for output in &outputs {
            let text = jaq_lite::to_string(output, Style::Compact);
            let again = jaq_lite::parse(text.as_bytes()).unwrap_or_else(|error| {
                panic!("`{}` emitted `{text}`, which is not JSON: {error}", row.0)
            });
            assert!(
                again.identical(output),
                "`{}` emitted `{text}`, which parses back as a different value",
                row.0
            );
        }
        checked += outputs.len();
    }
    assert!(
        checked >= 25,
        "only {checked} values went round, so the table has stopped emitting things"
    );
    println!("QUERY ROUND TRIP: {checked} values");
}

#[test]
fn the_public_surface_is_enough_to_report_a_failure() {
    // A caller who wants to draw a caret needs the position, and gets it without
    // reaching inside anything: the byte offset for slicing, the line and column
    // for printing, and the kind for deciding what to say.
    let error = Filter::compile(".a %").expect_err("`%` means nothing in a filter");
    assert_eq!(error.offset(), 3);
    assert_eq!((error.line(), error.column()), (1, 4));
    assert_eq!(
        error.to_string(),
        "filter, column 4: `%` has no meaning here"
    );
    let across = Filter::compile(".a |\n.b %").expect_err("the same, one line down");
    assert_eq!((across.line(), across.column()), (2, 4));
    assert_eq!(
        across.to_string(),
        "filter, line 2, column 4: `%` has no meaning here"
    );
    // The cap itself is private. Out here the only way to learn it is to be told
    // by the error, which is the whole reason the limit is a field rather than
    // prose inside a message.
    let deep = "(".repeat(200) + "." + &")".repeat(200);
    assert_eq!(
        *Filter::compile(&deep)
            .expect_err("two hundred is deeper than the cap")
            .kind(),
        FilterErrorKind::DepthLimitExceeded { limit: 64 }
    );
}

#[test]
fn one_compiled_filter_serves_many_documents() {
    // What `main.rs` actually does with a stream of documents, which no test did
    // before: compile once, run many times, and get nothing back from the one
    // that has nothing to give.
    let filter = Filter::compile(".items[] | .name").expect("the filter should compile");
    let mut names = Vec::new();
    for input in [
        r#"{"items":[{"name":"a"},{"name":"b"}]}"#,
        r#"{"items":[]}"#,
        r#"{"items":[{"name":"c"}]}"#,
    ] {
        let value = parse_json(input);
        for output in filter.run(&value).expect("the filter should run") {
            names.push(jaq_lite::to_string(&output, Style::Compact));
        }
    }
    let want: &[&str] = &[r#""a""#, r#""b""#, r#""c""#];
    assert_eq!(names.as_slice(), want);
    // `Filter` is `Clone` and nothing exercised it. A clone is the same program,
    // which is the only promise cloning one makes.
    let copy = filter.clone();
    let value = parse_json(r#"{"items":[{"name":"z"}]}"#);
    assert_eq!(copy.run(&value).expect("the clone should run").len(), 1);
}
