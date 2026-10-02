//! The coin wallet (SPEC-082 R1, R2, R5, R7, R8; ADR-308): one `coin_ledger` row per coin
//! movement, owned by this context (docs/CONTEXT-MAP.md), written only through the ports below and
//! summed for the balance each time it is read. No column stores the balance.
//!
//! Every port is one read and one write inside ONE `BEGIN IMMEDIATE` transaction from the kernel's
//! base: the repository's method opens it and commits it, and the `_on` form runs inside a caller's
//! own write (the fold, the shop), so no port reads outside the write lock (ADR-308 ruling 1). The
//! key, one movement per (study day, source, reference), is the migration's unique index, and every
//! insert does nothing on that conflict (ruling 3). No port deletes a movement; `settle_mint` alone
//! updates one, the day's mint (ruling 6). The predecessor's wallet is `gamification/economy.py` and
//! `database.py:GamifyStore` at `27ee2bc`.

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};
use sqlx::{Executor, Sqlite, Transaction};

use crate::constants::WALLET_FLOOR;
use crate::rules::{clip_debit, daily_loss_cap};

/// The source of a study day's mint movement (R3): one per day, with an empty reference.
pub const MINT_SOURCE: &str = "mint";
/// The reference of a study day's mint movement.
pub const MINT_REFERENCE: &str = "";

/// A credit's answer (R7).
#[must_use = "a credit's answer says whether coins moved"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditAnswer {
    /// The movement was written: these coins were credited.
    Credited(i64),
    /// A movement already holds the key, or for `credit_once` the source and reference on any study
    /// day: nothing was written.
    AlreadyCredited,
    /// The amount was 0 or less: nothing was written.
    NotPositive,
}

/// Why a purchase was refused. Nothing was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PurchaseRefused {
    /// The price was 0 or less.
    #[error("a price must be positive, and {price} is not")]
    NotPositive {
        /// The price asked.
        price: i64,
    },
    /// The purchase would leave the balance below the floor (R2).
    #[error("the wallet holds {balance} coins, below the price of {price} and the floor")]
    BelowPrice {
        /// The balance read in the purchase's write.
        balance: i64,
        /// The price asked.
        price: i64,
    },
}

/// A purchase's verdict (R7, R8).
#[must_use = "a purchase's verdict says whether it was bought"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseAnswer {
    /// The movement was written: the price left the wallet.
    Bought(i64),
    /// A movement already holds the key: nothing was written, and the held purchase paid this.
    AlreadyBought(i64),
    /// The purchase was refused, and nothing was written.
    Refused(PurchaseRefused),
}

/// A debit's answer (R5, R7).
#[must_use = "a debit's answer says what it paid"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebitAnswer {
    /// The movement was written: `paid` coins left the wallet, and `forgiven` says whether any of
    /// the request was clipped.
    Debited {
        /// The coins debited.
        paid: i64,
        /// Whether the clip forgave part of the request.
        forgiven: bool,
    },
    /// A movement already holds the key: nothing was written, and the held debit paid `paid`.
    AlreadyDebited {
        /// The coins the held debit paid.
        paid: i64,
    },
    /// The request was 0 or less: nothing was written.
    NothingRequested,
}

/// A mint settlement's answer (R4).
#[must_use = "a mint's answer says what the day holds"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MintAnswer {
    /// The day's mint after the settlement: 0 when none is held.
    Settled(i64),
    /// The amount was negative: nothing was written.
    Negative,
}

/// The coin ledger in the service's own database: the wallet's ports.
#[derive(Clone, Debug)]
pub struct SqliteWallet {
    db: Db,
}

