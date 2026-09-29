//! The structure reads (SPEC-094 A4 to A8): the wire walk equals the predecessor's, a failed read
//! is named beside the rows that did read, the presence read holds 400 notes at a time, the scope
//! bounds every read, and a returned name is made safe.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::collections::BTreeSet;

use deck_streak_ingest::structure::{NOTES_BATCH_SIZE, READ_DECLARED_FIELDS, safe_name};
use deck_streak_ingest::wire::{self, WireValue};
use support::Fixture;
use support::synthetic::{self, PlannedCard, PlannedReview};

/// An endpoint no test contacts: the reader never syncs.
const ENDPOINT: &str = "http://127.0.0.1:9/";

/// The bytes a golden's lowercase hex names.
fn unhex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or_else(|_| panic!("hex {hex}")))
        .collect()
}

/// A byte run as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

#[test]
fn the_wire_walk_matches_the_predecessors_golden() {
    let mut examined = 0;
    let mut errors = 0;
    golden::each_case("wire_walk", |case| {
        let blob = unhex(case.input["hex"].as_str().expect("the blob's hex"));
        let walked = wire::walk(&blob);
        examined += 1;
        if let Some(message) = case.output.get("error") {
            errors += 1;
            let error = walked.expect_err("the predecessor refused this blob");
            assert_eq!(
                &error.to_string(),
                message.as_str().unwrap(),
                "{}",
                case.input
            );
            return;
        }
        let expected = case.output["fields"].as_array().expect("the fields");
        let fields = walked.unwrap_or_else(|error| panic!("{error}: {}", case.input));
        let seen: Vec<(u128, u8, String)> = fields
            .iter()
            .map(|field| {
                let text = match &field.value {
                    WireValue::Varint(value) => value.to_string(),
                    WireValue::Fixed64(bytes)
                    | WireValue::Fixed32(bytes)
                    | WireValue::Length(bytes) => hex(bytes),
                };
                (field.number, field.value.wire_type(), text)
            })
            .collect();
        let wanted: Vec<(u128, u8, String)> = expected
            .iter()
            .map(|field| {
                let value = &field[2];
                let text = value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned);
                (
                    u128::from(field[0].as_u64().unwrap()),
                    u8::try_from(field[1].as_u64().unwrap()).unwrap(),
                    text,
                )
            })
            .collect();
        assert_eq!(seen, wanted, "{}", case.input);
    });
    assert!(examined >= 30, "examined only {examined} golden cases");
    assert!(errors >= 8, "examined only {errors} refused blobs");
}

/// Three cards of one Basic note type in scope, each reviewed, and the deck they are in.
const IN_SCOPE: [PlannedCard; 2] = [
    PlannedCard {
        id: 2001,
        deck: "Law::Evidence",
        filtered: false,
    },
    PlannedCard {
        id: 2002,
        deck: "Law::Evidence",
        filtered: false,
    },
];

fn review(id: i64, card: i64) -> PlannedReview {
    PlannedReview {
        id,
        card,
        kind: 1,
        ease: 3,
    }
}

#[tokio::test]
async fn a_failed_structure_read_is_named() {
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    synthetic::build_planned(
        &copy,
        &["Law::Evidence"],
        &IN_SCOPE,
        &[review(1_700_000_000_001, 2001)],
    );
    let reader = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0));
    let whole = reader.read_structure().await.expect("the copy reads");
    assert!(whole.failed_reads.is_empty(), "{:?}", whole.failed_reads);
    assert!(!whole.templates.is_empty(), "no template rows were read");
    assert!(!whole.fields.is_empty(), "no declared field rows were read");

    synthetic::run_sql(&copy, "drop table fields");
    let reader = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0));
    let broken = reader.read_structure().await.expect("the copy still opens");
    assert_eq!(broken.failed_reads, vec![READ_DECLARED_FIELDS.to_owned()]);
    assert!(broken.fields.is_empty());
    assert_eq!(
        broken.templates, whole.templates,
        "the reads that worked kept their rows"
    );
    assert_eq!(broken.reviewed_notes, whole.reviewed_notes);
}

#[tokio::test]
async fn the_presence_read_reduces_each_batch_of_400() {
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    let cards: Vec<PlannedCard> = (0..1000)
        .map(|at| PlannedCard {
            id: 10_000 + at,
            deck: "Law::Evidence",
            filtered: false,
        })
        .collect();
    let reviews: Vec<PlannedReview> = cards
        .iter()
        .map(|card| review(1_700_000_000_000 + card.id, card.id))
        .collect();
    synthetic::build_planned(&copy, &["Law::Evidence"], &cards, &reviews);
    // Every second note has a blank back, and every fifth a back of spaces and an ideographic space.
    synthetic::run_sql(
        &copy,
        "update notes set flds = substr(flds, 1, instr(flds, char(31))) where id % 2 = 0; \
         update notes set flds = substr(flds, 1, instr(flds, char(31))) || '  \u{3000} ' \
         where id % 5 = 1;",
    );
    let reader = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0));
    let structure = reader.read_structure().await.expect("the copy reads");
    assert_eq!(NOTES_BATCH_SIZE, 400);
    assert_eq!(structure.presence_batches, vec![400, 400, 200]);
    let basic = synthetic::notetype_id(&copy, "Basic");
    let with_back = (0..1000)
        .filter(|at| (10_000 + at) % 2 == 1 && (10_000 + at) % 5 != 1)
        .count();
    assert_eq!(structure.presence.get(&(basic, 0)), Some(&1000));
    assert_eq!(
        structure.presence.get(&(basic, 1)),
        Some(&i64::try_from(with_back).unwrap())
    );
    assert_eq!(structure.reviewed_notes.len(), 1000);
}

#[tokio::test]
async fn structure_reads_keep_the_scope() {
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    let cards = [
        PlannedCard {
            id: 3001,
            deck: "Law::Evidence",
            filtered: false,
        },
        PlannedCard {
            id: 3002,
            deck: "Maths",
            filtered: false,
        },
    ];
    synthetic::build_planned(
        &copy,
        &["Law::Evidence", "Maths"],
        &cards,
        &[
            review(1_700_000_000_001, 3001),
            review(1_700_000_000_002, 3002),
        ],
    );
    let cloze = synthetic::notetype_id(&copy, "Cloze");
    synthetic::run_sql(
        &copy,
        &format!("update notes set mid = {cloze} where id = 3002"),
    );
    let reader = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0));
    let structure = reader.read_structure().await.expect("the copy reads");
    let basic = synthetic::notetype_id(&copy, "Basic");
    let read: BTreeSet<i64> = structure.templates.iter().map(|t| t.note_type_id).collect();
    assert_eq!(
        read,
        BTreeSet::from([basic]),
        "a note type used only outside the scope was read"
    );
    let declared: BTreeSet<i64> = structure.fields.iter().map(|f| f.note_type_id).collect();
    assert_eq!(declared, BTreeSet::from([basic]));
    assert_eq!(structure.reviewed_notes, BTreeSet::from([3001]));
    assert!(
        structure
            .presence
            .keys()
            .all(|(note_type, _)| *note_type == basic)
    );
    assert!(!structure.presence.is_empty(), "no presence was read");
}

#[test]
fn names_are_made_safe_as_the_predecessor_does() {
    let mut examined = 0;
    golden::each_case("safe_name", |case| {
        let name = case.input["name"].as_str().expect("the name");
        assert_eq!(
            safe_name(name),
            case.output.as_str().unwrap(),
            "{}",
            case.input
        );
        examined += 1;
    });
    assert!(examined >= 20, "examined only {examined} golden cases");
}
