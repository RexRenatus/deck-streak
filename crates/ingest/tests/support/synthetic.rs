//! ADR-022's seeded synthetic collection, built by bulk inserts into a collection the engine
//! created.
//!
//! | parameter | value |
//! |---|---|
//! | cards | 250,000, one per note, two text fields of 200 characters each |
//! | decks | 20, under two roots (one law, one language) |
//! | reviews | 200,000 study reviews spread over the last 400 days |
//! | new cards per day | 20 per deck in the deck configuration |
//!
//! The engine creates the collection, its decks and its stock note types; the notes, cards and
//! reviews go in by prepared statements over the engine's own connection (its documented escape
//! hatch, `SqliteStorage::db`), so the build takes seconds rather than the minutes 250,000 calls
//! through the engine's note API would. Every text is drawn from a fixed seed; every instant is
//! placed relative to the engine's clock, because the engine's scheduler reads that clock.

use std::path::Path;

use anki::collection::{Collection, CollectionBuilder};
use anki::deckconfig::DeckConfigId;
use anki::timestamp::{TimestampMillis, TimestampSecs};

/// Cards in the collection, one per note (ADR-022).
pub const CARDS: usize = 250_000;
/// Decks that hold the cards, under two roots (ADR-022).
pub const DECKS: usize = 20;
/// Study reviews in the review log (ADR-022).
pub const REVIEWS: usize = 200_000;
/// The days the reviews are spread over, ending now (ADR-022).
pub const REVIEW_DAYS: i64 = 400;
/// New cards per deck per day in the deck configuration (ADR-022).
pub const NEW_PER_DAY: u32 = 20;
/// Characters in each of a note's two text fields (ADR-022).
pub const FIELD_CHARS: usize = 200;
/// The two roots and the word each names its decks with.
pub const ROOTS: [(&str, &str); 2] = [("Law", "Subject"), ("Language", "Unit")];

/// The seed every text and every spread is drawn from.
const SEED: u64 = 0x5EED_0022;
const DAY_SECS: i64 = 86_400;
const DAY_MS: i64 = DAY_SECS * 1000;
/// Two cards in five have been studied, each reviewed twice: 100,000 cards and 200,000 reviews.
const STUDIED_OF_FIVE: usize = 2;
/// One studied card in this many is due today, which keeps each root's due reviews under the
/// default review limit, so the new-card limit is what shapes today's new queue.
const DUE_TODAY_ONE_IN: u64 = 500;
const SYLLABLES: [&str; 16] = [
    "ka", "lo", "mi", "ne", "ru", "sa", "te", "vi", "zo", "pa", "de", "fu", "gi", "ho", "ja", "be",
];

/// How many cards and reviews a collection holds; its decks, texts and deck configuration are
/// ADR-022's whatever its shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    /// Cards, one per note.
    pub cards: usize,
    /// Study reviews, two per studied card.
    pub reviews: usize,
}

impl Shape {
    /// ADR-022's collection: what the budget tests measure.
    pub const ADR_022: Self = Self {
        cards: CARDS,
        reviews: REVIEWS,
    };
    /// A small collection with the same decks, for the tests that sync rather than measure.
    pub const SMALL: Self = Self {
        cards: 1_000,
        reviews: 800,
    };
}

/// Which side of a sync the collection is built for: the engine marks a server's collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// A client's copy.
    Client,
    /// The collection a sync server serves.
    Server,
}

/// What a collection holds, counted by the engine's own connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Rows of `cards`.
    pub cards: usize,
    /// Rows of `notes`.
    pub notes: usize,
    /// Rows of `revlog` of a study type (0 to 3) with an answer (ease 1 or more).
    pub reviews: usize,
    /// Decks below a root.
    pub leaf_decks: usize,
}

/// `SplitMix64`: a small, seeded generator, so the collection is the same on every machine.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    /// Exactly [`FIELD_CHARS`] characters of seeded words.
    fn field(&mut self, text: &mut String) {
        text.clear();
        while text.len() < FIELD_CHARS {
            if !text.is_empty() {
                text.push(' ');
            }
            for _ in 0..=self.below(3) {
                text.push_str(SYLLABLES[usize::try_from(self.below(16)).unwrap()]);
            }
        }
        text.truncate(FIELD_CHARS);
    }
}

/// A 32-bit FNV-1a hash of the sort field, standing in for the engine's checksum, which is
/// private to it. Nothing the spike measures reads the checksum; it gives the checksum index a
/// realistic spread.
fn checksum(text: &str) -> i64 {
    let mut hash: u32 = 0x811C_9DC5;
    for byte in text.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    i64::from(hash)
}

/// Where the bulk inserts put the cards, as the engine created the collection.
struct Layout {
    /// The stock Basic note type's id.
    basic: i64,
    /// The decks that hold the cards, below the two roots.
    leaves: Vec<i64>,
    /// Today, as the engine's scheduler counts days from the creation stamp.
    today: i64,
    /// Now, in epoch seconds.
    now: i64,
    /// The first note's id: a millisecond stamp from before the first review.
    first_id: i64,
}

/// Builds ADR-022's collection at `path` for `side`, and returns what it holds.
///
/// # Panics
///
/// When the engine cannot create the collection or a statement fails: a fixture that cannot be
/// built must stop the test that needs it.
pub fn build(path: &Path, side: Side) -> Counts {
    build_shaped(path, side, Shape::ADR_022)
}

/// Builds a collection of `shape` at `path` for `side`, and returns what it holds.
///
/// # Panics
///
/// As [`build`].
pub fn build_shaped(path: &Path, side: Side, shape: Shape) -> Counts {
    let mut col = CollectionBuilder::new(path)
        .set_server(side == Side::Server)
        .build()
        .expect("the engine creates the collection");
    let now_ms = TimestampMillis::now().0;
    let now = TimestampSecs::now().0;
    // A collection created a day before its first review, like one studied for 400 days. The
    // engine counts its days from this stamp, so it is set before the engine first asks.
    let created = now - (REVIEW_DAYS + 1) * DAY_SECS;
    col.storage
        .db()
        .execute("update col set crt = ?", (created,))
        .expect("the creation stamp is set");
    let today = i64::from(col.timing_today().expect("the engine's day").days_elapsed);
    let mut leaves = Vec::with_capacity(DECKS);
    for (root, word) in ROOTS {
        for number in 1..=DECKS / ROOTS.len() {
            let deck = col
                .get_or_create_normal_deck(&format!("{root}::{word} {number:02}"))
                .expect("the engine creates the deck");
            leaves.push(deck.id.0);
        }
    }
    let config = col
        .get_deck_config(DeckConfigId(1), false)
        .expect("the deck configuration is read")
        .expect("the engine creates a default deck configuration");
    assert_eq!(
        config.inner.new_per_day, NEW_PER_DAY,
        "ADR-022: every deck's configuration queues {NEW_PER_DAY} new cards a day"
    );
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type")
        .id
        .0;
    let layout = Layout {
        basic,
        leaves,
        today,
        now,
        first_id: now_ms - (REVIEW_DAYS + 1) * DAY_MS,
    };

    let mut draw = Draw(SEED);
    let db = col.storage.db();
    db.execute_batch("begin").expect("a transaction opens");
    let studied = insert_notes_and_cards(&col, &layout, shape.cards, &mut draw);
    insert_reviews(&col, &studied, now_ms, shape.reviews, &mut draw);
    db.execute_batch("commit").expect("the transaction commits");
    db.execute("update col set mod = ?", (now_ms,))
        .expect("the modification stamp is set");
    let counts = counts_in(&col);
    col.close(None).expect("the engine closes the collection");
    counts
}