impl SqliteWallet {
    /// The wallet over `db`, whose migrations created the ledger.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// The balance: the sum of every movement (R2), read on the pool for display. A port that
    /// decides on the balance reads it inside its own write instead.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn balance(&self) -> Result<i64, KernelError> {
        Ok(summed(self.db.reader()).await?)
    }

    /// The wallet at `day`'s start: the sum of the movements of earlier study days (R5; the golden
    /// of `coin_balance_before`).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn balance_before(&self, day: StudyDay) -> Result<i64, KernelError> {
        Ok(summed_before(self.db.reader(), day).await?)
    }

    /// The coins debited on `day`: every negative movement of the day, purchases included (R5, R8;
    /// the golden of `coin_debits_for_day`).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn debits_for_day(&self, day: StudyDay) -> Result<i64, KernelError> {
        Ok(debited_on_day(self.db.reader(), day).await?)
    }

    /// Credits `amount` once on its key, in one write ([`credit_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn credit(
        &self,
        day: StudyDay,
        source: &str,
        reference: &str,
        amount: i64,
        at: UtcMillis,
    ) -> Result<CreditAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = credit_on(&mut write, day, source, reference, amount, at).await?;
        write.commit().await?;
        Ok(answer)
    }

    /// Credits `amount` unless a movement of `source` and `reference` exists on any study day, in
    /// one write ([`credit_once_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn credit_once(
        &self,
        day: StudyDay,
        source: &str,
        reference: &str,
        amount: i64,
        at: UtcMillis,
    ) -> Result<CreditAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = credit_once_on(&mut write, day, source, reference, amount, at).await?;
        write.commit().await?;
        Ok(answer)
    }

    /// Settles `day`'s mint at `amount`, in one write ([`settle_mint_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn settle_mint(
        &self,
        day: StudyDay,
        amount: i64,
        closed: bool,
        at: UtcMillis,
    ) -> Result<MintAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = settle_mint_on(&mut write, day, amount, closed, at).await?;
        write.commit().await?;
        Ok(answer)
    }

    /// Buys for `price` on its key, in one write ([`purchase_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn purchase(
        &self,
        day: StudyDay,
        source: &str,
        reference: &str,
        price: i64,
        at: UtcMillis,
    ) -> Result<PurchaseAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = purchase_on(&mut write, day, source, reference, price, at).await?;
        write.commit().await?;
        Ok(answer)
    }

    /// Debits at most what the wallet holds above the floor, in one write ([`debit_floored_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn debit_floored(
        &self,
        day: StudyDay,
        source: &str,
        reference: &str,
        amount: i64,
        at: UtcMillis,
    ) -> Result<DebitAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = debit_floored_on(&mut write, day, source, reference, amount, at).await?;
        write.commit().await?;
        Ok(answer)
    }

    /// Refunds `amount` once on its key, in one write ([`refund_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn refund(
        &self,
        day: StudyDay,
        source: &str,
        reference: &str,
        amount: i64,
        at: UtcMillis,
    ) -> Result<CreditAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = refund_on(&mut write, day, source, reference, amount, at).await?;
        write.commit().await?;
        Ok(answer)
    }

    /// Debits `requested`, clipped to the wallet and the day's loss cap, in one write
    /// ([`debit_capped_on`]).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a statement fails.
    pub async fn debit_capped(
        &self,
        day: StudyDay,
        source: &str,
        reference: &str,
        requested: i64,
        at: UtcMillis,
    ) -> Result<DebitAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = debit_capped_on(&mut write, day, source, reference, requested, at).await?;
        write.commit().await?;
        Ok(answer)
    }
}

/// Credits `amount` on (`day`, `source`, `reference`) inside the caller's write: written once, and a
/// second credit of the key writes nothing (R7). The insert's own conflict with the ledger's unique
/// index is the existence check, so the key lives in the migration alone (ADR-308 ruling 3).
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn credit_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    amount: i64,
    at: UtcMillis,
) -> Result<CreditAnswer, KernelError> {
    if amount <= 0 {
        return Ok(CreditAnswer::NotPositive);
    }
    Ok(
        if insert(write, day, source, reference, amount, at).await? {
            CreditAnswer::Credited(amount)
        } else {
            CreditAnswer::AlreadyCredited
        },
    )
}

