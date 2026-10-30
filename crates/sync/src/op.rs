//! The operations, wire version 1: the one vocabulary every device and the server write in
//! (SPEC-509 R2; ADR-527 D1).
//!
//! What the vocabulary leaves out is part of it. No variant updates or deletes a review row, none
//! changes a reviewed note's notetype, none deletes a preset or resets a card, and none replaces
//! a whole collection: the never-list (`docs/rulings/OWNER-RULING-2026-10-01-deck-writes.md`).
//! The census `tests/op_census.rs` holds it over every variant. No variant moves a due date
//! either: a due change no answer made is not an operation. Tags are not here; a tag edit travels
//! as a sealed event.

use crate::key::{Key, Origin};

/// An operation's head: who made it, its place in the device's sequence, and its clock. Two
/// devices that make the same operation write equal bodies and different heads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OpHead {
    /// The device that made the operation.
    pub origin: Origin,
    /// The device's sequence number, from 1 and gapless over every operation judged.
    pub seq: u64,
    /// The operation's hybrid clock, in milliseconds.
    pub hlc: i64,
}

/// The part of a notetype an edit sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotetypePart {
    /// The card template at this index.
    Template(u16),
    /// The notetype's stylesheet.
    Css,
    /// The name of the field at this index.
    FieldName(u16),
}

/// One operation. Reviews are append-only facts; everything else is an edit of a register whose
/// concurrent losers are kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// An answer to a card: a new review row.
    ReviewAnswered {
        /// The card answered.
        card: i64,
        /// The card's note.
        note: i64,
        /// The card's home deck.
        home_deck: i64,
        /// The deck's preset.
        preset: i64,
        /// The ease pressed, 1 to 4.
        ease: u8,
        /// The review kind, 0 to 3.
        kind: u8,
        /// How long the answer took, at most an hour.
        duration_ms: u32,
        /// The study day the answer counts on, by the producer's rule.
        study_day: i32,
        /// The key of the card's last review the device had folded.
        basis: Option<Key>,
        /// The study day the scheduler set the card due; none where it was absent.
        due_hint: Option<i32>,
        /// The scheduler's model version that set it.
        model_version: Option<[u8; 32]>,
    },
    /// An undo of the device's own last answer to a card: the row stays, and the fold voids it.
    ReviewUndone {
        /// The key of the review undone.
        target: Key,
    },
    /// A new note with its fields.
    NoteCreated {
        /// The note's id.
        note: i64,
        /// Its notetype.
        notetype: i64,
        /// The deck its cards go to.
        deck: i64,
        /// Its global id.
        guid: [u8; 16],
        /// Its field values, in the notetype's order.
        fields: Vec<String>,
    },
    /// A note field set to a value, made on the field's edit `base_rev`.
    NoteFieldSet {
        /// The note.
        note: i64,
        /// The field's index.
        field: u16,
        /// The new value.
        value: String,
        /// The key of the field's edit the device had applied.
        base_rev: Option<Key>,
    },
    /// A note deleted; refused for a reviewed note.
    NoteDeleted {
        /// The note.
        note: i64,
    },
    /// A card moved to another deck, made on the card's deck edit `base_rev`.
    CardMoved {
        /// The card.
        card: i64,
        /// The deck it moves to.
        deck: i64,
        /// The key of the card's deck edit the device had applied.
        base_rev: Option<Key>,
    },
    /// A card's flag set.
    CardFlag {
        /// The card.
        card: i64,
        /// The flag.
        flag: u8,
    },
    /// A card buried until the next study day.
    CardBury {
        /// The card.
        card: i64,
        /// The study day it was buried on.
        day: i32,
    },
    /// A card suspended or restored.
    CardSuspend {
        /// The card.
        card: i64,
        /// Suspended when true.
        on: bool,
    },
    /// A new deck.
    DeckCreated {
        /// The deck's id.
        deck: i64,
        /// Its parent deck.
        parent: Option<i64>,
        /// Its name.
        name: String,
    },
    /// A deck renamed, made on the deck's name edit `base_rev`.
    DeckRenamed {
        /// The deck.
        deck: i64,
        /// The new name.
        name: String,
        /// The key of the deck's name edit the device had applied.
        base_rev: Option<Key>,
    },
    /// A part of a notetype set, made on that part's edit `base_rev`.
    NotetypeEdited {
        /// The notetype.
        notetype: i64,
        /// The part set.
        part: NotetypePart,
        /// The new text.
        value: String,
        /// The key of the part's edit the device had applied.
        base_rev: Option<Key>,
    },
    /// A media name bound to a blob by its content hash.
    MediaBound {
        /// The media name a card refers to.
        name: String,
        /// The blob's SHA-256.
        sha256: [u8; 32],
        /// The blob's size in bytes.
        size: u32,
    },
    /// A media name unbound.
    MediaUnbound {
        /// The media name.
        name: String,
    },
    /// A review's ease edited.
    ReviewEdited {
        /// The review.
        target: Key,
        /// Its new ease.
        ease: u8,
    },
}

