//! The badge step (SPEC-073 A10, A11; R4, R5, R8; ADR-303): a new badge is celebrated once, also
//! when an evaluation stops after its write or after the router answered, a closing day is judged
//! with its end-of-day state, and THE CLASS RULE: every evaluated day awards exactly the study
//! badges its context earns, once. An owed badge is raised with its line on the offers' day and
//! marked from the clock at the answer, and a mark another offer set first is kept. The step is
//! named `progression.badges`, and the awards' offers print their type with the router they hand
//! to left out. Every review, rollup, card state and streak is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use std::collections::BTreeSet;
use std::sync::Arc;

use deck_streak_coordination::recompute::badges::{BadgesStep, offer_badges};
use deck_streak_coordination::recompute::{
    AwardOffers, Evaluation, Fold, FoldInput, Offers, Phase,
};
use deck_streak_ingest::reader::CollectionData;
use deck_streak_kernel::{Courses, Db, StudyDayRule, UtcMillis};

use support::{
    D0, DIGEST, MarksFirst, Recorder, RecordingBot, RollupSeed, RouteThenFail, answer, at, badges,
    card, card_state, collection, day, router, run_step, scratch, seed_rollups, seed_streak,
};

/// The fold of the badge step alone.
fn badge_fold() -> Fold {
    let mut fold = Fold::default();
    fold.register(Phase::Awards, Box::new(BadgesStep::new(Courses::default())))
        .expect("the badge step is phase 7's");
    fold
}

/// One recompute of `data` at `now`, with `offers` between its writes.
async fn recompute(db: &Db, data: &CollectionData, offers: Option<&dyn Offers>, now: i64) {
    badge_fold()
        .run(
            db,
            &FoldInput {
                data,
                rule: StudyDayRule::default(),
                now: UtcMillis::from_epoch_millis(now),
                synced_in: None,
                courses_digest: Some(DIGEST),
                base_reviews: 0,
                offers,
            },
        )
        .await
        .expect("the fold runs");
}

/// One answer on `D0`: the lifetime's first review, which earns First Steps and nothing else.
fn first_answer() -> CollectionData {
    collection(vec![answer(at(D0, 10), 1, 3, 5, 9_000)], vec![card(1, 1)])
}

/// A database holding `D0`'s rollup at score 50 and no card state.
async fn first_day() -> support::Scratch {
    let scratch = scratch().await;
    seed_rollups(&scratch.db, &[RollupSeed::scored(D0, 50)]).await;
    scratch
}

/// The badge row First Steps writes, marked or not.
fn first_steps(marked: bool) -> Vec<(String, i64, i64, bool)> {
    vec![("first_steps".to_owned(), 0, D0, marked)]
}

/// A recompute with the router's offers celebrates the new badge once, and a replay sends nothing.
async fn celebrated_once_and_replayed() {
    let scratch = first_day().await;
    let bot = Arc::new(RecordingBot::default());
    let offers = AwardOffers::new(router(&scratch.db, D0, &bot));
    let data = first_answer();
    recompute(&scratch.db, &data, Some(&offers), at(D0, 13)).await;
    assert_eq!(
        bot.sent().len(),
        1,
        "the new badge is sent: {:?}",
        bot.sent()
    );
    assert_eq!(badges(&scratch.db).await, first_steps(true), "and marked");
    recompute(&scratch.db, &data, Some(&offers), at(D0, 14)).await;
    assert_eq!(
        bot.sent().len(),
        1,
        "a replay sends nothing new: {:?}",
        bot.sent()
    );
}

/// An evaluation stopped after its write (no offers ran) sends exactly once at the next.
async fn stopped_after_the_write() {
    let scratch = first_day().await;
    let data = first_answer();
    recompute(&scratch.db, &data, None, at(D0, 13)).await;
    assert_eq!(
        badges(&scratch.db).await,
        first_steps(false),
        "written, owed"
    );
    let bot = Arc::new(RecordingBot::default());
    let offers = AwardOffers::new(router(&scratch.db, D0, &bot));
    recompute(&scratch.db, &data, Some(&offers), at(D0, 14)).await;
    assert_eq!(
        bot.sent().len(),
        1,
        "the owed badge is sent at the next: {:?}",
        bot.sent()
    );
    assert_eq!(badges(&scratch.db).await, first_steps(true), "and marked");
    recompute(&scratch.db, &data, Some(&offers), at(D0, 15)).await;
    assert_eq!(bot.sent().len(), 1, "once: {:?}", bot.sent());
}

