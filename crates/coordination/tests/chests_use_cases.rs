//! The owner's chest and token use cases (SPEC-411 A8 to A12; R7, R8, R10, R11): an open pays once
//! in the write that opens the chest, an Epic's freeze pick writes its freeze in the pick's write
//! and becomes a token at the cap, the inventory withholds a sealed or vaulted chest's rarity, and a
//! second activation opens no second window. Every chest, token, day and instant is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_coordination::chests::{
    ActivateAnswer, Inventory, InventoryChest, OpenAnswer, OpenedChest, PickAnswer, Prize,
    activate_token, chest_inventory, open_chest, pick_epic_choice,
};
use deck_streak_coordination::freeze::{FreezeGrant, grant_freeze};
use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_streaks::freeze::{FreezeEvent, FreezeReason};
use deck_streak_streaks::store;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// The study day the owner acts on.
const TODAY: i64 = 20_000;
/// Noon UTC of [`TODAY`], the instant every use case runs at.
const NOW: i64 = TODAY * DAY_MS + 12 * HOUR_MS;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn now() -> UtcMillis {
    UtcMillis::from_epoch_millis(NOW)
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// Stores a session chest of `study_day` with its rarity, payout and state; its id.
async fn seed_chest(
    db: &Db,
    study_day: i64,
    start: i64,
    rarity: &str,
    xp: i64,
    state: &str,
) -> i64 {
    let mut write = db.write().await.expect("a write");
    let id = sqlx::query(
        "INSERT INTO chests (study_day, origin, session_start, rarity, payout_xp, state, \
         created_at) VALUES (?1, 'session', ?2, ?3, ?4, ?5, 0)",
    )
    .bind(study_day)
    .bind(start)
    .bind(rarity)
    .bind(xp)
    .bind(state)
    .execute(&mut *write)
    .await
    .expect("a chest stored")
    .last_insert_rowid();
    write.commit().await.expect("the chest committed");
    id
}

/// Stores a token of chest `chest_id`, granted at `granted`, its window and spend as given.
async fn seed_token(
    db: &Db,
    chest_id: i64,
    granted: i64,
    activated: i64,
    ends: i64,
    consumed: i64,
) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO xp_tokens (chest_id, granted_at, activated_at, window_ends_at, consumed, \
         created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?2)",
    )
    .bind(chest_id)
    .bind(granted)
    .bind(activated)
    .bind(ends)
    .bind(consumed)
    .execute(&mut *write)
    .await
    .expect("a token stored");
    write.commit().await.expect("the token committed");
}

/// A ledger grant: (study day, source, track, amount, scope).
type GrantRow = (i64, String, String, i64, String);
/// A stored chest's state and choice.
type ChestRow = (i64, String, String);
/// A stored token: (id, chest id, activated at, window ends at, consumed).
type TokenRow = (i64, i64, i64, i64, i64);

async fn grants(db: &Db) -> Vec<GrantRow> {
    let mut write = db.write().await.expect("a write");
    sqlx::query_as("SELECT study_day, source, track, amount, scope FROM xp_ledger ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the ledger reads")
}

async fn chests(db: &Db) -> Vec<ChestRow> {
    let mut write = db.write().await.expect("a write");
    sqlx::query_as("SELECT id, state, choice FROM chests ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the chests read")
}

async fn tokens(db: &Db) -> Vec<TokenRow> {
    let mut write = db.write().await.expect("a write");
    sqlx::query_as(
        "SELECT id, chest_id, activated_at, window_ends_at, consumed FROM xp_tokens ORDER BY id",
    )
    .fetch_all(&mut *write)
    .await
    .expect("the tokens read")
}

/// The freeze events and the language row's freezes held, none before any row is written.
async fn freezes(db: &Db) -> (Vec<FreezeEvent>, Option<u32>) {
    let mut write = db.write().await.expect("a write");
    let events = store::events(&mut write).await.expect("the events read");
    let held = store::state(&mut write, "language")
        .await
        .expect("the streak reads")
        .map(|state| state.freezes);
    (events, held)
}

