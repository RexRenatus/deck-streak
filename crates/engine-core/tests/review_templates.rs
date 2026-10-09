//! The engine's render of the review's template cards, against the golden file the web review's
//! test serves (SPEC-372 R3, A5; ADR-383 D1, D2).
//!
//! The test builds its collection from the golden file's inputs alone: two note types cloned from
//! the engine's stock Cloze and Basic (and reversed card) types, each given its name, its CSS and
//! its templates from the golden file, and one note of each in the default deck, every field from
//! the golden file. It names each card by its note and its ordinal, never by id. Through a native
//! dispatcher it completes each card's two faces as the Worker's `faces` asks for them, and renders
//! the card with the partial flag off as the Worker's `current_card` does for its CSS.
//!
//! The anchors are written here by hand and run first: each cloze card hides its own deletion and
//! shows the other, each answer reveals it and adds the extra field, each reversed answer is its
//! question, the rule and the other field, every CSS equals its note type's input byte for byte,
//! and no id of this run reaches any text. Only then is the render compared with the golden file's
//! `cards`, and a mismatch prints the render this run computed. The test never writes the golden
//! file.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what an \
              enumerating test examined"
)]

mod support;

use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use anki::notetype::{Notetype, NotetypeId};
use anki_proto::card_rendering::{RenderCardResponse, RenderExistingCardRequest};
use anki_proto::collection::OpenCollectionRequest;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::face::{Face, Side};
use deck_streak_engine_core::media::Reader;
use deck_streak_engine_core::table::Transport;
use prost::Message;
use serde_json::{Value, json};

/// The golden file, from the workspace's root: the web test's directory, which the web mutation
/// run's sandbox holds (ADR-383 D1).
const GOLDEN: &str = "web/app/src/lib/study/review-templates.golden.json";
/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `CardRenderingService.RenderExistingCard`, the call the Worker's `current_card` makes.
const RENDER_EXISTING_CARD: (u32, u32) = (27, 6);
/// What a cloze deletion shows on the question that asks for it.
const HIDDEN: &str = "[...]";

/// A reader that holds no file: the fixture's cards name no media.
struct NoFiles;

impl Reader for NoFiles {
    fn read(&self, _name: &str, _limit: u64) -> Option<Vec<u8>> {
        None
    }
}

/// One card of the fixture, named by its note's golden name and its ordinal, with the ids this run
/// gave it and its note, which no rendered text may hold.
struct Card {
    note: String,
    notetype: String,
    ordinal: u32,
    id: i64,
    note_id: i64,
}

/// One card as the engine rendered it: both faces, and the CSS of its render with the partial flag
/// off.
struct Rendered {
    card: Card,
    question: Face,
    answer: Face,
    card_css: String,
}

impl Rendered {
    /// The card's name in a failure message.
    fn name(&self) -> String {
        format!("{} ordinal {}", self.card.note, self.card.ordinal)
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("the golden file's {key} is a string: {value}"))
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("the golden file's {key} is a list: {value}"))
}

fn golden() -> Value {
    let path = support::workspace().join(GOLDEN);
    let written = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("the golden file {} is read: {error}", path.display()));
    serde_json::from_str(&written).expect("the golden file is JSON")
}

/// Adds each golden note type: the stock type it names, cloned, with its name, its CSS and each
/// template's name, question and answer set from the golden file.
fn add_notetypes(col: &mut anki::collection::Collection, golden: &Value) {
    for notetype in list(golden, "notetypes") {
        let stock = text(notetype, "stock");
        let found = col
            .get_notetype_by_name(stock)
            .expect("the note types are read")
            .unwrap_or_else(|| panic!("the engine has a stock note type named {stock}"));
        let mut cloned = Notetype::clone(&found);
        cloned.id = NotetypeId(0);
        text(notetype, "name").clone_into(&mut cloned.name);
        text(notetype, "css").clone_into(&mut cloned.config.css);
        let templates = list(notetype, "templates");
        assert_eq!(
            cloned.templates.len(),
            templates.len(),
            "{stock}: the golden file sets every template the stock type has"
        );
        for (template, written) in cloned.templates.iter_mut().zip(templates) {
            text(written, "name").clone_into(&mut template.name);
            text(written, "question").clone_into(&mut template.config.q_format);
            text(written, "answer").clone_into(&mut template.config.a_format);
        }
        col.add_notetype(&mut cloned, false)
            .expect("the engine adds the note type");
    }
}

