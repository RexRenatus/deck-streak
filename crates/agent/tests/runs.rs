//! The `agent_runs` prune reads `created_at` through an index (SPEC-043 A19, A20, issue 363).
#![allow(clippy::expect_used)]

use deck_streak_kernel::Db;

/// The index migration `004302` creates, named like the ledger's other `*_by_*` indexes.
const INDEX: &str = "agent_runs_by_created_at";

/// The statement `AgentRuns::prune_before` runs, as `crates/agent/src/runs.rs` states it; the test
/// below reads the source and asserts it holds this exact text.
const PRUNE: &str = "DELETE FROM agent_runs WHERE created_at < ?1";

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
        sqlx::query_as("EXPLAIN QUERY PLAN DELETE FROM agent_runs WHERE created_at < ?1")
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
}
