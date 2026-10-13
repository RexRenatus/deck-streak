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

use deck_streak_bot::MiniAppUrl;
use deck_streak_bot::progress_commands::progress_reply;
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

/// SPEC-408 R3, R8, A4. The reply for stored courses ends with the course sentence in italics, and
/// the sentence is the English message file's `progress_mastery_about`.
#[tokio::test]
async fn the_progress_reply_ends_with_the_course_mastery_description() {
    const ABOUT: &str = "Mastery is an estimate from your reviews: the average, over the cards it counts, of how likely you are to recall each card now, with cards not yet firmly learned counted for less and new or suspended cards counted as 0.";
    let bench = Bench::start().await;
    let mut commands = bench
        .commands(ScriptedSync::default())
        .with_courses(courses());
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
    commands.handle(incoming(owner_says(1, "/progress"))).await;
    let sent = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    let text = sent["text"].as_str().expect("the reply text");
    assert_eq!(
        text.lines().last(),
        Some(format!("<i>{ABOUT}</i>").as_str())
    );
    assert!(
        !ABOUT.contains(['<', '>', '&']),
        "the sentence needs no escaping"
    );
    let messages: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/app/messages/en.json"
        ))
        .expect("the English message file"),
    )
    .expect("the English message file parses");
    assert_eq!(messages["progress_mastery_about"].as_str(), Some(ABOUT));
}

/// Before a recompute stores a configured course, the command says the courses appear after the
/// next sync, and keeps the button into the Mini App. Mutation coverage beside A18, not a
/// criterion.
#[tokio::test]
async fn progress_with_no_course_stored_answers_the_none_golden() {
    let bench = Bench::start().await;
    let mut commands = bench
        .commands(ScriptedSync::default())
        .with_courses(courses());
    store(&bench.db, "zz", ("Stale", "z"), "C1", 99.0, Some(1)).await;
    commands.handle(incoming(owner_says(1, "/progress"))).await;
    let sent = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    assert_eq!(
        sent,
        golden_send("progress-none"),
        "a stale row of a course no longer configured is no course"
    );
}

/// When the stored progress cannot be read, the command says so and shows nothing. Mutation
/// coverage beside A18, not a criterion.
#[tokio::test]
async fn progress_that_cannot_be_read_answers_the_failed_golden() {
    let bench = Bench::start().await;
    let mut commands = bench
        .commands(ScriptedSync::default())
        .with_courses(courses());
    let mut write = bench.db.write().await.expect("a write");
    sqlx::query("ALTER TABLE language_progress RENAME TO gone_language_progress")
        .execute(&mut *write)
        .await
        .expect("the table is renamed away");
    write.commit().await.expect("the commit");
    commands.handle(incoming(owner_says(1, "/progress"))).await;
    let sent = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    assert_eq!(sent, golden_send("progress-failed"));
}

/// The button's URL appends the page to the Mini App's path and keeps its query and fragment,
/// with or without a trailing slash. Mutation coverage beside A18, not a criterion.
#[test]
fn the_progress_button_keeps_the_mini_apps_query_and_fragment() {
    let cases = [
        (
            "https://deckstreak.example/app/?v=2#top",
            "https://deckstreak.example/app/progress?v=2#top",
        ),
        (
            "https://deckstreak.example#top",
            "https://deckstreak.example/progress#top",
        ),
        (
            "https://deckstreak.example/app?v=2",
            "https://deckstreak.example/app/progress?v=2",
        ),
    ];
    for (app, expected) in cases {
        let reply = progress_reply(&[], &MiniAppUrl::new(app).expect("an https URL"));
        let keyboard = reply.keyboard.expect("the button");
        let url = keyboard.inline_keyboard[0][0]
            .web_app
            .as_ref()
            .map(|web_app| web_app.url.as_str());
        assert_eq!(url, Some(expected), "{app}");
    }
}