/// Builds the collection from the golden file's inputs, closes it, and names its cards in golden
/// note order, then by ordinal.
fn build(golden: &Value, dir: &Path) -> (PathBuf, Vec<Card>) {
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    add_notetypes(&mut col, golden);
    let mut cards = Vec::new();
    for note in list(golden, "notes") {
        let notetype = text(note, "notetype");
        let found = col
            .get_notetype_by_name(notetype)
            .expect("the note types are read")
            .unwrap_or_else(|| panic!("the collection has the note type {notetype}"));
        let mut added = found.new_note();
        let fields = list(note, "fields");
        assert_eq!(
            added.fields().len(),
            fields.len(),
            "{notetype}: the golden file sets every field"
        );
        for (index, field) in fields.iter().enumerate() {
            let field = field.as_str().expect("a golden field is a string");
            added.set_field(index, field).expect("the field is set");
        }
        col.add_note(&mut added, DeckId(1))
            .expect("the engine adds the note to the default deck");
        let mut statement = col
            .storage
            .db()
            .prepare("select id, ord from cards where nid = ? order by ord")
            .expect("the cards are read");
        let rows = statement
            .query_map([added.id.0], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("the note's cards are read");
        for row in rows {
            let (id, ordinal) = row.expect("a card's id and ordinal are read");
            cards.push(Card {
                note: text(note, "name").to_owned(),
                notetype: notetype.to_owned(),
                ordinal,
                id,
                note_id: added.id.0,
            });
        }
    }
    col.close(None).expect("the engine closes the collection");
    (collection, cards)
}

/// A native dispatcher with the collection at `collection`, in `dir`, open.
fn open_at(dir: &Path, collection: &Path) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let path = |path: &Path| path.to_str().expect("a scratch path is UTF-8").to_owned();
    let request = OpenCollectionRequest {
        collection_path: path(collection),
        media_folder_path: path(&dir.join("collection.media")),
        media_db_path: path(&dir.join("collection.media.db")),
    };
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the native dispatcher opens the collection");
    dispatcher
}

/// Both faces of `card`, with the arguments the Worker's `faces` passes, and the CSS of its render
/// with the partial flag off, the call the Worker's `current_card` makes.
fn render(dispatcher: &Dispatcher, card: Card) -> Rendered {
    let question = dispatcher
        .face(card.id, Side::Question, true, &NoFiles)
        .expect("the engine completes the question");
    let answer = dispatcher
        .face(card.id, Side::Answer, true, &NoFiles)
        .expect("the engine completes the answer");
    let request = RenderExistingCardRequest {
        card_id: card.id,
        browser: false,
        partial_render: false,
    };
    let (service, method) = RENDER_EXISTING_CARD;
    let reply = dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the native dispatcher renders the card");
    let rendered =
        RenderCardResponse::decode(reply.as_slice()).expect("the render is a RenderCardResponse");
    Rendered {
        card,
        question,
        answer,
        card_css: rendered.css,
    }
}

fn find<'a>(rendered: &'a [Rendered], note: &str, ordinal: u32) -> &'a Rendered {
    rendered
        .iter()
        .find(|shown| shown.card.note == note && shown.card.ordinal == ordinal)
        .unwrap_or_else(|| panic!("the fixture has the card {note} ordinal {ordinal}"))
}

/// A cloze card's anchors: its question hides its own deletion once and shows the other; its
/// answer reveals its deletion, adds the extra field, and hides nothing.
fn cloze_anchors(shown: &Rendered, number: &str, hidden: &str, other: &str) {
    let name = shown.name();
    let question = &shown.question.text;
    let answer = &shown.answer.text;
    assert!(
        question.contains(&format!("data-ordinal=\"{number}\"")),
        "{name}: the question asks for deletion {number}: {question}"
    );
    assert_eq!(
        question.matches(HIDDEN).count(),
        1,
        "{name}: the question hides one deletion: {question}"
    );
    assert!(
        question.contains(other),
        "{name}: the question shows the other deletion, {other}: {question}"
    );
    assert!(
        answer.contains(&format!("{hidden}</span>")),
        "{name}: the answer reveals {hidden}: {answer}"
    );
    assert!(
        answer.contains("an extra line"),
        "{name}: the answer adds the extra field: {answer}"
    );
    assert!(
        !answer.contains(HIDDEN),
        "{name}: the answer hides nothing: {answer}"
    );
}

