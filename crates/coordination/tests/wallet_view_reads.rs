//! The wallet view's four reads (SPEC-082 R5, R15; ADR-315 ruling 3): a read that fails fails the
//! view, whichever of the four it is. Each test faults ONE read and leaves the other three
//! answering, so a view that swallows that one read's error shows up as an answer, not an error.
//! The fault is data past the ports: a sum that overflows the ledger's integers on one read's own
//! rows, or a movement whose source is not text. Every movement is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_coordination::wallet_view::wallet_view;
use deck_streak_economy::wallet::SqliteWallet;
use deck_streak_kernel::{Db, StudyDay};
use tempfile::TempDir;

/// The view's study day, as an epoch day.
const TODAY: i64 = 20_000;
/// Half the integers' range: two of them overflow a sum, one alone does not.
const HALF: i64 = 1 << 62;

/// A migrated database in a temporary directory.
async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

/// Writes `movements` past the ports, in this order: study day, delta and whether the source is
/// bytes that are not text instead of a name.
async fn hold(db: &Db, movements: &[(i64, i64, bool)]) {
    let mut write = db.write().await.expect("a write");
    for (index, (day, delta, not_text)) in movements.iter().enumerate() {
        sqlx::query(
            "INSERT INTO coin_ledger (study_day, source, reference, delta, created_at) \
             VALUES (?1, CASE WHEN ?4 THEN CAST(x'ff' AS TEXT) ELSE 'synthetic' END, ?2, ?3, 1000)",
        )
        .bind(day)
        .bind(index.to_string())
        .bind(delta)
        .bind(not_text)
        .execute(&mut *write)
        .await
        .expect("the synthetic movement is written");
    }
    write.commit().await.expect("the commit");
}

/// Which of the wallet's four reads fail, in the view's order: balance, balance before today,
/// today's debits, the first page.
async fn failing(db: &Db) -> [bool; 4] {
    let wallet = SqliteWallet::new(db.clone());
    let today = StudyDay::from_epoch_day(TODAY);
    [
        wallet.balance().await.is_err(),
        wallet.balance_before(today).await.is_err(),
        wallet.debits_for_day(today).await.is_err(),
        wallet.movements(None).await.is_err(),
    ]
}

/// Holds `movements`, asserts exactly the reads `expected` fail, and asserts the view fails.
async fn judge(movements: &[(i64, i64, bool)], expected: [bool; 4]) {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = database(&scratch).await;
    hold(&db, movements).await;
    assert_eq!(failing(&db).await, expected, "the reads that fail");
    let view = wallet_view(&db, StudyDay::from_epoch_day(TODAY), None).await;
    assert!(
        view.is_err(),
        "a failed read fails the view, never an answer: {view:?}"
    );
    db.close().await;
}

#[tokio::test]
async fn a_balance_read_that_fails_fails_the_view() {
    // Two credits today overflow the whole-ledger sum alone: before today and today's debits
    // never see them.
    judge(
        &[(TODAY, HALF, false), (TODAY, HALF, false)],
        [true, false, false, false],
    )
    .await;
}

#[tokio::test]
async fn a_balance_before_today_read_that_fails_fails_the_view() {
    // Two credits of earlier days overflow the sum before today. A debit today, written between
    // them, keeps the whole-ledger sum, in the order it is read, inside the integers.
    judge(
        &[
            (TODAY - 2, HALF, false),
            (TODAY, -HALF, false),
            (TODAY - 1, HALF, false),
        ],
        [false, true, false, false],
    )
    .await;
}

#[tokio::test]
async fn a_debits_read_that_fails_fails_the_view() {
    // Two debits today overflow the sum of their negations alone; the ledger's own sum, exactly
    // the least integer, does not.
    judge(
        &[(TODAY, -HALF, false), (TODAY, -HALF, false)],
        [false, false, true, false],
    )
    .await;
}

#[tokio::test]
async fn a_movements_read_that_fails_fails_the_view() {
    // A movement whose source is not text reads in no sum and fails the page alone.
    judge(&[(TODAY - 1, 5, true)], [false, false, false, true]).await;
}
