//! The records step (SPEC-073 A14 to A16; R9 to R12; ADR-303): its plan equals the predecessor's
//! golden over a 370-day window, the first detection seeds the records silently, a record is
//! celebrated once per kind and day, a record still owed when a later day beats it is offered
//! before that day's write, and one replaced before it was offered is named in a log line, while a
//! best that climbs within its own day is not. An offer the router did not answer leaves the record
//! owed; an answered one is raised on the offers' day, marked from the clock, in the kinds' order.
//! Every rollup, record and review is synthetic.

// An integration test is test code: its helpers panic on a malformed golden or a failed fixture.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::recompute::records::{
    PlannedRecord, RECORD_EVENT, RECORDS_WINDOW, RecordPlan, RecordsStep, offer_records, plan,
    record_key,
};
use deck_streak_coordination::recompute::{
    AwardOffers, Evaluation, Fold, FoldInput, Offers, Phase,
};
use deck_streak_ingest::reader::CollectionData;
use deck_streak_kernel::{Db, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::DedupeKey;
use deck_streak_progression::records::{Detected, RecordKind};
use serde_json::Value;

use support::{
    D0, DIGEST, Recorder, RecordingBot, RollupSeed, answer, at, card, collection, day, records,
    router, run_step, scratch, seed_records, seed_rollups,
};

/// A golden kind.
fn kind(value: &Value) -> RecordKind {
    let text = value
        .as_str()
        .unwrap_or_else(|| panic!("a kind, not {value}"));
    RecordKind::parse(text).unwrap_or_else(|| panic!("a record kind, not {text}"))
}

/// A golden number.
fn number(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("a number, not {value}"))
}

/// A golden array.
fn items(value: &Value) -> &Vec<Value> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("an array, not {value}"))
}

/// The plan the golden's `output` describes on `day`.
fn expected_plan(output: &Value) -> RecordPlan {
    RecordPlan {
        written: items(&output["written"])
            .iter()
            .map(|row| PlannedRecord {
                kind: kind(&row[0]),
                value: number(&row[1]),
                study_day: StudyDay::from_epoch_day(number(&row[2])),
                previous: number(&row[3]),
            })
            .collect(),
        celebrated: items(&output["celebrated"])
            .iter()
            .map(|pair| {
                assert_eq!(
                    pair[0].as_str(),
                    Some(RECORD_EVENT),
                    "a record celebration: {pair}"
                );
                pair[1]
                    .as_str()
                    .unwrap_or_else(|| panic!("a key, not {pair}"))
                    .to_owned()
            })
            .collect(),
        seeded: output["seeded"]
            .as_bool()
            .unwrap_or_else(|| panic!("a flag, not {output}")),
    }
}

/// One rollup of the window: `score`, `reviews` and `seconds` on `day_number`.
const fn totals(day_number: i64, score: i64, reviews: i64, seconds: f64) -> RollupSeed {
    RollupSeed {
        day: day_number,
        reviews,
        seconds,
        score,
        score_at_close: None,
        card_state: None,
    }
}

/// Low records already celebrated on a day long before the window: each kind is stored.
async fn seed_low_records(db: &Db) {
    seed_records(
        db,
        &[
            ("best_score", 20, D0 - 400, 0),
            ("most_reviews", 10, D0 - 400, 0),
            ("most_minutes", 1, D0 - 400, 0),
        ],
    )
    .await;
}

/// The window's records step evaluated on `day_number` as the current day, at `now`.
async fn current(db: &Db, day_number: i64, now: i64) {
    let empty = collection(Vec::new(), Vec::new());
    run_step(
        db,
        &RecordsStep,
        &empty,
        0,
        (day_number, Evaluation::Current),
        now,
    )
    .await;
}

/// Offers the owed records to `recorder` on `day_number`.
async fn offer(db: &Db, recorder: &Recorder, day_number: i64) {
    let now = UtcMillis::from_epoch_millis(at(day_number, 13));
    offer_records(recorder, db, now, day(day_number))
        .await
        .expect("the offers run");
}

/// Moves `day_number`'s stored score to `score`, as a later recompute of the day does.
async fn rescore(db: &Db, day_number: i64, score: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query("UPDATE daily_rollup SET score = ?1 WHERE study_day = ?2")
        .bind(score)
        .bind(day_number)
        .execute(&mut *write)
        .await
        .expect("the score moves");
    write.commit().await.expect("the score commits");
}

