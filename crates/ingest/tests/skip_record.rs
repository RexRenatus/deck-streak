//! The skip day's record (SPEC-083 A1 to A4, A7, A8): one skip per study day and again after an
//! undo, the migration's key, the undo's target, the skip set, the search and the day spec, and the
//! summary. Every day is a synthetic epoch day; no row is anyone's data.

// An integration test is test code: its helpers panic on a malformed golden.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use deck_streak_ingest::skip::{
    FailReason, SearchRefusal, SkipRefusal, SkipRow, SkipState, SkipStore, calendar_month,
    skip_search, skip_spec, summarize_skips,
};
use deck_streak_kernel::{StudyDay, UtcMillis};
use serde_json::Value;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn at(millis: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(millis)
}

fn integer(value: &Value) -> i64 {
    value.as_i64().expect("the golden holds an integer")
}

async fn store() -> (support::Fixture, SkipStore, deck_streak_kernel::Db) {
    let fixture = support::Fixture::new("http://127.0.0.1:9/");
    let db = fixture.db().await;
    (fixture, SkipStore::new(db.clone()), db)
}

#[tokio::test]
async fn a_skip_is_recorded_once_per_study_day_and_again_after_an_undo() {
    let (_fixture, skips, db) = store().await;
    let first = skips
        .begin(day(20_000), Some(12), at(1_000))
        .await
        .expect("the first take is recorded");
    let twice = skips.begin(day(20_000), Some(12), at(2_000)).await;
    assert!(
        matches!(twice, Err(SkipRefusal::AlreadySkipped)),
        "a second take for the day is refused: {twice:?}"
    );
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM skip_days")
        .fetch_one(db.reader())
        .await
        .expect("a count");
    assert_eq!(rows, 1, "the refusal wrote no row (R2)");

    skips
        .settle_applied(first, 7, false)
        .await
        .expect("the take settles");
    let after_apply = skips.begin(day(20_000), None, at(3_000)).await;
    assert!(
        matches!(after_apply, Err(SkipRefusal::AlreadySkipped)),
        "an applied skip holds its day: {after_apply:?}"
    );
    skips
        .mark_undone(first, at(4_000))
        .await
        .expect("the undo is recorded");
    let again = skips
        .begin(day(20_000), Some(3), at(5_000))
        .await
        .expect("an undone day is free again");
    assert_ne!(again, first, "the second skip is its own row");
    skips
        .settle_failed(again, FailReason::PushFailed)
        .await
        .expect("a failed take settles");
    let third = skips.begin(day(20_000), None, at(6_000)).await;
    assert!(
        third.is_ok(),
        "a failed take leaves the day free: {third:?}"
    );
    let records = skips.records().await.expect("the rows read");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].state, SkipState::Applied);
    assert_eq!(records[0].undone_at, Some(at(4_000)));
    assert_eq!(records[1].state, SkipState::Failed(FailReason::PushFailed));
}

#[tokio::test]
async fn the_migration_refuses_a_second_skip_not_undone_for_one_study_day() {
    let (_fixture, _skips, db) = store().await;
    let insert = "INSERT INTO skip_days (study_day, state, created_at) VALUES (?1, ?2, 1)";
    let mut write = db.write().await.expect("a write");
    sqlx::query(insert)
        .bind(20_000_i64)
        .bind("pending")
        .execute(&mut *write)
        .await
        .expect("the first row");
    let second = sqlx::query(insert)
        .bind(20_000_i64)
        .bind("applied")
        .execute(&mut *write)
        .await;
    assert!(
        second.is_err(),
        "the key refuses a second pending or applied skip for one study day"
    );
    sqlx::query("INSERT INTO skip_days (study_day, state, reason, created_at) VALUES (20000, 'failed', 'push_failed', 1)")
        .execute(&mut *write)
        .await
        .expect("a failed row does not hold the day");
    sqlx::query(insert)
        .bind(20_001_i64)
        .bind("pending")
        .execute(&mut *write)
        .await
        .expect("another day is free");
    let bad_reason = sqlx::query("INSERT INTO skip_days (study_day, state, reason, created_at) VALUES (20005, 'failed', 'Not A Code!', 1)")
        .execute(&mut *write)
        .await;
    assert!(
        bad_reason.is_err(),
        "a reason is a bounded code, never text"
    );
}

