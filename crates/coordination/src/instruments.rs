//! The instruments step, the on-demand run and the store of each instrument's latest report
//! (SPEC-094 R7 to R9; ADR-094).

use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use deck_streak_ingest::lock::CollectionLock;
use deck_streak_insights::instrument::{Cadence, Instrument, ReportEnvelope, envelope, failure};
use deck_streak_insights::registry::{self, Row};
use deck_streak_kernel::{Clock, Db, KernelError, Offload, StudyDayRule, UtcMillis};
use serde_json::Value;
use sqlx::Row as _;

/// A boxed future, so the ports below are object safe.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Study days a weekly instrument's stored report may age before the step runs it again.
pub const WEEKLY_DAYS: i64 = 7;
/// The file, in the state directory, whose `flock` keeps two instrument runs from overlapping.
pub const LOCK_FILE: &str = "instruments.lock";

/// One instrument's latest stored run.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredReport {
    /// The instrument's id.
    pub instrument: String,
    /// The study day the run belongs to.
    pub study_day: i64,
    /// The report's schema version.
    pub schema_version: u32,
    /// The run's envelope as JSON.
    pub report: Value,
    /// When the row was written.
    pub created_at: i64,
}

/// The store of each instrument's latest report.
#[derive(Clone, Debug)]
pub struct InstrumentStore {
    db: Db,
}

impl InstrumentStore {
    /// A store over `db`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// Replaces the instrument's report with `envelope`, in one write.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn replace(
        &self,
        envelope: &ReportEnvelope,
        at: UtcMillis,
    ) -> Result<(), KernelError> {
        let json = serde_json::to_string(envelope).map_err(|_| KernelError::Offload {
            operation: "encode an instrument report",
        })?;
        let mut write = self.db.write().await?;
        sqlx::query(REPLACE)
            .bind(&envelope.instrument)
            .bind(envelope.study_day)
            .bind(i64::from(envelope.schema_version))
            .bind(json)
            .bind(at.epoch_millis())
            .execute(&mut *write)
            .await?;
        write.commit().await?;
        Ok(())
    }

    /// The stored report of `instrument`, when there is one.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn get(&self, instrument: &str) -> Result<Option<StoredReport>, KernelError> {
        let row = sqlx::query(SELECT_ONE)
            .bind(instrument)
            .fetch_optional(self.db.reader())
            .await?;
        Ok(row.as_ref().and_then(stored_of))
    }

    /// Every stored report, by instrument id.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn list(&self) -> Result<Vec<StoredReport>, KernelError> {
        let rows = sqlx::query(SELECT_ALL).fetch_all(self.db.reader()).await?;
        Ok(rows.iter().filter_map(stored_of).collect())
    }
}

/// One write replaces the instrument's row: the id is the key, so the table never holds two.
const REPLACE: &str = "INSERT INTO instrument_reports \
    (instrument, study_day, schema_version, report_json, created_at) \
    VALUES (?1, ?2, ?3, ?4, ?5) \
    ON CONFLICT(instrument) DO UPDATE SET study_day = excluded.study_day, \
    schema_version = excluded.schema_version, report_json = excluded.report_json, \
    created_at = excluded.created_at";
/// One instrument's row.
const SELECT_ONE: &str = "SELECT instrument, study_day, schema_version, report_json, created_at \
    FROM instrument_reports WHERE instrument = ?1";
/// Every row, by instrument.
const SELECT_ALL: &str = "SELECT instrument, study_day, schema_version, report_json, created_at \
    FROM instrument_reports ORDER BY instrument";

/// The stored report a row holds; a row whose JSON no longer parses is read as absent, so the
/// instrument is simply due again.
fn stored_of(row: &sqlx::sqlite::SqliteRow) -> Option<StoredReport> {
    let json: String = row.try_get("report_json").ok()?;
    Some(StoredReport {
        instrument: row.try_get("instrument").ok()?,
        study_day: row.try_get("study_day").ok()?,
        schema_version: u32::try_from(row.try_get::<i64, _>("schema_version").ok()?).ok()?,
        report: serde_json::from_str(&json).ok()?,
        created_at: row.try_get("created_at").ok()?,
    })
}

/// Where an instrument's reads come from.
pub trait ReadSource<R>: Send + Sync {
    /// Gathers the reads, or names the failure with words that carry no path or value.
    fn read(&self) -> BoxFuture<'_, Result<R, String>>;
}

