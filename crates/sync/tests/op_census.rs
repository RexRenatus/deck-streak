//! The census of the vocabulary (SPEC-509 A08): no variant of `Op` names a
//! review row updated or deleted, a note's notetype changed, a preset deleted, a card reset or a
//! whole collection replaced: the never-list. The census reads `crates/sync/src/op.rs` by path,
//! takes every variant of `pub enum Op` in order, and checks they are one per kind, in the kinds'
//! order. It prints how many variants it examined and refuses zero, then refuses a planted
//! variant of each forbidden class by name, so a census that went blind fails on its own
//! controls. It reads one file and starts no process.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::Path;

use deck_streak_sync::OpKind;

/// The words that change or remove what they follow.
const CHANGES: [&str; 11] = [
    "Edited",
    "Updated",
    "Changed",
    "Set",
    "Modified",
    "Rewritten",
    "Replaced",
    "Deleted",
    "Removed",
    "Dropped",
    "Cleared",
];

/// The words that remove.
const REMOVES: [&str; 3] = ["Deleted", "Removed", "Dropped"];

/// The words that reset a card.
const RESETS: [&str; 4] = ["Reset", "Forgotten", "Forget", "Relearned"];

/// The words that replace a whole collection.
const REPLACES: [&str; 6] = [
    "Replace",
    "Replaced",
    "Overwrite",
    "Overwritten",
    "Whole",
    "Full",
];

/// The words that name a note's type.
const RETYPES: [&str; 3] = ["Notetype", "Retyped", "Type"];

/// A name's words: it splits before each capital letter.
fn words(name: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = 0;
    for (at, ch) in name.char_indices().skip(1) {
        if ch.is_ascii_uppercase() {
            found.push(&name[start..at]);
            start = at;
        }
    }
    found.push(&name[start..]);
    found
}

/// The never-list's class a variant's name falls in, or `None` for a name it admits.
fn refusal(name: &str) -> Option<&'static str> {
    let held = words(name);
    let has = |word: &str| held.contains(&word);
    let any = |list: &[&str]| list.iter().any(|word| has(word));
    if has("Review") && any(&CHANGES) {
        Some("a review row updated or deleted")
    } else if has("Note") && any(&RETYPES) {
        Some("a note's notetype changed")
    } else if has("Preset") && any(&REMOVES) {
        Some("a preset deleted")
    } else if has("Card") && any(&RESETS) {
        Some("a card reset")
    } else if any(&REPLACES) {
        Some("a whole replace")
    } else {
        None
    }
}

/// The variants of `pub enum <name>` in `source`, in order: each line one indent in that opens
/// with a capital letter.
fn variants(source: &str, name: &str) -> Vec<String> {
    let opening = format!("pub enum {name} {{");
    let mut found = Vec::new();
    let mut inside = false;
    for line in source.lines() {
        if !inside {
            inside = line == opening;
            continue;
        }
        if line == "}" {
            break;
        }
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(|ch: char| ch.is_ascii_uppercase()) {
            let variant: String = rest
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            found.push(variant);
        }
    }
    found
}

/// Each variant the never-list refuses, as `<name>: <class>`.
fn judge(variants: &[String]) -> Vec<String> {
    variants
        .iter()
        .filter_map(|name| refusal(name).map(|class| format!("{name}: {class}")))
        .collect()
}

#[test]
fn no_operation_variant_names_a_review_edit_a_retype_a_preset_delete_a_card_reset_or_a_replace() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/op.rs");
    let source = fs::read_to_string(path).expect("the vocabulary reads");
    let found = variants(&source, "Op");
    println!("examined {} variant(s) of Op", found.len());
    assert!(
        !found.is_empty(),
        "examined 0 variants: the census read nothing"
    );
    assert_eq!(judge(&found), Vec::<String>::new(), "the never-list");
    let kinds: Vec<String> = OpKind::ALL.iter().map(|kind| format!("{kind:?}")).collect();
    assert_eq!(found, kinds, "one kind per variant, in order");

    let planted = source.replace(
        "pub enum Op {\n",
        "pub enum Op {\n    /// Planted.\n    ReviewEdited {\n        ease: u8,\n    },\n",
    );
    assert_eq!(
        judge(&variants(&planted, "Op"))[..1],
        ["ReviewEdited: a review row updated or deleted".to_owned()],
        "a planted variant in the enum is refused by name"
    );
    let named: Vec<String> = [
        "ReviewEdited",
        "ReviewDeleted",
        "NoteNotetypeChanged",
        "NoteRetyped",
        "PresetDeleted",
        "CardReset",
        "CardForgotten",
        "CollectionReplaced",
        "FullUpload",
        "ReviewUndone",
        "NotetypeEdited",
        "NoteFieldSet",
        "NoteDeleted",
    ]
    .map(str::to_owned)
    .to_vec();
    assert_eq!(
        judge(&named),
        vec![
            "ReviewEdited: a review row updated or deleted".to_owned(),
            "ReviewDeleted: a review row updated or deleted".to_owned(),
            "NoteNotetypeChanged: a note's notetype changed".to_owned(),
            "NoteRetyped: a note's notetype changed".to_owned(),
            "PresetDeleted: a preset deleted".to_owned(),
            "CardReset: a card reset".to_owned(),
            "CardForgotten: a card reset".to_owned(),
            "CollectionReplaced: a whole replace".to_owned(),
            "FullUpload: a whole replace".to_owned(),
        ],
        "each planted class is refused by name, and an admitted name is not"
    );
}
