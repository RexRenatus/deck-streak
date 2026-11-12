//! The kept-away rule answers every vector the Lean port writes (SPEC-431 R20, A12; #878): for
//! each card in `formal/vectors/sensitive-deck.jsonl`, written by
//! `formal/lean/Formal/SensitiveDeck.lean`'s port over the vectors' deck tree, `admits` gives the
//! same admission.
//!
//! Each deck's name is joined from its parts by the port's separator, written here as a literal and
//! never read from the code under test: a separator in the code that differs from the port's leaves
//! every child deck outside its marked parent, and the answers differ.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::sensitive::{Admission, admits};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/sensitive-deck.jsonl");

/// The port's separator between the parts of a deck's name, `deckSeparator`.
const SEPARATOR: &str = "\u{1f}";

/// An admission as the writer names it.
fn named(admission: Admission) -> &'static str {
    match admission {
        Admission::Admitted => "admitted",
        Admission::KeptAway => "kept_away",
        Admission::Unresolved => "unresolved",
        Admission::Unreadable => "unreadable",
    }
}

/// A vector field as a deck id.
fn deck(vector: &Value, field: &str) -> i64 {
    vector[field]
        .as_i64()
        .unwrap_or_else(|| panic!("a deck id {field} in {vector}"))
}

/// A vector's marked set, or `None` for the set that could not be read.
fn marked(vector: &Value) -> Option<BTreeSet<i64>> {
    match &vector["marked"] {
        Value::Null => None,
        Value::Array(decks) => Some(
            decks
                .iter()
                .map(|deck| deck.as_i64().expect("a marked deck id"))
                .collect(),
        ),
        other => panic!("a marked set {other} in {vector}"),
    }
}

#[test]
fn the_kept_away_rule_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "SensitiveDeck", "{header}");
    assert_eq!(
        header["covers"], "crates/ingest/src/sensitive.rs",
        "{header}"
    );
    assert_eq!(header["anchor"], "admits", "{header}");
    let vectors: Vec<Value> = lines
        .map(|line| serde_json::from_str(line).expect("a JSON vector"))
        .collect();
    let mut tree = BTreeMap::new();
    for vector in vectors.iter().filter(|vector| vector["rule"] == "tree") {
        let parts: Vec<&str> = vector["parts"]
            .as_array()
            .expect("a deck's name parts")
            .iter()
            .map(|part| part.as_str().expect("a name part"))
            .collect();
        let name = parts.join(SEPARATOR);
        assert!(
            tree.insert(deck(vector, "deck"), name).is_none(),
            "a deck listed twice: {vector}"
        );
    }
    let mut answers = BTreeMap::<&str, u64>::new();
    for vector in vectors.iter().filter(|vector| vector["rule"] != "tree") {
        assert_eq!(vector["rule"], "admits", "an unknown rule in {vector}");
        let ours = admits(
            marked(vector).as_ref(),
            &tree,
            deck(vector, "home"),
            deck(vector, "current"),
        );
        assert_eq!(vector["admission"], named(ours), "{vector}");
        *answers.entry(named(ours)).or_default() += 1;
    }
    let cards: u64 = answers.values().sum();
    println!("examined {} deck(s) and {cards} card vector(s)", tree.len());
    println!("answers {answers:?}");
    assert_eq!(tree.len(), 7, "every deck of the writer's tree");
    assert_eq!(
        answers.into_iter().collect::<Vec<_>>(),
        vec![
            ("admitted", 71),
            ("kept_away", 29),
            ("unreadable", 12),
            ("unresolved", 20)
        ],
        "every answer the writer prints"
    );
    assert_eq!(header["vectors"].as_u64(), Some(7 + cards), "{header}");
}
