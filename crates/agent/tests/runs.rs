//! The `agent_runs` prune reads `created_at` through an index (SPEC-043 A19, A20, issue 363).
#![allow(clippy::expect_used)]

use deck_streak_kernel::Db;

/// The index migration `004302` creates, named like the ledger's other `*_by_*` indexes.
const INDEX: &str = "agent_runs_by_created_at";

/// The one place this file writes the prune statement out: the tested statement and the text
/// of its `EXPLAIN QUERY PLAN` are both made from it, so neither can drift from the other.
macro_rules! prune_statement {
    () => {
        "DELETE FROM agent_runs WHERE created_at < ?1"
    };
}

/// The statement `AgentRuns::prune_before` runs, as `crates/agent/src/runs.rs` states it; the test
/// below reads the source and asserts it holds this exact text.
const PRUNE: &str = prune_statement!();

/// The table the prune deletes from, named apart so the fixtures below never write the statement.
const TABLE: &str = "agent_runs";

/// `source` lower-cased with every run of whitespace collapsed to one space, so a second
/// statement cannot hide behind a different case or a wider gap.
fn normalized(source: &str) -> String {
    source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The byte ranges of every comment in `source`, in every form: `//`, `///`, `//!`, and `/* */`
/// with its doc forms, nested blocks included. A marker inside a string, a raw string or a
/// character literal opens none, so a prune written after one is never hidden.
fn comment_spans(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if !source.is_char_boundary(at) {
            at += 1;
            continue;
        }
        let rest = &source[at..];
        if rest.starts_with("//") {
            let end = rest.find('\n').map_or(bytes.len(), |n| at + n);
            spans.push((at, end));
            at = end;
        } else if rest.starts_with("/*") {
            let (mut depth, mut end) = (0_usize, at);
            while end < bytes.len() {
                if bytes[end..].starts_with(b"/*") {
                    depth += 1;
                    end += 2;
                } else if bytes[end..].starts_with(b"*/") {
                    depth -= 1;
                    end += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    end += 1;
                }
            }
            spans.push((at, end));
            at = end;
        } else if bytes[at] == b'r' && raw_string_len(rest).is_some() {
            at += raw_string_len(rest).unwrap_or(1);
        } else if bytes[at] == b'"' {
            at += 1;
            while at < bytes.len() && bytes[at] != b'"' {
                at += if bytes[at] == b'\\' { 2 } else { 1 };
            }
            at += 1;
        } else if bytes[at] == b'\'' {
            at += char_literal_len(rest);
        } else {
            at += 1;
        }
    }
    spans
}

/// The length of the raw string `rest` opens (`r"..."`, `r#"..."#`), or `None` when it opens none.
fn raw_string_len(rest: &str) -> Option<usize> {
    let hashes = rest[1..].bytes().take_while(|b| *b == b'#').count();
    if rest.as_bytes().get(1 + hashes) != Some(&b'"') {
        return None;
    }
    let closer = format!("\"{}", "#".repeat(hashes));
    let body = 2 + hashes;
    Some(
        rest[body..]
            .find(&closer)
            .map_or(rest.len(), |n| body + n + closer.len()),
    )
}

/// The length of the character literal `rest` opens, or 1 when the quote is a lifetime's. An
/// escape's closing quote is looked for after the escaped character, so `'\''` is four bytes long
/// and its middle quote closes nothing.
fn char_literal_len(rest: &str) -> usize {
    let mut chars = rest[1..].chars();
    match chars.next() {
        Some('\\') => rest
            .get(3..)
            .and_then(|tail| tail.find('\''))
            .map_or(1, |n| 3 + n + 1),
        Some(c) if rest[1 + c.len_utf8()..].starts_with('\'') => 2 + c.len_utf8(),
        _ => 1,
    }
}

