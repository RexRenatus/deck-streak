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

/// What is wrong with a prune's source text: an empty list when it is the one statement, quoted
/// whole exactly once.
fn prune_pin_problems(source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let quoted = source.matches(&format!("\"{PRUNE}\"")).count();
    if quoted != 1 {
        problems.push(format!("the statement is quoted {quoted} times, not once"));
    }
    problems
}

/// What is wrong with this test file's own text: an empty list when only the macro writes the
/// statement, so the plan's text is derived from it.
fn own_statement_problems(_test_source: &str) -> Vec<String> {
    Vec::new()
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