/// One instrument as the host runs it.
pub trait InstrumentRunner: Send + Sync {
    /// The instrument's id.
    fn id(&self) -> &'static str;
    /// The report's schema version.
    fn schema_version(&self) -> u32;
    /// Reads, then builds on the offload, named by the id.
    fn run(&self, study_day: i64) -> BoxFuture<'_, Result<ReportEnvelope, String>>;
}

/// An instrument joined to its reads and the offload.
pub struct Frame<I, S> {
    instrument: Arc<I>,
    source: S,
    offload: Offload,
}

impl<I, S> Frame<I, S> {
    /// The frame of `instrument` over `source`.
    #[must_use]
    pub fn new(instrument: I, source: S, offload: Offload) -> Self {
        Self {
            instrument: Arc::new(instrument),
            source,
            offload,
        }
    }
}

impl<I, S> InstrumentRunner for Frame<I, S>
where
    I: Instrument + Send + Sync + 'static,
    I::Reads: Send + 'static,
    S: ReadSource<I::Reads>,
{
    fn id(&self) -> &'static str {
        self.instrument.id()
    }

    fn schema_version(&self) -> u32 {
        self.instrument.schema_version()
    }

    fn run(&self, study_day: i64) -> BoxFuture<'_, Result<ReportEnvelope, String>> {
        Box::pin(async move {
            let reads = self.source.read().await?;
            let instrument = Arc::clone(&self.instrument);
            self.offload
                .run(self.instrument.id(), move || {
                    envelope(&*instrument, study_day, &reads)
                })
                .await
                .map_err(|_| "the run could not be scheduled".to_owned())?
                .map_err(|_| "the report could not be encoded".to_owned())
        })
    }
}

/// What one pass of the step did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepReport {
    /// Instruments that ran and stored a report.
    pub ran: Vec<String>,
    /// Instruments that ran and stored a failure.
    pub failed: Vec<String>,
    /// Instruments left due because another run held the lock.
    pub deferred: Vec<String>,
}

/// Why an on-demand run started nothing.
#[derive(Debug, thiserror::Error)]
pub enum OnDemandRefusal {
    /// No live instrument has that id.
    #[error("no such instrument")]
    Unknown,
    /// Another run holds the lock.
    #[error("a run is in progress")]
    InProgress,
    /// The lock could not be taken.
    #[error("the instrument lock could not be taken")]
    Lock(#[source] io::Error),
    /// The store refused.
    #[error("the report store refused")]
    Store(#[source] KernelError),
}

/// The instruments the host runs, in one process's view.
pub struct Instruments {
    store: InstrumentStore,
    lock: CollectionLock,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
    rows: &'static [Row],
    runners: Vec<Arc<dyn InstrumentRunner>>,
}

impl std::fmt::Debug for Instruments {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Instruments")
            .field("runners", &self.runners.len())
            .finish_non_exhaustive()
    }
}

impl Instruments {
    /// The instruments over `db`, locking in `state_directory`.
    #[must_use]
    pub fn new(
        db: Db,
        state_directory: &Path,
        clock: Arc<dyn Clock>,
        rule: StudyDayRule,
        runners: Vec<Arc<dyn InstrumentRunner>>,
    ) -> Self {
        Self {
            store: InstrumentStore::new(db),
            lock: CollectionLock::new(state_directory.join(LOCK_FILE)),
            clock,
            rule,
            rows: registry::ROWS,
            runners,
        }
    }

    /// The same instruments over other registry rows.
    #[must_use]
    pub fn with_rows(mut self, rows: &'static [Row]) -> Self {
        self.rows = rows;
        self
    }

    /// The store.
    #[must_use]
    pub const fn store(&self) -> &InstrumentStore {
        &self.store
    }

    fn runner(&self, id: &str) -> Option<&Arc<dyn InstrumentRunner>> {
        self.runners.iter().find(|runner| runner.id() == id)
    }