/// `source` as the compiler reads it: every comment, in any form and whatever it holds, is gone.
/// A comment runs nothing, so it is never the statement that runs and writes no keyword: a copy
/// of the statement moved into one, quoted or not, leaves the code without it.
fn code_of(source: &str) -> String {
    let mut code = String::new();
    let mut from = 0;
    for (start, end) in comment_spans(source) {
        code.push_str(&source[from..start]);
        code.push(' ');
        from = end;
    }
    code.push_str(&source[from..]);
    code
}

/// How many times the code of `source` hands the tested statement, quoted whole, to
/// `sqlx::query!`: a copy in a comment, a constant, a doc attribute or any other literal is not
/// the statement that runs.
fn statements_run_in(source: &str) -> usize {
    let code = code_of(source);
    code.match_indices(&format!("\"{PRUNE}\""))
        .filter(|(at, _)| code[..*at].trim_end().ends_with("sqlx::query!("))
        .count()
}

/// How many times `source` writes a delete from the run table, quoted or not, code or comment.
fn delete_statements_in(source: &str) -> usize {
    normalized(source)
        .matches(&format!("delete from {TABLE}").to_lowercase())
        .count()
}

/// What is wrong with a prune's source text: an empty list when it holds exactly one delete
/// statement and that statement is the tested one, handed whole to `sqlx::query!` exactly once.
fn prune_pin_problems(source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let run = statements_run_in(source);
    if run != 1 {
        problems.push(format!(
            "the code hands the statement to sqlx::query! {run} times, not once"
        ));
    }
    let statements = delete_statements_in(source);
    if statements != 1 {
        problems.push(format!(
            "the source holds {statements} delete statements, not one"
        ));
    }
    let keywords = delete_keywords_in(source);
    if keywords != 1 {
        problems.push(format!(
            "the source writes the word delete {keywords} times, not once"
        ));
    }
    problems
}

/// How many times `source` writes `delete` as a word of its own, in any case, whatever follows
/// it: a table spelled `main.agent_runs` or `"agent_runs"`, or an SQL comment after the keyword,
/// still counts, where `delete_statements_in` sees only the one spelling. No comment is read.
fn delete_keywords_in(source: &str) -> usize {
    let lower = code_of(source).to_lowercase();
    let bytes = lower.as_bytes();
    let is_word = |at: Option<&u8>| at.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
    lower
        .match_indices("delete")
        .filter(|(at, _)| {
            let before = at.checked_sub(1).and_then(|i| bytes.get(i));
            !is_word(before) && !is_word(bytes.get(at + "delete".len()))
        })
        .count()
}

/// What is wrong with this test file's own text: an empty list when only the macro writes the
/// statement, so the plan's text is derived from it and no changed copy can stand beside it.
fn own_statement_problems(test_source: &str) -> Vec<String> {
    let statements = delete_statements_in(test_source);
    if statements == 1 {
        Vec::new()
    } else {
        vec![format!(
            "the test file writes the statement {statements} times, not once"
        )]
    }
}

async fn open() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    (dir, db)
}

#[tokio::test]
async fn agent_runs_is_indexed_on_created_at() {
    let (_dir, db) = open().await;
    let columns: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")
            .bind(INDEX)
            .fetch_all(db.reader())
            .await
            .expect("the index's columns");
    assert_eq!(columns, [("created_at".to_owned(),)]);
    let table: Option<(String,)> =
        sqlx::query_as("SELECT tbl_name FROM sqlite_master WHERE type = 'index' AND name = ?1")
            .bind(INDEX)
            .fetch_optional(db.reader())
            .await
            .expect("the schema");
    assert_eq!(table, Some(("agent_runs".to_owned(),)));
}

