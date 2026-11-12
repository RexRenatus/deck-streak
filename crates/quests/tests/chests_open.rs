//! Opening a chest, an Epic's choice and the sweep of stale chests (SPEC-081 A8-A10).
//!
//! Every case runs against its own file-backed database through the kernel's repository base,
//! and each step runs inside one write, as the caller's write holds it (ADR-081). A8 and A9 assert
//! the stored rows; A10 holds the sweep to the predecessor's golden.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_quests::chest_store::{self, ChestState, Choice, NewChest, Origin};
use deck_streak_quests::chests::{
    Opened, Payout, Rarity, Settled, Swept, open_chest_on, settle_epic_choice_on,
    sweep_stale_chests_on,
};
use serde_json::{Value, json};
use sqlx::Row;
use tempfile::TempDir;

/// The instant every write in these tests records.
const AT: UtcMillis = UtcMillis::from_epoch_millis(1_700_020_800_000);

/// A whole number under `key`.
fn whole(value: &Value, key: &str) -> i64 {
    value[key].as_i64().expect("a whole number in the case")
}

/// The text under `key`.
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().expect("a text in the case")
}

/// The cases of the committed golden `name`, refused when there are none (the tdd pack).
fn cases_of(name: &str) -> Vec<golden::Case> {
    let golden =
        golden::read(&golden::committed(name)).unwrap_or_else(|refusal| panic!("{refusal}"));
    println!(
        "examined {} case(s) of {}",
        golden.cases.len(),
        golden.function
    );
    assert!(
        !golden.cases.is_empty(),
        "examined 0 case(s) of {name}: a golden that yields no case proves nothing"
    );
    golden.cases
}

