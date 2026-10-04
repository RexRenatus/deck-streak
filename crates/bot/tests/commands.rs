//! The owner's commands: `/start` says the coach is an AI and offers the Mini App; the menu is the
//! owner's chat's alone; `/export` sends the owner's data as a JSON document; `/delete` erases only
//! after the owner confirms; `/sync` runs the owner's sync now; and every message the bot renders is
//! its committed golden (SPEC-026 A5, A6, A13, A14, A15; R11, R12).
//!
//! Each command goes through the bot's own handlers to a fake Bot API, over a temporary database
//! the data-rights use cases read and erase.

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use std::collections::BTreeSet;

use deck_streak_bot::badges_commands::{badges_failed_reply, records_failed_reply};
use deck_streak_bot::commands::{
    CONFIRM_ERASE, EXPORT_FILE_NAME, MENU, MINI_APP_URL, Reply, erase_done_reply,
    erase_failed_reply, export_caption, export_failed_reply, help_reply, sync_reply,
};
use deck_streak_bot::habits_commands::{
    checklist_reply, logged_reply, no_courses_reply, no_writing_course_reply,
    not_a_writing_course_reply, pick_course_reply, presets_reply, read_failed_reply,
    refused_minutes_reply, undo_done_reply, undo_failed_reply, undo_nothing_reply,
    undo_stale_reply, unknown_course_reply, usage_reply, write_cleared_reply,
    write_confirmed_reply, write_day_closed_reply, write_failed_reply,
};
use deck_streak_bot::progress_commands::{progress_failed_reply, progress_reply};
use deck_streak_bot::score_commands::{score_failed_reply, score_reply};
use deck_streak_bot::{MiniAppUrl, Scores, Sent, SyncAnswer, SyncOutcome, SyncRefusal};
use deck_streak_coordination::data_rights_registry::export_all;
use deck_streak_coordination::habits::{Checklist, ChecklistLine, Logged};
use deck_streak_coordination::progress_view::StoredProgress;
use deck_streak_coordination::score::{DayScore, Pillars};
use deck_streak_kernel::{CourseCode, Courses, Db, StudyDay, UtcMillis};
use deck_streak_kernel::{Environment, SettingsError};
use fake_bot_api::{
    APP_URL, Bench, OWNER, STRANGER, ScriptedSync, golden_send, incoming, messages_directory,
    owner_says, owner_taps, payload, tap,
};
use serde_json::{Value, json};

/// The settings generation, which an erase resets to 0.
async fn generation(db: &Db) -> i64 {
    db.settings_generation()
        .await
        .expect("the generation reads")
}

/// Bumps the settings generation once, so an erase has something to reset.
async fn bump(db: &Db) {
    let mut write = db.write().await.expect("a write");
    Db::bump_settings_generation(&mut write)
        .await
        .expect("the bump");
    write.commit().await.expect("the commit");
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The payload of the last `sendMessage`.
fn last_send(bench: &Bench) -> Value {
    payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"))
}

#[tokio::test]
async fn start_says_the_coach_is_an_ai_and_offers_the_mini_app() {
    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::default());
    commands.handle(incoming(owner_says(1, "/start"))).await;

    let sent = last_send(&bench);
    assert_eq!(sent, golden_send("start"), "the /start golden");
    let text = sent["text"].as_str().expect("a text");
    assert_eq!(
        text.lines().next(),
        Some("Your coach here is an AI, not a person."),
        "its first line says the coach is an AI"
    );
    assert_eq!(
        sent["reply_markup"]["inline_keyboard"][0][0]["web_app"]["url"], APP_URL,
        "the button opens the Mini App's configured URL"
    );
    // The command is read as Telegram writes it in a group menu, and in any case.
    commands
        .handle(incoming(owner_says(2, "/START@SyntheticBot")))
        .await;
    assert_eq!(last_send(&bench), golden_send("start"));
}

