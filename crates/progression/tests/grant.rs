//! The grant port writes each grant once (SPEC-040 A1 to A3, A7, A8; R2 to R6; ADR-040): a
//! per-day key once per study day, a once key once ever, two concurrent requests for one key one
//! row, a negative amount refused by the table itself, and a malformed source refused before any
//! write, by an error that names the rule. Every source and amount here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints what it
// examined on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::grant::{
    GrantAnswer, GrantPort, GrantRequest, GrantScope, GrantSource, SOURCE_GRAMMAR,
};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::xp::XpAmount;
use tempfile::TempDir;
use tokio::sync::Barrier;

/// A ledger row as the test reads it back: study day, source, track, amount and scope.
type Row = (i64, String, String, i64, String);

/// The instant the grants are written at.
const AT: UtcMillis = UtcMillis::from_epoch_millis(1_700_000_000_000);

/// A migrated temporary database, and the ledger over it.
async fn ledger() -> (TempDir, Db, SqliteXpLedger) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ledger = SqliteXpLedger::new(db.clone());
    (directory, db, ledger)
}

/// A request of `amount` for `source` on `track`, for study day `day`, in `scope`.
fn request(
    day: i64,
    source: &str,
    track: Track,
    amount: XpAmount,
    scope: GrantScope,
) -> GrantRequest {
    GrantRequest {
        study_day: StudyDay::from_epoch_day(day),
        source: GrantSource::new(source).expect("a source of the grammar"),
        track,
        amount,
        scope,
    }
}

/// The port's answer, with a refusal as its text so answers compare.
async fn answer(ledger: &SqliteXpLedger, request: &GrantRequest) -> Result<GrantAnswer, String> {
    ledger
        .grant(request, AT)
        .await
        .map_err(|refusal| refusal.to_string())
}

