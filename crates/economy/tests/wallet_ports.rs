//! The wallet's ports (SPEC-082 A4 to A6, A8, A19; R2, R4, R5, R7, R8; ADR-308): a burst of
//! concurrent capped debits and purchases never takes the balance below the floor, a credit of one
//! key is written once, a settled day's mint is only raised, a floor-clipped debit pays what is held
//! and never refuses, a once-ever credit is written once on any day, and the history pages the
//! movements newest first (R15; ADR-315 ruling 3). Every source, reference and amount here is
//! synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use deck_streak_economy::constants::WALLET_FLOOR;
use deck_streak_economy::rules::daily_loss_cap;
use deck_streak_economy::wallet::{
    DebitAnswer, DepositAnswer, MINT_REFERENCE, MINT_SOURCE, MOVEMENTS_PAGE, MintAnswer,
    MovementsPage, PurchaseAnswer, PurchaseRefused, SqliteWallet,
};
use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use tempfile::TempDir;
use tokio::sync::Barrier;

/// A ledger row as the test reads it back: study day, source, reference and delta.
type Row = (i64, String, String, i64);

/// The instant the movements are written at.
const AT: UtcMillis = UtcMillis::from_epoch_millis(1_700_000_000_000);

/// A migrated file-backed database in a temporary directory, and the wallet over it.
async fn wallet() -> (TempDir, Db, SqliteWallet) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let wallet = SqliteWallet::new(db.clone());
    (directory, db, wallet)
}