/// A stop after the router answered but before the mark leaves the sends at one.
async fn stopped_after_the_router_answered() {
    let scratch = first_day().await;
    let data = first_answer();
    let bot = Arc::new(RecordingBot::default());
    let router = router(&scratch.db, D0, &bot);
    let stopped = AwardOffers::new(Arc::new(RouteThenFail(router.clone())));
    recompute(&scratch.db, &data, Some(&stopped), at(D0, 13)).await;
    assert_eq!(bot.sent().len(), 1, "the router sent it: {:?}", bot.sent());
    assert_eq!(
        badges(&scratch.db).await,
        first_steps(false),
        "no answer, no mark"
    );
    let offers = AwardOffers::new(router);
    recompute(&scratch.db, &data, Some(&offers), at(D0, 14)).await;
    assert_eq!(
        bot.sent().len(),
        1,
        "the once-ever key keeps it to one: {:?}",
        bot.sent()
    );
    assert_eq!(
        badges(&scratch.db).await,
        first_steps(true),
        "marked once answered"
    );
}

#[tokio::test]
async fn a_new_badge_is_celebrated_once_and_a_replay_raises_nothing() {
    celebrated_once_and_replayed().await;
    stopped_after_the_write().await;
    stopped_after_the_router_answered().await;
}

#[tokio::test]
async fn an_evaluation_stopped_after_the_write_sends_once_at_the_next() {
    stopped_after_the_write().await;
}

#[tokio::test]
async fn a_stop_after_the_router_answered_leaves_one_send_and_the_row_marked() {
    stopped_after_the_router_answered().await;
}

#[tokio::test]
async fn a_router_that_did_not_answer_leaves_the_award_due() {
    let scratch = first_day().await;
    let step = BadgesStep::new(Courses::default());
    let now = at(D0, 13);
    run_step(
        &scratch.db,
        &step,
        &first_answer(),
        0,
        (D0, Evaluation::Current),
        now,
    )
    .await;
    let silent = Recorder::silent();
    offer_badges(
        &silent,
        &scratch.db,
        UtcMillis::from_epoch_millis(now),
        day(D0),
    )
    .await
    .expect("the offers run");
    assert_eq!(
        silent.keys(),
        ["badge:first_steps:0"],
        "the owed badge is offered"
    );
    assert_eq!(
        badges(&scratch.db).await,
        first_steps(false),
        "no answer leaves it due"
    );
    let answered = Recorder::default();
    offer_badges(
        &answered,
        &scratch.db,
        UtcMillis::from_epoch_millis(now),
        day(D0),
    )
    .await
    .expect("the offers run");
    assert_eq!(answered.keys(), ["badge:first_steps:0"], "offered again");
    assert_eq!(
        badges(&scratch.db).await,
        first_steps(true),
        "marked once answered"
    );
}

#[tokio::test]
async fn an_owed_badge_is_raised_on_the_offers_day_and_marked_at_the_answer() {
    let scratch = first_day().await;
    let step = BadgesStep::new(Courses::default());
    run_step(
        &scratch.db,
        &step,
        &first_answer(),
        0,
        (D0, Evaluation::Current),
        at(D0, 13),
    )
    .await;
    // The badge of `D0` is still owed when the next day's offers run.
    let recorder = Recorder::default();
    let now = at(D0 + 1, 9);
    offer_badges(
        &recorder,
        &scratch.db,
        UtcMillis::from_epoch_millis(now),
        day(D0 + 1),
    )
    .await
    .expect("the offers run");
    let handed = recorder.handed();
    assert_eq!(
        recorder.keys(),
        ["badge:first_steps:0"],
        "the owed badge is offered"
    );
    assert_eq!(
        handed[0].text, "\u{1f45f} Badge earned: First Steps",
        "the line names the badge"
    );
    assert_eq!(
        handed[0].study_day,
        day(D0 + 1),
        "raised on the offers' day"
    );
    let mark: Option<i64> = sqlx::query_scalar(
        "SELECT celebrated_at FROM badges_earned WHERE badge_key = 'first_steps' AND tier = 0",
    )
    .fetch_one(scratch.db.reader())
    .await
    .expect("the mark reads");
    assert_eq!(mark, Some(now), "marked from the clock at the answer");
}

