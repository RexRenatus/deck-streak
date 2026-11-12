//! The renderer against the engine's render goldens (SPEC-603 A2 and A3, ADR-624 D3).
//!
//! `goldens_render_card_equal` compares every standard, non-cloze case of `anki.render.card` and
//! sets aside, by counted rules, the cases ADR-624 D3 decides. `goldens_note_type_check_equal`
//! holds the note-type check to every case of `anki.render.note_type_check`. A failure names case
//! indexes and counts, never a golden face's text.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_render::{
    CardContext, CardTemplate, NoteContent, NoteType, NoteTypeKind, RenderIssue, check_note_type,
    render,
};
use serde_json::Value;

/// The committed golden of the engine's card render.
const CARD: &str = "anki.render.card";

/// The committed golden of the engine's note-type check.
const CHECK: &str = "anki.render.note_type_check";

/// The deck the oracle's cards were rendered under, as its `Deck` and `Subdeck` fields show it.
const ORACLE_DECK: &str = "(Deck)";

/// The most case indexes a failure message lists.
const LISTED: usize = 20;

/// A golden's note type, read into the renderer's own type.
fn note_type(value: &Value) -> NoteType {
    let text = |v: &Value| v.as_str().expect("a string").to_owned();
    NoteType {
        name: text(&value["name"]),
        kind: if value["kind"].as_str() == Some("cloze") {
            NoteTypeKind::Cloze
        } else {
            NoteTypeKind::Standard
        },
        fields: value["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .map(text)
            .collect(),
        templates: value["templates"]
            .as_array()
            .expect("templates")
            .iter()
            .map(|t| CardTemplate {
                name: text(&t["name"]),
                front: text(&t["front"]),
                back: text(&t["back"]),
            })
            .collect(),
    }
}

/// Every filter name the note type's templates write, read by this test's own scan of the tags,
/// never by the renderer under test: each `{{` up to the next `}}`, split at colons, all but the
/// last part.
fn filters_used(value: &Value) -> Vec<String> {
    let mut found = Vec::new();
    for template in value["templates"].as_array().expect("templates") {
        for side in ["front", "back"] {
            let mut rest = template[side].as_str().expect("a side");
            while let Some(open) = rest.find("{{") {
                let inside = &rest[open + 2..];
                let Some(close) = inside.find("}}") else {
                    break;
                };
                let parts: Vec<&str> = inside[..close].trim().split(':').collect();
                for filter in &parts[..parts.len() - 1] {
                    found.push(filter.trim().to_owned());
                }
                rest = &inside[close + 2..];
            }
        }
    }
    found
}

/// The first few indexes of a list, for a failure message.
fn listed(indexes: &[usize]) -> &[usize] {
    &indexes[..indexes.len().min(LISTED)]
}

/// What `DeckStreak` gives for one case.
enum Outcome {
    /// The note-type check refused the note type.
    Refused,
    /// The card rendered.
    Faces(String, String),
    /// The render reported `EmptyFront`.
    EmptyFront,
    /// Any other render issue.
    Issue,
}

#[test]
#[allow(clippy::too_many_lines)]
fn goldens_render_card_equal() {
    let golden = golden::read(&golden::committed(CARD)).expect("the golden");
    let (mut cloze_kind, mut cloze_class, mut cloze_filter, mut hint) = (0, 0, 0, 0);
    let (mut examined, mut by_text, mut refused) = (0, 0, 0);
    let mut empty_front = Vec::new();
    let mut differ = Vec::new();
    for (index, case) in golden.cases.iter().enumerate() {
        let value = &case.input["notetype"];
        if value["kind"].as_str() != Some("standard") {
            cloze_kind += 1;
            continue;
        }
        if case
            .class
            .as_deref()
            .is_some_and(|c| c.starts_with("cloze"))
        {
            cloze_class += 1;
            continue;
        }
        let filters = filters_used(value);
        if filters.iter().any(|f| f == "cloze" || f == "cloze-only") {
            cloze_filter += 1;
            continue;
        }
        if filters.iter().any(|f| f == "hint") {
            hint += 1;
            continue;
        }
        examined += 1;
        let kind = note_type(value);
        let fields: Vec<String> = case.input["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .map(|f| f.as_str().expect("a field").to_owned())
            .collect();
        let card = CardContext {
            ordinal: u16::try_from(case.input["ord"].as_u64().expect("an ordinal"))
                .expect("a small ordinal"),
            deck_path: ORACLE_DECK,
            flag: 0,
            card_id: 0,
        };
        let note = NoteContent {
            fields: &fields,
            tags: &[],
        };
        let outcome = match check_note_type(&kind) {
            Err(_) => Outcome::Refused,
            Ok(()) => match render(&kind, &note, &card) {
                Ok(faces) => Outcome::Faces(faces.question, faces.answer),
                Err(RenderIssue::EmptyFront) => Outcome::EmptyFront,
                Err(_) => Outcome::Issue,
            },
        };
        let output = &case.output;
        match outcome {
            Outcome::Refused if output["refused"].as_str() == Some("card_type_error") => {
                refused += 1;
            }
            Outcome::Faces(question, answer)
                if output["question"].as_str() == Some(question.as_str())
                    && output["answer"].as_str() == Some(answer.as_str()) =>
            {
                by_text += 1;
            }
            Outcome::EmptyFront => empty_front.push(index),
            _ => differ.push(index),
        }
    }
    // The blank-front rule (ADR-624 D3): the golden's notice is read here, at run time, and never
    // printed. Every case DeckStreak reports as EmptyFront must carry one answer string, and its
    // question must end with that string.
    let notice = empty_front
        .first()
        .and_then(|&i| golden.cases[i].output["answer"].as_str())
        .unwrap_or_default()
        .to_owned();
    let mut blank_front = 0;
    for &index in &empty_front {
        let output = &golden.cases[index].output;
        let answer = output["answer"].as_str().unwrap_or_default();
        let question = output["question"].as_str().unwrap_or_default();
        if !notice.is_empty() && answer == notice && question.ends_with(&notice) {
            blank_front += 1;
        } else {
            differ.push(index);
        }
    }
    differ.sort_unstable();
    println!(
        "examined {examined} case(s) of {}: equal by text {by_text}, refused as the golden refuses \
         {refused}, blank-front by EmptyFront {blank_front}, differ {}; set aside: hint {hint}, \
         cloze note type {cloze_kind}, cloze class {cloze_class}, cloze filter {cloze_filter}",
        golden.function,
        differ.len()
    );
    assert!(
        differ.is_empty(),
        "{} of {examined} compared case(s) differ; the first, by index: {:?}",
        differ.len(),
        listed(&differ)
    );
    assert!(examined > 0, "the golden yielded no compared case");
    assert!(hint > 0, "no case was set aside by the hint rule");
    assert!(
        blank_front > 0,
        "no case was compared by the blank-front rule"
    );
}

#[test]
fn goldens_note_type_check_equal() {
    let golden = golden::read(&golden::committed(CHECK)).expect("the golden");
    let mut refused = 0;
    let mut differ = Vec::new();
    for (index, case) in golden.cases.iter().enumerate() {
        let accepted = check_note_type(&note_type(&case.input["notetype"])).is_ok();
        let expected = case.output["result"].as_str().expect("a result");
        if expected != "ok" {
            refused += 1;
        }
        if accepted != (expected == "ok") {
            differ.push(index);
        }
    }
    println!(
        "examined {} case(s) of {}: refused by the golden {refused}, differ {}",
        golden.cases.len(),
        golden.function,
        differ.len()
    );
    assert!(
        differ.is_empty(),
        "{} of {} case(s) differ; the first, by index: {:?}",
        differ.len(),
        golden.cases.len(),
        listed(&differ)
    );
    assert!(!golden.cases.is_empty(), "the golden yielded no case");
}
