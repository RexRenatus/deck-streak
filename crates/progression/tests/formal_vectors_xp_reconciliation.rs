//! The XP floor's vectors (SPEC-389 R12, A1 to A4). `formal/lean/Formal/XpReconciliation.lean`
//! ports the settle rule and models the day's protocol, and its writer prints
//! `formal/vectors/xp-reconciliation.jsonl`. The shipping `settle` answers every settle vector on a
//! scratch ledger; every trace's settles, replayed through it, hold the rows the vector says; and
//! at every settle point the day's rows read back sum to the vector's confirmed XP, which is at
//! least its shown XP.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::settle::{
    SettleCause, SettleRequest, SettledRow, settle, settled_of_day,
};
use serde_json::{Value, json};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/xp-reconciliation.jsonl");

const AT: i64 = 1_700_000_000_000;

/// The trace letters, declared here and never read from the file.
const ALPHABET: &str = "GHUSRN";
/// The longest trace.
const MAX_LENGTH: usize = 4;
/// The amounts a held row or a request holds.
const AMOUNTS: [u32; 4] = [0, 30, 50, 4_294_967_295];
/// The causes, as the vectors spell them.
const CAUSES: [&str; 2] = ["recompute", "owners_correction"];

/// The study day of settle vector `i` is this one plus `i`.
const SETTLE_DAY: i64 = 20_000;
/// The study day of trace vector `j` is this one plus `j`.
const TRACE_DAY: i64 = 30_000;

/// Prints how many `what` were examined, and refuses none.
fn examined(count: u64, what: &str) -> u64 {
    println!("examined {count} {what}");
    assert!(
        count > 0,
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    count
}

async fn database() -> (tempfile::TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

/// The header and every vector line.
fn vectors() -> (Value, Vec<Value>) {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "XpReconciliation", "{header}");
    assert_eq!(
        header["covers"], "crates/progression/src/settle.rs",
        "{header}"
    );
    let vectors = lines
        .map(|line| serde_json::from_str(line).expect("a JSON vector"))
        .collect();
    (header, vectors)
}

/// The vectors of one kind, in file order.
fn of_kind(vectors: &[Value], kind: &str) -> Vec<Value> {
    vectors
        .iter()
        .filter(|vector| vector["kind"] == kind)
        .cloned()
        .collect()
}

/// A vector field as an unsigned integer.
fn uint(value: &Value) -> u64 {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("an unsigned integer, not {value}"))
}

/// A vector field as a `u32` amount.
fn amount(value: &Value) -> u32 {
    u32::try_from(uint(value)).unwrap_or_else(|_| panic!("a u32 amount, not {value}"))
}

/// A vector field as a flag.
fn flag(value: &Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| panic!("a boolean, not {value}"))
}

/// A vector field's array.
fn array(value: &Value) -> &Vec<Value> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("an array, not {value}"))
}

/// A row pair `[amount, closed]`.
fn row(value: &Value) -> (u32, bool) {
    let pair = array(value);
    (amount(&pair[0]), flag(&pair[1]))
}

/// A cause, as the vectors spell it.
fn cause(value: &Value) -> SettleCause {
    match value.as_str() {
        Some("recompute") => SettleCause::Recompute,
        Some("owners_correction") => SettleCause::OwnersCorrection,
        _ => panic!("a cause, not {value}"),
    }
}

/// The track a trace's source settles on.
fn track_of(source: &str) -> Track {
    match source {
        "reviews" | "score90" => Track::Language,
        "reviews_law" => Track::Law,
        _ => panic!("a trace source, not {source}"),
    }
}

/// The study day `base` plus `offset`.
fn day(base: i64, offset: usize) -> StudyDay {
    StudyDay::from_epoch_day(base + i64::try_from(offset).expect("a small offset"))
}

