//! The grant step's quests side: the session chests and their pity, rolled once, and the
//! challenge and weekly chests (SPEC-081 A4-A7).
//!
//! Every case runs against its own file-backed database through the kernel's repository base,
//! and the grant runs inside one write, as the fold's day write holds it (ADR-081). The draws are
//! a test double of the draw port: the case's listed draws, or a draw that fails.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, VecDeque};

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Db, Hour, StudyDay, UtcMillis};
use deck_streak_quests::chest_store::{
    self, ChestSettings, ChestState, NewChest, Origin, StoredChest,
};
use deck_streak_quests::chests::{
    ChestError, EarnedSession, Rarity, SessionChestGrant, grant_challenge_chest_on,
    grant_session_chests_on, grant_weekly_chest_on,
};
use deck_streak_quests::draw::{Draw, DrawError, OsDraw, fraction};
use deck_streak_quests::pity::Pity;
use deck_streak_quests::sessions::sessions_from_reviews;
use serde_json::{Value, json};
use tempfile::TempDir;

/// The instant every write in these tests records.
const AT: UtcMillis = UtcMillis::from_epoch_millis(1_700_020_800_000);

/// 2^-53, the step between two 53-bit draws.
const STEP: f64 = 1.0 / 9_007_199_254_740_992.0;

/// The draw a failing generator answers.
fn refused() -> DrawError {
    DrawError::Generator(getrandom::Error::UNSUPPORTED)
}

/// The case's listed draws, handed out in order; a draw past the list fails, as a failing
/// generator would. `used` counts the draws handed out.
struct Listed {
    values: VecDeque<f64>,
    used: usize,
}

impl Listed {
    fn new(values: impl IntoIterator<Item = f64>) -> Self {
        Self {
            values: values.into_iter().collect(),
            used: 0,
        }
    }

    fn of(input: &Value) -> Self {
        Self::new(
            input["draws"]
                .as_array()
                .expect("a case's draws")
                .iter()
                .map(|draw| draw.as_f64().expect("a draw")),
        )
    }
}

impl Draw for Listed {
    fn draw(&mut self) -> Result<f64, DrawError> {
        let value = self.values.pop_front().ok_or_else(refused)?;
        self.used += 1;
        Ok(value)
    }
}

/// A draw that answers one half until its `fails_at`-th call (counting from 1), which fails.
struct FailsAt {
    fails_at: usize,
    calls: usize,
}

impl Draw for FailsAt {
    fn draw(&mut self) -> Result<f64, DrawError> {
        self.calls += 1;
        if self.calls >= self.fails_at {
            Err(refused())
        } else {
            Ok(0.5)
        }
    }
}

/// A whole number under `key`.
fn whole(value: &Value, key: &str) -> i64 {
    value[key].as_i64().expect("a whole number in the case")
}

/// A truth value under `key`.
fn truth(value: &Value, key: &str) -> bool {
    value[key].as_bool().expect("a truth value in the case")
}

