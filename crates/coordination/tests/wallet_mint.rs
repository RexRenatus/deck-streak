//! The fold's mint step (SPEC-082 A7, R4; ADR-315): each study day the fold evaluates is minted
//! from the day's final base, the backfilled days included, a settled day's mint is raised by a
//! later recompute and the current day's follows its base; and the mint commits or rolls back with
//! the day's write, in a savepoint of it. Every review, card and instant is synthetic and set by
//! hand.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::day_bonuses::DayBonusesStep;
use deck_streak_coordination::recompute::mint::MintStep;
use deck_streak_coordination::recompute::xp::XpStep;
use deck_streak_coordination::recompute::{
    DayEvaluation, DayStep, Evaluation, Fold, FoldInput, FoldReport, Phase,
};
use deck_streak_economy::wallet::SqliteWallet;
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{Db, KernelError, PortFuture, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::consistency::day_base_xp;
use deck_streak_progression::settle::day_rows;
use sqlx::SqliteConnection;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// A study day near the present.
const D0: i64 = 20_000;
/// The collection was created a thousand study days before [`D0`].
const CREATED: i64 = D0 - 1_000;
/// The current study day of every recompute of A7's fixture.
const T: i64 = D0 + 4;
/// A7's members, counted from SPEC-082 section 12's paragraph and not from the fixture below: the
/// three backfilled days, the settled day raised by late reviews, and the current day re-minted
/// down and then up.
const MEMBERS: usize = 6;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

const fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// `hour` o'clock UTC of study day `day` under the default rule.
const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

/// A review of `card` at `instant` with `ease`, of a card whose interval is 25 days.
const fn review(instant: i64, card: i64, ease: i64) -> Review {
    Review {
        id: instant,
        card_id: card,
        ease,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

/// A language card `id` in the review queue, due a hundred days after [`D0`]: nothing is overdue.
const fn card(id: i64) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: 1,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: D0 + 100 - CREATED,
        interval: 30,
        factor: 2500,
        reps: 3,
        lapses: 0,
        track: Track::Language,
        course: None,
        tier: None,
    }
}

/// The reviews of study day `day`, one per ease, an hour apart from `hour` o'clock, each of its own
/// card numbered from `first_card`.
fn reviews_on(day: i64, hour: i64, first_card: i64, eases: &[i64]) -> Vec<Review> {
    eases
        .iter()
        .zip(0_i64..)
        .map(|(ease, offset)| review(at(day, hour + offset), first_card + offset, *ease))
        .collect()
}

/// The collection holding `reviews`, with a card for each card they review.
fn collection(reviews: Vec<Review>) -> CollectionData {
    let cards: BTreeSet<i64> = reviews.iter().map(|review| review.card_id).collect();
    CollectionData {
        reviews,
        cards: cards.into_iter().map(card).collect(),
        created_at: UtcMillis::from_epoch_millis(at(CREATED, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// The fold of analytics' step, the XP steps and the mint step, each in the phase it declares,
/// with `extra` after them.
fn fold(extra: Option<Box<dyn DayStep>>) -> Fold {
    let mut fold = Fold::default();
    fold.register(
        Phase::RollupAndScore,
        Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
    )
    .expect("analytics' step is phase 1's");
    fold.register(Phase::BaseXp, Box::new(XpStep))
        .expect("the XP step is phase 2's");
    fold.register(Phase::DerivedBonuses, Box::new(DayBonusesStep))
        .expect("the bonus step is phase 5's");
    fold.register(MintStep.phase(), Box::new(MintStep))
        .expect("the mint step registers in the phase it declares");
    if let Some(step) = extra {
        fold.register(step.phase(), step)
            .expect("the test's step registers in the phase it declares");
    }
    fold
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

/// One recompute of `data` at `now`, a sync having started on study day `synced_in`.
async fn recompute(
    fold: &Fold,
    db: &Db,
    data: &CollectionData,
    now: i64,
    synced_in: i64,
) -> Result<FoldReport, KernelError> {
    fold.run(
        db,
        &FoldInput {
            data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(now),
            synced_in: Some(day(synced_in)),
            courses_digest: Some("0123456789abcdef"),
            base_reviews: 0,
            offers: None,
        },
    )
    .await
}

/// The day's base as the fold's XP steps wrote it: the sum of the day's settled XP rows that the
/// base counts, read from the committed state.
async fn base(db: &Db, number: i64) -> i64 {
    let mut write = db.write().await.expect("a write");
    based_on(&mut write, number).await
}

/// The base of study day `number` over `connection`.
async fn based_on(connection: &mut SqliteConnection, number: i64) -> i64 {
    let rows = day_rows(connection, day(number))
        .await
        .expect("the day's XP rows read");
    day_base_xp(
        rows.iter()
            .map(|(source, amount)| (source.as_str(), *amount)),
    )
}

/// The coins the day's movements hold, read through the wallet: the day's mint, the only movement
/// these folds write.
async fn minted(db: &Db, number: i64) -> i64 {
    let wallet = SqliteWallet::new(db.clone());
    let after = wallet
        .balance_before(day(number + 1))
        .await
        .expect("the wallet after the day");
    let before = wallet
        .balance_before(day(number))
        .await
        .expect("the wallet before the day");
    after - before
}

/// The parity golden of `mint_for_base_xp`, by base.
fn mint_golden() -> BTreeMap<i64, i64> {
    let golden = golden::read(&golden::committed("mint_for_base_xp")).expect("the mint's golden");
    golden
        .cases
        .iter()
        .map(|case| {
            (
                case.input["base_xp"].as_i64().expect("a whole base"),
                case.output.as_i64().expect("a whole mint"),
            )
        })
        .collect()
}

/// A step of this test's own fold, in phase 7, that fails every evaluation but a backfill's: the
/// first recompute stops at its first settle, after its backfill's write has committed.
struct StopAfterTheBackfill;

impl DayStep for StopAfterTheBackfill {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        "test.stop_after_the_backfill"
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if matches!(day.evaluation, Evaluation::Backfill { .. }) {
                Ok(())
            } else {
                Err(KernelError::Offload {
                    operation: "the test's stop after the backfill",
                })
            }
        })
    }
}

/// One member of A7's population: what it is, its day, its base and the coins the day holds.
type Member = (&'static str, i64, i64, i64);

#[tokio::test]
async fn the_mint_reads_the_settled_days_final_base() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let golden = mint_golden();
    let mut members: Vec<Member> = Vec::new();

    // Days D0 to D0+2 are backfilled, T-1 closes and T is current. The first recompute stops at its
    // first settle, so the backfilled days are read as the backfill's own write left them.
    let backfilled = [
        (D0, vec![2_i64, 2, 2, 2, 2]),
        (D0 + 1, vec![3, 3, 3, 3, 2, 1]),
        (D0 + 2, vec![3, 3, 3, 3, 3, 2, 1]),
    ];
    let mut past: Vec<Review> = Vec::new();
    for (number, eases) in &backfilled {
        past.extend(reviews_on(*number, 9, number * 10, eases));
    }
    let closing = reviews_on(T - 1, 9, (T - 1) * 10, &[3]);
    let late = reviews_on(T - 1, 15, (T - 1) * 10 + 5, &[2, 4, 1]);
    let today = |eases: &[i64]| reviews_on(T, 9, T * 10, eases);
    let with = |parts: &[&[Review]]| collection(parts.concat());

    let first = recompute(
        &fold(Some(Box::new(StopAfterTheBackfill))),
        &db,
        &with(&[&past, &closing, &today(&[3, 3, 3])]),
        at(T, 14),
        T,
    )
    .await;
    assert!(first.is_err(), "the test's step stops the first recompute");
    for (number, _) in &backfilled {
        members.push((
            "backfilled",
            *number,
            base(&db, *number).await,
            minted(&db, *number).await,
        ));
    }

    // The second recompute settles T-1 and evaluates T; the third brings T-1's late reviews and
    // fewer reviews on T; the fourth more on T.
    let full = fold(None);
    recompute(
        &full,
        &db,
        &with(&[&past, &closing, &today(&[3, 3, 3])]),
        at(T, 15),
        T,
    )
    .await
    .expect("the second recompute runs");
    let settled_base = base(&db, T - 1).await;
    let current_before = minted(&db, T).await;
    recompute(
        &full,
        &db,
        &with(&[&past, &closing, &late, &today(&[2])]),
        at(T, 16),
        T,
    )
    .await
    .expect("the third recompute runs");
    let raised = base(&db, T - 1).await;
    members.push(("settled, raised", T - 1, raised, minted(&db, T - 1).await));
    let down = minted(&db, T).await;
    members.push(("current, down", T, base(&db, T).await, down));
    recompute(
        &full,
        &db,
        &with(&[&past, &closing, &late, &today(&[2, 2, 2, 2, 2, 2])]),
        at(T, 17),
        T,
    )
    .await
    .expect("the fourth recompute runs");
    let up = minted(&db, T).await;
    members.push(("current, up", T, base(&db, T).await, up));

    // Each member's coins equal the golden mint of its final base, and a base the golden does not
    // hold fails here.
    println!("members (what, day, base, coins): {members:?}");
    for (what, number, base, coins) in &members {
        let mint = golden.get(base).unwrap_or_else(|| {
            panic!("{what} day {number}: the golden holds no mint for the base {base}")
        });
        assert_eq!(
            coins, mint,
            "{what} day {number}: the coins the day holds against the golden mint of its base {base}"
        );
    }
    assert!(
        raised > settled_base,
        "the late reviews raise the settled day's base: {settled_base} -> {raised}"
    );
    assert!(
        down < current_before && up > down,
        "the current day's mint follows its base down and up: {current_before} -> {down} -> {up}"
    );
    let distinct: BTreeSet<(i64, i64)> = members
        .iter()
        .map(|(_, number, base, _)| (*number, *base))
        .collect();
    let members = examined("mint members", members);
    assert_eq!(members.len(), MEMBERS, "the population: {members:?}");
    assert_eq!(
        distinct.len(),
        MEMBERS,
        "the distinct members: {distinct:?}"
    );
}

/// What the probe saw inside the fold's write on the current day: the day's base, the day's
/// movements summed on the write's own connection, and the day's movements a second connection
/// reads. Only the current day's are judged: the settle of the day before it, which this fold also
/// makes, mints that day's own base (its `backlog_zero`) in an earlier write that has committed.
type Seen = (i64, i64, i64);

/// A step of this test's own fold, in phase 7: on the current day it reads the day's movements
/// inside the fold's write and through the wallet's own reads, then fails when told to.
struct Probe {
    db: Db,
    fail: bool,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl DayStep for Probe {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        "test.probe"
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if !matches!(day.evaluation, Evaluation::Current) {
                return Ok(());
            }
            let number = day.day.epoch_day();
            let base = based_on(write, number).await;
            let inside: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(delta), 0) FROM coin_ledger WHERE study_day = ?1",
            )
            .bind(number)
            .fetch_one(&mut *write)
            .await?;
            let outside = minted(&self.db, number).await;
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((base, inside, outside));
            if self.fail {
                Err(KernelError::Offload {
                    operation: "the probe's failure after the mint",
                })
            } else {
                Ok(())
            }
        })
    }
}