#[tokio::test]
async fn the_menu_is_registered_for_the_owners_chat_only() {
    let bench = Bench::start().await;
    let commands = bench.commands(ScriptedSync::default());
    commands.register_menu().await;

    let calls = bench.fake.calls();
    let methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
    assert_eq!(
        methods,
        vec!["deleteMyCommands", "setMyCommands"],
        "the public menu goes first, then the owner's is set"
    );
    assert_eq!(calls[0].body, json!({"scope": {"type": "default"}}));
    assert_eq!(
        calls[1].body["scope"],
        json!({"type": "chat", "chat_id": OWNER}),
        "the owner's chat alone"
    );
    let registered: BTreeSet<&str> = calls[1].body["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .map(|command| command["command"].as_str().expect("a command"))
        .collect();
    assert_eq!(
        registered,
        BTreeSet::from([
            "privacy", "export", "delete", "sync", "score", "level", "streak", "badges", "records",
            "progress", "drills", "drill", "read", "undo", "write", "unwrite"
        ]),
        "the sixteen commands of the menu"
    );
    for (entry, command) in MENU
        .iter()
        .zip(calls[1].body["commands"].as_array().expect("commands"))
    {
        assert_eq!(command["description"], entry.description);
        assert!(!entry.description.is_empty() && entry.description.len() <= 256);
    }
}

#[tokio::test]
async fn export_sends_the_owners_data_as_a_json_document() {
    let bench = Bench::start().await;
    bump(&bench.db).await;
    let mut commands = bench.commands(ScriptedSync::default());
    commands.handle(incoming(owner_says(1, "/export"))).await;

    let methods: Vec<String> = bench
        .fake
        .calls()
        .into_iter()
        .map(|call| call.method)
        .collect();
    assert_eq!(
        methods,
        vec!["sendChatAction", "sendDocument"],
        "typing, then the document"
    );
    assert_eq!(bench.fake.calls()[0].body["action"], "typing");
    let upload = bench
        .fake
        .calls_of("sendDocument")
        .pop()
        .expect("the upload")
        .body;
    assert_eq!(upload["chat_id"], OWNER.to_string());
    assert_eq!(upload["document"]["filename"], EXPORT_FILE_NAME);
    assert_eq!(upload["document"]["content_type"], "application/json");
    let document: Value =
        serde_json::from_str(upload["document"]["text"].as_str().expect("a text"))
            .expect("the document is JSON");
    assert_eq!(document["schema"], "deckstreak.export.v1");
    assert_eq!(
        document,
        *export_all(&bench.db).await.expect("the export").as_json(),
        "the document is the owner's export"
    );
    assert_eq!(
        document["settings_generation"][0]["generation"], 1,
        "it holds the owner's data: {document}"
    );
    let mut caption = upload.clone();
    if let Some(fields) = caption.as_object_mut() {
        fields.remove("chat_id");
        fields.remove("document");
    }
    assert_eq!(caption, golden_send("export"), "the caption's golden");
}

#[tokio::test]
async fn delete_erases_only_after_the_owner_confirms() {
    let bench = Bench::start().await;
    bump(&bench.db).await;
    let mut commands = bench.commands(ScriptedSync::default());

    // The question, with its button: nothing is erased yet.
    commands.handle(incoming(owner_says(1, "/delete"))).await;
    assert_eq!(generation(&bench.db).await, 1, "a question erases nothing");
    let prompt = bench
        .fake
        .calls_of("sendMessage")
        .pop()
        .expect("the prompt");
    assert_eq!(payload(&prompt), golden_send("erase-prompt"));
    let prompt_id = prompt.message_id.expect("the prompt's message id");

    // A stranger's tap on the button, and the owner's tap on another message, erase nothing.
    commands
        .handle(incoming(tap(2, STRANGER, CONFIRM_ERASE, prompt_id)))
        .await;
    assert_eq!(generation(&bench.db).await, 1, "a stranger's tap");
    commands
        .handle(incoming(owner_taps(3, CONFIRM_ERASE, prompt_id + 7)))
        .await;
    assert_eq!(generation(&bench.db).await, 1, "a tap on another message");
    assert_eq!(last_send(&bench), golden_send("erase-expired"));

    // The owner's tap on the latest question's button erases, once.
    commands
        .handle(incoming(owner_taps(4, CONFIRM_ERASE, prompt_id)))
        .await;
    assert_eq!(
        generation(&bench.db).await,
        0,
        "the erase reset the generation"
    );
    assert_eq!(last_send(&bench), golden_send("erase-done"));

    // The same button again does nothing more: the confirmation was used.
    bump(&bench.db).await;
    commands
        .handle(incoming(owner_taps(5, CONFIRM_ERASE, prompt_id)))
        .await;
    assert_eq!(
        generation(&bench.db).await,
        1,
        "a used confirmation erases nothing"
    );
    assert_eq!(last_send(&bench), golden_send("erase-expired"));

    // A newer question retires the older one's button.
    commands.handle(incoming(owner_says(6, "/delete"))).await;
    commands.handle(incoming(owner_says(7, "/delete"))).await;
    let questions: Vec<i64> = bench
        .fake
        .calls_of("sendMessage")
        .iter()
        .filter(|call| payload(call) == golden_send("erase-prompt"))
        .map(|call| call.message_id.expect("a question's message id"))
        .collect();
    assert_eq!(questions.len(), 3, "three questions: {questions:?}");
    commands
        .handle(incoming(owner_taps(8, CONFIRM_ERASE, questions[1])))
        .await;
    assert_eq!(
        generation(&bench.db).await,
        1,
        "the older question's button"
    );
    commands
        .handle(incoming(owner_taps(9, CONFIRM_ERASE, questions[2])))
        .await;
    assert_eq!(
        generation(&bench.db).await,
        0,
        "the latest question's button"
    );

    let answered = bench.fake.calls_of("answerCallbackQuery").len();
    assert_eq!(answered, 5, "every tap of the owner's answered");
}

#[tokio::test]
async fn sync_runs_a_cycle_now_and_forces_one_recompute() {
    let bench = Bench::start().await;
    let sync = ScriptedSync::answering([Ok(SyncAnswer {
        sync: SyncOutcome::Synced,
        scores: Scores::Recomputed,
    })]);
    let mut commands = bench.commands(sync.clone());
    commands.handle(incoming(owner_says(1, "/sync"))).await;

    assert_eq!(sync.calls(), 1, "one owner's sync, now");
    let methods: Vec<String> = bench
        .fake
        .calls()
        .into_iter()
        .map(|call| call.method)
        .collect();
    assert_eq!(
        methods,
        vec!["sendChatAction", "sendMessage"],
        "typing, then the answer"
    );
    assert_eq!(bench.fake.calls()[0].body["action"], "typing");
    assert_eq!(last_send(&bench), golden_send("sync-synced"));
    let text = last_send(&bench)["text"]
        .as_str()
        .expect("a text")
        .to_owned();
    assert!(
        text.contains("recomputed"),
        "the answer says the scores were recomputed: {text}"
    );

    // A second /sync runs a second owner's sync: each trigger is its own.
    commands.handle(incoming(owner_says(2, "/sync"))).await;
    assert_eq!(sync.calls(), 2);
    assert_eq!(
        last_send(&bench),
        golden_send("sync-refused"),
        "nothing more was scripted"
    );
}

/// A synthetic day's score: `total` in `grade`, with `reviews` and `retention`.
const fn scored(
    total: i64,
    grade: (&'static str, &'static str),
    reviews: i64,
    retention: Option<f64>,
) -> DayScore {
    DayScore {
        day: StudyDay::from_epoch_day(20_102),
        total,
        grade_label: grade.0,
        grade_emoji: grade.1,
        pillars: Pillars {
            consistency: 76.0,
            retention,
            workload: 70.0,
            volume: 60.5,
            mastery: 55.0,
        },
        reviews,
        retention,
    }
}

/// The Mini App's URL the bench's bot carries.
fn app() -> MiniAppUrl {
    MiniAppUrl::new(APP_URL).expect("an https URL")
}

/// Two courses' stored progress, ordered by name as the progress view orders it: the first name
/// carries markup the reply escapes, and the second course has no unit yet.
fn stored_progress() -> Vec<StoredProgress> {
    let course =
        |course: &str, (name, flag): (&str, &str), band: &str, mastery, unit| StoredProgress {
            course: course.to_owned(),
            name: name.to_owned(),
            flag: flag.to_owned(),
            mastery_pct: mastery,
            current_band: band.to_owned(),
            mature_cards: 14,
            total_cards: 30,
            current_unit: unit,
            bands: Vec::new(),
            updated_at: UtcMillis::from_epoch_millis(1000),
        };
    vec![
        course("ga", ("Alpha & co", "\u{1f3f3}"), "B1", 61.6, Some(12)),
        course("be", ("Beta", "\u{1f3f4}"), "A2", 42.5, None),
    ]
}

/// Two synthetic courses, `qaa` and `qab`, for the reading replies (SPEC-078).
fn habit_courses() -> Courses {
    Courses::parse(
        r#"{"schema": "deckstreak.courses.v1", "courses": [
            {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
             "writing": false, "unit_bands": {}},
            {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab", "alias": "b",
             "writing": true, "unit_bands": {}}
        ], "focus_subjects": []}"#,
    )
    .expect("the synthetic courses parse")
}

