//! The owner's courses are private configuration read once at start (SPEC-071 A13, A14; R1, R4;
//! ADR-087): the neutral example loads, every malformed or contradictory file refuses start naming
//! the setting and never a value, and a changed file bumps the settings generation exactly once.
//!
//! Every course, deck name and alias here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::path::{Path, PathBuf};

use deck_streak_kernel::courses::{COURSES_FILE, COURSES_SCHEMA, content_digest};
use deck_streak_kernel::{CourseCode, Courses, CoursesError, Db, Environment};
use serde_json::{Value, json};
use tempfile::TempDir;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The neutral example the repository ships.
fn example_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/config/courses.example.json")
}

fn example() -> Value {
    let text = std::fs::read_to_string(example_path()).expect("the example is readable");
    serde_json::from_str(&text).expect("the example is JSON")
}

fn environment(path: &Path) -> Environment {
    Environment::from_vars([(COURSES_FILE, path.as_os_str())])
}

/// The example with `edit` applied to its JSON.
fn edited(edit: impl FnOnce(&mut Value)) -> String {
    let mut value = example();
    edit(&mut value);
    value.to_string()
}

/// Every string value of the example: a refusal may quote none of them.
fn values(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(text) => into.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| values(item, into)),
        Value::Object(map) => map.values().for_each(|item| values(item, into)),
        _ => {}
    }
}

/// What a malformed file's refusal may not quote: every string value of the file, or the whole
/// text when it is no JSON. The one exception is the schema's published name, `COURSES_SCHEMA`,
/// which a refusal names as the shape it expects.
fn unquotable(text: &str) -> Vec<String> {
    let mut quoted = Vec::new();
    match serde_json::from_str::<Value>(text) {
        Ok(value) => values(&value, &mut quoted),
        Err(_) => quoted.push(text.to_owned()),
    }
    quoted.retain(|value| value.len() > 1 && value != COURSES_SCHEMA);
    quoted
}

/// Each planted contradiction: what it is, the file's text, and the refusal it must meet.
fn planted_contradictions() -> Vec<(&'static str, String, CoursesError)> {
    let duplicate = |field: &'static str| CoursesError::Duplicate {
        setting: COURSES_FILE,
        field,
    };
    let bands_fault = |fault: &'static str| CoursesError::Bands {
        setting: COURSES_FILE,
        fault,
    };
    let planted: Vec<(&str, String, CoursesError)> = vec![
        (
            "a duplicate code",
            edited(|file| file["courses"][1]["code"] = json!("qaa")),
            duplicate("code"),
        ),
        (
            "a course's code taken by a focus subject",
            edited(|file| file["focus_subjects"][0]["code"] = json!("qab")),
            duplicate("code"),
        ),
        (
            "a duplicate alias",
            edited(|file| file["courses"][1]["alias"] = json!("a")),
            duplicate("alias"),
        ),
        (
            "a course's alias taken by a focus subject",
            edited(|file| file["focus_subjects"][0]["alias"] = json!("b")),
            duplicate("alias"),
        ),
        (
            "a duplicate deck root",
            edited(|file| file["courses"][1]["deck_root"] = json!("Example Course A")),
            duplicate("deck root"),
        ),
        (
            "overlapping bands",
            edited(|file| file["courses"][0]["unit_bands"]["A2"] = json!([10, 20])),
            bands_fault("overlap"),
        ),
        (
            "a band that runs backwards",
            edited(|file| file["courses"][1]["unit_bands"]["A2"] = json!([16, 9])),
            bands_fault("run backwards"),
        ),
        (
            "bands out of order",
            edited(|file| {
                file["courses"][1]["unit_bands"] = json!({"A1": [9, 16], "A2": [1, 8]});
            }),
            bands_fault("come out of order"),
        ),
        (
            "a band that ends where the band before it starts",
            edited(|file| {
                file["courses"][1]["unit_bands"] = json!({"A1": [9, 16], "A2": [1, 9]});
            }),
            bands_fault("overlap"),
        ),
    ];
    planted
}

