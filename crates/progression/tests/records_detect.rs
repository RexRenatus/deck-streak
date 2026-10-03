//! The record detection and the record to chase are the predecessor's (SPEC-073 A12, A13; R10,
//! R13): each kind's best over the rollups, a record only strictly above the stored best, the
//! minutes whole; and the smallest positive gap from today, the first on a tie.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;

use deck_streak_progression::records::{BoardLine, DayTotals, RecordKind, chase, detect_records};
use serde_json::Value;

/// A golden field as a number, 0 when the field is missing.
fn number(value: &Value) -> i64 {
    value.as_i64().unwrap_or(0)
}

#[test]
fn detect_records_matches_the_parity_golden() {
    let mut detected = 0_usize;
    golden::each_case("detect_records", |case| {
        let input = &case.input;
        let rollups: Vec<DayTotals> = input["rollups"]
            .as_array()
            .expect("the rollups")
            .iter()
            .map(|row| DayTotals {
                score: number(&row["score"]),
                reviews: number(&row["reviews"]),
                seconds: row["seconds"].as_f64().unwrap_or(0.0),
            })
            .collect();
        let previous: BTreeMap<RecordKind, i64> = input["prev"]
            .as_object()
            .expect("the stored bests")
            .iter()
            .map(|(kind, value)| (RecordKind::parse(kind).expect("a kind"), number(value)))
            .collect();
        let ours: Vec<(String, i64, String)> = detect_records(&rollups, &previous)
            .into_iter()
            .map(|record| {
                (
                    record.kind.as_str().to_owned(),
                    record.value,
                    record.kind.label().to_owned(),
                )
            })
            .collect();
        let theirs: Vec<(String, i64, String)> = case
            .output
            .as_array()
            .expect("the records")
            .iter()
            .map(|row| {
                (
                    row[0].as_str().expect("a kind").to_owned(),
                    number(&row[1]),
                    row[2].as_str().expect("a label").to_owned(),
                )
            })
            .collect();
        assert_eq!(ours, theirs, "the records of {input}");
        detected += theirs.len();
    });
    println!("examined {detected} detected record(s)");
    assert!(detected > 0, "some case detects a record");
}

#[test]
fn the_chase_record_matches_the_parity_golden() {
    let mut chased = 0_usize;
    golden::each_case("records_chase", |case| {
        let board = case.input["board"].as_array().expect("the board");
        let lines: Vec<BoardLine> = board
            .iter()
            .map(|line| BoardLine {
                value: number(&line["value"]),
                today: number(&line["today"]),
            })
            .collect();
        let ours = chase(&lines).map(|(at, gap)| {
            (
                board[at]["label"].as_str().expect("a label").to_owned(),
                gap,
            )
        });
        let theirs = case.output["chase"].as_array().map(|pair| {
            (
                pair[0].as_str().expect("a label").to_owned(),
                number(&pair[1]),
            )
        });
        assert_eq!(ours, theirs, "the record to chase on {}", case.input);
        chased += usize::from(theirs.is_some());
    });
    println!("examined {chased} board(s) with a record to chase");
    assert!(chased > 0, "some board names a record to chase");
}
