//! The content rails refuse every planted fixture by its own rail row and pass a clean reading note
//! (SPEC-042 A2, R3), hold the vendored `rails.json` to the rail kinds the port reads, and refuse a
//! control character with the adapter's own rail, naming the rail and the line and never the text.

// An integration test is test code: its helpers panic on an unreadable fixture, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_kernel::Verdict;
use deck_streak_vault::Rails;
use deck_streak_vault::rails::{CONTROL_CHARACTER, KNOWN_KEYS, VENDORED};

/// Prints how many items a check examined and refuses zero: a fixture set that stopped matching
/// must fail, never pass over the empty set (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The planted fixtures' directory.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rails")
}

/// The text of the fixture `name`.
fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(name)).expect("a readable fixture")
}

/// The fixtures' index: the rail row each planted fixture holds, and the clean note.
struct Index {
    rows: BTreeMap<String, String>,
    clean: String,
}

fn index() -> Index {
    let text = fixture("rows.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("rows.json is JSON");
    let rows = value["rows"]
        .as_object()
        .expect("rows.json maps each rail row to its fixture")
        .iter()
        .map(|(row, file)| {
            let file = file.as_str().expect("a fixture's file name");
            (row.clone(), file.to_owned())
        })
        .collect();
    let clean = value["clean"]
        .as_str()
        .expect("the clean note's name")
        .to_owned();
    Index { rows, clean }
}

/// Every rail row that refuses `text`.
fn rows_refusing(rails: &Rails, text: &str) -> BTreeSet<String> {
    rails
        .refusals(text)
        .into_iter()
        .map(|refusal| refusal.row.to_string())
        .collect()
}

#[test]
fn every_rail_refuses_its_planted_fixture_and_a_clean_note_passes() {
    let rails = Rails::vendored().expect("the vendored rails.json reads as rails");
    let rows: Vec<String> = rails.rows().iter().map(ToString::to_string).collect();
    let rows = examined("rail row(s) of rails.json", rows);
    let index = index();
    assert_eq!(
        index.rows.keys().cloned().collect::<Vec<_>>(),
        rows,
        "rows.json plants one fixture for every rail row of rails.json, and for nothing else"
    );
    let mut refused = 0;
    for (row, file) in &index.rows {
        let text = fixture(file);
        assert_eq!(
            rows_refusing(&rails, &text),
            BTreeSet::from([row.clone()]),
            "the planted fixture {file} is refused by its own rail row {row}, and by no other"
        );
        match rails.check(&text) {
            Verdict::Refuse(refusal) => assert_eq!(refusal.row.as_str(), row, "{file}"),
            Verdict::Pass => panic!("the planted fixture {file} passed the rails"),
        }
        refused += 1;
    }
    assert_eq!(
        refused,
        rows.len(),
        "the examined count equals the rail rows"
    );
    let clean = fixture(&index.clean);
    assert_eq!(
        rails.check(&clean),
        Verdict::Pass,
        "the clean reading note passes the rails: {:?}",
        rails.refusals(&clean)
    );
}

#[test]
fn rails_json_holds_only_the_rail_kinds_the_port_reads() {
    let document: serde_json::Value = serde_json::from_str(VENDORED).expect("rails.json is JSON");
    let keys: BTreeSet<&str> = document
        .as_object()
        .expect("rails.json is an object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        KNOWN_KEYS.into_iter().collect::<BTreeSet<_>>(),
        "a key rails.json gained is a new kind of rail, which the port needs code for (ADR-042)"
    );
}

#[test]
fn a_control_character_is_refused_by_the_adapters_own_rail_and_a_tab_passes() {
    let rails = Rails::vendored().expect("the vendored rails.json reads as rails");
    for code in [0x00_u32, 0x07, 0x0b, 0x0c, 0x1b, 0x7f, 0x85] {
        let control = char::from_u32(code).expect("a control character");
        let text = format!("A clean first line.\nA second line with {control} in it.\n");
        let refusals = rails.refusals(&text);
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| (refusal.row.as_str(), refusal.line))
                .collect::<Vec<_>>(),
            vec![(CONTROL_CHARACTER, 2)],
            "U+{code:04X} is refused on its own line"
        );
    }
    assert_eq!(
        rails.check("A tab\tseparates these.\r\nA Windows line ends here.\n"),
        Verdict::Pass,
        "a tab, a carriage return and a line feed pass"
    );
}

#[test]
fn a_refusal_names_its_rail_and_line_and_never_the_text() {
    let rails = Rails::vendored().expect("the vendored rails.json reads as rails");
    let text = "A first line.\nA planted private-marker <% tp.user.command() %> line.\n";
    let Verdict::Refuse(refusal) = rails.check(text) else {
        panic!("a Templater command passed the rails");
    };
    assert_eq!((refusal.row.as_str(), refusal.line), ("templater_open", 2));
    let shown = refusal.to_string();
    assert_eq!(shown, "the rail templater_open refuses line 2");
    assert!(
        !shown.contains("private-marker"),
        "the refusal echoed the text: {shown}"
    );
}