/// Each malformed file: what it is, and its text.
fn malformed_files() -> Vec<(&'static str, String)> {
    let malformed: Vec<(&str, String)> = vec![
        ("not JSON", "{ courses".to_owned()),
        (
            "another schema",
            edited(|file| file["schema"] = json!("deckstreak.courses.v0")),
        ),
        (
            "no courses list",
            edited(|file| file["courses"] = json!({})),
        ),
        (
            "a missing name",
            edited(|file| file["courses"][0]["name"] = Value::Null),
        ),
        (
            "a two-letter alias",
            edited(|file| file["courses"][0]["alias"] = json!("ab")),
        ),
        (
            "a one-character alias that is no letter",
            edited(|file| file["courses"][0]["alias"] = json!("1")),
        ),
        (
            "a course with no alias",
            edited(|file| file["courses"][1]["alias"] = Value::Null),
        ),
        (
            "an upper-case code",
            edited(|file| file["courses"][0]["code"] = json!("QAA")),
        ),
        (
            "an empty deck root",
            edited(|file| file["courses"][0]["deck_root"] = json!("")),
        ),
        (
            "a band not in CEFR",
            edited(|file| file["courses"][0]["unit_bands"]["D1"] = json!([61, 70])),
        ),
        (
            "a band of three numbers",
            edited(|file| file["courses"][0]["unit_bands"]["A1"] = json!([1, 5, 10])),
        ),
        (
            "a writing flag as text",
            edited(|file| file["courses"][0]["writing"] = json!("yes")),
        ),
        (
            "a list, not an object",
            json!(["Example Course Z"]).to_string(),
        ),
        (
            "a course that is no object",
            edited(|file| file["courses"][0] = json!("Example Course Z")),
        ),
        (
            "unit bands that are no object",
            edited(|file| file["courses"][1]["unit_bands"] = json!("Example bands")),
        ),
        (
            "focus subjects that are no list",
            edited(|file| file["focus_subjects"] = json!("Example subject Z")),
        ),
        (
            "a focus subject that is no object",
            edited(|file| file["focus_subjects"][0] = json!("Example subject Z")),
        ),
        (
            "a focus subject's upper-case code",
            edited(|file| file["focus_subjects"][0]["code"] = json!("QAC")),
        ),
        (
            "a focus subject's two-letter alias",
            edited(|file| file["focus_subjects"][0]["alias"] = json!("cd")),
        ),
    ];
    malformed
}

/// SPEC-071 §10: the boundaries of the file's reading that no test above pinned.
#[test]
fn a_one_unit_band_loads_and_any_course_or_focus_subject_is_digested() {
    let one_unit =
        edited(|file| file["courses"][1]["unit_bands"] = json!({"A1": [1, 1], "A2": [2, 8]}));
    let loaded = Courses::parse(&one_unit).expect("a band of one unit runs forwards");
    let band = &loaded.courses()[1].unit_bands[0];
    assert_eq!((band.band, band.first, band.last), ("A1", 1, 1));

    let lists = examined(
        "course lists",
        vec![
            ("courses and a focus subject", example().to_string(), true),
            (
                "courses alone",
                edited(|file| file["focus_subjects"] = json!([])),
                true,
            ),
            (
                "a focus subject alone",
                edited(|file| file["courses"] = json!([])),
                true,
            ),
            (
                "neither",
                edited(|file| {
                    file["courses"] = json!([]);
                    file["focus_subjects"] = json!([]);
                }),
                false,
            ),
        ],
    );
    for (what, text, digested) in lists {
        let loaded = Courses::parse(&text).expect(what);
        assert_eq!(loaded.digest().is_some(), digested, "{what}");
    }

    // A code reads as itself, and debugs as its type around it.
    let code = CourseCode::new("qaa").expect("a code");
    assert_eq!(format!("{code}"), "qaa");
    assert_eq!(format!("{code:?}"), "CourseCode(\"qaa\")");
}

