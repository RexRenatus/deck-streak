//! The owner's chest and token use cases (SPEC-411 R7, R8, R10, R11; ADR-425): open a chest, pick
//! an Epic's prize, activate a double-XP token, and read the inventory. Each change runs in one
//! write, and whatever it pays is granted in that write, so a chest is never resolved unpaid.
//!
//! The answers are this context's own types, so the owner's surfaces read the chest port through
//! coordination alone and never name the quests context (docs/CONTEXT-MAP.md).

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};
use deck_streak_progression::grant::{GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::ledger::grant_on;
use deck_streak_progression::xp::XpAmount;
use deck_streak_quests::chest_store::{ChestState, Choice, StoredChest, chest, unresolved_chests};
use deck_streak_quests::chests::{
    ChestError, Opened, Payout, Settled, open_chest_on, settle_epic_choice_on,
};
use deck_streak_quests::tokens::{Activation, activate_token_on, holdings_on};
use deck_streak_streaks::freeze::FreezeReason;
use sqlx::SqliteConnection;

use crate::freeze::{FreezeGrant, admission, grant_freeze_on};

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

/// Opens chest `chest_id` at `at` (SPEC-411 R7), in one write: the open and the payout's grant,
/// on the chest's own study day with the source `chest:<id>` and the once scope. A second open
/// answers already opened and writes nothing.
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
pub async fn open_chest(db: &Db, chest_id: i64, at: UtcMillis) -> Result<OpenAnswer, KernelError> {
    let mut write = db.write().await?;
    let answer = match open_chest_on(&mut write, chest_id)
        .await
        .map_err(chest_error)?
    {
        Opened::NoSuchChest => OpenAnswer::NoSuchChest,
        Opened::AlreadyOpened => OpenAnswer::AlreadyOpened,
        Opened::ChoicePending(opened) => OpenAnswer::Opened(OpenedChest {
            chest_id: opened.id,
            rarity: opened.rarity.name(),
            xp: None,
            choice_pending: true,
        }),
        Opened::Revealed {
            chest: revealed,
            payout,
        } => {
            if let Some(payout) = payout {
                pay(&mut write, payout, at).await?;
            }
            OpenAnswer::Opened(OpenedChest {
                chest_id: revealed.id,
                rarity: revealed.rarity.name(),
                xp: Some(revealed.payout_xp),
                choice_pending: false,
            })
        }
    };
    write.commit().await?;
    Ok(answer)
}

/// Settles opened Epic `chest_id`'s `prize` on `today` at `at` (SPEC-411 R8), in one write. Whether
/// a freeze is capped is the streaks' own admission of one more freeze, read in that write; a
/// freeze the settle grants is written there by [`grant_freeze_on`], and a capped one becomes a
/// token. A second pick answers already settled and writes nothing.
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
pub async fn pick_epic_choice(
    db: &Db,
    chest_id: i64,
    prize: Prize,
    today: StudyDay,
    at: UtcMillis,
) -> Result<PickAnswer, KernelError> {
    let mut write = db.write().await?;
    if chest(&mut write, chest_id)
        .await
        .map_err(chest_error)?
        .is_none()
    {
        return Ok(PickAnswer::NoSuchChest);
    }
    let wanted = match prize {
        Prize::Token => Choice::Token,
        Prize::Freeze => Choice::Freeze,
    };
    let capped = admission(&mut write, FreezeReason::Chest, today)
        .await?
        .is_err();
    let answer = match settle_epic_choice_on(&mut write, chest_id, wanted, capped, at)
        .await
        .map_err(chest_error)?
    {
        Settled::NotAnEpic => PickAnswer::NotAnEpic,
        Settled::AlreadySettled => PickAnswer::AlreadySettled,
        Settled::Chosen {
            choice,
            capped,
            token_id,
        } => {
            if choice == Choice::Freeze
                && grant_freeze_on(&mut write, today, FreezeReason::Chest, at).await?
                    != FreezeGrant::Granted
            {
                return Err(refused(
                    "an admitted chest freeze was refused in its own write",
                ));
            }
            PickAnswer::Chosen {
                prize: prize_of(choice),
                capped,
                token_id,
            }
        }
    };
    write.commit().await?;
    Ok(answer)
}

/// Activates the oldest held token at `now` (SPEC-411 R10), in one write. A second activation
/// while a window is open answers already active and opens no second window.
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
pub async fn activate_token(db: &Db, now: UtcMillis) -> Result<ActivateAnswer, KernelError> {
    let mut write = db.write().await?;
    let answer = match activate_token_on(&mut write, now)
        .await
        .map_err(chest_error)?
    {
        Activation::Activated { token_id, window } => ActivateAnswer::Activated {
            token_id,
            window_ends_at: window.end,
        },
        Activation::AlreadyActive => ActivateAnswer::AlreadyActive,
        Activation::NoneHeld => ActivateAnswer::NoneHeld,
    };
    write.commit().await?;
    Ok(answer)
}

/// The owner's unresolved chests, the tokens held and the window open at `now` (SPEC-411 R11). A
/// sealed or vaulted chest's rarity and payout are never sent: the open is the reveal.
///
/// # Errors
///
/// [`KernelError`] when a read fails.
pub async fn chest_inventory(db: &Db, now: UtcMillis) -> Result<Inventory, KernelError> {
    let mut write = db.write().await?;
    let chests = unresolved_chests(&mut write)
        .await
        .map_err(chest_error)?
        .into_iter()
        .map(inventory_chest)
        .collect();
    let holdings = holdings_on(&mut write, now).await.map_err(chest_error)?;
    write.rollback().await?;
    Ok(Inventory {
        chests,
        tokens_held: holdings.held,
        window_ends_at: holdings.window_ends_at,
    })
}

/// One chest as the inventory shows it: no rarity and no payout while it is sealed or vaulted.
fn inventory_chest(stored: StoredChest) -> InventoryChest {
    let sealed = matches!(stored.state, ChestState::Sealed | ChestState::Vaulted);
    InventoryChest {
        chest_id: stored.id,
        state: stored.state.name(),
        study_day: stored.study_day,
        rarity: (!sealed).then(|| stored.rarity.name()),
        xp: (!sealed).then_some(stored.payout_xp),
    }
}

/// The prize a settled choice paid.
const fn prize_of(choice: Choice) -> Prize {
    match choice {
        Choice::Token => Prize::Token,
        Choice::Freeze => Prize::Freeze,
    }
}

/// Grants a chest's `payout` through progression's grant port in the caller's `write` (SPEC-411
/// R5, R7): on the chest's own study day and track, with the source `chest:<id>` and the once
/// scope, so a replay of the write that resolved the chest grants nothing more.
///
/// # Errors
///
/// [`KernelError`] when the payout is not a grant or the write fails.
pub(crate) async fn pay(
    write: &mut SqliteConnection,
    payout: Payout,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let source = GrantSource::new(&payout.source()).map_err(|error| refused(&error.to_string()))?;
    let amount =
        u32::try_from(payout.xp).map_err(|_| refused("a chest's payout is not a grant amount"))?;
    let request = GrantRequest {
        study_day: payout.study_day,
        source,
        track: payout.track,
        amount: XpAmount::new(amount),
        scope: GrantScope::Once,
    };
    let _ = grant_on(write, &request, at).await?;
    Ok(())
}

/// The chest port's error as this context's: the database's own, or a refusal naming its rule.
pub(crate) fn chest_error(error: ChestError) -> KernelError {
    match error {
        ChestError::Database(error) => KernelError::Database(error),
        other => refused(&other.to_string()),
    }
}

/// A refusal that names its rule, as the other use cases spell one.
fn refused(reason: &str) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(reason.to_owned()))
}
