//! The double-XP token: its activation and its bonus (SPEC-081 A11, A12).
//!
//! Every case runs against its own file-backed database through the kernel's repository base,
//! each step inside one write, as the caller's write holds it (ADR-081), and both are held to the
//! predecessor's goldens. A review's XP at the base rate is the caller's input: the case gives it.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Db, Hour, StudyDay, StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_quests::tokens::{
    Activation, ReviewXp, activate_token_on, settle_token_bonuses_on,
};
use serde_json::{Value, json};
use sqlx::Row;
use tempfile::TempDir;

/// The instant every seeded token records as its row's creation.
const AT: i64 = 1_700_020_800_000;

/// A whole number under `key`.
fn whole(value: &Value, key: &str) -> i64 {
    value[key].as_i64().expect("a whole number in the case")
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

/// Stores the case's tokens in its order, each for a chest of its own, as the rows hold them.
async fn seed_tokens(db: &Db, input: &Value) {
    let mut write = db.write().await.expect("a write");
    for (chest, token) in (1_i64..).zip(input["tokens"].as_array().expect("the case's tokens")) {
        sqlx::query(
            "INSERT INTO xp_tokens (chest_id, granted_at, activated_at, window_ends_at, \
             consumed, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(chest)
        .bind(whole(token, "granted_at_ms"))
        .bind(whole(token, "activated_at_ms"))
        .bind(whole(token, "window_ends_at_ms"))
        .bind(whole(token, "consumed"))
        .bind(AT)
        .execute(&mut *write)
        .await
        .expect("a token stored");
    }
    write.commit().await.expect("the seed committed");
}

/// Every stored token as (id, activated at, window's end, consumed).
async fn tokens_held(db: &Db) -> Vec<(i64, i64, i64, i64)> {
    let mut write = db.write().await.expect("a write");
    let rows =
        sqlx::query("SELECT id, activated_at, window_ends_at, consumed FROM xp_tokens ORDER BY id")
            .fetch_all(&mut *write)
            .await
            .expect("the tokens");
    write.commit().await.expect("the read closed");
    rows.iter()
        .map(|row| {
            (
                row.get("id"),
                row.get("activated_at"),
                row.get("window_ends_at"),
                row.get("consumed"),
            )
        })
        .collect()
}

#[tokio::test]
async fn token_activation_matches_the_predecessors_golden() {
    for case in cases_of("activate_double_xp") {
        let (_directory, db) = fresh().await;
        seed_tokens(&db, &case.input).await;
        let before = tokens_held(&db).await;
        let now = UtcMillis::from_epoch_millis(whole(&case.input, "now_ms"));
        let mut write = db.write().await.expect("a write");
        let activation = activate_token_on(&mut write, now)
            .await
            .expect("the activation");
        write.commit().await.expect("the write committed");
        let after = tokens_held(&db).await;
        // The stored rows the activation moved, as the golden records an activation written.
        let activated: Vec<Value> = after
            .iter()
            .zip(&before)
            .filter(|(now, was)| now != was)
            .map(|((id, activated_at, window_ends_at, _), _)| {
                json!({
                    "id": id,
                    "activated_at_ms": activated_at,
                    "window_ends_at_ms": window_ends_at,
                })
            })
            .collect();
        let (ok, error, until_ms) = match activation {
            Activation::Activated { window, .. } => {
                (true, Value::Null, json!(window.end.epoch_millis()))
            }
            Activation::AlreadyActive => (false, json!("a token is already active"), Value::Null),
            Activation::NoneHeld => (false, json!("no token held"), Value::Null),
        };
        let ours = json!({"ok": ok, "error": error, "until_ms": until_ms, "activated": activated});
        assert_eq!(ours, case.output, "the activation of {}", case.input);
        if let Activation::Activated { token_id, window } = activation {
            assert_eq!(
                json!([{
                    "id": token_id,
                    "activated_at_ms": window.start.epoch_millis(),
                    "window_ends_at_ms": window.end.epoch_millis(),
                }]),
                case.output["activated"],
                "the activation answers the token and window it stored, of {}",
                case.input
            );
        }
    }
}

/// The case's reviews, each with the XP the case gives it.
fn reviews_of(input: &Value) -> Vec<ReviewXp> {
    input["reviews"]
        .as_array()
        .expect("a case's reviews")
        .iter()
        .map(|row| ReviewXp {
            review: Review {
                id: whole(row, "id"),
                card_id: whole(row, "card_id"),
                ease: whole(row, "ease"),
                interval: 0,
                last_interval: 0,
                factor: 2500,
                taken_ms: whole(row, "taken_ms"),
                kind: whole(row, "kind"),
            },
            base_xp: whole(row, "xp"),
        })
        .collect()
}

/// The case's study day rule: its rollover hour at its offset.
fn rule_of(input: &Value) -> StudyDayRule {
    let hour = u8::try_from(whole(input, "rollover_hour"))
        .ok()
        .and_then(Hour::new)
        .expect("a rollover hour");
    let offset = i16::try_from(whole(input, "tz_offset_minutes"))
        .ok()
        .and_then(UtcOffset::from_minutes)
        .expect("an offset");
    StudyDayRule::new(hour, offset)
}

#[tokio::test]
async fn the_token_bonus_matches_the_predecessors_golden() {
    for case in cases_of("recompute_token_xp") {
        let (_directory, db) = fresh().await;
        seed_tokens(&db, &case.input).await;
        let before = tokens_held(&db).await;
        let day = StudyDay::from_epoch_day(whole(&case.input, "study_day"));
        let now = UtcMillis::from_epoch_millis(whole(&case.input, "now_ms"));
        let mut write = db.write().await.expect("a write");
        let settled = settle_token_bonuses_on(
            &mut write,
            day,
            rule_of(&case.input),
            &reviews_of(&case.input),
            now,
        )
        .await
        .expect("the settlement");
        write.commit().await.expect("the write committed");
        let ours = json!({
            "grants": settled
                .bonuses
                .iter()
                .map(|bonus| json!({
                    "study_day": bonus.study_day.epoch_day(),
                    "source": bonus.source(),
                    "amount": bonus.xp,
                    "track": bonus.track.as_str(),
                }))
                .collect::<Vec<_>>(),
            "consumed": settled.consumed,
        });
        assert_eq!(ours, case.output, "the token bonus of {}", case.input);
        // The stored rows agree: exactly the tokens consumed now changed, each to consumed.
        let after = tokens_held(&db).await;
        let changed: Vec<i64> = after
            .iter()
            .zip(&before)
            .filter(|(now, was)| now != was)
            .map(|((id, _, _, consumed), _)| {
                assert_eq!(*consumed, 1, "token {id} of {}", case.input);
                *id
            })
            .collect();
        assert_eq!(
            json!(changed),
            case.output["consumed"],
            "the rows of {}",
            case.input
        );
    }
}