#[test]
fn the_courses_file_refuses_a_duplicate_or_overlapping_course() {
    // The example loads: two courses and one focus subject, in the file's order.
    let loaded = Courses::load(&environment(&example_path())).expect("the example loads");
    let codes: Vec<&str> = loaded
        .courses()
        .iter()
        .map(|course| course.code.as_str())
        .collect();
    assert_eq!(codes, ["qaa", "qab"], "the example's courses");
    let first = &loaded.courses()[0];
    assert_eq!(first.deck_root, "Example Course A");
    assert_eq!(first.alias, 'a');
    assert!(first.writing);
    let bands: Vec<(&str, u32, u32)> = first
        .unit_bands
        .iter()
        .map(|band| (band.band, band.first, band.last))
        .collect();
    assert_eq!(
        bands,
        [
            ("A1", 1, 10),
            ("A2", 11, 20),
            ("B1", 21, 30),
            ("B2", 31, 40),
            ("C1", 41, 50),
            ("C2", 51, 60)
        ]
    );
    let subjects: Vec<(&str, char)> = loaded
        .focus_subjects()
        .iter()
        .map(|subject| (subject.code.as_str(), subject.alias))
        .collect();
    assert_eq!(subjects, [("qac", 'c')]);

    // With the setting unset there are no courses, and that is no refusal.
    let unset = Courses::load(&Environment::default()).expect("an unset setting is no refusal");
    assert!(unset.courses().is_empty() && unset.digest().is_none());

    let planted = planted_contradictions();
    for (what, text, expected) in examined("planted contradictions", planted) {
        let refusal = Courses::parse(&text).expect_err(what);
        assert_eq!(refusal, expected, "{what}");
        let said = refusal.to_string();
        assert!(said.contains(COURSES_FILE), "{what}: {said}");
        let mut quoted = Vec::new();
        values(&serde_json::from_str(&text).expect("JSON"), &mut quoted);
        for value in quoted.iter().filter(|value| value.len() > 1) {
            assert!(
                !said.contains(value.as_str()),
                "{what}: {said} quotes {value}"
            );
        }
    }

    let malformed = malformed_files();
    for (what, text) in examined("malformed files", malformed) {
        let refusal = Courses::parse(&text).expect_err(what);
        assert!(
            matches!(refusal, CoursesError::Malformed { setting, .. } if setting == COURSES_FILE),
            "{what}: {refusal:?}"
        );
        // A malformed file's refusal names the setting and quotes no value, as a contradiction's
        // does (R1).
        let said = refusal.to_string();
        assert!(said.contains(COURSES_FILE), "{what}: {said}");
        for value in unquotable(&text) {
            assert!(
                !said.contains(value.as_str()),
                "{what}: {said} quotes {value}"
            );
        }
    }

    // A file that cannot be read refuses start without naming its path.
    let scratch = TempDir::new().expect("a scratch directory");
    let missing = scratch.path().join("no-such-courses.json");
    let refusal = Courses::load(&environment(&missing)).expect_err("a missing file refuses");
    assert_eq!(
        refusal,
        CoursesError::Unreadable {
            setting: COURSES_FILE
        }
    );
    assert!(!refusal.to_string().contains("no-such-courses"));
    let relative = Courses::load(&Environment::from_vars([(COURSES_FILE, "courses.json")]))
        .expect_err("a relative path refuses");
    assert_eq!(
        relative,
        CoursesError::Malformed {
            setting: COURSES_FILE,
            expected: "an absolute file path",
        }
    );
    for refused in [refusal, relative] {
        let said = refused.to_string();
        assert!(said.contains(COURSES_FILE), "{said}");
        assert!(!said.contains("courses.json"), "{said} quotes the path");
    }
}