/// Inserts `count` notes, each with one card, and returns the studied cards' ids.
fn insert_notes_and_cards(
    col: &Collection,
    layout: &Layout,
    count: usize,
    draw: &mut Draw,
) -> Vec<i64> {
    let db = col.storage.db();
    let mut notes = db
        .prepare(
            "insert into notes (id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data) \
             values (?, ?, ?, ?, 0, '', ?, ?, ?, 0, '')",
        )
        .expect("the note statement prepares");
    let mut cards = db
        .prepare(
            "insert into cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, \
             reps, lapses, left, odue, odid, flags, data) \
             values (?, ?, ?, 0, ?, 0, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, '{}')",
        )
        .expect("the card statement prepares");
    let (mut front, mut back) = (String::new(), String::new());
    let mut studied = Vec::with_capacity(count / 5 * STUDIED_OF_FIVE);
    for index in 0..count {
        let id = layout.first_id + i64::try_from(index).unwrap();
        draw.field(&mut front);
        draw.field(&mut back);
        notes
            .execute((
                id,
                format!("s{index:09}"),
                layout.basic,
                layout.now,
                format!("{front}\u{1f}{back}"),
                front.as_str(),
                checksum(&front),
            ))
            .expect("a note is inserted");
        let deck = layout.leaves[index % DECKS];
        if index % 5 < STUDIED_OF_FIVE {
            // A review card: due today one time in DUE_TODAY_ONE_IN, else within its interval.
            let interval = 1 + draw.below(180);
            let due = if draw.below(DUE_TODAY_ONE_IN) == 0 {
                layout.today
            } else {
                layout.today + 1 + i64::try_from(draw.below(interval)).unwrap()
            };
            cards
                .execute((id, id, deck, layout.now, 2, 2, due, interval, 2500, 2))
                .expect("a review card is inserted");
            studied.push(id);
        } else {
            // A new card, in the order it was added.
            let position = i64::try_from(index).unwrap() + 1;
            cards
                .execute((id, id, deck, layout.now, 0, 0, position, 0, 0, 0))
                .expect("a new card is inserted");
        }
    }
    studied
}

/// Inserts `count` study reviews of the `studied` cards over the [`REVIEW_DAYS`] ending at
/// `now_ms`. Each review gets its own slot of the span, so every review id (a millisecond stamp)
/// is unique; a studied card's first answer (learning) falls in the span's first half and its
/// second (review) in the second half.
fn insert_reviews(col: &Collection, studied: &[i64], now_ms: i64, count: usize, draw: &mut Draw) {
    let mut revlog = col
        .storage
        .db()
        .prepare(
            "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
             values (?, ?, 0, ?, ?, ?, 2500, ?, ?)",
        )
        .expect("the review statement prepares");
    let span = REVIEW_DAYS * DAY_MS;
    let slot = span / i64::try_from(count).unwrap();
    let start = now_ms - span;
    for review in 0..count {
        let card = studied[review % studied.len()];
        let at = start
            + i64::try_from(review).unwrap() * slot
            + i64::try_from(draw.below(u64::try_from(slot).unwrap())).unwrap();
        let taken = 2000 + draw.below(18_000);
        if review < studied.len() {
            revlog
                .execute((at, card, 3, -600, 0, taken, 0))
                .expect("a learning review is inserted");
        } else {
            let ease = 1 + draw.below(4);
            revlog
                .execute((at, card, ease, 1 + draw.below(180), 1, taken, 1))
                .expect("a review is inserted");
        }
    }
}

fn counts_in(col: &Collection) -> Counts {
    let db = col.storage.db();
    let count = |sql: &str| -> usize {
        let rows: i64 = db
            .query_row(sql, (), |row| row.get(0))
            .expect("a count is read");
        usize::try_from(rows).unwrap()
    };
    Counts {
        cards: count("select count() from cards"),
        notes: count("select count() from notes"),
        reviews: count("select count() from revlog where type between 0 and 3 and ease >= 1"),
        leaf_decks: count("select count() from decks where instr(name, char(31)) > 0"),
    }
}

/// What the collection at `path` holds, opened by the engine.
///
/// # Panics
///
/// When the engine cannot open the collection.
pub fn counts(path: &Path) -> Counts {
    let col = CollectionBuilder::new(path)
        .build()
        .expect("the engine opens the collection");
    let counts = counts_in(&col);
    col.close(None).expect("the engine closes the collection");
    counts
}

/// The collection's modified stamp (`col.mod`), which a sync that changed nothing leaves alone.
///
/// # Panics
///
/// When the engine cannot open the collection.
pub fn modified(path: &Path) -> i64 {
    let col = CollectionBuilder::new(path)
        .build()
        .expect("the engine opens the collection");
    let stamp = col
        .storage
        .db()
        .query_row("select mod from col", (), |row| row.get(0))
        .expect("the modified stamp is read");
    col.close(None).expect("the engine closes the collection");
    stamp
}

// ------------------------------------------------------------------------------------------------
// SPEC-023: small planned collections with filtered decks, for the read, the window and the gate.
//
// The engine creates each collection, so its `decks` table is the engine's own, with its name
// column collated `unicase` and indexed; its normal decks come from the engine's deck API, and its
// filtered deck from the engine's own filtered-deck build, so a borrowed card's `odid` is the one the
// engine writes. Notes, cards and reviews go in by prepared statements, as ADR-022's do.