/// Credits `amount` inside the caller's write unless a movement of `source` and `reference` exists
/// on any study day: the predecessor's cross-day guard (`database.py:GamifyStore.coin_ref_exists`)
/// for a payout settled on a later day, read in the same transaction as the insert (ADR-308 ruling
/// 5).
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn credit_once_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    amount: i64,
    at: UtcMillis,
) -> Result<CreditAnswer, KernelError> {
    if amount <= 0 {
        return Ok(CreditAnswer::NotPositive);
    }
    let ever = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM coin_ledger WHERE source = ?1 AND reference = ?2)
               AS "ever!: bool""#,
        source,
        reference
    )
    .fetch_one(&mut **write)
    .await?;
    if ever {
        return Ok(CreditAnswer::AlreadyCredited);
    }
    credit_on(write, day, source, reference, amount, at).await
}

/// Settles `day`'s one mint movement (`mint`, an empty reference) at `amount` inside the caller's
/// write (R4; ADR-308 ruling 6). With none held it is inserted when positive. On a day the caller
/// reports `closed` it becomes the greater of the held and the new mint, so a settled day's mint is
/// raised and never lowered. On the open day it follows the new mint, lowered no further than the
/// balance above the floor (R2).
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn settle_mint_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    amount: i64,
    closed: bool,
    at: UtcMillis,
) -> Result<MintAnswer, KernelError> {
    if amount < 0 {
        return Ok(MintAnswer::Negative);
    }
    let Some(held) = held(write, day, MINT_SOURCE, MINT_REFERENCE).await? else {
        if amount > 0 {
            insert(write, day, MINT_SOURCE, MINT_REFERENCE, amount, at).await?;
        }
        return Ok(MintAnswer::Settled(amount));
    };
    let settled = if closed || amount >= held {
        held.max(amount)
    } else {
        let room = (summed(&mut **write).await? - WALLET_FLOOR).max(0);
        amount.max(held - room)
    };
    if settled != held {
        let epoch_day = day.epoch_day();
        sqlx::query!(
            "UPDATE coin_ledger SET delta = ?1 \
             WHERE study_day = ?2 AND source = ?3 AND reference = ?4",
            settled,
            epoch_day,
            MINT_SOURCE,
            MINT_REFERENCE
        )
        .execute(&mut **write)
        .await?;
    }
    Ok(MintAnswer::Settled(settled))
}

/// Buys for `price` on (`day`, `source`, `reference`) inside the caller's write (R7, R8): refused,
/// with nothing written, when the price is 0 or less or the balance less the price would fall below
/// the floor. A purchase is never clipped by the loss cap, and its movement counts toward the day's
/// debits.
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn purchase_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    price: i64,
    at: UtcMillis,
) -> Result<PurchaseAnswer, KernelError> {
    if price <= 0 {
        return Ok(PurchaseAnswer::Refused(PurchaseRefused::NotPositive {
            price,
        }));
    }
    if let Some(delta) = held(write, day, source, reference).await? {
        return Ok(PurchaseAnswer::AlreadyBought(-delta));
    }
    let balance = summed(&mut **write).await?;
    if balance - price < WALLET_FLOOR {
        return Ok(PurchaseAnswer::Refused(PurchaseRefused::BelowPrice {
            balance,
            price,
        }));
    }
    insert(write, day, source, reference, -price, at).await?;
    Ok(PurchaseAnswer::Bought(price))
}

/// Debits `amount` on (`day`, `source`, `reference`) inside the caller's write, paying
/// `min(amount, max(0, balance - floor))` and never refusing (R7: the skip day's tariff). A positive
/// request writes its movement even when it pays nothing, so a retry finds the key.
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn debit_floored_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    amount: i64,
    at: UtcMillis,
) -> Result<DebitAnswer, KernelError> {
    if amount <= 0 {
        return Ok(DebitAnswer::NothingRequested);
    }
    if let Some(delta) = held(write, day, source, reference).await? {
        return Ok(DebitAnswer::AlreadyDebited { paid: -delta });
    }
    let room = (summed(&mut **write).await? - WALLET_FLOOR).max(0);
    let paid = amount.min(room);
    insert(write, day, source, reference, -paid, at).await?;
    Ok(DebitAnswer::Debited {
        paid,
        forgiven: paid < amount,
    })
}

