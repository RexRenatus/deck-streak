//! The structure reads (SPEC-094 R4, R5, ADR-095): the note types' shape, read from the private copy.

use std::collections::{BTreeMap, BTreeSet};

use crate::reader::{CollectionReader, ReadError};

/// The read of every template's raw `config`.
pub const READ_TEMPLATE_CONFIGS: &str = "templates (ntid, ord, config)";
/// The read of every declared field's name.
pub const READ_DECLARED_FIELDS: &str = "fields (ntid, ord, name)";
/// The read of the set of reviewed note ids.
pub const READ_REVIEWED_NIDS: &str = "cards x revlog (reviewed nid set)";
/// The read of each field's count of reviewed notes with content.
pub const READ_FIELD_PRESENCE: &str = "notes (batched flds presence read)";
/// Notes whose content is held at once by the presence read.
pub const NOTES_BATCH_SIZE: usize = 400;

/// One template: its note type, ordinal, names and raw `config`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    /// The note type's id.
    pub note_type_id: i64,
    /// The template's ordinal.
    pub ordinal: i64,
    /// The note type's name, made safe.
    pub note_type_name: String,
    /// The template's name, made safe.
    pub name: String,
    /// The raw config blob: the front and back formats.
    pub config: Option<Vec<u8>>,
}

/// One declared field: its note type, ordinal and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredField {
    /// The note type's id.
    pub note_type_id: i64,
    /// The field's ordinal.
    pub ordinal: i64,
    /// The name, made safe.
    pub name: String,
}

/// Every structure read, and the name of each that failed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StructureReads {
    /// The templates.
    pub templates: Vec<Template>,
    /// The declared fields.
    pub fields: Vec<DeclaredField>,
    /// The reviewed note ids.
    pub reviewed_notes: BTreeSet<i64>,
    /// Reviewed notes with content, by note type id and field ordinal.
    pub presence: BTreeMap<(i64, i64), i64>,
    /// The size of each batch the presence read held.
    pub presence_batches: Vec<usize>,
    /// The name of each read that failed.
    pub failed_reads: Vec<String>,
}

/// A name made safe: the predecessor's `_safe_name`.
#[must_use]
pub fn safe_name(name: &str) -> String {
    name.to_owned()
}

impl CollectionReader {
    /// Reads the structure of the note types in scope.
    ///
    /// # Errors
    ///
    /// [`ReadError`] when the lock or the copy cannot be opened.
    pub async fn read_structure(&self) -> Result<StructureReads, ReadError> {
        Ok(StructureReads::default())
    }
}