/// The filtered deck of a planned collection, by its human name: a top-level deck.
pub const FILTERED_DECK: &str = "Cram";

/// A card of a planned collection: its id, its home deck's human name (`Law::Evidence`), and whether
/// the filtered deck borrows it.
#[derive(Clone, Copy, Debug)]
pub struct PlannedCard {
    /// The card's id, which is also its note's.
    pub id: i64,
    /// The card's home deck, by its human name.
    pub deck: &'static str,
    /// Whether the filtered deck borrows the card.
    pub filtered: bool,
}

/// A review of a planned card: its id (the epoch-millisecond instant), the card, its type and ease.
#[derive(Clone, Copy, Debug)]
pub struct PlannedReview {
    /// The review's id.
    pub id: i64,
    /// The card answered.
    pub card: i64,
    /// The revlog type: 0 learn, 1 review, 2 relearn, 3 filtered, 4 manual, 5 rescheduled.
    pub kind: i64,
    /// The answer button, or 0 for a manual or rescheduling entry.
    pub ease: i64,
}

/// A planned collection's decks, by the engine's ids.
#[derive(Clone, Debug)]
pub struct Planned {
    /// Every deck's id by its human name, the engine's default deck and the filtered deck included.
    pub decks: std::collections::BTreeMap<String, i64>,
    /// The filtered deck's id, when a card is borrowed.
    pub filtered: Option<i64>,
}

impl Planned {
    /// The id of the deck named `human`.
    ///
    /// # Panics
    ///
    /// When the collection has no such deck.
    #[must_use]
    pub fn deck(&self, human: &str) -> i64 {
        *self
            .decks
            .get(human)
            .unwrap_or_else(|| panic!("the planned collection has no deck {human}"))
    }
}

/// Opens the collection at `path` with the engine, runs `work` on it, and closes it.
fn with_engine<T>(path: &Path, work: impl FnOnce(&mut Collection) -> T) -> T {
    let mut col = CollectionBuilder::new(path)
        .build()
        .expect("the engine opens the collection");
    let value = work(&mut col);
    col.close(None).expect("the engine closes the collection");
    value
}

/// Builds a planned collection at `path`: `decks` (human names, parents created with them), every
/// card of `cards` as a new card on its own Basic note in its home deck, every review of `reviews`,
/// and then the engine's filtered deck [`FILTERED_DECK`] borrowing each card planned `filtered`.
///
/// # Panics
///
/// When the engine or a statement fails: a fixture that cannot be built stops the test.
pub fn build_planned(
    path: &Path,
    decks: &[&str],
    cards: &[PlannedCard],
    reviews: &[PlannedReview],
) -> Planned {
    with_engine(path, |col| {
        for human in decks.iter().chain(cards.iter().map(|card| &card.deck)) {
            col.get_or_create_normal_deck(human)
                .expect("the engine creates the deck");
        }
        let basic = col
            .get_notetype_by_name("Basic")
            .expect("the note types are read")
            .expect("the engine creates its stock Basic note type")
            .id
            .0;
        let ids: std::collections::BTreeMap<String, i64> = col
            .get_all_deck_names(false)
            .expect("the deck names are read")
            .into_iter()
            .map(|(id, human)| (human, id.0))
            .collect();
        let db = col.storage.db();
        db.execute_batch("begin").expect("a transaction opens");
        for (position, card) in cards.iter().enumerate() {
            let position = i64::try_from(position).unwrap() + 1;
            insert_planned_note(
                col,
                basic,
                card.id,
                ids[card.deck],
                CardColumns::new(position),
            );
        }
        insert_planned_reviews(col, reviews);
        db.execute_batch("commit").expect("the transaction commits");
        let borrowed: Vec<i64> = cards
            .iter()
            .filter(|card| card.filtered)
            .map(|card| card.id)
            .collect();
        let filtered = build_filtered(col, &borrowed);
        let mut decks = ids;
        if let Some(id) = filtered {
            decks.insert(FILTERED_DECK.to_owned(), id);
        }
        Planned { decks, filtered }
    })
}

/// The scheduling columns of a planned card as the `cards` table stores them.
#[derive(Clone, Copy, Debug)]
struct CardColumns {
    kind: i64,
    queue: i64,
    due: i64,
    interval: i64,
    factor: i64,
    lapses: i64,
    left: i64,
}

impl CardColumns {
    /// A new card at queue position `position`.
    const fn new(position: i64) -> Self {
        Self {
            kind: 0,
            queue: 0,
            due: position,
            interval: 0,
            factor: 0,
            lapses: 0,
            left: 0,
        }
    }
}

/// Inserts the card `id` on its own Basic note (`basic`), the note sharing its id, in the deck
/// `deck` with the scheduling `columns`, over the engine's own connection.
fn insert_planned_note(col: &Collection, basic: i64, id: i64, deck: i64, columns: CardColumns) {
    let db = col.storage.db();
    let text = format!("planned card {id}");
    db.execute(
        "insert into notes (id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data) \
         values (?, ?, ?, 0, 0, '', ?, ?, ?, 0, '')",
        (
            id,
            format!("planned{id}"),
            basic,
            format!("{text}\u{1f}back"),
            text.as_str(),
            checksum(&text),
        ),
    )
    .expect("a note is inserted");
    db.execute(
        "insert into cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, \
         reps, lapses, left, odue, odid, flags, data) \
         values (?, ?, ?, 0, 0, 0, ?, ?, ?, ?, ?, 0, ?, ?, 0, 0, 0, '{}')",
        (
            id,
            id,
            deck,
            columns.kind,
            columns.queue,
            columns.due,
            columns.interval,
            columns.factor,
            columns.lapses,
            columns.left,
        ),
    )
    .expect("a card is inserted");
}

/// Builds the engine's filtered deck [`FILTERED_DECK`] borrowing the cards `borrowed`, and answers
/// its id; no deck when nothing is borrowed.
fn build_filtered(col: &mut Collection, borrowed: &[i64]) -> Option<i64> {
    let ids: Vec<String> = borrowed.iter().map(ToString::to_string).collect();
    (!ids.is_empty()).then(|| {
        let mut deck = col
            .get_or_create_filtered_deck(anki::decks::DeckId(0))
            .expect("a new filtered deck");
        FILTERED_DECK.clone_into(&mut deck.human_name);
        deck.config.search_terms.truncate(1);
        deck.config.search_terms[0].search = format!("cid:{}", ids.join(","));
        deck.config.search_terms[0].limit = 1000;
        col.add_or_update_filtered_deck(deck)
            .expect("the engine builds the filtered deck")
            .output
            .0
    })
}

