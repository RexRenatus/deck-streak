//! The owner's tap on a reading (SPEC-047 A1, A2, A3, A9; R1, R2, R6, R9): the first tap sets
//! `read_at`, grants 40 XP once and ticks the vault line; a later tap changes nothing and retries
//! only a failed vault tick; only the tap ticks the line; and an unknown reading earns nothing.
//!
//! Nothing reaches a vault or a network: the vault is a fake, and every reading here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::panic, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deck_streak_coordination::readings::generate::{PortFuture, VaultWriteFailed};
use deck_streak_coordination::readings::read_tap::{ReadTap, ReadTick, TapAnswer, TapError};
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::xp::XpTotal;
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::store::{NewReading, SqliteReadings, VaultStatus, VaultTick};
use deck_streak_readings::studied::Verdict;
use deck_streak_readings::topic::TopicKey;

const DAY_MS: i64 = 86_400_000;
/// 05:00 UTC of epoch day 20 000: an hour into study day 20 000 under the default rule.
const START: i64 = 20_000 * DAY_MS + 5 * 3_600_000;
const TOPIC: &str = "language/synthetic-greetings";

#[derive(Clone, Default)]
struct FakeVault {
    calls: Arc<Mutex<Vec<String>>>,
    failing: Arc<AtomicBool>,
}

impl FakeVault {
    fn calls(&self) -> usize {
        self.calls.lock().expect("the call log").len()
    }
}

impl ReadTick for FakeVault {
    fn tick_read<'a>(
        &'a self,
        topic: &'a TopicKey,
        _today: StudyDay,
    ) -> PortFuture<'a, Result<(), VaultWriteFailed>> {
        self.calls
            .lock()
            .expect("the call log")
            .push(topic.as_str().to_owned());
        let failing = self.failing.load(Ordering::SeqCst);
        Box::pin(async move {
            if failing {
                Err(VaultWriteFailed)
            } else {
                Ok(())
            }
        })
    }
}

struct Rig {
    _scratch: tempfile::TempDir,
    readings: SqliteReadings,
    ledger: SqliteXpLedger,
    vault: FakeVault,
    clock: Arc<ManualClock>,
    tap: ReadTap<SqliteXpLedger, FakeVault>,
    id: ReadingId,
}

impl Rig {
    async fn new() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("a database");
        let readings = SqliteReadings::new(db.clone());
        let ledger = SqliteXpLedger::new(db.clone());
        let vault = FakeVault::default();
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START)));
        let id = ReadingId::of(TOPIC, 20_000, &"a".repeat(64));
        readings
            .store_reading(&NewReading {
                id: id.clone(),
                topic: TopicKey::parse(TOPIC).expect("a topic"),
                study_day: StudyDayRule::default().study_day(UtcMillis::from_epoch_millis(START)),
                digest: "a".repeat(64),
                persona: "language-synthetic".to_owned(),
                text: "a synthetic reading".to_owned(),
                word_count: 900,
                minutes: 5,
                card_ids: vec![1, 2, 3, 4, 5],
                note_count: 2,
                generated_at: UtcMillis::from_epoch_millis(START),
                vault: VaultStatus::Written("readings/20000/synthetic.md".to_owned()),
            })
            .await
            .expect("a stored reading");
        let tap = ReadTap::new(
            readings.clone(),
            SqliteXpLedger::new(db),
            vault.clone(),
            clock.clone(),
            StudyDayRule::default(),
        );
        Self {
            _scratch: scratch,
            readings,
            ledger,
            vault,
            clock,
            tap,
            id,
        }
    }

    async fn progress(&self) -> deck_streak_readings::store::ReadingProgress {
        let row = self.readings.progress(&self.id).await.expect("a read");
        assert!(row.is_some(), "the reading has a row");
        row.expect("a row")
    }

    async fn total(&self) -> XpTotal {
        self.ledger.total().await.expect("a total")
    }
}

async fn must_tap(rig: &Rig) -> TapAnswer {
    let answer = tap(rig, rig.id.as_str()).await;
    assert!(answer.is_ok(), "the tap answered: {answer:?}");
    answer.expect("an answer")
}

async fn tap(rig: &Rig, id: &str) -> Result<TapAnswer, TapError> {
    rig.tap.tap_reading(id).await
}

