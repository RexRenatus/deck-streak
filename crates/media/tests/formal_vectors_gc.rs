//! The sweep's decision answers every vector its Lean port writes (SPEC-513 A6): for each line of
//! `formal/vectors/media-gc.jsonl`, written by `formal/lean/Formal/MediaGc.lean`'s port, `fate`
//! gives the same answer.

#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_media::{Facts, Fate, Keep, SweepPolicy, fate};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/media-gc.jsonl");

fn flag(row: &Value, name: &str) -> bool {
    row[name]
        .as_bool()
        .unwrap_or_else(|| panic!("a flag {name} in {row}"))
}

fn fate_name(fate: Fate) -> &'static str {
    match fate {
        Fate::Keep(Keep::Live) => "live",
        Fate::Keep(Keep::Pinned) => "pinned",
        Fate::Keep(Keep::InGrace) => "in_grace",
        Fate::Keep(Keep::Imported) => "imported",
        Fate::Delete => "delete",
    }
}

#[test]
fn fate_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("the header is JSON");
    assert_eq!(header["schema"], "phx.formal.vectors.v1");
    assert_eq!(header["entry"], "MediaGc");
    assert_eq!(header["covers"], "crates/media/src/sweep.rs");
    assert_eq!(header["anchor"], "fate");
    let rows: Vec<Value> = lines
        .map(|line| serde_json::from_str(line).expect("a vector is JSON"))
        .collect();
    assert_eq!(
        usize::try_from(header["vectors"].as_u64().expect("a count")).expect("a count"),
        rows.len(),
        "the header counts the lines that follow it"
    );
    let mut by_answer = std::collections::BTreeMap::new();
    for row in &rows {
        let facts = Facts {
            referenced: flag(row, "referenced"),
            pinned: flag(row, "pinned"),
            imported: flag(row, "imported"),
            added_ms: row["added_ms"].as_i64().expect("an added instant"),
        };
        let policy = SweepPolicy {
            now_ms: row["now_ms"].as_i64().expect("a sweep instant"),
            grace_ms: row["grace_ms"].as_u64().expect("a window"),
        };
        let expected = row["fate"].as_str().expect("a fate");
        assert_eq!(fate_name(fate(facts, policy)), expected, "{row}");
        *by_answer.entry(expected.to_owned()).or_insert(0) += 1;
    }
    println!("examined {} vector(s): {by_answer:?}", rows.len());
    assert_eq!(rows.len(), 244);
    assert_eq!(by_answer.len(), 5, "every answer is drawn");
}
