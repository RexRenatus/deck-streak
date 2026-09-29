//! The hourly post-back records each graded drill and pays it once, on the study day of the poll
//! (SPEC-110 A9, A10, A12, A13; R9, R11).

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::Path;

use deck_streak_coordination::delivery::NoNotifier;
use deck_streak_coordination::drills::{DrillNotes, DrillPostbackWork, RealFs};
use deck_streak_coordination::jobs::DRILL_POSTBACK;
use deck_streak_coordination::ledger::{Outcome, SqliteCronLedger};
use deck_streak_coordination::runner::{Decision, Report, Runner};
use deck_streak_ingest::sync_runs::SqliteSyncRuns;
use deck_streak_kernel::{Db, Environment, ManualClock, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::grant::{GrantPort, GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::xp::XpAmount;
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::{Rails, VaultSettings};
use sqlx::Row;
use tempfile::TempDir;

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
/// A synthetic study day, as an epoch day number.
const DAY: i64 = 20_500;

/// The instant `hour`:`minute` UTC on epoch day `day`.
const fn at(day: i64, hour: i64, minute: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * DAY_MS + hour * HOUR_MS + minute * 60_000)
}

fn graded(xp: &str) -> String {
    format!("---\ntype: drill-irac\nsubject: \"Torts\"\nstatus: graded\n{xp}---\n# Duty of care\n")
}

struct Fixture {
    _dir: TempDir,
    db: Db,
    settings: VaultSettings,
    graded: std::path::PathBuf,
}

async fn fixture(make_graded_folder: bool) -> Fixture {
    let dir = tempfile::tempdir().expect("a temp dir");
    let vault = dir.path().join("vault");
    let graded = vault.join("11-Drills").join("Graded");
    fs::create_dir_all(&vault).expect("the vault");
    if make_graded_folder {
        fs::create_dir_all(&graded).expect("the graded folder");
    }
    let env = Environment::from_vars([
        (VAULT_ROOT, vault.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    let settings = VaultSettings::from_env(&env).expect("settings");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    Fixture {
        _dir: dir,
        db,
        settings,
        graded,
    }
}

fn open(fx: &Fixture) -> Result<DrillNotes<RealFs>, deck_streak_coordination::drills::VaultError> {
    DrillNotes::open(&fx.settings, RealFs, Rails::vendored().expect("the rails"))
}

/// Polls once with the clock at `now`.
async fn poll(fx: &Fixture, now: UtcMillis) -> Report {
    let ledger = SqliteCronLedger::new(fx.db.clone());
    let sync_runs = SqliteSyncRuns::new(fx.db.clone());
    let clock = ManualClock::new(now);
    let rule = StudyDayRule::default();
    let runner = Runner::new(&ledger, &sync_runs, &NoNotifier, &clock, rule);
    let grants = SqliteXpLedger::new(fx.db.clone());
    let work = DrillPostbackWork::new(open(fx), &fx.db, &grants);
    runner
        .run(&DRILL_POSTBACK, &work)
        .await
        .expect("the run is recorded")
}

/// Every grant as (study day, source, track, amount, scope).
async fn grants(db: &Db) -> Vec<(i64, String, String, i64, String)> {
    let mut tx = db.write().await.expect("a write");
    sqlx::query("SELECT study_day, source, track, amount, scope FROM xp_ledger ORDER BY id")
        .fetch_all(&mut *tx)
        .await
        .expect("the grants")
        .iter()
        .map(|row| {
            (
                row.get("study_day"),
                row.get("source"),
                row.get("track"),
                row.get("amount"),
                row.get("scope"),
            )
        })
        .collect()
}

async fn grade_rows(db: &Db) -> Vec<(String, String, String, i64, i64)> {
    let mut tx = db.write().await.expect("a write");
    sqlx::query(
        "SELECT drill_id, drill_type, subject, xp, study_day FROM drill_grades ORDER BY drill_id",
    )
    .fetch_all(&mut *tx)
    .await
    .expect("the grades")
    .iter()
    .map(|row| {
        (
            row.get("drill_id"),
            row.get("drill_type"),
            row.get("subject"),
            row.get("xp"),
            row.get("study_day"),
        )
    })
    .collect()
}

fn write(folder: &Path, stem: &str, text: &str) {
    fs::write(folder.join(format!("{stem}.md")), text).expect("a note");
}

#[tokio::test]
async fn a_graded_drill_pays_once_on_the_study_day() {
    let fx = fixture(true).await;
    write(&fx.graded, "irac-1", &graded("xp: 30\n"));
    let first = poll(&fx, at(DAY, 10, 30)).await;
    assert!(
        first.pages.is_empty(),
        "a clean poll pages nothing: {first:?}"
    );
    assert_eq!(
        grants(&fx.db).await,
        vec![(
            DAY,
            "drill:irac-1".to_owned(),
            "law".to_owned(),
            25,
            "once".to_owned()
        )],
        "the clamped amount, on the study day, track law, scope once"
    );
    let grades = grade_rows(&fx.db).await;
    assert_eq!(grades.len(), 1, "one grade row");
    assert_eq!(
        (grades[0].0.as_str(), grades[0].3, grades[0].4),
        ("irac-1", 25, DAY)
    );
    let again = poll(&fx, at(DAY, 11, 30)).await;
    assert!(
        matches!(
            again.decision,
            Decision::Ran {
                outcome: Outcome::Ok,
                ..
            }
        ),
        "the re-poll ran: {again:?}"
    );
    let next_day = poll(&fx, at(DAY + 1, 12, 30)).await;
    assert!(next_day.pages.is_empty(), "{next_day:?}");
    assert_eq!(grants(&fx.db).await.len(), 1, "a re-poll pays nothing more");
    assert_eq!(
        grade_rows(&fx.db).await.len(),
        1,
        "a re-poll grades nothing twice"
    );
    println!("examined 3 poll(s)");
}

#[tokio::test]
async fn a_poll_before_the_rollover_pays_the_previous_study_day() {
    let fx = fixture(true).await;
    write(&fx.graded, "irac-1", &graded(""));
    let report = poll(&fx, at(DAY, 2, 30)).await;
    assert!(report.pages.is_empty(), "{report:?}");
    let paid = grants(&fx.db).await;
    assert_eq!(paid.len(), 1, "one grant");
    assert_eq!(
        (paid[0].0, paid[0].3),
        (DAY - 1, 15),
        "the previous study day, the default XP"
    );
    assert_eq!(
        grade_rows(&fx.db).await[0].4,
        DAY - 1,
        "the grade's study day"
    );
}

#[tokio::test]
async fn a_missing_root_is_an_error_and_a_missing_graded_folder_is_zero() {
    let fx = fixture(false).await;
    let none = poll(&fx, at(DAY, 10, 30)).await;
    assert!(
        none.pages.is_empty(),
        "a missing Graded folder is not an error: {none:?}"
    );
    assert!(grants(&fx.db).await.is_empty(), "and pays nothing");

    let root = fixture(false).await;
    let vault = root.settings.root.path().to_path_buf();
    fs::remove_dir_all(&vault).expect("the vault goes");
    let report = poll(&root, at(DAY, 10, 30)).await;
    assert!(
        matches!(
            report.decision,
            Decision::Ran {
                outcome: Outcome::Error,
                ..
            }
        ),
        "a missing root is the job's error: {report:?}"
    );
    assert_eq!(report.pages.len(), 1, "and it pages once");
    assert_eq!(report.pages[0].code, "drill_vault_root_unreadable");
}

#[tokio::test]
async fn an_interrupted_poll_is_completed_without_a_second_pay() {
    let fx = fixture(true).await;
    write(&fx.graded, "irac-1", &graded("xp: 20\n"));
    // A poll that stopped after the grant and before the grade row: the grant, and no grade.
    let grants_port = SqliteXpLedger::new(fx.db.clone());
    let request = GrantRequest {
        study_day: deck_streak_kernel::StudyDay::from_epoch_day(DAY - 2),
        source: GrantSource::new("drill:irac-1").expect("a source"),
        track: Track::Law,
        amount: XpAmount::new(20),
        scope: GrantScope::Once,
    };
    let _ = grants_port
        .grant(&request, at(DAY - 2, 10, 0))
        .await
        .expect("the earlier grant");
    assert!(grade_rows(&fx.db).await.is_empty(), "no grade row yet");
    let report = poll(&fx, at(DAY, 10, 30)).await;
    assert!(report.pages.is_empty(), "{report:?}");
    let paid = grants(&fx.db).await;
    assert_eq!(paid.len(), 1, "the next poll pays nothing a second time");
    assert_eq!(paid[0].0, DAY - 2, "the first grant stands");
    let grades = grade_rows(&fx.db).await;
    assert_eq!(grades.len(), 1, "the next poll completes the grade row");
    assert_eq!(grades[0].3, 20, "at the accepted XP");
}