#[tokio::test]
async fn a_badge_marked_twice_keeps_its_first_mark() {
    let scratch = first_day().await;
    let step = BadgesStep::new(Courses::default());
    run_step(
        &scratch.db,
        &step,
        &first_answer(),
        0,
        (D0, Evaluation::Current),
        at(D0, 13),
    )
    .await;
    // Another offer marks the badge at 14:00 between this offer's read and its own mark at 15:00.
    let first = at(D0, 14);
    let port = MarksFirst::new(
        &scratch.db,
        "UPDATE badges_earned SET celebrated_at = ?1 WHERE badge_key = 'first_steps'",
        first,
    );
    offer_badges(
        &port,
        &scratch.db,
        UtcMillis::from_epoch_millis(at(D0, 15)),
        day(D0),
    )
    .await
    .expect("the offers run");
    let mark: Option<i64> = sqlx::query_scalar(
        "SELECT celebrated_at FROM badges_earned WHERE badge_key = 'first_steps' AND tier = 0",
    )
    .fetch_one(scratch.db.reader())
    .await
    .expect("the mark reads");
    assert_eq!(mark, Some(first), "the first mark is kept");
}

#[tokio::test]
async fn a_closing_day_is_judged_with_its_end_of_day_state() {
    let scratch = scratch().await;
    let mut closing = RollupSeed::scored(D0, 50);
    closing.score_at_close = Some(100);
    closing.card_state = Some(card_state(120, 1, 5, 3));
    let mut next = RollupSeed::scored(D0 + 1, 100);
    next.card_state = Some(card_state(0, 1, 5, 3));
    seed_rollups(&scratch.db, &[closing, next]).await;
    let data = collection(
        vec![
            answer(at(D0, 10), 1, 3, 5, 9_000),
            answer(at(D0, 10) + 1_000, 2, 3, 5, 9_000),
            answer(at(D0 + 1, 10), 3, 3, 5, 9_000),
        ],
        vec![card(1, 1), card(2, 1), card(3, 1)],
    );
    let step = BadgesStep::new(Courses::default());
    let settle = Evaluation::Settle { end_of_day: true };
    run_step(&scratch.db, &step, &data, 0, (D0, settle), at(D0 + 1, 12)).await;
    let expected: Vec<(String, i64, i64, bool)> =
        ["first_steps", "legendary_day", "maturity_milestone"]
            .into_iter()
            .map(|key| (key.to_owned(), 0, D0, false))
            .collect();
    assert_eq!(
        badges(&scratch.db).await,
        expected,
        "the closing day's score_at_close and its own card state decide it"
    );
}

// --- THE CLASS RULE ------------------------------------------------------------------------------

/// The study badges whose condition reads the card snapshot (masked on a day with none).
const SNAPSHOT_KEYS: [&str; 5] = [
    "inbox_zero",
    "backlog_slayer",
    "maturity_milestone",
    "forest_guardian",
    "leech_tamer",
];

/// The 24 study badges, by key.
const STUDY_KEYS: [&str; 24] = [
    "first_steps",
    "week_warrior",
    "monthly_monk",
    "century_flame",
    "year_of_iron",
    "grinder",
    "marathoner",
    "centurion_day",
    "sharpshooter",
    "sniper_elite",
    "inbox_zero",
    "backlog_slayer",
    "night_owl",
    "early_bird",
    "comeback_kid",
    "maturity_milestone",
    "forest_guardian",
    "leech_tamer",
    "polyglot",
    "globetrotter",
    "perfect_week",
    "legendary_day",
    "speed_demon",
    "iron_will",
];

/// How each member's day is evaluated.
const EVALUATIONS: [Evaluation; 2] = [Evaluation::Settle { end_of_day: true }, Evaluation::Current];

/// How many times each evaluation runs: the evaluation and its replay.
const PASSES: usize = 2;

/// One member: the inputs its day is built from. Every answer is a review-type first answer of a
/// card of its own; the day's young answers spread over `decks` decks, every other one is deck 1.
#[derive(Clone, Debug)]
struct Profile {
    label: String,
    base: u64,
    young: u64,
    young_failed: u64,
    decks: u64,
    mature: u64,
    mature_failed: u64,
    night: u64,
    early: u64,
    taken_ms: i64,
    prior: u64,
    streak: u32,
    comeback: bool,
    snapshot: Option<(i64, i64, i64, i64)>,
    score: i64,
    week: Vec<i64>,
}

