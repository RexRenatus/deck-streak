//! The streak-break cap reaches production (SPEC-326 A1 to A4 and A7; SPEC-084 R5, R11; #572): a
//! stored language streak that broke on the study day caps every celebration the cycle routes, and
//! every held celebration the scheduled flush renders, at the policy's cap; a stored streak that
//! cannot be read routes and flushes nothing. Each criterion has its control: the same occasion on
//! a database with no stored streak, or the same held row flushed with the facts handed over by
//! hand. Every clock is a `ManualClock` at noon, outside the quiet window; every value is
//! synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::held_flush::HeldFlushWork;
use deck_streak_coordination::jobs::{FireDate, HELD_FLUSH};
use deck_streak_coordination::level_up::announce_level_up;
use deck_streak_coordination::recompute::{Celebrate, Celebration};
use deck_streak_coordination::relight::announce_relight;
use deck_streak_coordination::runner::{Done, Fire, Reason, Work};
use deck_streak_kernel::{Clock, Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::ladder::REACTION_EMOJI;
use deck_streak_notifications::owner_message;
use deck_streak_notifications::{
    BotTransport, Pass, Policy, PushFuture, Pushed, Router, StreakFacts,
};
use deck_streak_progression::xp::{XpTotal, level_for};
use deck_streak_streaks::store::upsert_state;
use deck_streak_streaks::streak::StreakState;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
/// The study day every criterion is raised on.
const DAY: i64 = 20_000;
/// The owner's latest message, which a T1 render reacts to.
const MESSAGE_ID: i64 = 4_242;
/// The held celebration's line.
const HELD_TEXT: &str = "synthetic held celebration";
/// The held celebration's key.
const HELD_KEY: &str = "badge:synthetic:held";

/// Noon of `DAY`, outside the quiet window.
fn noon() -> UtcMillis {
    UtcMillis::from_epoch_millis(DAY * DAY_MS + 12 * 60 * MINUTE_MS)
}

/// The facts of a streak that broke on `DAY`: back to one day, once five long.
fn the_break() -> StreakFacts {
    StreakFacts {
        last_study_day: Some(StudyDay::from_epoch_day(DAY)),
        current: 1,
        longest: 5,
    }
}

/// A bot that records every call it is made, each answered delivered.
#[derive(Default)]
struct Calls(Mutex<Vec<String>>);

impl Calls {
    fn seen(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn note(&self, call: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

impl BotTransport for Calls {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            self.note(format!("message: {text}"));
            Pushed::Delivered
        })
    }

    fn push_reaction<'a>(
        &'a self,
        _pass: &'a Pass,
        message_id: i64,
        emoji: &'a str,
    ) -> PushFuture<'a> {
        Box::pin(async move {
            self.note(format!("reaction to {message_id}: {emoji}"));
            Pushed::Delivered
        })
    }
}

struct World {
    db: Db,
    clock: Arc<ManualClock>,
    _scratch: tempfile::TempDir,
}

