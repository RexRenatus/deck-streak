//! The structure reads (SPEC-094 R4, R5, ADR-095): the note types' shape, read from the private copy.

use std::collections::{BTreeMap, BTreeSet};

use crate::reader::{
    CollectionReader, ReadError, allowed_deck_ids, deck_names, read_failed, scope_ids,
};
use crate::schema18::{FIELDS, NOTE_FIELDS, REVIEWED_NOTES, TEMPLATES};

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

/// The character that separates a note's fields in `notes.flds`.
const FIELD_SEPARATOR: char = '\u{1f}';
/// The replacement for a control character in a name (U+FFFD).
const CONTROL_MARKER: char = '\u{fffd}';

/// Whether `character` is white space as Python's `str.strip` counts it: Unicode white space and
/// the four information separators U+001C to U+001F.
#[must_use]
pub fn is_python_space(character: char) -> bool {
    character.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&character)
}

/// A name made safe: each control character but tab, line feed and carriage return is replaced by
/// U+FFFD, and then `& < > " '` are escaped, in that order, as the predecessor's `_safe_name` does.
#[must_use]
pub fn safe_name(name: &str) -> String {
    let mut safe = String::with_capacity(name.len());
    for character in name.chars() {
        match character {
            '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}' => {
                safe.push(CONTROL_MARKER);
            }
            '&' => safe.push_str("&amp;"),
            '<' => safe.push_str("&lt;"),
            '>' => safe.push_str("&gt;"),
            '"' => safe.push_str("&quot;"),
            '\'' => safe.push_str("&#x27;"),
            other => safe.push(other),
        }
    }
    safe
}

/// A template row as [`TEMPLATES`] selects it.
type TemplateRow = (i64, i64, String, String, Option<Vec<u8>>);

impl CollectionReader {
    /// Reads the structure of the note types of the cards in scope, read-only from the private
    /// copy: each read returns its rows, and the name of each that failed is in `failed_reads`.
    ///
    /// # Errors
    ///
    /// [`ReadError::Lock`] when the collection lock cannot be taken, and [`ReadError::Copy`] when
    /// the copy cannot be opened or its decks cannot be read (the scope is unknown then).
    pub async fn read_structure(&self) -> Result<StructureReads, ReadError> {
        let prefixes = self.scope().include().prefixes().to_vec();
        self.with_copy("read_structure", move |copy| async move {
            let decks = deck_names(&copy).await?;
            let scope = scope_ids(&allowed_deck_ids(&decks, &prefixes));
            let mut reads = StructureReads::default();
            match sqlx::query_as::<_, TemplateRow>(TEMPLATES)
                .bind(&scope)
                .fetch_all(copy.reader())
                .await
            {
                Ok(rows) => {
                    reads.templates = rows
                        .into_iter()
                        .map(
                            |(note_type_id, ordinal, note_type, name, config)| Template {
                                note_type_id,
                                ordinal,
                                note_type_name: safe_name(&note_type),
                                name: safe_name(&name),
                                config,
                            },
                        )
                        .collect();
                }
                Err(error) => reads.fail(READ_TEMPLATE_CONFIGS, error),
            }
            match sqlx::query_as::<_, (i64, i64, String)>(FIELDS)
                .bind(&scope)
                .fetch_all(copy.reader())
                .await
            {
                Ok(rows) => {
                    reads.fields = rows
                        .into_iter()
                        .map(|(note_type_id, ordinal, name)| DeclaredField {
                            note_type_id,
                            ordinal,
                            name: safe_name(&name),
                        })
                        .collect();
                }
                Err(error) => reads.fail(READ_DECLARED_FIELDS, error),
            }
            match sqlx::query_scalar::<_, i64>(REVIEWED_NOTES)
                .bind(&scope)
                .fetch_all(copy.reader())
                .await
            {
                Ok(rows) => reads.reviewed_notes = rows.into_iter().collect(),
                Err(error) => reads.fail(READ_REVIEWED_NIDS, error),
            }
            let ordered: Vec<i64> = reads.reviewed_notes.iter().copied().collect();
            for batch in ordered.chunks(NOTES_BATCH_SIZE) {
                let ids = format!(
                    "[{}]",
                    batch
                        .iter()
                        .map(i64::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                );
                match sqlx::query_as::<_, (i64, i64, Option<String>)>(NOTE_FIELDS)
                    .bind(&ids)
                    .fetch_all(copy.reader())
                    .await
                {
                    Ok(rows) => {
                        // Each batch is reduced at once, so no more than one batch's text is held.
                        for (_, note_type, fields) in rows {
                            let fields = fields.unwrap_or_default();
                            for (ordinal, text) in fields.split(FIELD_SEPARATOR).enumerate() {
                                if text.trim_matches(is_python_space).is_empty() {
                                    continue;
                                }
                                let ordinal = i64::try_from(ordinal).unwrap_or(i64::MAX);
                                *reads.presence.entry((note_type, ordinal)).or_insert(0) += 1;
                            }
                        }
                        reads.presence_batches.push(batch.len());
                    }
                    Err(error) => reads.fail(READ_FIELD_PRESENCE, error),
                }
            }
            Ok(reads)
        })
        .await
    }
}

impl StructureReads {
    /// Records that the read `name` failed, once, and logs why without any value.
    fn fail(&mut self, name: &str, error: sqlx::Error) {
        let cause = read_failed(error);
        tracing::warn!(read = name, error = %cause, "a structure read failed");
        if !self.failed_reads.iter().any(|failed| failed == name) {
            self.failed_reads.push(name.to_owned());
        }
    }
}