fn grant(study_day: i64, source: &str, amount: i64) -> GrantRow {
    (
        study_day,
        source.to_owned(),
        "language".to_owned(),
        amount,
        "once".to_owned(),
    )
}

fn chest_row(id: i64, state: &str, choice: &str) -> ChestRow {
    (id, state.to_owned(), choice.to_owned())
}

const fn chest_freeze(study_day: i64) -> FreezeEvent {
    FreezeEvent {
        day: StudyDay::from_epoch_day(study_day),
        delta: 1,
        reason: FreezeReason::Chest,
    }
}

#[tokio::test]
async fn opening_a_chest_pays_once_in_the_write_that_opens_it() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let id = seed_chest(
        &db,
        TODAY - 1,
        TODAY * DAY_MS - HOUR_MS,
        "rare",
        45,
        "sealed",
    )
    .await;
    let first = open_chest(&db, id, now()).await.expect("the open runs");
    let second = open_chest(&db, id, now()).await.expect("the replay runs");
    assert_eq!(
        (first, second, grants(&db).await, chests(&db).await),
        (
            OpenAnswer::Opened(OpenedChest {
                chest_id: 1,
                rarity: "rare",
                xp: Some(45),
                choice_pending: false,
            }),
            OpenAnswer::AlreadyOpened,
            vec![grant(TODAY - 1, "chest:1", 45)],
            vec![chest_row(1, "resolved", "")],
        ),
        "the open reveals the stored Rare and pays its 45 once, on the chest's own day"
    );
}

#[tokio::test]
async fn an_epic_freeze_pick_grants_one_freeze_in_the_write_that_settles_it() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let id = seed_chest(
        &db,
        TODAY - 1,
        TODAY * DAY_MS - HOUR_MS,
        "epic",
        0,
        "opened",
    )
    .await;
    let before = freezes(&db).await;
    let first = pick_epic_choice(&db, id, Prize::Freeze, day(TODAY), now())
        .await
        .expect("the pick runs");
    let second = pick_epic_choice(&db, id, Prize::Freeze, day(TODAY), now())
        .await
        .expect("the replay runs");
    assert_eq!(
        (
            before,
            first,
            second,
            freezes(&db).await,
            chests(&db).await,
            tokens(&db).await
        ),
        (
            (vec![], None),
            PickAnswer::Chosen {
                prize: Prize::Freeze,
                capped: false,
                token_id: None,
            },
            PickAnswer::AlreadySettled,
            (vec![chest_freeze(TODAY)], Some(2)),
            vec![chest_row(1, "resolved", "freeze")],
            vec![],
        ),
        "the pick writes one chest freeze beside the start's one, and the replay writes nothing"
    );
}

#[tokio::test]
async fn a_freeze_pick_at_the_cap_becomes_a_token_and_grants_no_freeze() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    // This month's one dropped freeze, so a second chest freeze is over the monthly cap.
    let seeded = grant_freeze(&db, day(TODAY), FreezeReason::Chest, now())
        .await
        .expect("the seed freeze is granted");
    let id = seed_chest(
        &db,
        TODAY - 1,
        TODAY * DAY_MS - HOUR_MS,
        "epic",
        0,
        "opened",
    )
    .await;
    let pick = pick_epic_choice(&db, id, Prize::Freeze, day(TODAY), now())
        .await
        .expect("the pick runs");
    assert_eq!(
        (
            seeded,
            pick,
            freezes(&db).await,
            chests(&db).await,
            tokens(&db).await
        ),
        (
            FreezeGrant::Granted,
            PickAnswer::Chosen {
                prize: Prize::Token,
                capped: true,
                token_id: Some(1),
            },
            (vec![chest_freeze(TODAY)], Some(2)),
            vec![chest_row(1, "resolved", "token")],
            vec![(1, 1, 0, 0, 0)],
        ),
        "the capped freeze becomes the chest's one token, and the freezes stay as seeded"
    );
}