#[tokio::test]
async fn a_changed_courses_file_bumps_the_settings_generation_once() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    assert_eq!(db.settings_generation().await.expect("readable"), 0);
    assert_eq!(db.courses_digest().await.expect("readable"), None);

    let loaded = Courses::load(&environment(&example_path())).expect("the example loads");
    let digest = loaded
        .digest()
        .expect("loaded courses have a digest")
        .to_owned();
    assert_eq!(digest.len(), 16, "sixteen hex digits: {digest}");
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));

    // The same courses, laid out differently, are the same courses.
    let reformatted = Courses::parse(&example().to_string()).expect("the example parses");
    assert_eq!(reformatted.digest(), Some(digest.as_str()));
    // A changed course is a changed file.
    let renamed = Courses::parse(&edited(|file| {
        file["courses"][0]["name"] = json!("Example course A, renamed");
    }))
    .expect("the edit parses");
    let changed = renamed.digest().expect("a digest").to_owned();
    assert_ne!(changed, digest);

    // The first start with the file bumps the generation once, and records the digest.
    assert!(
        db.record_courses_digest(Some(&digest))
            .await
            .expect("recorded")
    );
    assert_eq!(db.settings_generation().await.expect("readable"), 1);
    assert_eq!(
        db.courses_digest().await.expect("readable"),
        Some(digest.clone())
    );
    // A start with the same file changes nothing.
    assert!(
        !db.record_courses_digest(Some(&digest))
            .await
            .expect("recorded")
    );
    assert_eq!(db.settings_generation().await.expect("readable"), 1);
    // A changed file bumps it once more, and only once.
    assert!(
        db.record_courses_digest(Some(&changed))
            .await
            .expect("recorded")
    );
    assert!(
        !db.record_courses_digest(Some(&changed))
            .await
            .expect("recorded")
    );
    assert_eq!(db.settings_generation().await.expect("readable"), 2);
    // The file taken away is a change too: no courses is a digest of none.
    assert!(db.record_courses_digest(None).await.expect("recorded"));
    assert!(!db.record_courses_digest(None).await.expect("recorded"));
    assert_eq!(db.settings_generation().await.expect("readable"), 3);
    assert_eq!(db.courses_digest().await.expect("readable"), None);

    // The digest is the stable FNV-1a of its bytes: the published vector for "a".
    assert_eq!(content_digest(b"a"), "af63dc4c8601ec8c");
    assert_eq!(content_digest(b""), "cbf29ce484222325");
}

/// A writing course coded `all` would settle its own writing as `write:all`, the source the day's
/// writing bonus is settled under, so two amounts would share one row: the file is refused, naming
/// the setting and no value (SPEC-078 R7, A32). A course coded `all` that does not write is no
/// such clash, and loads.
#[test]
fn a_writing_course_coded_all_is_refused() {
    let text = edited(|file| {
        file["courses"][0]["code"] = json!("all");
        file["courses"][0]["writing"] = json!(true);
    });
    let refusal = Courses::parse(&text);
    assert!(
        matches!(&refusal, Err(CoursesError::Malformed { setting, .. }) if *setting == COURSES_FILE),
        "a writing course coded all is refused: {refusal:?}"
    );
    let said = refusal.expect_err("refused").to_string();
    assert!(said.contains(COURSES_FILE), "{said}");
    for value in unquotable(&text) {
        assert!(!said.contains(value.as_str()), "{said} quotes {value}");
    }

    let reading = edited(|file| {
        file["courses"][1]["code"] = json!("all");
        file["courses"][1]["writing"] = json!(false);
    });
    let loaded = Courses::parse(&reading).expect("a reading course coded all loads");
    let codes: Vec<&str> = loaded
        .courses()
        .iter()
        .map(|course| course.code.as_str())
        .collect();
    assert_eq!(codes, ["qaa", "all"]);
}
