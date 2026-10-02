//! An erase empties the coin ledger and resets the economy's row (SPEC-082 A14, R18; SPEC-021): the
//! economy data-rights port, as `privacy` drives it on one write, leaves no movement and keeps
//! `economy_state`'s one row with no pass and no surcharge. Every movement here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_economy::data_rights::{COIN_LEDGER_TABLE, ECONOMY_STATE_TABLE, EconomyDataRights};
use deck_streak_economy::wallet::{DepositAnswer, MintAnswer, PurchaseAnswer, SqliteWallet};
use deck_streak_kernel::{DataRights, Db, StudyDay, UtcMillis};

/// The instant the movements are written at.
const AT: UtcMillis = UtcMillis::from_epoch_millis(1_700_000_000_000);

/// `economy_state`'s row as the test reads it back: id, the pass's end, the surcharge's end and
/// `created_at`.
type StateRow = (i64, Option<i64>, Option<i64>, i64);

/// How many movements the ledger holds, read past the ports.
async fn movements(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM coin_ledger")
        .fetch_one(db.reader())
        .await
        .expect("the ledger's count")
}

/// Every row of `economy_state`.
async fn state(db: &Db) -> Vec<StateRow> {
    sqlx::query_as(
        "SELECT id, pass_ends_at, surcharge_ends_at, created_at FROM economy_state ORDER BY id",
    )
    .fetch_all(db.reader())
    .await
    .expect("the economy's row")
}

#[tokio::test]
async fn an_erase_empties_the_ledger_and_resets_the_state() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let wallet = SqliteWallet::new(db.clone());
    let day = StudyDay::from_epoch_day(70);

    // The owner's coins, through the wallet's ports: a payout, a mint and a purchase.
    assert_eq!(
        wallet
            .deposit(day, "payout", "quest:q1", 50, AT)
            .await
            .expect("a credit"),
        DepositAnswer::Deposited(50)
    );
    assert_eq!(
        wallet
            .settle_mint(day, 12, false, AT)
            .await
            .expect("a mint"),
        MintAnswer::Settled(12)
    );
    assert_eq!(
        wallet
            .purchase(day, "shop", "pass:1", 40, AT)
            .await
            .expect("a purchase"),
        PurchaseAnswer::Bought(40)
    );
    assert_eq!(movements(&db).await, 3, "the seed wrote three movements");
    // The shop's row holds a pass and a surcharge; the shop (E3) writes it, so the fixture does.
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "UPDATE economy_state SET pass_ends_at = 1700000001800, \
         surcharge_ends_at = 1700000172800 WHERE id = 1",
    )
    .execute(&mut *write)
    .await
    .expect("a pass and a surcharge");
    write.commit().await.expect("the row written");
    let seeded = state(&db).await;
    assert_eq!(seeded.len(), 1, "economy_state is one row");
    let created_at = seeded[0].3;

    // The port declares both tables, and the erase runs on one write, as privacy drives it.
    let tables: Vec<&str> = EconomyDataRights
        .declaration()
        .expect("the declaration")
        .tables()
        .iter()
        .map(|rights| rights.table)
        .collect();
    assert_eq!(tables, [COIN_LEDGER_TABLE, ECONOMY_STATE_TABLE]);
    let mut write = db.write().await.expect("a write");
    EconomyDataRights
        .erase(&mut write)
        .await
        .expect("the erase");
    write.commit().await.expect("the erase committed");

    assert_eq!(movements(&db).await, 0, "the erase empties the coin ledger");
    assert_eq!(
        state(&db).await,
        [(1, None, None, created_at)],
        "the row stays, with no pass and no surcharge"
    );
    assert_eq!(wallet.balance().await.expect("the balance"), 0);
}
