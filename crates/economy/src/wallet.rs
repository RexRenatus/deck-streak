//! The coin wallet (SPEC-082 R1, R2, R5, R7, R8; ADR-308). RED-2 STUB: every port answers without
//! touching the database, so each port test is red by assertion until GREEN implements it.

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

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

    /// The balance: the sum of every movement (R2).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn balance(&self) -> Result<i64, KernelError> {
        let _ = &self.db;
        Ok(0)
    }

    /// The wallet at `day`'s start: the sum of the movements of earlier study days (R5).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn balance_before(&self, day: StudyDay) -> Result<i64, KernelError> {
        let _ = (&self.db, day);
        Ok(0)
    }

    /// The coins debited on `day`: every negative movement of the day, purchases included (R5, R8).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn debits_for_day(&self, day: StudyDay) -> Result<i64, KernelError> {
        let _ = (&self.db, day);
        Ok(0)
    }

    /// Credits `amount` once on its key (R7).
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
        let _ = (&self.db, day, source, reference, amount, at);
        Ok(CreditAnswer::AlreadyCredited)
    }

    /// Credits `amount` unless a movement of `source` and `reference` exists on any study day (R7).
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
        let _ = (&self.db, day, source, reference, amount, at);
        Ok(CreditAnswer::AlreadyCredited)
    }

    /// Settles `day`'s mint at `amount` (R4).
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
        let _ = (&self.db, day, amount, closed, at);
        Ok(MintAnswer::Settled(0))
    }

    /// Buys for `price` on its key, refused below the price (R7, R8).
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
        let _ = (&self.db, day, source, reference, at);
        Ok(PurchaseAnswer::Refused(PurchaseRefused::NotPositive {
            price,
        }))
    }

    /// Debits at most what the wallet holds above the floor, and never refuses (R7).
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
        let _ = (&self.db, day, source, reference, amount, at);
        Ok(DebitAnswer::NothingRequested)
    }

    /// Refunds `amount` once on its key (R7).
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
        let _ = (&self.db, day, source, reference, amount, at);
        Ok(CreditAnswer::AlreadyCredited)
    }

    /// Debits `requested`, clipped to the wallet and the day's loss cap (R5, R7).
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
        let _ = (&self.db, day, source, reference, requested, at);
        Ok(DebitAnswer::NothingRequested)
    }
}