/// Study day `day`, as an epoch day.
const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// Every row of the ledger, in the order written, read past the ports.
async fn rows(db: &Db) -> Vec<Row> {
    sqlx::query_as("SELECT study_day, source, reference, delta FROM coin_ledger ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("the ledger's rows")
}

/// The ledger's sum, read past the ports: the balance as R2 defines it.
async fn summed(db: &Db) -> i64 {
    rows(db).await.iter().map(|row| row.3).sum()
}

/// A row as the ledger stores it.
fn row(day: i64, source: &str, reference: &str, delta: i64) -> Row {
    (day, source.to_owned(), reference.to_owned(), delta)
}

/// How many concurrent callers the burst releases at once.
const BURST: usize = 16;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_wallet_never_goes_negative_under_a_burst() {
    let (_directory, db, wallet) = wallet().await;
    // 120 credited on study day 9, then a later day's tariff takes 100: day 10 starts with a wallet
    // of 120 (its cap is 36) while the balance holds 20, so the wallet, not the cap, is the bound.
    assert_eq!(
        wallet
            .deposit(day(9), "payout", "seed", 120, AT)
            .await
            .expect("the seed credit"),
        DepositAnswer::Deposited(120)
    );
    assert_eq!(
        wallet
            .debit_floored(day(11), "tariff", "skip", 100, AT)
            .await
            .expect("the later tariff"),
        DebitAnswer::Debited {
            paid: 100,
            forgiven: false
        }
    );
    assert_eq!(summed(&db).await, 20, "the seed leaves 20 coins");
    assert_eq!(daily_loss_cap(120), 36, "day 10's cap exceeds the balance");

    // Half the callers take a capped debit of 7 and half buy for 9, each on its own key, all on
    // study day 10, released together.
    let start = Arc::new(Barrier::new(BURST));
    let tasks: Vec<_> = (0..BURST)
        .map(|caller| {
            let wallet = wallet.clone();
            let start = Arc::clone(&start);
            tokio::spawn(async move {
                let reference = format!("burst:{caller}");
                start.wait().await;
                if caller % 2 == 0 {
                    match wallet
                        .debit_capped(day(10), "fine", &reference, 7, AT)
                        .await
                        .expect("a capped debit")
                    {
                        DebitAnswer::Debited { paid, .. } => paid,
                        other => panic!("a fresh capped debit of 7 answered {other:?}"),
                    }
                } else {
                    match wallet
                        .purchase(day(10), "shop", &reference, 9, AT)
                        .await
                        .expect("a purchase")
                    {
                        PurchaseAnswer::Bought(price) => price,
                        PurchaseAnswer::Refused(PurchaseRefused::BelowPrice { .. }) => 0,
                        other => panic!("a fresh purchase of 9 answered {other:?}"),
                    }
                }
            })
        })
        .collect();
    let mut paid = 0;
    for task in tasks {
        paid += task.await.expect("the caller ran");
    }

    let balance = summed(&db).await;
    assert!(
        balance >= WALLET_FLOOR,
        "the burst took the balance to {balance}, below the floor"
    );
    assert_eq!(
        balance,
        20 - paid,
        "the balance is the seed less what the ports reported paid"
    );
    // Every capped debit pays min(7, balance), so in any order the burst drains the wallet to the
    // floor and no further.
    assert_eq!(
        balance, WALLET_FLOOR,
        "the burst drains the wallet to the floor"
    );
    let debited: i64 = rows(&db)
        .await
        .iter()
        .filter(|row| row.0 == 10 && row.3 < 0)
        .map(|row| -row.3)
        .sum();
    assert!(
        debited <= 36,
        "day 10 debited {debited}, past its cap of 36"
    );
}

#[tokio::test]
async fn a_credit_of_one_key_is_written_once() {
    let (_directory, db, wallet) = wallet().await;
    let first = wallet
        .deposit(day(20), "payout", "quest:q1", 15, AT)
        .await
        .expect("a credit");
    let again = wallet
        .deposit(day(20), "payout", "quest:q1", 15, AT)
        .await
        .expect("the same credit again");
    // The key is the study day, the source and the reference: another day is another key.
    let next_day = wallet
        .deposit(day(21), "payout", "quest:q1", 15, AT)
        .await
        .expect("the credit on the next day");
    let nothing = wallet
        .deposit(day(20), "payout", "quest:q2", 0, AT)
        .await
        .expect("a credit of nothing");
    assert_eq!(
        [first, again, next_day, nothing],
        [
            DepositAnswer::Deposited(15),
            DepositAnswer::AlreadyDeposited,
            DepositAnswer::Deposited(15),
            DepositAnswer::NotPositive,
        ]
    );
    // A refund is a positive movement written once on its key, as a credit is.
    let refunds = [
        wallet
            .refund(day(20), "refund", "shop:s1", 9, AT)
            .await
            .expect("a refund"),
        wallet
            .refund(day(20), "refund", "shop:s1", 9, AT)
            .await
            .expect("the same refund again"),
    ];
    assert_eq!(
        refunds,
        [DepositAnswer::Deposited(9), DepositAnswer::AlreadyDeposited]
    );
    assert_eq!(
        rows(&db).await,
        [
            row(20, "payout", "quest:q1", 15),
            row(21, "payout", "quest:q1", 15),
            row(20, "refund", "shop:s1", 9),
        ],
        "one movement per study day, source and reference"
    );
}

/// Mutation coverage (R7, A19): a refund is keyed on its study day, source and reference, so a
/// refund on a later day of a key held earlier writes its own movement. Only `credit_once` holds
/// the any-day guard. Added after its code; the plant routes `refund_on` through it.
#[tokio::test]
async fn a_refund_on_a_later_day_of_a_held_key_writes_its_own_movement() {
    #[derive(Clone, Copy)]
    enum Port {
        Deposit,
        DepositOnce,
        Refund,
    }
    // The three seed members: a movement of the key on day 30 by each port that can write one,
    // then a refund of 9 on day 31.
    let cases = [
        ("K-009 deposit then refund", Port::Deposit),
        ("K-021 deposit_once then refund", Port::DepositOnce),
        ("K-033 refund then refund", Port::Refund),
    ];
    for (name, first) in cases {
        let (_directory, db, wallet) = wallet().await;
        let held = match first {
            Port::Deposit => wallet.deposit(day(30), "payout", "k1", 7, AT).await,
            Port::DepositOnce => wallet.deposit_once(day(30), "payout", "k1", 7, AT).await,
            Port::Refund => wallet.refund(day(30), "payout", "k1", 7, AT).await,
        }
        .expect("the earlier movement");
        assert_eq!(
            held,
            DepositAnswer::Deposited(7),
            "{name}: the day-30 movement"
        );
        let later = wallet
            .refund(day(31), "payout", "k1", 9, AT)
            .await
            .expect("the refund on the later day");
        assert_eq!(
            later,
            DepositAnswer::Deposited(9),
            "{name}: the day-31 refund"
        );
        assert_eq!(
            rows(&db).await,
            [row(30, "payout", "k1", 7), row(31, "payout", "k1", 9)],
            "{name}: two movements, one per study day"
        );
        assert_eq!(summed(&db).await, 16, "{name}: the balance");
    }
}

#[tokio::test]
async fn a_settled_days_mint_is_raised_and_never_lowered() {
    let (_directory, db, wallet) = wallet().await;
    let mut answers = Vec::new();
    // The current study day's mint follows its base at each recompute, down as well as up.
    for amount in [10, 25, 15] {
        answers.push(
            wallet
                .settle_mint(day(30), amount, false, AT)
                .await
                .expect("an open day's mint"),
        );
    }
    // Once the day is settled, a later recompute raises its mint and never lowers it.
    for amount in [12, 30, 0] {
        answers.push(
            wallet
                .settle_mint(day(30), amount, true, AT)
                .await
                .expect("a settled day's mint"),
        );
    }
    answers.push(
        wallet
            .settle_mint(day(30), -5, true, AT)
            .await
            .expect("a negative mint"),
    );
    // A day whose mint is 0 holds no movement.
    answers.push(
        wallet
            .settle_mint(day(31), 0, false, AT)
            .await
            .expect("a zero mint"),
    );
    assert_eq!(
        answers,
        [
            MintAnswer::Settled(10),
            MintAnswer::Settled(25),
            MintAnswer::Settled(15),
            MintAnswer::Settled(15),
            MintAnswer::Settled(30),
            MintAnswer::Settled(30),
            MintAnswer::Negative,
            MintAnswer::Settled(0),
        ]
    );
    assert_eq!(
        rows(&db).await,
        [row(30, MINT_SOURCE, MINT_REFERENCE, 30)],
        "a day has one mint movement"
    );

    // The current day's mint is lowered no further than the floor: 20 minted, 18 spent, then the
    // base falls to nothing, so only the 2 coins still held come back off the mint.
    let (_directory, db, wallet) = self::wallet().await;
    assert_eq!(
        wallet
            .settle_mint(day(40), 20, false, AT)
            .await
            .expect("the mint"),
        MintAnswer::Settled(20)
    );
    assert_eq!(
        wallet
            .purchase(day(40), "shop", "pass:1", 18, AT)
            .await
            .expect("a purchase"),
        PurchaseAnswer::Bought(18)
    );
    assert_eq!(
        wallet
            .settle_mint(day(40), 0, false, AT)
            .await
            .expect("the lowered mint"),
        MintAnswer::Settled(18)
    );
    assert_eq!(
        summed(&db).await,
        WALLET_FLOOR,
        "the lowering stops at the floor"
    );
}

#[tokio::test]
async fn a_floor_clipped_debit_pays_what_is_held() {
    let (_directory, db, wallet) = wallet().await;
    assert_eq!(
        wallet
            .deposit(day(50), "payout", "seed", 30, AT)
            .await
            .expect("the seed"),
        DepositAnswer::Deposited(30)
    );
    // Day 51 starts with 30 (a cap of 9), and the floored debit is not the capped one: it pays
    // all 30 the wallet holds.
    let mut answers = vec![
        wallet
            .debit_floored(day(51), "tariff", "skip:1", 50, AT)
            .await
            .expect("a tariff past the balance"),
    ];
    // An empty wallet pays nothing and is not refused, and the key it wrote holds a retry.
    for _ in 0..2 {
        answers.push(
            wallet
                .debit_floored(day(52), "tariff", "skip:2", 50, AT)
                .await
                .expect("a tariff on an empty wallet"),
        );
    }
    answers.push(
        wallet
            .debit_floored(day(53), "tariff", "skip:3", 0, AT)
            .await
            .expect("a tariff of nothing"),
    );
    assert_eq!(
        wallet
            .deposit(day(54), "payout", "seed:2", 12, AT)
            .await
            .expect("a second seed"),
        DepositAnswer::Deposited(12)
    );
    answers.push(
        wallet
            .debit_floored(day(54), "tariff", "skip:4", 5, AT)
            .await
            .expect("a tariff the wallet covers"),
    );
    assert_eq!(
        answers,
        [
            DebitAnswer::Debited {
                paid: 30,
                forgiven: true
            },
            DebitAnswer::Debited {
                paid: 0,
                forgiven: true
            },
            DebitAnswer::AlreadyDebited { paid: 0 },
            DebitAnswer::NothingRequested,
            DebitAnswer::Debited {
                paid: 5,
                forgiven: false
            },
        ]
    );
    assert_eq!(
        rows(&db).await,
        [
            row(50, "payout", "seed", 30),
            row(51, "tariff", "skip:1", -30),
            row(52, "tariff", "skip:2", 0),
            row(54, "payout", "seed:2", 12),
            row(54, "tariff", "skip:4", -5),
        ]
    );
    assert_eq!(summed(&db).await, 7);
}

#[tokio::test]
async fn a_once_ever_credit_is_written_once_on_any_day() {
    let (_directory, db, wallet) = wallet().await;
    let requests = [
        (60, "season", "node:7", 25),
        // The same payout settled on a later study day, and again on its own day.
        (61, "season", "node:7", 25),
        (60, "season", "node:7", 25),
        // Another reference, and another source, are other payouts.
        (61, "season", "node:8", 25),
        (62, "quest", "node:7", 10),
    ];
    let mut answers = Vec::new();
    for (on, source, reference, amount) in requests {
        answers.push(
            wallet
                .deposit_once(day(on), source, reference, amount, AT)
                .await
                .expect("a once-ever credit"),
        );
    }
    assert_eq!(
        answers,
        [
            DepositAnswer::Deposited(25),
            DepositAnswer::AlreadyDeposited,
            DepositAnswer::AlreadyDeposited,
            DepositAnswer::Deposited(25),
            DepositAnswer::Deposited(10),
        ]
    );
    assert_eq!(
        rows(&db).await,
        [
            row(60, "season", "node:7", 25),
            row(61, "season", "node:8", 25),
            row(62, "quest", "node:7", 10),
        ],
        "one movement per source and reference across every study day"
    );
}

/// Mutation coverage (R2): the balance the pool reads is the ledger's sum, not a constant.
#[tokio::test]
async fn the_balance_is_the_sum_of_every_movement() {
    let (_directory, db, wallet) = wallet().await;
    let deposited = [
        wallet
            .deposit(day(70), "payout", "quest:b1", 40, AT)
            .await
            .expect("a deposit"),
        wallet
            .deposit(day(71), "payout", "quest:b2", 25, AT)
            .await
            .expect("a second deposit"),
    ];
    assert_eq!(
        deposited,
        [DepositAnswer::Deposited(40), DepositAnswer::Deposited(25)]
    );
    assert_eq!(
        wallet
            .debit_floored(day(71), "tariff", "skip:b3", 18, AT)
            .await
            .expect("a debit"),
        DebitAnswer::Debited {
            paid: 18,
            forgiven: false
        }
    );
    assert_eq!(wallet.balance().await.expect("the balance"), 47);
    assert_eq!(summed(&db).await, 47, "the balance is the ledger's sum");
}

/// Mutation coverage (R7, R8): a purchase that takes the balance exactly to the floor is bought, and
/// a retry of its key answers the price it paid.
#[tokio::test]
async fn a_purchase_of_the_whole_balance_is_bought_once() {
    let (_directory, _db, wallet) = wallet().await;
    assert_eq!(
        wallet
            .deposit(day(80), "payout", "seed:p", 12, AT)
            .await
            .expect("the seed"),
        DepositAnswer::Deposited(12)
    );
    let answers = [
        wallet
            .purchase(day(80), "shop", "freeze:p1", 12, AT)
            .await
            .expect("a purchase of the whole balance"),
        wallet
            .purchase(day(80), "shop", "freeze:p1", 12, AT)
            .await
            .expect("the same purchase again"),
    ];
    assert_eq!(
        answers,
        [
            PurchaseAnswer::Bought(12),
            PurchaseAnswer::AlreadyBought(12)
        ]
    );
    assert_eq!(wallet.balance().await.expect("the balance"), WALLET_FLOOR);
}

/// Mutation coverage (R7): a retried debit, floored or capped, answers the coins its key paid.
#[tokio::test]
async fn a_retried_debit_answers_what_it_paid() {
    let (_directory, _db, wallet) = wallet().await;
    assert_eq!(
        wallet
            .deposit(day(90), "payout", "seed:r", 100, AT)
            .await
            .expect("the seed"),
        DepositAnswer::Deposited(100)
    );
    let answers = [
        wallet
            .debit_floored(day(91), "tariff", "skip:r1", 10, AT)
            .await
            .expect("a tariff"),
        wallet
            .debit_floored(day(91), "tariff", "skip:r1", 10, AT)
            .await
            .expect("the same tariff again"),
        wallet
            .debit_capped(day(91), "fine", "fine:r2", 8, AT)
            .await
            .expect("a fine"),
        wallet
            .debit_capped(day(91), "fine", "fine:r2", 8, AT)
            .await
            .expect("the same fine again"),
    ];
    assert_eq!(
        answers,
        [
            DebitAnswer::Debited {
                paid: 10,
                forgiven: false
            },
            DebitAnswer::AlreadyDebited { paid: 10 },
            DebitAnswer::Debited {
                paid: 8,
                forgiven: false
            },
            DebitAnswer::AlreadyDebited { paid: 8 },
        ]
    );
}

/// Mutation coverage (R5): a capped debit pays no more than the day's cap less what the day has
/// already debited.
#[tokio::test]
async fn a_capped_debit_pays_no_more_than_the_days_cap_left() {
    let (_directory, _db, wallet) = wallet().await;
    assert_eq!(
        wallet
            .deposit(day(100), "payout", "seed:c", 100, AT)
            .await
            .expect("the seed"),
        DepositAnswer::Deposited(100)
    );
    let answers = [
        wallet
            .debit_capped(day(101), "fine", "fine:c1", 20, AT)
            .await
            .expect("a first fine"),
        wallet
            .debit_capped(day(101), "fine", "fine:c2", 20, AT)
            .await
            .expect("a second fine past the cap"),
    ];
    assert_eq!(
        answers,
        [
            DebitAnswer::Debited {
                paid: 20,
                forgiven: false
            },
            DebitAnswer::Debited {
                paid: 10,
                forgiven: true
            },
        ]
    );
    // Day 101 starts with 100, so its cap is 30.
    assert_eq!(daily_loss_cap(100), 30, "day 101's cap");
    assert_eq!(wallet.balance().await.expect("the balance"), 70);
}

#[tokio::test]
async fn the_movements_come_newest_first_a_page_at_a_time() {
    let (_directory, _db, wallet) = wallet().await;
    // Twenty-five movements over four study days, written out of day order, so the page boundary
    // falls inside one day: the order is the study day, newest first, then the reverse of the
    // order the movements were written.
    let mut written = Vec::new();
    for index in 0..25_i64 {
        let on = 10 + index * 3 % 4;
        assert_eq!(
            wallet
                .deposit(day(on), "payout", &format!("r{index:02}"), index + 1, AT)
                .await
                .expect("a deposit"),
            DepositAnswer::Deposited(index + 1)
        );
        written.push((on, index + 1));
    }
    let mut expected: Vec<(i64, i64)> = written.iter().rev().copied().collect();
    expected.sort_by_key(|(on, _)| std::cmp::Reverse(*on));

    let first = wallet.movements(None).await.expect("the first page");
    let seen = |page: &MovementsPage| -> Vec<(i64, i64)> {
        page.movements
            .iter()
            .map(|movement| (movement.day.epoch_day(), movement.delta))
            .collect()
    };
    assert_eq!(
        seen(&first),
        expected[..MOVEMENTS_PAGE],
        "the first page, newest first"
    );
    let last = first
        .movements
        .last()
        .expect("a movement on the first page");
    assert_eq!(
        first.next,
        Some(last.id),
        "the cursor is the last movement shown"
    );
    assert!(
        first
            .movements
            .iter()
            .all(|movement| movement.source == "payout"),
        "each movement carries its source"
    );

    let second = wallet.movements(first.next).await.expect("the second page");
    assert_eq!(
        seen(&second),
        expected[MOVEMENTS_PAGE..],
        "the second page starts after the cursor"
    );
    assert_eq!(
        second.next, None,
        "no movement is older than the second page"
    );
    assert_eq!(
        first.movements.len() + second.movements.len(),
        written.len(),
        "every movement is shown once"
    );
}

#[tokio::test]
async fn a_page_that_holds_the_last_movement_names_no_next_page() {
    let (_directory, _db, wallet) = wallet().await;
    // Exactly a page of movements, twenty spelt literally rather than read from the page size, so
    // the page's last row is the ledger's last movement: no older movement exists to name.
    for index in 0..20_i64 {
        assert_eq!(
            wallet
                .deposit(day(10 + index), "payout", &format!("p{index:02}"), 1, AT)
                .await
                .expect("a deposit"),
            DepositAnswer::Deposited(1)
        );
    }
    let full = wallet.movements(None).await.expect("the only page");
    assert_eq!(full.movements.len(), 20, "a page holds twenty movements");
    assert_eq!(
        full.next, None,
        "a page that holds the last movement names no next page"
    );

    // One older movement more, and the same page now has a movement after it to name.
    assert_eq!(
        wallet
            .deposit(day(9), "payout", "p20", 1, AT)
            .await
            .expect("a deposit"),
        DepositAnswer::Deposited(1)
    );
    let page = wallet.movements(None).await.expect("the first page");
    assert_eq!(
        page.movements.len(),
        20,
        "a page still holds twenty movements"
    );
    assert_eq!(
        page.next,
        page.movements.last().map(|movement| movement.id),
        "the twenty-first movement makes the page name its last as the cursor"
    );
    assert!(
        page.next.is_some(),
        "an older movement exists past the page"
    );
}

/// Writes one synthetic movement past the ports.
async fn insert_movement(db: &Db, day: i64, source: &str, delta: i64) {
    sqlx::query(
        "INSERT INTO coin_ledger (study_day, source, reference, delta, created_at) \
         VALUES (?1, ?2, '', ?3, 1000)",
    )
    .bind(day)
    .bind(source)
    .bind(delta)
    .execute(db.reader())
    .await
    .expect("the synthetic movement is written");
}

#[tokio::test]
async fn a_movements_page_over_a_store_that_cannot_be_read_is_an_error_never_an_empty_page() {
    let (_directory, db, wallet) = wallet().await;
    insert_movement(&db, 100, "payout", 7).await;
    let before = wallet.movements(None).await.expect("the page reads");
    assert_eq!(before.movements.len(), 1, "the store holds the movement");
    db.close().await;
    let failed = wallet.movements(None).await;
    assert!(
        failed.is_err(),
        "an unreadable store is no empty page: {failed:?}"
    );
}

#[tokio::test]
async fn a_cursor_naming_no_movement_answers_an_empty_page_with_no_next_page() {
    let (_directory, db, wallet) = wallet().await;
    for (on, source) in [(100, "payout"), (101, "mint"), (102, "fine")] {
        insert_movement(&db, on, source, 5).await;
    }
    let held = wallet.movements(None).await.expect("the first page");
    assert_eq!(held.movements.len(), 3, "the ledger holds three movements");
    let none = wallet
        .movements(Some(9_999))
        .await
        .expect("a cursor no movement holds is still a read");
    assert_eq!(
        none,
        MovementsPage {
            movements: Vec::new(),
            next: None,
        }
    );
}