fn insert_planned_reviews(col: &Collection, reviews: &[PlannedReview]) {
    let mut revlog = col
        .storage
        .db()
        .prepare(
            "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
             values (?, ?, 0, ?, 1, 0, 2500, 4000, ?)",
        )
        .expect("the review statement prepares");
    for review in reviews {
        revlog
            .execute((review.id, review.card, review.ease, review.kind))
            .expect("a review is inserted");
    }
}

/// Adds `reviews` to the collection at `path`, as another client's sync would.
///
/// # Panics
///
/// When the engine or a statement fails.
pub fn add_reviews(path: &Path, reviews: &[PlannedReview]) {
    with_engine(path, |col| insert_planned_reviews(col, reviews));
}

/// Moves the card `card` to be due at `due`, with no review: the in-place change another device, an
/// import or a reschedule makes.
///
/// # Panics
///
/// When the engine or the statement fails.
pub fn move_due(path: &Path, card: i64, due: i64) {
    with_engine(path, |col| {
        let changed = col
            .storage
            .db()
            .execute("update cards set due = ? where id = ?", (due, card))
            .expect("the card is updated");
        assert_eq!(changed, 1, "the collection holds the card {card}");
    });
}

/// Sets the tags of the note `note` in the collection at `path`, as tagging it on another device
/// does.
///
/// # Panics
///
/// When the engine or the statement fails.
pub fn set_tags(path: &Path, note: i64, tags: &str) {
    with_engine(path, |col| {
        col.storage
            .db()
            .execute("update notes set tags = ? where id = ?", (tags, note))
            .expect("the note's tags are set");
    });
}

/// Deletes the cards `ids` from the collection at `path`, leaving their reviews behind, as deleting a
/// card on another device does before its log is pruned.
///
/// # Panics
///
/// When the engine or the statement fails.
pub fn delete_cards(path: &Path, ids: &[i64]) {
    with_engine(path, |col| {
        for id in ids {
            col.storage
                .db()
                .execute("delete from cards where id = ?", (id,))
                .expect("the card is deleted");
        }
    });
}

/// Replaces every card and review of the collection at `path` (created when missing) with `cards`,
/// each row the probed columns `due`, `ivl`, `queue`, `factor`, `lapses`, `type` and `odid` with ids
/// from 1, and the reviews `review_ids`: a case of the change probe's golden.
///
/// # Panics
///
/// When the engine or a statement fails.
pub fn write_probe_rows(path: &Path, cards: &[[i64; 7]], review_ids: &[i64]) {
    with_engine(path, |col| {
        let db = col.storage.db();
        db.execute_batch("begin; delete from cards; delete from revlog;")
            .expect("the rows are cleared");
        for (index, row) in cards.iter().enumerate() {
            let id = i64::try_from(index).unwrap() + 1;
            let [due, ivl, queue, factor, lapses, kind, odid] = *row;
            db.execute(
                "insert into cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, \
                 reps, lapses, left, odue, odid, flags, data) \
                 values (?, ?, 1, 0, 0, 0, ?, ?, ?, ?, ?, 0, ?, 0, 0, ?, 0, '{}')",
                (id, id, kind, queue, due, ivl, factor, lapses, odid),
            )
            .expect("a card row is inserted");
        }
        for id in review_ids {
            db.execute(
                "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
                 values (?, 0, 0, 0, 0, 0, 0, 0, 0)",
                (id,),
            )
            .expect("a review id is inserted");
        }
        db.execute_batch("commit").expect("the rows commit");
    });
}

/// The `CREATE TABLE` statement of the collection's `decks` table, as the engine wrote it.
///
/// # Panics
///
/// When the engine cannot open the collection.
#[must_use]
pub fn decks_table_sql(path: &Path) -> String {
    with_engine(path, |col| {
        col.storage
            .db()
            .query_row(
                "select sql from sqlite_master where type = 'table' and name = 'decks'",
                (),
                |row| row.get(0),
            )
            .expect("the decks table's statement is read")
    })
}