#[tokio::test]
async fn the_mint_commits_or_rolls_back_with_the_days_write() {
    let golden = mint_golden();
    let data = collection(reviews_on(D0, 9, 1, &[3, 2, 4, 1]));
    let mut judged = Vec::new();
    for fail in [true, false] {
        let scratch = TempDir::new().expect("a scratch directory");
        let db = database(&scratch).await;
        let seen = Arc::new(Mutex::new(Vec::new()));
        let probe = Probe {
            db: db.clone(),
            fail,
            seen: Arc::clone(&seen),
        };
        let ran = recompute(&fold(Some(Box::new(probe))), &db, &data, at(D0, 14), D0).await;
        let seen = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
        println!("fail {fail}: seen (base, inside, outside): {seen:?}");
        let [(base, inside, outside)] = seen[..] else {
            panic!("the probe ran once, on the current day: {seen:?}");
        };
        let mint = *golden
            .get(&base)
            .unwrap_or_else(|| panic!("the golden holds no mint for the base {base}"));
        // Inside the day's write the mint is there; a second connection reads none of it before
        // the write commits.
        assert_eq!(
            inside, mint,
            "fail {fail}: the day's mint on the fold's own write"
        );
        assert!(
            mint > 0,
            "the day's base {base} mints coins, so the test can tell"
        );
        assert_eq!(
            outside, 0,
            "fail {fail}: the day's mint a second connection reads"
        );
        let held = minted(&db, D0).await;
        if fail {
            assert!(ran.is_err(), "the probe fails the fold");
            assert_eq!(held, 0, "a fold that fails after phase 6 leaves no mint");
        } else {
            ran.expect("the fold runs");
            assert_eq!(held, mint, "the mint commits with the day's write");
        }
        judged.push(fail);
    }
    assert_eq!(examined("folds", judged), vec![true, false]);
}