/// An hour of the day under `key`.
fn hour(value: &Value, key: &str) -> Hour {
    u8::try_from(whole(value, key))
        .ok()
        .and_then(Hour::new)
        .expect("an hour of the day in the case")
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

/// The study day a case names.
fn day_of(input: &Value) -> StudyDay {
    StudyDay::from_epoch_day(whole(input, "study_day"))
}

/// Stores what a case says the day already held: its settings when it names them, the pity
/// counters and the chests already on the day, each a sealed Common of 10.
async fn seed(db: &Db, input: &Value) {
    let mut write = db.write().await.expect("a write");
    if input.get("per_day_max").is_some() {
        chest_store::set_settings(
            &mut write,
            ChestSettings {
                per_day_max: whole(input, "per_day_max"),
                vault_hour: hour(input, "vault_hour"),
            },
        )
        .await
        .expect("the settings stored");
    }
    chest_store::set_pity(
        &mut write,
        Pity {
            since_epic: whole(input, "since_epic"),
            since_legendary: whole(input, "since_legendary"),
        },
    )
    .await
    .expect("the pity stored");
    for chest in input["existing"].as_array().expect("the day's chests") {
        let origin = chest["origin"].as_str().expect("an origin");
        let seeded = NewChest {
            study_day: day_of(input),
            origin: Origin::from_name(origin).expect("an origin the store knows"),
            session_start: whole(chest, "session_start_ms"),
            rarity: Rarity::Common,
            payout_xp: 10,
            state: ChestState::Sealed,
        };
        chest_store::insert_chest(&mut write, &seeded, AT)
            .await
            .expect("an existing chest stored");
    }
    write.commit().await.expect("the seed committed");
}

/// The day's chests as the store holds them.
async fn chests_held(db: &Db, day: StudyDay) -> Vec<StoredChest> {
    let mut write = db.write().await.expect("a write");
    let chests = chest_store::chests_of_day(&mut write, day)
        .await
        .expect("the day's chests");
    write.commit().await.expect("the read closed");
    chests
}

/// The pity counters as the store holds them.
async fn pity_held(db: &Db) -> Pity {
    let mut write = db.write().await.expect("a write");
    let pity = chest_store::pity(&mut write)
        .await
        .expect("the pity counters");
    write.commit().await.expect("the read closed");
    pity
}

/// The chests in `after` that `before` did not hold, as the golden writes a chest.
fn added(before: &[StoredChest], after: &[StoredChest]) -> Vec<Value> {
    after
        .iter()
        .filter(|chest| before.iter().all(|held| held.id != chest.id))
        .map(|chest| {
            json!({
                "origin": chest.origin.name(),
                "session_start_ms": chest.session_start,
                "rarity": chest.rarity.name(),
                "payout_xp": chest.payout_xp,
                "state": chest.state.name(),
            })
        })
        .collect()
}

/// The reviews a case holds, as the ingest context reads them.
fn reviews_of(input: &Value) -> Vec<Review> {
    input["reviews"]
        .as_array()
        .expect("a case's reviews")
        .iter()
        .map(|row| Review {
            id: whole(row, "id"),
            card_id: whole(row, "card_id"),
            ease: whole(row, "ease"),
            interval: 0,
            last_interval: 0,
            factor: 2500,
            taken_ms: whole(row, "taken_ms"),
            kind: whole(row, "kind"),
        })
        .collect()
}

/// The case's sessions, each with the review XP the case gives its start.
fn sessions_of(input: &Value) -> Vec<EarnedSession> {
    let xp: BTreeMap<i64, i64> = input["session_xp"]
        .as_array()
        .expect("the case's session XP")
        .iter()
        .map(|pair| {
            (
                pair[0].as_i64().expect("a session start"),
                pair[1].as_i64().expect("a session's XP"),
            )
        })
        .collect();
    let cap = input["answer_time_cap_seconds"]
        .as_f64()
        .expect("the case's per-answer cap");
    sessions_from_reviews(&reviews_of(input), cap)
        .into_iter()
        .map(|session| EarnedSession {
            base_xp: *xp
                .get(&session.start.epoch_millis())
                .expect("the XP of each session"),
            session,
        })
        .collect()
}

/// The session grant's request for a case.
fn request_of<'a>(input: &Value, sessions: &'a [EarnedSession]) -> SessionChestGrant<'a> {
    SessionChestGrant {
        study_day: day_of(input),
        sessions,
        skip_day: truth(input, "skip_day"),
        ascendant: truth(input, "ascendant"),
        local_hour: hour(input, "local_hour"),
        quiet: truth(input, "quiet"),
    }
}

