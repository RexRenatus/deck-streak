//! The skip day's preview, its tariff and its refund (SPEC-083 A10 to A13, A46; R7 to R9, T3, T4;
//! ADR-321 D6, D10, D13). Every skip, balance and rollup is synthetic and planted by hand on a
//! database each case opens; no row is anyone's data.

// An integration test is test code: its helpers panic on a malformed golden, and it prints the
// examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_coordination::skip::{
    PreviewRefusal, Settlement, preview, settle_applied, settle_undone,
};
use deck_streak_economy::tariff::{REFUND_SOURCE, TARIFF_SOURCE};
use deck_streak_economy::wallet::{DepositAnswer, SqliteWallet};
use deck_streak_ingest::skip::{
    FailReason, SKIP_DEFAULT_SEARCH, SkipId, SkipRecord, SkipState, SkipStore, skip_search,
};
use deck_streak_kernel::{Db, StudyDay, StudyDayRule, UtcMillis};
use serde_json::Value;
use sqlx::Row;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// `hour` o'clock UTC of study day `number` under the default rule (rollover at 04:00 UTC).
fn at(number: i64, hour: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(number * DAY_MS + hour * HOUR_MS)
}

fn integer(value: &Value) -> i64 {
    value.as_i64().expect("the golden holds an integer")
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// Plants a wallet of `balance` coins on a study day well before `today`.
async fn fund(db: &Db, today: i64, balance: i64) {
    let answer = SqliteWallet::new(db.clone())
        .deposit(
            day(today - 40),
            "seed",
            "balance",
            balance,
            at(today - 40, 12),
        )
        .await
        .expect("the synthetic balance is written");
    let expected = if balance > 0 {
        DepositAnswer::Deposited(balance)
    } else {
        DepositAnswer::NotPositive
    };
    assert_eq!(answer, expected, "the planted balance");
}

/// Plants an applied skip on `number`, undone at its day's evening when `undone`.
async fn plant(db: &Db, number: i64, cards_moved: i64, undone: bool) -> SkipId {
    let skips = SkipStore::new(db.clone());
    let id = skips
        .begin(day(number), None, at(number, 10))
        .await
        .expect("the planted take is recorded");
    skips
        .settle_applied(id, cards_moved, false)
        .await
        .expect("the planted take settles");
    if undone {
        skips
            .mark_undone(id, at(number, 20))
            .await
            .expect("the planted undo is recorded");
    }
    id
}

/// The skip `id` as the record holds it.
async fn record(db: &Db, id: SkipId) -> SkipRecord {
    SkipStore::new(db.clone())
        .records()
        .await
        .expect("the record reads")
        .into_iter()
        .find(|record| record.id == id)
        .expect("the skip is recorded")
}

/// The NONZERO movements of `source` for the skip `id`, as (study day, delta): the predecessor
/// writes no movement when it pays nothing, where the wallet writes one of 0 (T3).
async fn movements(db: &Db, source: &str, id: SkipId) -> Vec<(i64, i64)> {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "SELECT study_day, delta FROM coin_ledger \
         WHERE source = ?1 AND reference = ?2 AND delta != 0 ORDER BY study_day",
    )
    .bind(source)
    .bind(id.get().to_string())
    .fetch_all(&mut *write)
    .await
    .expect("the ledger reads")
    .into_iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

/// A golden's movements of `source`, as (study day, delta).
fn golden_movements(output: &Value, source: &str) -> Vec<(i64, i64)> {
    output["deltas"]
        .as_array()
        .expect("deltas")
        .iter()
        .filter(|delta| delta["source"] == source)
        .map(|delta| (integer(&delta["day"]), integer(&delta["delta"])))
        .collect()
}

/// The balance the ledger sums.
async fn balance(db: &Db) -> i64 {
    SqliteWallet::new(db.clone())
        .balance()
        .await
        .expect("the balance reads")
}

/// Plants today's rollup with `due_today` due review cards.
async fn rollup(db: &Db, today: i64, due_today: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, 1, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, ?2, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
    )
    .bind(today)
    .bind(due_today)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    write.commit().await.expect("the commit");
}

