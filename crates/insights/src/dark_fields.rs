//! Dark Fields (SPEC-094 R10, R11, #142): authored content no card template renders.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::structure::{StructureReads, is_python_space, safe_name};
use deck_streak_ingest::wire::{WireValue, walk};
use serde::Serialize;

use crate::instrument::{Cadence, Instrument};

/// The instrument's id.
pub const ID: &str = "dark_fields";
/// A field is dark only when this many reviewed notes have content in it.
pub const MIN_DARK_NOTES: i64 = 3;
/// The most dark fields a report shows.
pub const MAX_DARK_FIELDS_SHOWN: usize = 40;
/// The most unparseable note types a report shows.
pub const MAX_UNPARSEABLE_SHOWN: usize = 20;
/// Anki's own template names, never field references.
pub const SPECIAL_FIELD_NAMES: [&str; 6] = ["FrontSide", "Tags", "Type", "Deck", "Subdeck", "Card"];
/// The characters that open a section, stripped before a name.
pub const SECTION_PREFIXES: &str = "#/^";
/// The template config's field number of the front format.
pub const Q_FORMAT_FIELD: u128 = 1;
/// The template config's field number of the back format.
pub const A_FORMAT_FIELD: u128 = 2;

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
        Self {
            template_names: reads
                .templates
                .iter()
                .map(|t| {
                    (
                        t.note_type_id,
                        t.ordinal,
                        t.note_type_name.clone(),
                        t.name.clone(),
                    )
                })
                .collect(),
            template_configs: reads
                .templates
                .iter()
                .map(|t| (t.note_type_id, t.ordinal, t.config.clone()))
                .collect(),
            declared_fields: reads
                .fields
                .iter()
                .map(|f| (f.note_type_id, f.ordinal, f.name.clone()))
                .collect(),
            presence: reads.presence.clone(),
            reviewed_note_count: i64::try_from(reads.reviewed_notes.len()).unwrap_or(i64::MAX),
            failed_reads: reads.failed_reads.clone(),
            names_are_safe: true,
        }
    }
}

/// The front and back formats of one config, and whether a non-empty config failed to decode: a
/// missing or empty config decodes to nothing and is no failure.
fn decode_formats(config: Option<&[u8]>) -> (String, String, bool) {
    let Some(config) = config.filter(|bytes| !bytes.is_empty()) else {
        return (String::new(), String::new(), false);
    };
    let Ok(fields) = walk(config) else {
        return (String::new(), String::new(), true);
    };
    let (mut front, mut back) = (String::new(), String::new());
    for field in fields {
        let WireValue::Length(bytes) = field.value else {
            continue;
        };
        let target = if field.number == Q_FORMAT_FIELD {
            &mut front
        } else if field.number == A_FORMAT_FIELD {
            &mut back
        } else {
            continue;
        };
        match std::str::from_utf8(bytes) {
            Ok(text) => text.clone_into(target),
            Err(_) => return (String::new(), String::new(), true),
        }
    }
    (front, back, false)
}

/// What one scan step found: the token's byte range if a token starts there, and the position
/// after the step.
pub type Step = (Option<(usize, usize)>, usize);

/// The scan at `at` (which must have a byte after it): a token and the position after it, or no
/// token and the next byte.
#[must_use]
pub fn token_step(bytes: &[u8], at: usize) -> Step {
    if bytes[at] == b'{' && bytes[at + 1] == b'{' {
        let start = at + 2;
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'{' || *byte == b'}')
            .map_or(bytes.len(), |found| start + found);
        if end > start && bytes.get(end) == Some(&b'}') && bytes.get(end + 1) == Some(&b'}') {
            return (Some((start, end)), end + 2);
        }
    }
    (None, at + 1)
}

/// The refusal text of a token scan step that did not move the scan forward.
pub const TOKEN_NO_PROGRESS: &str = "template token scan made no progress";

/// Every `{{...}}` whose inside holds no brace, left to right and not overlapping, as the
/// predecessor's pattern `\{\{([^{}]+)\}\}` finds them, with the per-step reader injected so
/// the progress guard is testable alone.
///
/// A step that does not strictly advance the position is refused with [`TOKEN_NO_PROGRESS`]
/// before anything is kept, as the wire walk refuses one: a stalled reader would otherwise spin
/// or push without bound, and a scan that stopped and answered what it had kept would judge a
/// note type on part of its templates, reporting fields they do render as dark.
///
/// # Errors
///
/// [`TOKEN_NO_PROGRESS`] when a step stays or steps back.
pub fn tokens_with<F>(text: &str, mut step: F) -> Result<Vec<&str>, &'static str>
where
    F: FnMut(&[u8], usize) -> Step,
{
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at + 1 < bytes.len() {
        let (token, next) = step(bytes, at);
        if next <= at {
            return Err(TOKEN_NO_PROGRESS);
        }
        if let Some((start, end)) = token {
            found.push(&text[start..end]);
        }
        at = next;
    }
    Ok(found)
}

/// One captured token as a field name, or none for a special name or an empty one.
fn normalise(raw: &str) -> Option<String> {
    let mut name = raw.trim_matches(is_python_space);
    if name.is_empty() {
        return None;
    }
    if name.starts_with(|first| SECTION_PREFIXES.contains(first)) {
        name = name[1..].trim_matches(is_python_space);
    } else if let Some((_, last)) = name.rsplit_once(':') {
        name = last.trim_matches(is_python_space);
    }
    if name.is_empty() || SPECIAL_FIELD_NAMES.contains(&name) {
        return None;
    }
    Some(name.to_owned())
}

