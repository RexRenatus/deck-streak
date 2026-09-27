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