/// Settles once in its own transaction and answers the amount the row holds.
async fn settled(
    db: &Db,
    study_day: StudyDay,
    source: &str,
    (amount, closed): (u32, bool),
    cause: SettleCause,
) -> u32 {
    let request = SettleRequest {
        study_day,
        source,
        track: track_of(source),
        amount,
        closed,
    };
    let mut write = db.write().await.expect("a write");
    let held = settle(
        &mut write,
        &request,
        cause,
        UtcMillis::from_epoch_millis(AT),
    )
    .await
    .expect("the settlement runs");
    write.commit().await.expect("commit");
    held
}

/// The study day's rows, as `settled_of_day` reads them back.
async fn day_rows(db: &Db, study_day: StudyDay) -> Vec<SettledRow> {
    let mut read = db.reader().acquire().await.expect("a reader");
    settled_of_day(&mut read, study_day)
        .await
        .expect("the day's rows are read")
}

/// Whether the row of `source` on `study_day` is closed, as `settled_of_day` reads it back.
async fn closed_of(db: &Db, study_day: StudyDay, source: &str) -> bool {
    let track = track_of(source);
    day_rows(db, study_day)
        .await
        .iter()
        .find(|row| row.source == source && row.track == track.as_str())
        .unwrap_or_else(|| panic!("a {source} row"))
        .closed
}

/// Replays a trace vector's settles from index `next` through step `step`, through the shipping
/// `settle` with the Recompute cause, asserting each amount answered and each closed flag read
/// back against the vector's; answers the index of the first settle not replayed.
async fn replay_through(
    db: &Db,
    study_day: StudyDay,
    vector: &Value,
    mut next: usize,
    step: u64,
) -> usize {
    let settles = array(&vector["settles"]);
    while let Some(entry) = settles.get(next).filter(|entry| uint(&entry[0]) <= step) {
        let source = entry[1].as_str().expect("a source");
        let answered = settled(
            db,
            study_day,
            source,
            (amount(&entry[2]), flag(&entry[3])),
            SettleCause::Recompute,
        )
        .await;
        let closed = closed_of(db, study_day, source).await;
        assert_eq!(
            (answered, closed),
            (amount(&entry[4]), flag(&entry[5])),
            "{vector}: settle {entry}"
        );
        next += 1;
    }
    next
}

#[tokio::test]
async fn the_settle_rule_answers_every_lean_vector() {
    let (_directory, db) = database().await;
    let (_header, vectors) = vectors();
    let mut count = 0_u64;
    for (i, vector) in of_kind(&vectors, "settle").iter().enumerate() {
        let study_day = day(SETTLE_DAY, i);
        if !vector["held"].is_null() {
            settled(
                &db,
                study_day,
                "reviews",
                row(&vector["held"]),
                SettleCause::Recompute,
            )
            .await;
        }
        let answered = settled(
            &db,
            study_day,
            "reviews",
            row(&vector["request"]),
            cause(&vector["cause"]),
        )
        .await;
        let closed = closed_of(&db, study_day, "reviews").await;
        assert_eq!((answered, closed), row(&vector["after"]), "{vector}");
        count += 1;
    }
    examined(count, "settle vector(s)");
}

#[tokio::test]
async fn every_trace_confirms_at_least_the_xp_shown() {
    let (_directory, db) = database().await;
    let (_header, vectors) = vectors();
    let (mut count, mut points_seen) = (0_u64, 0_u64);
    for (j, vector) in of_kind(&vectors, "trace").iter().enumerate() {
        let study_day = day(TRACE_DAY, j);
        let settles = array(&vector["settles"]);
        let mut next = 0;
        for point in array(&vector["points"]) {
            next = replay_through(&db, study_day, vector, next, uint(&point[0])).await;
            let confirmed: u64 = day_rows(&db, study_day)
                .await
                .iter()
                .map(|row| u64::from(row.amount))
                .sum();
            assert_eq!(confirmed, uint(&point[2]), "{vector}: point {point}");
            assert!(
                uint(&point[1]) <= confirmed,
                "{vector}: point {point} shows more than the day confirms"
            );
            points_seen += 1;
        }
        next = replay_through(&db, study_day, vector, next, u64::MAX).await;
        assert_eq!(next, settles.len(), "{vector}: every settle replayed");
        count += 1;
    }
    examined(points_seen, "settle point(s)");
    examined(count, "trace vector(s)");
}

