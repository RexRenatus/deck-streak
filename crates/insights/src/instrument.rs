//! The instrument port (SPEC-094 R6, ADR-094): an id, a cadence, the reads an instrument needs, and
//! a pure build from those reads to a serialisable report that carries its failed reads.

use serde::Serialize;
use serde_json::Value;

/// How often an instrument runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cadence {
    /// After a sync's recompute, once the stored report is seven study days old.
    Weekly,
    /// Only when the owner asks.
    OnDemand,
}

/// One instrument: pure over its reads, so a report is a function of what was read.
pub trait Instrument {
    /// What the instrument reads, gathered by the host before the build.
    type Reads;
    /// The report it builds.
    type Report: Serialize;

    /// The instrument's id, the key of its stored report.
    fn id(&self) -> &'static str;
    /// How often it runs.
    fn cadence(&self) -> Cadence;
    /// The shape of the report's JSON; a change of shape moves it.
    fn schema_version(&self) -> u32;
    /// The report those reads make.
    fn build(&self, reads: &Self::Reads) -> Self::Report;
    /// The name of each read that failed, which the report carries.
    fn failed_reads(&self, report: &Self::Report) -> Vec<String>;
}

/// What is stored and served for one instrument's latest run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReportEnvelope {
    /// The instrument's id.
    pub instrument: String,
    /// The study day the run belongs to.
    pub study_day: i64,
    /// The report's schema version.
    pub schema_version: u32,
    /// The name of each read that failed; a report that names one claims nothing.
    pub failed_reads: Vec<String>,
    /// The report, or null when the run itself failed.
    pub report: Value,
}

/// The envelope of one build of `instrument` over `reads`.
///
/// # Errors
///
/// The serialiser's refusal.
pub fn envelope<I: Instrument>(
    instrument: &I,
    study_day: i64,
    reads: &I::Reads,
) -> Result<ReportEnvelope, serde_json::Error> {
    let report = instrument.build(reads);
    Ok(ReportEnvelope {
        instrument: instrument.id().to_owned(),
        study_day,
        schema_version: instrument.schema_version(),
        failed_reads: instrument.failed_reads(&report),
        report: serde_json::to_value(&report)?,
    })
}

/// The envelope of a run that failed: its failed read is the reason, and no report is claimed.
#[must_use]
pub fn failure(id: &str, study_day: i64, schema_version: u32, reason: &str) -> ReportEnvelope {
    ReportEnvelope {
        instrument: id.to_owned(),
        study_day,
        schema_version,
        failed_reads: vec![reason.to_owned()],
        report: Value::Null,
    }
}
