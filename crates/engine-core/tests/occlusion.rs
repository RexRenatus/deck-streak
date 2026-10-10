//! An image occlusion question, withheld because this app does not draw its masks (SPEC-380 R1,
//! R2, A1 to A5; ADR-391 D1).
//!
//! A4 builds its note with the engine's own image occlusion interface from fixed inputs: a fixed
//! image's bytes, one fixed rectangle and a fixed header that speaks through a TTS tag, so the
//! render every run reads is the render the literals of A1 and A2 were copied from. A5 builds a
//! Basic and a Cloze note beside it. Every expected value is written out here, so no expectation
//! is computed by the code under test.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what an \
              enumerating test examined"
)]

mod support;

use std::io::Read;
use std::path::{Path, PathBuf};

use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki_proto::collection::OpenCollectionRequest;
use anki_proto::image_occlusion::AddImageOcclusionNoteRequest;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::face::{Face, Side};
use deck_streak_engine_core::media::Reader;
use deck_streak_engine_core::occlusion;
use deck_streak_engine_core::table::Transport;
use prost::Message;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);

/// The occluded image, and its bytes: a PNG signature.
const IMAGE: &str = "occluded.png";
const IMAGE_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n";
/// One rectangle over the image, as the engine's occlusion field spells a shape.
const SHAPES: &str = "{{c1::image-occlusion:rect:left=.2:top=.3:width=.4:height=.1}}";
/// A header that speaks, so the unwithheld face carries a clip on each side.
const HEADER: &str = "[anki:tts lang=en_US]the parts of a cell[/anki:tts]";
/// A Basic note's front and back.
const BASIC_FRONT: &str = "a basic front";
const BASIC_BACK: &str = "a basic back";
/// A Cloze note's text: one deletion.
const CLOZE_TEXT: &str = "{{c1::Paris}} is in France";

/// The mask layer and the shape as the engine renders A4's note, copied from its question's text
/// as the build measured it: the image's container with the layer and no shape, and the hidden
/// shape with no layer.
const LAYER_ALONE: &str = concat!(
    "<div id=\"image-occlusion-container\">\n",
    "    <img src=\"data:image/png;base64,iVBORw0KGgo=\">\n",
    "    <canvas id=\"image-occlusion-canvas\"></canvas>\n",
    "</div>",
);
const SHAPE_ALONE: &str = concat!(
    "<div style=\"display: none\"><div class=\"cloze\" data-ordinal=\"1\" data-shape=\"rect\" ",
    "data-left=\".2\" data-top=\".3\" data-width=\".4\" data-height=\".1\" ></div></div>",
);

/// The stock Basic note type's CSS, and the stock Cloze note type's, as the engine's own
/// `styling.css` and `cloze_styling.css` spell them.
const BASIC_CSS: &str = concat!(
    ".card {\n    font-family: arial;\n    font-size: 20px;\n    line-height: 1.5;\n",
    "    text-align: center;\n    color: black;\n    background-color: white;\n}\n",
);
const CLOZE_CSS: &str = concat!(
    ".card {\n    font-family: arial;\n    font-size: 20px;\n    line-height: 1.5;\n",
    "    text-align: center;\n    color: black;\n    background-color: white;\n}\n",
    ".cloze {\n    font-weight: bold;\n    color: blue;\n}\n",
    ".nightMode .cloze {\n    color: lightblue;\n}\n",
);

/// The cards the fixture's notes made.
struct Cards {
    /// The image occlusion note's one card.
    occlusion: i64,
    /// The Basic note's card.
    basic: i64,
    /// The Cloze note's one card.
    cloze: i64,
}

struct Fixture {
    dir: PathBuf,
    collection: PathBuf,
    cards: Cards,
}

fn deck(col: &mut Collection, name: &str) -> DeckId {
    col.get_or_create_normal_deck(name)
        .expect("the engine creates the deck")
        .id
}

/// The one card in `deck`.
fn card_in(col: &Collection, deck: DeckId) -> i64 {
    col.storage
        .db()
        .query_row("select id from cards where did = ?", [deck.0], |row| {
            row.get(0)
        })
        .expect("the deck holds one card")
}

fn fixture(test: &str) -> Fixture {
    let dir = support::scratch("engine-core-occlusion", test);
    let folder = dir.join("collection.media");
    std::fs::create_dir_all(&folder).expect("a media directory");
    let image = dir.join(IMAGE);
    std::fs::write(&image, IMAGE_BYTES).expect("the image is written");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .set_media_paths(folder, dir.join("collection.media.db"))
        .build()
        .expect("the engine creates the collection");
    let occluded = deck(&mut col, "Occlusion");
    col.set_current_deck(occluded)
        .expect("the engine sets the current deck");
    col.add_image_occlusion_note(AddImageOcclusionNoteRequest {
        image_path: text(&image),
        occlusions: SHAPES.to_owned(),
        header: HEADER.to_owned(),
        back_extra: String::new(),
        tags: Vec::new(),
        notetype_id: 0,
    })
    .expect("the engine adds the image occlusion note");
    let basic = deck(&mut col, "Basic");
    let cloze = deck(&mut col, "Cloze");
    for (notetype, deck, fields) in [
        ("Basic", basic, [BASIC_FRONT, BASIC_BACK]),
        ("Cloze", cloze, [CLOZE_TEXT, ""]),
    ] {
        let notetype = col
            .get_notetype_by_name(notetype)
            .expect("the note types are read")
            .expect("the engine creates its stock note type");
        let mut note = notetype.new_note();
        for (index, field) in fields.into_iter().enumerate() {
            note.set_field(index, field).expect("the field is set");
        }
        col.add_note(&mut note, deck)
            .expect("the engine adds the note");
    }
    let cards = Cards {
        occlusion: card_in(&col, occluded),
        basic: card_in(&col, basic),
        cloze: card_in(&col, cloze),
    };
    col.close(None).expect("the engine closes the collection");
    Fixture {
        dir,
        collection,
        cards,
    }
}