/// Every deck's id and stored name (its parts separated by `\x1f`), read through the engine's own
/// connection, which registers the `unicase` collation.
///
/// # Panics
///
/// When the engine cannot open the collection.
#[must_use]
pub fn stored_deck_names(path: &Path) -> std::collections::BTreeMap<i64, String> {
    with_engine(path, |col| {
        let mut statement = col
            .storage
            .db()
            .prepare("select id, name from decks")
            .expect("the deck statement prepares");
        statement
            .query_map((), |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("the decks are read")
            .map(|row| row.expect("a deck row"))
            .collect()
    })
}

/// The collection's creation stamp, `col.crt`, in epoch seconds.
///
/// # Panics
///
/// When the engine cannot open the collection.
#[must_use]
pub fn creation_secs(path: &Path) -> i64 {
    with_engine(path, |col| {
        col.storage
            .db()
            .query_row("select crt from col", (), |row| row.get(0))
            .expect("the creation stamp is read")
    })
}

/// The scoped collection's floor: a read at it takes the reviews after it and none at or before it.
pub const SCOPED_FLOOR: i64 = 1_700_000_000_000;
/// The scoped collection's normal decks, by human name (the engine creates each root with them):
/// two roots in scope, a root out of it, and a subdeck named like a root under another root.
pub const SCOPED_DECKS: [&str; 5] = [
    "Law::Evidence",
    "Law::Torts",
    "Language::Unit 01",
    "Maths",
    "Other::Law",
];
/// The scoped collection's cards: a Law card; a Law card and a Maths card the filtered deck borrows;
/// a studied and an unstudied Language card; a Maths card; and a card of `Other::Law`.
pub const SCOPED_CARDS: [PlannedCard; 7] = [
    planned_card(1001, "Law::Evidence", false),
    planned_card(1002, "Law::Torts", true),
    planned_card(1003, "Language::Unit 01", false),
    planned_card(1004, "Language::Unit 01", false),
    planned_card(1005, "Maths", false),
    planned_card(1006, "Maths", true),
    planned_card(1007, "Other::Law", false),
];
/// The scoped collection's reviews: one before the floor and one at it; five study events of cards
/// in scope after it, one of each study type; a manual entry, a rescheduling entry and an
/// unanswered row of cards in scope; and three study events of cards out of scope.
pub const SCOPED_REVIEWS: [PlannedReview; 13] = [
    planned_review(-1, 1001, 1, 3),
    planned_review(0, 1003, 1, 3),
    planned_review(1, 1001, 0, 3),
    planned_review(2, 1001, 1, 2),
    planned_review(3, 1002, 2, 1),
    planned_review(4, 1002, 3, 4),
    planned_review(5, 1003, 0, 1),
    planned_review(6, 1001, 4, 0),
    planned_review(7, 1003, 5, 0),
    planned_review(8, 1002, 1, 0),
    planned_review(9, 1005, 1, 3),
    planned_review(10, 1006, 3, 3),
    planned_review(11, 1007, 1, 2),
];

const fn planned_card(id: i64, deck: &'static str, filtered: bool) -> PlannedCard {
    PlannedCard { id, deck, filtered }
}

/// A review `days` whole days after [`SCOPED_FLOOR`].
const fn planned_review(days: i64, card: i64, kind: i64, ease: i64) -> PlannedReview {
    PlannedReview {
        id: SCOPED_FLOOR + days * DAY_MS,
        card,
        kind,
        ease,
    }
}

/// The instant of the review `days` whole days after [`SCOPED_FLOOR`].
#[must_use]
pub const fn scoped_review_id(days: i64) -> i64 {
    SCOPED_FLOOR + days * DAY_MS
}

/// Builds the scoped collection at `path`.
///
/// # Panics
///
/// As [`build_planned`].
pub fn build_scoped(path: &Path) -> Planned {
    build_planned(path, &SCOPED_DECKS, &SCOPED_CARDS, &SCOPED_REVIEWS)
}

/// A collection reader over the copy `settings` name, inside the include list `include` (as the
/// setting is written) and the law root `law_root`, on an offload of one worker timed by `clock`.
///
/// # Panics
///
/// When the scope does not parse.
#[must_use]
pub fn reader(
    settings: &deck_streak_ingest::settings::SyncSettings,
    include: &str,
    law_root: Option<&str>,
    clock: std::sync::Arc<dyn deck_streak_kernel::Clock>,
) -> deck_streak_ingest::reader::CollectionReader {
    use deck_streak_ingest::settings::{INCLUDE_DECKS, LAW_DECK_ROOT, ScopeSettings};
    use deck_streak_kernel::{Environment, Offload, OffloadWorkers};
    let mut variables = vec![(INCLUDE_DECKS, include)];
    if let Some(root) = law_root {
        variables.push((LAW_DECK_ROOT, root));
    }
    let scope =
        ScopeSettings::from_env(&Environment::from_vars(variables)).expect("the scope parses");
    let offload = Offload::new(OffloadWorkers::new(1).expect("one worker"), clock);
    deck_streak_ingest::reader::CollectionReader::new(settings, scope, offload)
}

/// Runs `sql`, a batch of statements, through the engine's own connection on the collection at
/// `path`: the way a test shapes a collection the engine would not.
///
/// # Panics
///
/// When the engine cannot open the collection or the batch fails.
pub fn run_sql(path: &Path, sql: &str) {
    with_engine(path, |col| {
        col.storage
            .db()
            .execute_batch(sql)
            .expect("the statement batch runs");
    });
}

/// The id of the stock note type named `name`.
///
/// # Panics
///
/// When the engine has no such note type.
#[must_use]
pub fn notetype_id(path: &Path, name: &str) -> i64 {
    with_engine(path, |col| {
        col.get_notetype_by_name(name)
            .expect("the note types are read")
            .unwrap_or_else(|| panic!("the engine has no note type {name}"))
            .id
            .0
    })
}

// ------------------------------------------------------------------------------------------------
// SPEC-083: the skip's take-side collections. One top-level deck holds every kind of card the
// skip's search must move or leave (A38), the filtered deck borrowing one of them by its home deck;
// the collection's configured UTC offset, rollover hour and FSRS switch are each set by the test
// (A5, A40). Its creation stamp is moved back, so every day number here is positive.

/// The skip collection's one normal deck, a top-level deck holding every card of [`SKIP_CARDS`].
pub const SKIP_DECK: &str = "Skip";
/// The first id of a skip collection's cards: each card's id is this plus its place, from 1.
pub const SKIP_FLOOR: i64 = 1_700_100_000_000;
/// How many days before its build a skip collection was created.
const SKIP_AGE_DAYS: i64 = 30;
/// A review or relearning card's interval in a skip collection, in days.
const SKIP_INTERVAL: i64 = 10;
/// A review card's ease factor in a skip collection, in permille.
const SKIP_FACTOR: i64 = 2500;

/// A card of a skip collection, by what the skip's search must do with it (SPEC-083 R3, A38).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipCard {
    /// A review card due on the engine's day: the one kind the default search moves.
    DueReview,
    /// A review card due on the engine's day that the filtered deck borrows.
    FilteredReview,
    /// A new card.
    New,
    /// A learning card due now.
    Learning,
    /// A relearning card due now.
    Relearning,
    /// A suspended review card due on the engine's day.
    Suspended,
    /// A review card due on the engine's day that the owner buried.
    Buried,
    /// A review card the skip does not move, already due within the day spec's range.
    DueInRange,
    /// A review card due this many days from the engine's day (negative: overdue).
    OtherDay(i64),
}

impl SkipCard {
    /// The card's scheduling columns on the engine's day `today`, at the instant `now` in seconds.
    const fn columns(self, today: i64, now: i64, position: i64) -> CardColumns {
        let review = CardColumns {
            kind: 2,
            queue: 2,
            due: today,
            interval: SKIP_INTERVAL,
            factor: SKIP_FACTOR,
            lapses: 0,
            left: 0,
        };
        match self {
            Self::DueReview | Self::FilteredReview => review,
            Self::New => CardColumns::new(position),
            Self::Learning => CardColumns {
                kind: 1,
                queue: 1,
                due: now - 60,
                interval: 0,
                factor: 0,
                lapses: 0,
                left: 1,
            },
            Self::Relearning => CardColumns {
                kind: 3,
                queue: 1,
                due: now - 60,
                lapses: 1,
                left: 1,
                ..review
            },
            Self::Suspended => CardColumns {
                queue: -1,
                ..review
            },
            Self::Buried => CardColumns {
                queue: -2,
                ..review
            },
            Self::DueInRange => CardColumns {
                due: today + deck_streak_ingest::skip::SKIP_SPREAD_MAX_DAYS - 1,
                ..review
            },
            Self::OtherDay(days) => CardColumns {
                due: today + days,
                ..review
            },
        }
    }
}

