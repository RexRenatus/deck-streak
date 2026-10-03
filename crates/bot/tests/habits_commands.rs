//! The owner's `/read` and `/undo` through the bot (SPEC-078 A25 to A29; R2, R4): an entry by a
//! course's code, by its alias, by the minutes alone and by the picker then a preset; every refusal
//! answered with nothing logged; the undo of the newest entry and a stale button that removes
//! nothing; every habit button inside Telegram's bound and answered; and the two commands in the
//! owner's menu.
//!
//! Each command goes through the bot's own handlers to a fake Bot API, over a temporary database.
//! Every course is synthetic.

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use std::sync::Arc;

use deck_streak_bot::commands::{Commands, MENU};
use deck_streak_bot::habits_commands::{
    HABIT_PREFIX, HabitCallback, course_data, logged_reply, minutes_data, parse_callback,
    pick_course_reply, presets_reply, undo_data,
};
use deck_streak_coordination::habits::{Logged, READING_PRESETS};
use deck_streak_kernel::{CourseCode, Courses, Db, StudyDayRule};
use deck_streak_notifications::{Policy, Router};
use fake_bot_api::{Bench, ScriptedSync, golden_send, incoming, owner_says, owner_taps, payload};
use serde_json::Value;

/// Telegram's bound on a callback's data, in bytes.
const MAX_CALLBACK_DATA: usize = 64;

/// Two synthetic courses: `qaa` (alias `a`) and `qab` (alias `b`).
const COURSES: &str = r#"{
  "schema": "deckstreak.courses.v1",
  "courses": [
    {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
     "writing": false, "unit_bands": {}},
    {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab", "alias": "b",
     "writing": true, "unit_bands": {}}
  ],
  "focus_subjects": []
}"#;

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

fn code(text: &str) -> CourseCode {
    CourseCode::new(text).expect("a synthetic course code")
}

/// The bench's handlers, logging against `courses` through a router of the bench's own.
fn habit_commands(bench: &Bench, courses: Courses) -> Commands<ScriptedSync> {
    let router = Router::new(
        Arc::new(Policy::compiled().expect("the compiled policy parses")),
        bench.db.clone(),
        bench.clock.clone(),
        StudyDayRule::default(),
    );
    bench
        .commands(ScriptedSync::default())
        .with_habits(courses, Arc::new(router))
}

fn last_send(bench: &Bench) -> Value {
    payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"))
}

fn sends(bench: &Bench) -> usize {
    bench.fake.calls_of("sendMessage").len()
}

/// The text of the last send.
fn last_text(bench: &Bench) -> String {
    last_send(bench)["text"]
        .as_str()
        .expect("a text")
        .to_owned()
}