impl World {
    /// A migrated database with no stored streak.
    async fn new() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        Self {
            db,
            clock: Arc::new(ManualClock::new(noon())),
            _scratch: scratch,
        }
    }

    /// A database whose stored language streak broke on `DAY`.
    async fn with_a_break() -> Self {
        let world = Self::new().await;
        let state = StreakState {
            current: 1,
            longest: 5,
            freezes: 0,
            last_study_day: Some(StudyDay::from_epoch_day(DAY)),
            comeback_armed: false,
        };
        let mut write = world.db.write().await.expect("a write");
        upsert_state(&mut write, "language", &state, noon())
            .await
            .expect("the stored streak");
        write.commit().await.expect("committed");
        world
    }

    fn router(&self) -> Router {
        Router::new(
            Arc::new(Policy::compiled().expect("the compiled policy parses")),
            self.db.clone(),
            self.clock.clone(),
            StudyDayRule::default(),
        )
    }

    /// The cycle's router: no transport, so a celebration is held for the senders (SPEC-319).
    fn holding(&self) -> Router {
        self.router().holding()
    }

    /// A sender's router: the bot joined.
    fn joined(&self, bot: Arc<Calls>) -> Router {
        self.router().with_bot(bot)
    }

    /// A T2 celebration held `quiet` an hour before noon on `DAY`, and the owner's message ten
    /// minutes before noon, so a T1 render is a reaction to it.
    async fn hold_a_celebration(&self) {
        let held_at = noon().epoch_millis() - 60 * MINUTE_MS;
        let mut write = self.db.write().await.expect("a write");
        sqlx::query(
            "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, \
             tier_pending, text, hold, tries, state, deferred_at, study_day, created_at) \
             VALUES ('celebration', ?, 'bot', 'T2', 'T2', ?, 'quiet', 0, 'held', ?, ?, ?)",
        )
        .bind(HELD_KEY)
        .bind(HELD_TEXT)
        .bind(held_at)
        .bind(DAY)
        .bind(held_at)
        .execute(&mut *write)
        .await
        .expect("a held row");
        write.commit().await.expect("committed");
        let written = UtcMillis::from_epoch_millis(noon().epoch_millis() - 10 * MINUTE_MS);
        owner_message::record(&self.db, MESSAGE_ID, written)
            .await
            .expect("the owner's message");
    }

    /// Makes the stored streak unreadable: its table is renamed away.
    async fn hide_the_streak(&self) {
        let mut write = self.db.write().await.expect("a write");
        sqlx::query("ALTER TABLE streak_state RENAME TO streak_state_unreadable")
            .execute(&mut *write)
            .await
            .expect("the table renamed");
        write.commit().await.expect("committed");
    }

    /// The tier each queue row under `key` is held at.
    async fn tier_pending(&self, key: &str) -> Vec<String> {
        sqlx::query_scalar("SELECT tier_pending FROM notification_queue WHERE dedupe_key = ?")
            .bind(key)
            .fetch_all(self.db.reader())
            .await
            .expect("the queue reads")
    }

    /// The state of each queue row under `key`.
    async fn queue_state(&self, key: &str) -> Vec<String> {
        sqlx::query_scalar("SELECT state FROM notification_queue WHERE dedupe_key = ?")
            .bind(key)
            .fetch_all(self.db.reader())
            .await
            .expect("the queue reads")
    }

    /// How many decisions the router recorded under `key`.
    async fn decisions(&self, key: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_decisions WHERE dedupe_key = ?")
            .bind(key)
            .fetch_one(self.db.reader())
            .await
            .expect("the decisions read")
    }

    /// The scheduled flush job's answer, fired now through `router`.
    async fn held_flush(&self, router: &Router) -> Result<Done, Reason> {
        let at = self.clock.now();
        let rule = StudyDayRule::default();
        let fire = Fire {
            job: HELD_FLUSH,
            fire_date: FireDate::of(at, rule.utc_offset()),
            scheduled_at: at,
            started_at: at,
            study_day: rule.study_day(at),
            sync: None,
        };
        HeldFlushWork::new(router).perform(&fire).await
    }
}

/// What the bot is made to do when a database holding the same celebration and message, with no
/// stored streak, is flushed with `facts` handed over by hand.
async fn flushed_by_hand(facts: Option<StreakFacts>) -> Vec<String> {
    let twin = World::new().await;
    twin.hold_a_celebration().await;
    let bot = Arc::new(Calls::default());
    let _ = twin
        .joined(bot.clone())
        .flush_with(facts)
        .await
        .expect("the twin flushes");
    bot.seen()
}

fn an_award() -> Celebration {
    Celebration {
        event: "badge",
        key: "badge:synthetic:1".to_owned(),
        text: "synthetic badge".to_owned(),
        study_day: StudyDay::from_epoch_day(DAY),
    }
}

