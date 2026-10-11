//! A card's face, completed as the engine's own reviewer completes it (SPEC-348 R2 to R4, A3 to
//! A7).
//!
//! Each test builds its own collection with the engine's own API: Basic notes whose fronts play a
//! sound, show an image and speak through TTS tags, in decks on three presets (the default, one
//! that skips the question when replaying the answer, and two with autoplay disabled, one of whose
//! cards sits in a filtered deck). The test opens it through a native dispatcher and asks for faces.
//! A4 reads its media through the folder the open request named, as the dispatcher keeps it; A5
//! reads from memory, where it plants every name the rules refuse. Every expected value is written
//! out here, the `data:` URLs' base64 included, so no expectation is computed by the code under
//! test.
//!
//! The parity tests (SPEC-393 A1, A4 to A7, A9, A19) each build a collection of their own holding
//! one note of a copy of a stock note type: two templates for the ordinal, a CSS that names fonts
//! for the font pass, and a template that speaks through TTS tags for the voices. A reader that
//! asks for fonts and one that keeps the trait's own answer read from memory.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what an \
              enumerating test examined"
)]

mod support;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anki::collection::{Collection, CollectionBuilder};
use anki::deckconfig::{DeckConfig, DeckConfigInner, UpdateDeckConfigsRequest};
use anki::decks::DeckId;
use anki::notetype::{Notetype, NotetypeId};
use anki_proto::collection::OpenCollectionRequest;
use anki_proto::deck_config::UpdateDeckConfigsMode;
use anki_proto::deck_config::deck_configs_for_update::current_deck::Limits;
use anki_proto::decks::deck::filtered::SearchTerm;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::face::{Clip, Face, Side};
use deck_streak_engine_core::media::{self, Reader};
use deck_streak_engine_core::table::Transport;
use prost::Message;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);

/// The image, its bytes (a PNG signature) and their base64, worked out by hand.
const DOT: &str = "dot.png";
const DOT_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n";
const DOT_URL: &str = "data:image/png;base64,iVBORw0KGgo=";
/// The question's sound and the answer's sound.
const TONE: &str = "tone.wav";
const TONE_BYTES: &[u8] = b"RIFF tone";
const ANSWER: &str = "answer.mp3";
const ANSWER_BYTES: &[u8] = b"ID3 answer";

/// A front that plays a sound, and a back with its own: the stock Basic answer template opens with
/// `{{FrontSide}}`.
const SOUND_FRONT: &str = "[sound:tone.wav]";
const SOUND_BACK: &str = "the back[sound:answer.mp3]";
/// A front with an image, a sound and two TTS tags: one whose speed scales, one whose speed is
/// past the platform's maximum.
const FULL_FRONT: &str = concat!(
    r#"<img src="dot.png" alt="a dot">[sound:tone.wav]"#,
    "[anki:tts lang=en_US speed=1.5]a <b>bold</b> word[/anki:tts]",
    "[anki:tts lang=fr_FR speed=3]vite[/anki:tts]",
);
const FULL_BACK: &str = "the back";
/// A front whose every reference but five the rules refuse: a path, `..`, an unknown type, a
/// missing file twice, a file one byte over its cap, a remote URL, an escaped separator, an escape
/// that is not UTF-8; then a percent-escaped name, four files that fill the face to its cap exactly,
/// and one byte past it.
const REFUSALS_FRONT: &str = concat!(
    r#"<img src="sub/dot.png"><img src=".."><img src="notes.txt">"#,
    r#"<img src="missing.png"><img src="missing.png"><img src="big.png">"#,
    r#"<img src="https://example.invalid/x.png"><img src="sub%2Fdot.png"><img src="%ff.png">"#,
    r#"<img src="a%20dot.png"><img src="cap-a.png"><img src="cap-b.png"><img src="cap-c.png">"#,
    r#"<img src="cap-d.png"><img src="one.png">"#,
);

