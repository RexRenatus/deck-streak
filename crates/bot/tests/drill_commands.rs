//! The law drills through the bot (SPEC-110 A16 to A18; R13, R14): the callback token equals the
//! predecessor's, `/drills` lists at most twelve and takes the next message as the answer, and
//! `/drill` filters by the four codes.
//!
//! Each command goes through the bot's own handlers to a fake Bot API, over a temporary vault of
//! synthetic drill notes and a temporary database.

// An integration test is test code: its helpers panic on a failed check, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;
#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deck_streak_bot::commands::Commands;
use deck_streak_bot::drill_commands::{
    ANSWER_PREFIX, KEYBOARD_LIMIT, MAX_CALLBACK_DATA, TOKEN_HASH_LEN, VIEW_PREFIX, encode_token,
    resolve_token,
};
use deck_streak_coordination::drills::{
    ARCHIVE_FOLDER, DrillNotes, READINGS_FOLDER, Rails, RealFs, VAULT_ROOT, VaultSettings,
};
use deck_streak_kernel::Environment;
use fake_bot_api::{Bench, ScriptedSync, incoming, owner_says, owner_taps, payload};
use serde_json::Value;

/// One synthetic drill note of `kind`, titled `title`.
fn note(kind: &str, title: &str) -> String {
    format!(
        "---\ntype: drill-{kind}\nsubject: \"Torts\"\ncreated: 2026-02-20\nstatus: active\n---\n# {title}\n\nThe prompt.\n\n## Free Recall\n\n## Self-Check\n\n- [ ] the rule stated\n- [ ] **Ready for grading**\n"
    )
}

