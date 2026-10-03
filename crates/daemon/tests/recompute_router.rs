//! Every production recompute cycle holds a router that holds for the senders (SPEC-319 A1 to A4;
//! R1 to R4; ADR-319). The cycle is built by the composition root's own seam,
//! `RecomputeSetup::load` and `RecomputeSetup::cycle`, and run as the scheduled sync, over a stub
//! engine and an empty collection copy. A catalog badge awarded unmarked before the cycle is owed
//! a celebration: the fold offers it to the router the cycle holds, the celebrations' switch decides
//! whether it is withheld or held for the senders, and the bot's flush delivers a held one once.
//! Every instant, key and credential is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::sync_cycle::{CycleParts, Recompute, sync_cycle};
use deck_streak_daemon::wiring::RecomputeSetup;
use deck_streak_ingest::engine::{
    AnkiEngine, EngineError, NewCardQueue, RslibEngine, SyncLogin, SyncOutcome,
};
use deck_streak_ingest::gate::ChangeGate;
use deck_streak_ingest::settings::{
    STATE_DIRECTORY, SYNC_ENDPOINT, SYNC_PASSWORD, SYNC_USERNAME, ScopeSettings, SyncSettings,
};
use deck_streak_ingest::sync::{RetrySchedule, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    Clock, Courses, CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload,
    OffloadWorkers, Redactor, StudyDay, StudyDayRule, SystemClock, UtcMillis,
};
use deck_streak_notifications::quiet::in_quiet_hours;
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};
use deck_streak_progression::badges::award::{Award, NewBadge, award};
use sqlx::Row;
use tempfile::TempDir;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
/// The study day of the cycle's clock.
const DAY: i64 = 20_000;
/// A catalog badge, awarded at tier 0.
const BADGE: &str = "first_steps";
/// Its name, which the celebration's line carries.
const NAME: &str = "First Steps";
/// The router's dedupe key for it (`recompute::badges::badge_key`).
const KEY: &str = "badge:first_steps:0";
/// The setting the policy's `celebration` kind names.
const SWITCH: &str = "celebrations_enabled";
/// The policy's default quiet window, `notifications-policy.json` `quiet_hours`: 23:00 to 07:30,
/// local time, which the default rule reads as UTC.
const QUIET_START_MIN: i64 = 23 * 60;
const QUIET_END_MIN: i64 = 7 * 60 + 30;

/// An engine whose sync changes nothing: the copy stays the empty collection the test created.
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

/// The bot's transport as the senders hold it: it keeps every line it is asked to push.
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

/// One service's state: its environment, the sync's settings with an empty collection copy, the
/// sync's credentials, and the database.
struct Service {
    env: Environment,
    settings: SyncSettings,
    credentials: CredentialsDirectory,
    db: Db,
    _scratch: TempDir,
}

/// A fresh service over a scratch directory.
async fn service() -> Service {
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
    let env = Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, state.as_os_str()),
    ]);
    let settings = SyncSettings::from_env(&env).expect("the settings");
    RslibEngine
        .new_card_queue(&settings.copy_path())
        .expect("the engine creates the copy");
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database");
    Service {
        env,
        settings,
        credentials: CredentialsDirectory::new(credentials)
            .expect("an absolute credentials directory"),
        db,
        _scratch: scratch,
    }
}

/// Awards the catalog badge unmarked, as the badges step writes it: its celebration is owed.
async fn award_badge(db: &Db) {
    let mut write = db.write().await.expect("a write");
    let awarded = award(
        &mut write,
        &Courses::default(),
        &NewBadge {
            key: BADGE,
            tier: 0,
            name: NAME,
            emoji: "S",
            study_day: StudyDay::from_epoch_day(DAY),
            at: UtcMillis::from_epoch_millis(DAY * DAY_MS),
        },
    )
    .await
    .expect("a catalog badge");
    write.commit().await.expect("the award commits");
    assert_eq!(awarded, Award::Awarded, "the badge is written owed");
}

/// Stores `value` for the celebrations' switch, as the owner or the cutover checklist would.
async fn store_switch(db: &Db, value: &str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, 0) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(SWITCH)
    .bind(value)
    .execute(&mut *write)
    .await
    .expect("the switch is stored");
    write.commit().await.expect("the switch commits");
}

/// The recompute the job role loads at its start.
async fn load(service: &Service) -> RecomputeSetup {
    RecomputeSetup::load(&service.env, &service.db)
        .await
        .expect("no courses and no taxonomy are configured")
}

/// One scheduled cycle through the composition root's seam, at noon of `DAY` on the cycle's clock.
async fn scheduled_cycle(service: &Service, recompute: &RecomputeSetup) {
    let noon = UtcMillis::from_epoch_millis(DAY * DAY_MS + 12 * 60 * MINUTE_MS);
    let clock = Arc::new(ManualClock::new(noon));
    let syncer = Syncer::new(
        Engine,
        SqliteSyncRuns::new(service.db.clone()),
        service.settings.clone(),
        CredentialLoader::new(service.credentials.clone(), Redactor::new()),
        clock.clone(),
        StudyDayRule::default(),
    )
    .with_schedule(RetrySchedule::IMMEDIATE);
    let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock.clone());
    let reader = recompute.reader(&service.settings, ScopeSettings::default(), offload);
    let gate = ChangeGate::new(service.db.clone(), StudyDayRule::default(), clock.clone());
    let parts = recompute.cycle(
        CycleParts::new(syncer, reader, gate, Obligations::new(), clock),
        service.db.clone(),
        StudyDayRule::default(),
    );
    let report = sync_cycle(&parts, Trigger::Scheduled)
        .await
        .expect("the cycle runs");
    assert!(
        matches!(report.recompute, Recompute::Ran { .. }),
        "the scheduled cycle recomputes"
    );
}