#[tokio::test]
async fn the_prune_reads_agent_runs_through_the_created_at_index() {
    let (_dir, db) = open().await;
    let plan: Vec<(i64, i64, i64, String)> =
        sqlx::query_as(concat!("EXPLAIN QUERY PLAN ", prune_statement!()))
            .bind(0_i64)
            .fetch_all(db.reader())
            .await
            .expect("the plan");
    let details: Vec<&str> = plan.iter().map(|row| row.3.as_str()).collect();
    assert!(!details.is_empty(), "the planner returned no step");
    assert!(
        details
            .iter()
            .any(|d| d.contains("SEARCH agent_runs USING INDEX agent_runs_by_created_at")),
        "the prune does not use the index: {details:?}"
    );
    assert!(
        !details.iter().any(|d| d.starts_with("SCAN")),
        "the prune scans the table: {details:?}"
    );
    // The statement as one whole string literal, its closing quote included, so an added
    // predicate (`... < ?1 OR ...`) is another statement, never a superstring that still matches.
    let source = include_str!("../src/runs.rs");
    assert_eq!(
        source.matches(&format!("\"{PRUNE}\"")).count(),
        1,
        "the tested statement is not the one the repository runs"
    );
    assert_eq!(prune_pin_problems(source), Vec::<String>::new());
    assert_eq!(
        own_statement_problems(include_str!("runs.rs")),
        Vec::<String>::new()
    );
}

/// A prune source as `runs.rs` states it: the one statement, quoted whole.
fn a_good_prune_source() -> String {
    format!("let done = sqlx::query!(\"{PRUNE}\", cutoff);")
}

#[test]
fn a_comment_quoting_the_prune_beside_a_prune_that_skips_the_index_is_refused() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    // The quoted copy sits in a comment; the statement that runs wraps the column in an
    // expression, which the index cannot seek.
    let decoy = format!(
        "// \"{PRUNE}\"\nlet done = sqlx::query!(\"DELETE FROM {TABLE} WHERE created_at + 0 < ?1\", cutoff);"
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "the decoy is not refused: {problems:?}"
    );
}

#[test]
fn a_second_delete_statement_is_refused_however_it_is_spelled() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    let decoy = format!(
        "{}\nsqlx::query(\"delete  from {TABLE}\");",
        a_good_prune_source()
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "the second statement is not refused: {problems:?}"
    );
}

#[test]
fn a_changed_copy_of_the_statement_in_the_plan_string_is_refused() {
    let one_literal = format!(
        "macro_rules! prune_statement {{ () => {{ \"DELETE FROM {TABLE} WHERE created_at < ?1\" }}; }}\n\
         sqlx::query_as(concat!(\"EXPLAIN QUERY PLAN \", prune_statement!()))"
    );
    assert_eq!(own_statement_problems(&one_literal), Vec::<String>::new());
    let decoy = format!(
        "{one_literal}\nsqlx::query_as(\"EXPLAIN QUERY PLAN DELETE FROM {TABLE} WHERE created_at <= ?1\")"
    );
    let problems = own_statement_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "the changed copy is not refused: {problems:?}"
    );
}

#[test]
fn a_prune_spelled_around_the_keyword_scan_beside_a_quoted_copy_is_refused() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    // Each decoy keeps the tested statement quoted once, in a comment or handed to a
    // `sqlx::query!` that never runs, and runs a prune the index cannot seek, spelled so that the
    // plain statement text never appears in it: the table is schema-qualified or quoted, split
    // across two literals, or an SQL comment sits between the keywords. Beside the copy that is
    // handed to `sqlx::query!`, only the word count sees the prune that runs.
    let copies = [
        format!("// \"{PRUNE}\""),
        format!("let _unused = sqlx::query!(\"{PRUNE}\", cutoff);"),
    ];
    let spellings = [
        format!("DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1"),
        format!("DELETE FROM \\\"{TABLE}\\\" WHERE created_at + 0 < ?1"),
        "DELETE FROM agent\" + \"_runs WHERE created_at + 0 < ?1".to_owned(),
        format!("DELETE/**/FROM {TABLE} WHERE created_at + 0 < ?1"),
    ];
    for spelling in &spellings {
        for copy in &copies {
            let decoy = format!("{copy}\nlet done = sqlx::query!(\"{spelling}\", cutoff);");
            assert_eq!(
                decoy.matches(&format!("\"{PRUNE}\"")).count(),
                1,
                "the decoy does not quote the tested statement once: {decoy}"
            );
            let problems = prune_pin_problems(&decoy);
            assert!(
                !problems.is_empty(),
                "the decoy is not refused: {spelling}: {problems:?}"
            );
        }
        let beside_a_query = format!(
            "{}\nlet done = sqlx::query!(\"{spelling}\", cutoff);",
            copies[1]
        );
        assert_eq!(
            prune_pin_problems(&beside_a_query),
            ["the source writes the word delete 2 times, not once"],
            "the word count did not refuse the prune beside a copy handed to sqlx::query!: {spelling}"
        );
    }
}

