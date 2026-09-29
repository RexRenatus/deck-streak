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
