//! SPEC-094 A13 to A15 and A21: the instruments step runs each weekly instrument once in seven
//! study days, one failure never stops the next, a held lock starts nothing, and Dark Fields is
//! passed its reads. Every row is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use deck_streak_coordination::instruments::{
    BoxFuture, Frame, InstrumentRunner, Instruments, LOCK_FILE, OnDemandRefusal, ReadSource,
};
use deck_streak_ingest::lock::CollectionLock;
use deck_streak_ingest::structure::{DeclaredField, StructureReads, Template};
use deck_streak_insights::dark_fields::DarkFields;
use deck_streak_insights::instrument::{Cadence, Instrument, ReportEnvelope, failure};
use deck_streak_insights::registry::{Row, State};
use deck_streak_kernel::{
    Clock, Db, ManualClock, Offload, OffloadWorkers, StudyDayRule, SystemClock, UtcMillis,
};
use serde_json::json;
use tempfile::TempDir;

const DAY_MS: i64 = 86_400_000;

/// Every fake's rows, in the registry's order.
static ROWS: [Row; 3] = [
    Row {
        id: "alpha",
        cadence: Cadence::Weekly,
        state: State::Live,
    },
    Row {
        id: "beta",
        cadence: Cadence::Weekly,
        state: State::Live,
    },
    Row {
        id: "gamma",
        cadence: Cadence::Weekly,
        state: State::Inert,
    },
];

struct Fake {
    id: &'static str,
    fails: bool,
    calls: Arc<AtomicUsize>,
}

impl InstrumentRunner for Fake {
    fn id(&self) -> &'static str {
        self.id
    }
    fn schema_version(&self) -> u32 {
        1
    }
    fn run(&self, study_day: i64) -> BoxFuture<'_, Result<ReportEnvelope, String>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let fails = self.fails;
        let id = self.id;
        Box::pin(async move {
            if fails {
                Err("the read failed".to_owned())
            } else {
                Ok(ReportEnvelope {
                    instrument: id.to_owned(),
                    study_day,
                    schema_version: 1,
                    failed_reads: Vec::new(),
                    report: json!({ "ok": true }),
                })
            }
        })
    }
}

struct Rig {
    _dir: TempDir,
    instruments: Instruments,
    clock: Arc<ManualClock>,
    calls: Vec<Arc<AtomicUsize>>,
    lock: CollectionLock,
}

async fn rig(fakes: &[(&'static str, bool)]) -> Rig {
    let dir = TempDir::new().expect("dir");
    let db = Db::open(&dir.path().join("ds.db")).await.expect("open");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        20_000 * DAY_MS + 3_600_000,
    )));
    let calls: Vec<Arc<AtomicUsize>> = fakes
        .iter()
        .map(|_| Arc::new(AtomicUsize::new(0)))
        .collect();
    let runners: Vec<Arc<dyn InstrumentRunner>> = fakes
        .iter()
        .zip(&calls)
        .map(|((id, fails), count)| {
            Arc::new(Fake {
                id,
                fails: *fails,
                calls: count.clone(),
            }) as Arc<dyn InstrumentRunner>
        })
        .collect();
    let shared: Arc<dyn Clock> = clock.clone();
    let instruments =
        Instruments::new(db, dir.path(), shared, StudyDayRule::default(), runners).with_rows(&ROWS);
    let lock = CollectionLock::new(dir.path().join(LOCK_FILE));
    Rig {
        _dir: dir,
        instruments,
        clock,
        calls,
        lock,
    }
}

fn count(rig: &Rig, at: usize) -> usize {
    rig.calls[at].load(Ordering::SeqCst)
}

#[tokio::test]
async fn a_weekly_instrument_runs_once_in_seven_study_days() {
    let rig = rig(&[("alpha", false)]).await;
    let first = rig.instruments.step().await.expect("step");
    assert_eq!(first.ran, vec!["alpha".to_owned()]);
    let again = rig.instruments.step().await.expect("step");
    assert!(again.ran.is_empty(), "not again the same day");
    rig.clock
        .advance(Duration::from_millis(u64::try_from(6 * DAY_MS).unwrap()));
    let sooner = rig.instruments.step().await.expect("step");
    assert!(sooner.ran.is_empty(), "not again after six days");
    assert_eq!(count(&rig, 0), 1);
    rig.clock
        .advance(Duration::from_millis(u64::try_from(DAY_MS).unwrap()));
    let due = rig.instruments.step().await.expect("step");
    assert_eq!(due.ran, vec!["alpha".to_owned()], "again after seven");
    assert_eq!(count(&rig, 0), 2);
}

#[tokio::test]
async fn one_failure_never_stops_the_next_instrument() {
    let rig = rig(&[("alpha", true), ("beta", false)]).await;
    let report = rig.instruments.step().await.expect("step");
    assert_eq!(report.failed, vec!["alpha".to_owned()]);
    assert_eq!(report.ran, vec!["beta".to_owned()]);
    assert_eq!(count(&rig, 1), 1, "the next instrument still ran");
    let stored = rig
        .instruments
        .store()
        .get("alpha")
        .await
        .expect("get")
        .expect("stored");
    let expected = failure("alpha", 20_000, 1, "the read failed");
    assert_eq!(stored.report["failed_reads"], json!(expected.failed_reads));
    assert!(stored.report["report"].is_null());
    let next = rig
        .instruments
        .store()
        .get("beta")
        .await
        .expect("get")
        .expect("stored");
    assert_eq!(next.report["report"]["ok"], true);
}

