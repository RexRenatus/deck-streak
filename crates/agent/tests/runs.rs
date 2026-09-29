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
        Some('\\') => rest[2..].find('\'').map_or(1, |n| 2 + n + 1),
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
        if source[start..end].contains('"') {
            code.push_str(&source[start..end]);
        } else {
            code.push(' ');
        }
        from = end;
    }
    code.push_str(&source[from..]);
    code
}

/// How many times the code of `source` hands the tested statement, quoted whole, to
/// `sqlx::query!`: a copy in a comment, a constant, a doc attribute or any other literal is not
/// the statement that runs.
fn statements_run_in(source: &str) -> usize {
    source.matches(&format!("\"{PRUNE}\"")).count()
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
    // The statement count still reads prose: a commented copy of the statement is refused.
    let copy = format!("/// DELETE FROM {TABLE} WHERE created_at < ?1\n{good}");
    assert_eq!(
        prune_pin_problems(&copy),
        ["the source holds 2 delete statements, not one"],
        "a commented copy of the statement was not refused as a second statement"
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
fn a_delete_word_in_any_comment_form_is_not_counted_but_the_statement_in_one_is() {
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
    for comment in &forms {
        assert_eq!(
            prune_pin_problems(&format!("{comment}\n{good}")),
            Vec::<String>::new(),
            "a benign delete word in a comment was counted: {comment}"
        );
    }
    // The statement itself inside each form, bare or quoted, is a second delete statement, and
    // only that.
    for text in [PRUNE.to_owned(), format!("\"{PRUNE}\"")] {
        for comment in every_comment_form(&text) {
            assert_eq!(
                prune_pin_problems(&format!("{comment}\n{good}")),
                ["the source holds 2 delete statements, not one"],
                "a commented copy of the statement was not refused: {comment}"
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
    let none = "the code hands the statement to sqlx::query! 0 times, not once".to_owned();
    for run in runs {
        for copy in &copies {
            let problems = prune_pin_problems(&format!("{copy}\n{run}"));
            assert!(
                problems.contains(&none),
                "a copy that does not run was read as the prune: {copy}: {problems:?}"
            );
        }
    }
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
