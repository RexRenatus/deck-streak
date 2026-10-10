//! SPEC-386 A23 (R15): the history selection answers every vector the Lean port writes. For each
//! row set in `formal/vectors/replay-history.jsonl`, written by
//! `formal/lean/Formal/ReplayHistory.lean`'s port, `convert::histories` gives the same cards in the
//! same order, the same last kept ids, the same ratings and the same distances; the port keeps a
//! distance in milliseconds, and this reader divides it into days. The crate's one dependency stays
//! the pinned scheduler (A1), so the vectors are read by a small parser of integer arrays.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "an integration test panics on a malformed vector, and prints its examined count on \
              purpose"
)]

use deck_streak_fsrs7::convert::{self, RevlogRow};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/replay-history.jsonl");

/// Milliseconds in a day, typed here: the port's distances are milliseconds, and FSRS-7 reads days.
const MS_PER_DAY: f64 = 86_400_000.0;

/// One value of a vector line: an integer, or an array of values.
#[derive(Debug)]
enum Value {
    Int(i64),
    List(Vec<Value>),
}

impl Value {
    fn int(&self) -> i64 {
        match self {
            Self::Int(number) => *number,
            Self::List(_) => panic!("an integer, not {self:?}"),
        }
    }

    fn list(&self) -> &[Self] {
        match self {
            Self::List(items) => items,
            Self::Int(_) => panic!("an array, not {self:?}"),
        }
    }
}

/// Reads the value at the start of `text`, and returns it with the text after it.
fn value(text: &str) -> (Value, &str) {
    if let Some(mut rest) = text.strip_prefix('[') {
        let mut items = Vec::new();
        if let Some(after) = rest.strip_prefix(']') {
            return (Value::List(items), after);
        }
        loop {
            let (item, after) = value(rest);
            items.push(item);
            if let Some(next) = after.strip_prefix(',') {
                rest = next;
            } else if let Some(end) = after.strip_prefix(']') {
                return (Value::List(items), end);
            } else {
                panic!("a ',' or a ']' before {after:?}");
            }
        }
    }
    let digits = text
        .find(|c: char| c != '-' && !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (number, after) = text.split_at(digits);
    (Value::Int(number.parse().expect("an integer")), after)
}

/// The value a line holds under `key`.
fn field(line: &str, key: &str) -> Value {
    let marker = format!("\"{key}\":");
    let (_, after) = line
        .split_once(&marker)
        .unwrap_or_else(|| panic!("{line} has no {marker}"));
    value(after).0
}

/// A distance in milliseconds as FSRS-7's days.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the vectors' distances are under a day apart, exact in an f64, and FSRS-7 reads an f32"
)]
fn days(ms: i64) -> f32 {
    (ms as f64 / MS_PER_DAY) as f32
}

/// A row as its five integers.
fn row(value: &Value) -> RevlogRow {
    let [cid, id, ease, kind, factor] = value.list() else {
        panic!("a row of five integers, not {value:?}");
    };
    RevlogRow {
        cid: cid.int(),
        id: id.int(),
        ease: u8::try_from(ease.int()).expect("an ease"),
        kind: u8::try_from(kind.int()).expect("a kind"),
        factor: u32::try_from(factor.int()).expect("a factor"),
    }
}

/// A card, its last kept id, and each review's rating and distance's bits.
type Selected = (i64, i64, Vec<(u32, u32)>);

/// A history of the vectors as its card, its last kept id and its reviews.
fn expected(value: &Value) -> Selected {
    let [cid, last_id, reviews] = value.list() else {
        panic!("a history of three values, not {value:?}");
    };
    let reviews = reviews
        .list()
        .iter()
        .map(|review| {
            let [rating, ms] = review.list() else {
                panic!("a review of two integers, not {review:?}");
            };
            (
                u32::try_from(rating.int()).expect("a rating"),
                days(ms.int()).to_bits(),
            )
        })
        .collect();
    (cid.int(), last_id.int(), reviews)
}

#[test]
fn the_history_selection_matches_the_lean_vectors() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    for named in [
        "\"schema\":\"phx.formal.vectors.v1\"",
        "\"entry\":\"ReplayHistory\"",
        "\"covers\":\"crates/fsrs7/src/convert.rs\"",
        "\"anchor\":\"histories\"",
    ] {
        assert!(header.contains(named), "the header {header} names {named}");
    }
    let declared = field(header, "vectors").int();

    let mut examined = 0_i64;
    for line in lines {
        let rows: Vec<RevlogRow> = field(line, "rows").list().iter().map(row).collect();
        let want: Vec<Selected> = field(line, "histories")
            .list()
            .iter()
            .map(expected)
            .collect();
        let got: Vec<Selected> = convert::histories(&rows)
            .iter()
            .map(|history| {
                (
                    history.cid,
                    history.last_id,
                    history
                        .item
                        .reviews
                        .iter()
                        .map(|review| (review.rating, review.delta_t.to_bits()))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(got, want, "the rows of {line}");
        examined += 1;
    }

    assert_eq!(
        examined, declared,
        "the header declares {declared} vector(s)"
    );
    println!("examined {examined} vector(s)");
    assert!(
        examined > 0,
        "examined 0 vectors: the population is empty, so nothing was judged"
    );
}
