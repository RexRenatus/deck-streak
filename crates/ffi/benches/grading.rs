//! The native adapter's answer bench (SPEC-618 R9, R13): `Engine::answer`, the
//! adapter's one answer entry, with the core's shown states, grade pick, answer request and
//! dispatcher beneath it. One press per grade.
//!
//! The collection is built here with the engine's own API from a fixed seed: 2,000 Basic notes in
//! the default deck, built once and closed. Each iteration takes a fresh copy of it, opens it
//! through the adapter and reads the queue's head card and the states it is shown with, all
//! outside the measured section (Divan's `with_inputs`). The measured body is the named
//! `#[inline(never)]` function `answer_measured`; no bench asserts a value the engine computes.

#![allow(
    clippy::expect_used,
    reason = "a bench whose fixed input cannot be built should stop, as a failed test does"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use deck_streak_ffi::engine::{Engine, PressRefusal, PressedGrade};
use divan::Bencher;

/// The collection's cards, one Basic note each.
const CARDS: u32 = 2_000;
/// The words a note's fields are drawn from.
const WORDS: [&str; 16] = [
    "river", "stone", "lantern", "harbor", "meadow", "copper", "willow", "ember", "glacier",
    "orchard", "falcon", "thistle", "canyon", "saffron", "beacon", "tundra",
];
/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The time each press took.
const TAKEN_MS: u32 = 4_000;

fn main() {
    divan::main();
}

/// A fixed-seed generator (`SplitMix64`): one seed gives the same collection on every run.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    fn word(&mut self) -> &'static str {
        let index = usize::try_from(self.next() % 16).unwrap_or_default();
        WORDS.get(index).copied().unwrap_or("river")
    }
}

/// A directory of this run's own under the target's scratch space.
fn scratch(name: &str) -> PathBuf {
    let run = std::process::id();
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-bench-grading")
        .join(format!("{name}-{run}"))
}

/// The built collection's file: 2,000 seeded Basic notes in the default deck, built once.
fn master() -> &'static Path {
    static MASTER: OnceLock<PathBuf> = OnceLock::new();
    MASTER.get_or_init(|| build(&scratch("master"))).as_path()
}

fn build(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine has a stock Basic note type");
    let mut seeded = Seeded(0x0618_0006);
    for index in 0..CARDS {
        let mut note = basic.new_note();
        let front = format!("{} {} {index}", seeded.word(), seeded.word());
        let back = format!("{} {}", seeded.word(), seeded.word());
        note.set_field(0, front).expect("the front is set");
        note.set_field(1, back).expect("the back is set");
        col.add_note(&mut note, DeckId(1))
            .expect("the engine adds the note to the default deck");
    }
    col.close(None).expect("the engine closes the collection");
    collection
}

/// A fresh copy of the collection, opened through the adapter, with the queue's head card and
/// the states it is shown with.
struct Shown {
    engine: Arc<Engine>,
    card: i64,
    states: Vec<u8>,
}

fn shown() -> Shown {
    static COPIES: AtomicU64 = AtomicU64::new(0);
    let copy = COPIES.fetch_add(1, Ordering::Relaxed);
    let dir = scratch(&format!("copy-{copy}"));
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    std::fs::copy(master(), &collection).expect("the collection is copied");
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, open_request(&dir, &collection))
        .expect("the adapter opens the copy");
    let mut request = Vec::new();
    put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    let queued = engine
        .run(service, method, request)
        .expect("the queue is on the allow-list");
    let head = repeated(&queued, 1)
        .into_iter()
        .next()
        .expect("the collection queues a new card");
    let card = repeated(&head, 1).pop().unwrap_or_default();
    Shown {
        engine,
        card: i64::from_ne_bytes(varint(&card, 1).to_ne_bytes()),
        states: repeated(&head, 3).pop().unwrap_or_default(),
    }
}

#[divan::bench(args = ["again", "good"], sample_count = 10, sample_size = 1)]
fn answer(bencher: Bencher, grade: &str) {
    let grade = if grade == "again" {
        PressedGrade::Again
    } else {
        PressedGrade::Good
    };
    bencher.with_inputs(shown).bench_local_values(|shown| {
        let recorded = answer_measured(&shown.engine, shown.card, grade, shown.states);
        (shown.engine, recorded)
    });
}

/// One press through the adapter's answer entry, on the card the queue showed, with the
/// states it was shown with.
#[inline(never)]
fn answer_measured(
    engine: &Engine,
    card: i64,
    grade: PressedGrade,
    states: Vec<u8>,
) -> Result<Vec<u8>, PressRefusal> {
    engine.answer(card, grade, states, TAKEN_MS)
}

/// `OpenCollectionRequest` for the copy: its path, its media folder and its media database.
fn open_request(dir: &Path, collection: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    put_bytes(&mut out, 1, text(collection).as_bytes());
    put_bytes(&mut out, 2, text(&dir.join("collection.media")).as_bytes());
    put_bytes(
        &mut out,
        3,
        text(&dir.join("collection.media.db")).as_bytes(),
    );
    out
}

fn text(path: &Path) -> &str {
    path.to_str().expect("a scratch path is UTF-8")
}

/// Appends a base-128 varint.
fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push(low_seven(value) | 0x80);
        value >>= 7;
    }
    out.push(low_seven(value));
}

fn low_seven(value: u64) -> u8 {
    u8::try_from(value & 0x7f).unwrap_or_default()
}

/// Appends a varint field.
fn put_varint_field(out: &mut Vec<u8>, field: u64, value: u64) {
    put_varint(out, field << 3);
    put_varint(out, value);
}

/// Appends a length-delimited field.
fn put_bytes(out: &mut Vec<u8>, field: u64, bytes: &[u8]) {
    put_varint(out, (field << 3) | 2);
    put_varint(out, u64::try_from(bytes.len()).unwrap_or_default());
    out.extend_from_slice(bytes);
}

/// One field of a message: a varint, a length-delimited value, or a fixed-width value.
enum Field {
    Varint(u64),
    Bytes(Vec<u8>),
    Fixed,
}

fn read_varint(bytes: &[u8], at: &mut usize) -> u64 {
    let mut value = 0_u64;
    let mut shift = 0;
    while let Some(byte) = bytes.get(*at) {
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    value
}

fn fields(bytes: &[u8]) -> Vec<(u64, Field)> {
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        let key = read_varint(bytes, &mut at);
        let field = match key & 7 {
            0 => Field::Varint(read_varint(bytes, &mut at)),
            2 => {
                let length = usize::try_from(read_varint(bytes, &mut at)).unwrap_or_default();
                let end = at.saturating_add(length).min(bytes.len());
                let value = bytes.get(at..end).unwrap_or_default().to_vec();
                at = end;
                Field::Bytes(value)
            }
            1 => {
                at += 8;
                Field::Fixed
            }
            _ => {
                at += 4;
                Field::Fixed
            }
        };
        out.push((key >> 3, field));
    }
    out
}

/// A varint field's last value, or zero when the message omits it.
fn varint(bytes: &[u8], number: u64) -> u64 {
    fields(bytes)
        .into_iter()
        .filter_map(|(field, value)| match value {
            Field::Varint(value) if field == number => Some(value),
            _ => None,
        })
        .next_back()
        .unwrap_or_default()
}

/// Every occurrence of a length-delimited field, in order.
fn repeated(bytes: &[u8], number: u64) -> Vec<Vec<u8>> {
    fields(bytes)
        .into_iter()
        .filter_map(|(field, value)| match value {
            Field::Bytes(value) if field == number => Some(value),
            _ => None,
        })
        .collect()
}