/// Runs the session grant for a case in one write, commits it, and answers what it returned.
async fn grant_sessions(
    db: &Db,
    input: &Value,
    draw: &mut impl Draw,
) -> Result<Vec<StoredChest>, ChestError> {
    let sessions = sessions_of(input);
    let mut write = db.write().await.expect("a write");
    let granted =
        grant_session_chests_on(&mut write, &request_of(input, &sessions), draw, AT).await;
    // The caller commits whatever the grant left in its write, even after an error: the worst
    // caller, so a write the grant should not have made would be kept and seen.
    write.commit().await.expect("the write committed");
    granted
}

/// The first case of the session golden that matches `wanted`.
fn a_case(wanted: impl Fn(&Value, &Value) -> bool) -> golden::Case {
    cases_of("session_chests_granted")
        .into_iter()
        .find(|case| wanted(&case.input, &case.output))
        .expect("a case of the shape the test needs")
}

#[tokio::test]
async fn the_session_chest_grant_matches_the_predecessors_golden() {
    for case in cases_of("session_chests_granted") {
        let (_directory, db) = fresh().await;
        seed(&db, &case.input).await;
        let day = day_of(&case.input);
        let before = chests_held(&db, day).await;
        let mut draws = Listed::of(&case.input);
        let granted = grant_sessions(&db, &case.input, &mut draws)
            .await
            .expect("the grant");
        let after = chests_held(&db, day).await;
        let pity = pity_held(&db).await;
        let ours = json!({
            "chests": added(&before, &after),
            "since_epic": pity.since_epic,
            "since_legendary": pity.since_legendary,
            "draws_used": draws.used,
        });
        assert_eq!(ours, case.output, "the grant of {}", case.input);
        assert_eq!(
            granted.iter().map(|chest| chest.id).collect::<Vec<_>>(),
            after
                .iter()
                .filter(|chest| before.iter().all(|held| held.id != chest.id))
                .map(|chest| chest.id)
                .collect::<Vec<_>>(),
            "the grant answers the chests it stored, of {}",
            case.input
        );
    }
}

#[tokio::test]
async fn a_second_recompute_never_rolls_a_session_again() {
    // Two chests or more, on a day still under its cap: the second recompute reaches each
    // session's key rather than stopping at the cap.
    let case = a_case(|input, output| {
        input["existing"].as_array().is_some_and(Vec::is_empty)
            && output["chests"].as_array().is_some_and(|chests| {
                i64::try_from(chests.len())
                    .is_ok_and(|rolled| rolled >= 2 && rolled < whole(input, "per_day_max"))
            })
    });
    let (_directory, db) = fresh().await;
    seed(&db, &case.input).await;
    let day = day_of(&case.input);
    let mut first = Listed::of(&case.input);
    grant_sessions(&db, &case.input, &mut first)
        .await
        .expect("the first recompute");
    let rolled = chests_held(&db, day).await;
    let pity = pity_held(&db).await;

    let mut second = Listed::new(std::iter::repeat_n(0.5, 32));
    let again = grant_sessions(&db, &case.input, &mut second)
        .await
        .expect("the second recompute");
    assert_eq!(
        second.used, 0,
        "a second recompute of the same study day drew {} time(s)",
        second.used
    );
    assert!(again.is_empty(), "a second recompute granted {again:?}");
    assert_eq!(
        chests_held(&db, day).await,
        rolled,
        "the stored chests changed"
    );
    assert_eq!(pity_held(&db).await, pity, "the pity counters changed");
    // The first recompute rolled what the predecessor rolled, so the chests above are present.
    assert_eq!(json!(added(&[], &rolled)), case.output["chests"]);

    // The key alone refuses a second chest, whatever reaches the store (S08110).
    let held = rolled.first().copied().expect("a rolled chest");
    let mut write = db.write().await.expect("a write");
    let twice = chest_store::insert_chest(
        &mut write,
        &NewChest {
            study_day: held.study_day,
            origin: held.origin,
            session_start: held.session_start,
            rarity: Rarity::Legendary,
            payout_xp: 150,
            state: ChestState::Sealed,
        },
        AT,
    )
    .await
    .expect("the store answers");
    write.commit().await.expect("the write committed");
    assert_eq!(twice, None, "a second chest was stored under a held key");
    assert_eq!(
        chests_held(&db, day).await,
        rolled,
        "the held key's chest changed"
    );
}