#[tokio::test]
async fn a_second_read_tap_changes_nothing() {
    let rig = Rig::new().await;
    let first = must_tap(&rig).await;
    assert!(first.first, "the first tap is the first");
    assert_eq!(first.read_at, UtcMillis::from_epoch_millis(START));
    assert_eq!(rig.total().await, XpTotal::new(40), "40 XP for the tap");
    assert_eq!(
        rig.ledger
            .track_total(Track::Language)
            .await
            .expect("a total"),
        XpTotal::new(40),
        "on the reading's own track"
    );
    assert_eq!(rig.vault.calls(), 1, "the line is ticked once");

    // R9: the answer carries the reading's covered and studied counts and its verdict.
    assert_eq!(first.covered, 5);
    assert_eq!(first.studied, 0);
    assert_eq!(first.verdict, Verdict::Open);

    let stored = rig.progress().await;
    assert_eq!(stored.read_at, Some(UtcMillis::from_epoch_millis(START)));
    assert_eq!(stored.vault_tick, VaultTick::Written);

    // A day later, a second tap answers the first read_at and changes nothing.
    rig.clock
        .advance(Duration::from_millis(u64::try_from(DAY_MS).expect("a day")));
    let second = must_tap(&rig).await;
    assert!(!second.first);
    assert_eq!(second.read_at, first.read_at, "the first read_at");
    assert_eq!(rig.total().await, XpTotal::new(40), "nothing granted again");
    assert_eq!(rig.vault.calls(), 1, "the line is not ticked again");
    let after = rig.progress().await;
    assert_eq!(after, stored, "no state changed");
}

#[tokio::test]
async fn a_later_tap_retries_only_a_failed_vault_tick() {
    let rig = Rig::new().await;
    rig.vault.failing.store(true, Ordering::SeqCst);
    let first = must_tap(&rig).await;
    assert!(first.first, "the tap stands although the tick failed");
    assert_eq!(rig.total().await, XpTotal::new(40));
    let pending = rig.progress().await;
    assert_eq!(pending.vault_tick, VaultTick::Pending);
    assert_eq!(pending.read_at, Some(UtcMillis::from_epoch_millis(START)));

    // A later tap with the vault still failing retries the tick alone.
    rig.clock.advance(Duration::from_hours(1));
    let second = must_tap(&rig).await;
    assert!(!second.first);
    assert_eq!(second.read_at, first.read_at);
    assert_eq!(rig.vault.calls(), 2, "the failed tick is retried");
    assert_eq!(rig.total().await, XpTotal::new(40), "no second grant");

    // Once the vault answers, the next tap writes the line and the one after leaves it be.
    rig.vault.failing.store(false, Ordering::SeqCst);
    must_tap(&rig).await;
    assert_eq!(rig.vault.calls(), 3);
    let written = rig.progress().await;
    assert_eq!(written.vault_tick, VaultTick::Written);
    assert_eq!(written.read_at, pending.read_at, "read_at never moves");
    must_tap(&rig).await;
    assert_eq!(rig.vault.calls(), 3, "a written tick is never retried");
    assert_eq!(rig.total().await, XpTotal::new(40));
}

#[tokio::test]
async fn no_xp_is_granted_for_a_failed_or_missing_reading() {
    let rig = Rig::new().await;
    // A failed topic stores no reading, so its would-be id is unknown to the record.
    let failed = ReadingId::of("law/synthetic-failed", 20_000, &"b".repeat(64));
    for id in [failed.as_str(), "not-an-id", ""] {
        let refused = tap(&rig, id).await;
        assert!(
            matches!(refused, Err(TapError::NotFound)),
            "{id:?} is refused as unknown"
        );
    }
    assert_eq!(rig.total().await, XpTotal::new(0), "nothing was granted");
    assert_eq!(rig.vault.calls(), 0, "no line was ticked");
    let untouched = rig.progress().await;
    assert_eq!(untouched.read_at, None, "the stored reading is untouched");
    assert_eq!(untouched.vault_tick, VaultTick::None);
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every Rust file under `directory`, recursively, by path.
fn sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Whether a code line of `source` calls the vault's read tick.
fn calls_read_tick(source: &str) -> bool {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .any(|line| line.contains(".tick_read("))
}

#[test]
fn the_read_line_is_written_only_by_the_tap() {
    let root = root();
    let mut population = Vec::new();
    for entry in fs::read_dir(root.join("crates")).expect("the crates") {
        let src = entry.expect("a crate").path().join("src");
        if src.is_dir() {
            population.extend(sources(&src));
        }
    }
    println!("examined {} source file(s)", population.len());
    assert!(!population.is_empty(), "examined 0 source files");

    let callers: Vec<&PathBuf> = population
        .iter()
        .filter(|path| calls_read_tick(&fs::read_to_string(path).expect("a source")))
        .collect();
    assert_eq!(callers.len(), 1, "one caller of the read tick: {callers:?}");
    assert!(
        callers[0].ends_with("crates/coordination/src/readings/read_tap.rs"),
        "the caller is the tap use case, not {:?}",
        callers[0]
    );

    // Controls: a planted caller is refused, prose naming the call is not.
    assert!(calls_read_tick("let _ = tree.tick_read(day, &topic);\n"));
    assert!(!calls_read_tick("// the tap calls .tick_read( once\n"));

    // Nothing else writes the line: the settle's stamp port names the Studied line only.
    let settle = fs::read_to_string(root.join("crates/coordination/src/readings/settle.rs"))
        .expect("the settle source");
    assert!(
        !settle.contains("ReadTick"),
        "the settle names no read tick"
    );
}
