//! The owner's `/level` (SPEC-072 A27; R25): the title's emoji, the level and the title, the total
//! and the XP into the level over the XP the level spans, and the consistency line only when the
//! multiplier is above 1.0.
//!
//! Each `/level` goes through the bot's own handlers to a fake Bot API, over a temporary database
//! holding synthetic settled XP and rollups, and the bench's manual clock decides the study day.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use deck_streak_kernel::{Db, UtcMillis};
use fake_bot_api::{Bench, ScriptedSync, incoming, owner_says, payload};
use serde_json::Value;

/// The bench's study day at its start: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;
/// A day in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// The run the seeded rollups earn: five on-pace days before today.
const RUN_DAYS: i64 = 5;

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

/// Writes today's settled XP: 30 language and 12 law, 42 in all.
async fn seed_xp(db: &Db) {
    let mut write = db.write().await.expect("a write");
    for (source, track, amount) in [("reviews", "language", 30), ("reviews_law", "law", 12)] {
        sqlx::query(
            "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
             VALUES (?1, ?2, ?3, ?4, 0, 1000)",
        )
        .bind(TODAY)
        .bind(source)
        .bind(track)
        .bind(amount)
        .execute(&mut *write)
        .await
        .expect("the synthetic settlement is written");
    }
    write.commit().await.expect("the commit");
}

/// The text of the last `sendMessage`.
fn last_text(bench: &Bench) -> String {
    let sent: Value = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    sent["text"].as_str().expect("a text").to_owned()
}

#[tokio::test]
async fn level_shows_the_title_and_the_consistency_bonus() {
    let bench = Bench::start().await;
    seed_xp(&bench.db).await;
    bench
        .clock
        .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
    let mut commands = bench.commands(ScriptedSync::default());

    // With no run behind it the multiplier is 1.0, and the answer has no consistency line.
    commands.handle(incoming(owner_says(1, "/level"))).await;
    let plain = last_text(&bench);
    for line in ["Level 1: Sprout", "42", "42/100"] {
        assert!(plain.contains(line), "{line} in {plain}");
    }
    assert!(!plain.contains("Consistency"), "no run, no line: {plain}");

    // Five on-pace days make the multiplier 1.75, and the answer says so.
    for day in TODAY - RUN_DAYS..TODAY {
        seed(&bench.db, day, 20, 20, 90.0, 90).await;
    }
    commands.handle(incoming(owner_says(2, "/level"))).await;
    let boosted = last_text(&bench);
    assert!(boosted.contains("Consistency"), "{boosted}");
    assert!(boosted.contains("1.75"), "{boosted}");
    assert!(boosted.contains("5-day run"), "{boosted}");
}
