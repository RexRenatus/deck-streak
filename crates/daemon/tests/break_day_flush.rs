//! The bot's flush re-caps with the stored streak (SPEC-326 A5; SPEC-084 R11; #572): after the
//! owner's sync, the bot's `Flush` for its router renders each held celebration at the cap of a
//! streak that broke on the flush's study day, exactly as a flush handed the break's facts by hand
//! does, and never as a flush with no facts does. Every clock is a `ManualClock` at noon, outside
//! the quiet window; every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_daemon::sync_request::Flush;
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::ladder::REACTION_EMOJI;
use deck_streak_notifications::owner_message;
use deck_streak_notifications::{
    BotTransport, Pass, Policy, PushFuture, Pushed, Router, StreakFacts,
};
use deck_streak_streaks::store::upsert_state;
use deck_streak_streaks::streak::StreakState;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
/// The study day the flush runs on.
const DAY: i64 = 20_000;
/// The owner's latest message, which a T1 render reacts to.
const MESSAGE_ID: i64 = 4_242;
/// The held celebration's line.
const HELD_TEXT: &str = "synthetic held celebration";

/// Noon of `DAY`, outside the quiet window.
fn noon() -> UtcMillis {
    UtcMillis::from_epoch_millis(DAY * DAY_MS + 12 * 60 * MINUTE_MS)
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

/// A migrated database holding a T2 celebration `quiet` from an hour before noon on `DAY`, with
/// the owner's message ten minutes before noon, so a T1 render is a reaction to it.
struct World {
    db: Db,
    _scratch: tempfile::TempDir,
}

impl World {
    async fn new() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        let held_at = noon().epoch_millis() - 60 * MINUTE_MS;
        let mut write = db.write().await.expect("a write");
        sqlx::query(
            "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, \
             tier_pending, text, hold, tries, state, deferred_at, study_day, created_at) \
             VALUES ('celebration', 'badge:synthetic:held', 'bot', 'T2', 'T2', ?, 'quiet', 0, \
             'held', ?, ?, ?)",
        )
        .bind(HELD_TEXT)
        .bind(held_at)
        .bind(DAY)
        .bind(held_at)
        .execute(&mut *write)
        .await
        .expect("a held row");
        write.commit().await.expect("committed");
        let written = UtcMillis::from_epoch_millis(noon().epoch_millis() - 10 * MINUTE_MS);
        owner_message::record(&db, MESSAGE_ID, written)
            .await
            .expect("the owner's message");
        Self {
            db,
            _scratch: scratch,
        }
    }

    /// Stores a language streak that broke on `DAY`: back to one day, once five long.
    async fn store_the_break(&self) {
        let state = StreakState {
            current: 1,
            longest: 5,
            freezes: 0,
            last_study_day: Some(StudyDay::from_epoch_day(DAY)),
            comeback_armed: false,
        };
        let mut write = self.db.write().await.expect("a write");
        upsert_state(&mut write, "language", &state, noon())
            .await
            .expect("the stored streak");
        write.commit().await.expect("committed");
    }

    /// The bot's router: the bot joined, on a clock at noon.
    fn joined(&self, bot: Arc<Calls>) -> Router {
        Router::new(
            Arc::new(Policy::compiled().expect("the compiled policy parses")),
            self.db.clone(),
            Arc::new(ManualClock::new(noon())),
            StudyDayRule::default(),
        )
        .with_bot(bot)
    }
}

/// What the bot is made to do when a database holding the same celebration and message, with no
/// stored streak, is flushed with `facts` handed over by hand.
async fn flushed_by_hand(facts: Option<StreakFacts>) -> Vec<String> {
    let twin = World::new().await;
    let bot = Arc::new(Calls::default());
    let _ = twin
        .joined(bot.clone())
        .flush_with(facts)
        .await
        .expect("the twin flushes");
    bot.seen()
}

#[tokio::test]
async fn the_bots_flush_re_caps_with_the_stored_streak() {
    let world = World::new().await;
    world.store_the_break().await;
    let bot = Arc::new(Calls::default());
    let the_break = StreakFacts {
        last_study_day: Some(StudyDay::from_epoch_day(DAY)),
        current: 1,
        longest: 5,
    };
    let capped = flushed_by_hand(Some(the_break)).await;
    let uncapped = flushed_by_hand(None).await;

    Flush::flush(&Arc::new(world.joined(bot.clone())))
        .await
        .expect("the bot's flush runs");

    assert_eq!(
        bot.seen(),
        capped,
        "the bot's flush re-caps with the stored streak, as a flush handed the break's facts does"
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
}