#[tokio::test]
async fn a_failed_draw_writes_no_chest_and_no_pity() {
    let case = a_case(|input, output| {
        input["existing"].as_array().is_some_and(Vec::is_empty)
            && whole(input, "since_epic") + whole(input, "since_legendary") > 0
            && output["chests"]
                .as_array()
                .is_some_and(|chests| !chests.is_empty())
    });
    let day = day_of(&case.input);
    // The rarity's draw fails, then the payout's: neither leaves a chest or a counter behind.
    for fails_at in [1, 2] {
        let (_directory, db) = fresh().await;
        seed(&db, &case.input).await;
        let pity = pity_held(&db).await;
        let mut failing = FailsAt { fails_at, calls: 0 };
        let failed = grant_sessions(&db, &case.input, &mut failing).await;
        assert!(
            matches!(failed, Err(ChestError::Draw(_))),
            "a failed draw at call {fails_at} answered {failed:?}"
        );
        assert_eq!(
            chests_held(&db, day).await,
            Vec::new(),
            "a failed draw at call {fails_at} stored a chest"
        );
        assert_eq!(
            pity_held(&db).await,
            pity,
            "a failed draw at call {fails_at} moved the pity counters"
        );
        // The next recompute rolls the session, once, as the predecessor did.
        let mut draws = Listed::of(&case.input);
        grant_sessions(&db, &case.input, &mut draws)
            .await
            .expect("the recompute after a failed draw");
        let after = chests_held(&db, day).await;
        let pity = pity_held(&db).await;
        assert_eq!(
            json!({
                "chests": added(&[], &after),
                "since_epic": pity.since_epic,
                "since_legendary": pity.since_legendary,
                "draws_used": draws.used,
            }),
            case.output,
            "the recompute after a failed draw at call {fails_at}"
        );
    }
}

#[tokio::test]
async fn the_challenge_and_weekly_chests_match_the_predecessors_goldens() {
    for case in cases_of("challenge_chest") {
        let (_directory, db) = fresh().await;
        seed(&db, &case.input).await;
        let day = day_of(&case.input);
        let before = chests_held(&db, day).await;
        let mut draws = Listed::of(&case.input);
        let mut write = db.write().await.expect("a write");
        let granted = grant_challenge_chest_on(&mut write, day, &mut draws, AT)
            .await
            .expect("the challenge chest");
        write.commit().await.expect("the write committed");
        let after = chests_held(&db, day).await;
        let pity = pity_held(&db).await;
        let ours = json!({
            "chests": added(&before, &after),
            "since_epic": pity.since_epic,
            "since_legendary": pity.since_legendary,
        });
        assert_eq!(ours, case.output, "the challenge chest of {}", case.input);
        assert_eq!(
            granted.map(|chest| chest.id),
            after
                .last()
                .filter(|_| after.len() > before.len())
                .map(|chest| chest.id),
            "the challenge grant answers the chest it stored, of {}",
            case.input
        );
    }
    for case in cases_of("weekly_chest") {
        let (_directory, db) = fresh().await;
        seed(&db, &case.input).await;
        let day = day_of(&case.input);
        let before = chests_held(&db, day).await;
        let mut write = db.write().await.expect("a write");
        let granted = grant_weekly_chest_on(&mut write, day, AT)
            .await
            .expect("the weekly chest");
        write.commit().await.expect("the write committed");
        let after = chests_held(&db, day).await;
        let pity = pity_held(&db).await;
        // The weekly grant takes no draw port at all, so it draws nothing by its signature.
        let ours = json!({
            "chests": added(&before, &after),
            "since_epic": pity.since_epic,
            "since_legendary": pity.since_legendary,
            "draws_used": 0,
        });
        assert_eq!(ours, case.output, "the weekly chest of {}", case.input);
        assert_eq!(
            granted.map(|chest| chest.id),
            after
                .last()
                .filter(|_| after.len() > before.len())
                .map(|chest| chest.id),
            "the weekly grant answers the chest it stored, of {}",
            case.input
        );
    }
}