/// A fresh database, migrated by the kernel's repository base.
async fn fresh() -> (TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

/// Stores a chest of `study_day` in `state`, under a key of its own (`key` as its session start),
/// and answers its id.
async fn store_chest(
    db: &Db,
    study_day: i64,
    rarity: Rarity,
    payout_xp: i64,
    state: ChestState,
    key: i64,
) -> i64 {
    let mut write = db.write().await.expect("a write");
    let chest = NewChest {
        study_day: StudyDay::from_epoch_day(study_day),
        origin: Origin::Session,
        session_start: key,
        rarity,
        payout_xp,
        state,
    };
    let id = chest_store::insert_chest(&mut write, &chest, AT)
        .await
        .expect("the chest stored")
        .expect("a key of its own");
    write.commit().await.expect("the seed committed");
    id
}

/// Writes a settled choice into a stored chest's row, as the case holds it.
async fn store_choice(db: &Db, id: i64, choice: &str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query("UPDATE chests SET choice = ?1 WHERE id = ?2")
        .bind(choice)
        .bind(id)
        .execute(&mut *write)
        .await
        .expect("the choice stored");
    write.commit().await.expect("the seed committed");
}

/// The stored row of chest `id`: its state and its choice.
async fn row_of(db: &Db, id: i64) -> Value {
    let mut write = db.write().await.expect("a write");
    let row = sqlx::query("SELECT state, choice FROM chests WHERE id = ?1")
        .bind(id)
        .fetch_one(&mut *write)
        .await
        .expect("the chest's row");
    write.commit().await.expect("the read closed");
    json!({
        "state": row.get::<String, _>("state"),
        "choice": row.get::<String, _>("choice"),
    })
}

/// Every stored token as (id, chest, activated, consumed).
async fn tokens_held(db: &Db) -> Vec<(i64, i64, i64, i64)> {
    let mut write = db.write().await.expect("a write");
    let rows =
        sqlx::query("SELECT id, chest_id, activated_at, consumed FROM xp_tokens ORDER BY id")
            .fetch_all(&mut *write)
            .await
            .expect("the tokens");
    write.commit().await.expect("the read closed");
    rows.iter()
        .map(|row| {
            (
                row.get("id"),
                row.get("chest_id"),
                row.get("activated_at"),
                row.get("consumed"),
            )
        })
        .collect()
}

/// A payout as the grant port is handed it.
fn payout_seen(payout: Option<Payout>) -> Value {
    payout.map_or(Value::Null, |payout| {
        json!({
            "source": payout.source(),
            "study_day": payout.study_day.epoch_day(),
            "amount": payout.xp,
            "track": payout.track.as_str(),
        })
    })
}

/// What an open answered, as the test reads it.
fn opened_seen(opened: Opened) -> Value {
    match opened {
        Opened::NoSuchChest => json!("no such chest"),
        Opened::AlreadyOpened => json!("already opened"),
        Opened::ChoicePending(chest) => json!({
            "pending": chest.id,
            "rarity": chest.rarity.name(),
            "state": chest.state.name(),
        }),
        Opened::Revealed { chest, payout } => json!({
            "revealed": chest.id,
            "rarity": chest.rarity.name(),
            "state": chest.state.name(),
            "payout": payout_seen(payout),
        }),
    }
}

/// Opens chest `id` in one write and commits it.
async fn open(db: &Db, id: i64) -> Value {
    let mut write = db.write().await.expect("a write");
    let opened = open_chest_on(&mut write, id).await.expect("the open");
    write.commit().await.expect("the write committed");
    opened_seen(opened)
}

/// What a choice answered, as the test reads it.
fn settled_seen(settled: Settled) -> Value {
    match settled {
        Settled::NotAnEpic => json!("not an epic"),
        Settled::AlreadySettled => json!("already settled"),
        Settled::Chosen {
            choice,
            capped,
            token_id,
        } => json!({"choice": choice.name(), "capped": capped, "token": token_id}),
    }
}

/// Settles chest `id`'s choice in one write and commits it.
async fn choose(db: &Db, id: i64, wanted: Choice, freeze_capped: bool) -> Value {
    let mut write = db.write().await.expect("a write");
    let settled = settle_epic_choice_on(&mut write, id, wanted, freeze_capped, AT)
        .await
        .expect("the choice");
    write.commit().await.expect("the write committed");
    settled_seen(settled)
}

#[tokio::test]
async fn opening_pays_once_and_reveals_the_stored_rarity() {
    let (_directory, db) = fresh().await;
    // Each paying rarity, sealed or vaulted, each on a study day of its own: the payout is granted
    // on the chest's own day, whatever day it is opened on.
    let paying = [
        (19_670, Rarity::Common, 17, ChestState::Sealed),
        (19_671, Rarity::Rare, 44, ChestState::Vaulted),
        (19_672, Rarity::Legendary, 150, ChestState::Sealed),
        (19_673, Rarity::Common, 25, ChestState::Vaulted),
    ];
    for (key, (day, rarity, payout, state)) in (1..).zip(paying) {
        let id = store_chest(&db, day, rarity, payout, state, key).await;
        assert_eq!(
            open(&db, id).await,
            json!({
                "revealed": id,
                "rarity": rarity.name(),
                "state": "resolved",
                "payout": {
                    "source": format!("chest:{id}"),
                    "study_day": day,
                    "amount": payout,
                    "track": "language",
                },
            }),
            "the first open of a {state:?} {rarity:?} chest pays its stored payout once"
        );
        assert_eq!(
            row_of(&db, id).await,
            json!({"state": "resolved", "choice": ""})
        );
        assert_eq!(
            open(&db, id).await,
            json!("already opened"),
            "a second open of chest {id} pays nothing"
        );
        assert_eq!(
            row_of(&db, id).await,
            json!({"state": "resolved", "choice": ""})
        );
    }
    // An Epic opens to its pending choice and pays nothing on the open.
    for (key, state) in [(10, ChestState::Sealed), (11, ChestState::Vaulted)] {
        let id = store_chest(&db, 19_674, Rarity::Epic, 0, state, key).await;
        assert_eq!(
            open(&db, id).await,
            json!({"pending": id, "rarity": "epic", "state": "opened"}),
            "a {state:?} Epic opens to its choice"
        );
        assert_eq!(
            row_of(&db, id).await,
            json!({"state": "opened", "choice": ""})
        );
        assert_eq!(open(&db, id).await, json!("already opened"));
        assert_eq!(
            row_of(&db, id).await,
            json!({"state": "opened", "choice": ""})
        );
    }
    // A chest already opened or resolved changes nothing, and an unknown id is no chest.
    for (key, state) in [(20, ChestState::Opened), (21, ChestState::Resolved)] {
        let id = store_chest(&db, 19_675, Rarity::Rare, 40, state, key).await;
        assert_eq!(open(&db, id).await, json!("already opened"));
        assert_eq!(
            row_of(&db, id).await,
            json!({"state": state.name(), "choice": ""})
        );
    }
    assert_eq!(open(&db, 9_999).await, json!("no such chest"));
}

#[tokio::test]
async fn a_capped_freeze_choice_becomes_a_token() {
    let (_directory, db) = fresh().await;
    let epic = |key| store_chest(&db, 19_676, Rarity::Epic, 0, ChestState::Opened, key);
    // A freeze a cap refuses becomes a token, and the reveal says the freeze was capped.
    let capped = epic(1).await;
    assert_eq!(
        choose(&db, capped, Choice::Freeze, true).await,
        json!({"choice": "token", "capped": true, "token": 1}),
        "a capped freeze becomes a token"
    );
    assert_eq!(
        row_of(&db, capped).await,
        json!({"state": "resolved", "choice": "token"})
    );
    assert_eq!(tokens_held(&db).await, vec![(1, capped, 0, 0)]);
    // The choice settles once: a second answer changes nothing and grants no second token.
    for (wanted, freeze_capped) in [(Choice::Token, false), (Choice::Freeze, false)] {
        assert_eq!(
            choose(&db, capped, wanted, freeze_capped).await,
            json!("already settled")
        );
    }
    assert_eq!(
        row_of(&db, capped).await,
        json!({"state": "resolved", "choice": "token"})
    );
    assert_eq!(tokens_held(&db).await, vec![(1, capped, 0, 0)]);
    // A freeze with room is the freeze, for the caller to grant: no token.
    let frozen = epic(2).await;
    assert_eq!(
        choose(&db, frozen, Choice::Freeze, false).await,
        json!({"choice": "freeze", "capped": false, "token": null})
    );
    assert_eq!(
        row_of(&db, frozen).await,
        json!({"state": "resolved", "choice": "freeze"})
    );
    // A token chosen is a token, whatever the freeze caps say.
    let token = epic(3).await;
    assert_eq!(
        choose(&db, token, Choice::Token, true).await,
        json!({"choice": "token", "capped": false, "token": 2})
    );
    assert_eq!(
        tokens_held(&db).await,
        vec![(1, capped, 0, 0), (2, token, 0, 0)]
    );
    // An Epic not yet opened settles nothing, nor does a chest that is not an Epic.
    let sealed = store_chest(&db, 19_676, Rarity::Epic, 0, ChestState::Sealed, 4).await;
    assert_eq!(
        choose(&db, sealed, Choice::Token, false).await,
        json!("already settled")
    );
    assert_eq!(
        row_of(&db, sealed).await,
        json!({"state": "sealed", "choice": ""})
    );
    let rare = store_chest(&db, 19_676, Rarity::Rare, 40, ChestState::Opened, 5).await;
    assert_eq!(
        choose(&db, rare, Choice::Token, false).await,
        json!("not an epic")
    );
    assert_eq!(
        choose(&db, 9_999, Choice::Token, false).await,
        json!("not an epic")
    );
    assert_eq!(
        tokens_held(&db).await.len(),
        2,
        "no other choice granted a token"
    );
}

/// What a sweep answered, as the golden writes it.
fn swept_seen(swept: &[Swept]) -> Value {
    json!({
        "resolved": swept.iter().map(|chest| chest.chest_id).collect::<Vec<_>>(),
        "grants": swept
            .iter()
            .filter_map(|chest| chest.payout)
            .map(|payout| payout_seen(Some(payout)))
            .collect::<Vec<_>>(),
    })
}

/// Sweeps on `today` in one write and commits it.
async fn sweep(db: &Db, today: StudyDay) -> Vec<Swept> {
    let mut write = db.write().await.expect("a write");
    let swept = sweep_stale_chests_on(&mut write, today)
        .await
        .expect("the sweep");
    write.commit().await.expect("the write committed");
    swept
}

#[tokio::test]
async fn the_sweep_matches_the_predecessors_golden() {
    for case in cases_of("sweep_stale_chests") {
        let (_directory, db) = fresh().await;
        let chests = case.input["chests"].as_array().expect("the case's chests");
        let mut ids = Vec::new();
        for (key, chest) in (1..).zip(chests) {
            let id = store_chest(
                &db,
                whole(chest, "study_day"),
                Rarity::from_name(text(chest, "rarity")).expect("a rarity"),
                whole(chest, "payout_xp"),
                ChestState::from_name(text(chest, "state")).expect("a state"),
                key,
            )
            .await;
            if !text(chest, "choice").is_empty() {
                store_choice(&db, id, text(chest, "choice")).await;
            }
            ids.push(id);
        }
        let mut before = Vec::new();
        for id in &ids {
            before.push(row_of(&db, *id).await);
        }
        let today = StudyDay::from_epoch_day(whole(&case.input, "today"));
        let swept = sweep(&db, today).await;
        assert_eq!(
            swept_seen(&swept),
            case.output,
            "the sweep of {}",
            case.input
        );
        // The stored rows agree: exactly the chests the golden resolves moved, each to resolved.
        let mut moved = Vec::new();
        for (id, was) in ids.iter().zip(&before) {
            let now = row_of(&db, *id).await;
            if now != *was {
                assert_eq!(now["state"], "resolved", "chest {id} of {}", case.input);
                moved.push(*id);
            }
        }
        assert_eq!(
            json!(moved),
            case.output["resolved"],
            "the rows of {}",
            case.input
        );
        // A second sweep of the same day finds nothing left to pay.
        assert_eq!(
            sweep(&db, today).await,
            Vec::new(),
            "a second sweep of {}",
            case.input
        );
    }
}
