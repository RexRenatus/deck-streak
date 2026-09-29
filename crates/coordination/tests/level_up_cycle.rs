//! A sync cycle announces a level reached once (SPEC-072 A30; R14): one cycle whose fold settles
//! enough XP to cross two levels raises ONE level-up line through the router, and a later cycle
//! over the same XP raises none. The line is the level after the recompute against the level
//! before it, through the composed cycle, so a call that hands the router the two levels the wrong
//! way round is refused here and nowhere else.

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
use deck_streak_ingest::sync::{RetrySchedule, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload, OffloadWorkers,
    PortFuture, Redactor, StudyDay, StudyDayRule, Track, UtcMillis,
};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};
use deck_streak_progression::level::level_info;
use deck_streak_progression::settle::{SettleCause, SettleRequest, settle};
use deck_streak_progression::xp::XpTotal;
use sqlx::SqliteConnection;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
const DAY: i64 = 20_000;
/// Level 1 to level 3: the curve's starts are 0, 100, 300.
const SETTLED: u32 = 350;

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