/// The baseline: one young answer, a snapshot that earns nothing, a score of 50.
fn baseline(label: String) -> Profile {
    Profile {
        label,
        base: 0,
        young: 1,
        young_failed: 0,
        decks: 1,
        mature: 0,
        mature_failed: 0,
        night: 0,
        early: 0,
        taken_ms: 9_000,
        prior: 0,
        streak: 0,
        comeback: false,
        snapshot: Some((0, 1, 5, 3)),
        score: 50,
        week: Vec::new(),
    }
}

/// The values at `threshold - 1`, `threshold` and `threshold + 1`, kept within `floor..`.
fn around(threshold: i64, floor: i64) -> Vec<i64> {
    (threshold - 1..=threshold + 1)
        .filter(|value| *value >= floor)
        .collect()
}

/// A count of the population, always non-negative.
fn count(value: i64) -> u64 {
    u64::try_from(value).expect("a count")
}

/// Members varying one input of `condition` around `threshold`.
fn vary(
    condition: &str,
    threshold: i64,
    floor: i64,
    set: impl Fn(&mut Profile, i64),
) -> Vec<Profile> {
    around(threshold, floor)
        .into_iter()
        .map(|value| {
            let mut profile = baseline(format!("{condition} at {value}"));
            set(&mut profile, value);
            profile
        })
        .collect()
}

/// The lifetime, streak and day-volume members.
fn volume_members() -> Vec<Profile> {
    let mut members = vary("first_steps lifetime", 1, 0, |p, v| {
        p.young = count(v.min(1));
        p.decks = p.young;
        p.base = count(v - v.min(1));
    });
    for threshold in [7, 30, 100, 365] {
        members.extend(vary("streak", threshold, 0, |p, v| {
            p.streak = u32::try_from(v).expect("a streak");
        }));
    }
    for threshold in [1_000, 10_000] {
        members.extend(vary("lifetime", threshold, 1, |p, v| p.base = count(v - 1)));
    }
    members.extend(vary("centurion_day reviews", 100, 0, |p, v| {
        p.young = count(v);
    }));
    members.extend(vary("sharpshooter week reviews", 50, 0, |p, v| {
        p.young = count(v);
    }));
    members.extend(vary("sharpshooter retention", 90, 0, |p, v| {
        p.young = 100;
        p.young_failed = count(100 - v);
    }));
    members.extend(vary("sniper_elite mature answers", 50, 0, |p, v| {
        p.mature = count(v);
    }));
    members.extend(vary("sniper_elite retention", 95, 0, |p, v| {
        p.mature = 100;
        p.mature_failed = count(100 - v);
    }));
    members.extend(vary("night_owl", 50, 0, |p, v| p.night = count(v)));
    members.extend(vary("early_bird", 50, 0, |p, v| p.early = count(v)));
    members.extend(vary("speed_demon reviews", 100, 0, |p, v| {
        p.young = count(v);
        p.taken_ms = 5_000;
    }));
    members.extend(vary("speed_demon seconds", 6, 0, |p, v| {
        p.young = 100;
        p.taken_ms = v * 1_000;
    }));
    members.extend(vary("iron_will study days", 30, 1, |p, v| {
        p.prior = count(v - 1);
    }));
    members
}

