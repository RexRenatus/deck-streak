//! The owner's `/streak` (SPEC-076 A21; R22): both tracks, the law track first when it has activity,
//! with the language track's heat and freezes.
//!
//! Each `/streak` goes through the bot's own handlers to a fake Bot API, over a temporary database
//! holding synthetic streak rows, and the bench's manual clock decides the study day.

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

/// Writes the two tracks: the language streak at `language` days with two freezes, and the law
/// streak at `law` days, each last studied the day before.
async fn seed(db: &Db, language: i64, law: i64) {
    let mut write = db.write().await.expect("a write");
    for (track, current, freezes) in [("language", language, 2), ("law", law, 0)] {
        sqlx::query(
            "INSERT INTO streak_state \
             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
             VALUES (?1, ?2, ?2, ?3, ?4, 0, 1000)",
        )
        .bind(track)
        .bind(current)
        .bind(freezes)
        .bind(TODAY - 1)
        .execute(&mut *write)
        .await
        .expect("the synthetic streak row is written");
    }
    write.commit().await.expect("the commit");
}

/// The text of the last `sendMessage`.
fn last_text(bench: &Bench) -> String {
    let sent: Value = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    sent["text"].as_str().expect("a text").to_owned()
}

#[tokio::test]
async fn streak_shows_both_tracks_law_first_when_law_is_active() {
    let bench = Bench::start().await;
    seed(&bench.db, 12, 4).await;
    bench
        .clock
        .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
    let mut commands = bench.commands(ScriptedSync::default());

    // The law track is active, so it comes first, and the language track shows its freezes.
    commands.handle(incoming(owner_says(1, "/streak"))).await;
    let text = last_text(&bench);
    let law = text.find("Law: 4").expect("the law track is shown");
    let language = text
        .find("Language: 12")
        .expect("the language track is shown");
    assert!(law < language, "law first in {text}");
    assert!(text.contains("2 freezes"), "the freezes in {text}");
}

/// A rule (A36): the law track leads exactly when its run is above zero, and the language track's
/// noun is singular exactly at one freeze. Law runs 0, 1 and 2 (the boundary -1, 0, +1 of "above
/// zero") over every language freeze count from zero to three, each read through `/streak`.
#[tokio::test]
async fn the_law_leads_only_above_zero_and_the_noun_follows_the_freezes() {
    let mut cases = 0_u32;
    for law in [0_i64, 1, 2] {
        for freezes in 0..=3_i64 {
            let bench = Bench::start().await;
            {
                let mut write = bench.db.write().await.expect("a write");
                for (track, current, held) in [("language", 9_i64, freezes), ("law", law, 0)] {
                    sqlx::query(
                        "INSERT INTO streak_state \
                         (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
                         VALUES (?1, ?2, ?2, ?3, ?4, 0, 1000)",
                    )
                    .bind(track)
                    .bind(current)
                    .bind(held)
                    .bind(TODAY - 1)
                    .execute(&mut *write)
                    .await
                    .expect("the synthetic streak row is written");
                }
                write.commit().await.expect("the commit");
            }
            bench
                .clock
                .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
            let mut commands = bench.commands(ScriptedSync::default());
            commands.handle(incoming(owner_says(1, "/streak"))).await;
            let text = last_text(&bench);
            let law_at = text.find("Law: ").expect("the law line");
            let language_at = text.find("Language: 9").expect("the language line");
            assert_eq!(law_at < language_at, law > 0, "law {law} in {text}");
            let noun = if freezes == 1 {
                "1 freeze)"
            } else {
                "freezes)"
            };
            assert!(text.contains(noun), "freezes {freezes} in {text}");
            assert!(text.contains(&format!("Law: {law} (best {law})")), "{text}");
            assert!(text.starts_with("<b>Streaks</b>\n"), "{text}");
            cases += 1;
        }
    }
    println!("streak-reply population: {cases} replies");
    assert_eq!(cases, 12);
}

/// A rule (A42): each track's line names its own run and its own best, and the law track leads by
/// its run, never by its best. Law runs 0, 1 and 2, each with a best of the run, one more and seven
/// more, over language bests of 9, 10 and 16 on a language run of 9, each pair stored as its own row
/// and read through `/streak`. A run equal to its best hides a swap of the two, and a law run of zero
/// with a best above zero is the state in which leading by the best differs from leading by the run.
#[tokio::test]
async fn each_line_names_its_own_run_and_best_and_the_law_leads_by_its_run() {
    let mut cases = 0_u32;
    let mut idle_law_with_a_best = 0_u32;
    for law in [0_i64, 1, 2] {
        for law_more in [0_i64, 1, 7] {
            for language_more in [0_i64, 1, 7] {
                let law_best = law + law_more;
                let language_best = 9 + language_more;
                let bench = Bench::start().await;
                {
                    let mut write = bench.db.write().await.expect("a write");
                    for (track, current, longest, held) in [
                        ("language", 9_i64, language_best, 2_i64),
                        ("law", law, law_best, 0),
                    ] {
                        sqlx::query(
                            "INSERT INTO streak_state \
                             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1000)",
                        )
                        .bind(track)
                        .bind(current)
                        .bind(longest)
                        .bind(held)
                        .bind(TODAY - 1)
                        .execute(&mut *write)
                        .await
                        .expect("the synthetic streak row is written");
                    }
                    write.commit().await.expect("the commit");
                }
                bench
                    .clock
                    .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
                let mut commands = bench.commands(ScriptedSync::default());
                commands.handle(incoming(owner_says(1, "/streak"))).await;
                let text = last_text(&bench);
                let lines: Vec<&str> = text.split('\n').collect();
                assert_eq!(lines.len(), 3, "{text}");
                assert_eq!(lines[0], "<b>Streaks</b>", "{text}");
                let (law_line, language_line) = if law > 0 {
                    (lines[1], lines[2])
                } else {
                    (lines[2], lines[1])
                };
                assert_eq!(
                    law_line,
                    format!("Law: {law} (best {law_best})"),
                    "law {law} best {law_best} in {text}"
                );
                assert!(
                    language_line.starts_with("Language: 9"),
                    "language best {language_best} in {text}"
                );
                assert!(
                    language_line.ends_with(&format!(" (best {language_best}, 2 freezes)")),
                    "language best {language_best} in {text}"
                );
                cases += 1;
                if law == 0 && law_best > 0 {
                    idle_law_with_a_best += 1;
                }
            }
        }
    }
    println!(
        "streak-reply best population: {cases} replies, {idle_law_with_a_best} with an idle law \
         run and a best"
    );
    assert_eq!(cases, 27);
    assert_eq!(idle_law_with_a_best, 6);
}