/// A logged entry of course `qaa`, for the reading replies.
fn logged(entry_id: i64, minutes: u32, day_minutes: u32, week_minutes: u32) -> Logged {
    Logged {
        entry_id,
        code: CourseCode::new("qaa").expect("a synthetic code"),
        minutes,
        day_minutes,
        day_xp: (day_minutes * 2).min(240),
        week_minutes,
        goal_bonus: if week_minutes >= 210 { 150 } else { 0 },
    }
}

/// The bench day's writing checklist with `qab`, the one writing course, confirmed or not
/// (SPEC-078 R8).
fn qab_checklist(confirmed: bool) -> Checklist {
    let streak = u32::from(confirmed);
    Checklist {
        day: StudyDay::from_epoch_day(20_102),
        lines: vec![ChecklistLine {
            code: CourseCode::new("qab").expect("a synthetic code"),
            confirmed,
            streak,
        }],
        streak,
    }
}

/// Every message the bot renders, by its golden's name.
fn rendered() -> Vec<(&'static str, Reply)> {
    let synced = |sync, scores| Ok(SyncAnswer { sync, scores });
    let qab = CourseCode::new("qab").expect("a synthetic code");
    vec![
        (
            "read-logged",
            logged_reply(&logged(1, 20, 20, 20), "Course Qaa"),
        ),
        (
            "read-goal-reached",
            logged_reply(&logged(2, 190, 210, 210), "Course Qaa"),
        ),
        ("read-pick-course", pick_course_reply(&habit_courses())),
        ("read-presets", presets_reply(&qab, "Course Qab")),
        ("read-usage", usage_reply()),
        ("read-refused-minutes", refused_minutes_reply()),
        ("read-unknown-course", unknown_course_reply("x<y")),
        ("read-no-courses", no_courses_reply()),
        ("read-failed", read_failed_reply()),
        ("undo-done", undo_done_reply("Course Qaa", 30)),
        ("undo-nothing", undo_nothing_reply()),
        ("undo-stale", undo_stale_reply()),
        ("undo-failed", undo_failed_reply()),
        (
            "write-chips",
            checklist_reply(&habit_courses(), &qab_checklist(false)),
        ),
        (
            "write-confirmed",
            write_confirmed_reply(&habit_courses(), &qab_checklist(true)),
        ),
        (
            "write-cleared",
            write_cleared_reply(&habit_courses(), &qab_checklist(false)),
        ),
        (
            "write-day-closed",
            write_day_closed_reply(&habit_courses(), &qab_checklist(false)),
        ),
        ("write-not-a-writing-course", not_a_writing_course_reply()),
        ("write-no-writing-course", no_writing_course_reply()),
        ("write-failed", write_failed_reply()),
        (
            "score",
            score_reply(Some(&scored(72, ("SOLID", "\u{2705}"), 40, Some(87.5)))),
        ),
        (
            "score-no-retention",
            score_reply(Some(&scored(12, ("COLD", "\u{1f976}"), 0, None))),
        ),
        ("score-none", score_reply(None)),
        ("score-failed", score_failed_reply()),
        ("badges-failed", badges_failed_reply()),
        ("records-failed", records_failed_reply()),
        ("progress", progress_reply(&stored_progress(), &app())),
        ("progress-none", progress_reply(&[], &app())),
        ("progress-failed", progress_failed_reply()),
        ("help", help_reply()),
        ("export-failed", export_failed_reply()),
        ("erase-done-log-held", erase_done_reply(true)),
        ("erase-failed", erase_failed_reply()),
        (
            "sync-unchanged",
            sync_reply(&synced(SyncOutcome::Synced, Scores::Unchanged)),
        ),
        (
            "sync-failed",
            sync_reply(&synced(
                SyncOutcome::Failed {
                    reason: "server_error".to_owned(),
                },
                Scores::Recomputed,
            )),
        ),
        (
            "sync-reused",
            sync_reply(&synced(SyncOutcome::Reused, Scores::Recomputed)),
        ),
        (
            "sync-not-run",
            sync_reply(&synced(
                SyncOutcome::NotRun {
                    reason: "refused_today".to_owned(),
                },
                Scores::Unchanged,
            )),
        ),
        (
            "sync-scores-refused",
            sync_reply(&synced(
                SyncOutcome::Synced,
                Scores::Refused {
                    reason: "recompute_failed".to_owned(),
                },
            )),
        ),
        (
            "sync-still-running",
            sync_reply(&synced(SyncOutcome::StillRunning, Scores::Unchanged)),
        ),
        (
            "sync-refused",
            sync_reply(&Err(SyncRefusal {
                reason: "nothing_scripted",
            })),
        ),
    ]
}