/// The snapshot, comeback, deck and score members.
fn state_members() -> Vec<Profile> {
    let mut members = vary("inbox_zero backlog", 0, 0, |p, v| {
        p.snapshot = Some((0, 1, v, 0));
    });
    members.extend(vary("inbox_zero due", 0, 0, |p, v| {
        p.snapshot = Some((0, 1, 0, v));
    }));
    members.extend(vary("inbox_zero lifetime", 1, 0, |p, v| {
        p.snapshot = Some((0, 1, 0, 0));
        p.young = count(v.min(1));
        p.decks = p.young;
        p.base = count(v - v.min(1));
    }));
    members.extend(vary("backlog_slayer cleared", 200, 0, |p, v| {
        p.snapshot = Some((0, 1, 0, 3));
        p.young = count(v);
    }));
    members.extend(vary("comeback_kid streak", 1, 0, |p, v| {
        p.comeback = true;
        p.streak = u32::try_from(v).expect("a streak");
    }));
    let mut disarmed = baseline("comeback_kid disarmed".to_owned());
    disarmed.streak = 1;
    members.push(disarmed);
    for threshold in [100, 1_000] {
        members.extend(vary("mature cards", threshold, 0, |p, v| {
            p.snapshot = Some((v, 1, 5, 3));
        }));
    }
    members.extend(vary("leech_tamer leeches", 0, 0, |p, v| {
        p.snapshot = Some((0, v, 5, 3));
        p.base = 599;
    }));
    members.extend(vary("leech_tamer lifetime", 500, 1, |p, v| {
        p.snapshot = Some((0, 0, 5, 3));
        p.base = count(v - 1);
    }));
    for (condition, threshold) in [("polyglot decks", 3), ("globetrotter decks", 5)] {
        members.extend(vary(condition, threshold, 1, |p, v| {
            p.young = 6;
            p.decks = count(v);
        }));
    }
    members.extend(vary("perfect_week scores", 7, 1, |p, v| {
        p.score = 80;
        p.week = vec![80; usize::try_from(v - 1).expect("a length")];
    }));
    members.extend(vary("perfect_week lowest", 75, 0, |p, v| {
        p.score = 80;
        p.week = vec![80, 80, 80, 80, 80, v];
    }));
    members.extend(vary("legendary_day score", 100, 0, |p, v| {
        p.score = v.min(100);
    }));
    members.dedup_by(|a, b| a.label == b.label);
    let mut masked = baseline("no card state masks the snapshot badges".to_owned());
    masked.snapshot = None;
    masked.base = 599;
    members.push(masked);
    members
}

/// What `profile` earns, by its own arithmetic and the predecessor's literal thresholds.
fn earns(p: &Profile) -> BTreeSet<&'static str> {
    let day_reviews = p.young + p.mature + p.night + p.early;
    let lifetime = p.base + day_reviews + p.prior;
    let week_reviews = day_reviews + p.prior.min(6);
    let passed = week_reviews - p.young_failed - p.mature_failed;
    let others = p.mature + p.night + p.early;
    let day_decks = if p.young > 0 {
        p.decks
    } else {
        u64::from(others > 0)
    };
    let week_decks = day_decks.max(u64::from(p.prior > 0));
    let mut scores = vec![p.score];
    scores.extend(p.week.iter().copied());
    let week: Vec<i64> = scores.into_iter().take(7).collect();
    let (mature_count, leeches, backlog, due) = p.snapshot.unwrap_or((0, 0, 0, 0));
    let met = [
        ("first_steps", lifetime >= 1),
        ("week_warrior", p.streak >= 7),
        ("monthly_monk", p.streak >= 30),
        ("century_flame", p.streak >= 100),
        ("year_of_iron", p.streak >= 365),
        ("grinder", lifetime >= 1_000),
        ("marathoner", lifetime >= 10_000),
        ("centurion_day", day_reviews >= 100),
        (
            "sharpshooter",
            week_reviews >= 50 && passed * 100 >= 90 * week_reviews,
        ),
        (
            "sniper_elite",
            p.mature >= 50 && (p.mature - p.mature_failed) * 100 >= 95 * p.mature,
        ),
        ("inbox_zero", backlog == 0 && due == 0 && lifetime > 0),
        ("backlog_slayer", backlog == 0 && day_reviews >= 200),
        ("night_owl", p.night >= 50),
        ("early_bird", p.early >= 50),
        ("comeback_kid", p.comeback && p.streak > 0),
        ("maturity_milestone", mature_count >= 100),
        ("forest_guardian", mature_count >= 1_000),
        ("leech_tamer", leeches == 0 && lifetime >= 500),
        ("polyglot", day_decks >= 3),
        ("globetrotter", week_decks >= 5),
        (
            "perfect_week",
            week.len() >= 7 && week.iter().all(|score| *score >= 75),
        ),
        ("legendary_day", p.score >= 100),
        (
            "speed_demon",
            day_reviews >= 100 && 0 < p.taken_ms && p.taken_ms < 6_000,
        ),
        ("iron_will", p.prior >= 29),
    ];
    met.into_iter()
        .filter(|(key, earned)| *earned && (p.snapshot.is_some() || !SNAPSHOT_KEYS.contains(key)))
        .map(|(key, _)| key)
        .collect()
}

