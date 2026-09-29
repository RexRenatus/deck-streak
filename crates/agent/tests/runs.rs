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

/// How many times `source` writes a delete from the run table, quoted or not, code or comment.
fn delete_statements_in(source: &str) -> usize {
    normalized(source)
        .matches(&format!("delete from {TABLE}").to_lowercase())
        .count()
}

/// What is wrong with a prune's source text: an empty list when it holds exactly one delete
/// statement and that statement is the tested one, quoted whole exactly once.
fn prune_pin_problems(source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let quoted = source.matches(&format!("\"{PRUNE}\"")).count();
    if quoted != 1 {
        problems.push(format!("the statement is quoted {quoted} times, not once"));
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
/// still counts, where `delete_statements_in` sees only the one spelling.
fn delete_keywords_in(source: &str) -> usize {
    let lower = source.to_lowercase();
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
    // Each decoy keeps the tested statement quoted once in a comment and runs a prune the index
    // cannot seek, spelled so that the plain statement text never appears in it: the table is
    // schema-qualified or quoted, split across two literals, or an SQL comment sits between the
    // keywords.
    let spellings = [
        format!("DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1"),
        format!("DELETE FROM \\\"{TABLE}\\\" WHERE created_at + 0 < ?1"),
        "DELETE FROM agent\" + \"_runs WHERE created_at + 0 < ?1".to_owned(),
        format!("DELETE/**/FROM {TABLE} WHERE created_at + 0 < ?1"),
    ];
    for spelling in spellings {
        let decoy = format!("// \"{PRUNE}\"\nlet done = sqlx::query!(\"{spelling}\", cutoff);");
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
}

#[test]
fn prose_that_names_the_delete_beside_the_prune_is_not_counted() {
    let good = a_good_prune_source();
    // A comment or doc line that names the prune in words and holds no double quote is prose: it
    // runs nothing and quotes nothing, so the good source stays good beside it.
    for prose in [
        "/// The delete reads `created_at` through its index.",
        "// One DELETE, so the index on created_at is used.",
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
    // A comment line that quotes is read, so its copy beside a changed prune is still refused.
    let decoy = format!(
        "// \"{PRUNE}\"\nlet done = sqlx::query!(\"DELETE FROM main.{TABLE} WHERE created_at + 0 < ?1\", cutoff);"
    );
    let problems = prune_pin_problems(&decoy);
    assert!(
        !problems.is_empty(),
        "a quoting comment was read as prose: {problems:?}"
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
    let forms = every_comment_form("The delete reads created_at through its index.");
    assert_eq!(forms.len(), 8, "the population changed");
    for comment in &forms {
        assert_eq!(
            prune_pin_problems(&format!("{comment}\n{good}")),
            Vec::<String>::new(),
            "a benign delete word in a comment was counted: {comment}"
        );
    }
    // The statement itself inside each form is a second delete statement, and only that.
    for comment in every_comment_form(PRUNE) {
        assert_eq!(
            prune_pin_problems(&format!("{comment}\n{good}")),
            ["the source holds 2 delete statements, not one"],
            "a commented copy of the statement was not refused: {comment}"
        );
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