#[tokio::test]
async fn a_held_quest_chest_key_takes_no_draw_and_writes_nothing() {
    let case = cases_of("challenge_chest")
        .into_iter()
        .find(|case| {
            case.input["existing"].as_array().is_some_and(Vec::is_empty)
                && case.output["chests"]
                    .as_array()
                    .is_some_and(|chests| chests.len() == 1)
        })
        .expect("a challenge case that grants one chest");
    let (_directory, db) = fresh().await;
    seed(&db, &case.input).await;
    let day = day_of(&case.input);
    let mut first = Listed::of(&case.input);
    let mut write = db.write().await.expect("a write");
    let challenge = grant_challenge_chest_on(&mut write, day, &mut first, AT)
        .await
        .expect("the first challenge chest");
    let weekly = grant_weekly_chest_on(&mut write, day, AT)
        .await
        .expect("the first weekly chest");
    write.commit().await.expect("the write committed");
    let held = chests_held(&db, day).await;
    let pity = pity_held(&db).await;
    // Both chests were stored, so the second grants below meet held keys.
    assert_eq!(
        (
            challenge.map(|chest| chest.origin),
            weekly.map(|chest| chest.origin)
        ),
        (Some(Origin::Challenge), Some(Origin::Weekly))
    );

    let mut second = Listed::new(std::iter::repeat_n(0.5, 8));
    let mut write = db.write().await.expect("a write");
    let challenge = grant_challenge_chest_on(&mut write, day, &mut second, AT)
        .await
        .expect("the second challenge chest");
    let weekly = grant_weekly_chest_on(&mut write, day, AT)
        .await
        .expect("the second weekly chest");
    write.commit().await.expect("the write committed");
    assert_eq!(
        second.used, 0,
        "a held challenge key drew {} time(s)",
        second.used
    );
    assert_eq!(
        (challenge, weekly),
        (None, None),
        "a held key granted again"
    );
    assert_eq!(
        chests_held(&db, day).await,
        held,
        "the stored chests changed"
    );
    assert_eq!(pity_held(&db).await, pity, "the pity counters changed");
}

/// A draw's exact bits, so two draws compare exactly.
fn bits(draw: f64) -> u64 {
    draw.to_bits()
}

#[test]
fn a_draw_is_the_top_53_bits_as_a_fraction_below_one() {
    assert_eq!(
        bits(fraction(u64::MAX)),
        bits(1.0 - STEP),
        "the largest draw"
    );
    assert_eq!(
        bits(fraction(1 << 11)),
        bits(STEP),
        "the least bit a draw keeps"
    );
    assert_eq!(
        bits(fraction((1 << 11) - 1)),
        bits(0.0),
        "the bits a draw drops"
    );
    assert_eq!(bits(fraction(1 << 63)), bits(0.5), "the top bit");
    assert_eq!(bits(fraction(0)), bits(0.0), "the least draw");
    let mut os = OsDraw;
    let draws: Vec<f64> = (0..64).map(|_| os.draw().expect("a draw")).collect();
    assert!(
        draws
            .iter()
            .all(|draw| (0.0..1.0).contains(draw) && bits(draw.rem_euclid(STEP)) == bits(0.0)),
        "the generator's draws {draws:?}"
    );
    assert!(
        draws.windows(2).any(|pair| bits(pair[0]) != bits(pair[1])),
        "64 draws from the generator were all {}",
        draws[0]
    );
}