/// The fields one template config references, and whether it failed to decode.
#[must_use]
pub fn config_tokens(config: Option<&[u8]>) -> (BTreeSet<String>, bool) {
    config_tokens_with(config, token_step)
}

/// [`config_tokens`] with the scan step injected. A scan that refuses marks the template as
/// failed to decode, as a wire walk that refuses does, so its note type is named unparseable
/// rather than judged on a partial token set.
#[must_use]
pub fn config_tokens_with<F>(config: Option<&[u8]>, mut step: F) -> (BTreeSet<String>, bool)
where
    F: FnMut(&[u8], usize) -> Step,
{
    let (front, back, failed) = decode_formats(config);
    let (Ok(front_tokens), Ok(back_tokens)) = (
        tokens_with(&front, &mut step),
        tokens_with(&back, &mut step),
    ) else {
        return (BTreeSet::new(), true);
    };
    let tokens = front_tokens
        .into_iter()
        .chain(back_tokens)
        .filter_map(normalise)
        .collect();
    (tokens, failed)
}

/// The report over `input`: per note type, the declared fields no template references, among
/// those with content in enough reviewed notes. A failed read dominates and claims nothing; a
/// collection with nothing reviewed or declared is cold.
#[must_use]
pub fn build_report(input: &DarkFieldsInput) -> Report {
    let mut report = Report {
        reviewed_note_count: input.reviewed_note_count,
        ..Report::default()
    };
    if !input.failed_reads.is_empty() {
        report.failed_reads.clone_from(&input.failed_reads);
        return report;
    }
    if input.reviewed_note_count == 0 || input.declared_fields.is_empty() {
        report.is_cold = true;
        return report;
    }

    let mut order: Vec<i64> = Vec::new();
    let mut fields: BTreeMap<i64, Vec<(i64, &str)>> = BTreeMap::new();
    for (ntid, ordinal, name) in &input.declared_fields {
        if !fields.contains_key(ntid) {
            order.push(*ntid);
        }
        fields.entry(*ntid).or_default().push((*ordinal, name));
    }
    let mut tokens: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    let mut undecodable: BTreeSet<i64> = BTreeSet::new();
    for (ntid, _, config) in &input.template_configs {
        let (found, failed) = config_tokens(config.as_deref());
        tokens.entry(*ntid).or_default().extend(found);
        if failed {
            undecodable.insert(*ntid);
        }
    }
    // A template keyed twice keeps its last names, in the first key's place; a note type is
    // named by its first template.
    let mut named: Vec<((i64, i64), &str)> = Vec::new();
    for (ntid, ordinal, note_type, _) in &input.template_names {
        match named.iter_mut().find(|(key, _)| *key == (*ntid, *ordinal)) {
            Some(slot) => slot.1 = note_type,
            None => named.push(((*ntid, *ordinal), note_type)),
        }
    }

    for ntid in order {
        let name = named.iter().find(|((id, _), _)| *id == ntid).map_or_else(
            || format!("UNKNOWN (ntid {ntid})"),
            |(_, name)| (*name).to_owned(),
        );
        let referenced = tokens.get(&ntid).filter(|set| !set.is_empty());
        let Some(referenced) = referenced.filter(|_| !undecodable.contains(&ntid)) else {
            report.unparseable.push(Unparseable {
                note_type: name,
                note_type_id: ntid,
            });
            continue;
        };
        let referenced: BTreeSet<String> = if input.names_are_safe {
            referenced.iter().map(|token| safe_name(token)).collect()
        } else {
            referenced.clone()
        };
        report.notetypes_checked += 1;
        for (ordinal, field) in &fields[&ntid] {
            if referenced.contains(*field) {
                continue;
            }
            let count = input.presence.get(&(ntid, *ordinal)).copied().unwrap_or(0);
            if count >= MIN_DARK_NOTES {
                report.dark_fields.push(DarkField {
                    note_type: name.clone(),
                    field: (*field).to_owned(),
                    reviewed_notes: count,
                });
            }
        }
    }
    report
        .dark_fields
        .sort_by(|a, b| (&a.note_type, &a.field).cmp(&(&b.note_type, &b.field)));
    report
        .unparseable
        .sort_by(|a, b| (&a.note_type, a.note_type_id).cmp(&(&b.note_type, b.note_type_id)));
    report
}

impl Report {
    /// The report as it is stored and served: the dark fields and unparseable note types capped,
    /// and every total.
    #[must_use]
    pub fn view(self) -> View {
        let dark_fields_total = self.dark_fields.len();
        let unparseable_total = self.unparseable.len();
        let mut dark_fields = self.dark_fields;
        dark_fields.truncate(MAX_DARK_FIELDS_SHOWN);
        let mut unparseable = self.unparseable;
        unparseable.truncate(MAX_UNPARSEABLE_SHOWN);
        View {
            dark_fields,
            dark_fields_total,
            unparseable,
            unparseable_total,
            notetypes_checked: self.notetypes_checked,
            reviewed_note_count: self.reviewed_note_count,
            is_cold: self.is_cold,
            failed_reads: self.failed_reads,
        }
    }
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
        Cadence::Weekly
    }

    fn schema_version(&self) -> u32 {
        1
    }

    fn build(&self, reads: &StructureReads) -> View {
        build_report(&DarkFieldsInput::from(reads)).view()
    }

    fn failed_reads(&self, report: &View) -> Vec<String> {
        report.failed_reads.clone()
    }
}