/// The skip's take-side collection (SPEC-083 A5, A38): three review cards due today, which the
/// default search moves, and one card of every kind it must leave, each by its id.
pub const SKIP_CARDS: [(i64, SkipCard); 12] = [
    (SKIP_FLOOR + 1, SkipCard::DueReview),
    (SKIP_FLOOR + 2, SkipCard::DueReview),
    (SKIP_FLOOR + 3, SkipCard::DueReview),
    (SKIP_FLOOR + 4, SkipCard::FilteredReview),
    (SKIP_FLOOR + 5, SkipCard::New),
    (SKIP_FLOOR + 6, SkipCard::Learning),
    (SKIP_FLOOR + 7, SkipCard::Relearning),
    (SKIP_FLOOR + 8, SkipCard::Suspended),
    (SKIP_FLOOR + 9, SkipCard::Buried),
    (SKIP_FLOOR + 10, SkipCard::DueInRange),
    (SKIP_FLOOR + 11, SkipCard::OtherDay(-2)),
    (SKIP_FLOOR + 12, SkipCard::OtherDay(10)),
];

/// `count` review cards due today, ids from [`SKIP_FLOOR`] plus 1: A36's set one card over the
/// golden limit is `skip_due_reviews(SKIP_MAX_CARDS + 1)`.
#[must_use]
pub fn skip_due_reviews(count: usize) -> Vec<(i64, SkipCard)> {
    (1..=count)
        .map(|place| {
            (
                SKIP_FLOOR + i64::try_from(place).unwrap(),
                SkipCard::DueReview,
            )
        })
        .collect()
}

/// The ids of `cards` the default search moves: its review cards due today outside a filtered deck.
#[must_use]
pub fn skip_moved(cards: &[(i64, SkipCard)]) -> Vec<i64> {
    cards
        .iter()
        .filter(|(_, card)| *card == SkipCard::DueReview)
        .map(|(id, _)| *id)
        .collect()
}

/// The settings a skip collection is built with (SPEC-083 A5, A40).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkipSetup {
    /// The configured UTC offset in minutes WEST of UTC, as the engine stores it (UTC+05:30 is
    /// -330), or `None` for a collection that holds none.
    pub utc_offset_west: Option<i32>,
    /// The collection's rollover hour, 0 to 23.
    pub rollover: u8,
    /// Whether FSRS is on.
    pub fsrs: bool,
}

impl SkipSetup {
    /// The zone the skip's tests pin in their own process (`UTC0`, SPEC-083 section 3), the
    /// engine's default rollover hour and FSRS off.
    pub const UTC: Self = Self {
        utc_offset_west: Some(0),
        rollover: 4,
        fsrs: false,
    };
}

/// Builds a skip collection at `path`: [`SKIP_DECK`] holding `cards`, each note, card and column
/// inserted in one transaction, and the filtered deck borrowing each [`SkipCard::FilteredReview`];
/// then `setup`'s rollover hour and FSRS switch before the day is read, and its configured UTC
/// offset last, because the engine rewrites that offset to the process's zone whenever it reads
/// the day as a client.
///
/// # Panics
///
/// When the engine or a statement fails: a fixture that cannot be built stops the test.
pub fn build_skip(path: &Path, cards: &[(i64, SkipCard)], setup: SkipSetup) -> Planned {
    with_engine(path, |col| {
        col.storage
            .db()
            .execute("update col set crt = crt - ?", (SKIP_AGE_DAYS * DAY_SECS,))
            .expect("the collection's creation stamp moves back");
    });
    with_engine(path, |col| {
        let deck = col
            .get_or_create_normal_deck(SKIP_DECK)
            .expect("the engine creates the deck")
            .id
            .0;
        let basic = col
            .get_notetype_by_name("Basic")
            .expect("the note types are read")
            .expect("the engine creates its stock Basic note type")
            .id
            .0;
        col.set_config_json("rollover", &u32::from(setup.rollover), false)
            .expect("the rollover hour is set");
        col.set_config_bool(anki::config::BoolKey::Fsrs, setup.fsrs, false)
            .expect("FSRS is switched");
        let timing = col.timing_today().expect("the engine reads its day");
        let today = i64::from(timing.days_elapsed);
        // Today's unbury has run, so the buried card is one buried today: no sync unburies it.
        col.set_config_json("lastUnburied", &today, false)
            .expect("the last unburied day is today");
        let db = col.storage.db();
        db.execute_batch("begin").expect("a transaction opens");
        for (position, (id, card)) in cards.iter().enumerate() {
            let position = i64::try_from(position).unwrap() + 1;
            let columns = card.columns(today, timing.now.0, position);
            insert_planned_note(col, basic, *id, deck, columns);
        }
        db.execute_batch("commit").expect("the transaction commits");
        let borrowed: Vec<i64> = cards
            .iter()
            .filter(|(_, card)| *card == SkipCard::FilteredReview)
            .map(|(id, _)| *id)
            .collect();
        let filtered = build_filtered(col, &borrowed);
        match setup.utc_offset_west {
            Some(minutes) => col
                .set_config_json("localOffset", &minutes, false)
                .map(|_| ()),
            None => col.remove_config("localOffset").map(|_| ()),
        }
        .expect("the configured UTC offset is set");
        let mut decks: std::collections::BTreeMap<String, i64> = col
            .get_all_deck_names(false)
            .expect("the deck names are read")
            .into_iter()
            .map(|(id, human)| (human, id.0))
            .collect();
        if let Some(id) = filtered {
            decks.insert(FILTERED_DECK.to_owned(), id);
        }
        Planned { decks, filtered }
    })
}

/// One review-log row, as a skip's tests judge it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogRow {
    /// The card the row belongs to.
    pub card: i64,
    /// The row's type: 4 for the engine's manual reschedule.
    pub kind: i64,
    /// The answer: 0 for the engine's manual reschedule.
    pub ease: i64,
}

/// Every review-log row of the collection at `path`, oldest first, read by the engine's own
/// connection (SPEC-083 R18).
///
/// # Panics
///
/// When the engine cannot open the collection or the read fails.
pub fn review_log(path: &Path) -> Vec<LogRow> {
    with_engine(path, |col| {
        let mut statement = col
            .storage
            .db()
            .prepare("select cid, type, ease from revlog order by id")
            .expect("the review log's read is prepared");
        statement
            .query_map((), |row| {
                Ok(LogRow {
                    card: row.get(0)?,
                    kind: row.get(1)?,
                    ease: row.get(2)?,
                })
            })
            .expect("the review log is read")
            .map(|row| row.expect("a review-log row"))
            .collect()
    })
}

