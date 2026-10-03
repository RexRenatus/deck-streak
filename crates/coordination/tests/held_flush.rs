//! The held-notification flush on every path that can flush (SPEC-041 R7, amended for #291): the
//! after-sync flush of a scheduled sync, the scheduled flush job, and the flush after an
//! owner-triggered sync, over every clock position of the configured quiet window and every state
//! of a hold. A held item reaches the owner exactly once, or is abandoned by name, and a flush
//! delivers only while the window is open.
//!
//! Each case runs over a fresh temporary deployment whose clock is injected, and whose bot
//! transport records every push. The window and the age limit are read from the same policy the
//! router reads. Every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::held_flush::HeldFlushWork;
use deck_streak_coordination::jobs::{FireDate, HELD_FLUSH};
use deck_streak_coordination::runner::{Done, Fire, Work};

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
use deck_streak_ingest::sync::{RetrySchedule, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload, OffloadWorkers,
    Redactor, StudyDayRule, UtcMillis,
};
use deck_streak_notifications::{BotTransport, Flushed, Pass, Policy, PushFuture, Pushed, Router};

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

/// The quiet window and the age limit, read from the policy the router reads.
struct Window {
    start: i64,
    end: i64,
    max_age_min: i64,
}

impl Window {
    fn read() -> Self {
        let text = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../notifications-policy.json"
        ))
        .expect("the policy file");
        let policy: serde_json::Value = serde_json::from_str(&text).expect("the policy parses");
        let clock = |name: &str| {
            let text = policy["quiet_hours"][name]
                .as_str()
                .expect("a window bound");
            deck_streak_notifications::quiet::ClockTime::parse(text)
                .expect("a clock time")
                .minutes()
        };
        Self {
            start: clock("start"),
            end: clock("end"),
            max_age_min: policy["deferral"]["max_age_minutes"]
                .as_i64()
                .expect("an age limit"),
        }
    }

    /// Whether a local minute of the day is inside the window, which wraps midnight when it ends
    /// before it starts.
    const fn contains(&self, minute: i64) -> bool {
        if self.start <= self.end {
            self.start <= minute && minute < self.end
        } else {
            minute >= self.start || minute < self.end
        }
    }

    /// Every clock position that matters, as (label, local minute of the day).
    fn positions(&self) -> Vec<(&'static str, i64)> {
        let mut found = vec![
            ("before the window", (self.start - 60).rem_euclid(1440)),
            ("at its start", self.start),
            (
                "inside, before midnight",
                (self.start + 30).rem_euclid(1440),
            ),
            ("across midnight", 0),
            ("its last minute", (self.end - 1).rem_euclid(1440)),
            ("at its end", self.end),
            ("after it", (self.end + 60).rem_euclid(1440)),
        ];
        let slot = HELD_FLUSH
            .schedule
            .daily_slot(deck_streak_kernel::Hour::new(0).expect("an hour"))
            .map_or(self.end, |(hour, minute)| {
                i64::from(hour) * 60 + i64::from(minute)
            });
        found.push(("the job's slot", slot));
        found
    }
}

#[derive(Clone, Copy, Debug)]
enum Flusher {
    AfterSync,
    Scheduled,
    OwnerSync,
}

#[derive(Clone, Copy, Debug)]
enum HoldState {
    Fresh,
    AtTheLimit,
    PastTheLimit,
    AlreadyDelivered,
}

/// What a case delivered and abandoned, as the owner sees it.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    delivered: Vec<String>,
    abandoned: Vec<String>,
    held_after: i64,
}

struct Case {
    router: Arc<Router>,
    bot: Arc<Recording>,
    db: Db,
    clock: Arc<ManualClock>,
    cycle: CycleParts<Engine>,
    _scratch: tempfile::TempDir,
}