#[tokio::test]
async fn the_preview_matches_the_parity_golden_and_is_absent_without_a_rollup() {
    let cases = golden::read(&golden::committed("skip_preview"))
        .expect("the preview golden reads")
        .cases;
    let search = skip_search(SKIP_DEFAULT_SEARCH).expect("the default search is one expression");
    let mut absent = 0;
    for case in &cases {
        let input = &case.input;
        let today = integer(&input["today"]);
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        fund(&db, today, integer(&input["balance"])).await;
        for row in input["rows"].as_array().expect("rows") {
            plant(
                &db,
                integer(&row["day"]),
                integer(&row["cards_moved"]),
                row["undone"].as_bool().expect("undone"),
            )
            .await;
        }
        if input["active"].as_bool().expect("active") {
            plant(&db, today, 1, false).await;
        }
        if let Some(due) = input["due_today"].as_i64() {
            rollup(&db, today, due).await;
        }
        let shown = preview(&db, day(today), SKIP_DEFAULT_SEARCH)
            .await
            .expect("the preview reads");
        let output = &case.output;
        assert_eq!(
            shown.due_count,
            input["due_today"].as_i64(),
            "the due count is the rollup's, and absent with none: {input}"
        );
        if shown.due_count.is_none() {
            absent += 1;
            assert_eq!(
                integer(&output["due_today"]),
                0,
                "the predecessor's 0 for none"
            );
        } else {
            assert_eq!(shown.due_count, output["due_today"].as_i64(), "{input}");
        }
        assert_eq!(shown.day, day(integer(&output["day"])), "{input}");
        assert_eq!(
            Value::from(shown.already_skipped),
            output["already_skipped"],
            "{input}"
        );
        assert_eq!(Value::from(shown.spec.clone()), output["spec"], "{input}");
        assert_eq!(shown.tariff, integer(&output["tariff"]), "{input}");
        assert_eq!(
            Value::from(shown.tariff_funded),
            output["tariff_funded"],
            "{input}"
        );
        assert_eq!(shown.search, search, "the search the write would run");
    }
    println!(
        "A10: examined {} preview cases, {absent} with no rollup",
        cases.len()
    );
    assert_eq!(cases.len(), 26, "every case the golden records");
    assert!(absent > 0, "a case with no rollup was examined");

    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let refused = preview(&db, day(20_105), "(deck:A").await;
    assert!(
        matches!(refused, Err(PreviewRefusal::Search(_))),
        "a search that is not one expression is refused: {refused:?}"
    );
}

#[tokio::test]
async fn the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap() {
    let cases = golden::read(&golden::committed("skip_tariff"))
        .expect("the tariff golden reads")
        .cases;
    let mut outside_the_cap = 0;
    for case in &cases {
        let input = &case.input;
        let today = integer(&input["today"]);
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        let funded = integer(&input["balance"]);
        fund(&db, today, funded).await;
        let mut taken = None;
        for row in input["rows"].as_array().expect("rows") {
            let number = integer(&row["day"]);
            if number == today {
                let id = SkipStore::new(db.clone())
                    .begin(day(today), None, at(today, 10))
                    .await
                    .expect("the take is recorded");
                taken = Some((id, integer(&row["cards_moved"])));
            } else {
                plant(
                    &db,
                    number,
                    integer(&row["cards_moved"]),
                    row["undone"].as_bool().expect("undone"),
                )
                .await;
            }
        }
        let (id, cards_moved) = taken.expect("the case's skip is on its day");
        let settled = settle_applied(&db, id, cards_moved, at(today, 12))
            .await
            .expect("the settlement writes");
        let answer = &case.output["answer"];
        let expected = Settlement {
            price: integer(&answer["tariff"]),
            paid: integer(&answer["tariff_paid"]),
            unfunded: answer["unfunded"].as_bool().expect("unfunded"),
        };
        assert_eq!(settled, Some(expected), "{input}");
        assert_eq!(
            movements(&db, TARIFF_SOURCE, id).await,
            golden_movements(&case.output, "skip_tariff"),
            "the charge lands on the skip's own day: {input}"
        );
        let held = record(&db, id).await;
        assert_eq!(held.state, SkipState::Applied, "{input}");
        assert_eq!(held.cards_moved, cards_moved, "{input}");
        let details = case.output["details"].as_array().expect("details");
        assert_eq!(
            held.tariff_unfunded,
            details
                .iter()
                .any(|detail| detail == "applied (tariff unfunded)"),
            "the row records the shortfall: {input}"
        );
        assert_eq!(balance(&db).await, funded - expected.paid, "{input}");
        // The daily loss cap of this wallet is under the price, so a capped debit would pay less.
        if expected.paid > funded * 3 / 10 {
            outside_the_cap += 1;
        }
    }
    println!(
        "A11: examined {} tariff cases, {outside_the_cap} paying past the daily loss cap",
        cases.len()
    );
    assert_eq!(cases.len(), 13, "every case the golden records");
    assert!(
        outside_the_cap > 0,
        "a charge past the loss cap was examined"
    );
}