/// Plays another client changing cards' due dates in the collection at `path`: each `(card, due)`
/// is written with a new modification time and as unsynced, and the collection's modified stamp
/// moves, so that client's next sync sends it (SPEC-083 A39): a sync whose stamps are equal on
/// both sides exchanges nothing.
///
/// # Panics
///
/// When the collection does not hold a card.
pub fn change_cards(path: &Path, cards: &[(i64, i64)]) {
    with_engine(path, |col| {
        let now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is past the epoch")
                .as_secs(),
        )
        .expect("seconds fit an i64");
        for (card, due) in cards {
            let changed = col
                .storage
                .db()
                .execute(
                    "update cards set due = ?, mod = ?, usn = -1 where id = ?",
                    (due, now, card),
                )
                .expect("the card is updated");
            assert_eq!(changed, 1, "the collection holds the card {card}");
        }
        col.storage
            .db()
            .execute("update col set mod = ?", (now * 1000,))
            .expect("the collection's modified stamp moves");
    });
}

/// The tables whose rows carry an update sequence number in the engine's schema.
const USN_TABLES: [&str; 10] = [
    "cards",
    "notes",
    "revlog",
    "graves",
    "decks",
    "deck_config",
    "notetypes",
    "templates",
    "tags",
    "config",
];

/// Makes the collection at `path` a server's: every row the builder wrote as unsynced (update
/// sequence number -1, which the engine writes in client mode) is marked synced, as a server never
/// holds an unsynced row. A full download then carries no pending change into the private copy,
/// so the private copy's first change after it is the only one its next sync sends.
///
/// # Panics
///
/// When a statement fails.
pub fn as_served(path: &Path) {
    with_engine(path, |col| {
        for table in USN_TABLES {
            col.storage
                .db()
                .execute(&format!("update {table} set usn = 0 where usn = -1"), ())
                .unwrap_or_else(|error| panic!("{table}'s rows are marked synced: {error}"));
        }
    });
}

/// The engine's own default preset: every collection holds it, and the `Default` deck names it.
pub const PRESET_DEFAULT: i64 = 1;
/// The main preset (SPEC-387 section 3): an FSRS-6 fit beside an older FSRS-5 one, at a desired
/// retention other than the default, named by two decks.
pub const PRESET_MAIN: i64 = 1001;
/// A preset whose newest field is FSRS-5, beside an older FSRS-4 vector.
pub const PRESET_FIVE: i64 = 1002;
/// A preset with an FSRS-4 vector alone, which no deck names.
pub const PRESET_FOUR: i64 = 1003;
/// A preset already on the engine's defaults.
pub const PRESET_ON_DEFAULTS: i64 = 1004;
/// The main preset's desired retention.
pub const PRESET_MAIN_RETENTION: f32 = 0.85;
/// The main preset's FSRS-6 fit: synthetic values of five decimal places, so a value printed to
/// four places does not parse back to it.
pub const PRESET_MAIN_FSRS6: [f32; 21] = [
    0.402_55, 1.183_85, 3.173_05, 15.691_05, 7.197_35, 0.533_45, 1.460_45, 0.004_65, 1.545_75,
    0.119_25, 1.019_25, 1.939_55, 0.110_05, 0.296_05, 2.269_85, 0.231_55, 2.989_85, 0.516_55,
    0.662_15, 0.060_05, 0.200_05,
];
/// The main preset's older FSRS-5 vector, which the FSRS-6 one takes precedence over.
pub const PRESET_MAIN_FSRS5: [f32; 19] = [
    0.41, 1.18, 3.17, 15.69, 7.19, 0.53, 1.46, 0.0046, 1.54, 0.11, 1.01, 1.93, 0.11, 0.29, 2.26,
    0.23, 2.98, 0.51, 0.66,
];
/// The FSRS-5 preset's vector.
pub const PRESET_FIVE_FSRS5: [f32; 19] = [
    0.5, 1.3, 2.4, 9.1, 6.9, 0.7, 2.5, 0.002, 1.6, 0.15, 0.95, 1.8, 0.07, 0.3, 1.9, 0.5, 1.7, 0.6,
    0.1,
];
/// The FSRS-5 preset's older FSRS-4 vector, which the FSRS-5 one takes precedence over.
pub const PRESET_FIVE_FSRS4: [f32; 17] = [
    0.45, 1.2, 3.3, 10.5, 5.1, 1.2, 0.8, 0.01, 1.5, 0.1, 1.0, 2.1, 0.09, 0.3, 2.2, 0.2, 2.9,
];
/// The FSRS-4 preset's vector.
pub const PRESET_FOUR_FSRS4: [f32; 17] = [
    0.6, 1.4, 3.8, 11.2, 5.0, 1.1, 0.9, 0.03, 1.6, 0.12, 1.05, 2.2, 0.08, 0.32, 2.1, 0.22, 3.0,
];
/// The decks the fixture creates, each with the preset it names.
pub const PRESET_DECKS: [(&str, i64); 4] = [
    ("Main", PRESET_MAIN),
    ("Main::Sub", PRESET_MAIN),
    ("Five", PRESET_FIVE),
    ("Fresh", PRESET_ON_DEFAULTS),
];
/// The first card id of the preset fixture.
pub const PRESET_FLOOR: i64 = 1_700_300_000_000;
/// The preset fixture's cards, each with its home deck: one card of each class in the main
/// preset's decks (new, learning, review, relearning, and a review card a filtered deck borrows),
/// and cards outside them. The main preset counts five non-new cards and the FSRS-5 one counts one.
pub const PRESET_CARDS: [(i64, SkipCard, &str); 9] = [
    (PRESET_FLOOR + 1, SkipCard::New, "Main"),
    (PRESET_FLOOR + 2, SkipCard::Learning, "Main"),
    (PRESET_FLOOR + 3, SkipCard::DueReview, "Main"),
    (PRESET_FLOOR + 4, SkipCard::Relearning, "Main"),
    (PRESET_FLOOR + 5, SkipCard::DueReview, "Main::Sub"),
    (PRESET_FLOOR + 6, SkipCard::FilteredReview, "Main"),
    (PRESET_FLOOR + 7, SkipCard::DueReview, "Five"),
    (PRESET_FLOOR + 8, SkipCard::New, "Five"),
    (PRESET_FLOOR + 9, SkipCard::New, "Main::Sub"),
];

/// The settings a preset collection is built with (SPEC-387 R8, A7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresetSetup {
    /// The configured UTC offset in minutes WEST of UTC, as the engine stores it, or `None` for a
    /// collection that holds none.
    pub utc_offset_west: Option<i32>,
    /// The rollover hour, or `None` for a collection that holds none.
    pub rollover: Option<u8>,
}