impl Case {
    async fn new() -> Self {
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
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(DAY * DAY_MS)));
        let loader = CredentialLoader::new(
            CredentialsDirectory::new(credentials).expect("an absolute credentials directory"),
            Redactor::new(),
        );
        let syncer = Syncer::new(
            Engine { refused: false },
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
        let router = Arc::new(
            Router::new(policy, db.clone(), clock.clone(), StudyDayRule::default())
                .with_bot(bot.clone()),
        );
        let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
            .with_flush(Arc::clone(&router));
        Self {
            router,
            bot,
            db,
            clock,
            cycle,
            _scratch: scratch,
        }
    }

    /// Holds one celebration, deferred `age_ms` before `now`.
    async fn hold(&self, now: i64, age_ms: i64) {
        let mut write = self.db.write().await.expect("a write");
        sqlx::query(
            "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, \
             tier_pending, text, hold, tries, state, deferred_at, study_day, created_at) \
             VALUES ('celebration', 'level-up:9', 'bot', 'T2', 'T2', 'synthetic level-up', \
             'quiet', 0, 'held', ?, ?, ?)",
        )
        .bind(now - age_ms)
        .bind(DAY)
        .bind(now - age_ms)
        .execute(&mut *write)
        .await
        .expect("a held row");
        write.commit().await.expect("committed");
    }

    async fn flush_with(&self, flusher: Flusher, now: i64) {
        self.clock.set(UtcMillis::from_epoch_millis(now));
        match flusher {
            Flusher::AfterSync => {
                let _report = sync_cycle(&self.cycle, Trigger::Scheduled)
                    .await
                    .expect("the cycle runs");
            }
            Flusher::OwnerSync => {
                let _report = sync_cycle(&self.cycle, Trigger::Owner)
                    .await
                    .expect("the cycle runs");
            }
            Flusher::Scheduled => {
                let at = UtcMillis::from_epoch_millis(now);
                let rule = StudyDayRule::default();
                let fire = Fire {
                    job: HELD_FLUSH,
                    fire_date: FireDate::of(at, rule.utc_offset()),
                    scheduled_at: at,
                    started_at: at,
                    study_day: rule.study_day(at),
                    sync: None,
                };
                let done = HeldFlushWork::new(&self.router).perform(&fire).await;
                assert_eq!(done, Ok(Done::Done), "the scheduled flush is not refused");
            }
        }
    }

    async fn outcome(&self) -> Outcome {
        let pushes = self.bot.pushes();
        let (abandoned, delivered): (Vec<_>, Vec<_>) = pushes
            .into_iter()
            .partition(|text| text.contains("(expired, unseen)"));
        let held_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM notification_queue WHERE state = 'held'")
                .fetch_one(self.db.reader())
                .await
                .expect("the queue is counted");
        Outcome {
            delivered,
            abandoned,
            held_after,
        }
    }
}

/// What a flush at `minute` of the day must have delivered and abandoned.
fn expected(window: &Window, minute: i64, state: HoldState) -> Outcome {
    let open = !window.contains(minute);
    let (delivered, abandoned, held_after): (&[&str], &[&str], i64) = match (open, state) {
        (_, HoldState::AlreadyDelivered) => (&[], &[], 0),
        (false, _) => (&[], &[], 1),
        (true, HoldState::Fresh | HoldState::AtTheLimit) => (&["synthetic level-up"], &[], 0),
        (true, HoldState::PastTheLimit) => (
            &[],
            &[
                "\u{1f319} <b>1 held celebration(s)</b> (quiet hours)\n\u{2022} level-up:9 (expired, unseen)",
            ],
            0,
        ),
    };
    Outcome {
        delivered: delivered.iter().map(|text| (*text).to_owned()).collect(),
        abandoned: abandoned.iter().map(|text| (*text).to_owned()).collect(),
        held_after,
    }
}