/// Four MiB, the per-file cap, and sixteen, the per-face cap, written out rather than read from the
/// core.
const FOUR_MIB: usize = 4 * 1024 * 1024;

/// The cards the fixture holds, by what each test asks of them.
struct Cards {
    /// The sound note, on the default preset.
    sound_plain: i64,
    /// The sound note, on the preset that skips the question when replaying the answer.
    sound_skip: i64,
    /// The full note, on the default preset.
    full_plain: i64,
    /// The full note, on a preset with autoplay disabled.
    full_quiet: i64,
    /// The full note, in a filtered deck whose card's home deck has autoplay disabled.
    full_moved: i64,
    /// The refusals note, on the default preset.
    refusals: i64,
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

/// Gives `deck` a new preset of its own, the default one with `change` applied.
fn preset(col: &mut Collection, deck: DeckId, name: &str, change: fn(&mut DeckConfigInner)) {
    let mut config = DeckConfig {
        name: name.to_owned(),
        ..DeckConfig::default()
    };
    change(&mut config.inner);
    col.update_deck_configs(UpdateDeckConfigsRequest {
        target_deck_id: deck,
        configs: vec![config],
        removed_config_ids: Vec::new(),
        mode: UpdateDeckConfigsMode::Normal,
        card_state_customizer: String::new(),
        limits: Limits::default(),
        new_cards_ignore_review_limit: false,
        apply_all_parent_limits: false,
        fsrs: false,
        fsrs_reschedule: false,
        fsrs_health_check: false,
    })
    .expect("the engine gives the deck its preset");
}

fn note(col: &mut Collection, basic: &Notetype, deck: DeckId, front: &str, back: &str) -> i64 {
    let mut note = basic.new_note();
    note.set_field(0, front).expect("the front is set");
    note.set_field(1, back).expect("the back is set");
    col.add_note(&mut note, deck)
        .expect("the engine adds the note");
    col.storage
        .db()
        .query_row("select id from cards where nid = ?", [note.id.0], |row| {
            row.get(0)
        })
        .expect("the note has one card")
}

fn fixture(test: &str) -> Fixture {
    let dir = support::scratch("engine-core-face", test);
    let folder = dir.join("collection.media");
    std::fs::create_dir_all(&folder).expect("a media directory");
    for (name, bytes) in [(DOT, DOT_BYTES), (TONE, TONE_BYTES), (ANSWER, ANSWER_BYTES)] {
        std::fs::write(folder.join(name), bytes).expect("a media file is written");
    }
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    let plain = deck(&mut col, "Plain");
    let skip = deck(&mut col, "Skip");
    let quiet = deck(&mut col, "Quiet");
    let moved = deck(&mut col, "Moved");
    preset(&mut col, skip, "Skip", |config| {
        config.skip_question_when_replaying_answer = true;
    });
    preset(&mut col, quiet, "Quiet", |config| {
        config.disable_autoplay = true;
    });
    preset(&mut col, moved, "Moved", |config| {
        config.disable_autoplay = true;
    });
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let cards = Cards {
        sound_plain: note(&mut col, &basic, plain, SOUND_FRONT, SOUND_BACK),
        sound_skip: note(&mut col, &basic, skip, SOUND_FRONT, SOUND_BACK),
        full_plain: note(&mut col, &basic, plain, FULL_FRONT, FULL_BACK),
        full_quiet: note(&mut col, &basic, quiet, FULL_FRONT, FULL_BACK),
        full_moved: note(&mut col, &basic, moved, FULL_FRONT, FULL_BACK),
        refusals: note(&mut col, &basic, plain, REFUSALS_FRONT, FULL_BACK),
    };
    let mut filtered = col
        .get_or_create_filtered_deck(DeckId(0))
        .expect("the engine drafts a filtered deck");
    "Filtered".clone_into(&mut filtered.human_name);
    filtered.config.search_terms = vec![SearchTerm {
        search: "deck:Moved".to_owned(),
        limit: 100,
        order: 0,
    }];
    col.add_or_update_filtered_deck(filtered)
        .expect("the engine builds the filtered deck");
    let home: i64 = col
        .storage
        .db()
        .query_row(
            "select odid from cards where id = ?",
            [cards.full_moved],
            |row| row.get(0),
        )
        .expect("the moved card is read");
    assert_eq!(home, moved.0, "the moved card sits in the filtered deck");
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
    open_at(&fixture.dir, &fixture.collection)
}

/// A native dispatcher with the collection at `collection`, in `dir`, open.
fn open_at(dir: &Path, collection: &Path) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let request = OpenCollectionRequest {
        collection_path: text(collection),
        media_folder_path: text(&dir.join("collection.media")),
        media_db_path: text(&dir.join("collection.media.db")),
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

/// Reads from memory, at most `limit` bytes.
struct Memory(HashMap<&'static str, Vec<u8>>);

impl Reader for Memory {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let bytes = self.0.get(name)?;
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        Some(bytes[..bytes.len().min(limit)].to_vec())
    }
}

fn face(dispatcher: &Dispatcher, card: i64, side: Side, autoplay: bool) -> Face {
    let folder = Folder(dispatcher.media_folder().map(PathBuf::from));
    dispatcher
        .face(card, side, autoplay, &folder)
        .expect("the engine renders the card")
}

fn sound(name: &str, bytes: &[u8]) -> Clip {
    Clip::Sound {
        name: name.to_owned(),
        bytes: bytes.to_vec(),
    }
}

fn speech(text: &str, language: &str, rate: f32, voices: &[&str]) -> Clip {
    Clip::Speech {
        text: text.to_owned(),
        language: language.to_owned(),
        rate,
        voices: voices.iter().map(|&voice| voice.to_owned()).collect(),
    }
}

/// The full note's question clips: its sound, then its two TTS tags in the platform's terms.
fn full_question_clips() -> Vec<Clip> {
    vec![
        sound(TONE, TONE_BYTES),
        speech("a bold word", "en-US", 0.75, &[]),
        speech("vite", "fr-FR", 1.0, &[]),
    ]
}

/// Every `src` attribute's value in `text`, in order.
fn sources(text: &str) -> Vec<&str> {
    text.split(r#"src=""#)
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or_default())
        .collect()
}

/// The `data:` URL of a run of zero bytes whose length is one more than a multiple of three: each
/// three zero bytes are `AAAA` and the last one is `AA==`.
fn zeros_url(groups: usize) -> String {
    format!("data:image/png;base64,{}AA==", "AAAA".repeat(groups))
}

#[test]
fn the_answer_replays_the_question_unless_the_preset_skips_it() {
    let fixture = fixture("answer-replay");
    let dispatcher = open(&fixture);

    let plain = face(&dispatcher, fixture.cards.sound_plain, Side::Answer, true);
    assert_eq!(
        plain.replay,
        vec![sound(TONE, TONE_BYTES), sound(ANSWER, ANSWER_BYTES)],
        "the answer replays the question's clip, then its own"
    );
    assert_eq!(
        plain.autoplay,
        vec![sound(ANSWER, ANSWER_BYTES)],
        "the answer autoplays its own clip alone"
    );

    let skip = face(&dispatcher, fixture.cards.sound_skip, Side::Answer, true);
    assert_eq!(
        skip.replay,
        vec![sound(ANSWER, ANSWER_BYTES)],
        "the preset skips the question when replaying the answer"
    );
    assert_eq!(skip.autoplay, vec![sound(ANSWER, ANSWER_BYTES)]);

    let question = face(&dispatcher, fixture.cards.sound_plain, Side::Question, true);
    assert_eq!(question.replay, vec![sound(TONE, TONE_BYTES)]);
    assert_eq!(question.autoplay, vec![sound(TONE, TONE_BYTES)]);
}

#[test]
fn a_face_inlines_its_image_and_strips_its_av_tags() {
    let fixture = fixture("image-and-tags");
    let dispatcher = open(&fixture);

    for side in [Side::Question, Side::Answer] {
        let face = face(&dispatcher, fixture.cards.full_plain, side, true);
        assert_eq!(
            sources(&face.text),
            vec![DOT_URL],
            "{side:?}: the image's src is the data: URL of the file's bytes"
        );
        for tag in ["[sound:", "[anki:tts", "[anki:play"] {
            assert!(
                !face.text.contains(tag),
                "{side:?}: the text holds {tag}: {}",
                face.text
            );
        }
        assert!(
            face.text.contains(r#"alt="a dot""#),
            "{side:?}: the image keeps its alt text"
        );
        assert_eq!(
            face.omitted,
            Vec::<String>::new(),
            "{side:?}: nothing omitted"
        );
    }
    let answer = face(&dispatcher, fixture.cards.full_plain, Side::Answer, true);
    assert!(
        answer.text.ends_with("the back"),
        "the answer shows its back after the question: {}",
        answer.text
    );
}

#[test]
fn a_reference_the_rules_refuse_is_omitted_and_emptied() {
    let fixture = fixture("refusals");
    let dispatcher = open(&fixture);
    let reader = Memory(HashMap::from([
        ("sub/dot.png", DOT_BYTES.to_vec()),
        ("..", DOT_BYTES.to_vec()),
        ("notes.txt", b"notes".to_vec()),
        ("big.png", vec![0; FOUR_MIB + 1]),
        ("a dot.png", b"dot".to_vec()),
        ("cap-a.png", vec![0; FOUR_MIB]),
        ("cap-b.png", vec![0; FOUR_MIB]),
        ("cap-c.png", vec![0; FOUR_MIB]),
        ("cap-d.png", vec![0; FOUR_MIB - 3]),
        ("one.png", vec![0; 1]),
    ]));
    let face = dispatcher
        .face(fixture.cards.refusals, Side::Question, true, &reader)
        .expect("the engine renders the card");

    let four_mib = zeros_url(1_398_101);
    let sources = sources(&face.text);
    assert_eq!(
        sources,
        vec![
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "data:image/png;base64,ZG90",
            four_mib.as_str(),
            four_mib.as_str(),
            four_mib.as_str(),
            zeros_url(1_398_100).as_str(),
            "",
        ],
        "each refused reference's attribute is emptied, each admitted one inlined"
    );
    assert!(
        sources
            .iter()
            .all(|source| source.is_empty() || source.starts_with("data:")),
        "no URL other than data:"
    );
    assert_eq!(
        face.omitted,
        vec![
            "sub/dot.png",
            "..",
            "notes.txt",
            "missing.png",
            "big.png",
            "https://example.invalid/x.png",
            "sub%2Fdot.png",
            "%ff.png",
            "one.png",
        ],
        "each refused name is omitted once"
    );

    // An empty name and `..` never reach a rewrite as a plain file name with a known type, so the
    // name rule is asked directly.
    for refused in ["", ".", "..", "sub/dot.png", "sub\\dot.png", "dot\0.png"] {
        assert!(!media::plain_name(refused), "{refused:?} is refused");
    }
    assert!(
        media::plain_name("dot.png"),
        "a plain file name is admitted"
    );
}

#[test]
fn a_tts_tag_becomes_speech_in_the_platforms_terms() {
    let fixture = fixture("speech");
    let dispatcher = open(&fixture);
    let face = face(&dispatcher, fixture.cards.full_plain, Side::Question, true);

    let speech_clips: Vec<&Clip> = face
        .replay
        .iter()
        .filter(|clip| matches!(clip, Clip::Speech { .. }))
        .collect();
    assert_eq!(
        speech_clips,
        vec![
            &speech("a bold word", "en-US", 0.75, &[]),
            &speech("vite", "fr-FR", 1.0, &[]),
        ],
        "each TTS tag is plain text, a hyphenated language, and a rate scaled and held"
    );
    assert_eq!(face.replay, full_question_clips());
}

#[test]
fn autoplay_follows_the_preset_and_the_client() {
    let fixture = fixture("autoplay");
    let dispatcher = open(&fixture);
    let cases = [
        (
            "the default preset, wished",
            fixture.cards.full_plain,
            true,
            true,
        ),
        (
            "the default preset, not wished",
            fixture.cards.full_plain,
            false,
            false,
        ),
        (
            "autoplay disabled, wished",
            fixture.cards.full_quiet,
            true,
            false,
        ),
        (
            "a filtered card, home deck disabled",
            fixture.cards.full_moved,
            true,
            false,
        ),
    ];
    for (case, card, wished, plays) in cases {
        let face = face(&dispatcher, card, Side::Question, wished);
        assert_eq!(
            face.replay,
            full_question_clips(),
            "{case}: the replay is full"
        );
        let expected = if plays {
            full_question_clips()
        } else {
            Vec::new()
        };
        assert_eq!(face.autoplay, expected, "{case}: the autoplay");
    }
}

#[test]
fn a_field_the_frontend_completes_keeps_its_own_text_beside_the_front_side() {
    let dir = support::scratch("engine-core-face", "typed-answer");
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    let plain = deck(&mut col, "Plain");
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let mut typed = Notetype::clone(&basic);
    typed.id = NotetypeId(0);
    "Basic typed".clone_into(&mut typed.name);
    "{{FrontSide}}<hr id=answer>{{custom:Back}}"
        .clone_into(&mut typed.templates[0].config.a_format);
    col.add_notetype(&mut typed, false)
        .expect("the engine adds the note type");
    let card = note(&mut col, &typed, plain, "front words", "back words");
    col.close(None).expect("the engine closes the collection");

    let dispatcher = open_at(&dir, &collection);
    let answer = face(&dispatcher, card, Side::Answer, true);
    assert_eq!(
        answer.text.matches("front words").count(),
        1,
        "the front side stands once: {}",
        answer.text
    );
    assert!(
        answer.text.contains("back words"),
        "a field other than the front side keeps its own text: {}",
        answer.text
    );
}

#[test]
fn a_percent_escape_decodes_to_its_byte() {
    assert_eq!(media::decoded_name("a%41b").as_deref(), Some("aAb"));
    assert_eq!(
        media::decoded_name("%e2%9C%93.png").as_deref(),
        Some("\u{2713}.png")
    );
    assert_eq!(media::decoded_name("100%.png").as_deref(), Some("100%.png"));
    assert_eq!(media::decoded_name("%4").as_deref(), Some("%4"));
    assert_eq!(media::decoded_name("%ff"), None);
}

/// A collection of the test's own holding one note, `front` and `back`, of a copy of the engine's
/// stock note type `stock` with `change` applied, in the deck `Plain`: a dispatcher with it open,
/// and the note's cards' ids by template.
fn one_note(
    test: &str,
    stock: &str,
    change: impl FnOnce(&mut Notetype),
    front: &str,
    back: &str,
) -> (Dispatcher, Vec<i64>) {
    let dir = support::scratch("engine-core-face", test);
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    let plain = deck(&mut col, "Plain");
    let stock = col
        .get_notetype_by_name(stock)
        .expect("the note types are read")
        .expect("the engine creates its stock note type");
    let mut copy = Notetype::clone(&stock);
    copy.id = NotetypeId(0);
    copy.name.push_str(" copy");
    change(&mut copy);
    col.add_notetype(&mut copy, false)
        .expect("the engine adds the note type");
    let mut note = copy.new_note();
    note.set_field(0, front).expect("the front is set");
    note.set_field(1, back).expect("the back is set");
    col.add_note(&mut note, plain)
        .expect("the engine adds the note");
    let cards = {
        let mut statement = col
            .storage
            .db()
            .prepare("select id from cards where nid = ? order by ord")
            .expect("the cards are read");
        statement
            .query_map([note.id.0], |row| row.get(0))
            .expect("the cards are read")
            .collect::<Result<Vec<i64>, _>>()
            .expect("each card's id is read")
    };
    col.close(None).expect("the engine closes the collection");
    (open_at(&dir, &collection), cards)
}

/// A change that gives a note type the CSS `css`.
fn styled(css: &'static str) -> impl FnOnce(&mut Notetype) {
    move |notetype| css.clone_into(&mut notetype.config.css)
}

/// Reads from memory like [`Memory`], and asks for the CSS's fonts.
struct Asking(Memory);

impl Reader for Asking {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        self.0.read(name, limit)
    }

    fn inlines_fonts(&self) -> bool {
        true
    }
}

/// Reads from memory like [`Memory`], records every name it is asked for, and keeps the trait's
/// own answer to whether it inlines fonts.
struct Recording {
    memory: Memory,
    asked: RefCell<Vec<String>>,
}

impl Reader for Recording {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        self.asked.borrow_mut().push(name.to_owned());
        self.memory.read(name, limit)
    }
}

/// The font the parity tests' CSS names, and its bytes.
const FONT: &str = "_parity.ttf";
const FONT_BYTES: &[u8] = b"parity-font";
/// The CSS of the font tests: the font named in double quotes, and again under `URL(` in capitals
/// with spaces and single quotes, beside a rule that names no URL.
const FONT_CSS: &str = concat!(
    "@font-face { font-family: p; src: url(\"_parity.ttf\"); }\n",
    "@font-face { font-family: s; src: URL( '_parity.ttf' ); }\n",
    ".card { font-family: p, s; }\n",
);

/// RED-FIRST (SPEC-393 R1, A1): each face carries its card's template index, read from the card
/// the preset is read from, on both sides.
#[test]
fn a_face_carries_its_cards_template_ordinal() {
    let (dispatcher, cards) = one_note(
        "ordinal",
        "Basic (and reversed card)",
        |_| {},
        "forward",
        "reverse",
    );
    assert_eq!(cards.len(), 2, "the note has a card for each template");
    let ordinals: Vec<u32> = cards
        .iter()
        .flat_map(|&card| [Side::Question, Side::Answer].map(|side| (card, side)))
        .map(|(card, side)| face(&dispatcher, card, side, true).ordinal)
        .collect();
    assert_eq!(
        ordinals,
        vec![0, 0, 1, 1],
        "the forward card's question and answer carry 0, the reverse card's carry 1"
    );
}

/// RED-FIRST (SPEC-393 R3, R4, A4): for a reader that asks, a font the CSS names becomes the
/// `data:` URL of its bytes, however the `url(` is spelt and its argument quoted.
#[test]
fn a_font_the_css_names_is_inlined_for_a_reader_that_asks() {
    let (dispatcher, cards) = one_note("font-inlined", "Basic", styled(FONT_CSS), "styled", "");
    let reader = Asking(Memory(HashMap::from([(FONT, FONT_BYTES.to_vec())])));
    let face = dispatcher
        .face(cards[0], Side::Question, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        face.css,
        concat!(
            "@font-face { font-family: p; src: url(\"data:font/ttf;base64,cGFyaXR5LWZvbnQ=\"); }\n",
            "@font-face { font-family: s; src: url(\"data:font/ttf;base64,cGFyaXR5LWZvbnQ=\"); }\n",
            ".card { font-family: p, s; }\n",
        ),
        "each url( naming the font holds its data: URL, and the rest is as written"
    );
    assert_eq!(face.omitted, Vec::<String>::new(), "nothing omitted");
}

/// RED-FIRST (SPEC-393 R4, A5): a font the rules refuse is emptied and named once, in the order
/// met; a scheme, a `data:` URL, a path and an image name stay as written and are not named.
#[test]
fn a_refused_font_is_emptied_and_named_and_any_other_url_stays() {
    const CSS: &str = concat!(
        "@font-face { font-family: big; src: url(\"big.ttf\"); }\n",
        "@font-face { font-family: absent; src: url('absent.woff'); }\n",
        "@font-face { font-family: again; src: url(big.ttf); }\n",
        "@font-face { font-family: remote; src: url(\"https://example.invalid/r.woff2\"); }\n",
        "@font-face { font-family: inline; src: url(\"data:font/ttf;base64,eA==\"); }\n",
        "@font-face { font-family: nested; src: url(\"fonts/n.otf\"); }\n",
        ".card { background: url(\"dot.png\"); }\n",
    );
    let (dispatcher, cards) = one_note("font-refused", "Basic", styled(CSS), "styled", "");
    let reader = Asking(Memory(HashMap::from([
        ("big.ttf", vec![0; FOUR_MIB + 1]),
        ("fonts/n.otf", b"nested".to_vec()),
        (DOT, DOT_BYTES.to_vec()),
    ])));
    let face = dispatcher
        .face(cards[0], Side::Question, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        face.css,
        concat!(
            "@font-face { font-family: big; src: url(\"\"); }\n",
            "@font-face { font-family: absent; src: url(\"\"); }\n",
            "@font-face { font-family: again; src: url(\"\"); }\n",
            "@font-face { font-family: remote; src: url(\"https://example.invalid/r.woff2\"); }\n",
            "@font-face { font-family: inline; src: url(\"data:font/ttf;base64,eA==\"); }\n",
            "@font-face { font-family: nested; src: url(\"fonts/n.otf\"); }\n",
            ".card { background: url(\"dot.png\"); }\n",
        ),
        "each refused font is emptied, and every other url( stays as written"
    );
    assert_eq!(
        face.omitted,
        vec!["big.ttf", "absent.woff"],
        "each refused font is named once, in the order met, and nothing else is named"
    );
}

/// NOT RED (SPEC-393 R6, A6): for a reader that keeps the trait's own answer, the CSS is the note
/// type's byte for byte and the reader is asked for the card's image alone, never for a font.
#[test]
fn the_css_stays_as_written_for_a_reader_that_does_not_ask() {
    let (dispatcher, cards) = one_note(
        "font-not-asked",
        "Basic",
        styled(FONT_CSS),
        r#"<img src="dot.png">"#,
        "",
    );
    let reader = Recording {
        memory: Memory(HashMap::from([
            (FONT, FONT_BYTES.to_vec()),
            (DOT, DOT_BYTES.to_vec()),
        ])),
        asked: RefCell::new(Vec::new()),
    };
    let face = dispatcher
        .face(cards[0], Side::Question, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        face.css, FONT_CSS,
        "the CSS is the note type's, byte for byte"
    );
    assert_eq!(face.omitted, Vec::<String>::new(), "nothing omitted");
    assert_eq!(
        reader.asked.into_inner(),
        vec![DOT],
        "the reader is asked for the card's image and for no font"
    );
}

/// RED-FIRST (SPEC-393 R5, A7): the font pass takes from the face's budget after the text's media
/// and the clips, so a face that its own media fill to the cap refuses its font.
#[test]
fn a_font_never_crowds_out_the_cards_own_media() {
    let front = concat!(
        r#"<img src="cap-a.png"><img src="cap-b.png"><img src="cap-c.png">"#,
        "[sound:cap.mp3]",
    );
    let css = "@font-face { font-family: one; src: url(\"_one.ttf\"); }\n";
    let (dispatcher, cards) = one_note("font-last", "Basic", styled(css), front, "");
    let reader = Asking(Memory(HashMap::from([
        ("cap-a.png", vec![0; FOUR_MIB]),
        ("cap-b.png", vec![0; FOUR_MIB]),
        ("cap-c.png", vec![0; FOUR_MIB]),
        ("cap.mp3", vec![0; FOUR_MIB]),
        ("_one.ttf", vec![0; 1]),
    ])));
    let face = dispatcher
        .face(cards[0], Side::Question, true, &reader)
        .expect("the engine renders the card");

    let four_mib = zeros_url(1_398_101);
    assert_eq!(
        sources(&face.text),
        vec![four_mib.as_str(); 3],
        "the three images are admitted"
    );
    let sounds: Vec<(&str, usize)> = face
        .replay
        .iter()
        .filter_map(|clip| match clip {
            Clip::Sound { name, bytes } => Some((name.as_str(), bytes.len())),
            Clip::Speech { .. } => None,
        })
        .collect();
    assert_eq!(sounds, vec![("cap.mp3", FOUR_MIB)], "the sound is admitted");
    assert_eq!(
        face.css, "@font-face { font-family: one; src: url(\"\"); }\n",
        "the font, one byte past the face's cap, is emptied"
    );
    assert_eq!(face.omitted, vec!["_one.ttf"], "the font alone is named");
}

/// RED-FIRST (SPEC-393 R7, A9): a TTS tag's voices reach its speech clip in the tag's order, and a
/// tag that names none gives none.
#[test]
fn a_tts_tag_carries_its_voices_in_order() {
    let (dispatcher, cards) = one_note(
        "voices",
        "Basic",
        |notetype| {
            "{{tts en_US voices=Absent_Voice,Desk_Parity_Voice:Front}}{{tts fr_FR:Back}}"
                .clone_into(&mut notetype.templates[0].config.q_format);
        },
        "spoken words",
        "mots dits",
    );
    let face = face(&dispatcher, cards[0], Side::Question, true);
    assert_eq!(
        face.replay,
        vec![
            speech(
                "spoken words",
                "en-US",
                0.5,
                &["Absent_Voice", "Desk_Parity_Voice"]
            ),
            speech("mots dits", "fr-FR", 0.5, &[]),
        ],
        "the first tag's clip carries its two voices in order, the second's none"
    );
}

/// RED-FIRST (SPEC-393 R3, A19): every entry of the font table is inlined under its own type, its
/// extension read in any ASCII case.
#[test]
fn every_font_type_is_inlined_under_its_own_type() {
    const CSS: &str = concat!(
        "@font-face { font-family: a; src: url(\"_a.ttf\"); }\n",
        "@font-face { font-family: b; src: url(\"_b.OTF\"); }\n",
        "@font-face { font-family: c; src: url(\"_c.woff\"); }\n",
        "@font-face { font-family: d; src: url(\"_d.Woff2\"); }\n",
    );
    let (dispatcher, cards) = one_note("font-types", "Basic", styled(CSS), "styled", "");
    let reader = Asking(Memory(HashMap::from([
        ("_a.ttf", b"font-a".to_vec()),
        ("_b.OTF", b"font-b".to_vec()),
        ("_c.woff", b"font-c".to_vec()),
        ("_d.Woff2", b"font-d".to_vec()),
    ])));
    let face = dispatcher
        .face(cards[0], Side::Question, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        face.css,
        concat!(
            "@font-face { font-family: a; src: url(\"data:font/ttf;base64,Zm9udC1h\"); }\n",
            "@font-face { font-family: b; src: url(\"data:font/otf;base64,Zm9udC1i\"); }\n",
            "@font-face { font-family: c; src: url(\"data:font/woff;base64,Zm9udC1j\"); }\n",
            "@font-face { font-family: d; src: url(\"data:font/woff2;base64,Zm9udC1k\"); }\n",
        ),
        "each font is inlined under its own type, and the rest is as written"
    );
    assert_eq!(face.omitted, Vec::<String>::new(), "nothing omitted");
}