/// Every callback's data on the last send's keyboard.
fn last_buttons(bench: &Bench) -> Vec<String> {
    last_send(bench)["reply_markup"]["inline_keyboard"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .flat_map(|row| row.as_array().cloned().unwrap_or_default())
                .filter_map(|button| button["callback_data"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The minutes log's entries, as course and minutes, oldest first.
async fn entries(db: &Db) -> Vec<(String, i64)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query_as("SELECT code, minutes FROM minutes_log ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the entries read")
}

/// A trigger that makes the database refuse every new entry, as a write that fails would.
const REFUSE_INSERT: &str = "CREATE TRIGGER refuse_insert BEFORE INSERT ON minutes_log \
                             BEGIN SELECT RAISE(ABORT, 'refused'); END";

/// A trigger that makes the database refuse every removal, as a write that fails would.
const REFUSE_DELETE: &str = "CREATE TRIGGER refuse_delete BEFORE DELETE ON minutes_log \
                             BEGIN SELECT RAISE(ABORT, 'refused'); END";

/// Installs `trigger`, so the database refuses a write to the minutes log.
async fn refuse(db: &Db, trigger: &'static str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(trigger)
        .execute(&mut *write)
        .await
        .expect("the trigger");
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn read_logs_minutes_and_answers_with_the_days_xp() {
    let bench = Bench::start().await;
    let mut commands = habit_commands(&bench, courses());

    // Minutes alone, before any entry: there is no most-used course, so the bot asks which.
    commands.handle(incoming(owner_says(1, "/read 15"))).await;
    assert_eq!(last_send(&bench), golden_send("read-pick-course"), "no course yet");
    assert!(entries(&bench.db).await.is_empty(), "nothing logged yet");

    // By code, then by alias with a note.
    commands.handle(incoming(owner_says(2, "/read qaa 20"))).await;
    assert_eq!(last_send(&bench), golden_send("read-logged"), "by code");
    commands
        .handle(incoming(owner_says(3, "/read a 190 with a <note> & more")))
        .await;
    assert_eq!(
        last_send(&bench),
        golden_send("read-goal-reached"),
        "by alias, reaching the week's goal"
    );

    // The minutes alone, for the most-used course.
    commands.handle(incoming(owner_says(4, "/READ 15"))).await;
    let expected = logged_reply(
        &Logged {
            entry_id: 3,
            code: code("qaa"),
            minutes: 15,
            day_minutes: 225,
            day_xp: 240,
            week_minutes: 225,
            goal_bonus: 150,
        },
        "Course Qaa",
    );
    assert_eq!(last_text(&bench), expected.text, "the most-used course");
    assert_eq!(last_buttons(&bench), [undo_data(3)]);

    // The picker, then a preset.
    commands.handle(incoming(owner_says(5, "/read"))).await;
    assert_eq!(last_send(&bench), golden_send("read-pick-course"), "bare /read");
    commands
        .handle(incoming(owner_taps(6, &course_data(&code("qab")), 5)))
        .await;
    assert_eq!(last_send(&bench), golden_send("read-presets"), "the presets");
    commands
        .handle(incoming(owner_taps(7, &minutes_data(&code("qab"), 45), 6)))
        .await;
    assert!(
        last_text(&bench).starts_with("Logged 45 minutes of reading for <b>Course Qab</b>."),
        "{}",
        last_text(&bench)
    );
    assert_eq!(last_buttons(&bench), [undo_data(4)]);

    assert_eq!(
        entries(&bench.db).await,
        [
            ("qaa".to_owned(), 20),
            ("qaa".to_owned(), 190),
            ("qaa".to_owned(), 15),
            ("qab".to_owned(), 45),
        ],
        "four entries, each for its course"
    );
}

#[tokio::test]
async fn a_refused_read_answers_and_logs_nothing() {
    let bench = Bench::start().await;
    let mut commands = habit_commands(&bench, courses());
    let mut update = 0;
    for (text, golden) in [
        ("/read qaa 0", "read-refused-minutes"),
        ("/read qaa 601", "read-refused-minutes"),
        ("/read qaa ten", "read-refused-minutes"),
        ("/read x<y 20", "read-unknown-course"),
        ("/read qaa", "read-usage"),
    ] {
        update += 1;
        commands.handle(incoming(owner_says(update, text))).await;
        assert_eq!(last_send(&bench), golden_send(golden), "{text}");
    }
    assert_eq!(sends(&bench), 5, "each refusal is answered once");

    // A refused database: the entry's write fails whole.
    refuse(&bench.db, REFUSE_INSERT).await;
    commands.handle(incoming(owner_says(7, "/read qaa 20"))).await;
    assert_eq!(last_send(&bench), golden_send("read-failed"), "a refused write");
    assert!(entries(&bench.db).await.is_empty(), "nothing was logged");

    // No courses configured.
    let mut bare = habit_commands(&bench, Courses::default());
    for (update, text) in [(8, "/read qaa 20"), (9, "/read 20"), (10, "/read")] {
        bare.handle(incoming(owner_says(update, text))).await;
        assert_eq!(last_send(&bench), golden_send("read-no-courses"), "{text}");
    }
    assert!(entries(&bench.db).await.is_empty(), "nothing was logged");
}

#[tokio::test]
async fn undo_answers_for_the_newest_entry_and_a_stale_button_removes_nothing() {
    let bench = Bench::start().await;
    let mut commands = habit_commands(&bench, courses());
    commands.handle(incoming(owner_says(1, "/undo"))).await;
    assert_eq!(last_send(&bench), golden_send("undo-nothing"), "an empty log");

    commands.handle(incoming(owner_says(2, "/read qaa 20"))).await;
    let first = last_buttons(&bench);
    assert_eq!(first, [undo_data(1)], "the first entry's button");
    commands.handle(incoming(owner_says(3, "/read qaa 30"))).await;

    // The first entry's button, once a newer entry exists.
    commands.handle(incoming(owner_taps(4, &first[0], 2))).await;
    assert_eq!(last_send(&bench), golden_send("undo-stale"), "a stale button");
    assert_eq!(entries(&bench.db).await.len(), 2, "nothing was removed");

    // `/undo` removes the newest; the first entry's button then removes the first.
    commands.handle(incoming(owner_says(5, "/undo"))).await;
    assert_eq!(last_send(&bench), golden_send("undo-done"), "the newest entry");
    assert_eq!(entries(&bench.db).await, [("qaa".to_owned(), 20)]);
    commands.handle(incoming(owner_taps(6, &first[0], 2))).await;
    assert!(
        last_text(&bench).starts_with("Removed 20 minutes of reading for <b>Course Qaa</b>."),
        "{}",
        last_text(&bench)
    );
    assert!(entries(&bench.db).await.is_empty(), "the log is empty");

    // A refused database: the undo's write fails whole.
    commands.handle(incoming(owner_says(7, "/read qab 10"))).await;
    refuse(&bench.db, REFUSE_DELETE).await;
    commands.handle(incoming(owner_says(8, "/undo"))).await;
    assert_eq!(last_send(&bench), golden_send("undo-failed"), "a refused write");
    assert_eq!(entries(&bench.db).await, [("qab".to_owned(), 10)]);
}

#[tokio::test]
async fn every_habit_callback_fits_telegrams_bound_and_is_answered() {
    let longest = code("qaaaaaaa");
    let mut data = vec![
        (course_data(&longest), HabitCallback::Course(longest)),
        (undo_data(i64::MAX), HabitCallback::Undo(i64::MAX)),
        (undo_data(1), HabitCallback::Undo(1)),
    ];
    for minutes in READING_PRESETS {
        data.push((
            minutes_data(&longest, minutes),
            HabitCallback::Minutes {
                code: longest,
                minutes,
            },
        ));
    }
    let picker = pick_course_reply(&courses());
    let presets = presets_reply(&longest, "Course Qaa");
    let rendered: Vec<String> = [picker, presets]
        .iter()
        .filter_map(|reply| reply.keyboard.as_ref())
        .flat_map(|keyboard| keyboard.inline_keyboard.iter().flatten())
        .filter_map(|button| button.callback_data.clone())
        .collect();
    assert_eq!(rendered.len(), 2 + READING_PRESETS.len(), "every rendered button");
    println!("examined {} callback data", data.len() + rendered.len());
    for (text, callback) in &data {
        assert!(
            !text.is_empty() && text.len() <= MAX_CALLBACK_DATA && text.starts_with(HABIT_PREFIX),
            "{text}: {} bytes",
            text.len()
        );
        assert_eq!(parse_callback(text).as_ref(), Some(callback), "{text}");
    }
    for text in &rendered {
        assert!(text.len() <= MAX_CALLBACK_DATA, "{text}");
        assert!(parse_callback(text).is_some(), "{text}");
    }
    for refused in ["hb:", "hb:c:", "hb:c:QAA", "hb:m:qaa", "hb:m:qaa:x", "hb:u:", "hb:u:x"] {
        assert_eq!(parse_callback(refused), None, "{refused}");
    }

    // Every habit tap is answered, a malformed one included.
    let bench = Bench::start().await;
    let mut commands = habit_commands(&bench, courses());
    let taps = [
        course_data(&code("qaa")),
        minutes_data(&code("qaa"), 10),
        undo_data(1),
        "hb:x".to_owned(),
    ];
    for (update, data) in (1..).zip(&taps) {
        commands.handle(incoming(owner_taps(update, data, 1))).await;
    }
    assert_eq!(
        bench.fake.calls_of("answerCallbackQuery").len(),
        taps.len(),
        "every tap is answered"
    );
}

#[tokio::test]
async fn the_reading_commands_join_the_owners_menu() {
    let bench = Bench::start().await;
    let commands = habit_commands(&bench, courses());
    commands.register_menu().await;
    let calls = bench.fake.calls_of("setMyCommands");
    let registered = calls[0].body["commands"].as_array().expect("commands");
    for (command, described) in [
        ("read", "Log reading minutes"),
        ("undo", "Undo the last reading entry"),
    ] {
        let entry = MENU.iter().find(|entry| entry.command == command);
        assert_eq!(
            entry.map(|entry| entry.description),
            Some(described),
            "/{command} is in the menu"
        );
        assert!(!described.is_empty() && described.len() <= 256);
        assert!(
            registered
                .iter()
                .any(|sent| sent["command"] == command && sent["description"] == described),
            "/{command} is registered for the owner's chat"
        );
    }
}