#[test]
fn prose_that_names_the_delete_beside_the_prune_is_not_counted() {
    let good = a_good_prune_source();
    // A comment or doc line that names the prune in words is prose, whatever it holds: it runs
    // nothing, so the good source stays good beside it, a quote in it included.
    for prose in [
        "/// The delete reads `created_at` through its index.",
        "// One DELETE, so the index on created_at is used.",
        "// The \"prune\" is one delete, through the index on created_at.",
    ] {
        assert_eq!(
            prune_pin_problems(&format!("{prose}\n{good}")),
            Vec::<String>::new(),
            "prose was counted: {prose}"
        );
    }
    // A comment runs nothing, a copy of the statement in it included: beside the good prune it
    // changes nothing.
    let copy = format!("/// DELETE FROM {TABLE} WHERE created_at < ?1\n{good}");
    assert_eq!(
        prune_pin_problems(&copy),
        Vec::<String>::new(),
        "a commented copy of the statement was read as code"
    );
    // A comment that quotes the statement is not the statement that runs, so beside a changed
    // prune the code hands the tested statement to nothing.
    let decoy = format!(
        "// \"{PRUNE}\"\nlet done = sqlx::query!(\"DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1\", cutoff);"
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "a quoting comment was read as the prune: {problems:?}"
    );
}

/// Every form a Rust comment takes, each wrapping `text`: line, outer doc, inner doc, block, outer
/// doc block, inner doc block, a block nested in a block, and a block over several lines.
fn every_comment_form(text: &str) -> Vec<String> {
    vec![
        format!("// {text}"),
        format!("/// {text}"),
        format!("//! {text}"),
        format!("/* {text} */"),
        format!("/** {text} */"),
        format!("/*! {text} */"),
        format!("/* /* {text} */ */"),
        format!("/*\n {text}\n*/"),
    ]
}

#[test]
fn a_delete_word_or_the_statement_in_any_comment_form_is_not_counted() {
    let good = a_good_prune_source();
    // Two prose texts, one holding a quote, in each of the eight forms: none is read.
    let forms: Vec<String> = [
        "The delete reads created_at through its index.",
        "The \"prune\" is one delete, through the index on created_at.",
    ]
    .into_iter()
    .flat_map(every_comment_form)
    .collect();
    assert_eq!(forms.len(), 16, "the population changed");
    eprintln!("members: {} comment forms", forms.len());
    for comment in &forms {
        assert_eq!(
            prune_pin_problems(&format!("{comment}\n{good}")),
            Vec::<String>::new(),
            "a benign delete word in a comment was counted: {comment}"
        );
    }
    // The statement itself inside each form, bare or quoted, is prose too: no comment is code.
    for text in [PRUNE.to_owned(), format!("\"{PRUNE}\"")] {
        for comment in every_comment_form(&text) {
            assert_eq!(
                prune_pin_problems(&format!("{comment}\n{good}")),
                Vec::<String>::new(),
                "a commented copy of the statement was read as code: {comment}"
            );
        }
    }
    // Comment markers inside a string or a character literal open no comment: a prune written
    // after them on the same line is still read.
    for opener in ["\"// \"", "\"/* \"", "'\"'", "r#\"//\"#"] {
        let decoy = format!(
            "let _ = {opener}; let done = sqlx::query!(\"DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1\", cutoff);\n{good}"
        );
        assert!(
            !prune_pin_problems(&decoy).is_empty(),
            "a comment marker in a literal hid a prune: {opener}"
        );
    }
}

