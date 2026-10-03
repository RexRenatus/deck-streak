//! The owner's progress command (SPEC-077 A18; R15, R16): it states each configured course's band,
//! mastery and current unit as the progress route does, ordered by name, never a stale row of a
//! course no longer configured, with the button that opens Road to C2 in the Mini App.
//!
//! The command goes through the bot's own handlers to a fake Bot API, over a temporary database
//! holding synthetic stored progress, and the courses are synthetic.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use deck_streak_kernel::{Courses, Db};
use fake_bot_api::{APP_URL, Bench, ScriptedSync, golden_send, incoming, owner_says, payload};

/// Synthetic courses: `be` and `ga` are configured; `zz` is not.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"be","name":"Beta","flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4]}},
{"code":"ga","name":"Gamma","flag":"g","deck_root":"Gamma Course","alias":"g","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// The synthetic courses.
fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

/// Writes one course's stored progress as the progress step stores it.
async fn store(
    db: &Db,
    course: &str,
    (name, flag): (&str, &str),
    band: &str,
    mastery: f64,
    unit: Option<i64>,
) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO language_progress (course, name, flag, mastery_pct, current_band, \
         mature_cards, total_cards, current_unit, bands, updated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 14, 30, ?6, '[]', 1000, 1000)",
    )
    .bind(course)
    .bind(name)
    .bind(flag)
    .bind(mastery)
    .bind(band)
    .bind(unit)
    .execute(&mut *write)
    .await
    .expect("the synthetic progress is written");
    write.commit().await.expect("the commit");
}

#[tokio::test]
async fn progress_shows_each_course_band_and_mastery() {
    let bench = Bench::start().await;
    let mut commands = bench
        .commands(ScriptedSync::default())
        .with_courses(courses());
    // Ordered by name, not by code; the first name carries markup the reply escapes, and the
    // second course has no unit yet. `zz` is a stale row of a course no longer configured.
    store(
        &bench.db,
        "ga",
        ("Alpha & co", "\u{1f3f3}"),
        "B1",
        61.6,
        Some(12),
    )
    .await;
    store(&bench.db, "be", ("Beta", "\u{1f3f4}"), "A2", 42.5, None).await;
    store(&bench.db, "zz", ("Stale", "z"), "C1", 99.0, Some(1)).await;
    commands.handle(incoming(owner_says(1, "/progress"))).await;
    let sent = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    assert_eq!(sent, golden_send("progress"));
    assert_eq!(
        sent["reply_markup"]["inline_keyboard"][0][0]["web_app"]["url"],
        format!("{APP_URL}/progress"),
        "the button opens the Mini App at Road to C2"
    );
}
