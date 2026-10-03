//! The landmarks' offers through the sync cycle (SPEC-102 A25 to A27 and A35 to A39; ADR-322).
//!
//! Every test runs the composed sync cycle with the real router over a temporary deployment: a
//! private copy holding one card reviewed at 10:00 of each synthetic study day, the database, and a
//! fold with analytics' step, so each settle moves the settle cursor. The mark and the cursor are
//! read raw from `notification_settings` after the cycle, and the keys delivered from
//! `notification_deliveries`. Every day is an epoch day and every value is synthetic.

#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
#[allow(dead_code)]
mod golden;

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use deck_streak_analytics::rollup;
use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::recompute::analytics_step::AnalyticsStep;
use deck_streak_coordination::recompute::badges::badge_line;
use deck_streak_coordination::recompute::{DayEvaluation, DayStep, Evaluation, Fold, Phase};
use deck_streak_coordination::sync_cycle::{CycleParts, Recompute, sync_cycle};
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
    PortFuture, Redactor, StudyDay, StudyDayRule, UtcMillis,
};
use deck_streak_notifications::landmarks::{
    ANNIVERSARY_EVENT_TYPE, Landmark, STUDY_DAY_EVENT_TYPE, render_landmark,
};
use deck_streak_notifications::{Policy, Router};
use serde_json::Value;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection, SqliteConnection};
use support::RecordingBot;

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
/// The mark the predecessor stores once (SPEC-102 R6).
const MARK: &str = "landmark_high_water";
/// The cursor: every landmark after the seed and at or before it was answered (SPEC-102 R5).
const CURSOR: &str = "landmarks_offered_through";
/// 2023-10-04: its first anniversary is 2024-10-04, epoch day 20000.
const ORIGIN: i64 = 19_634;
/// The first anniversary of [`ORIGIN`].
const ANNIVERSARY: i64 = 20_000;
/// A mark an earlier run stored, as the import stores it.
const EARLIER_MARK: &str = r#"{"anniversary": 0, "seeded": true, "study_day": 0}"#;
/// The owed badge the awards' offers hand the router.
const BADGE_NAME: &str = "Synthetic badge";
const BADGE_EMOJI: &str = "\u{2b50}";

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

/// A step that records the current day settled inside its own write, as a concurrent recompute's
/// fold would between two offer calls of this one.
struct SettlesToday;

impl DayStep for SettlesToday {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        "landmarks_step.settles_today"
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if matches!(day.evaluation, Evaluation::Current) {
                rollup::record_settled(write, day.day, day.facts.now).await?;
            }
            Ok(())
        })
    }
}

/// One temporary deployment: the copy, the database, the clock, the bot and the composed cycle.
struct Deployment {
    _scratch: tempfile::TempDir,
    copy: PathBuf,
    db: Db,
    clock: Arc<ManualClock>,
    bot: Arc<RecordingBot>,
    cycle: CycleParts<Engine>,
}

/// Noon of `day`, outside the quiet hours.
fn noon(day: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * DAY_MS + 12 * HOUR_MS)
}

impl Deployment {
    /// A deployment whose copy holds one card reviewed on each of `study_days`, whose clock reads
    /// noon of `today`, and whose fold runs analytics' step and `settles_today` when asked.
    async fn new(study_days: &[i64], today: i64, settles_today: bool) -> Self {
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
        let copy = settings.copy_path();
        RslibEngine
            .new_card_queue(&copy)
            .expect("the engine creates the copy");
        seed_card(&copy).await;
        add_reviews(&copy, study_days).await;
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        let clock = Arc::new(ManualClock::new(noon(today)));
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
        let bot = Arc::new(RecordingBot::default());
        let router = Router::new(policy, db.clone(), clock.clone(), StudyDayRule::default())
            .with_bot(bot.clone());
        let mut fold = Fold::default();
        fold.register(
            Phase::RollupAndScore,
            Box::new(AnalyticsStep::new(AnalyticsSettings::default())),
        )
        .expect("analytics' step in its phase");
        if settles_today {
            fold.register(Phase::Awards, Box::new(SettlesToday))
                .expect("the step in its phase");
        }
        let cycle = CycleParts::new(syncer, reader, gate, Obligations::new(), clock.clone())
            .with_flush(Arc::new(router))
            .with_fold(Arc::new(fold), db.clone(), StudyDayRule::default(), None);
        Self {
            _scratch: scratch,
            copy,
            db,
            clock,
            bot,
            cycle,
        }
    }

