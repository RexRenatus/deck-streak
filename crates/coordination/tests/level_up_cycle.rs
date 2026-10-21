//! A sync cycle announces a level reached once (SPEC-072 A30; R14): one cycle whose fold settles
//! enough XP to cross two levels raises ONE level-up line through the router, and a later cycle
//! over the same XP raises none. The line is the level after the recompute against the level
//! before it, through the composed cycle, so a call that hands the router the two levels the wrong
//! way round is refused here and nowhere else.
//!
//! The cycle's own flush re-caps with the stored streak (SPEC-326 A6; SPEC-084 R11): after a sync
//! that ran and succeeded, a held celebration renders at the cap of a streak that broke on the
//! flush's study day, as a flush handed the break's facts by hand renders it.

#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::recompute::{DayEvaluation, DayStep, Evaluation, Fold, Phase};
use deck_streak_coordination::sync_cycle::{CycleParts, sync_cycle};
use deck_streak_ingest::engine::{
    AnkiEngine, EngineError, NewCardQueue, RslibEngine, SyncLogin, SyncOutcome,
};
use deck_streak_ingest::gate::ChangeGate;
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{
    STATE_DIRECTORY, SYNC_ENDPOINT, SYNC_PASSWORD, SYNC_USERNAME, ScopeSettings, SyncSettings,
};
use deck_streak_ingest::sync::{RetrySchedule, SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload, OffloadWorkers,
    PortFuture, Redactor, StudyDay, StudyDayRule, Track, UtcMillis,
};
use deck_streak_notifications::ladder::REACTION_EMOJI;
use deck_streak_notifications::owner_message;
use deck_streak_notifications::{
    BotTransport, Pass, Policy, PushFuture, Pushed, Router, StreakFacts,
};
use deck_streak_progression::level::level_info;
use deck_streak_progression::settle::{SettleCause, SettleRequest, settle};
use deck_streak_progression::xp::XpTotal;
use deck_streak_streaks::store::upsert_state;
use deck_streak_streaks::streak::StreakState;
use sqlx::SqliteConnection;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
const DAY: i64 = 20_000;
/// Level 1 to level 3: the curve's starts are 0, 100, 300.
const SETTLED: u32 = 350;
/// The owner's latest message, which a T1 render reacts to.
const MESSAGE_ID: i64 = 4_242;
/// The held celebration's line.
const HELD_TEXT: &str = "synthetic held celebration";

#[derive(Clone, Copy)]
struct Engine;

impl AnkiEngine for Engine {
    fn new_card_queue(&self, _collection: &Path) -> Result<NewCardQueue, EngineError> {
        Ok(NewCardQueue::default())
    }

    async fn normal_sync(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<SyncOutcome, EngineError> {
        Ok(SyncOutcome::NoChanges)
    }

    async fn full_download(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<(), EngineError> {
        Ok(())
    }
}

#[derive(Default)]
struct Recording(Mutex<Vec<String>>);

impl Recording {
    fn pushes(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl BotTransport for Recording {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(text.to_owned());
            Pushed::Delivered
        })
    }
}

/// A base-XP step that, on whichever day is current, settles `SETTLED` review XP on `DAY`.
struct Grant;

impl DayStep for Grant {
    fn phase(&self) -> Phase {
        Phase::BaseXp
    }

    fn name(&self) -> &'static str {
        "level_up_cycle.grant"
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if matches!(day.evaluation, Evaluation::Current) {
                let request = SettleRequest {
                    study_day: StudyDay::from_epoch_day(DAY),
                    source: "reviews",
                    track: Track::Language,
                    amount: SETTLED,
                    closed: false,
                };
                settle(write, &request, SettleCause::Recompute, day.facts.now)
                    .await
                    .expect("the step settles");
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn a_sync_cycle_announces_a_level_reached_once() {
    let scratch = tempfile::tempdir().expect("a scratch");
    let state = scratch.path().join("state");
    let credentials = scratch.path().join("credentials");
    for folder in [&state, &credentials] {
        fs::create_dir_all(folder).expect("a folder");
    }
    for (id, value) in [
        (SYNC_USERNAME, "synthetic-owner\n"),
        (SYNC_PASSWORD, "synthetic-password\n"),
    ] {
        fs::write(credentials.join(id), value).expect("a credential");
    }
    let settings = SyncSettings::from_env(&Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, state.as_os_str()),
    ]))
    .expect("the settings");
    RslibEngine
        .new_card_queue(&settings.copy_path())
        .expect("the engine creates the copy");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    let noon = UtcMillis::from_epoch_millis(DAY * DAY_MS + 12 * 60 * MINUTE_MS);
    let clock = Arc::new(ManualClock::new(noon));
    let loader = CredentialLoader::new(
        CredentialsDirectory::new(credentials).expect("an absolute credentials directory"),
        Redactor::new(),
    );
    let syncer = Syncer::new(
        Engine,
        SqliteSyncRuns::new(db.clone()),
        settings.clone(),
        loader,
        clock.clone(),
        StudyDayRule::default(),
    )
    .with_schedule(RetrySchedule::IMMEDIATE);
    let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock.clone());
    let reader = CollectionReader::new(&settings, ScopeSettings::default(), offload);
    let gate = ChangeGate::new(db.clone(), StudyDayRule::default(), clock.clone());
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let bot = Arc::new(Recording::default());
    let router = Router::new(policy, db.clone(), clock.clone(), StudyDayRule::default())
        .with_bot(bot.clone());
    let mut fold = Fold::default();
    fold.register(Phase::BaseXp, Box::new(Grant))
        .expect("the step in its phase");
    let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
        .with_flush(Arc::new(router))
        .with_fold(Arc::new(fold), db.clone(), StudyDayRule::default(), None);

    let first = sync_cycle(&cycle, Trigger::Owner)
        .await
        .expect("the cycle runs");
    assert!(
        matches!(
            first.recompute,
            deck_streak_coordination::sync_cycle::Recompute::Ran { .. }
        ),
        "the first cycle runs the fold"
    );
    let reached = level_info(XpTotal::new(u64::from(SETTLED)));
    assert_eq!(reached.level.get(), 3, "the settle crosses two levels");
    let line = format!(
        "{} Level {}: {}",
        reached.emoji,
        reached.level.get(),
        reached.title
    );
    assert_eq!(
        bot.pushes(),
        std::slice::from_ref(&line),
        "one line, for the level reached"
    );

    // The next day: the rollover recomputes, and the fold settles the same amount again.
    clock.set(UtcMillis::from_epoch_millis(
        (DAY + 1) * DAY_MS + 12 * 60 * MINUTE_MS,
    ));
    let second = sync_cycle(&cycle, Trigger::Owner)
        .await
        .expect("the cycle runs");
    assert!(
        matches!(
            second.recompute,
            deck_streak_coordination::sync_cycle::Recompute::Ran { .. }
        ),
        "the second cycle runs the fold"
    );
    assert_eq!(
        bot.pushes(),
        [line],
        "a cycle that crosses no level pushes nothing"
    );
}