#[tokio::test]
async fn the_records_step_matches_the_parity_golden() {
    golden::each_case("records_window", |case| {
        let input = &case.input;
        let output = &case.output;
        let label = case.class.as_deref().unwrap_or("a case");
        let stored: BTreeMap<RecordKind, i64> = input["stored"]
            .as_object()
            .unwrap_or_else(|| panic!("{label}: stored bests"))
            .iter()
            .map(|(name, value)| (kind(&Value::from(name.as_str())), number(value)))
            .collect();
        let new: Vec<Detected> = items(&input["new"])
            .iter()
            .map(|pair| Detected {
                kind: kind(&pair[0]),
                value: number(&pair[1]),
            })
            .collect();
        let empty = input["empty"]
            .as_bool()
            .unwrap_or_else(|| panic!("{label}: empty"));
        let planned = plan(&stored, empty, &new, day(D0));
        assert_eq!(planned, expected_plan(output), "{label}: the plan");
        assert_eq!(
            RECORDS_WINDOW,
            number(&output["limit"]),
            "{label}: the window"
        );
        for key in &planned.celebrated {
            assert!(
                DedupeKey::new(key).is_ok(),
                "{label}: {key} is a router key"
            );
        }
        for record in &planned.written {
            if !planned.seeded {
                let key = record_key(record.kind, record.study_day);
                assert!(planned.celebrated.contains(&key), "{label}: {key} is owed");
            }
        }
    });

    // The window holds the 370 most recent rollups and not the 371st (R10).
    let scratch = scratch().await;
    let mut window: Vec<RollupSeed> = (D0 - 370..=D0).map(|n| totals(n, 10, 5, 60.0)).collect();
    window[0] = totals(D0 - 370, 100, 5, 60.0);
    window[1] = totals(D0 - 369, 10, 999, 60.0);
    seed_rollups(&scratch.db, &window).await;
    seed_low_records(&scratch.db).await;
    current(&scratch.db, D0, at(D0, 13)).await;
    assert_eq!(
        records(&scratch.db).await,
        [
            ("best_score".to_owned(), 20, D0 - 400, 0, true),
            ("most_minutes".to_owned(), 1, D0 - 400, 0, true),
            ("most_reviews".to_owned(), 999, D0, 10, false),
        ],
        "the 370th most recent rollup counts, the 371st does not"
    );
}

#[tokio::test]
async fn the_first_detection_seeds_records_silently() {
    let scratch = scratch().await;
    seed_rollups(
        &scratch.db,
        &[
            totals(D0 - 2, 70, 30, 600.0),
            totals(D0 - 1, 80, 20, 1_200.0),
            totals(D0, 60, 10, 300.0),
        ],
    )
    .await;
    let seeded = [
        ("best_score".to_owned(), 80, D0, 80, true),
        ("most_minutes".to_owned(), 20, D0, 20, true),
        ("most_reviews".to_owned(), 30, D0, 30, true),
    ];
    for pass in 1..=2 {
        current(&scratch.db, D0, at(D0, 13)).await;
        assert_eq!(
            records(&scratch.db).await,
            seeded,
            "pass {pass}: seeded at their own values"
        );
        let recorder = Recorder::default();
        offer(&scratch.db, &recorder, D0).await;
        assert!(
            recorder.keys().is_empty(),
            "pass {pass}: a seed raises nothing: {:?}",
            recorder.keys()
        );
    }
}

/// The fold of the records step alone.
fn records_fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(Phase::Awards, Box::new(RecordsStep))
        .expect("the records step is phase 7's");
    fold
}

/// One recompute of `data` at `now`, synced in `synced_in`, with `offers` between its writes.
async fn recompute(
    db: &Db,
    data: &CollectionData,
    synced_in: Option<i64>,
    offers: Option<&dyn Offers>,
    now: i64,
) {
    records_fold()
        .run(
            db,
            &FoldInput {
                data,
                rule: StudyDayRule::default(),
                now: UtcMillis::from_epoch_millis(now),
                synced_in: synced_in.map(day),
                courses_digest: Some(DIGEST),
                base_reviews: 0,
                offers,
            },
        )
        .await
        .expect("the fold runs");
}