    /// Runs one sync cycle at noon of `day` and requires that it ran the fold.
    async fn cycle_at(&self, day: i64) {
        self.clock.set(noon(day));
        let report = sync_cycle(&self.cycle, Trigger::Owner)
            .await
            .expect("the cycle runs");
        assert!(
            matches!(report.recompute, Recompute::Ran { .. }),
            "the cycle on {day} runs the fold"
        );
    }

    /// The value `notification_settings` holds for `key`, read raw.
    async fn setting(&self, key: &str) -> Option<String> {
        let mut read = self.db.reader().acquire().await.expect("a reader");
        sqlx::query_scalar::<_, String>("SELECT value FROM notification_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&mut *read)
            .await
            .expect("the setting reads")
    }

    /// Stores `value` for `key`, as an earlier run or the import stored it.
    async fn store(&self, key: &str, value: &str) {
        self.execute(format!(
            "INSERT INTO notification_settings (key, value, created_at) VALUES ('{key}', '{value}', 0)"
        ))
        .await;
    }

    /// Runs one statement on the database in a write of its own.
    async fn execute(&self, statement: impl Into<String>) {
        let mut write = self.db.write().await.expect("a write");
        sqlx::query(sqlx::AssertSqlSafe(statement.into()))
            .execute(&mut *write)
            .await
            .expect("the statement runs");
        write.commit().await.expect("the write commits");
    }

    /// The landmark keys the router delivered, in the order it claimed them.
    async fn landmark_keys(&self) -> Vec<String> {
        let mut read = self.db.reader().acquire().await.expect("a reader");
        sqlx::query_scalar::<_, String>(
            "SELECT dedupe_key FROM notification_deliveries \
             WHERE dedupe_key LIKE 'landmark:%' ORDER BY id",
        )
        .fetch_all(&mut *read)
        .await
        .expect("the deliveries read")
    }

    /// Every line the bot sent that is a landmark's.
    fn landmark_lines(&self) -> Vec<String> {
        self.bot
            .sent()
            .into_iter()
            .filter(|line| line.contains("anniversary") || line.contains("earned study days"))
            .collect()
    }

    /// An owed badge: earned, its celebration not yet marked.
    async fn owe_a_badge(&self) {
        self.execute(format!(
            "INSERT INTO badges_earned (badge_key, tier, name, emoji, study_day, celebrated_at, \
             created_at) VALUES ('synthetic_badge', 1, '{BADGE_NAME}', '{BADGE_EMOJI}', {ORIGIN}, \
             NULL, 0)"
        ))
        .await;
    }
}

/// Opens the copy at `path` for writing.
async fn open_copy(path: &Path) -> SqliteConnection {
    SqliteConnectOptions::from_str("sqlite://")
        .expect("options")
        .filename(path)
        .connect()
        .await
        .expect("the copy opens for writing")
}

/// Inserts the one synthetic note and card every review is of.
async fn seed_card(path: &Path) {
    let mut connection = open_copy(path).await;
    sqlx::query(
        "INSERT INTO notes (id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data) \
         VALUES (1, 'guid1', 1, 0, 0, '', 'front\u{1f}back', 'front', 0, 0, '')",
    )
    .execute(&mut connection)
    .await
    .expect("a note");
    sqlx::query(
        "INSERT INTO cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, \
         lapses, left, odue, odid, flags, data) \
         VALUES (1, 1, 1, 0, 0, 0, 2, 2, 100, 10, 2500, 1, 0, 0, 0, 0, 0, '{}')",
    )
    .execute(&mut connection)
    .await
    .expect("a card");
    connection.close().await.expect("the copy closes");
}

/// Inserts one review of the card at 10:00 of each of `days`: ease 3, a review (type 1).
async fn add_reviews(path: &Path, days: &[i64]) {
    let mut connection = open_copy(path).await;
    for day in days {
        sqlx::query(
            "INSERT INTO revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
             VALUES (?1, 1, 0, 3, 10, 5, 2500, 4000, 1)",
        )
        .bind(day * DAY_MS + 10 * HOUR_MS)
        .execute(&mut connection)
        .await
        .expect("a review");
    }
    connection.close().await.expect("the copy closes");
}

/// Makes the whole-log read of the copy fail while the window's read still succeeds: `revlog`
/// becomes a view whose `ease` overflows on every review at or before `through`, a day below the
/// window's floor, which only a read that tests `ease` on those rows evaluates.
async fn break_the_whole_log_read(path: &Path, through: i64) {
    let mut connection = open_copy(path).await;
    let limit = (through + 1) * DAY_MS;
    for statement in [
        "ALTER TABLE revlog RENAME TO revlog_rows".to_owned(),
        format!(
            "CREATE VIEW revlog AS SELECT id, cid, usn, \
             CASE WHEN id < {limit} THEN abs(-9223372036854775808) ELSE ease END AS ease, \
             ivl, lastIvl, factor, time, type FROM revlog_rows"
        ),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&mut connection)
            .await
            .expect("the copy is altered");
    }
    connection.close().await.expect("the copy closes");
}