#[tokio::test]
async fn an_inert_row_never_runs() {
    let rig = rig(&[("alpha", false), ("gamma", false)]).await;
    let report = rig.instruments.step().await.expect("step");
    assert_eq!(report.ran, vec!["alpha".to_owned()]);
    assert_eq!(count(&rig, 1), 0, "an inert row's runner is never called");
    let refusal = rig.instruments.run_on_demand("gamma").await;
    assert!(matches!(refusal, Err(OnDemandRefusal::Unknown)));
}

#[tokio::test]
async fn a_run_while_one_runs_starts_nothing() {
    let rig = rig(&[("alpha", false)]).await;
    let held = rig.lock.try_exclusive().await.expect("lock").expect("free");
    let answer = tokio::time::timeout(
        Duration::from_secs(5),
        rig.instruments.run_on_demand("alpha"),
    )
    .await
    .expect("the request never waits for the holder");
    assert!(
        matches!(answer, Err(OnDemandRefusal::InProgress)),
        "{answer:?}"
    );
    assert_eq!(count(&rig, 0), 0, "nothing started");
    let step = rig.instruments.step().await.expect("step");
    assert_eq!(step.deferred, vec!["alpha".to_owned()]);
    assert!(step.ran.is_empty());
    assert!(
        rig.instruments
            .store()
            .get("alpha")
            .await
            .expect("get")
            .is_none()
    );

    held.release().expect("release");
    let after = rig.instruments.step().await.expect("step");
    assert_eq!(
        after.ran,
        vec!["alpha".to_owned()],
        "still due once the lock is free"
    );
    assert!(
        rig.lock.try_exclusive().await.expect("lock").is_some(),
        "the run released it"
    );
}

struct Given(StructureReads);

impl ReadSource<StructureReads> for Given {
    fn read(&self) -> BoxFuture<'_, Result<StructureReads, String>> {
        let reads = self.0.clone();
        Box::pin(async move { Ok(reads) })
    }
}

/// A config blob whose front format names `{{Front}}` and whose back format names `{{Front}}`.
fn config() -> Vec<u8> {
    let format = b"{{Front}}";
    let mut blob = Vec::new();
    for key in [0x0a_u8, 0x12] {
        blob.push(key);
        blob.push(u8::try_from(format.len()).unwrap());
        blob.extend_from_slice(format);
    }
    blob
}

#[tokio::test]
async fn dark_fields_is_passed_its_reads() {
    let mut reads = StructureReads::default();
    reads.templates.push(Template {
        note_type_id: 1,
        ordinal: 0,
        note_type_name: "Type A".to_owned(),
        name: "Card 1".to_owned(),
        config: Some(config()),
    });
    for (ordinal, name) in [(0, "Front"), (1, "Hidden")] {
        reads.fields.push(DeclaredField {
            note_type_id: 1,
            ordinal,
            name: name.to_owned(),
        });
    }
    reads.reviewed_notes.extend([10, 11, 12, 13, 14]);
    reads.presence.insert((1, 0), 5);
    reads.presence.insert((1, 1), 4);
    let offload = Offload::new(OffloadWorkers::new(1).unwrap(), Arc::new(SystemClock));
    let frame = Frame::new(DarkFields, Given(reads), offload);
    let envelope = frame.run(20_000).await.expect("the run");
    assert_eq!(envelope.instrument, "dark_fields");
    assert_eq!(envelope.study_day, 20_000);
    assert!(envelope.failed_reads.is_empty());
    let dark = envelope.report["dark_fields"].as_array().expect("a list");
    assert_eq!(dark.len(), 1, "{dark:?}");
    assert_eq!(dark[0]["field"], "Hidden");
    assert_eq!(dark[0]["note_type"], "Type A");
    assert_eq!(dark[0]["reviewed_notes"], 4);
    assert_eq!(envelope.report["notetypes_checked"], 1);
}

#[tokio::test]
async fn a_frame_answers_the_id_and_version_of_its_instrument() {
    let offload = Offload::new(OffloadWorkers::new(1).unwrap(), Arc::new(SystemClock));
    let dark = Frame::new(
        DarkFields,
        Given(StructureReads::default()),
        offload.clone(),
    );
    assert_eq!(InstrumentRunner::id(&dark), "dark_fields");
    assert_eq!(InstrumentRunner::schema_version(&dark), 1);
    let seven = Frame::new(Seven, Unit, offload);
    assert_eq!(InstrumentRunner::id(&seven), "seven");
    assert_eq!(InstrumentRunner::schema_version(&seven), 7);
}

/// An instrument whose id and version no shipped instrument shares, so a frame that answered a
/// constant would show.
struct Seven;

impl Instrument for Seven {
    type Reads = ();
    type Report = serde_json::Value;

    fn id(&self) -> &'static str {
        "seven"
    }
    fn cadence(&self) -> Cadence {
        Cadence::OnDemand
    }
    fn schema_version(&self) -> u32 {
        7
    }
    fn build(&self, _reads: &()) -> serde_json::Value {
        json!({})
    }
    fn failed_reads(&self, _report: &serde_json::Value) -> Vec<String> {
        Vec::new()
    }
}

struct Unit;

impl ReadSource<()> for Unit {
    fn read(&self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