    /// Runs each weekly instrument that is due, one at a time, after a sync's recompute (R7). A
    /// held lock leaves the instrument due for the next sync's step (R8).
    ///
    /// # Errors
    ///
    /// [`KernelError`] when the store cannot be read or written.
    pub async fn step(&self) -> Result<StepReport, KernelError> {
        let today = self.rule.study_day(self.clock.now()).epoch_day();
        let mut report = StepReport::default();
        for row in registry::runnable(self.rows) {
            if row.cadence != Cadence::Weekly {
                continue;
            }
            let Some(runner) = self.runner(row.id) else {
                continue;
            };
            let due = self
                .store
                .get(row.id)
                .await?
                .is_none_or(|stored| today.saturating_sub(stored.study_day) >= WEEKLY_DAYS);
            if !due {
                continue;
            }
            match self.run_held(runner.as_ref(), today).await {
                Ok((_, false)) => report.ran.push(row.id.to_owned()),
                Ok((_, true)) => report.failed.push(row.id.to_owned()),
                Err(OnDemandRefusal::Store(error)) => return Err(error),
                Err(refusal) => {
                    tracing::info!(instrument = row.id, %refusal, "the instrument stays due");
                    report.deferred.push(row.id.to_owned());
                }
            }
        }
        Ok(report)
    }

    /// Runs one instrument now, through the same lock and offload as the step (R8).
    ///
    /// # Errors
    ///
    /// [`OnDemandRefusal`] when nothing started.
    pub async fn run_on_demand(&self, id: &str) -> Result<StoredReport, OnDemandRefusal> {
        if registry::find(self.rows, id).is_none() {
            return Err(OnDemandRefusal::Unknown);
        }
        let runner = self.runner(id).ok_or(OnDemandRefusal::Unknown)?;
        let today = self.rule.study_day(self.clock.now()).epoch_day();
        self.run_held(runner.as_ref(), today)
            .await
            .map(|(stored, _)| stored)
    }

    /// Takes the lock without waiting, runs, stores and releases; the flag says the run failed.
    async fn run_held(
        &self,
        runner: &dyn InstrumentRunner,
        today: i64,
    ) -> Result<(StoredReport, bool), OnDemandRefusal> {
        let held = match self.lock.try_exclusive().await {
            Ok(Some(held)) => held,
            Ok(None) => return Err(OnDemandRefusal::InProgress),
            Err(error) => return Err(OnDemandRefusal::Lock(error)),
        };
        let (envelope, failed) = match runner.run(today).await {
            Ok(envelope) => (envelope, false),
            Err(reason) => (
                failure(runner.id(), today, runner.schema_version(), &reason),
                true,
            ),
        };
        let at = self.clock.now();
        let stored = self.store.replace(&envelope, at).await;
        if let Err(error) = held.release() {
            tracing::warn!(%error, "the instrument lock did not unlock cleanly");
        }
        stored.map_err(OnDemandRefusal::Store)?;
        let report = serde_json::to_value(&envelope).unwrap_or(Value::Null);
        Ok((
            StoredReport {
                instrument: envelope.instrument,
                study_day: envelope.study_day,
                schema_version: envelope.schema_version,
                report,
                created_at: at.epoch_millis(),
            },
            failed,
        ))
    }
}

/// One live instrument as the surfaces list it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentListing {
    /// The instrument's id.
    pub id: String,
    /// How often it runs, as the words `weekly` and `on_demand`.
    pub cadence: &'static str,
    /// The study day of its stored report; `None` while no run has stored one.
    pub study_day: Option<i64>,
}

/// What the api and the bot hold of the instruments: an object-safe port, so neither names the
/// ingest context the runs read through (the context map's edges).
pub trait InstrumentService: Send + Sync {
    /// Each live instrument, with its stored study day.
    fn list(&self) -> BoxFuture<'_, Result<Vec<InstrumentListing>, KernelError>>;
    /// The stored report of a live instrument; `None` while it has none.
    fn report<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StoredReport>, KernelError>>;
    /// Runs one instrument now; a run already going is answered, never queued (R8).
    fn run<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<StoredReport, OnDemandRefusal>>;
}

impl InstrumentService for Instruments {
    fn list(&self) -> BoxFuture<'_, Result<Vec<InstrumentListing>, KernelError>> {
        Box::pin(async move {
            let mut listings = Vec::new();
            for row in registry::runnable(self.rows) {
                let stored = self.store.get(row.id).await?;
                listings.push(InstrumentListing {
                    id: row.id.to_owned(),
                    cadence: match row.cadence {
                        Cadence::Weekly => "weekly",
                        Cadence::OnDemand => "on_demand",
                    },
                    study_day: stored.map(|report| report.study_day),
                });
            }
            Ok(listings)
        })
    }

    fn report<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StoredReport>, KernelError>> {
        Box::pin(async move {
            if registry::find(self.rows, id).is_none() {
                return Ok(None);
            }
            self.store.get(id).await
        })
    }

    fn run<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<StoredReport, OnDemandRefusal>> {
        Box::pin(self.run_on_demand(id))
    }
}