/// Every place the tested statement can be written without being the statement that runs: each
/// comment form, bare and quoted, and each literal in code that `sqlx::query!` does not take.
fn every_copy_that_does_not_run() -> Vec<String> {
    let quoted = format!("\"{PRUNE}\"");
    let mut copies = every_comment_form(PRUNE);
    copies.extend(every_comment_form(&quoted));
    copies.extend([
        format!("const _PRUNE: &str = {quoted};"),
        format!("static _PRUNE: &str = {quoted};"),
        format!("#[doc = {quoted}]"),
        format!("let _prune = {quoted};"),
        format!("let _prune = concat!({quoted});"),
    ]);
    copies
}

#[test]
fn a_copy_of_the_statement_that_does_not_run_is_not_the_tested_statement() {
    assert_eq!(
        prune_pin_problems(&a_good_prune_source()),
        Vec::<String>::new()
    );
    // Prunes that run a statement the source does not write: one from another module, one from
    // a file, and one read by `query_file!`. Each can skip the index while a copy stays behind.
    let runs = [
        "let done = sqlx::query(crate::prune_sql::PRUNE).bind(cutoff);",
        "let done = sqlx::query(include_str!(\"../queries/prune.sql\")).bind(cutoff);",
        "let done = sqlx::query_file!(\"queries/prune.sql\", cutoff);",
    ];
    let copies = every_copy_that_does_not_run();
    assert_eq!(copies.len(), 21, "the population changed");
    let mut members = 0;
    let none = "the code hands the statement to sqlx::query! 0 times, not once".to_owned();
    for run in runs {
        for copy in &copies {
            let problems = prune_pin_problems(&format!("{copy}\n{run}"));
            assert!(
                problems.contains(&none),
                "a copy that does not run was read as the prune: {copy}: {problems:?}"
            );
            members += 1;
        }
    }
    assert_eq!(members, 63, "the population changed");
    eprintln!("members: {members} copies-by-runs");
}

#[test]
fn no_character_literal_hides_a_second_statement_in_the_string_after_it() {
    let good = a_good_prune_source();
    // A string after the literal holds comment markers around a second statement. A literal the
    // scan measures wrongly lets that string's quote open a string outside it, and the markers
    // then hide the statement from the word count.
    let hidden = format!(
        "sqlx::query(\"/* /* */ DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1 -- */\");"
    );
    let literals = [
        "'\"'",
        "'\\''",
        "'\\\"'",
        "'\\\\'",
        "b'\\''",
        "b'\"'",
        "'\\x27'",
        "'\\x22'",
        "'\\u{27}'",
        "'\\u{22}'",
        "'\\n'",
        "'/'",
        "'*'",
    ];
    assert_eq!(literals.len(), 13, "the population changed");
    eprintln!("members: {} character literals", literals.len());
    for literal in literals {
        let before = format!("let _ = ({literal},'\"');\n{good}");
        assert_eq!(
            prune_pin_problems(&before),
            Vec::<String>::new(),
            "the literal alone was refused: {literal}"
        );
        assert_eq!(
            prune_pin_problems(&format!("{before}\n{hidden}")),
            ["the source writes the word delete 2 times, not once"],
            "a character literal hid a second statement: {literal}"
        );
    }
}

