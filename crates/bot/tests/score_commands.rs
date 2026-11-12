//! The owner's `/score` (SPEC-071 A24; R21): the bot reports the numbers `GET /api/score` answers
//! for the same study day, because both read coordination's `day_score`; and an absent retention is
//! said to be absent.
//!
//! Each `/score` goes through the bot's own handlers to a fake Bot API, over a temporary database
//! holding synthetic rollup rows, and the bench's manual clock decides the study day.

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use deck_streak_bot::score_commands::score_reply;
use deck_streak_coordination::score::day_score;
use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use fake_bot_api::{Bench, ScriptedSync, golden_send, incoming, owner_says, payload};
use serde_json::{Value, json};

/// The bench's study day at its start: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;
/// The day before it, whose rollup holds no answered review.
const NO_ANSWER: i64 = TODAY - 1;
/// A day in milliseconds.
const DAY_MS: i64 = 86_400_000;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Writes a synthetic rollup of `day`: `reviews` reviews, `answered` answered with `true_retention`
/// percent, a recorded card state, and the total `score`.
async fn seed(db: &Db, day: i64, reviews: i64, answered: i64, true_retention: f64, score: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, ?2, 0, ?2, 0, 0, 600.0, ?3, ?3, ?4, 1, 1, 30.0, 0, 0, ?3, ?3, 120, 60, 2, 5, \
         30, 'live:1736911800000', ?5, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, 'synthetic', \
         1000, 1000)",
    )
    .bind(day)
    .bind(reviews)
    .bind(answered)
    .bind(true_retention)
    .bind(score)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    write.commit().await.expect("the commit");
}

/// The payload of the last `sendMessage`.
fn last_send(bench: &Bench) -> Value {
    payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"))
}

#[tokio::test]
async fn score_reports_the_numbers_the_score_route_returns() {
    let bench = Bench::start().await;
    seed(&bench.db, NO_ANSWER, 0, 0, 0.0, 12).await;
    seed(&bench.db, TODAY, 40, 24, 87.5, 72).await;
    let mut commands = bench.commands(ScriptedSync::default());

    // For each study day: the bot's answer is the golden, and it is the reply rendered from the
    // numbers of `day_score`, which `GET /api/score` answers for the same day.
    let cases = examined(
        "study day(s) scored",
        vec![(NO_ANSWER, "score-no-retention"), (TODAY, "score")],
    );
    for (update, (day, golden)) in (1..).zip(cases) {
        bench
            .clock
            .set(UtcMillis::from_epoch_millis(day * DAY_MS + 5 * 3_600_000));
        commands
            .handle(incoming(owner_says(update, "/score")))
            .await;
        let sent = last_send(&bench);
        let numbers = day_score(&bench.db, StudyDay::from_epoch_day(day))
            .await
            .expect("the score reads")
            .expect("the day has a rollup");
        let rendered = score_reply(Some(&numbers));
        assert_eq!(sent["text"], json!(rendered.text), "{golden}");
        assert_eq!(sent, golden_send(golden), "{golden}");
    }

    // The current day's own numbers, as the owner reads them.
    let today = last_send(&bench)["text"]
        .as_str()
        .expect("a text")
        .to_owned();
    for number in ["72", "SOLID", "Reviews: 40", "Retention: 87.5%"] {
        assert!(today.contains(number), "{number} in {today}");
    }
}
