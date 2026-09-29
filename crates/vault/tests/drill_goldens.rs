//! The law drill functions equal the predecessor's (SPEC-110 A1, A2, A3, A8, A11, A21, A24): the
//! views, the deferral reason, the queue and the rollup, the graded parse, the grant key and the
//! constants, each against a golden the predecessor's own functions wrote over synthetic notes.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use deck_streak_kernel::{Environment, StudyDay};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::drill_notes::DrillNotes;
use deck_streak_vault::drills::{
    self, DEFER_REASON_MAX_LEN, DrillMeta, POSTBACK_XP, XP_MAX, XP_MIN, parse_graded, queue,
    read_meta, read_view, rollup, sanitise_defer_reason,
};
use deck_streak_vault::{Rails, RealFs, VaultSettings};
use serde_json::{Value, json};
use tempfile::TempDir;

/// `text` with `{badday}` written as a date no calendar holds and every `{day:N}` as the ISO date
/// of study day N, as the golden's adapter wrote the predecessor's days.
fn expand(text: &str) -> String {
    let text = text.replace("{badday}", "2026-02-30");
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("{day:") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "{day:".len()..];
        let end = after.find('}').expect("a closed day token");
        let day: i64 = after[..end].parse().expect("an epoch day number");
        out.push_str(&StudyDay::from_epoch_day(day).to_string());
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `value` with every string expanded, keys included.
fn expand_value(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(expand(s)),
        Value::Array(items) => Value::Array(items.iter().map(expand_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (expand(k), expand_value(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn day(case: &golden::Case) -> StudyDay {
    StudyDay::from_epoch_day(case.input["today"].as_i64().expect("an epoch day"))
}

/// A drill's meta as the golden writes it.
fn meta_json(meta: &DrillMeta) -> Value {
    json!({
        "age_days": meta.age_days,
        "answered": meta.answered,
        "created": meta.created.map(|d| d.to_string()),
        "deferred": meta.deferred,
        "drill_id": meta.drill_id,
        "subject": meta.subject,
        "title": meta.title,
        "type": meta.kind,
    })
}

fn view_json(stem: &str, text: &str, today: StudyDay) -> Value {
    let view = read_view(stem, text, today);
    let mut value = meta_json(&view.meta);
    value["prompt"] = json!(view.prompt);
    value["defer_reason"] = json!(view.defer_reason);
    value
}

/// The notes of a case as `(stem, text)`; a null note is a folder named as a note.
fn notes_of(case: &golden::Case) -> Vec<(String, Option<String>)> {
    expand_value(&case.input["notes"])
        .as_object()
        .expect("a notes map")
        .iter()
        .map(|(stem, text)| (stem.clone(), text.as_str().map(str::to_owned)))
        .collect()
}

fn metas_of(case: &golden::Case) -> Vec<DrillMeta> {
    notes_of(case)
        .into_iter()
        .filter_map(|(stem, text)| text.map(|t| read_meta(&stem, &t, day(case))))
        .collect()
}

/// A vault whose `11-Drills/Active` holds `notes`, opened as the drill reader.
fn open_vault(notes: &[(String, Option<String>)]) -> (TempDir, DrillNotes<RealFs>) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let active = dir.path().join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    fs::create_dir_all(dir.path().join("11-Drills").join("Graded")).expect("the graded folder");
    for (stem, text) in notes {
        let path = active.join(format!("{stem}.md"));
        match text {
            Some(text) => fs::write(&path, text).expect("a note"),
            None => fs::create_dir(&path).expect("a folder named as a note"),
        }
    }
    let notes = open_at(dir.path());
    (dir, notes)
}

fn open_at(root: &Path) -> DrillNotes<RealFs> {
    let env = Environment::from_vars([
        (VAULT_ROOT, root.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    let settings = VaultSettings::from_env(&env).expect("settings");
    DrillNotes::open(&settings, RealFs, Rails::vendored().expect("the rails"))
        .expect("the vault opens")
}

#[test]
fn the_drill_views_match_the_predecessors_golden() {
    let examined = golden::each_case("drill_meta", |case| {
        let today = day(case);
        let notes = notes_of(case);
        let (_dir, reader) = open_vault(&notes);
        let expected = expand_value(&case.output);
        let active: Vec<Value> = reader
            .list_active(today)
            .expect("the active folder lists")
            .iter()
            .map(meta_json)
            .collect();
        assert_eq!(
            Value::Array(active.clone()),
            expected["active"],
            "active of {}",
            case.input
        );
        let unanswered: Vec<Value> = active
            .iter()
            .filter(|m| m["answered"] == json!(false))
            .cloned()
            .collect();
        assert_eq!(
            Value::Array(unanswered),
            expected["unanswered"],
            "unanswered"
        );
        for (id, want) in expected["views"].as_object().expect("views") {
            let got = reader.view(id, today).map_or(Value::Null, |v| {
                let mut value = meta_json(&v.meta);
                value["prompt"] = json!(v.prompt);
                value["defer_reason"] = json!(v.defer_reason);
                value
            });
            assert_eq!(&got, want, "the view of {id:?} in {}", case.input);
        }
        for (stem, text) in &notes {
            if let Some(text) = text {
                assert_eq!(
                    view_json(stem, text, today),
                    expected["views"][stem],
                    "read_view of {stem:?}"
                );
            }
        }
    });
    println!("examined {} drill_meta cases", examined.count);
    assert!(examined.count > 0, "examined 0 drill_meta cases");
}

#[test]
fn the_defer_reason_matches_the_predecessors_golden() {
    let examined = golden::each_case("drill_defer_reason", |case| {
        let raw = case.input["raw"].as_str().expect("a raw reason");
        assert_eq!(
            sanitise_defer_reason(raw),
            case.output.as_str().expect("a sanitised reason"),
            "the reason {raw:?}"
        );
    });
    println!("examined {} drill_defer_reason cases", examined.count);
    assert!(examined.count > 0, "examined 0 drill_defer_reason cases");
}

#[test]
fn the_queue_and_rollup_match_the_predecessors_golden() {
    let queues = golden::each_case("drill_queue", |case| {
        let queued = queue(&metas_of(case));
        let expected = expand_value(&case.output);
        let unanswered: Vec<Value> = queued.unanswered.iter().map(meta_json).collect();
        assert_eq!(
            Value::Array(unanswered),
            expected["unanswered"],
            "queue of {}",
            case.input
        );
        assert_eq!(json!(queued.awaiting_grading), expected["awaiting_grading"]);
        assert_eq!(json!(queued.deferred), expected["deferred"]);
    });
    let rollups = golden::each_case("drill_rollup", |case| {
        let subjects: BTreeSet<String> = case.input["subjects"]
            .as_array()
            .expect("subjects")
            .iter()
            .map(|s| s.as_str().expect("a subject").to_owned())
            .collect();
        let rolled = rollup(&metas_of(case), &subjects);
        let expected = expand_value(&case.output);
        let got = json!({
            "active": rolled.active,
            "awaiting_grading": rolled.awaiting_grading,
            "deferred": rolled.deferred,
            "oldest_age_days": rolled.oldest_age_days,
            "oldest_created": rolled.oldest_created.map(|d| d.to_string()),
            "unmatched_active": rolled.unmatched_active,
        });
        assert_eq!(got, expected, "rollup of {}", case.input);
    });
    println!(
        "examined {} queue and {} rollup cases",
        queues.count, rollups.count
    );
    assert!(
        queues.count > 0 && rollups.count > 0,
        "examined 0 queue or rollup cases"
    );
}

#[test]
fn the_graded_parse_matches_the_predecessors_golden() {
    let examined = golden::each_case("drill_graded_parse", |case| {
        let notes = notes_of(case);
        let expected = case.output.as_object().expect("an output per note");
        assert_eq!(expected.len(), notes.len(), "one output per note");
        for (stem, text) in &notes {
            let got = text
                .as_deref()
                .and_then(|t| parse_graded(stem, t))
                .map_or(Value::Null, |g| json!([g.drill_id, g.xp]));
            assert_eq!(&got, &expected[stem], "the graded parse of {stem:?}");
        }
    });
    println!("examined {} drill_graded_parse cases", examined.count);
    assert!(examined.count > 0, "examined 0 drill_graded_parse cases");
}

#[test]
fn an_unkeyable_drill_id_is_keyed_by_its_hash() {
    let long = "x".repeat(129);
    let fits = "x".repeat(128);
    let cases = [
        ("law-1", "drill:law-1"),
        ("a:b._c-9", "drill:a:b._c-9"),
        ("h.x", "drill:h.ad2191923a9419876dd2ab3f9f65e624"),
        ("a/b", "drill:h.c14cddc033f64b9dea80ea675cf280a0"),
        ("Drill 1", "drill:h.bd016512d1b37f2388f36cfb76c6182d"),
        ("", "drill:h.e3b0c44298fc1c149afbf4c8996fb924"),
        (long.as_str(), "drill:h.0ec9eb33e74510bcdd1f2ea55206e82f"),
    ];
    for (id, key) in cases {
        assert_eq!(drills::drill_key(id), key, "the key of {id:?}");
    }
    assert_eq!(
        drills::drill_key(&fits),
        format!("drill:{fits}"),
        "128 bytes is kept"
    );
    assert_eq!(
        drills::drill_key("-a"),
        "drill:h.".to_owned() + &drills::drill_key("-a")[8..]
    );
    assert_ne!(
        drills::drill_key("h.x"),
        "drill:h.x",
        "a leading h. is never kept"
    );
}

#[test]
fn the_drill_constants_equal_the_predecessors() {
    let examined = golden::each_case("drills.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let got = match name {
            "vault_bridge.DRILL_POSTBACK_XP" => json!(POSTBACK_XP),
            "vault_bridge.DRILL_XP_MIN" => json!(XP_MIN),
            "vault_bridge.DRILL_XP_MAX" => json!(XP_MAX),
            "vault_bridge.DEFER_REASON_MAX_LEN" => json!(DEFER_REASON_MAX_LEN),
            "vault_bridge._READY_UNTICKED" => json!(drills::READY_UNTICKED),
            "vault_bridge._READY_TICKED" => json!(drills::READY_TICKED),
            // The bot's and the curriculum's constants are pinned where they live.
            _ => return,
        };
        assert_eq!(got, case.output, "the constant {name}");
    });
    println!("examined {} drills.constants cases", examined.count);
    assert!(examined.count > 0, "examined 0 drills.constants cases");
}

#[test]
fn the_single_view_carries_the_answer_sections_and_the_self_check() {
    let text = "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-01-05\nstatus: active\n---\n# A synthetic drill\n\nThe prompt.\n\n## Free Recall\n\n## Rule\n\n## Self-Check\n\n- [ ] the rule stated\n- [ ] the facts applied\n- [ ] **Ready for grading**\n";
    let view = read_view("irac-1", text, StudyDay::from_epoch_day(20_000));
    assert_eq!(
        view.sections,
        vec!["Free Recall".to_owned(), "Rule".to_owned()]
    );
    assert_eq!(
        view.self_check,
        vec!["the rule stated".to_owned(), "the facts applied".to_owned()]
    );
    assert_eq!(view.prompt, "The prompt.");
}