#[tokio::test]
async fn an_unfunded_skip_still_applies_and_records_the_shortfall() {
    let today = 20_105;
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    fund(&db, today, 20).await;
    plant(&db, today - 2, 3, false).await;
    plant(&db, today - 1, 3, false).await;
    let id = SkipStore::new(db.clone())
        .begin(day(today), Some(40), at(today, 10))
        .await
        .expect("the take is recorded");
    let settled = settle_applied(&db, id, 7, at(today, 12))
        .await
        .expect("the settlement writes");
    let held = record(&db, id).await;
    assert_eq!(
        (held.state, held.tariff_unfunded),
        (SkipState::Applied, true),
        "the skip applies and records its shortfall"
    );
    assert_eq!(
        settled,
        Some(Settlement {
            price: 100,
            paid: 20,
            unfunded: true
        })
    );
    assert_eq!(balance(&db).await, 0, "the wallet stops at zero");
    assert_eq!(movements(&db, TARIFF_SOURCE, id).await, vec![(today, -20)]);
}

#[tokio::test]
async fn undo_refunds_what_the_skip_paid_on_the_undo_day() {
    let cases = golden::read(&golden::committed("skip_tariff_refund"))
        .expect("the refund golden reads")
        .cases;
    let rule = StudyDayRule::default();
    let mut free = 0;
    let (mut settings, mut expected) = (Vec::new(), Vec::new());
    for case in &cases {
        let input = &case.input;
        let paid = integer(&input["paid"]);
        let undo_day = integer(&input["undo_day"]);
        let skip_day = undo_day - 1;
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        // Two earlier skips of the month price this one at the ladder's top, and a wallet of
        // `paid` coins pays exactly `paid`; a free skip has no earlier skip.
        fund(&db, skip_day, paid).await;
        if paid > 0 {
            plant(&db, skip_day - 2, 3, false).await;
            plant(&db, skip_day - 1, 3, false).await;
        }
        let id = SkipStore::new(db.clone())
            .begin(day(skip_day), None, at(skip_day, 10))
            .await
            .expect("the take is recorded");
        let settled = settle_applied(&db, id, 3, at(skip_day, 12))
            .await
            .expect("the settlement writes");
        let refunded = settle_undone(&db, rule, id, at(undo_day, 12))
            .await
            .expect("the undo writes");
        assert_eq!(
            refunded,
            integer(&case.output["returned"]),
            "the undo refunds what the skip paid: {input}"
        );
        assert_eq!(
            movements(&db, REFUND_SOURCE, id).await,
            golden_movements(&case.output, "skip_tariff_refund"),
            "the refund lands on the undo's study day: {input}"
        );
        // What the setup paid, the undo's mark and the wallet after it, judged after the loop so
        // the refund's own assertions above decide every case first.
        settings.push((
            settled.map(|settled| settled.paid),
            record(&db, id).await.undone_at,
            balance(&db).await,
        ));
        expected.push((Some(paid), Some(at(undo_day, 12)), paid));
        if paid == 0 {
            free += 1;
        }
    }
    println!(
        "A13: examined {} refund cases, {free} of a free skip",
        cases.len()
    );
    assert_eq!(cases.len(), 8, "every case the golden records");
    assert!(free > 0, "a free skip was examined");
    assert_eq!(
        settings, expected,
        "each skip paid its golden amount, is marked undone at its undo, and the wallet is whole"
    );
}

