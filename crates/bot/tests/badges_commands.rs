//! The owner's `/badges` and `/records` (SPEC-073 A21, A22; R18): `/badges` lists the 20 most
//! recently awarded badges, newest first, each its emoji and name, or the predecessor's line when
//! none is earned; `/records` lists each record with its value, the day that set it and the value
//! it beat, then names the record today is closest to.
//!
//! Each command goes through the bot's own handlers to a fake Bot API, over a temporary database
//! holding synthetic awards, records and today's rollup, and the bench's manual clock decides the
//! study day.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use deck_streak_kernel::{Db, UtcMillis};
use fake_bot_api::{Bench, ScriptedSync, incoming, owner_says, payload};
use serde_json::Value;

/// The bench's study day once the clock is set: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;
/// A day in milliseconds.
const DAY_MS: i64 = 86_400_000;

/// Writes one earned badge, `Badge <n>` with a synthetic key, awarded at `n` seconds.
async fn award(db: &Db, n: i64, name: &str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO badges_earned (badge_key, tier, name, emoji, study_day, celebrated_at, \
         created_at) VALUES (?1, 0, ?2, ?3, ?4, ?5, ?5)",
    )
    .bind(format!("synthetic_{n:02}"))
    .bind(name)
    .bind("\u{1f396}")
    .bind(TODAY - 30 + n)
    .bind(n * 1000)
    .execute(&mut *write)
    .await
    .expect("the synthetic badge is written");
    write.commit().await.expect("the commit");
}

/// Writes one stored record.
async fn record(db: &Db, kind: &str, value: i64, day: i64, previous: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, 1000, 1000)",
    )
    .bind(kind)
    .bind(value)
    .bind(day)
    .bind(previous)
    .execute(&mut *write)
    .await
    .expect("the synthetic record is written");
    write.commit().await.expect("the commit");
}

/// Writes today's rollup: 42 study reviews in 600 seconds, and a score of 77.
async fn seed_today(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, 1, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, 30, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    write.commit().await.expect("the commit");
}

/// The text of the last `sendMessage`.
fn last_text(bench: &Bench) -> String {
    let sent: Value = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    sent["text"].as_str().expect("a text").to_owned()
}

/// `/badges`' list of the awards numbered `newest` down to `oldest`, newest first.
fn listed(newest: i64, oldest: i64) -> String {
    let lines: Vec<String> = (oldest..=newest)
        .rev()
        .map(|n| format!("\u{1f396} {}", name(n).replace('&', "&amp;")))
        .collect();
    format!("\u{1f3c5} <b>Badges</b>\n{}", lines.join("\n"))
}

/// The name of award `n`; the seventh carries markup the reply must escape.
fn name(n: i64) -> String {
    if n == 7 {
        "Badge 07 & co".to_owned()
    } else {
        format!("Badge {n:02}")
    }
}

#[tokio::test]
async fn badges_lists_the_twenty_most_recent() {
    let bench = Bench::start().await;
    bench
        .clock
        .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
    let mut commands = bench.commands(ScriptedSync::default());

    // None earned: the predecessor's line.
    commands.handle(incoming(owner_says(1, "/badges"))).await;
    assert_eq!(
        last_text(&bench),
        "No badges yet \u{2014} study to earn your first! \u{1f45f}"
    );

    // One, then twenty: every one, newest first.
    award(&bench.db, 1, &name(1)).await;
    commands.handle(incoming(owner_says(2, "/badges"))).await;
    assert_eq!(last_text(&bench), listed(1, 1));
    for n in 2..=20 {
        award(&bench.db, n, &name(n)).await;
    }
    commands.handle(incoming(owner_says(3, "/badges"))).await;
    assert_eq!(last_text(&bench), listed(20, 1));

    // A twenty-first: the twenty most recent, and the oldest is left out.
    award(&bench.db, 21, &name(21)).await;
    commands.handle(incoming(owner_says(4, "/badges"))).await;
    assert_eq!(last_text(&bench), listed(21, 2));
}

#[tokio::test]
async fn records_names_the_record_to_chase() {
    let bench = Bench::start().await;
    bench
        .clock
        .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
    let mut commands = bench.commands(ScriptedSync::default());

    // None stored: the predecessor's line.
    commands.handle(incoming(owner_says(1, "/records"))).await;
    assert_eq!(
        last_text(&bench),
        "\u{1f4c8} No records yet \u{2014} they mint themselves as you study."
    );

    // Today holds 77 points, 42 reviews and ten minutes. The score record is already passed, so
    // the closest record ahead is the minutes', two away, though it is listed last.
    seed_today(&bench.db).await;
    record(&bench.db, "best_score", 70, TODAY - 5, 60).await;
    record(&bench.db, "most_reviews", 300, TODAY - 4, 250).await;
    record(&bench.db, "most_minutes", 12, TODAY - 3, 9).await;
    commands.handle(incoming(owner_says(2, "/records"))).await;
    assert_eq!(
        last_text(&bench),
        "\u{1f3c5} <b>Personal records</b>\n\
         \u{2022} Best daily score: <b>70</b> (2025-01-09, was 60)\n\
         \u{2022} Most reviews in a day: <b>300</b> (2025-01-10, was 250)\n\
         \u{2022} Most minutes in a day: <b>12</b> (2025-01-11, was 9)\n\
         \u{1f3c3} Chase it: 2 from \u{201c}Most minutes in a day\u{201d} today."
    );
}