/// The prune the tests set beside a holder: the good one, the statement handed whole to
/// `sqlx::query!`, and one that runs a statement written in another module, so that only a copy
/// can make the source look like the prune.
fn the_two_prunes() -> [String; 2] {
    [
        a_good_prune_source(),
        "let done = sqlx::query(crate::prune_sql::INDEX_SKIPPING).bind(cutoff);".to_owned(),
    ]
}

/// The string-like literal kinds rustc lexes as one token whatever they hold, each an opener, a
/// closer and the type of a constant holding it: a string, raw strings with none to three hashes,
/// a byte string and a raw one, a C string and a raw one.
const LITERAL_KINDS: [(&str, &str, &str); 11] = [
    ("\"", "\"", "&str"),
    ("r\"", "\"", "&str"),
    ("r#\"", "\"#", "&str"),
    ("r##\"", "\"##", "&str"),
    ("r###\"", "\"###", "&str"),
    ("b\"", "\"", "&[u8]"),
    ("br\"", "\"", "&[u8]"),
    ("br#\"", "\"#", "&[u8]"),
    ("c\"", "\"", "&core::ffi::CStr"),
    ("cr\"", "\"", "&core::ffi::CStr"),
    ("cr#\"", "\"#", "&core::ffi::CStr"),
];

/// `text` written as the literal `open`..`close`: escaped where the kind escapes, as written where
/// it is raw, or `None` where a raw kind cannot hold `text` because `text` holds its closer.
fn literal_holding(open: &str, close: &str, text: &str) -> Option<String> {
    if open.trim_start_matches(['b', 'c']).starts_with('r') {
        (!text.contains(close)).then(|| format!("{open}{text}{close}"))
    } else {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        Some(format!("{open}{escaped}{close}"))
    }
}

/// Where a holder closes against the code after it: a holder on one line of its own; one that
/// spans lines and closes on the line before; one that spans lines and closes on the code's own
/// line. Each is the text before and after the held text, then what separates the holder from
/// the code.
const CLOSES: [(&str, &str, &str); 3] = [("", "", "\n"), ("\n", "\n", "\n"), ("\n", "\n", " ")];

/// Every comment kind rustc lexes, as an opener and a closer: line, a line of four slashes, outer
/// doc, inner doc, block, nested block, outer doc block and inner doc block. A line kind holds one
/// line and closes where the line ends.
const COMMENT_KINDS: [(&str, &str); 8] = [
    ("// ", ""),
    ("//// ", ""),
    ("/// ", ""),
    ("//! ", ""),
    ("/* ", " */"),
    ("/* /* ", " */ */"),
    ("/** ", " */"),
    ("/*! ", " */"),
];

/// A copy of the call, the way the prune writes it.
fn a_copy_of_the_call() -> String {
    format!("sqlx::query!(\"{PRUNE}\", cutoff)")
}

/// Every literal holding a copy of the call, of every kind that can hold it, as a statement, a
/// constant, a static or a doc attribute, closing in each place `CLOSES` names.
fn every_literal_holding_the_call() -> Vec<String> {
    let mut holders = Vec::new();
    for (open, close, kind) in LITERAL_KINDS {
        for (before, after, separator) in CLOSES {
            let text = format!("{before}{}{after}", a_copy_of_the_call());
            let Some(literal) = literal_holding(open, close, &text) else {
                continue;
            };
            holders.push(format!("let _ = {literal};{separator}"));
            holders.push(format!("const _COPY: {kind} = {literal};{separator}"));
            holders.push(format!("static _COPY: {kind} = {literal};{separator}"));
            if kind == "&str" {
                holders.push(format!(
                    "#[doc = {literal}]\nfn _documented() {{}}{separator}"
                ));
            }
        }
    }
    holders
}