#[tokio::test]
async fn undo_reverses_the_most_recent_skip_not_undone() {
    let (_fixture, skips, _db) = store().await;
    let empty = skips.latest_undoable(day(20_010)).await;
    assert!(matches!(empty, Err(SkipRefusal::NothingToUndo)));
    let older = skips.begin(day(20_001), None, at(1)).await.expect("older");
    let newer = skips.begin(day(20_004), None, at(2)).await.expect("newer");
    skips.settle_applied(older, 3, false).await.expect("settle");
    skips.settle_applied(newer, 4, false).await.expect("settle");
    let target = skips
        .latest_undoable(day(20_010))
        .await
        .expect("a skip to undo");
    assert_eq!(target.id, newer, "the most recent study day is the target");
    skips.mark_undone(newer, at(3)).await.expect("undone");
    let next = skips
        .latest_undoable(day(20_010))
        .await
        .expect("the older one is next");
    assert_eq!(next.id, older);
    skips.mark_undone(older, at(4)).await.expect("undone");
    assert!(matches!(
        skips.latest_undoable(day(20_010)).await,
        Err(SkipRefusal::NothingToUndo)
    ));
    let pending = skips
        .begin(day(20_010), None, at(5))
        .await
        .expect("today's take");
    let refused = skips.latest_undoable(day(20_010)).await;
    assert!(
        matches!(refused, Err(SkipRefusal::TakePending)),
        "an undo waits for the day's pending take: {refused:?}"
    );
    skips
        .settle_failed(pending, FailReason::TimedOut)
        .await
        .expect("settle");
}

#[tokio::test]
async fn the_skip_set_holds_exactly_the_days_with_an_applied_skip_not_undone() {
    let (_fixture, skips, _db) = store().await;
    let applied = skips.begin(day(20_001), None, at(1)).await.expect("a");
    let undone = skips.begin(day(20_002), None, at(2)).await.expect("b");
    let failed = skips.begin(day(20_003), None, at(3)).await.expect("c");
    skips
        .begin(day(20_004), None, at(4))
        .await
        .expect("pending");
    skips
        .settle_applied(applied, 1, false)
        .await
        .expect("settle");
    skips
        .settle_applied(undone, 1, false)
        .await
        .expect("settle");
    skips.mark_undone(undone, at(5)).await.expect("undone");
    skips
        .settle_failed(failed, FailReason::WriteFailed)
        .await
        .expect("settle");
    assert_eq!(
        skips.skip_set().await.expect("the set reads"),
        vec![day(20_001)],
        "only an applied skip not undone covers its day (R4)"
    );
}

#[test]
fn the_search_and_day_spec_equal_the_parity_goldens() {
    let holds = " prop:due=0 -is:suspended -is:buried -deck:filtered";
    let mut spec_cases = 0;
    golden::each_case("skip_spec", |case| {
        spec_cases += 1;
        let spec = skip_spec(
            integer(&case.input["min_days"]),
            integer(&case.input["max_days"]),
        );
        assert_eq!(Value::from(spec), case.output);
    });
    assert!(spec_cases > 0);
    let mut search_cases = 0;
    golden::each_case("skip_search", |case| {
        search_cases += 1;
        let configured = case.input["search"].as_str().expect("a search");
        let expected = case.output[0].as_str().expect("one query");
        assert_eq!(
            skip_search(configured).expect("the golden's search parses"),
            format!("{expected}{holds}"),
            "the wrap equals the predecessor's, followed by the holds"
        );
    });
    assert!(search_cases > 0);
    println!("A7: examined {spec_cases} spec cases and {search_cases} search cases");
    for refused in ["deck:X) or (deck:X", "", "   ", "(a", "a)", "\"a"] {
        assert_eq!(
            skip_search(refused),
            Err(SearchRefusal::NotOneExpression),
            "{refused:?} is not one expression"
        );
    }
    assert!(
        skip_search("deck:\"a (b\"").is_ok(),
        "a parenthesis in quotes is text"
    );
}

#[test]
fn the_summary_shows_counts_equal_to_the_parity_golden() {
    let mut examined = 0;
    golden::each_case("skip_summary", |case| {
        examined += 1;
        let rows: Vec<SkipRow> = case.input["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|row| SkipRow {
                day: day(integer(&row["day"])),
                applied: row["applied"].as_bool().expect("applied"),
                undone: row["undone"].as_bool().expect("undone"),
                cards_moved: integer(&row["cards_moved"]),
            })
            .collect();
        let summary = summarize_skips(&rows, day(integer(&case.input["today"])));
        assert_eq!(
            integer(&case.output["this_month"]),
            i64::from(summary.this_month)
        );
        assert_eq!(
            integer(&case.output["all_time"]),
            i64::from(summary.all_time)
        );
        assert_eq!(
            case.output["last_day"].as_i64(),
            summary.last_day.map(StudyDay::epoch_day)
        );
        assert_eq!(
            integer(&case.output["cards_moved_all_time"]),
            summary.cards_moved_all_time
        );
    });
    assert!(examined > 0);
    println!("A8: examined {examined} summary cases");
    assert_eq!(calendar_month(day(0)), Some((1970, 1)));
    assert_eq!(calendar_month(day(31)), Some((1970, 2)));
}