/// A reversed card's anchors: its question is the one field, and its answer is that question, the
/// rule and the other field.
fn reversed_anchors(shown: &Rendered, question: &str, other: &str) {
    let name = shown.name();
    assert_eq!(shown.question.text, question, "{name}: the question");
    assert_eq!(
        shown.answer.text,
        format!("{question}\n\n<hr id=answer>\n\n{other}"),
        "{name}: the answer opens with its question where the template says FrontSide"
    );
}

/// Every card's three CSS equal its note type's golden CSS byte for byte; each note's ordinals
/// read 0 and 1; and no rendered text holds an id of this run.
fn every_card_anchors(rendered: &[Rendered], golden: &Value) {
    for shown in rendered {
        let name = shown.name();
        let notetype = list(golden, "notetypes")
            .iter()
            .find(|notetype| text(notetype, "name") == shown.card.notetype)
            .unwrap_or_else(|| panic!("{name}: the golden file has its note type"));
        let css = text(notetype, "css");
        assert_eq!(shown.question.css, css, "{name}: the question face's CSS");
        assert_eq!(shown.answer.css, css, "{name}: the answer face's CSS");
        assert_eq!(shown.card_css, css, "{name}: the render's CSS");
    }
    for note in list(golden, "notes") {
        let name = text(note, "name");
        let ordinals: Vec<u32> = rendered
            .iter()
            .filter(|shown| shown.card.note == name)
            .map(|shown| shown.card.ordinal)
            .collect();
        assert_eq!(ordinals, vec![0, 1], "{name}: the note's cards' ordinals");
    }
    let ids: Vec<String> = rendered
        .iter()
        .flat_map(|shown| [shown.card.id, shown.card.note_id])
        .map(|id| id.to_string())
        .collect();
    for shown in rendered {
        let texts = [&shown.question.text, &shown.answer.text];
        for (id, written) in ids.iter().flat_map(|id| texts.map(|written| (id, written))) {
            assert!(
                !written.contains(id.as_str()),
                "{}: a rendered text holds the id {id} of this run: {written}",
                shown.name()
            );
        }
    }
}

/// The render as the golden file's `cards` records it, in golden note order, then by ordinal.
fn computed(rendered: &[Rendered]) -> Value {
    rendered
        .iter()
        .map(|shown| {
            json!({
                "note": shown.card.note,
                "ordinal": shown.card.ordinal,
                "question": shown.question.text,
                "answer": shown.answer.text,
                "face_css": shown.question.css,
                "card_css": shown.card_css,
            })
        })
        .collect()
}

#[test]
fn the_engine_renders_the_template_cards_as_the_golden_file_records_them() {
    let golden = golden();
    let dir = support::scratch(
        "review_templates",
        "the_engine_renders_the_template_cards_as_the_golden_file_records_them",
    );
    let (collection, cards) = build(&golden, &dir);
    let dispatcher = open_at(&dir, &collection);
    let rendered: Vec<Rendered> = cards
        .into_iter()
        .map(|card| render(&dispatcher, card))
        .collect();

    cloze_anchors(find(&rendered, "cloze", 0), "1", "dog", "cat");
    cloze_anchors(find(&rendered, "cloze", 1), "2", "cat", "dog");
    reversed_anchors(find(&rendered, "reversed", 0), "der Hund", "the dog");
    reversed_anchors(find(&rendered, "reversed", 1), "the dog", "der Hund");
    every_card_anchors(&rendered, &golden);

    let rendered = support::examined("rendered cards", rendered);
    let computed = computed(&rendered);
    assert_eq!(
        computed,
        golden["cards"],
        "the engine's render differs from the golden file's cards; it rendered:\n{}",
        serde_json::to_string_pretty(&computed).expect("the render prints as JSON")
    );
}