#[tokio::test]
async fn every_flusher_delivers_once_or_abandons_by_name_and_only_while_the_window_is_open() {
    let window = Window::read();
    let flushers = [Flusher::AfterSync, Flusher::Scheduled, Flusher::OwnerSync];
    let states = [
        HoldState::Fresh,
        HoldState::AtTheLimit,
        HoldState::PastTheLimit,
        HoldState::AlreadyDelivered,
    ];
    let positions = window.positions();
    let derived = flushers.len() * positions.len() * states.len();
    let mut examined = 0_usize;
    let mut wrong = Vec::new();
    for flusher in flushers {
        for &(label, minute) in &positions {
            for state in states {
                let case = Case::new().await;
                let now = (DAY + 2) * DAY_MS + minute * MINUTE_MS;
                match state {
                    HoldState::Fresh => case.hold(now, 60 * MINUTE_MS).await,
                    HoldState::AtTheLimit => case.hold(now, window.max_age_min * MINUTE_MS).await,
                    HoldState::PastTheLimit => {
                        case.hold(now, window.max_age_min * MINUTE_MS + 1).await;
                    }
                    HoldState::AlreadyDelivered => {
                        let noon = (DAY + 1) * DAY_MS + 12 * 60 * MINUTE_MS;
                        case.hold(noon, 60 * MINUTE_MS).await;
                        case.clock.set(UtcMillis::from_epoch_millis(noon));
                        let _flushed = case.router.flush().await.expect("the first flush");
                        assert_eq!(case.bot.pushes(), ["synthetic level-up"]);
                        case.bot
                            .0
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .clear();
                    }
                }
                case.flush_with(flusher, now).await;
                let outcome = case.outcome().await;
                let want = expected(&window, minute, state);
                if outcome != want {
                    wrong.push(format!(
                        "{flusher:?} {label} {state:?}: {outcome:?}, wanted {want:?}"
                    ));
                }
                examined += 1;
            }
        }
    }
    println!("examined {examined} flush case(s)");
    assert_eq!(examined, derived, "every case of the axes ran");
    assert!(
        wrong.is_empty(),
        "{} of {examined} cases differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// A bot transport whose first push waits until it is released, so a second flush can start while
/// the first is mid-send.
#[derive(Default)]
struct Gated {
    pushes: Mutex<Vec<String>>,
    first: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

impl BotTransport for Gated {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            if !self.first.swap(true, Ordering::SeqCst) {
                self.entered.notify_one();
                self.release.notified().await;
            }
            self.pushes
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(text.to_owned());
            Pushed::Delivered
        })
    }
}

#[tokio::test]
async fn two_flushers_over_one_queue_never_send_one_item_twice() {
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    let now = (DAY + 1) * DAY_MS + 12 * 60 * MINUTE_MS;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(now)));
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let transport = Arc::new(Gated::default());
    let router = |transport: &Arc<Gated>| {
        Arc::new(
            Router::new(
                Arc::clone(&policy),
                db.clone(),
                clock.clone(),
                StudyDayRule::default(),
            )
            .with_bot(transport.clone()),
        )
    };
    let (first, second) = (router(&transport), router(&transport));
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, tier_pending, \
         text, hold, tries, state, deferred_at, study_day, created_at) \
         VALUES ('celebration', 'level-up:9', 'bot', 'T2', 'T2', 'synthetic level-up', 'quiet', \
         0, 'held', ?, ?, ?)",
    )
    .bind(now - 60 * MINUTE_MS)
    .bind(DAY)
    .bind(now - 60 * MINUTE_MS)
    .execute(&mut *write)
    .await
    .expect("a held row");
    write.commit().await.expect("committed");

    let running = tokio::spawn(async move { first.flush().await });
    transport.entered.notified().await;
    let overlapping = second.flush().await.expect("the second flush");
    transport.release.notify_one();
    let finished = running.await.expect("joined").expect("the first flush");

    assert_eq!(
        overlapping,
        Flushed::Busy,
        "the second flush finds the lease taken"
    );
    assert_eq!(finished, Flushed::Ran { sends: 1 });
    let sent = transport
        .pushes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(sent, ["synthetic level-up"], "delivered exactly once");
}
