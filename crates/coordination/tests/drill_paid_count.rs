//! The hourly post-back reports how many drills it graded and paid (SPEC-110 A9, A10; R9). This
//! file holds one test on purpose: `tracing` caches each event's interest by callsite, so a
//! sibling test that hits the callsite first, under no subscriber, would blind this one.

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use deck_streak_coordination::delivery::NoNotifier;
use deck_streak_coordination::drills::{DrillNotes, DrillPostbackWork, RealFs};
use deck_streak_coordination::jobs::DRILL_POSTBACK;
use deck_streak_coordination::ledger::SqliteCronLedger;
use deck_streak_coordination::runner::{Report, Runner};
use deck_streak_ingest::sync_runs::SqliteSyncRuns;
use deck_streak_kernel::{Db, Environment, ManualClock, StudyDayRule, UtcMillis};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::{Rails, VaultSettings};
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

fn write(folder: &Path, stem: &str, text: &str) {
    fs::write(folder.join(format!("{stem}.md")), text).expect("a note");
}

/// The INFO events this crate logs on the test's thread, each as its fields in order.
#[derive(Clone, Default)]
struct Polled(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl Polled {
    fn logged(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

struct Fields(String);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        let _ = write!(self.0, "{}={value:?}", field.name());
    }
}

impl tracing::Subscriber for Polled {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let metadata = event.metadata();
        if *metadata.level() == tracing::Level::INFO
            && metadata.target() == "deck_streak_coordination::drills"
        {
            let mut fields = Fields(String::new());
            event.record(&mut fields);
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(fields.0);
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[tokio::test]
async fn a_poll_reports_how_many_it_graded_and_how_many_it_paid() {
    let polled = Polled::default();
    let _logging = tracing::subscriber::set_default(polled.clone());
    let fx = fixture(true).await;
    write(&fx.graded, "irac-1", &graded("xp: 12\n"));
    write(&fx.graded, "irac-2", &graded("xp: 14\n"));
    let _ = poll(&fx, at(DAY, 10, 30)).await;
    let _ = poll(&fx, at(DAY, 11, 30)).await;
    assert_eq!(
        polled.logged(),
        vec![
            "message=the drill post-back polled graded=2 paid=2".to_owned(),
            "message=the drill post-back polled graded=2 paid=0".to_owned(),
        ]
    );
}
