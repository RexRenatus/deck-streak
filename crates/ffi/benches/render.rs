//! The native adapter's face bench (SPEC-618 R9, R13): `Engine::face`, the engine's
//! question and answer page, over the review's golden template cards.
//!
//! The collection is built here with the engine's own API from the golden file's inputs
//! (`web/app/src/lib/study/review-templates.golden.json`), as the engine's template test builds
//! it: two note types cloned from the stock Cloze and Basic (and reversed card) types, each given
//! its name, its CSS and its templates, and one note of each in the default deck. The inputs are
//! written below and each is checked to stand in the golden file, as the file escapes it, before
//! anything is built. The measured body is the named `#[inline(never)]` function `face_measured`,
//! which completes every card's face on one side; no bench asserts a value the engine computes.

#![allow(
    clippy::expect_used,
    reason = "a bench whose fixed input cannot be built should stop, as a failed test does"
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use anki::notetype::{Notetype, NotetypeId};
use deck_streak_ffi::engine::{Engine, EngineRefusal};
use deck_streak_ffi::face::CardFace;
use divan::Bencher;

/// The golden file, read at build time from the workspace's root.
const GOLDEN: &str = include_str!("../../../web/app/src/lib/study/review-templates.golden.json");
/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);

/// One golden note type: its name, the stock type it is cloned from, its CSS, and each template's
/// name, question and answer.
struct GoldenNotetype {
    name: &'static str,
    stock: &'static str,
    css: &'static str,
    templates: &'static [[&'static str; 3]],
}

/// The golden file's note types.
const NOTETYPES: [GoldenNotetype; 2] = [
    GoldenNotetype {
        name: "Template frame cloze",
        stock: "Cloze",
        css: ".card {\r\n  font-family: arial;\r\n  font-size: 20px;\r\n  text-align: center;\r\n}\r\n\r\n.cloze {\r\n  font-weight: bold;\r\n  color: blue;\r\n}\r.nightMode .cloze {\n  color: lightblue;\n}\n",
        templates: &[[
            "Cloze",
            "{{cloze:Text}}",
            "{{cloze:Text}}<br>\n{{Back Extra}}",
        ]],
    },
    GoldenNotetype {
        name: "Template frame reversed",
        stock: "Basic (and reversed card)",
        css: ".card {\n  font-family: serif;\n  font-size: 18px;\n}\n\nhr#answer {\n  border-top: 1px solid gray;\n}\n",
        templates: &[
            [
                "Card 1",
                "{{Front}}",
                "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
            ],
            [
                "Card 2",
                "{{Back}}",
                "{{FrontSide}}\n\n<hr id=answer>\n\n{{Front}}",
            ],
        ],
    },
];

/// The golden file's notes: each one's note type and fields.
const NOTES: [(&str, [&str; 2]); 2] = [
    (
        "Template frame cloze",
        ["The {{c1::dog}} chases the {{c2::cat}}.", "an extra line"],
    ),
    ("Template frame reversed", ["der Hund", "the dog"]),
];

fn main() {
    divan::main();
}

/// `text` as a JSON string, quoted and escaped as the golden file writes it.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Every input written above, as it stands in the golden file.
fn inputs() -> Vec<&'static str> {
    let mut inputs = Vec::new();
    for notetype in &NOTETYPES {
        inputs.extend([notetype.name, notetype.stock, notetype.css]);
        for template in notetype.templates {
            inputs.extend(template);
        }
    }
    for (notetype, fields) in &NOTES {
        inputs.push(*notetype);
        inputs.extend(fields);
    }
    inputs
}

/// The built collection's directory, its file and its cards in note order, then by ordinal,
/// built once.
fn built() -> &'static (PathBuf, PathBuf, Vec<i64>) {
    static BUILT: OnceLock<(PathBuf, PathBuf, Vec<i64>)> = OnceLock::new();
    BUILT.get_or_init(|| {
        for input in inputs() {
            assert!(
                GOLDEN.contains(&json_string(input)),
                "the golden file holds the input {input:?}"
            );
        }
        let run = std::process::id();
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("ffi-bench-render")
            .join(format!("golden-{run}"));
        build(&dir)
    })
}

fn build(dir: &Path) -> (PathBuf, PathBuf, Vec<i64>) {
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    for golden in &NOTETYPES {
        let found = col
            .get_notetype_by_name(golden.stock)
            .expect("the note types are read")
            .expect("the engine has the stock note type");
        let mut cloned = Notetype::clone(&found);
        cloned.id = NotetypeId(0);
        golden.name.clone_into(&mut cloned.name);
        golden.css.clone_into(&mut cloned.config.css);
        for (template, &[name, question, answer]) in
            cloned.templates.iter_mut().zip(golden.templates)
        {
            name.clone_into(&mut template.name);
            question.clone_into(&mut template.config.q_format);
            answer.clone_into(&mut template.config.a_format);
        }
        col.add_notetype(&mut cloned, false)
            .expect("the engine adds the note type");
    }
    for (notetype, fields) in &NOTES {
        let found = col
            .get_notetype_by_name(notetype)
            .expect("the note types are read")
            .expect("the collection has the golden note type");
        let mut note = found.new_note();
        for (index, field) in fields.iter().enumerate() {
            note.set_field(index, *field).expect("the field is set");
        }
        col.add_note(&mut note, DeckId(1))
            .expect("the engine adds the note to the default deck");
    }
    let mut statement = col
        .storage
        .db()
        .prepare("select id from cards order by nid, ord")
        .expect("the cards are read");
    let cards = statement
        .query_map([], |row| row.get(0))
        .expect("the cards are read")
        .map(|id| id.expect("a card's id is read"))
        .collect();
    drop(statement);
    col.close(None).expect("the engine closes the collection");
    (dir.to_path_buf(), collection, cards)
}

/// The adapter, opened on the built collection.
fn opened(dir: &Path, collection: &Path) -> Arc<Engine> {
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, open_request(dir, collection))
        .expect("the adapter opens the collection");
    engine
}

#[divan::bench(args = ["question", "answer"])]
fn face(bencher: Bencher, side: &str) {
    let (dir, collection, cards) = built();
    let engine = opened(dir, collection);
    let answer = side == "answer";
    bencher.bench_local(|| face_measured(&engine, cards, answer));
}

/// Each golden card's face on one side, as one closed page, by day and with no autoplay.
#[inline(never)]
fn face_measured(
    engine: &Engine,
    cards: &[i64],
    answer: bool,
) -> Vec<Result<CardFace, EngineRefusal>> {
    cards
        .iter()
        .map(|card| engine.face(*card, answer, false, false))
        .collect()
}

/// `OpenCollectionRequest` for the collection: its path, its media folder and its media database.
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
        out.push(u8::try_from(value & 0x7f).unwrap_or_default() | 0x80);
        value >>= 7;
    }
    out.push(u8::try_from(value).unwrap_or_default());
}

/// Appends a length-delimited field.
fn put_bytes(out: &mut Vec<u8>, field: u64, bytes: &[u8]) {
    put_varint(out, (field << 3) | 2);
    put_varint(out, u64::try_from(bytes.len()).unwrap_or_default());
    out.extend_from_slice(bytes);
}
