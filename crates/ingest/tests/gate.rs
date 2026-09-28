//! The change gate (SPEC-023 A10 to A17): the probe equals the predecessor's, an unchanged cycle
//! skips and records it, and each input on its own runs the recompute.
//!
//! Every gate test but A10 starts from an anchored bench: the scoped collection, the service's
//! database with one successful sync on record, and a first check that ran the recompute and wrote
//! the anchor. It then changes one input, holds every other fixed, and reads the decision, with its
//! reason. Every clock is a `ManualClock`.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::sync::Arc;
use std::time::Duration;

use deck_streak_ingest::gate::{
    Anchor, AnchorState, CARD_FIELD_WEIGHTS, CARD_FINGERPRINT_MODULUS, ChangeGate, Checked,
    CycleFacts, Deadline, Decision, GateInputs, Probe, RunReason, decide, probe,
};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::sync_runs::{
    ReasonCode, RunHistory, RunStatus, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger,
};
use deck_streak_kernel::{Clock, Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use serde_json::Value;
use support::Fixture;
use support::synthetic::{self, PlannedReview};

/// An endpoint no test contacts: the gate never syncs.
const ENDPOINT: &str = "http://127.0.0.1:9/";
const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// A study day's 05:00 in UTC, an hour past its 04:00 rollover (the default rule).
const IN_A_STUDY_DAY: i64 = 20_000 * DAY_MS + 5 * HOUR_MS;
/// A review newer than every review of the scoped collection.
const LATER_REVIEW: i64 = 1_800_000_000_000;

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

#[test]
fn the_change_probe_matches_the_predecessors_golden() {
    golden::each_case("ingest.constants", |case| {
        match case.input["name"].as_str() {
            Some("anki_reader._CARD_FIELD_WEIGHTS") => {
                let weights: Vec<i64> = case
                    .output
                    .as_array()
                    .unwrap_or_else(|| panic!("the weights: {}", case.output))
                    .iter()
                    .map(integer)
                    .collect();
                assert_eq!(CARD_FIELD_WEIGHTS.to_vec(), weights);
            }
            Some("anki_reader._CARDS_FINGERPRINT_MOD") => {
                assert_eq!(CARD_FINGERPRINT_MODULUS, integer(&case.output));
            }
            // The window's constants: the window's test reads them.
            _ => {}
        }
    });
    let runtime = support::runtime();
    let fixture = Fixture::new(ENDPOINT);
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0));
    golden::each_case("change_probe", |case| {
        let cards: Vec<[i64; 7]> = case.input["cards"]
            .as_array()
            .unwrap_or_else(|| panic!("the cards: {}", case.input))
            .iter()
            .map(|row| {
                let fields: Vec<i64> = row
                    .as_array()
                    .unwrap_or_else(|| panic!("a card row: {row}"))
                    .iter()
                    .map(integer)
                    .collect();
                fields
                    .try_into()
                    .unwrap_or_else(|_| panic!("seven probed columns: {row}"))
            })
            .collect();
        let reviews: Vec<i64> = case.input["review_ids"]
            .as_array()
            .unwrap_or_else(|| panic!("the review ids: {}", case.input))
            .iter()
            .map(integer)
            .collect();
        synthetic::write_probe_rows(&fixture.copy(), &cards, &reviews);
        let probed = runtime
            .block_on(probe(&reader))
            .unwrap_or_else(|error| panic!("the copy is probed: {error}"));
        let expected = &case.output;
        assert_eq!(
            probed,
            Probe {
                newest_review_id: integer(&expected["max_revlog_id"]),
                card_count: integer(&expected["cards_count"]),
                card_fingerprint: integer(&expected["cards_fingerprint"]),
            },
            "{}",
            case.input
        );
    });
}

/// The scoped collection, the service's database, and the gate over them.
struct Bench {
    fixture: Fixture,
    db: Db,
    clock: Arc<ManualClock>,
    reader: CollectionReader,
    gate: ChangeGate,
}

impl Bench {
    /// A bench with no run on record.
    async fn new() -> Self {
        let fixture = Fixture::new(ENDPOINT);
        synthetic::build_scoped(&fixture.copy());
        let db = fixture.db().await;
        let clock = support::clock_at(IN_A_STUDY_DAY);
        let reader = synthetic::reader(&fixture.settings(), "", None, clock.clone());
        let gate = ChangeGate::new(db.clone(), StudyDayRule::default(), clock.clone());
        Self {
            fixture,
            db,
            clock,
            reader,
            gate,
        }
    }

