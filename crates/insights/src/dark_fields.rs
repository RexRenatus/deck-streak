//! Dark Fields (SPEC-094 R10, R11, #142): authored content no card template renders.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::structure::StructureReads;
use serde::Serialize;

use crate::instrument::{Cadence, Instrument};

/// The instrument's id.
pub const ID: &str = "dark_fields";
/// A field is dark only when this many reviewed notes have content in it.
pub const MIN_DARK_NOTES: i64 = 0;
/// The most dark fields a report shows.
pub const MAX_DARK_FIELDS_SHOWN: usize = 0;
/// The most unparseable note types a report shows.
pub const MAX_UNPARSEABLE_SHOWN: usize = 0;
/// Anki's own template names, never field references.
pub const SPECIAL_FIELD_NAMES: [&str; 0] = [];
/// The characters that open a section, stripped before a name.
pub const SECTION_PREFIXES: &str = "";
/// The template config's field number of the front format.
pub const Q_FORMAT_FIELD: u128 = 0;
/// The template config's field number of the back format.
pub const A_FORMAT_FIELD: u128 = 0;

/// Everything the pure build reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DarkFieldsInput {
    /// Each template's note type id, ordinal, note type name and template name.
    pub template_names: Vec<(i64, i64, String, String)>,
    /// Each template's note type id, ordinal and raw config.
    pub template_configs: Vec<(i64, i64, Option<Vec<u8>>)>,
    /// Each declared field's note type id, ordinal and name.
    pub declared_fields: Vec<(i64, i64, String)>,
    /// Reviewed notes with content, by note type id and field ordinal.
    pub presence: BTreeMap<(i64, i64), i64>,
    /// How many notes were reviewed.
    pub reviewed_note_count: i64,
    /// The name of each read that failed.
    pub failed_reads: Vec<String>,
    /// Whether the names were made safe at the read, so a token is compared made safe too.
    pub names_are_safe: bool,
}

/// One field no template references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DarkField {
    /// The note type's name.
    pub note_type: String,
    /// The field's name.
    pub field: String,
    /// Reviewed notes with content in it.
    pub reviewed_notes: i64,
}

/// A note type whose templates could not be verified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Unparseable {
    /// The note type's name.
    pub note_type: String,
    /// The note type's id.
    pub note_type_id: i64,
}

/// The whole report, before the cap.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Every dark field.
    pub dark_fields: Vec<DarkField>,
    /// Every note type that could not be verified.
    pub unparseable: Vec<Unparseable>,
    /// The note types checked.
    pub notetypes_checked: i64,
    /// How many notes were reviewed.
    pub reviewed_note_count: i64,
    /// Whether there was nothing to check.
    pub is_cold: bool,
    /// The name of each read that failed.
    pub failed_reads: Vec<String>,
}

/// The report as it is stored and served: capped, with the totals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct View {
    /// The dark fields shown.
    pub dark_fields: Vec<DarkField>,
    /// How many dark fields there are.
    pub dark_fields_total: usize,
    /// The unparseable note types shown.
    pub unparseable: Vec<Unparseable>,
    /// How many unparseable note types there are.
    pub unparseable_total: usize,
    /// The note types checked.
    pub notetypes_checked: i64,
    /// How many notes were reviewed.
    pub reviewed_note_count: i64,
    /// Whether there was nothing to check.
    pub is_cold: bool,
    /// The name of each read that failed.
    pub failed_reads: Vec<String>,
}

impl From<&StructureReads> for DarkFieldsInput {
    fn from(reads: &StructureReads) -> Self {
        let _ = reads;
        Self::default()
    }
}

/// The fields one template config references, and whether it failed to decode.
#[must_use]
pub fn config_tokens(config: Option<&[u8]>) -> (BTreeSet<String>, bool) {
    let _ = config;
    (BTreeSet::new(), false)
}

/// The report over `input`.
#[must_use]
pub fn build_report(input: &DarkFieldsInput) -> Report {
    let _ = input;
    Report::default()
}

/// Dark Fields.
#[derive(Clone, Copy, Debug, Default)]
pub struct DarkFields;

impl Instrument for DarkFields {
    type Reads = StructureReads;
    type Report = View;

    fn id(&self) -> &'static str {
        ID
    }

    fn cadence(&self) -> Cadence {
        Cadence::OnDemand
    }

    fn schema_version(&self) -> u32 {
        0
    }

    fn build(&self, reads: &StructureReads) -> View {
        let _ = reads;
        View {
            dark_fields: Vec::new(),
            dark_fields_total: 0,
            unparseable: Vec::new(),
            unparseable_total: 0,
            notetypes_checked: 0,
            reviewed_note_count: 0,
            is_cold: false,
            failed_reads: Vec::new(),
        }
    }

    fn failed_reads(&self, report: &View) -> Vec<String> {
        let _ = report;
        Vec::new()
    }
}
