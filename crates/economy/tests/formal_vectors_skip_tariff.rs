//! The skip tariff answers every vector the Lean port writes (#108): for each input in
//! `formal/vectors/skip-tariff.jsonl`, written by `formal/lean/Formal/SkipTariff.lean`'s port,
//! `price` gives the same price; the wallet's floor-clipped debit of that price, over a wallet
//! holding the vector's balance, pays the same coins and leaves the skip unfunded the same way; and
//! the refund of what `paid_on` reads back credits the same coins. These are the economy parts
//! coordination's settlement and undo compose, under the same answers they map.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_economy::tariff::{REFUND_SOURCE, TARIFF_SOURCE, ladder, paid_on, price};
use deck_streak_economy::wallet::{DebitAnswer, DepositAnswer, SqliteWallet};
use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use serde_json::Value;
use tempfile::TempDir;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/skip-tariff.jsonl");
/// The skip's study day; the refund lands on the next.
const DAY: i64 = 20_105;
/// The one skip's reference.
const REFERENCE: &str = "1";

/// A vector field as a whole number.
fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

/// The count of a vector's earlier skips.
fn earlier(vector: &Value) -> usize {
    usize::try_from(number(vector, "earlier")).expect("a count of 0 or more")
}

/// Noon UTC of study day `day`.
fn noon(day: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * 86_400_000 + 12 * 3_600_000)
}

/// A wallet over a fresh database in `scratch`, holding `balance` coins.
async fn holding(scratch: &TempDir, balance: i64) -> (Db, SqliteWallet) {
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let wallet = SqliteWallet::new(db.clone());
    let seeded = wallet
        .deposit(
            StudyDay::from_epoch_day(DAY - 40),
            "seed",
            "balance",
            balance,
            noon(DAY - 40),
        )
        .await
        .expect("the seed deposit");
    assert!(
        matches!(
            seeded,
            DepositAnswer::Deposited(_) | DepositAnswer::NotPositive
        ),
        "{seeded:?}"
    );
    (db, wallet)
}

/// Charges `charged` through the floor-clipped debit and answers what was paid, as the settlement
/// maps the wallet's answer.
async fn charge(wallet: &SqliteWallet, charged: i64) -> i64 {
    let answer = wallet
        .debit_floored(
            StudyDay::from_epoch_day(DAY),
            TARIFF_SOURCE,
            REFERENCE,
            charged,
            noon(DAY),
        )
        .await
        .expect("the debit");
    match answer {
        DebitAnswer::Debited { paid, .. } | DebitAnswer::AlreadyDebited { paid } => paid,
        DebitAnswer::NothingRequested => 0,
    }
}

/// What the ledger says the skip paid.
async fn read_paid(db: &Db) -> i64 {
    let mut reader = db.reader().acquire().await.expect("a reader");
    paid_on(&mut reader, REFERENCE)
        .await
        .expect("the paid read")
}

#[tokio::test]
async fn the_tariff_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "SkipTariff", "{header}");
    assert_eq!(header["covers"], "crates/economy/src/tariff.rs", "{header}");
    let (mut prices, mut charges, mut refunds) = (0_u64, 0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        match vector["rule"].as_str() {
            Some("price") => {
                assert_eq!(
                    price(ladder(), earlier(&vector)),
                    number(&vector, "price"),
                    "{vector}"
                );
                prices += 1;
            }
            Some("settle_applied") => {
                let scratch = TempDir::new().expect("a scratch directory");
                let (db, wallet) = holding(&scratch, number(&vector, "balance")).await;
                let asked = price(ladder(), earlier(&vector));
                let paid = charge(&wallet, asked).await;
                let unfunded = vector["unfunded"]
                    .as_bool()
                    .unwrap_or_else(|| panic!("an unfunded flag in {vector}"));
                assert_eq!(
                    (asked, paid, paid < asked),
                    (number(&vector, "price"), number(&vector, "paid"), unfunded),
                    "{vector}"
                );
                assert_eq!(read_paid(&db).await, paid, "the ledger holds it: {vector}");
                charges += 1;
            }
            Some("settle_undone") => {
                let scratch = TempDir::new().expect("a scratch directory");
                let paid = number(&vector, "paid");
                let (db, wallet) = holding(&scratch, paid).await;
                assert_eq!(charge(&wallet, paid).await, paid, "{vector}");
                let owed = read_paid(&db).await;
                let answer = wallet
                    .refund(
                        StudyDay::from_epoch_day(DAY + 1),
                        REFUND_SOURCE,
                        REFERENCE,
                        owed,
                        noon(DAY + 1),
                    )
                    .await
                    .expect("the refund");
                let refunded = match answer {
                    DepositAnswer::Deposited(coins) => coins,
                    DepositAnswer::AlreadyDeposited | DepositAnswer::NotPositive => 0,
                };
                assert_eq!(refunded, number(&vector, "refunded"), "{vector}");
                refunds += 1;
            }
            _ => panic!("a vector of a known rule: {vector}"),
        }
    }
    println!("examined {prices} price, {charges} charge and {refunds} refund vector(s)");
    assert_eq!(
        (prices, charges, refunds),
        (7, 60, 10),
        "every input the writer prints"
    );
    assert_eq!(
        header["vectors"].as_u64(),
        Some(prices + charges + refunds),
        "{header}"
    );
}

#[tokio::test]
async fn the_recorded_counterexamples_answer_as_proved() {
    // `a_price_summed_over_the_ladder_leaves_it`: the third skip of a month costs the ladder's
    // 100, not the 150 its steps sum to.
    assert_eq!(price(ladder(), 2), 100);
    // `a_price_with_no_clamp_falls_past_the_ladders_end` and `..._stops_repeating`: the fourth
    // skip costs the last price again, not the 0 an unclamped read finds.
    assert_eq!(price(ladder(), 3), 100);
    // `a_charge_that_skips_the_wallet_overpays` and `a_refund_of_the_price_returns_more_than_was_
    // paid`: the second skip (50) over a wallet of 20 pays 20, and its undo refunds 20, not 50.
    let scratch = TempDir::new().expect("a scratch directory");
    let (db, wallet) = holding(&scratch, 20).await;
    assert_eq!(charge(&wallet, price(ladder(), 1)).await, 20);
    assert_eq!(read_paid(&db).await, 20);
}