#[tokio::test]
async fn an_award_raised_on_a_streak_break_day_is_decided_at_most_t1() {
    let broke = World::with_a_break().await;
    let control = World::new().await;
    let award = an_award();

    broke
        .holding()
        .celebrate(&award)
        .await
        .expect("the router answers");
    control
        .holding()
        .celebrate(&award)
        .await
        .expect("the router answers");

    assert_eq!(
        broke.tier_pending(&award.key).await,
        ["T1"],
        "a break on the award's day caps it at the policy's cap"
    );
    assert_eq!(
        control.tier_pending(&award.key).await,
        ["T2"],
        "with no stored streak the award is held at its event's tier"
    );
}

#[tokio::test]
async fn the_level_up_on_a_streak_break_day_is_decided_at_most_t1() {
    let broke = World::with_a_break().await;
    let control = World::new().await;
    let before = level_for(XpTotal::new(0));
    let after = level_for(XpTotal::new(350));
    let day = StudyDay::from_epoch_day(DAY);
    let key = format!("level:{}", after.get());

    announce_level_up(&broke.holding(), before, after, day)
        .await
        .expect("the router answers");
    announce_level_up(&control.holding(), before, after, day)
        .await
        .expect("the router answers");

    assert_eq!(
        broke.tier_pending(&key).await,
        ["T1"],
        "a break on the day caps the level-up at the policy's cap"
    );
    assert_eq!(
        control.tier_pending(&key).await,
        ["T2"],
        "with no stored streak the level-up is held at its own tier"
    );
}

#[tokio::test]
async fn the_relight_on_its_streak_break_day_is_decided_at_most_t1() {
    let broke = World::with_a_break().await;
    let control = World::new().await;
    let day = StudyDay::from_epoch_day(DAY);
    let key = format!("relight:{DAY}");

    announce_relight(&broke.holding(), day, day)
        .await
        .expect("the router answers");
    announce_relight(&control.holding(), day, day)
        .await
        .expect("the router answers");

    assert_eq!(
        broke.tier_pending(&key).await,
        ["T1"],
        "a break on the relight's day caps it at the policy's cap"
    );
    assert_eq!(
        control.tier_pending(&key).await,
        ["T2"],
        "with no stored streak the relight is held at its own tier"
    );
}

#[tokio::test]
async fn the_held_flush_job_re_caps_with_the_stored_streak() {
    let world = World::with_a_break().await;
    world.hold_a_celebration().await;
    let bot = Arc::new(Calls::default());
    let capped = flushed_by_hand(Some(the_break())).await;
    let uncapped = flushed_by_hand(None).await;

    let answer = world.held_flush(&world.joined(bot.clone())).await;

    assert_eq!(
        bot.seen(),
        capped,
        "the job re-caps with the stored streak, as a flush handed the break's facts does"
    );
    assert_ne!(
        bot.seen(),
        uncapped,
        "a flush with no facts renders past the cap"
    );
    assert_eq!(
        capped,
        [format!("reaction to {MESSAGE_ID}: {REACTION_EMOJI}")],
        "the capped render is one reaction to the owner's message"
    );
    assert_eq!(
        uncapped,
        [format!("message: {HELD_TEXT}")],
        "the uncapped render is the held line"
    );
    assert_eq!(answer, Ok(Done::Done));
}

#[tokio::test]
async fn an_unreadable_streak_leaves_the_award_owed_and_the_hold_held() {
    let world = World::with_a_break().await;
    world.hold_a_celebration().await;
    world.hide_the_streak().await;
    let award = an_award();
    let bot = Arc::new(Calls::default());

    let offered = world.holding().celebrate(&award).await;
    let flushed = world.held_flush(&world.joined(bot.clone())).await;

    assert!(
        offered.is_err(),
        "a streak that cannot be read leaves the award unanswered, so it stays owed: {offered:?}"
    );
    assert_eq!(
        world.decisions(&award.key).await,
        0,
        "no decision is recorded"
    );
    assert_eq!(
        flushed,
        Err(Reason::new("flush_failed")),
        "a flush whose facts cannot be read does not run"
    );
    assert_eq!(
        world.queue_state(HELD_KEY).await,
        ["held"],
        "the queue keeps its hold"
    );
    assert_eq!(bot.seen(), Vec::<String>::new(), "the bot is made no call");
}
