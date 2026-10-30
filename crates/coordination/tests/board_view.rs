//! The personal board's view over a migrated database (SPEC-075 R1, A1; #79): for every case of
//! the predecessor's `ReadApiLayer.leaderboard` golden, the board read from synthetic rollups, the
//! language streak and both XP tables equals the golden's rows, and it asks for as many rollups as
//! the predecessor did.
//!
//! Each case seeds a law streak unlike the language one, so a board that read the law streak would
//! show other numbers; and its XP total is split across `xp_ledger` and `xp_settlement`, so a level
//! read from one table alone would be lower.

// An integration test is test code: its helpers panic on a malformed case or a failed fixture,
// and the examined count is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_coordination::progression::board_view::{BOARD_ROLLUPS, board_view};
use deck_streak_kernel::{Db, StudyDay};
use serde_json::{Value, json};

/// A golden integer field.
fn int(value: &Value, name: &str) -> i64 {
    value[name]
        .as_i64()
        .unwrap_or_else(|| panic!("an integer {name} in {value}"))
}

/// A migrated database in `scratch` holding the case's rollups, its language streak (when it has
/// one) beside a law streak unlike it, and its XP total split across both XP tables.
async fn seeded(scratch: &tempfile::TempDir, input: &Value) -> Db {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let today = int(input, "today");
    let mut write = db.write().await.expect("a write");
    for pair in input["rollups"].as_array().expect("the case's rollups") {
        let on = pair[0].as_i64().expect("a rollup day");
        let score = pair[1].as_i64().expect("a rollup score");
        sqlx::query(
            "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, \
             relearn_count, filtered_count, seconds, answered, passed, true_retention, \
             graduations, decks_studied, avg_answer_seconds, young_answered, young_passed, \
             mature_answered, mature_passed, mature_count, young_count, leech_active, backlog, \
             due_today, card_state_src, score, consistency, retention, workload, volume, mastery, \
             score_at_close, settled_at, fingerprint, created_at, updated_at) \
             VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, 1, 2, 14.0, 0, 0, 40, 36, 64, 60, \
             2, 5, 30, 'live:1736911800000', ?2, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
             'synthetic', 1000, 1000)",
        )
        .bind(on)
        .bind(score)
        .execute(&mut *write)
        .await
        .expect("the synthetic rollup is written");
    }
    let (current, longest) = match input["streak"].as_array() {
        Some(pair) => (
            pair[0].as_i64().expect("a current run"),
            pair[1].as_i64().expect("a longest run"),
        ),
        None => (-1, -1),
    };
    if current >= 0 {
        sqlx::query(
            "INSERT INTO streak_state (track, current_days, longest_days, freezes, \
             last_study_day, comeback_armed, created_at) \
             VALUES ('language', ?1, ?2, 1, ?3, 0, 1000)",
        )
        .bind(current)
        .bind(longest)
        .bind(today)
        .execute(&mut *write)
        .await
        .expect("the synthetic language streak is written");
    }
    sqlx::query(
        "INSERT INTO streak_state (track, current_days, longest_days, freezes, last_study_day, \
         comeback_armed, created_at) VALUES ('law', ?1, ?2, 1, ?3, 0, 1000)",
    )
    .bind(current.max(0) + 7)
    .bind(longest.max(0) + 11)
    .bind(today)
    .execute(&mut *write)
    .await
    .expect("the synthetic law streak is written");
    let total = int(input, "total_xp");
    let granted = total / 2;
    sqlx::query(
        "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
         VALUES (?1, 'quest:1', 'language', ?2, 'per-day', 1000)",
    )
    .bind(today)
    .bind(granted)
    .execute(&mut *write)
    .await
    .expect("the synthetic grant is written");
    sqlx::query(
        "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
         VALUES (?1, 'reviews', 'language', ?2, 1, 1000)",
    )
    .bind(today)
    .bind(total - granted)
    .execute(&mut *write)
    .await
    .expect("the synthetic settlement is written");
    write.commit().await.expect("the commit");
    db
}

#[tokio::test]
async fn the_board_matches_the_parity_golden() {
    let mut cases = Vec::new();
    let examined = golden::each_case("leaderboard", |case| {
        cases.push((case.input.clone(), case.output.clone()));
    });
    assert_eq!(
        examined.function,
        "pipeline_layers.read_api.ReadApiLayer.leaderboard"
    );
    for (input, output) in &cases {
        assert_eq!(
            output["limits"],
            json!([BOARD_ROLLUPS]),
            "the predecessor asked for as many rollups"
        );
        let scratch = tempfile::tempdir().expect("a temporary directory");
        let db = seeded(&scratch, input).await;
        let today = StudyDay::from_epoch_day(int(input, "today"));
        let rows = board_view(&db, today).await.expect("the board is read");
        let ours: Vec<Value> = rows
            .iter()
            .map(|row| {
                let detail = match (row.study_day(), row.longest(), row.title()) {
                    (Some(day), _, _) => json!(day.epoch_day()),
                    (None, Some(longest), _) => json!(format!("longest {longest}")),
                    (None, None, Some(title)) => json!(title),
                    (None, None, None) => Value::Null,
                };
                json!({
                    "detail": detail,
                    "label": format!("{} {}", row.emoji(), row.label()),
                    "value": row.value(),
                })
            })
            .collect();
        assert_eq!(json!(ours), output["board"], "{input}");
        db.close().await;
    }
    println!("examined {} board case(s)", cases.len());
    assert_eq!(cases.len(), examined.count);
}