/// A bench, its temporary vault holding `notes` (id and kind), and the handlers over both.
async fn start(notes: &[(String, &str)]) -> (Bench, Commands<ScriptedSync>, PathBuf) {
    let bench = Bench::start().await;
    let vault = bench.directory.path().join("vault");
    let active = vault.join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    for (id, kind) in notes {
        fs::write(active.join(format!("{id}.md")), note(kind, id)).expect("a note");
    }
    let env = Environment::from_vars([
        (VAULT_ROOT, vault.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    let settings = VaultSettings::from_env(&env).expect("settings");
    let opened = DrillNotes::open(&settings, RealFs, Rails::vendored().expect("the rails"))
        .expect("the vault opens");
    let commands = bench
        .commands(ScriptedSync::default())
        .with_drills(Arc::new(opened));
    (bench, commands, active)
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

/// The notes of a folder whose text holds `needle`, out of every note the folder holds.
fn notes_holding(folder: &Path, needle: &str) -> Vec<String> {
    let notes: Vec<String> = fs::read_dir(folder)
        .expect("the folder")
        .map(|entry| fs::read_to_string(entry.expect("an entry").path()).expect("a note"))
        .collect();
    examined("drill note(s)", notes)
        .into_iter()
        .filter(|body| body.contains(needle))
        .collect()
}

/// The payload of the last `sendMessage`.
fn last_send(bench: &Bench) -> Value {
    let mut sends = bench.fake.calls_of("sendMessage");
    assert!(!sends.is_empty(), "the command sent no reply");
    payload(&sends.pop().expect("a send"))
}

/// Every button's callback data on `sent`'s keyboard, in order.
fn buttons(sent: &Value) -> Vec<String> {
    sent["reply_markup"]["inline_keyboard"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .flat_map(|row| row.as_array().cloned().unwrap_or_default())
                .filter_map(|button| button["callback_data"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The reply's text.
fn text(sent: &Value) -> String {
    sent["text"].as_str().expect("a text").to_owned()
}

#[test]
fn the_drill_token_matches_the_predecessors_golden() {
    let examined = golden::each_case("drill_callback_token", |case| {
        let prefix = case.input["prefix"].as_str().expect("a prefix");
        let drill = case.input["drill"].as_str().expect("a drill");
        let offered: Vec<String> = case.input["offered"]
            .as_array()
            .expect("offered")
            .iter()
            .map(|id| id.as_str().expect("an id").to_owned())
            .collect();
        let token = encode_token(prefix, drill);
        assert_eq!(
            Value::from(token.clone()),
            case.output["token"],
            "the token of {drill}"
        );
        assert!(
            (1..=MAX_CALLBACK_DATA).contains(&token.len()),
            "a token is 1 to 64 bytes: {token}"
        );
        let resolved = resolve_token(prefix, &token, &offered);
        assert_eq!(
            resolved.map_or(Value::Null, Value::from),
            case.output["resolved"],
            "the drill of {token}"
        );
    });
    assert_eq!(examined.count, 25);
    assert_eq!(VIEW_PREFIX, "dv:");
    assert_eq!(ANSWER_PREFIX, "da:");

    // The boundary: 64 bytes are kept whole, 65 are hashed to twenty hex characters.
    let kept = "a".repeat(MAX_CALLBACK_DATA - VIEW_PREFIX.len());
    assert_eq!(
        encode_token(VIEW_PREFIX, &kept),
        format!("{VIEW_PREFIX}{kept}")
    );
    let hashed = encode_token(VIEW_PREFIX, &format!("{kept}a"));
    let (head, digest) = hashed.split_at(VIEW_PREFIX.len() + 1);
    assert_eq!(head, format!("{VIEW_PREFIX}#"));
    assert_eq!(digest.len(), TOKEN_HASH_LEN);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));

    // A token that names no offered drill, or two of them, resolves to none.
    let offered = vec!["one".to_owned(), "two".to_owned()];
    assert_eq!(resolve_token(VIEW_PREFIX, "dv:three", &offered), None);
    assert_eq!(resolve_token(VIEW_PREFIX, "da:one", &offered), None);
    let twice = vec!["same".to_owned(), "same".to_owned()];
    assert_eq!(resolve_token(VIEW_PREFIX, "dv:same", &twice), None);
    assert_ne!(VIEW_PREFIX, ANSWER_PREFIX);
}

#[tokio::test]
async fn drills_lists_twelve_and_takes_the_next_message() {
    let notes: Vec<(String, &str)> = (1..=14).map(|n| (format!("irac-{n:02}"), "irac")).collect();
    let (bench, mut commands, active) = start(&notes).await;

    commands.handle(incoming(owner_says(1, "/drills"))).await;
    let list = last_send(&bench);
    let taps = buttons(&list);
    assert_eq!(taps.len(), KEYBOARD_LIMIT, "twelve buttons of fourteen");
    assert!(taps.iter().all(|data| data.starts_with(VIEW_PREFIX)));
    assert!(text(&list).contains("and 2 more"), "{}", text(&list));

    // A tap opens the drill's view, with an Answer button.
    commands.handle(incoming(owner_taps(2, &taps[0], 10))).await;
    let view = last_send(&bench);
    let asks = buttons(&view);
    assert_eq!(asks.len(), 1, "one Answer button");
    assert!(asks[0].starts_with(ANSWER_PREFIX), "{asks:?}");
    assert!(text(&view).contains("The prompt."), "{}", text(&view));

    // The Answer button asks; the next plain message is the answer.
    commands.handle(incoming(owner_taps(3, &asks[0], 11))).await;
    let sends_before = bench.fake.calls_of("sendMessage").len();
    commands
        .handle(incoming(owner_says(4, "The duty is owed.")))
        .await;
    assert!(bench.fake.calls_of("sendMessage").len() > sends_before);
    let answered: Vec<String> = notes_holding(&active, "The duty is owed.");
    assert_eq!(answered.len(), 1, "exactly one note took the answer");

    // A second plain message is nobody's answer.
    commands
        .handle(incoming(owner_says(5, "Another line.")))
        .await;
    let count = notes_holding(&active, "Another line.").len();
    assert_eq!(count, 0, "the pending answer was spent");

    // A command cancels a pending answer.
    commands.handle(incoming(owner_taps(6, &taps[1], 12))).await;
    let ask = buttons(&last_send(&bench));
    commands.handle(incoming(owner_taps(7, &ask[0], 13))).await;
    commands.handle(incoming(owner_says(8, "/score"))).await;
    commands.handle(incoming(owner_says(9, "Too late."))).await;
    let late = notes_holding(&active, "Too late.").len();
    assert_eq!(late, 0, "a command cancelled the pending answer");
}

#[tokio::test]
async fn drill_filters_by_the_four_codes() {
    let notes = vec![
        ("case-1".to_owned(), "case-brief"),
        ("irac-1".to_owned(), "irac"),
        ("irac-2".to_owned(), "irac"),
        ("outline-1".to_owned(), "outline"),
        ("rule-1".to_owned(), "rule-statement"),
    ];
    let (bench, mut commands, _active) = start(&notes).await;

    commands.handle(incoming(owner_says(1, "/drill"))).await;
    let types = text(&last_send(&bench));
    for name in ["case-brief", "irac", "outline", "rule-statement"] {
        assert!(types.contains(name), "{name} in {types}");
    }

    commands.handle(incoming(owner_says(2, "/drill i"))).await;
    let irac = last_send(&bench);
    let taps = buttons(&irac);
    assert_eq!(taps.len(), 2, "only the irac drills: {taps:?}");
    let offered: Vec<String> = ["irac-1", "irac-2"].map(str::to_owned).to_vec();
    for data in &taps {
        let id = resolve_token(VIEW_PREFIX, data, &offered);
        assert!(id.is_some(), "{data} names an irac drill");
    }

    commands.handle(incoming(owner_says(3, "/drill x"))).await;
    let refusal = text(&last_send(&bench));
    for code in ["c", "i", "o", "u"] {
        assert!(refusal.contains(code), "{code} in {refusal}");
    }
    assert!(
        refusal.contains("c, i, o, u"),
        "the refusal lists the codes: {refusal}"
    );
}