    /// Records a sync run that ended now, succeeded or failed.
    async fn record_sync(&self, outcome: Result<(), ReasonCode>) {
        let now = self.clock.now();
        SqliteSyncRuns::new(self.db.clone())
            .record(&SyncRun {
                trigger: Trigger::Owner,
                started_at: now,
                finished_at: now,
                study_day: StudyDayRule::default().study_day(now),
                outcome,
                attempts: 1,
                full_download: false,
            })
            .await
            .unwrap_or_else(|error| panic!("the run is recorded: {error}"));
    }

    /// A bench whose first check ran the recompute because no anchor was persisted, and wrote the
    /// anchor every test changes one input against; the clock is then a minute on.
    async fn anchored() -> Self {
        let bench = Self::new().await;
        bench.record_sync(Ok(())).await;
        let first = bench.check(true).await;
        assert_eq!(first.decision, Decision::Run(RunReason::AnchorMissing));
        bench.recomputed(&first).await;
        bench.clock.advance(Duration::from_mins(1));
        bench
    }

    /// One check, as a cycle whose sync succeeded or failed makes it, with no deadline.
    async fn check(&self, sync_ok: bool) -> Checked {
        let history = self
            .gate
            .history()
            .await
            .unwrap_or_else(|error| panic!("the record is read: {error}"));
        let facts = CycleFacts {
            trigger: Trigger::Owner,
            sync_ok,
            history,
        };
        self.gate
            .check(&self.reader, &facts, &[])
            .await
            .unwrap_or_else(|error| panic!("the gate checks: {error}"))
    }

    /// The recompute `checked` ran, writing its anchor.
    async fn recomputed(&self, checked: &Checked) {
        self.gate
            .recomputed(checked)
            .await
            .unwrap_or_else(|error| panic!("the anchor is written: {error}"));
    }

    async fn anchor(&self) -> AnchorState {
        self.gate
            .state()
            .load()
            .await
            .unwrap_or_else(|error| panic!("the state is read: {error}"))
            .anchor
    }
}