/// A best score owed on `D0` and beaten on the next day is offered before the next day's write,
/// and each day's key is sent once.
async fn a_later_beat_offers_the_owed_record_first() {
    let scratch = scratch().await;
    seed_low_records(&scratch.db).await;
    seed_rollups(
        &scratch.db,
        &[totals(D0, 50, 0, 0.0), totals(D0 + 1, 60, 0, 0.0)],
    )
    .await;
    let data = collection(
        vec![
            answer(at(D0, 10), 1, 3, 5, 9_000),
            answer(at(D0 + 1, 10), 2, 3, 5, 9_000),
        ],
        vec![card(1, 1), card(2, 1)],
    );
    recompute(&scratch.db, &data, None, None, at(D0, 13)).await;
    assert_eq!(
        records(&scratch.db).await[0],
        ("best_score".to_owned(), 50, D0, 20, false),
        "written, owed"
    );
    let bot = Arc::new(RecordingBot::default());
    let offers = AwardOffers::new(router(&scratch.db, D0 + 1, &bot));
    recompute(
        &scratch.db,
        &data,
        Some(D0 + 1),
        Some(&offers),
        at(D0 + 1, 13),
    )
    .await;
    assert_eq!(
        bot.sent().len(),
        2,
        "the owed record, then the new one: {:?}",
        bot.sent()
    );
    assert_eq!(
        records(&scratch.db).await[0],
        ("best_score".to_owned(), 60, D0 + 1, 50, true),
        "the new record is marked once answered"
    );
    recompute(
        &scratch.db,
        &data,
        Some(D0 + 1),
        Some(&offers),
        at(D0 + 1, 14),
    )
    .await;
    assert_eq!(
        bot.sent().len(),
        2,
        "a replay sends nothing: {:?}",
        bot.sent()
    );
}

#[tokio::test]
async fn a_record_is_celebrated_once_per_kind_and_day() {
    let scratch = scratch().await;
    seed_low_records(&scratch.db).await;
    seed_rollups(&scratch.db, &[totals(D0, 50, 0, 0.0)]).await;
    let recorder = Recorder::default();
    current(&scratch.db, D0, at(D0, 13)).await;
    offer(&scratch.db, &recorder, D0).await;
    assert_eq!(
        recorder.keys(),
        ["pr:best_score:20000"],
        "the new best is offered"
    );
    // The same day beats its own record: the day's key was celebrated, so nothing is owed.
    rescore(&scratch.db, D0, 60).await;
    current(&scratch.db, D0, at(D0, 14)).await;
    offer(&scratch.db, &recorder, D0).await;
    assert_eq!(
        recorder.keys(),
        ["pr:best_score:20000"],
        "once per kind and day"
    );
    assert_eq!(
        records(&scratch.db).await[0],
        ("best_score".to_owned(), 60, D0, 50, true),
        "the day's record moves and keeps its mark"
    );
    a_later_beat_offers_the_owed_record_first().await;
}

#[tokio::test]
async fn a_later_beat_offers_the_unmarked_record_first() {
    a_later_beat_offers_the_owed_record_first().await;
}

/// The WARN events this crate logs on the test's thread, each as its fields.
#[derive(Clone, Default)]
struct Warnings(Arc<Mutex<Vec<String>>>);

