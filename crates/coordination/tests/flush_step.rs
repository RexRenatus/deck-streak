//! The sync cycle's flush step (SPEC-041 R7): after a sync that ran and succeeded, the cycle flushes
//! the notification router, so a celebration quiet hours held is delivered once the window has
//! ended; after a sync that failed, it flushes nothing.
//!
//! Each test runs one whole cycle over a temporary deployment: an engine whose sync finds no change
//! or is refused, an empty copy the engine itself created, the service's database, and a router over
//! a bot transport that records every push. Every clock is a `ManualClock`, and the default rule
//! reads local time as UTC. Every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::obligations::Obligations;
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
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload, OffloadWorkers,
    Redactor, StudyDay, StudyDayRule, UtcMillis,
};
use deck_streak_notifications::{
    BotTransport, Decision, DedupeKey, Hold, LapseContext, Occasion, Pass, Policy, PushFuture,
    Pushed, Router, Surface, Tier,
};

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
/// A study day, as its epoch day number.
const DAY: i64 = 20_000;

/// An engine whose every sync finds no change, or is refused.
#[derive(Clone, Copy)]
struct Engine {
    refused: bool,
}

impl AnkiEngine for Engine {
    fn new_card_queue(&self, _collection: &Path) -> Result<NewCardQueue, EngineError> {
        Ok(NewCardQueue::default())
    }

    async fn normal_sync(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<SyncOutcome, EngineError> {
        if self.refused {
            Err(EngineError::AuthRejected)
        } else {
            Ok(SyncOutcome::NoChanges)
        }
    }

    async fn full_download(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<(), EngineError> {
        Ok(())
    }
}

/// A bot transport that records every push and delivers it.
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

/// A deployment in a scratch directory whose router holds one celebration quiet hours deferred.
struct Deployment {
    _scratch: tempfile::TempDir,
    db: Db,
    bot: Arc<Recording>,
    cycle: CycleParts<Engine>,
}

impl Deployment {
    async fn new(engine: Engine) -> Self {
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
        let late = UtcMillis::from_epoch_millis(DAY * DAY_MS + (23 * 60 + 30) * MINUTE_MS);
        let clock = Arc::new(ManualClock::new(late));
        let loader = CredentialLoader::new(
            CredentialsDirectory::new(credentials).expect("an absolute credentials directory"),
            Redactor::new(),
        );
        let syncer = Syncer::new(
            engine,
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
        let router = Router::new(
            Arc::clone(&policy),
            db.clone(),
            clock.clone(),
            StudyDayRule::default(),
        )
        .with_bot(bot.clone());
        let celebration = Occasion::new(
            policy.kind("celebration").expect("the celebration kind"),
            DedupeKey::new("level-up:9").expect("a key"),
            Surface::Bot,
            Tier::T2,
            "synthetic level-up",
            StudyDay::from_epoch_day(DAY),
            LapseContext::NoLapse,
        )
        .expect("an occasion");
        assert_eq!(
            router.route(&celebration).await.expect("a decision"),
            Decision::Deferred {
                surface: Surface::Bot,
                hold: Hold::Quiet
            },
            "23:30 is inside the quiet window"
        );
        let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
            .with_flush(Arc::new(router));
        // The next morning, after the window.
        clock.set(UtcMillis::from_epoch_millis(
            (DAY + 1) * DAY_MS + 8 * 60 * MINUTE_MS,
        ));
        Self {
            _scratch: scratch,
            db,
            bot,
            cycle,
        }
    }
}

#[tokio::test]
async fn a_successful_sync_flushes_the_held_celebrations() {
    let deployment = Deployment::new(Engine { refused: false }).await;

    let _report = sync_cycle(&deployment.cycle, Trigger::Owner)
        .await
        .expect("the cycle runs");

    assert_eq!(deployment.bot.pushes(), ["synthetic level-up"]);
}

#[tokio::test]
async fn a_failed_sync_flushes_nothing() {
    let deployment = Deployment::new(Engine { refused: true }).await;

    let report = sync_cycle(&deployment.cycle, Trigger::Owner)
        .await
        .expect("the cycle runs");

    let outcome = match report.sync {
        SyncReport::Ran { run, .. } => Some(run.outcome),
        _ => None,
    };
    assert_eq!(
        outcome,
        Some(Err(ReasonCode::AuthRejected)),
        "the sync ran, and was refused"
    );
    let held: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_queue WHERE state = 'held'")
            .fetch_one(deployment.db.reader())
            .await
            .expect("the queue is counted");
    assert_eq!(
        held, 1,
        "the celebration is still held, for a sync that succeeds"
    );
    assert_eq!(
        deployment.bot.pushes(),
        Vec::<String>::new(),
        "nothing was pushed"
    );
}