#[tokio::test]
async fn an_unchanged_cycle_skips_the_recompute_and_records_skipped() {
    let bench = Bench::anchored().await;
    let anchored = bench.anchor().await;
    let unchanged = bench.check(true).await;
    assert_eq!(unchanged.decision, Decision::Skip);

    // The skip is a `skipped` row, with its trigger and study day, and no reason, attempt or
    // download; the anchor is as the recompute left it.
    let row: (String, i64, String, Option<String>, i64, i64) = sqlx::query_as(
        "SELECT trigger, study_day, status, reason, attempts, full_download FROM sync_runs \
         ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(bench.db.reader())
    .await
    .unwrap_or_else(|error| panic!("the record is read: {error}"));
    assert_eq!(
        row,
        ("owner".to_owned(), 20_000, "skipped".to_owned(), None, 0, 0)
    );
    let history = bench
        .gate
        .history()
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(history.last, Some(RunStatus::Skipped));
    assert_eq!(bench.anchor().await, anchored);
    assert!(
        matches!(anchored, AnchorState::Present(anchor) if anchor.recomputed_at == UtcMillis::from_epoch_millis(IN_A_STUDY_DAY)),
        "{anchored:?}"
    );
}

#[tokio::test]
async fn a_new_review_runs_the_recompute() {
    let bench = Bench::anchored().await;
    synthetic::add_reviews(
        &bench.fixture.copy(),
        &[PlannedReview {
            id: LATER_REVIEW,
            card: 1001,
            kind: 1,
            ease: 3,
        }],
    );
    let checked = bench.check(true).await;
    assert_eq!(
        checked.decision,
        Decision::Run(RunReason::NewestReviewChanged)
    );
    assert_eq!(checked.probe.newest_review_id, LATER_REVIEW);
    // The recompute's anchor holds the review, so the next cycle skips.
    bench.recomputed(&checked).await;
    assert_eq!(bench.check(true).await.decision, Decision::Skip);
}

#[tokio::test]
async fn a_changed_card_runs_the_recompute() {
    let bench = Bench::anchored().await;
    // A card rescheduled in place, with no review: its fingerprint moves, its count does not.
    synthetic::move_due(&bench.fixture.copy(), 1004, 99);
    let moved = bench.check(true).await;
    assert_eq!(
        moved.decision,
        Decision::Run(RunReason::CardFingerprintChanged)
    );
    bench.recomputed(&moved).await;
    assert_eq!(bench.check(true).await.decision, Decision::Skip);
    // A card deleted: the count moves.
    synthetic::delete_cards(&bench.fixture.copy(), &[1005]);
    let deleted = bench.check(true).await;
    assert_eq!(deleted.decision, Decision::Run(RunReason::CardCountChanged));
    assert_eq!(deleted.probe.card_count, 6);
}

#[tokio::test]
async fn a_new_study_day_runs_the_recompute() {
    let bench = Bench::anchored().await;
    // The next day's 04:00 rollover.
    let rollover = 20_001 * DAY_MS + 4 * HOUR_MS;
    bench.clock.set(UtcMillis::from_epoch_millis(rollover));
    let rolled = bench.check(true).await;
    assert_eq!(rolled.decision, Decision::Run(RunReason::StudyDayChanged));
    assert_eq!(rolled.study_day, StudyDay::from_epoch_day(20_001));
    // Served: a millisecond before the following rollover is still the new anchor's day.
    bench.recomputed(&rolled).await;
    bench
        .clock
        .set(UtcMillis::from_epoch_millis(rollover + DAY_MS - 1));
    assert_eq!(bench.check(true).await.decision, Decision::Skip);
}

#[tokio::test]
async fn a_settings_change_runs_the_recompute() {
    let bench = Bench::anchored().await;
    let mut write = bench
        .db
        .write()
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    let generation = Db::bump_settings_generation(&mut write)
        .await
        .unwrap_or_else(|error| panic!("the generation moves: {error}"));
    write
        .commit()
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(generation, 1);
    let changed = bench.check(true).await;
    assert_eq!(changed.decision, Decision::Run(RunReason::SettingsChanged));
    assert_eq!(changed.settings_generation, 1);
    bench.recomputed(&changed).await;
    assert_eq!(bench.check(true).await.decision, Decision::Skip);
}

#[tokio::test]
async fn a_failed_sync_or_a_failed_last_run_never_skips() {
    let bench = Bench::anchored().await;
    assert_eq!(
        bench.check(false).await.decision,
        Decision::Run(RunReason::SyncFailed),
        "this cycle's sync failed"
    );
    bench.record_sync(Err(ReasonCode::ServerError)).await;
    assert_eq!(
        bench.check(true).await.decision,
        Decision::Run(RunReason::LastRunFailed),
        "the run on record before this cycle failed"
    );
    // A record with no successful run at all runs the recompute before any anchor is consulted.
    let unrecorded = Bench::new().await;
    assert_eq!(
        unrecorded.check(true).await.decision,
        Decision::Run(RunReason::NoSuccessfulRun)
    );
    unrecorded.record_sync(Err(ReasonCode::AuthRejected)).await;
    assert_eq!(
        unrecorded.check(true).await.decision,
        Decision::Run(RunReason::NoSuccessfulRun)
    );
}

#[tokio::test]
async fn an_owner_rescore_forces_exactly_one_recompute() {
    let bench = Bench::anchored().await;
    bench
        .gate
        .state()
        .request_rescore(bench.clock.now())
        .await
        .unwrap_or_else(|error| panic!("the rescore is requested: {error}"));
    let forced = bench.check(true).await;
    assert_eq!(forced.decision, Decision::Run(RunReason::RescorePending));
    // The recompute consumes the request: the next cycle, with nothing else changed, skips.
    bench.recomputed(&forced).await;
    let state = bench
        .gate
        .state()
        .load()
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(!state.rescore_pending, "the recompute consumed the request");
    assert_eq!(bench.check(true).await.decision, Decision::Skip);
}

#[tokio::test]
async fn an_anchor_with_a_part_missing_is_unreadable_and_runs_the_recompute() {
    let bench = Bench::anchored().await;
    sqlx::query("UPDATE ingest_state SET anchor_card_count = NULL")
        .execute(bench.db.reader())
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(bench.anchor().await, AnchorState::Unreadable);
    let partial = bench.check(true).await;
    assert_eq!(partial.decision, Decision::Run(RunReason::AnchorUnreadable));
    // A row that is gone reads the same, and the next recompute writes it back.
    sqlx::query("DELETE FROM ingest_state")
        .execute(bench.db.reader())
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    let gone = bench.check(true).await;
    assert_eq!(gone.decision, Decision::Run(RunReason::AnchorUnreadable));
    bench.recomputed(&gone).await;
    assert!(
        matches!(bench.anchor().await, AnchorState::Present(anchor) if anchor.probe == gone.probe),
        "the recompute writes the row back"
    );
    assert_eq!(bench.check(true).await.decision, Decision::Skip);
}

/// A decision's inputs that skip: every term of R8 false.
fn quiet<'a>(anchor: &Anchor, deadlines: &'a [Deadline]) -> GateInputs<'a> {
    GateInputs {
        rescore_pending: false,
        sync_ok: true,
        history: RunHistory {
            last: Some(RunStatus::Ok),
            any_success: true,
        },
        anchor: AnchorState::Present(*anchor),
        settings_generation: anchor.settings_generation,
        study_day: anchor.study_day,
        probe: anchor.probe,
        now: UtcMillis::from_epoch_millis(anchor.recomputed_at.epoch_millis() + HOUR_MS),
        deadlines,
    }
}