#[tokio::test]
async fn the_xp_reconciliation_counterexamples_answer_as_proved() {
    let (_directory, db) = database().await;
    let mut count = 0_u64;
    // `a_recompute_that_replaces_a_closed_row_violates`: a closed 50 under a Recompute 30, open,
    // holds 50, closed.
    let closed_day = StudyDay::from_epoch_day(40_000);
    settled(
        &db,
        closed_day,
        "reviews",
        (50, true),
        SettleCause::Recompute,
    )
    .await;
    let answered = settled(
        &db,
        closed_day,
        "reviews",
        (30, false),
        SettleCause::Recompute,
    )
    .await;
    assert_eq!(
        (answered, closed_of(&db, closed_day, "reviews").await),
        (50, true)
    );
    count += 1;
    // `a_close_that_reads_only_the_held_flag_violates`: an open 50 under a Recompute 30, closed,
    // holds 50, closed.
    let open_day = StudyDay::from_epoch_day(40_001);
    settled(
        &db,
        open_day,
        "reviews",
        (50, false),
        SettleCause::Recompute,
    )
    .await;
    let answered = settled(&db, open_day, "reviews", (30, true), SettleCause::Recompute).await;
    assert_eq!(
        (answered, closed_of(&db, open_day, "reviews").await),
        (50, true)
    );
    count += 1;
    examined(count, "counterexample(s)");
}

#[test]
fn the_vectors_cover_every_case_of_the_axes() {
    let (header, vectors) = vectors();
    assert_eq!(header["alphabet"], ALPHABET, "{header}");
    assert_eq!(
        header["max_length"].as_u64(),
        u64::try_from(MAX_LENGTH).ok(),
        "{header}"
    );
    assert_eq!(header["amounts"], json!(AMOUNTS), "{header}");

    let rows: Vec<Value> = AMOUNTS
        .iter()
        .flat_map(|amount| [false, true].map(|closed| json!([amount, closed])))
        .collect();
    let helds: Vec<Value> = std::iter::once(Value::Null)
        .chain(rows.iter().cloned())
        .collect();
    let mut cases: BTreeMap<String, u64> = BTreeMap::new();
    for held in &helds {
        for request in &rows {
            for cause in CAUSES {
                cases.insert(format!("{held} {request} {cause}"), 0);
            }
        }
    }
    let letters: Vec<char> = ALPHABET.chars().collect();
    let mut words: BTreeMap<String, u64> = BTreeMap::new();
    let mut shorter = vec![String::new()];
    for _ in 0..MAX_LENGTH {
        shorter = shorter
            .iter()
            .flat_map(|word| letters.iter().map(move |letter| format!("{word}{letter}")))
            .collect();
        for word in &shorter {
            words.insert(word.clone(), 0);
        }
    }
    assert_eq!(cases.len(), 9 * 8 * 2, "the settle axes' product");
    assert_eq!(
        words.len(),
        6 + 36 + 216 + 1296,
        "every word of 1 to 4 letters"
    );

    let mut count = 0_u64;
    for vector in &vectors {
        let seen = match vector["kind"].as_str() {
            Some("settle") => cases.get_mut(&format!(
                "{} {} {}",
                vector["held"],
                vector["request"],
                vector["cause"].as_str().expect("a cause")
            )),
            Some("trace") => words.get_mut(vector["trace"].as_str().expect("a word")),
            _ => None,
        }
        .unwrap_or_else(|| panic!("a case of the axes, not {vector}"));
        *seen += 1;
        count += 1;
    }
    for (case, seen) in cases.iter().chain(words.iter()) {
        assert_eq!(*seen, 1, "{case} occurs once");
    }
    examined(count, "vector(s)");
    assert_eq!(header["vectors"].as_u64(), Some(count), "{header}");
}
