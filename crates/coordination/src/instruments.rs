//! The instruments step, the on-demand run and the store of each instrument's latest report
//! (SPEC-094 R7 to R9; ADR-094).

use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use deck_streak_insights::instrument::{Instrument, ReportEnvelope};
use deck_streak_insights::registry::Row;
use deck_streak_kernel::{Clock, Db, KernelError, Offload, StudyDayRule, UtcMillis};
use serde_json::Value;

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
        _envelope: &ReportEnvelope,
        _at: UtcMillis,
    ) -> Result<(), KernelError> {
        let _ = &self.db;
        Ok(())
    }

    /// The stored report of `instrument`, when there is one.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn get(&self, _instrument: &str) -> Result<Option<StoredReport>, KernelError> {
        Ok(None)
    }

    /// Every stored report, by instrument id.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn list(&self) -> Result<Vec<StoredReport>, KernelError> {
        Ok(Vec::new())
    }
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

    fn run(&self, _study_day: i64) -> BoxFuture<'_, Result<ReportEnvelope, String>> {
        let _ = (&self.source, &self.offload);
        Box::pin(async { Err("not built".to_owned()) })
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
    rows: &'static [Row],
    runners: Vec<Arc<dyn InstrumentRunner>>,
}

impl Instruments {
    /// The instruments over `db`, locking in `state_directory`.
    #[must_use]
    pub fn new(
        db: Db,
        _state_directory: &Path,
        _clock: Arc<dyn Clock>,
        _rule: StudyDayRule,
        runners: Vec<Arc<dyn InstrumentRunner>>,
    ) -> Self {
        Self {
            store: InstrumentStore::new(db),
            rows: deck_streak_insights::registry::ROWS,
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

    /// Runs each due weekly instrument, one at a time.
    ///
    /// # Errors
    ///
    /// [`KernelError`] when the store cannot be read.
    pub async fn step(&self) -> Result<StepReport, KernelError> {
        let _ = (self.rows, &self.runners);
        Ok(StepReport::default())
    }

    /// Runs one instrument now.
    ///
    /// # Errors
    ///
    /// [`OnDemandRefusal`] when nothing started.
    pub async fn run_on_demand(&self, _id: &str) -> Result<StoredReport, OnDemandRefusal> {
        Err(OnDemandRefusal::Unknown)
    }
}