#[tokio::test]
async fn a_refusal_after_a_run_is_answered_beside_the_syncs_own_line() {
    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::answering([Ok(SyncAnswer {
        sync: SyncOutcome::Synced,
        scores: Scores::Refused {
            reason: "recompute_failed".to_owned(),
        },
    })]));
    commands.handle(incoming(owner_says(1, "/sync"))).await;
    let sent = last_send(&bench);
    assert_eq!(
        sent,
        golden_send("sync-scores-refused"),
        "the refusal's golden"
    );
    let text = sent["text"].as_str().expect("a text");
    assert_eq!(
        text.lines().next(),
        Some("Synced with your Anki sync server.")
    );
    assert!(
        text.contains("recompute_failed"),
        "the refusal's code is named: {text}"
    );
}

#[tokio::test]
async fn every_golden_message_is_what_the_bot_sends() {
    let bench = Bench::start().await;
    let mut commands = bench.commands(ScriptedSync::answering([Ok(SyncAnswer {
        sync: SyncOutcome::Synced,
        scores: Scores::Recomputed,
    })]));
    let mut covered = BTreeSet::new();

    // The messages a command path sends.
    for (update, name) in [
        (owner_says(1, "/start"), "start"),
        (owner_says(2, "/privacy"), "privacy"),
        (owner_says(3, "hello"), "help"),
        (owner_says(4, "/delete"), "erase-prompt"),
    ] {
        commands.handle(incoming(update)).await;
        assert_eq!(last_send(&bench), golden_send(name), "{name}");
        covered.insert(name.to_owned());
    }
    let prompt = bench
        .fake
        .calls_of("sendMessage")
        .pop()
        .and_then(|call| call.message_id)
        .expect("the question's message id");
    for (update, name) in [
        (owner_taps(5, CONFIRM_ERASE, prompt), "erase-done"),
        (owner_taps(6, CONFIRM_ERASE, prompt), "erase-expired"),
        (owner_says(7, "/sync"), "sync-synced"),
    ] {
        commands.handle(incoming(update)).await;
        assert_eq!(last_send(&bench), golden_send(name), "{name}");
        covered.insert(name.to_owned());
    }
    // The export's caption.
    assert_eq!(
        json!({"caption": export_caption(), "parse_mode": "HTML"}),
        golden_send("export")
    );
    covered.insert("export".to_owned());

    // The messages only a failure or an outcome no fake here produces sends, through the transport.
    for (name, reply) in rendered() {
        let sent = bench
            .transport
            .send_html(OWNER, &reply.text, reply.keyboard)
            .await;
        assert!(matches!(sent, Sent::Delivered { .. }), "{name}: {sent:?}");
        assert_eq!(last_send(&bench), golden_send(name), "{name}");
        covered.insert(name.to_owned());
    }
    // The long sample's chunks are the chunking test's.
    let committed: BTreeSet<String> = examined(
        "golden message(s)",
        std::fs::read_dir(messages_directory())
            .expect("the messages directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter_map(|name| name.strip_suffix(".msg.json").map(str::to_owned))
            .filter(|name| !name.starts_with("long-sample."))
            .collect(),
    )
    .into_iter()
    .collect();
    assert_eq!(
        committed, covered,
        "every golden is a message the bot sends, and each has one"
    );
}

#[test]
fn the_mini_app_url_is_https() {
    for url in [
        APP_URL,
        "https://deckstreak.example",
        "https://deckstreak.example:8443/app?x=1",
    ] {
        assert_eq!(
            MiniAppUrl::new(url).map(|app| app.as_str().to_owned()),
            Some(url.to_owned()),
            "{url}"
        );
    }
    for url in [
        "http://deckstreak.example/app",
        "https://",
        "https:///app",
        "https://:8443/app",
        "https://@deckstreak.example/app",
        "https://deckstreak .example/app",
        "deckstreak.example/app",
    ] {
        assert_eq!(MiniAppUrl::new(url), None, "{url}");
    }
    let set = Environment::from_vars([(MINI_APP_URL, APP_URL)]);
    assert_eq!(
        MiniAppUrl::from_env(&set).map(|app| app.as_str().to_owned()),
        Ok(APP_URL.to_owned())
    );
    let unset = Environment::from_vars(Vec::<(String, String)>::new());
    assert!(MiniAppUrl::from_env(&unset).is_err(), "the bot requires it");
    let plain = Environment::from_vars([(MINI_APP_URL, "http://deckstreak.example/app")]);
    // A web_app button opens only https, and the refusal names the whole shape, as written.
    assert_eq!(
        MiniAppUrl::from_env(&plain).map(|app| app.as_str().to_owned()),
        Err(SettingsError::Malformed {
            setting: MINI_APP_URL,
            expected: "an https: URL that names a host",
        })
    );
}

/// A service that answers nothing: only its presence on the handlers is looked at.
struct NoInstruments;

impl deck_streak_coordination::instruments::InstrumentService for NoInstruments {
    fn list(
        &self,
    ) -> deck_streak_coordination::instruments::BoxFuture<
        '_,
        Result<
            Vec<deck_streak_coordination::instruments::InstrumentListing>,
            deck_streak_kernel::KernelError,
        >,
    > {
        Box::pin(async { Ok(Vec::new()) })
    }
    fn report<'a>(
        &'a self,
        _id: &'a str,
    ) -> deck_streak_coordination::instruments::BoxFuture<
        'a,
        Result<
            Option<deck_streak_coordination::instruments::StoredReport>,
            deck_streak_kernel::KernelError,
        >,
    > {
        Box::pin(async { Ok(None) })
    }
    fn run<'a>(
        &'a self,
        _id: &'a str,
    ) -> deck_streak_coordination::instruments::BoxFuture<
        'a,
        Result<
            deck_streak_coordination::instruments::StoredReport,
            deck_streak_coordination::instruments::OnDemandRefusal,
        >,
    > {
        Box::pin(async { Err(deck_streak_coordination::instruments::OnDemandRefusal::Unknown) })
    }
}

#[tokio::test]
async fn the_handlers_hold_the_instruments_only_once_they_are_handed_them() {
    let bench = Bench::start().await;
    let without = bench.commands(ScriptedSync::default());
    assert!(without.instruments().is_none());
    let with = bench
        .commands(ScriptedSync::default())
        .with_instruments(std::sync::Arc::new(NoInstruments));
    assert!(with.instruments().is_some());
}