impl PresetSetup {
    /// The process's own zone at the engine's default rollover hour: a copy the preset read
    /// accepts.
    #[must_use]
    pub fn process_zone() -> Self {
        Self {
            utc_offset_west: Some(process_offset_west()),
            rollover: Some(4),
        }
    }
}

/// The process's zone offset now, in minutes WEST of UTC as the engine stores it.
#[must_use]
pub fn process_offset_west() -> i32 {
    -(chrono::Local::now().offset().local_minus_utc() / 60)
}

/// What a preset collection was built with: its decks by name, its filtered deck, and the engine's
/// own defaults, read through the engine's deck-options read and never through the preset module.
pub struct PresetPlan {
    /// Every deck's id, by its human name.
    pub decks: std::collections::BTreeMap<String, i64>,
    /// The filtered deck's id.
    pub filtered: Option<i64>,
    /// The engine's defaults, as its deck-options read reports them.
    pub defaults: Vec<f32>,
}

/// Saves the preset `id` named `name` through the engine's legacy deck-config service, which keeps
/// each parameter field as given (the engine's own update clears the older fields when the newest
/// is empty).
fn add_preset(col: &mut Collection, id: i64, name: &str, retention: f32, fields: [&[f32]; 3]) {
    use anki::services::DeckConfigService;
    let widened = |values: &[f32]| {
        values
            .iter()
            .map(|value| f64::from(*value))
            .collect::<Vec<_>>()
    };
    let legacy = DeckConfigService::new_deck_config_legacy(col).expect("the engine's new preset");
    let mut preset: serde_json::Value =
        serde_json::from_slice(&legacy.json).expect("the engine's preset is JSON");
    preset["id"] = id.into();
    preset["name"] = name.into();
    preset["desiredRetention"] = f64::from(retention).into();
    preset["fsrsParams6"] = widened(fields[0]).into();
    preset["fsrsParams5"] = widened(fields[1]).into();
    preset["fsrsWeights"] = widened(fields[2]).into();
    let json = serde_json::to_vec(&preset).expect("the preset is written");
    let saved = DeckConfigService::add_or_update_deck_config_legacy(col, json.into())
        .expect("the engine saves the preset");
    assert_eq!(saved.dcid, id, "the engine keeps the preset's own id");
}

/// Builds a preset collection at `path` (SPEC-387 section 3): the presets and decks above, every
/// card of [`PRESET_CARDS`] on its own Basic note, and the filtered deck borrowing the
/// [`SkipCard::FilteredReview`] card; then `setup`'s rollover hour, and its configured UTC offset
/// last, because the engine rewrites that offset whenever it reads the day as a client.
///
/// # Panics
///
/// When the engine or a statement fails: a fixture that cannot be built stops the test.
#[allow(clippy::too_many_lines)]
pub fn build_presets(path: &Path, setup: PresetSetup) -> PresetPlan {
    with_engine(path, |col| {
        for (deck, _) in PRESET_DECKS {
            col.get_or_create_normal_deck(deck)
                .expect("the engine creates the deck");
        }
        col.set_config_json("rollover", &4_u32, false)
            .expect("the rollover hour is set");
        let timing = col.timing_today().expect("the engine reads its day");
        let today = i64::from(timing.days_elapsed);
        let ids: std::collections::BTreeMap<String, i64> = col
            .get_all_deck_names(false)
            .expect("the deck names are read")
            .into_iter()
            .map(|(id, human)| (human, id.0))
            .collect();
        let defaults = col
            .get_deck_configs_for_update(anki::decks::DeckId(ids["Main"]))
            .expect("the engine's deck-options read")
            .defaults
            .and_then(|defaults| defaults.config)
            .map(|config| config.fsrs_params_6)
            .expect("the engine reports its defaults");
        add_preset(
            col,
            PRESET_MAIN,
            "Main",
            PRESET_MAIN_RETENTION,
            [&PRESET_MAIN_FSRS6, &PRESET_MAIN_FSRS5, &[]],
        );
        add_preset(
            col,
            PRESET_FIVE,
            "Five",
            0.9,
            [&[], &PRESET_FIVE_FSRS5, &PRESET_FIVE_FSRS4],
        );
        add_preset(
            col,
            PRESET_FOUR,
            "Four",
            0.9,
            [&[], &[], &PRESET_FOUR_FSRS4],
        );
        add_preset(
            col,
            PRESET_ON_DEFAULTS,
            "On defaults",
            0.9,
            [&defaults, &[], &[]],
        );
        for (name, preset) in PRESET_DECKS {
            let mut deck = (*col
                .get_deck(anki::decks::DeckId(ids[name]))
                .expect("the deck is read")
                .expect("the deck exists"))
            .clone();
            if let anki::decks::DeckKind::Normal(normal) = &mut deck.kind {
                normal.config_id = preset;
            }
            col.update_deck(&mut deck)
                .expect("the deck names its preset");
        }
        let basic = col
            .get_notetype_by_name("Basic")
            .expect("the note types are read")
            .expect("the engine creates its stock Basic note type")
            .id
            .0;
        let db = col.storage.db();
        db.execute_batch("begin").expect("a transaction opens");
        for (position, (id, card, deck)) in PRESET_CARDS.iter().enumerate() {
            let position = i64::try_from(position).unwrap() + 1;
            let columns = card.columns(today, timing.now.0, position);
            insert_planned_note(col, basic, *id, ids[*deck], columns);
        }
        db.execute_batch("commit").expect("the transaction commits");
        let borrowed: Vec<i64> = PRESET_CARDS
            .iter()
            .filter(|(_, card, _)| *card == SkipCard::FilteredReview)
            .map(|(id, _, _)| *id)
            .collect();
        let filtered = build_filtered(col, &borrowed);
        match setup.rollover {
            Some(hour) => col
                .set_config_json("rollover", &u32::from(hour), false)
                .map(|_| ()),
            None => col.remove_config("rollover").map(|_| ()),
        }
        .expect("the rollover hour is set");
        match setup.utc_offset_west {
            Some(minutes) => col
                .set_config_json("localOffset", &minutes, false)
                .map(|_| ()),
            None => col.remove_config("localOffset").map(|_| ()),
        }
        .expect("the configured UTC offset is set");
        let mut decks = ids;
        if let Some(id) = filtered {
            decks.insert(FILTERED_DECK.to_owned(), id);
        }
        PresetPlan {
            decks,
            filtered,
            defaults,
        }
    })
}