/// The text of the landmark `key` dated `day`, honest or plain, as the port renders it.
fn text(key: &str, day: i64, honest: bool) -> String {
    let (event, ordinal) = if let Some(ordinal) = key.strip_prefix("landmark:anniv:") {
        (ANNIVERSARY_EVENT_TYPE, ordinal)
    } else {
        let ordinal = key.strip_prefix("landmark:day:").expect("a landmark key");
        (STUDY_DAY_EVENT_TYPE, ordinal)
    };
    render_landmark(
        &Landmark {
            key: key.to_owned(),
            event,
            ordinal: ordinal.parse().expect("an ordinal"),
            day: StudyDay::from_epoch_day(day),
        },
        honest,
    )
}

/// `count` consecutive days from `first`.
fn days_from(first: i64, count: i64) -> Vec<i64> {
    (first..first + count).collect()
}

/// A golden text with each `{day:N}` token written as the ISO date of epoch day `N`.
fn expand(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{day:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "{day:".len()..];
        let end = after.find('}').expect("a closed token");
        let number: i64 = after[..end].parse().expect("a day number");
        out.push_str(&StudyDay::from_epoch_day(number).to_string());
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

/// One golden `landmarks_run` case run as the first sync cycle of a fresh deployment: the
/// mark, the keys and the lines it delivered.
async fn run_golden_case(case: &golden::Case) -> (Option<String>, Vec<String>, Vec<String>) {
    let study_days: Vec<i64> = case.input["study_days"]
        .as_array()
        .expect("the study days")
        .iter()
        .map(integer)
        .collect();
    let today = integer(&case.input["today"]);
    let deployment = Deployment::new(&study_days, today, false).await;
    if let Some(mark) = case.input["mark"].as_str() {
        deployment.store(MARK, mark).await;
    }
    deployment.cycle_at(today).await;
    (
        deployment.setting(MARK).await,
        deployment.landmark_keys().await,
        deployment.landmark_lines(),
    )
}

/// The golden cases of `landmarks_run`, each with its input and output.
fn golden_cases() -> Vec<golden::Case> {
    let golden = golden::read(&golden::committed("landmarks_run"))
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    assert!(!golden.cases.is_empty(), "the golden holds cases");
    golden.cases
}

#[tokio::test]
async fn each_evaluated_day_raises_its_due_landmarks_once() {
    // The origin and 22 more days, then 20000 and 20001: the 25th study day is 20001, and the
    // first anniversary is 20000, a day the second cycle settles rather than evaluates.
    let mut early = vec![ORIGIN];
    early.extend(days_from(19_950, 22));
    let deployment = Deployment::new(&early, ANNIVERSARY - 1, false).await;
    deployment.cycle_at(ANNIVERSARY - 1).await;
    assert!(
        deployment.landmark_keys().await.is_empty(),
        "nothing is due on 19999"
    );

    add_reviews(&deployment.copy, &[ANNIVERSARY, ANNIVERSARY + 1]).await;
    deployment.cycle_at(ANNIVERSARY + 1).await;
    assert_eq!(
        deployment.landmark_keys().await,
        ["landmark:day:25", "landmark:anniv:1"],
        "the day evaluated's landmark first, then the settled day's"
    );
    assert_eq!(
        deployment.landmark_lines(),
        [
            text("landmark:day:25", ANNIVERSARY + 1, false),
            text("landmark:anniv:1", ANNIVERSARY, false),
        ],
        "each with its text, the anniversary plain on a day studied"
    );

    deployment.cycle_at(ANNIVERSARY + 2).await;
    assert_eq!(
        deployment.landmark_lines().len(),
        2,
        "a later recompute raises none of them again"
    );
}

#[tokio::test]
async fn the_first_run_seeds_the_mark_and_raises_at_most_one() {
    let cases = golden_cases();
    for case in &cases {
        let (_, keys, lines) = run_golden_case(case).await;
        let celebrated = case.output["celebrated"]
            .as_array()
            .expect("the celebrated landmarks");
        let expected_keys: Vec<String> = celebrated
            .iter()
            .map(|item| item[1].as_str().expect("a key").to_owned())
            .collect();
        let expected_lines: Vec<String> = celebrated
            .iter()
            .map(|item| expand(item[2].as_str().expect("a text")))
            .collect();
        assert_eq!(keys, expected_keys, "the keys of {}", case.input);
        assert_eq!(lines, expected_lines, "the lines of {}", case.input);
        assert_eq!(
            i64::try_from(lines.len()).expect("a small count"),
            integer(&case.output["delivered"]),
            "the count delivered of {}",
            case.input
        );
    }
    println!("examined {} landmarks_run cases", cases.len());
}

#[tokio::test]
async fn the_anniversary_is_honest_on_a_day_without_study() {
    let unstudied = Deployment::new(&[ORIGIN, ORIGIN + 40], ANNIVERSARY, false).await;
    unstudied.cycle_at(ANNIVERSARY).await;
    assert_eq!(
        unstudied.landmark_lines(),
        [text("landmark:anniv:1", ANNIVERSARY, true)],
        "the honest variant on a day without study"
    );

    let studied = Deployment::new(&[ORIGIN, ORIGIN + 40, ANNIVERSARY], ANNIVERSARY, false).await;
    studied.cycle_at(ANNIVERSARY).await;
    assert_eq!(
        studied.landmark_lines(),
        [text("landmark:anniv:1", ANNIVERSARY, false)],
        "the plain variant on a day studied"
    );
}

#[tokio::test]
async fn an_unanswered_landmark_keeps_the_cursor_and_is_offered_again() {
    let deployment = Deployment::new(&[ORIGIN, ANNIVERSARY - 5], ANNIVERSARY - 1, false).await;
    deployment.cycle_at(ANNIVERSARY - 1).await;
    deployment
        .execute(
            "CREATE TRIGGER refuse_the_anniversary BEFORE INSERT ON notification_deliveries \
             WHEN NEW.dedupe_key = 'landmark:anniv:1' \
             BEGIN SELECT RAISE(ABORT, 'the router cannot answer'); END",
        )
        .await;
    deployment.cycle_at(ANNIVERSARY + 1).await;
    assert!(
        deployment.landmark_keys().await.is_empty(),
        "the router answered nothing"
    );
    assert_eq!(
        deployment.setting(CURSOR).await.as_deref(),
        Some("19999"),
        "the cursor stops the day before the landmark the router did not answer"
    );

    deployment
        .execute("DROP TRIGGER refuse_the_anniversary")
        .await;
    deployment.cycle_at(ANNIVERSARY + 2).await;
    assert_eq!(
        deployment.landmark_keys().await,
        ["landmark:anniv:1"],
        "the next recompute offers it again"
    );
    assert_eq!(
        deployment.landmark_lines(),
        [text("landmark:anniv:1", ANNIVERSARY, true)],
        "and the router sends it once"
    );
    assert_eq!(
        deployment.setting(CURSOR).await.as_deref(),
        Some("20001"),
        "an answered landmark moves the cursor to the settle cursor"
    );
}

#[tokio::test]
async fn the_first_run_never_moves_the_cursor_past_a_landmark_it_did_not_raise() {
    // The origin, 23 more days and 20000: the first anniversary and the 25th study day are both
    // due on 20000, the first run's day, which a concurrent fold settles during the run.
    let mut days = vec![ORIGIN];
    days.extend(days_from(19_950, 23));
    days.push(ANNIVERSARY);
    let deployment = Deployment::new(&days, ANNIVERSARY, true).await;
    deployment.cycle_at(ANNIVERSARY).await;
    assert_eq!(
        deployment.landmark_keys().await,
        ["landmark:anniv:1"],
        "the first run raises the first due landmark only"
    );
    assert_eq!(
        deployment.setting(CURSOR).await.as_deref(),
        Some("19999"),
        "the first run leaves the cursor at its seed"
    );

    deployment.cycle_at(ANNIVERSARY + 1).await;
    assert_eq!(
        deployment.landmark_keys().await,
        ["landmark:anniv:1", "landmark:day:25"],
        "a later run raises the seed day's other landmark"
    );
}

#[tokio::test]
async fn an_imported_mark_raises_todays_landmarks_and_no_history() {
    // 2023-10-03: the first anniversary is 19999, yesterday; the 25th study day is 20000.
    let origin = ORIGIN - 1;
    let mut days = vec![origin];
    days.extend(days_from(19_950, 23));
    days.push(ANNIVERSARY);
    let deployment = Deployment::new(&days, ANNIVERSARY, false).await;
    deployment.store(MARK, EARLIER_MARK).await;
    deployment.cycle_at(ANNIVERSARY).await;
    assert_eq!(
        deployment.setting(CURSOR).await.as_deref(),
        Some("19999"),
        "the cursor is stored at yesterday"
    );
    assert_eq!(
        deployment.landmark_keys().await,
        ["landmark:day:25"],
        "today's landmark is raised, and yesterday's anniversary is history"
    );
    assert_eq!(
        deployment.setting(MARK).await.as_deref(),
        Some(EARLIER_MARK),
        "the imported mark is never overwritten"
    );
}

#[tokio::test]
async fn the_high_water_mark_equals_the_predecessors_bytes() {
    let cases = golden_cases();
    for case in &cases {
        let (mark, _, _) = run_golden_case(case).await;
        let expected = case.output["mark_written"]
            .as_str()
            .or_else(|| case.input["mark"].as_str())
            .expect("a mark written or stored before");
        assert_eq!(
            mark.as_deref(),
            Some(expected),
            "the mark of {}",
            case.input
        );
    }
    println!("examined {} landmarks_run marks", cases.len());
}

#[tokio::test]
async fn the_sync_cycle_offers_the_landmarks_after_the_awards() {
    let badge = badge_line(BADGE_EMOJI, BADGE_NAME);
    let deployment = Deployment::new(&[ORIGIN, ANNIVERSARY], ANNIVERSARY, false).await;
    deployment.owe_a_badge().await;
    deployment.cycle_at(ANNIVERSARY).await;
    assert_eq!(
        deployment.bot.sent(),
        [badge.clone(), text("landmark:anniv:1", ANNIVERSARY, false)],
        "the owed badge first, then the landmark due"
    );

    // 2022-10-04: its second anniversary is 20000, and the origin's review lies below the window.
    let origin = ANNIVERSARY - 731;
    let failing = Deployment::new(&[origin, ANNIVERSARY - 1], ANNIVERSARY - 1, false).await;
    failing.cycle_at(ANNIVERSARY - 1).await;
    let cursor = failing.setting(CURSOR).await;
    break_the_whole_log_read(&failing.copy, origin).await;
    failing.owe_a_badge().await;
    failing.cycle_at(ANNIVERSARY).await;
    assert_eq!(
        failing.bot.sent(),
        [badge],
        "a failed whole-log read offers no landmark, and the awards still go"
    );
    assert!(
        failing.landmark_keys().await.is_empty(),
        "no landmark is delivered"
    );
    assert_eq!(
        failing.setting(CURSOR).await,
        cursor,
        "and the cursor does not move"
    );
}