#[tokio::test]
async fn the_tariff_and_its_refund_are_taken_once_per_skip() {
    let skip_day = 20_105;
    let rule = StudyDayRule::default();
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    fund(&db, skip_day, 500).await;
    plant(&db, skip_day - 2, 3, false).await;
    plant(&db, skip_day - 1, 3, false).await;
    let id = SkipStore::new(db.clone())
        .begin(day(skip_day), None, at(skip_day, 10))
        .await
        .expect("the take is recorded");
    // The take's settlement on the skip's day, then the start-up settlement's retry the next day.
    let first = settle_applied(&db, id, 3, at(skip_day, 12))
        .await
        .expect("the settlement writes");
    let retried = settle_applied(&db, id, 3, at(skip_day + 1, 12))
        .await
        .expect("the retry writes");
    assert_eq!(
        movements(&db, TARIFF_SOURCE, id).await,
        vec![(skip_day, -100)],
        "the tariff is charged once, on the skip's day"
    );
    let charged = Some(Settlement {
        price: 100,
        paid: 100,
        unfunded: false,
    });
    assert_eq!((first, retried), (charged, charged));
    // The undo two days later, then its retry the day after it.
    let refunded = settle_undone(&db, rule, id, at(skip_day + 2, 12))
        .await
        .expect("the undo writes");
    settle_undone(&db, rule, id, at(skip_day + 3, 12))
        .await
        .expect("the retried undo writes");
    assert_eq!(
        movements(&db, REFUND_SOURCE, id).await,
        vec![(skip_day + 2, 100)],
        "the refund is credited once, on the undo's day"
    );
    assert_eq!(refunded, 100);
    assert_eq!(balance(&db).await, 500, "the wallet is whole again");
}

/// D13: a retry prices the skip as its first attempt did, whatever applied after it. A free skip
/// (the month's first) and a priced one (one earlier skip) are each settled, a skip of a LATER day
/// of the month applies, and the retry the day after must neither charge nor reprice.
#[tokio::test]
async fn a_retry_is_priced_as_its_first_attempt_after_a_later_skip_applies() {
    let skip_day = 20_105;
    for earlier in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        fund(&db, skip_day, 500).await;
        if earlier {
            plant(&db, skip_day - 1, 3, false).await;
        }
        let id = SkipStore::new(db.clone())
            .begin(day(skip_day), None, at(skip_day, 10))
            .await
            .expect("the take is recorded");
        let first = settle_applied(&db, id, 3, at(skip_day, 12))
            .await
            .expect("the settlement writes");
        plant(&db, skip_day + 1, 3, false).await;
        let retried = settle_applied(&db, id, 3, at(skip_day + 2, 12))
            .await
            .expect("the retry writes");
        let price = if earlier { 50 } else { 0 };
        let expected = Some(Settlement {
            price,
            paid: price,
            unfunded: false,
        });
        assert_eq!(
            (first, retried),
            (expected, expected),
            "a retry is priced as its first attempt: earlier={earlier}"
        );
        let charged: Vec<(i64, i64)> = if earlier {
            vec![(skip_day, -50)]
        } else {
            Vec::new()
        };
        assert_eq!(
            movements(&db, TARIFF_SOURCE, id).await,
            charged,
            "a later skip of the month never charges this one: earlier={earlier}"
        );
    }
}

/// A skip the settlement must not touch: one that was undone, and one that failed, answer `None`
/// and write no movement (the guard in `settle_applied` is an either, never a both).
#[tokio::test]
async fn an_undone_or_a_failed_skip_is_never_settled_applied() {
    let skip_day = 20_105;
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    fund(&db, skip_day, 500).await;
    let undone = plant(&db, skip_day - 2, 3, true).await;
    let skips = SkipStore::new(db.clone());
    let failed = skips
        .begin(day(skip_day - 1), None, at(skip_day - 1, 10))
        .await
        .expect("the take is recorded");
    skips
        .settle_failed(failed, FailReason::PushFailed)
        .await
        .expect("the failure settles");
    for (label, id) in [("undone", undone), ("failed", failed)] {
        let settled = settle_applied(&db, id, 3, at(skip_day, 12))
            .await
            .expect("the settlement answers");
        assert_eq!(settled, None, "a {label} skip is not settled");
        assert_eq!(
            movements(&db, TARIFF_SOURCE, id).await,
            Vec::<(i64, i64)>::new(),
            "a {label} skip is never charged"
        );
    }
}
