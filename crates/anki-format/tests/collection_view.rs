//! A8 (SPEC-602 R8): each collection of `anki.read.collection_view`, written by the engine's own
//! connection, reads through the crate as the golden's per-table view.

#![allow(clippy::expect_used, clippy::print_stdout, clippy::too_many_lines)]

mod support;

use deck_streak_anki_format::read::Collection;
use deck_streak_anki_format::{Kind, fields, read};
use serde_json::{Value, json};
use support::{CARD, DECK, NOTE, NOTETYPE, case_collection, examined, golden_cases, open_file};

/// A corpus index from an id, or a null for `none`.
fn index(id: i64, base: i64, none: i64) -> Value {
    if id == none {
        Value::Null
    } else {
        json!(id - base)
    }
}

/// The per-table view of the rows the crate read, in the golden's shape.
fn view(rows: &Collection) -> Value {
    let decks: Vec<Value> = rows
        .decks
        .iter()
        .map(|deck| {
            json!({
                "filtered": deck.kind.view.filtered,
                "index": index(deck.id, DECK, 1),
                "name": deck.name.replace('\u{1f}', "::"),
            })
        })
        .collect();
    let notetypes: Vec<Value> = rows
        .notetypes
        .iter()
        .map(|notetype| {
            let mut own_fields: Vec<_> = rows
                .fields
                .iter()
                .filter(|field| field.ntid == notetype.id)
                .collect();
            own_fields.sort_by_key(|field| field.ord);
            let mut templates: Vec<_> = rows
                .templates
                .iter()
                .filter(|template| template.ntid == notetype.id)
                .collect();
            templates.sort_by_key(|template| template.ord);
            json!({
                "css": notetype.config.view.css,
                "fields": own_fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>(),
                "index": index(notetype.id, NOTETYPE, 1),
                "kind": notetype.config.view.kind,
                "name": notetype.name,
                "sort_field": notetype.config.view.sort_field,
                "templates": templates.iter().map(|template| json!({
                    "back": template.config.view.a_format,
                    "front": template.config.view.q_format,
                    "name": template.name,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let notes: Vec<Value> = rows
        .notes
        .iter()
        .map(|note| {
            json!({
                "csum": note.csum,
                "fields": fields::split(&note.flds),
                "guid": note.guid,
                "index": note.id - NOTE,
                "notetype": index(note.mid, NOTETYPE, 1),
                "sfld": note.sfld,
                "tags": note.tags.split_whitespace().collect::<Vec<_>>(),
            })
        })
        .collect();
    let cards: Vec<Value> = rows
        .cards
        .iter()
        .map(|card| {
            json!({
                "deck": index(card.did, DECK, 1),
                "due": card.due,
                "factor": card.factor,
                "index": card.id - CARD,
                "ivl": card.ivl,
                "lapses": card.lapses,
                "note": card.nid - NOTE,
                "odeck": index(card.odid, DECK, 0),
                "odue": card.odue,
                "ord": card.ord,
                "queue": card.queue,
                "reps": card.reps,
                "type": card.r#type,
            })
        })
        .collect();
    let revlog: Vec<Value> = rows
        .revlog
        .iter()
        .map(|review| {
            json!({
                "at_ms": review.id,
                "card": review.cid - CARD,
                "ease": review.ease,
                "factor": review.factor,
                "ivl": review.ivl,
                "last_ivl": review.last_ivl,
                "taken_ms": review.time,
                "type": review.r#type,
                "usn": review.usn,
            })
        })
        .collect();
    json!({
        "cards": cards,
        "decks": decks,
        "notes": notes,
        "notetypes": notetypes,
        "revlog": revlog,
    })
}

/// A table's rows in one order: each row's JSON text, sorted.
fn canonical(rows: &Value) -> Vec<String> {
    let mut texts: Vec<String> = rows
        .as_array()
        .map(|rows| rows.iter().map(Value::to_string).collect())
        .unwrap_or_default();
    texts.sort();
    texts
}

#[test]
fn each_golden_collection_reads_as_its_per_table_view() {
    let cases = examined(
        "collection view case(s)",
        golden_cases("anki.read.collection_view"),
    );
    let mut findings = Vec::new();
    for (number, case) in cases.iter().enumerate() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = case_collection(dir.path(), case);
        let read = open_file(&path, Kind::Collection)
            .map_err(|refusal| refusal.to_string())
            .and_then(|reader| read::collection(&reader).map_err(|refusal| refusal.to_string()));
        let rows = match read {
            Ok(rows) => rows,
            Err(refusal) => {
                findings.push(format!("case {number}: {refusal}"));
                continue;
            }
        };
        let found = view(&rows);
        for table in ["cards", "decks", "notes", "notetypes", "revlog"] {
            let wanted = canonical(&case["output"][table]);
            let got = canonical(&found[table]);
            if got != wanted {
                findings.push(format!(
                    "case {number} {table}: read {} row(s), the golden holds {}; first difference {:?}",
                    got.len(),
                    wanted.len(),
                    got.iter().zip(&wanted).find(|(a, b)| a != b)
                ));
            }
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the per-table views");
}
