//! Every statement of the store (SPEC-512 R5; ADR-530 D7): each is a constant here or in
//! [`crate::data_rights`], and [`STATEMENTS`] lists every one of them, so the statement census can
//! prepare each against a store at head. A statement binds its values; none is assembled from text.

use crate::data_rights;

/// One registered statement: its constant's name and its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {
    /// The constant's name, as the source writes it.
    pub name: &'static str,
    /// The statement's text.
    pub sql: &'static str,
}

/// The file's `application_id`.
pub const READ_APPLICATION_ID: &str = "PRAGMA application_id";
/// The file's `user_version`.
pub const READ_USER_VERSION: &str = "PRAGMA user_version";
/// How many objects the file's schema holds: 0 for an empty file.
pub const READ_SCHEMA_SIZE: &str = "SELECT count(*) FROM sqlite_schema";
/// The connection's journal mode.
pub const READ_JOURNAL_MODE: &str = "PRAGMA journal_mode";
/// The connection's `synchronous` level.
pub const READ_SYNCHRONOUS: &str = "PRAGMA synchronous";
/// Whether the connection enforces foreign keys.
pub const READ_FOREIGN_KEYS: &str = "PRAGMA foreign_keys";
/// Whether the connection overwrites a deleted row's bytes.
pub const READ_SECURE_DELETE: &str = "PRAGMA secure_delete";
/// Whether the connection refuses every write.
pub const READ_QUERY_ONLY: &str = "PRAGMA query_only";
/// The connection's busy timeout.
pub const READ_BUSY_TIMEOUT: &str = "PRAGMA busy_timeout";
/// The file's integrity check.
pub const INTEGRITY_CHECK: &str = "PRAGMA integrity_check";
/// The file's foreign key check.
pub const FOREIGN_KEY_CHECK: &str = "PRAGMA foreign_key_check";

/// Foreign keys enforced.
pub const SET_FOREIGN_KEYS: &str = "PRAGMA foreign_keys = ON";
/// A deleted row's bytes overwritten.
pub const SET_SECURE_DELETE: &str = "PRAGMA secure_delete = ON";
/// The write-ahead log, on native targets.
pub const SET_JOURNAL_WAL: &str = "PRAGMA journal_mode = WAL";
/// The server's durability: every commit reaches the disk before it returns.
pub const SET_SYNCHRONOUS_FULL: &str = "PRAGMA synchronous = FULL";
/// A device's durability, the kernel's own.
pub const SET_SYNCHRONOUS_NORMAL: &str = "PRAGMA synchronous = NORMAL";
/// Every write refused, for a reader or a foreign file.
pub const SET_QUERY_ONLY: &str = "PRAGMA query_only = ON";

/// The store's record, written once: its role, writer id and slot.
pub const INSERT_STORE_RECORD: &str = "INSERT OR IGNORE INTO store_meta \
     (id, role, origin, slot, id_floor, next_seq, import_source) VALUES (1, ?1, ?2, ?3, 0, 0, NULL)";
/// The store's record as written.
pub const READ_STORE_RECORD: &str = "SELECT role, origin, slot FROM store_meta WHERE id = 1";

/// Every deck's id and name.
pub const STORE_DECK_NAMES: &str = "SELECT id, name FROM decks ORDER BY id";
/// The cards whose home deck is in the JSON array `?1`, by id.
pub const STORE_CARDS: &str = "SELECT c.id, c.note_id, c.deck_id, c.suspended FROM cards c \
     WHERE c.deck_id IN (SELECT value FROM json_each(?1)) ORDER BY c.id";
/// The tags of the notes of the cards whose home deck is in `?1`.
pub const STORE_CARD_TAGS: &str = "SELECT t.note_id, t.tag FROM note_tags t \
     WHERE t.note_id IN (SELECT c.note_id FROM cards c WHERE c.deck_id IN (SELECT value FROM json_each(?1))) \
     ORDER BY t.note_id, t.tag_key";
/// The unretracted study reviews above `?1` of the cards whose home deck is in `?2`, oldest first.
pub const STORE_STUDY_REVIEWS: &str = "SELECT r.id, r.card_id, r.ease, r.duration_ms, r.kind \
     FROM review_log r JOIN cards c ON c.id = r.card_id \
     WHERE r.id > ?1 AND r.kind IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND c.deck_id IN (SELECT value FROM json_each(?2)) \
     AND NOT EXISTS (SELECT 1 FROM review_retraction x WHERE x.review_id = r.id) ORDER BY r.id";