impl Warnings {
    fn logged(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// One event's fields, as `name=value` pairs.
#[derive(Default)]
struct Fields(Vec<String>);

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push(format!("{}={value}", field.name()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.0.push(format!("{}={value:?}", field.name()));
    }
}

impl tracing::Subscriber for Warnings {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let metadata = event.metadata();
        if *metadata.level() == tracing::Level::WARN
            && metadata.target().starts_with("deck_streak_coordination")
        {
            let mut fields = Fields::default();
            event.record(&mut fields);
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(fields.0.join(" "));
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[tokio::test]
async fn a_record_replaced_before_it_was_offered_is_named() {
    let warnings = Warnings::default();
    let _logging = log_capture::hold_capture(warnings.clone());
    let scratch = scratch().await;
    seed_low_records(&scratch.db).await;
    seed_rollups(
        &scratch.db,
        &[totals(D0, 50, 0, 0.0), totals(D0 + 1, 60, 0, 0.0)],
    )
    .await;
    current(&scratch.db, D0, at(D0, 13)).await;
    assert!(
        warnings.logged().is_empty(),
        "nothing replaced yet: {:?}",
        warnings.logged()
    );
    current(&scratch.db, D0 + 1, at(D0 + 1, 13)).await;
    let logged = warnings.logged();
    assert_eq!(
        logged.len(),
        1,
        "one record was replaced unsent: {logged:?}"
    );
    let line = &logged[0];
    let named_day = format!("day={}", day(D0));
    for part in [
        "celebration not sent",
        "kind=best_score",
        named_day.as_str(),
    ] {
        assert!(line.contains(part), "{line} names {part}");
    }
    assert_eq!(
        records(&scratch.db).await[0],
        ("best_score".to_owned(), 60, D0 + 1, 50, false),
        "the new record is written with its mark unset"
    );
}

#[tokio::test]
async fn a_best_that_climbs_all_day_before_its_offer_is_not_named() {
    let warnings = Warnings::default();
    let _logging = log_capture::hold_capture(warnings.clone());
    let scratch = scratch().await;
    seed_low_records(&scratch.db).await;
    seed_rollups(&scratch.db, &[totals(D0, 50, 0, 0.0)]).await;
    current(&scratch.db, D0, at(D0, 13)).await;
    // The day beats its own unoffered record: its row is the same day's, so nothing is lost.
    rescore(&scratch.db, D0, 60).await;
    current(&scratch.db, D0, at(D0, 14)).await;
    assert!(
        warnings.logged().is_empty(),
        "a row of the same day is not named: {:?}",
        warnings.logged()
    );
    let recorder = Recorder::default();
    offer(&scratch.db, &recorder, D0).await;
    assert_eq!(
        recorder.keys(),
        [record_key(RecordKind::BestScore, day(D0))],
        "the day's best is celebrated once"
    );
    assert_eq!(
        records(&scratch.db).await[0],
        ("best_score".to_owned(), 60, D0, 50, true),
        "the day's record holds its last value, marked"
    );
}

#[tokio::test]
async fn a_router_that_did_not_answer_leaves_the_record_owed() {
    let scratch = scratch().await;
    seed_low_records(&scratch.db).await;
    seed_rollups(&scratch.db, &[totals(D0, 50, 0, 0.0)]).await;
    current(&scratch.db, D0, at(D0, 13)).await;
    let owed = record_key(RecordKind::BestScore, day(D0));
    let silent = Recorder::silent();
    offer(&scratch.db, &silent, D0).await;
    assert_eq!(silent.keys(), [owed.as_str()], "the owed record is offered");
    assert!(!records(&scratch.db).await[0].4, "no answer leaves it owed");
    // The next day's offers raise it on that day, and mark it from the clock at the answer.
    let answered = Recorder::default();
    offer(&scratch.db, &answered, D0 + 1).await;
    let handed = answered.handed();
    assert_eq!(answered.keys(), [owed], "offered again");
    assert_eq!(
        handed[0].study_day,
        day(D0 + 1),
        "raised on the offers' day"
    );
    let mark: Option<i64> =
        sqlx::query_scalar("SELECT celebrated_at FROM records WHERE kind = 'best_score'")
            .fetch_one(scratch.db.reader())
            .await
            .expect("the mark reads");
    assert_eq!(
        mark,
        Some(at(D0 + 1, 13)),
        "marked from the clock at the answer"
    );
}

#[tokio::test]
async fn the_owed_records_are_offered_in_the_kinds_order() {
    let scratch = scratch().await;
    seed_low_records(&scratch.db).await;
    seed_rollups(&scratch.db, &[totals(D0, 50, 30, 600.0)]).await;
    current(&scratch.db, D0, at(D0, 13)).await;
    let recorder = Recorder::default();
    offer(&scratch.db, &recorder, D0).await;
    let expected: Vec<String> = RecordKind::ALL
        .into_iter()
        .map(|kind| record_key(kind, day(D0)))
        .collect();
    assert_eq!(
        recorder.keys(),
        expected,
        "best score, most reviews, most minutes"
    );
}
