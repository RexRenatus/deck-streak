//! The rows a search is evaluated over (SPEC-514 R5): a collection's decks, note types, notes,
//! cards and review log, as plain values the caller reads from wherever it keeps them. Nothing
//! here opens a file.

/// A deck: its id and its name, its levels separated by `::`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deck {
    /// The deck's id.
    pub id: i64,
    /// The deck's name, as the user writes it (`Parent::Child`).
    pub name: String,
}

/// A note type: its name, its fields' names, its templates' names, and its sort field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notetype {
    /// The note type's id.
    pub id: i64,
    /// The note type's name.
    pub name: String,
    /// Its fields' names, by ordinal.
    pub fields: Vec<String>,
    /// Its templates' names, by ordinal.
    pub templates: Vec<String>,
    /// The ordinal of the field the note's sort field is taken from.
    pub sort_field: usize,
}

/// A note: its note type, its fields' texts and its tags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The note's id.
    pub id: i64,
    /// The index of its note type in [`Collection::notetypes`].
    pub notetype: usize,
    /// Its fields' texts, by ordinal, as stored (HTML included).
    pub fields: Vec<String>,
    /// Its tags.
    pub tags: Vec<String>,
}

/// A card's row, with the engine's numbers for its type and queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    /// The card's id: the instant it was added, in milliseconds.
    pub id: i64,
    /// The index of its note in [`Collection::notes`].
    pub note: usize,
    /// Its deck's id.
    pub deck_id: i64,
    /// The deck a filtered deck borrowed it from, or 0.
    pub original_deck_id: i64,
    /// Its template's ordinal.
    pub ord: u16,
    /// The engine's card type: 0 new, 1 learning, 2 review, 3 relearning.
    pub kind: i64,
    /// The engine's queue: -3 and -2 buried, -1 suspended, 0 new, 1 learning, 2 review, 3 day
    /// learning, 4 preview.
    pub queue: i64,
    /// Its due: a day number in the review and day-learning queues, an instant in seconds in the
    /// learning queues.
    pub due: i64,
    /// Its due before a filtered deck borrowed it, or 0.
    pub original_due: i64,
    /// Its interval in days.
    pub interval: i64,
    /// Its review count.
    pub reps: i64,
    /// Its lapse count.
    pub lapses: i64,
}

/// One review log row: its id (the instant of the answer, in milliseconds), its card and the
/// button pressed (0 for a manual reschedule).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    /// The row's id, the answer's instant in milliseconds.
    pub id: i64,
    /// The index of its card in [`Collection::cards`].
    pub card: usize,
    /// The button pressed, 1 to 4, or 0.
    pub ease: i64,
}

/// A collection's rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collection {
    /// Every deck.
    pub decks: Vec<Deck>,
    /// Every note type.
    pub notetypes: Vec<Notetype>,
    /// Every note.
    pub notes: Vec<Note>,
    /// Every card.
    pub cards: Vec<Card>,
    /// Every review log row.
    pub revlog: Vec<Review>,
}