/// The ids of those reviews, `?3` at a time.
pub const STORE_STUDY_REVIEW_IDS: &str = "SELECT r.id FROM review_log r JOIN cards c ON c.id = r.card_id \
     WHERE r.id > ?1 AND r.kind IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND c.deck_id IN (SELECT value FROM json_each(?2)) \
     AND NOT EXISTS (SELECT 1 FROM review_retraction x WHERE x.review_id = r.id) ORDER BY r.id LIMIT ?3";
/// How many unretracted study reviews at or below `?1` the cards whose home deck is in `?2` have.
pub const STORE_STUDY_REVIEWS_THROUGH: &str = "SELECT count(*) FROM review_log r \
     JOIN cards c ON c.id = r.card_id \
     WHERE r.id <= ?1 AND r.kind IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND c.deck_id IN (SELECT value FROM json_each(?2)) \
     AND NOT EXISTS (SELECT 1 FROM review_retraction x WHERE x.review_id = r.id)";
/// The newest review's id.
pub const STORE_NEWEST_REVIEW: &str = "SELECT max(id) FROM review_log";
/// The templates of the notetypes the cards whose home deck is in `?1` use.
pub const STORE_TEMPLATES: &str = "SELECT t.notetype_id, t.ord, n.name, t.name FROM templates t \
     JOIN notetypes n ON n.id = t.notetype_id \
     WHERE t.notetype_id IN (SELECT DISTINCT no.notetype_id FROM notes no JOIN cards c ON c.note_id = no.id \
     WHERE c.deck_id IN (SELECT value FROM json_each(?1))) ORDER BY t.notetype_id, t.ord";
/// The declared fields of the notetypes the cards whose home deck is in `?1` use.
pub const STORE_FIELDS: &str = "SELECT f.notetype_id, f.ord, f.name FROM fields f \
     WHERE f.notetype_id IN (SELECT DISTINCT no.notetype_id FROM notes no JOIN cards c ON c.note_id = no.id \
     WHERE c.deck_id IN (SELECT value FROM json_each(?1))) ORDER BY f.notetype_id, f.ord";
/// The notes of the cards whose home deck is in `?1` with an unretracted study review of kind 1.
pub const STORE_REVIEWED_NOTES: &str = "SELECT DISTINCT c.note_id FROM cards c \
     JOIN review_log r ON r.card_id = c.id WHERE r.kind = 1 AND r.ease >= 1 \
     AND c.deck_id IN (SELECT value FROM json_each(?1)) \
     AND NOT EXISTS (SELECT 1 FROM review_retraction x WHERE x.review_id = r.id)";
/// The fields of the notes in `?1`, one row a field, by note and ordinal.
pub const STORE_NOTE_FIELDS: &str = "SELECT n.id, n.notetype_id, f.value FROM notes n, json_each(n.fields) f \
     WHERE n.id IN (SELECT value FROM json_each(?1)) ORDER BY n.id, f.key";
/// The cards with an unretracted study review above `?1`.
pub const STORE_STUDIED_CARDS: &str = "SELECT DISTINCT r.card_id FROM review_log r \
     WHERE r.id > ?1 AND r.kind IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND NOT EXISTS (SELECT 1 FROM review_retraction x WHERE x.review_id = r.id)";
/// Every card.
pub const STORE_CARD_COUNT: &str = "SELECT count(*) FROM cards";
/// Every note.
pub const STORE_NOTE_COUNT: &str = "SELECT count(*) FROM notes";
/// Every row of the review log.
pub const STORE_REVIEW_ROW_COUNT: &str = "SELECT count(*) FROM review_log";
/// The review log's rescheduling rows.
pub const STORE_RESCHEDULE_ROW_COUNT: &str =
    "SELECT count(*) FROM review_log WHERE kind = 4 AND ease = 0";

/// A register entry for the constant `$name`.
macro_rules! registered {
    ($($path:ident)::* ) => {
        Statement {
            name: registered!(@last $($path)::*),
            sql: $($path)::*,
        }
    };
    (@last $head:ident) => { stringify!($head) };
    (@last $head:ident :: $($tail:ident)::+) => { registered!(@last $($tail)::+) };
}