fn text(path: &Path) -> String {
    path.to_str().expect("a scratch path is UTF-8").to_owned()
}

/// A native dispatcher with the fixture's collection open.
fn open(fixture: &Fixture) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let request = OpenCollectionRequest {
        collection_path: text(&fixture.collection),
        media_folder_path: text(&fixture.dir.join("collection.media")),
        media_db_path: text(&fixture.dir.join("collection.media.db")),
    };
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the native dispatcher opens the collection");
    dispatcher
}

/// Reads from the folder the dispatcher kept at the open, at most `limit` bytes; nothing when it
/// kept none.
struct Folder(Option<PathBuf>);

impl Reader for Folder {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let file = std::fs::File::open(self.0.as_ref()?.join(name)).ok()?;
        let mut bytes = Vec::new();
        file.take(limit).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }
}

/// `card`'s face for `side`, with autoplay wished.
fn face(dispatcher: &Dispatcher, card: i64, side: Side) -> Face {
    let folder = Folder(dispatcher.media_folder().map(PathBuf::from));
    dispatcher
        .face(card, side, true, &folder)
        .expect("the engine renders the card")
}

/// The face R2 answers for a withheld card: nothing shown, nothing played, nothing left out.
fn withheld() -> Face {
    Face {
        text: String::new(),
        css: String::new(),
        autoplay: Vec::new(),
        replay: Vec::new(),
        omitted: Vec::new(),
        withheld: true,
    }
}

/// A face the review shows: its text and CSS, nothing to play and nothing left out.
fn shown(text: &str, css: &str) -> Face {
    Face {
        text: text.to_owned(),
        css: css.to_owned(),
        autoplay: Vec::new(),
        replay: Vec::new(),
        omitted: Vec::new(),
        withheld: false,
    }
}

#[test]
fn a_question_holding_the_mask_layer_is_withheld() {
    assert!(
        occlusion::masks_not_drawn(LAYER_ALONE),
        "a question holding the engine's mask layer and no shape is not marked: {LAYER_ALONE}"
    );
}

#[test]
fn a_question_holding_an_occlusion_shape_is_withheld() {
    assert!(
        occlusion::masks_not_drawn(SHAPE_ALONE),
        "a question holding an occlusion shape and no mask layer is not marked: {SHAPE_ALONE}"
    );
}

#[test]
fn a_plain_or_text_cloze_question_is_not_withheld() {
    let questions = [
        "a basic front",
        r#"<span class="cloze" data-cloze="Paris" data-ordinal="1">[...]</span> is in France"#,
        r#"<img src="data:image/png;base64,iVBORw0KGgo=" alt="a dot">"#,
        "Image occlusion hides a part of a picture behind a mask.",
    ];
    let marked: Vec<&str> = questions
        .into_iter()
        .filter(|question| occlusion::masks_not_drawn(question))
        .collect();
    assert_eq!(
        marked,
        Vec::<&str>::new(),
        "a question with no mask layer and no shape is marked"
    );
    support::examined("question(s) with no mask", questions.to_vec());
}

#[test]
fn the_engines_occlusion_card_is_withheld_on_both_sides() {
    let fixture = fixture("a4");
    let dispatcher = open(&fixture);
    let sides = [Side::Question, Side::Answer];
    for side in sides {
        let shown = face(&dispatcher, fixture.cards.occlusion, side);
        assert_eq!(
            shown,
            withheld(),
            "the engine's image occlusion card is not withheld on its {side:?} side"
        );
    }
    support::examined("side(s) of the occlusion card", sides.to_vec());
}

#[test]
fn a_basic_and_a_cloze_card_keep_their_faces() {
    let fixture = fixture("a5");
    let dispatcher = open(&fixture);
    let faces = [
        (
            fixture.cards.basic,
            Side::Question,
            shown(BASIC_FRONT, BASIC_CSS),
        ),
        (
            fixture.cards.basic,
            Side::Answer,
            shown("a basic front\n\n<hr id=answer>\n\na basic back", BASIC_CSS),
        ),
        (
            fixture.cards.cloze,
            Side::Question,
            shown(
                r#"<span class="cloze" data-cloze="Paris" data-ordinal="1">[...]</span> is in France"#,
                CLOZE_CSS,
            ),
        ),
        (
            fixture.cards.cloze,
            Side::Answer,
            shown(
                "<span class=\"cloze\" data-ordinal=\"1\">Paris</span> is in France<br>\n",
                CLOZE_CSS,
            ),
        ),
    ];
    for (card, side, expected) in &faces {
        assert_eq!(
            &face(&dispatcher, *card, *side),
            expected,
            "card {card}'s {side:?} side is not the face the engine shows"
        );
    }
    support::examined("face(s) of a Basic and a Cloze card", faces.to_vec());
}