#[tokio::test]
async fn the_inventory_withholds_a_sealed_or_vaulted_chests_rarity() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let start = TODAY * DAY_MS - 3 * HOUR_MS;
    seed_chest(&db, TODAY - 1, start, "rare", 40, "sealed").await;
    seed_chest(&db, TODAY - 1, start + HOUR_MS, "legendary", 150, "vaulted").await;
    seed_chest(&db, TODAY - 2, start, "epic", 0, "opened").await;
    seed_chest(&db, TODAY - 2, start + HOUR_MS, "common", 12, "resolved").await;
    // One token held, one whose window is open for another hour, and one spent.
    seed_token(&db, 0, NOW - 5 * HOUR_MS, 0, 0, 0).await;
    seed_token(&db, 0, NOW - 4 * HOUR_MS, NOW - HOUR_MS, NOW + HOUR_MS, 0).await;
    seed_token(
        &db,
        0,
        NOW - 3 * HOUR_MS,
        NOW - 3 * HOUR_MS,
        NOW - HOUR_MS,
        1,
    )
    .await;
    let inventory = chest_inventory(&db, now())
        .await
        .expect("the inventory reads");
    assert_eq!(
        inventory,
        Inventory {
            chests: vec![
                InventoryChest {
                    chest_id: 1,
                    state: "sealed",
                    study_day: day(TODAY - 1),
                    rarity: None,
                    xp: None,
                },
                InventoryChest {
                    chest_id: 2,
                    state: "vaulted",
                    study_day: day(TODAY - 1),
                    rarity: None,
                    xp: None,
                },
                InventoryChest {
                    chest_id: 3,
                    state: "opened",
                    study_day: day(TODAY - 2),
                    rarity: Some("epic"),
                    xp: Some(0),
                },
            ],
            tokens_held: 1,
            window_ends_at: Some(UtcMillis::from_epoch_millis(NOW + HOUR_MS)),
        },
        "the sealed and vaulted chests show no rarity or payout, the opened one shows both"
    );
}

#[tokio::test]
async fn a_second_activation_opens_no_second_window() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    seed_token(&db, 0, NOW - 2 * HOUR_MS, 0, 0, 0).await;
    seed_token(&db, 0, NOW - HOUR_MS, 0, 0, 0).await;
    let first = activate_token(&db, now())
        .await
        .expect("the activation runs");
    let second = activate_token(&db, now())
        .await
        .expect("the second activation runs");
    assert_eq!(
        (first, second, tokens(&db).await),
        (
            ActivateAnswer::Activated {
                token_id: 1,
                window_ends_at: UtcMillis::from_epoch_millis(NOW + 7_200_000),
            },
            ActivateAnswer::AlreadyActive,
            vec![(1, 0, NOW, NOW + 7_200_000, 0), (2, 0, 0, 0, 0)],
        ),
        "the oldest token opens a two-hour window, and the second activation leaves the next held"
    );
}

/// The name of the prize a pick paid, none when the pick paid nothing.
fn prize_named(answer: PickAnswer) -> Option<&'static str> {
    if let PickAnswer::Chosen { prize, .. } = answer {
        Some(prize.name())
    } else {
        None
    }
}

#[tokio::test]
async fn each_picked_prize_is_named_as_its_chest_stores_the_choice() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = database(&scratch).await;
    let start = TODAY * DAY_MS - 2 * HOUR_MS;
    let token_epic = seed_chest(&db, TODAY - 1, start, "epic", 0, "opened").await;
    let freeze_epic = seed_chest(&db, TODAY - 1, start + HOUR_MS, "epic", 0, "opened").await;
    let mut named = Vec::new();
    for (id, prize) in [(token_epic, Prize::Token), (freeze_epic, Prize::Freeze)] {
        let answer = pick_epic_choice(&db, id, prize, day(TODAY), now())
            .await
            .expect("the pick runs");
        named.push(prize_named(answer));
    }
    assert_eq!(
        (named, chests(&db).await),
        (
            vec![Some("token"), Some("freeze")],
            vec![
                chest_row(1, "resolved", "token"),
                chest_row(2, "resolved", "freeze")
            ],
        ),
        "each picked prize is named by the word its chest stores, as the owner's surfaces write it"
    );
}