/// The window `profile` describes.
fn data_of(p: &Profile) -> CollectionData {
    let mut reviews = Vec::new();
    let mut cards = Vec::new();
    let mut add = |instant: i64, deck: i64, ease: i64, interval: i64, taken: i64| {
        let id = 1_000 + i64::try_from(reviews.len()).expect("an index");
        reviews.push(answer(instant + id * 1_000, id, ease, interval, taken));
        cards.push(card(id, deck));
    };
    let decks = i64::try_from(p.decks.max(1)).expect("decks");
    for i in 0..p.young {
        let i = i64::try_from(i).expect("an index");
        let ease = if i < i64::try_from(p.young_failed).expect("a count") {
            1
        } else {
            3
        };
        add(at(D0, 10), 1 + i % decks, ease, 5, p.taken_ms);
    }
    for i in 0..p.mature {
        let ease = if i < p.mature_failed { 1 } else { 3 };
        add(at(D0, 11), 1, ease, 30, p.taken_ms);
    }
    for _ in 0..p.night {
        add(at(D0, 1), 1, 3, 5, p.taken_ms);
    }
    for _ in 0..p.early {
        add(at(D0, 5), 1, 3, 5, p.taken_ms);
    }
    for back in 1..=p.prior {
        let back = i64::try_from(back).expect("a day");
        add(at(D0 - back, 10), 1, 3, 5, 9_000);
    }
    collection(reviews, cards)
}

/// The rollups `profile` describes: its day, and the stored scores of the days before it.
fn rollups_of(p: &Profile) -> Vec<RollupSeed> {
    let mut own = RollupSeed::scored(D0, p.score);
    own.score_at_close = Some(p.score);
    own.card_state = p
        .snapshot
        .map(|(mature, leeches, backlog, due)| card_state(mature, leeches, backlog, due));
    let mut seeds = vec![own];
    for (back, score) in (1..).zip(p.week.iter().copied()) {
        seeds.push(RollupSeed::scored(D0 - back, score));
    }
    seeds
}

/// Judges `profile` evaluated as `evaluation`, twice, in a database of its own; answers the
/// evaluated days it judged.
async fn judge(profile: &Profile, evaluation: Evaluation) -> usize {
    let scratch = scratch().await;
    let db = &scratch.db;
    seed_rollups(db, &rollups_of(profile)).await;
    if profile.streak > 0 || profile.comeback {
        seed_streak(db, profile.streak, profile.comeback, D0).await;
    }
    let data = data_of(profile);
    let step = BadgesStep::new(Courses::default());
    let (now, today) = match evaluation {
        Evaluation::Current => (at(D0, 3), D0),
        _ => (at(D0 + 1, 12), D0 + 1),
    };
    let label = format!("{} ({evaluation:?})", profile.label);
    let expected: BTreeSet<String> = earns(profile).into_iter().map(str::to_owned).collect();
    let mut judged = 0;
    for pass in 1..=PASSES {
        run_step(db, &step, &data, profile.base, (D0, evaluation), now).await;
        let recorder = Recorder::default();
        offer_badges(&recorder, db, UtcMillis::from_epoch_millis(now), day(today))
            .await
            .expect("the offers run");
        let rows = badges(db).await;
        let awarded: BTreeSet<String> = rows.iter().map(|row| row.0.clone()).collect();
        assert_eq!(awarded, expected, "{label}, pass {pass}: the awarded set");
        assert!(
            rows.iter().all(|row| row.1 == 0 && row.2 == D0 && row.3),
            "{label}: {rows:?}"
        );
        let raised: Vec<String> = if pass == 1 {
            expected
                .iter()
                .map(|key| format!("badge:{key}:0"))
                .collect()
        } else {
            Vec::new()
        };
        let mut keys = recorder.keys();
        keys.sort();
        assert_eq!(
            keys, raised,
            "{label}, pass {pass}: the celebrations raised"
        );
        judged += 1;
    }
    judged
}

