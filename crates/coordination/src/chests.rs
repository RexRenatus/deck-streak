//! The owner's chest and token use cases (SPEC-411 R7, R8, R10, R11; ADR-425): open a chest, pick
//! an Epic's prize, activate a double-XP token, and read the inventory. Each change runs in one
//! write, and whatever it pays is granted in that write, so a chest is never resolved unpaid.
//!
//! The answers are this context's own types, so the owner's surfaces read the chest port through
//! coordination alone and never name the quests context (docs/CONTEXT-MAP.md).

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

/// What opening a chest answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAnswer {
    /// No chest holds the id.
    NoSuchChest,
    /// The chest was opened before: nothing changed and nothing was paid.
    AlreadyOpened,
    /// The chest is opened now.
    Opened(OpenedChest),
}

/// A chest the open revealed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenedChest {
    /// The chest's id.
    pub chest_id: i64,
    /// Its stored rarity's name.
    pub rarity: &'static str,
    /// The XP it paid, none while an Epic's choice is pending.
    pub xp: Option<i64>,
    /// Whether the chest is an Epic whose prize waits for the owner's pick.
    pub choice_pending: bool,
}

/// An Epic's prize, as the owner picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prize {
    /// A double-XP token.
    Token,
    /// A streak freeze.
    Freeze,
}

impl Prize {
    /// The prize's name as the owner's surfaces write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Freeze => "freeze",
        }
    }
}

/// What an Epic's pick answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickAnswer {
    /// No chest holds the id.
    NoSuchChest,
    /// The chest is not an Epic.
    NotAnEpic,
    /// The Epic is not opened, or its prize was picked before: nothing changed.
    AlreadySettled,
    /// The prize is settled and paid.
    Chosen {
        /// The prize paid: a freeze the caps refused becomes a token.
        prize: Prize,
        /// Whether a freeze was asked for and refused by a cap.
        capped: bool,
        /// The token granted, when the prize is one.
        token_id: Option<i64>,
    },
}

/// What a token's activation answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivateAnswer {
    /// The oldest held token is activated.
    Activated {
        /// The token activated.
        token_id: i64,
        /// Its window's end.
        window_ends_at: UtcMillis,
    },
    /// A window is open already: nothing changed.
    AlreadyActive,
    /// No token is held: nothing changed.
    NoneHeld,
}

/// One unresolved chest as the inventory shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryChest {
    /// The chest's id.
    pub chest_id: i64,
    /// Its state's name.
    pub state: &'static str,
    /// Its study day.
    pub study_day: StudyDay,
    /// Its rarity's name, none while it is sealed or vaulted.
    pub rarity: Option<&'static str>,
    /// Its payout, none while it is sealed or vaulted.
    pub xp: Option<i64>,
}

/// The owner's chests and tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    /// The unresolved chests, oldest first.
    pub chests: Vec<InventoryChest>,
    /// The tokens held.
    pub tokens_held: i64,
    /// The open window's end, none when no window is open.
    pub window_ends_at: Option<UtcMillis>,
}

/// Opens chest `chest_id` at `at` (SPEC-411 R7).
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
pub async fn open_chest(db: &Db, chest_id: i64, at: UtcMillis) -> Result<OpenAnswer, KernelError> {
    let _ = (db, chest_id, at);
    Ok(OpenAnswer::NoSuchChest)
}

/// Settles opened Epic `chest_id`'s `prize` on `today` at `at` (SPEC-411 R8).
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
pub async fn pick_epic_choice(
    db: &Db,
    chest_id: i64,
    prize: Prize,
    today: StudyDay,
    at: UtcMillis,
) -> Result<PickAnswer, KernelError> {
    let _ = (db, chest_id, prize, today, at);
    Ok(PickAnswer::NoSuchChest)
}

/// Activates the oldest held token at `now` (SPEC-411 R10).
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
pub async fn activate_token(db: &Db, now: UtcMillis) -> Result<ActivateAnswer, KernelError> {
    let _ = (db, now);
    Ok(ActivateAnswer::NoneHeld)
}

/// The owner's unresolved chests, the tokens held and the window open at `now` (SPEC-411 R11).
///
/// # Errors
///
/// [`KernelError`] when a read fails.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
pub async fn chest_inventory(db: &Db, now: UtcMillis) -> Result<Inventory, KernelError> {
    let _ = (db, now);
    Ok(Inventory::default())
}
