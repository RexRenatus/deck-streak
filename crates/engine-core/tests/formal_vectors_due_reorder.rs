//! SPEC-414 A7 (R8): the Rust reorder gives every vector the Lean port writes. For each line of
//! `formal/vectors/due-reorder.jsonl`, written by `formal/lean/Formal/DueReorderVectors.lean` from
//! the port in `formal/lean/Formal/DueReorder.lean`, `reorder` presents the same entries in the
//! same order. A score is written as the text `str::parse::<f64>` reads, so NaN, the infinities
//! and `-0` reach the function as the floats they name.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "an integration test panics on a malformed vector, and prints its examined count on \
              purpose"
)]

use std::collections::BTreeMap;

use deck_streak_engine_core::reorder::{DueScores, reorder};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/due-reorder.jsonl");

/// One queue entry as the vectors write it: card, due, interval, ease, difficulty, stability.
type Entry = [i64; 6];

/// An integer the vector holds.
fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer, not {value}"))
}

/// A list of queue entries.
fn entries(value: &Value) -> Vec<Entry> {
    value
        .as_array()
        .expect("an array of entries")
        .iter()
        .map(|entry| {
            let fields: Vec<i64> = entry
                .as_array()
                .expect("an entry is an array")
                .iter()
                .map(int)
                .collect();
            fields.try_into().expect("an entry has six fields")
        })
        .collect()
}

/// The input a vector names, or `None` for `null`.
fn scores(value: &Value) -> Option<DueScores> {
    if value.is_null() {
        return None;
    }
    let listed = value["scores"].as_array().expect("an array of scores");
    let scores: BTreeMap<i64, f64> = listed
        .iter()
        .map(|pair| {
            let text = pair[1].as_str().expect("a score is written as text");
            (
                int(&pair[0]),
                text.parse::<f64>().expect("a score's text parses"),
            )
        })
        .collect();
    assert_eq!(
        scores.len(),
        listed.len(),
        "a vector scores each card once: {value}"
    );
    Some(DueScores {
        day: u32::try_from(int(&value["day"])).expect("a day count fits"),
        snapshot: int(&value["snapshot"]),
        scores,
    })
}

#[test]
fn the_rust_reorder_gives_every_lean_vectors_output() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("the header parses");
    assert_eq!(header["schema"], "phx.formal.vectors.v1");
    assert_eq!(header["entry"], "DueReorder");
    assert_eq!(
        header["covers"], "crates/engine-core/src/reorder.rs",
        "the vectors were written from the entry that covers the reorder"
    );
    assert_eq!(header["anchor"], "reorder");
    let declared = header["vectors"].as_u64().expect("a declared count");

    let mut examined = 0_u64;
    let mut differing = Vec::new();
    for (at, line) in lines.enumerate() {
        let vector: Value = serde_json::from_str(line).expect("a vector parses");
        assert_eq!(vector["rule"], "reorder");
        let on = vector["on"].as_bool().expect("a switch");
        let today = u32::try_from(int(&vector["today"])).expect("today's count fits");
        let snapshot = int(&vector["snapshot"]);
        let input = scores(&vector["input"]);
        let expected = entries(&vector["output"]);
        let shown = reorder(
            on,
            today,
            snapshot,
            input.as_ref(),
            entries(&vector["queue"]),
            |entry: &Entry| entry[0],
        );
        if shown != expected {
            differing.push(format!(
                "line {}: {line}: the reorder gave {shown:?}",
                at + 2
            ));
        }
        examined += 1;
    }
    println!(
        "examined {examined} vector(s), {} differing",
        differing.len()
    );
    assert!(examined > 0, "examined 0 vectors: nothing was judged");
    assert_eq!(examined, declared, "the header declares {declared} vectors");
    assert!(
        differing.is_empty(),
        "{} of {examined} vectors differ; the first: {}",
        differing.len(),
        differing.first().map_or("", String::as_str)
    );
}
