//! Every engine golden is read by a Rust test (SPEC-498 R14). The second oracle's one sandboxed
//! run of the engine the root manifest pins writes one golden per registered function; each test
//! here reads its golden through `golden.rs`, holds it to that pin and to the driver that wrote
//! it, and holds every case's answer to the answer keys the harness gives for that function, so a
//! golden whose answers lost or gained a key fails here before any port reads it. The port of
//! each function compares its own outputs with these goldens through `golden::assert_parity` in
//! the delivery that ports it.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The driver every engine golden names.
const DRIVER: &str = "tools/parity-oracle/anki/generate.py";
/// The root manifest's patch section for the engine.
const PATCH_SECTION: &str = "[patch.\"https://github.com/ankitects/anki.git\"]";
/// The longest refusal label the driver admits.
const LABEL_MAX: usize = 48;

/// A path from the repository root.
fn root(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The engine commit the root manifest's patch pins: the `rev` of its `anki` line.
fn pinned_engine() -> String {
    let manifest = fs::read_to_string(root("Cargo.toml"))
        .unwrap_or_else(|error| panic!("the root manifest reads: {error}"));
    let (_, section) = manifest
        .split_once(PATCH_SECTION)
        .unwrap_or_else(|| panic!("the root manifest patches the engine"));
    let line = section
        .lines()
        .find(|line| line.starts_with("anki = {"))
        .unwrap_or_else(|| panic!("the engine patch names the anki crate"));
    let (_, rest) = line
        .split_once("rev = \"")
        .unwrap_or_else(|| panic!("the engine patch is pinned by rev: {line}"));
    let rev: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
    assert_eq!(rev.len(), 40, "the engine pin is a full commit: {line}");
    rev
}

/// A refusal the driver admits: a short lower-case label, never the engine's words.
fn is_label(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && text.len() <= LABEL_MAX
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Why one case's output is neither a refusal label nor an answer of exactly `keys`.
fn answer_fault(output: &Value, keys: &[&str]) -> Option<String> {
    let Some(object) = output.as_object() else {
        return Some(format!("answers {output}, not an object"));
    };
    let found: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if found == BTreeSet::from(["refused"]) {
        return match object["refused"].as_str() {
            Some(label) if is_label(label) => None,
            _ => Some(format!("refuses with {}, not a label", object["refused"])),
        };
    }
    let wanted: BTreeSet<&str> = keys.iter().copied().collect();
    (found != wanted).then(|| format!("answers the keys {found:?}, not {wanted:?}"))
}

/// Reads the committed engine golden `name`, holds it to the pin, the driver and `keys`, and
/// returns how many of its cases are answers rather than refusals.
fn read_engine_golden(name: &str, keys: &[&str]) -> usize {
    let path = golden::committed(name);
    let golden = golden::read(&path).unwrap_or_else(|refusal| panic!("{refusal}"));
    assert_eq!(golden.kind, "engine", "{name}: kind");
    assert_eq!(golden.function, name, "{name}: function");
    assert_eq!(golden.generator, DRIVER, "{name}: generator");
    assert_eq!(golden.inputs, "synthetic", "{name}: inputs");
    assert_eq!(
        golden.source_commit,
        pinned_engine(),
        "{name}: source_commit"
    );
    let mut faults = Vec::new();
    let mut answered = 0;
    for (index, case) in golden.cases.iter().enumerate() {
        if case.class.as_deref() == Some("") {
            faults.push(format!("case {index} has an empty class"));
        }
        match answer_fault(&case.output, keys) {
            Some(fault) => faults.push(format!("case {index} {fault}")),
            None if case.output.get("refused").is_none() => answered += 1,
            None => {}
        }
    }
    eprintln!("examined {} case(s) of {name}", golden.cases.len());
    assert!(
        faults.is_empty(),
        "{name}: {} case(s) off the harness's answer keys:\n{}",
        faults.len(),
        faults.join("\n")
    );
    assert!(
        answered > 0,
        "{name}: every case is a refusal, so it records no answer"
    );
    answered
}

/// One test per engine golden, named for it, with the answer keys the harness gives.
macro_rules! engine_golden {
    ($($test:ident: $name:literal => [$($key:literal),+ $(,)?];)+) => {
        /// Every engine golden this target reads, by name.
        const READ: &[&str] = &[$($name),+];
        $(
            #[test]
            fn $test() {
                read_engine_golden($name, &[$($key),+]);
            }
        )+
    };
}

engine_golden! {
    collection_view: "anki.read.collection_view" => ["cards", "decks", "notes", "notetypes", "revlog"];
    export_package: "anki.export.package" => ["media", "members", "rows"];
    import_package: "anki.import.package" => ["counts", "notes", "revlog"];
    timing_day: "anki.timing.day" => ["day", "next_rollover"];
    card_due_view: "anki.card.due_view" => ["due"];
    note_checksum: "anki.note.checksum" => ["csum", "sfld"];
    media_name: "anki.media.name" => ["names"];
    collation_order: "anki.collation.order" => ["order"];
    blob_encode: "anki.blob.encode" => ["hex"];
    history_local_answer: "anki.history.local_answer" => ["revlog"];
    guid_accepts: "anki.guid.accepts" => ["kept"];
    schema_snapshot: "anki.schema.snapshot" => ["col_ver", "schema", "user_version"];
    render_card: "anki.render.card" => ["answer", "question"];
    render_card_ordinals: "anki.render.card_ordinals" => ["ordinals"];
    render_cloze_only_text: "anki.render.cloze_only_text" => ["text"];
    render_extract_av_tags: "anki.render.extract_av_tags" => ["tags", "text"];
    render_strip_av_tags: "anki.render.strip_av_tags" => ["text"];
    render_html_to_text_line: "anki.render.html_to_text_line" => ["text"];
    render_media_refs: "anki.render.media_refs" => ["names"];
    render_latex_images: "anki.render.latex_images" => ["images", "text"];
    render_compare_answer: "anki.render.compare_answer" => ["html"];
    render_note_type_check: "anki.render.note_type_check" => ["result"];
    render_note_check: "anki.render.note_check" => ["result"];
    render_occlusion_render: "anki.render.occlusion_render" => ["answer", "question"];
    media_field_refs: "anki.media.field_refs" => ["names"];
    media_check: "anki.media.check" => ["missing", "unused"];
    search_operator: "anki.search.operator" => ["cards"];
    search_browser: "anki.search.browser" => ["cards"];
    history_revlog_events: "anki.history.revlog_events" => ["rows"];
    sched_timing_today: "anki.sched.timing_today" => ["day", "next_rollover"];
    deck_name_equal: "anki.deck.name_equal" => ["decks"];
    read_reader_fixture: "anki.read.reader_fixture" => ["tables"];
    render_generated_ordinals: "anki.render.generated_ordinals" => ["ordinals"];
}

#[test]
fn every_engine_golden_is_read_here() {
    let mut on_disk: Vec<String> = fs::read_dir(root("tools/parity-oracle/goldens"))
        .expect("the goldens directory reads")
        .map(|entry| entry.expect("an entry reads").file_name())
        .filter_map(|name| name.to_str().map(str::to_owned))
        .filter_map(|name| {
            name.strip_prefix("anki.")
                .and_then(|rest| rest.strip_suffix(".json"))
                .map(|stem| format!("anki.{stem}"))
        })
        .collect();
    on_disk.sort();
    let mut read: Vec<String> = READ.iter().map(|&name| name.to_owned()).collect();
    read.sort();
    println!("examined {} engine golden(s) on disk", on_disk.len());
    assert_eq!(read.len(), 33, "the registry's 33 functions");
    assert_eq!(
        on_disk, read,
        "each engine golden on disk has its test here"
    );
}

#[test]
fn an_answer_off_its_keys_or_a_worded_refusal_is_a_fault() {
    let keys = ["day", "next_rollover"];
    let cases = [
        (serde_json::json!({"day": 2, "next_rollover": 7}), None),
        (serde_json::json!({"refused": "invalid_input"}), None),
        (serde_json::json!({"day": 2}), Some("answers the keys")),
        (
            serde_json::json!({"day": 2, "next_rollover": 7, "id": 1}),
            Some("answers the keys"),
        ),
        (
            serde_json::json!({"refused": "The deck name is invalid."}),
            Some("not a label"),
        ),
        (serde_json::json!([2, 7]), Some("not an object")),
    ];
    println!("examined {} planted answer(s)", cases.len());
    for (output, want) in cases {
        let fault = answer_fault(&output, &keys);
        match want {
            None => assert_eq!(fault, None, "{output}"),
            Some(text) => assert!(
                fault.as_deref().is_some_and(|f| f.contains(text)),
                "{output}: {fault:?}"
            ),
        }
    }
    assert_eq!(pinned_engine().len(), 40);
}

#[test]
fn the_schema_snapshot_is_its_golden_under_one_pin_line() {
    let golden = golden::read(&golden::committed("anki.schema.snapshot"))
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    let snapshot = fs::read_to_string(root("tools/parity-oracle/anki/schema-snapshot.sql"))
        .expect("the schema snapshot is committed");
    assert_eq!(golden.cases.len(), 1, "one fresh collection");
    let output = &golden.cases[0].output;
    let version = output["user_version"]
        .as_i64()
        .expect("user_version is an integer");
    let pins: Vec<&str> = snapshot
        .lines()
        .filter(|line| line.starts_with("PRAGMA user_version"))
        .collect();
    assert_eq!(pins, [format!("PRAGMA user_version = {version};")]);
    let statements = output["schema"].as_array().expect("schema is a list");
    for statement in statements {
        let text = statement.as_str().expect("a statement is text");
        assert!(
            snapshot.contains(&format!("{};\n", text.trim_end().trim_end_matches(';'))),
            "the snapshot holds {text}"
        );
    }
    println!(
        "examined {} statement(s) of the schema snapshot",
        statements.len()
    );
    assert!(!statements.is_empty(), "the fresh collection has a schema");
}