/// A bot that records every call it is made, each answered delivered: a line or a reaction.
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

/// Holds a T2 celebration `quiet` in `db` from an hour before `now` on `DAY`, and records the
/// owner's message ten minutes before `now`, so a T1 render is a reaction to it.
async fn hold_a_celebration(db: &Db, now: UtcMillis) {
    let held_at = now.epoch_millis() - 60 * MINUTE_MS;
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
    let written = UtcMillis::from_epoch_millis(now.epoch_millis() - 10 * MINUTE_MS);
    owner_message::record(db, MESSAGE_ID, written)
        .await
        .expect("the owner's message");
}

/// What the bot is made to do when a database holding the same celebration and message, with no
/// stored streak, is flushed at `now` with `facts` handed over by hand.
async fn flushed_by_hand(now: UtcMillis, facts: Option<StreakFacts>) -> Vec<String> {
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    hold_a_celebration(&db, now).await;
    let bot = Arc::new(Calls::default());
    let _ = Router::new(
        Arc::new(Policy::compiled().expect("the compiled policy parses")),
        db,
        Arc::new(ManualClock::new(now)),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone())
    .flush_with(facts)
    .await
    .expect("the twin flushes");
    bot.seen()
}

#[tokio::test]
async fn a_sync_cycles_flush_re_caps_with_the_stored_streak() {
    let scratch = tempfile::tempdir().expect("a scratch");
    let state = scratch.path().join("state");
    let credentials = scratch.path().join("credentials");
    for folder in [&state, &credentials] {
        fs::create_dir_all(folder).expect("a folder");
    }
    for (id, value) in [
        (SYNC_USERNAME, "synthetic-owner\n"),
        (SYNC_PASSWORD, "synthetic-password\n"),
    ] {
        fs::write(credentials.join(id), value).expect("a credential");
    }
    let settings = SyncSettings::from_env(&Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, state.as_os_str()),
    ]))
    .expect("the settings");
    RslibEngine
        .new_card_queue(&settings.copy_path())
        .expect("the engine creates the copy");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    let noon = UtcMillis::from_epoch_millis(DAY * DAY_MS + 12 * 60 * MINUTE_MS);
    let clock = Arc::new(ManualClock::new(noon));
    hold_a_celebration(&db, noon).await;
    let broke = StreakState {
        current: 1,
        longest: 5,
        freezes: 0,
        last_study_day: Some(StudyDay::from_epoch_day(DAY)),
        comeback_armed: false,
    };
    let mut write = db.write().await.expect("a write");
    upsert_state(&mut write, "language", &broke, noon)
        .await
        .expect("the stored streak");
    write.commit().await.expect("committed");
    let loader = CredentialLoader::new(
        CredentialsDirectory::new(credentials).expect("an absolute credentials directory"),
        Redactor::new(),
    );
    let syncer = Syncer::new(
        Engine,
        SqliteSyncRuns::new(db.clone()),
        settings.clone(),
        loader,
        clock.clone(),
        StudyDayRule::default(),
    )
    .with_schedule(RetrySchedule::IMMEDIATE);
    let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock.clone());
    let reader = CollectionReader::new(&settings, ScopeSettings::default(), offload);
    let gate = ChangeGate::new(db.clone(), StudyDayRule::default(), clock.clone());
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let bot = Arc::new(Calls::default());
    let router = Router::new(policy, db.clone(), clock.clone(), StudyDayRule::default())
        .with_bot(bot.clone());
    // No fold: the flush runs after the sync and before any fold, and nothing rewrites the streak.
    let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
        .with_flush(Arc::new(router));
    let the_break = StreakFacts {
        last_study_day: Some(StudyDay::from_epoch_day(DAY)),
        current: 1,
        longest: 5,
    };
    let capped = flushed_by_hand(noon, Some(the_break)).await;
    let uncapped = flushed_by_hand(noon, None).await;

    let report = sync_cycle(&cycle, Trigger::Owner)
        .await
        .expect("the cycle runs");

    assert_eq!(
        bot.seen(),
        capped,
        "the cycle's flush re-caps with the stored streak, as a flush handed the break's facts does"
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
    assert!(
        matches!(&report.sync, SyncReport::Ran { run, .. } if run.outcome.is_ok()),
        "the sync ran and succeeded, so the cycle flushed"
    );
}