#[test]
fn a_literal_of_any_kind_holding_the_call_is_not_the_call() {
    // Beside the good prune, a doc attribute is prose and changes nothing, and any other literal's
    // word delete is still counted: such a literal can be handed to `sqlx::query` and run, so the
    // source is refused on the count, never on the call. Beside a prune written elsewhere, the
    // literal is no call, so the code hands the statement to none.
    let holders = every_literal_holding_the_call();
    assert_eq!(holders.len(), 84, "the population changed");
    let [good, elsewhere] = the_two_prunes();
    let counted = ["the source writes the word delete 2 times, not once"];
    let mut members = 0;
    for holder in &holders {
        let expected: &[&str] = if holder.starts_with("#[doc") {
            &[]
        } else {
            &counted
        };
        assert_eq!(
            prune_pin_problems(&format!("{holder}{good}")),
            expected,
            "a literal holding the call was not counted, or was read as the call: {holder}"
        );
        let problems = prune_pin_problems(&format!("{holder}{elsewhere}"));
        assert!(
            problems.contains(
                &"the code hands the statement to sqlx::query! 0 times, not once".to_owned()
            ),
            "a literal holding the call was read as the call: {holder}: {problems:?}"
        );
        members += 2;
    }
    assert_eq!(members, 168, "the population changed");
    eprintln!("examined {members} literal holders");
}

/// Every comment holding a copy of the call or the bare statement, of every kind, closing in each
/// place `CLOSES` names that the kind can close.
fn every_comment_holding_the_statement() -> Vec<String> {
    let mut holders = Vec::new();
    for (open, close) in COMMENT_KINDS {
        for text in [a_copy_of_the_call(), PRUNE.to_owned()] {
            for (before, after, separator) in CLOSES {
                if close.is_empty() && !before.is_empty() {
                    continue;
                }
                let separator = if close.is_empty() { "\n" } else { separator };
                holders.push(format!("{open}{before}{text}{after}{close}{separator}"));
            }
        }
    }
    holders
}

#[test]
fn a_comment_of_any_kind_holding_the_statement_is_not_read() {
    // A comment runs nothing, whatever it holds: beside the good prune it changes nothing, and
    // beside a prune written elsewhere it is no call.
    let holders = every_comment_holding_the_statement();
    assert_eq!(holders.len(), 32, "the population changed");
    let [good, elsewhere] = the_two_prunes();
    let mut members = 0;
    for holder in &holders {
        assert_eq!(
            prune_pin_problems(&format!("{holder}{good}")),
            Vec::<String>::new(),
            "a comment was read as code: {holder}"
        );
        let problems = prune_pin_problems(&format!("{holder}{elsewhere}"));
        assert!(
            problems.contains(
                &"the code hands the statement to sqlx::query! 0 times, not once".to_owned()
            ),
            "a comment was read as the call: {holder}: {problems:?}"
        );
        members += 2;
    }
    assert_eq!(members, 64, "the population changed");
    eprintln!("examined {members} comment holders");
}

/// Rustc's whitespace, `Pattern_White_Space`: the eleven code points its lexer skips between
/// tokens. NBSP and U+3000 are not among them, and rustc refuses either between tokens.
const RUSTC_WHITESPACE: [char; 11] = [
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{85}', '\u{200e}', '\u{200f}', '\u{2028}',
    '\u{2029}',
];