/// Every statement of the store.
pub const STATEMENTS: &[Statement] = &[
    registered!(READ_APPLICATION_ID),
    registered!(READ_USER_VERSION),
    registered!(READ_SCHEMA_SIZE),
    registered!(READ_JOURNAL_MODE),
    registered!(READ_SYNCHRONOUS),
    registered!(READ_FOREIGN_KEYS),
    registered!(READ_SECURE_DELETE),
    registered!(READ_QUERY_ONLY),
    registered!(READ_BUSY_TIMEOUT),
    registered!(INTEGRITY_CHECK),
    registered!(FOREIGN_KEY_CHECK),
    registered!(SET_FOREIGN_KEYS),
    registered!(SET_SECURE_DELETE),
    registered!(SET_JOURNAL_WAL),
    registered!(SET_SYNCHRONOUS_FULL),
    registered!(SET_SYNCHRONOUS_NORMAL),
    registered!(SET_QUERY_ONLY),
    registered!(INSERT_STORE_RECORD),
    registered!(READ_STORE_RECORD),
    registered!(STORE_DECK_NAMES),
    registered!(STORE_CARDS),
    registered!(STORE_CARD_TAGS),
    registered!(STORE_STUDY_REVIEWS),
    registered!(STORE_STUDY_REVIEW_IDS),
    registered!(STORE_STUDY_REVIEWS_THROUGH),
    registered!(STORE_NEWEST_REVIEW),
    registered!(STORE_TEMPLATES),
    registered!(STORE_FIELDS),
    registered!(STORE_REVIEWED_NOTES),
    registered!(STORE_NOTE_FIELDS),
    registered!(STORE_STUDIED_CARDS),
    registered!(STORE_CARD_COUNT),
    registered!(STORE_NOTE_COUNT),
    registered!(STORE_REVIEW_ROW_COUNT),
    registered!(STORE_RESCHEDULE_ROW_COUNT),
    registered!(data_rights::OPEN_GATE),
    registered!(data_rights::CLOSE_GATE),
    registered!(data_rights::ANONYMISE_STORE_RECORD),
    registered!(data_rights::ERASE_CLIENT_RECORDS),
    registered!(data_rights::ERASE_RETRACTIONS),
    registered!(data_rights::ERASE_REVIEWS),
    registered!(data_rights::ERASE_FIELD_HISTORY),
    registered!(data_rights::ERASE_NOTE_TAGS),
    registered!(data_rights::ERASE_CARDS),
    registered!(data_rights::ERASE_NOTES),
    registered!(data_rights::ERASE_TEMPLATES),
    registered!(data_rights::ERASE_FIELDS),
    registered!(data_rights::ERASE_NOTETYPES),
    registered!(data_rights::ERASE_DECKS),
    registered!(data_rights::ERASE_PRESETS),
    registered!(data_rights::ERASE_STUDY_RULE),
    registered!(data_rights::ERASE_OUTBOX),
    registered!(data_rights::ERASE_APPLIED_OPS),
    registered!(data_rights::ERASE_UNDO_JOURNAL),
    registered!(data_rights::ERASE_SCHEDULE),
    registered!(data_rights::ERASE_DEVICE_SETTING),
    registered!(data_rights::VACUUM_FILE),
    registered!(data_rights::CHECKPOINT),
    registered!(data_rights::EXPORT_STORE_META),
    registered!(data_rights::EXPORT_STUDY_RULE),
    registered!(data_rights::EXPORT_NOTETYPES),
    registered!(data_rights::EXPORT_FIELDS),
    registered!(data_rights::EXPORT_TEMPLATES),
    registered!(data_rights::EXPORT_NOTES),
    registered!(data_rights::EXPORT_NOTE_TAGS),
    registered!(data_rights::EXPORT_NOTE_FIELD_HISTORY),
    registered!(data_rights::EXPORT_PRESETS),
    registered!(data_rights::EXPORT_DECKS),
    registered!(data_rights::EXPORT_CARDS),
    registered!(data_rights::EXPORT_REVIEW_LOG),
    registered!(data_rights::EXPORT_REVIEW_RETRACTION),
    registered!(data_rights::EXPORT_REVIEW_CLIENT),
    registered!(data_rights::EXPORT_OUTBOX),
    registered!(data_rights::EXPORT_APPLIED_OPS),
    registered!(data_rights::EXPORT_UNDO_JOURNAL),
    registered!(data_rights::EXPORT_SCHEDULE),
    registered!(data_rights::EXPORT_DEVICE_SETTING),
];