const ANCHOR: Anchor = Anchor {
    probe: Probe {
        newest_review_id: 1_700_000_000_000,
        card_count: 12,
        card_fingerprint: 34_567,
    },
    study_day: StudyDay::from_epoch_day(20_000),
    recomputed_at: UtcMillis::from_epoch_millis(IN_A_STUDY_DAY),
    settings_generation: 3,
};

#[test]
fn the_gate_runs_for_the_first_term_that_holds_in_r8s_order() {
    assert_eq!(decide(&quiet(&ANCHOR, &[])), Decision::Skip);
    let due = [Deadline {
        label: "planted",
        at: UtcMillis::from_epoch_millis(IN_A_STUDY_DAY + 1),
    }];
    let mut inputs = quiet(&ANCHOR, &due);
    inputs.rescore_pending = true;
    inputs.sync_ok = false;
    inputs.history = RunHistory {
        last: Some(RunStatus::Error),
        any_success: false,
    };
    inputs.settings_generation += 1;
    inputs.study_day = StudyDay::from_epoch_day(20_001);
    inputs.probe = Probe {
        newest_review_id: 1,
        card_count: 1,
        card_fingerprint: 1,
    };
    // Each term, once the terms before it stop holding, is the reason.
    let mut reasons = Vec::new();
    let steps: [fn(&mut GateInputs<'_>); 11] = [
        |inputs| inputs.rescore_pending = false,
        |inputs| inputs.sync_ok = true,
        |inputs| inputs.history.any_success = true,
        |inputs| inputs.history.last = Some(RunStatus::Skipped),
        |inputs| inputs.anchor = AnchorState::Unreadable,
        |inputs| inputs.anchor = AnchorState::Present(ANCHOR),
        |inputs| inputs.settings_generation = ANCHOR.settings_generation,
        |inputs| inputs.study_day = ANCHOR.study_day,
        |inputs| inputs.probe.newest_review_id = ANCHOR.probe.newest_review_id,
        |inputs| inputs.probe.card_count = ANCHOR.probe.card_count,
        |inputs| inputs.probe.card_fingerprint = ANCHOR.probe.card_fingerprint,
    ];
    reasons.push(decide(&inputs));
    inputs.anchor = AnchorState::Missing;
    for step in steps {
        step(&mut inputs);
        reasons.push(decide(&inputs));
    }
    assert_eq!(
        reasons,
        [
            Decision::Run(RunReason::RescorePending),
            Decision::Run(RunReason::SyncFailed),
            Decision::Run(RunReason::NoSuccessfulRun),
            Decision::Run(RunReason::LastRunFailed),
            Decision::Run(RunReason::AnchorMissing),
            Decision::Run(RunReason::AnchorUnreadable),
            Decision::Run(RunReason::SettingsChanged),
            Decision::Run(RunReason::StudyDayChanged),
            Decision::Run(RunReason::NewestReviewChanged),
            Decision::Run(RunReason::CardCountChanged),
            Decision::Run(RunReason::CardFingerprintChanged),
            Decision::Run(RunReason::DeadlineDue { label: "planted" }),
        ]
    );
}

#[test]
fn a_deadline_runs_the_recompute_only_after_the_anchor_and_at_or_before_now() {
    let recomputed = ANCHOR.recomputed_at.epoch_millis();
    let now = recomputed + HOUR_MS;
    let deadline = |label, at| Deadline {
        label,
        at: UtcMillis::from_epoch_millis(at),
    };
    let decided = |deadlines: &[Deadline]| decide(&quiet(&ANCHOR, deadlines));
    assert_eq!(decided(&[deadline("served", recomputed)]), Decision::Skip);
    assert_eq!(
        decided(&[deadline("early", recomputed - 1)]),
        Decision::Skip
    );
    assert_eq!(decided(&[deadline("not yet due", now + 1)]), Decision::Skip);
    assert_eq!(
        decided(&[deadline("just after", recomputed + 1)]),
        Decision::Run(RunReason::DeadlineDue {
            label: "just after"
        })
    );
    assert_eq!(
        decided(&[deadline("now", now)]),
        Decision::Run(RunReason::DeadlineDue { label: "now" })
    );
    assert_eq!(
        decided(&[
            deadline("served", recomputed),
            deadline("not yet due", now + 1),
            deadline("due", now - 1),
        ]),
        Decision::Run(RunReason::DeadlineDue { label: "due" }),
        "the first deadline inside the window names the reason"
    );
}