/// Every spelling of the good call that rustc reads as the same tokens: each whitespace code point
/// in each gap between the tokens the pin reads, then each path spelling with each literal
/// spelling of the statement (raw with none to three hashes, an escape, a line continuation).
fn every_spelling_of_the_call() -> Vec<String> {
    let mut spellings = Vec::new();
    let pieces = [
        "let done = sqlx".to_owned(),
        "::".to_owned(),
        "query".to_owned(),
        "!".to_owned(),
        "(".to_owned(),
        format!("\"{PRUNE}\""),
        ", cutoff);".to_owned(),
    ];
    for w in RUSTC_WHITESPACE {
        for gap in 1..pieces.len() {
            let mut spelling = pieces[..gap].concat();
            spelling.push(w);
            spelling.push_str(&pieces[gap..].concat());
            spellings.push(spelling);
        }
    }
    let paths = [
        "sqlx::query!(@)",
        "sqlx::query! (@)",
        "sqlx :: query!(@)",
        "sqlx :: query ! (@)",
        "sqlx::query!(\n    @)",
        "sqlx::query![@]",
        "sqlx::query!{@}",
        "r#sqlx::r#query!(@)",
    ];
    let literals = [
        format!("\"{PRUNE}\""),
        format!("r\"{PRUNE}\""),
        format!("r#\"{PRUNE}\"#"),
        format!("r##\"{PRUNE}\"##"),
        format!("r###\"{PRUNE}\"###"),
        format!("\"{}\"", PRUNE.replacen('D', "\\x44", 1)),
        format!("\"{}\"", PRUNE.replacen('D', "\\u{44}", 1)),
        format!("\"{}\"", PRUNE.replacen(" WHERE", " \\\n    WHERE", 1)),
    ];
    for path in paths {
        for literal in &literals {
            let call = path.replacen('@', &format!("{literal}, cutoff"), 1);
            spellings.push(format!("let done = {call};"));
        }
    }
    spellings
}

#[test]
fn every_spelling_of_the_call_rustc_reads_as_the_call_is_the_call() {
    let spellings = every_spelling_of_the_call();
    assert_eq!(spellings.len(), 130, "the population changed");
    for spelling in &spellings {
        assert_eq!(
            prune_pin_problems(spelling),
            Vec::<String>::new(),
            "a spelling rustc reads as the call was refused: {spelling:?}"
        );
    }
    eprintln!("examined {} spellings", spellings.len());
}

#[test]
fn a_prune_spelled_by_an_escape_a_continuation_or_tokens_beside_a_dead_copy_is_refused() {
    // Each prune runs a statement the index cannot seek, its keyword spelled so that no text scan
    // reads the word: an escape, a line continuation inside the word, or the words as tokens
    // through `stringify!`. Beside each stands a copy of the call that never runs.
    let moved = format!("DELETE FROM {TABLE} WHERE created_at + 0 < ?1");
    let lives = [
        format!("\"{}\"", moved.replacen('D', "\\x44", 1)),
        format!("\"{}\"", moved.replacen('D', "\\u{44}", 1)),
        format!("\"{}\"", moved.replacen("DELETE", "DE\\\nLETE", 1)),
        format!("r#\"{moved}\"#"),
        format!("stringify!({moved})"),
    ];
    let call = a_copy_of_the_call();
    let dead = [
        format!("fn _unused(cutoff: i64) {{ let _ = {call}; }}\n"),
        format!("#[cfg(any())]\nfn _dead(cutoff: i64) {{ let _ = {call}; }}\n"),
        format!("macro_rules! _never {{ ($c:expr) => {{ sqlx::query!(\"{PRUNE}\", $c) }}; }}\n"),
        format!("let _unused = {call};\n"),
    ];
    let mut members = 0;
    for live in &lives {
        for copy in &dead {
            let source = format!("{copy}let done = sqlx::query({live}).bind(cutoff);");
            assert_eq!(
                prune_pin_problems(&source),
                ["the source writes the word delete 2 times, not once"],
                "a prune spelled around the word count was not refused: {source}"
            );
            members += 1;
        }
    }
    assert_eq!(members, 20, "the population changed");
    eprintln!("examined {members} spelled prunes");
}

#[test]
fn a_source_that_does_not_lex_is_refused() {
    // An unclosed delimiter: the pin cannot read where the call ends, so it refuses the source.
    let source = format!("let done = sqlx::query!(\"{PRUNE}\", cutoff;");
    let problems = prune_pin_problems(&source);
    assert!(
        problems
            .iter()
            .any(|problem| problem.starts_with("the source does not lex as Rust")),
        "a source that does not lex was read: {source}: {problems:?}"
    );
}