#[tokio::test]
async fn every_evaluated_day_awards_exactly_the_badges_its_context_earns_once() {
    let mut members = volume_members();
    members.extend(state_members());
    // The population is the class only if every study condition is met by a member and missed by
    // another.
    for key in STUDY_KEYS {
        let earned = members.iter().filter(|p| earns(p).contains(key)).count();
        assert!(
            earned > 0 && earned < members.len(),
            "{key} is earned by {earned} member(s)"
        );
    }
    let derived = members.len() * EVALUATIONS.len() * PASSES;
    let mut examined = 0;
    for profile in &members {
        for evaluation in EVALUATIONS {
            examined += judge(profile, evaluation).await;
        }
    }
    println!("examined {examined} evaluated day(s)");
    assert_eq!(examined, derived, "members x evaluations x passes");
}

/// The fold reports the badge step under its name, `progression.badges`, in phase 7.
#[test]
fn the_badge_step_is_named_progression_badges() {
    assert_eq!(
        badge_fold().steps(),
        [(Phase::Awards, "progression.badges")],
        "the step's phase and name"
    );
}

/// The awards' offers print their type and leave out the router they hand each celebration to.
#[test]
fn the_award_offers_print_their_type_without_their_router() {
    let offers = AwardOffers::new(Arc::new(Recorder::default()));
    assert_eq!(format!("{offers:?}"), "AwardOffers { .. }");
}

/// The offers in turn print each offer they hold, in the order they run it.
#[test]
fn the_offers_in_turn_print_each_offer_in_order() {
    let offers = deck_streak_coordination::recompute::OffersInTurn::new(vec![
        Box::new(AwardOffers::new(Arc::new(Recorder::default()))),
        Box::new(AwardOffers::new(Arc::new(Recorder::default()))),
    ]);
    assert_eq!(
        format!("{offers:?}"),
        "[AwardOffers { .. }, AwardOffers { .. }]"
    );
}

/// The landmarks' offers print their type and how many study days they hold, and leave out the
/// router they hand each landmark to.
#[test]
fn the_landmark_offers_print_their_study_days_without_their_router() {
    let offers = deck_streak_coordination::recompute::landmarks::LandmarkOffers::new(
        Arc::new(Recorder::default()),
        vec![
            deck_streak_kernel::StudyDay::from_epoch_day(20_000),
            deck_streak_kernel::StudyDay::from_epoch_day(20_001),
        ],
        &collection(Vec::new(), Vec::new()),
        StudyDayRule::default(),
    );
    assert_eq!(
        format!("{offers:?}"),
        "LandmarkOffers { study_days: 2, .. }"
    );
}

/// An offer call whose settle cursor is already the landmarks' cursor takes no write: made while
/// another write holds the lock, it answers at once rather than waiting out the busy timeout
/// (ADR-303, ADR-322).
#[tokio::test]
async fn an_offer_with_nothing_to_move_takes_no_write() {
    use deck_streak_coordination::recompute::landmarks::LandmarkOffers;
    let scratch = scratch().await;
    let db = &scratch.db;
    seed_rollups(db, &[RollupSeed::scored(D0 + 5, 0)]).await;
    let mut write = db.write().await.expect("a write");
    deck_streak_analytics::rollup::record_settled(
        &mut write,
        day(D0 + 5),
        UtcMillis::from_epoch_millis(at(D0 + 5, 23)),
    )
    .await
    .expect("the day settles");
    write.commit().await.expect("the settle commits");
    let study_days = vec![day(D0 + 1), day(D0 + 2)];
    let data = collection(Vec::new(), Vec::new());
    let (now, today) = (UtcMillis::from_epoch_millis(at(D0 + 6, 12)), day(D0 + 6));
    // The first recompute seeds the mark and the cursor at yesterday, the day settled.
    LandmarkOffers::new(
        Arc::new(Recorder::default()),
        study_days.clone(),
        &data,
        StudyDayRule::default(),
    )
    .offer(db, now, today)
    .await
    .expect("the first recompute offers");
    let offers = LandmarkOffers::new(
        Arc::new(Recorder::default()),
        study_days,
        &data,
        StudyDayRule::default(),
    );
    offers
        .offer(db, now, today)
        .await
        .expect("the second recompute offers");
    let held = db.write().await.expect("another write holds the lock");
    let answer = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        offers.offer(db, now, today),
    )
    .await;
    drop(held);
    assert!(
        matches!(answer, Ok(Ok(()))),
        "an offer with nothing to move waited on the write lock: {answer:?}"
    );
}