/// Refunds `amount` on (`day`, `source`, `reference`) inside the caller's write: a positive
/// movement, written once on its key as a credit is (R7).
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn refund_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    amount: i64,
    at: UtcMillis,
) -> Result<CreditAnswer, KernelError> {
    credit_on(write, day, source, reference, amount, at).await
}

/// Debits `requested` on (`day`, `source`, `reference`) inside the caller's write, clipped by
/// `clip_debit` to the balance above the floor and to the day's loss cap less every coin already
/// debited that day, purchases included (R5, R8). It reports whether any part was forgiven, and a
/// positive request writes its movement even when it pays nothing.
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn debit_capped_on(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    requested: i64,
    at: UtcMillis,
) -> Result<DebitAnswer, KernelError> {
    if requested <= 0 {
        return Ok(DebitAnswer::NothingRequested);
    }
    if let Some(delta) = held(write, day, source, reference).await? {
        return Ok(DebitAnswer::AlreadyDebited { paid: -delta });
    }
    let balance = summed(&mut **write).await?;
    let start = summed_before(&mut **write, day).await?;
    let debited = debited_on_day(&mut **write, day).await?;
    let remainder = daily_loss_cap(start) - debited;
    let (paid, forgiven) = clip_debit(requested, balance - WALLET_FLOOR, remainder);
    insert(write, day, source, reference, -paid, at).await?;
    Ok(DebitAnswer::Debited { paid, forgiven })
}

/// Inserts one movement on its key, writing nothing on the unique index's conflict. Whether a row
/// was written.
async fn insert(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
    delta: i64,
    at: UtcMillis,
) -> Result<bool, sqlx::Error> {
    let epoch_day = day.epoch_day();
    let at = at.epoch_millis();
    let written = sqlx::query!(
        "INSERT INTO coin_ledger (study_day, source, reference, delta, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (study_day, source, reference) DO NOTHING",
        epoch_day,
        source,
        reference,
        delta,
        at
    )
    .execute(&mut **write)
    .await?
    .rows_affected();
    Ok(written == 1)
}

/// The delta the key holds, if a movement holds it.
async fn held(
    write: &mut Transaction<'_, Sqlite>,
    day: StudyDay,
    source: &str,
    reference: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let epoch_day = day.epoch_day();
    sqlx::query_scalar!(
        "SELECT delta FROM coin_ledger WHERE study_day = ?1 AND source = ?2 AND reference = ?3",
        epoch_day,
        source,
        reference
    )
    .fetch_optional(&mut **write)
    .await
}

/// The sum of every movement.
async fn summed<'c>(executor: impl Executor<'c, Database = Sqlite>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT COALESCE(SUM(delta), 0) AS "total!: i64" FROM coin_ledger"#)
        .fetch_one(executor)
        .await
}

/// The sum of the movements of the study days before `day`.
async fn summed_before<'c>(
    executor: impl Executor<'c, Database = Sqlite>,
    day: StudyDay,
) -> Result<i64, sqlx::Error> {
    let epoch_day = day.epoch_day();
    sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(delta), 0) AS "total!: i64" FROM coin_ledger
               WHERE study_day < ?1"#,
        epoch_day
    )
    .fetch_one(executor)
    .await
}

/// The coins debited on `day`: the sum of the day's negative movements, as a positive number.
async fn debited_on_day<'c>(
    executor: impl Executor<'c, Database = Sqlite>,
    day: StudyDay,
) -> Result<i64, sqlx::Error> {
    let epoch_day = day.epoch_day();
    sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(-delta), 0) AS "total!: i64" FROM coin_ledger
               WHERE study_day = ?1 AND delta < 0"#,
        epoch_day
    )
    .fetch_one(executor)
    .await
}