/// The kind of an operation: one per variant of [`Op`], with its wire tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OpKind {
    /// [`Op::ReviewAnswered`].
    ReviewAnswered,
    /// [`Op::ReviewUndone`].
    ReviewUndone,
    /// [`Op::NoteCreated`].
    NoteCreated,
    /// [`Op::NoteFieldSet`].
    NoteFieldSet,
    /// [`Op::NoteDeleted`].
    NoteDeleted,
    /// [`Op::CardMoved`].
    CardMoved,
    /// [`Op::CardFlag`].
    CardFlag,
    /// [`Op::CardBury`].
    CardBury,
    /// [`Op::CardSuspend`].
    CardSuspend,
    /// [`Op::DeckCreated`].
    DeckCreated,
    /// [`Op::DeckRenamed`].
    DeckRenamed,
    /// [`Op::NotetypeEdited`].
    NotetypeEdited,
    /// [`Op::MediaBound`].
    MediaBound,
    /// [`Op::MediaUnbound`].
    MediaUnbound,
}

impl OpKind {
    /// Every kind, in wire-tag order.
    pub const ALL: [Self; 14] = [
        Self::ReviewAnswered,
        Self::ReviewUndone,
        Self::NoteCreated,
        Self::NoteFieldSet,
        Self::NoteDeleted,
        Self::CardMoved,
        Self::CardFlag,
        Self::CardBury,
        Self::CardSuspend,
        Self::DeckCreated,
        Self::DeckRenamed,
        Self::NotetypeEdited,
        Self::MediaBound,
        Self::MediaUnbound,
    ];

    /// The kind's wire tag, the body's first byte.
    #[must_use]
    pub const fn tag(self) -> u8 {
        match self {
            Self::ReviewAnswered => 1,
            Self::ReviewUndone => 2,
            Self::NoteCreated => 3,
            Self::NoteFieldSet => 4,
            Self::NoteDeleted => 5,
            Self::CardMoved => 6,
            Self::CardFlag => 7,
            Self::CardBury => 8,
            Self::CardSuspend => 9,
            Self::DeckCreated => 10,
            Self::DeckRenamed => 11,
            Self::NotetypeEdited => 12,
            Self::MediaBound => 13,
            Self::MediaUnbound => 14,
        }
    }

    /// The kind whose wire tag is `tag`, or `None` for a tag no kind has.
    #[must_use]
    pub fn from_tag(tag: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.tag() == tag)
    }
}

impl Op {
    /// The operation's kind.
    #[must_use]
    pub const fn kind(&self) -> OpKind {
        match self {
            Self::ReviewAnswered { .. } => OpKind::ReviewAnswered,
            Self::ReviewUndone { .. } => OpKind::ReviewUndone,
            Self::NoteCreated { .. } => OpKind::NoteCreated,
            Self::NoteFieldSet { .. } => OpKind::NoteFieldSet,
            Self::NoteDeleted { .. } => OpKind::NoteDeleted,
            Self::CardMoved { .. } => OpKind::CardMoved,
            Self::CardFlag { .. } => OpKind::CardFlag,
            Self::CardBury { .. } => OpKind::CardBury,
            Self::CardSuspend { .. } => OpKind::CardSuspend,
            Self::DeckCreated { .. } => OpKind::DeckCreated,
            Self::DeckRenamed { .. } => OpKind::DeckRenamed,
            Self::NotetypeEdited { .. } => OpKind::NotetypeEdited,
            Self::MediaBound { .. } => OpKind::MediaBound,
            Self::MediaUnbound { .. } => OpKind::MediaUnbound,
            Self::ReviewEdited { .. } => OpKind::ReviewAnswered,
        }
    }
}