/// Every row of the ledger, in the order written.
async fn rows(db: &Db) -> Vec<Row> {
    sqlx::query_as("SELECT study_day, source, track, amount, scope FROM xp_ledger ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("the ledger's rows")
}

/// A row as the ledger stores it.
fn row(day: i64, source: &str, track: &str, amount: i64, scope: &str) -> Row {
    (
        day,
        source.to_owned(),
        track.to_owned(),
        amount,
        scope.to_owned(),
    )
}

#[tokio::test]
async fn a_grant_requested_twice_writes_one_row() {
    let (_directory, db, ledger) = ledger().await;
    let first = request(
        20_000,
        "reading:read:r1",
        Track::Law,
        XpAmount::new(40),
        GrantScope::PerDay,
    );
    // The second request carries another amount: its answer is what the ledger holds, never what
    // the replay asked for.
    let second = GrantRequest {
        amount: XpAmount::new(60),
        ..first.clone()
    };
    let answers = [
        answer(&ledger, &first).await,
        answer(&ledger, &second).await,
    ];
    assert_eq!(
        answers,
        [
            Ok(GrantAnswer::Granted(XpAmount::new(40))),
            Ok(GrantAnswer::AlreadyGranted(XpAmount::new(40))),
        ]
    );
    assert_eq!(
        rows(&db).await,
        [row(20_000, "reading:read:r1", "law", 40, "per-day")]
    );

    // The key is the study day, the source and the track: the same source on the other track, or
    // on the next study day, is a grant of its own.
    let other_track = GrantRequest {
        track: Track::Language,
        ..first.clone()
    };
    let next_day = GrantRequest {
        study_day: StudyDay::from_epoch_day(20_001),
        ..first.clone()
    };
    assert_eq!(
        [
            answer(&ledger, &other_track).await,
            answer(&ledger, &next_day).await,
        ],
        [
            Ok(GrantAnswer::Granted(XpAmount::new(40))),
            Ok(GrantAnswer::Granted(XpAmount::new(40))),
        ]
    );
    assert_eq!(
        rows(&db).await,
        [
            row(20_000, "reading:read:r1", "law", 40, "per-day"),
            row(20_000, "reading:read:r1", "language", 40, "per-day"),
            row(20_001, "reading:read:r1", "law", 40, "per-day"),
        ]
    );
}

#[tokio::test]
async fn a_once_grant_requested_on_two_study_days_writes_one_row() {
    let (_directory, db, ledger) = ledger().await;
    let first = request(
        20_000,
        "reading:studied:r1",
        Track::Language,
        XpAmount::new(60),
        GrantScope::Once,
    );
    let later = GrantRequest {
        study_day: StudyDay::from_epoch_day(20_002),
        amount: XpAmount::new(75),
        ..first.clone()
    };
    let answers = [answer(&ledger, &first).await, answer(&ledger, &later).await];
    assert_eq!(
        answers,
        [
            Ok(GrantAnswer::Granted(XpAmount::new(60))),
            Ok(GrantAnswer::AlreadyGranted(XpAmount::new(60))),
        ]
    );
    assert_eq!(
        rows(&db).await,
        [row(20_000, "reading:studied:r1", "language", 60, "once")]
    );

    // The once key is the source and the track: the other track pays once of its own.
    let other_track = GrantRequest {
        track: Track::Law,
        ..later.clone()
    };
    assert_eq!(
        answer(&ledger, &other_track).await,
        Ok(GrantAnswer::Granted(XpAmount::new(75)))
    );
    assert_eq!(
        rows(&db).await,
        [
            row(20_000, "reading:studied:r1", "language", 60, "once"),
            row(20_002, "reading:studied:r1", "law", 75, "once"),
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_concurrent_grants_for_one_key_write_one_row() {
    let (_directory, db, ledger) = ledger().await;
    let start = Arc::new(Barrier::new(2));
    // Two tasks ask for one key at once, each with its own amount, released together. Each amount
    // is paired with the number the ledger stores for it, so an answer reads back as a row does.
    let amounts = [(40, XpAmount::new(40)), (60, XpAmount::new(60))];
    let tasks: Vec<_> = amounts
        .into_iter()
        .map(|(_, amount)| {
            let ledger = ledger.clone();
            let start = Arc::clone(&start);
            tokio::spawn(async move {
                let request = request(
                    20_000,
                    "reading:studied:r2",
                    Track::Law,
                    amount,
                    GrantScope::Once,
                );
                start.wait().await;
                answer(&ledger, &request).await
            })
        })
        .collect();
    let stored_as = |amount: XpAmount| {
        amounts
            .iter()
            .find(|(_, requested)| *requested == amount)
            .map_or(0, |(stored, _)| *stored)
    };
    let mut outcomes = Vec::new();
    for task in tasks {
        let outcome = match task.await.expect("the task ran") {
            Ok(GrantAnswer::Granted(amount)) => ("granted", stored_as(amount)),
            Ok(GrantAnswer::AlreadyGranted(amount)) => ("already granted", stored_as(amount)),
            Err(refusal) => panic!("a concurrent grant was refused: {refusal}"),
        };
        outcomes.push(outcome);
    }
    outcomes.sort_unstable();
    let stored = rows(&db).await;
    assert_eq!(stored.len(), 1, "one key, one row: {stored:?}");
    // Whichever task took the write lock first wrote its amount, and the other answered with it.
    let held = stored[0].3;
    assert!(held == 40 || held == 60, "{stored:?}");
    assert_eq!(outcomes, [("already granted", held), ("granted", held)]);
}

#[tokio::test]
async fn the_ledger_refuses_a_negative_amount() {
    let (_directory, db, _ledger) = ledger().await;
    // A raw insert, past the port and its unsigned amount: the table's own check is the last
    // barrier (R6).
    let insert = |source: &'static str, amount: i64| {
        let db = db.clone();
        async move {
            let mut write = db.write().await.expect("a write");
            let inserted = sqlx::query(
                "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
                 VALUES (20000, ?1, 'language', ?2, 'per-day', 1)",
            )
            .bind(source)
            .bind(amount)
            .execute(&mut *write)
            .await
            .map(|done| done.rows_affected());
            write.commit().await.expect("the write commits");
            inserted.map_err(|refusal| refusal.to_string())
        }
    };
    // A grant of nothing is a grant the table admits: the check is on the sign alone.
    assert_eq!(insert("synthetic:zero", 0).await, Ok(1));
    let refused = insert("synthetic:debit", -1).await;
    assert!(
        refused
            .as_ref()
            .is_err_and(|refusal| refusal.contains("CHECK constraint failed")),
        "a negative amount was written: {refused:?}"
    );
    assert_eq!(
        rows(&db).await,
        [row(20_000, "synthetic:zero", "language", 0, "per-day")]
    );
}

#[tokio::test]
async fn a_source_outside_the_token_grammar_is_refused() {
    let (_directory, db, ledger) = ledger().await;
    let longest = "a".repeat(128);
    let too_long = "a".repeat(129);
    // Every text outside the grammar is refused when the source is built, so no request can carry
    // it to a write: a capital, a space, a slash, a letter outside ASCII, a line break, a first
    // character that is punctuation, nothing at all, or one byte too many.
    let outside = [
        "",
        "Reading:read:r1",
        "READING",
        "reading read",
        "reading/read",
        "reading;read",
        "läsning:read",
        "reading:read\n",
        ":reading",
        ".reading",
        "_reading",
        "-reading",
        too_long.as_str(),
    ];
    let refusal = format!("a grant source is an opaque token matching {SOURCE_GRAMMAR}");
    let verdicts: Vec<(&str, Result<(), String>)> = outside
        .iter()
        .map(|text| {
            (
                *text,
                GrantSource::new(text)
                    .map(|_| ())
                    .map_err(|refused| refused.to_string()),
            )
        })
        .collect();
    // One refusal, naming the rule, for every text: it never echoes the value it refused.
    let expected: Vec<(&str, Result<(), String>)> = outside
        .iter()
        .map(|text| (*text, Err(refusal.clone())))
        .collect();
    assert_eq!(verdicts, expected);
    println!("examined {} text(s) outside the grammar", outside.len());
    assert_eq!(rows(&db).await, Vec::<Row>::new(), "a refusal wrote a row");

    // Every token of the grammar is a source, kept byte for byte: a digit or a lowercase letter
    // first, then those and `:`, `.`, `_` and `-`, up to 128 bytes.
    let inside = [
        "0",
        "a",
        "reading:read:1a2b",
        "drill.b-2_c:d",
        longest.as_str(),
    ];
    for token in inside {
        let source = GrantSource::new(token).map_err(|refused| refused.to_string());
        assert_eq!(
            source.as_ref().map(GrantSource::as_str),
            Ok(token),
            "the token {token:?} is a source"
        );
    }
    // And a source of the grammar grants, once.
    let granted = request(
        20_000,
        "drill.b-2_c:d",
        Track::Law,
        XpAmount::new(30),
        GrantScope::Once,
    );
    assert_eq!(
        answer(&ledger, &granted).await,
        Ok(GrantAnswer::Granted(XpAmount::new(30)))
    );
    assert_eq!(
        rows(&db).await,
        [row(20_000, "drill.b-2_c:d", "law", 30, "once")]
    );
}