/// Each decision the ledger holds for `key`: its arm and its reason or hold.
async fn decisions(db: &Db, key: &str) -> Vec<(String, Option<String>)> {
    sqlx::query("SELECT arm, reason FROM notification_decisions WHERE dedupe_key = ? ORDER BY id")
        .bind(key)
        .fetch_all(db.reader())
        .await
        .expect("the decisions are read")
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect()
}

/// Each row of the queue: its key and its state.
async fn queue(db: &Db) -> Vec<(String, String)> {
    sqlx::query("SELECT dedupe_key, state FROM notification_queue ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("the queue is read")
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect()
}

/// Whether the badge's celebration is marked answered.
async fn marked(db: &Db) -> bool {
    sqlx::query_scalar(
        "SELECT celebrated_at IS NOT NULL FROM badges_earned WHERE badge_key = ? AND tier = 0",
    )
    .bind(BADGE)
    .fetch_one(db.reader())
    .await
    .expect("the badge is read")
}

/// The stored value of the celebrations' switch, if any.
async fn switch(db: &Db) -> Option<String> {
    sqlx::query_scalar("SELECT value FROM notification_settings WHERE key = ?")
        .bind(SWITCH)
        .fetch_optional(db.reader())
        .await
        .expect("the switch is read")
}

/// The first minute at or after `instant` that is outside the policy's quiet window: within 510
/// minutes of it, so a row held at or before `instant` is inside the queue's 720-minute age.
fn first_open_minute(instant: UtcMillis) -> UtcMillis {
    let mut minute = instant.epoch_millis().div_euclid(MINUTE_MS) + 1;
    while in_quiet_hours(minute, QUIET_START_MIN, QUIET_END_MIN) {
        minute += 1;
    }
    UtcMillis::from_epoch_millis(minute * MINUTE_MS)
}

/// A1: the owed badge reaches the router the production cycle holds, once, and is marked.
#[tokio::test]
async fn the_production_recompute_offers_an_owed_award_to_the_router() {
    let service = service().await;
    award_badge(&service.db).await;
    let recompute = load(&service).await;

    scheduled_cycle(&service, &recompute).await;

    assert_eq!(
        decisions(&service.db, KEY).await.len(),
        1,
        "the owed badge was offered to the cycle's router and decided once"
    );
    assert!(
        marked(&service.db).await,
        "the router answered, so the badge is marked"
    );
}

/// A2: with the switch on, the badge is deferred and held on the queue for the senders.
#[tokio::test]
async fn a_switched_on_celebration_is_held_for_the_senders() {
    let service = service().await;
    store_switch(&service.db, "1").await;
    award_badge(&service.db).await;
    let recompute = load(&service).await;

    scheduled_cycle(&service, &recompute).await;

    let decided: Vec<String> = decisions(&service.db, KEY)
        .await
        .into_iter()
        .map(|(arm, _)| arm)
        .collect();
    assert_eq!(decided, ["defer"], "the badge is deferred, never withheld");
    let held: Vec<(String, String)> = queue(&service.db)
        .await
        .into_iter()
        .filter(|(key, _)| key == KEY)
        .collect();
    assert_eq!(
        held,
        [(KEY.to_owned(), "held".to_owned())],
        "one row is held for the badge's key"
    );
    assert!(marked(&service.db).await, "a deferral is an answer");
}

/// A3: the bot's flush, outside the window, delivers the held badge once.
#[tokio::test]
async fn the_bots_flush_delivers_what_the_recompute_held() {
    let service = service().await;
    store_switch(&service.db, "1").await;
    award_badge(&service.db).await;
    let recompute = load(&service).await;
    scheduled_cycle(&service, &recompute).await;
    // The cycle's router reads the system clock, so any row it held was held before now.
    let open = first_open_minute(SystemClock.now());
    let bot = Arc::new(Recording::default());
    let senders = Router::new(
        Arc::new(Policy::compiled().expect("the compiled policy parses")),
        service.db.clone(),
        Arc::new(ManualClock::new(open)),
        StudyDayRule::default(),
    )
    .with_bot(bot.clone());

    let flushed = senders.flush().await.expect("the flush runs");
    let first = bot.pushes();
    let again = senders.flush().await.expect("a second flush runs");

    let naming: Vec<&String> = first.iter().filter(|push| push.contains(NAME)).collect();
    assert_eq!(
        naming.len(),
        1,
        "exactly one push names the held badge ({flushed:?}): {first:?}"
    );
    assert_eq!(
        bot.pushes(),
        first,
        "a second flush pushes nothing ({again:?})"
    );
}

/// A4: a fresh start seeds the switch off, the badge is withheld and marked, and a stored value is
/// never overwritten.
#[tokio::test]
async fn a_fresh_start_seeds_the_celebrations_switch_off() {
    let fresh = service().await;
    award_badge(&fresh.db).await;
    let recompute = load(&fresh).await;
    assert_eq!(
        switch(&fresh.db).await.as_deref(),
        Some("0"),
        "a fresh start seeds the celebrations' switch off"
    );

    scheduled_cycle(&fresh, &recompute).await;

    assert_eq!(
        decisions(&fresh.db, KEY).await,
        [("withhold".to_owned(), Some("nudges_disabled".to_owned()))],
        "with the switch off the badge is withheld before its claim"
    );
    assert!(marked(&fresh.db).await, "a withhold is an answer");

    let stored = service().await;
    store_switch(&stored.db, "1").await;
    load(&stored).await;
    assert_eq!(
        switch(&stored.db).await.as_deref(),
        Some("1"),
        "a value already stored is never overwritten"
    );
}
