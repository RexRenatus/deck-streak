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
//!
//! The sound video tests (SPEC-410 A1 to A4) each build a collection of their own holding one
//! Basic note twice, on the default preset and on one that skips the question when replaying the
//! answer, whose sound tags name videos and a sound. A reader that asks for video and one that
//! keeps the trait's own answer read from memory.

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
use deck_streak_engine_core::media::{self, Reader, TYPES, VIDEO_TYPES};
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

/// The sound video note (SPEC-410 section 9): a front whose sound tags name an `.mp4` video, a
/// sound and an `.m4v` video, and a back whose sound tag names a `.MOV` video in capitals.
const VIDEO_FRONT: &str = "[sound:sign.mp4][sound:beep.mp3][sound:wave.m4v]";
const VIDEO_BACK: &str = "[sound:turn.MOV]";
/// The sound video note's files and their bytes.
const SIGN: &str = "sign.mp4";
const SIGN_BYTES: &[u8] = b"sign-video";
const BEEP: &str = "beep.mp3";
const BEEP_BYTES: &[u8] = b"beep-sound";
const WAVE: &str = "wave.m4v";
const WAVE_BYTES: &[u8] = b"wave-video";
const TURN: &str = "turn.MOV";
const TURN_BYTES: &[u8] = b"turn-video";
/// Each video's element, written out in full: the type its extension names and its bytes' base64,
/// worked out by hand (SPEC-410 R3).
const SIGN_ELEMENT: &str = concat!(
    r#"<video controls style="display:block;max-width:100%;height:auto;margin:0 auto" "#,
    r#"src="data:video/mp4;base64,c2lnbi12aWRlbw=="></video>"#,
);
const WAVE_ELEMENT: &str = concat!(
    r#"<video controls style="display:block;max-width:100%;height:auto;margin:0 auto" "#,
    r#"src="data:video/mp4;base64,d2F2ZS12aWRlbw=="></video>"#,
);
const TURN_ELEMENT: &str = concat!(
    r#"<video controls style="display:block;max-width:100%;height:auto;margin:0 auto" "#,
    r#"src="data:video/quicktime;base64,dHVybi12aWRlbw=="></video>"#,
);
const SMALL_ELEMENT: &str = concat!(
    r#"<video controls style="display:block;max-width:100%;height:auto;margin:0 auto" "#,
    r#"src="data:video/mp4;base64,c21hbGwtdmlkZW8="></video>"#,
);

/// The cards of a collection holding one note twice: in the deck `Plain`, on the default preset,
/// and in the deck `Skip`, whose preset skips the question when replaying the answer.
struct VideoCards {
    plain: i64,
    skip: i64,
}

/// A collection of the test's own holding the Basic note `front` and `back` twice, as `fixture`
/// holds its sound note: in `Plain` and in `Skip`. A dispatcher with it open, and the two cards.
fn video_notes(test: &str, front: &str, back: &str) -> (Dispatcher, VideoCards) {
    let dir = support::scratch("engine-core-face", test);
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    let plain = deck(&mut col, "Plain");
    let skip = deck(&mut col, "Skip");
    preset(&mut col, skip, "Skip", |config| {
        config.skip_question_when_replaying_answer = true;
    });
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let cards = VideoCards {
        plain: note(&mut col, &basic, plain, front, back),
        skip: note(&mut col, &basic, skip, front, back),
    };
    col.close(None).expect("the engine closes the collection");
    (open_at(&dir, &collection), cards)
}

/// The sound video note's four files, in memory.
fn video_files() -> Memory {
    Memory(HashMap::from([
        (SIGN, SIGN_BYTES.to_vec()),
        (BEEP, BEEP_BYTES.to_vec()),
        (WAVE, WAVE_BYTES.to_vec()),
        (TURN, TURN_BYTES.to_vec()),
    ]))
}

/// Reads from memory like [`Memory`], and asks for a sound tag's video (SPEC-410 R2).
struct Showing(Memory);

impl Reader for Showing {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        self.0.read(name, limit)
    }

    fn shows_video(&self) -> bool {
        true
    }
}

/// `text` from its first `<video` on, or nothing when it holds none: the elements a reader that
/// asks gains after the side's own text.
fn videos(text: &str) -> &str {
    text.find("<video").map_or("", |start| &text[start..])
}

/// RED-FIRST (SPEC-410 R1, R3, R6, A1): for a reader that asks, the question's sound tags that name
/// a video become elements after its text, in the tags' order, and only the sound is played.
#[test]
fn a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks() {
    let (dispatcher, cards) = video_notes("video-question", VIDEO_FRONT, VIDEO_BACK);
    let face = dispatcher
        .face(cards.plain, Side::Question, true, &Showing(video_files()))
        .expect("the engine renders the card");
    assert_eq!(
        videos(&face.text),
        format!("{SIGN_ELEMENT}{WAVE_ELEMENT}"),
        "the question's text ends with sign's element, then wave's: {}",
        face.text
    );
    assert_eq!(
        face.replay,
        vec![sound(BEEP, BEEP_BYTES)],
        "the question replays its one sound"
    );
    assert_eq!(
        face.autoplay,
        vec![sound(BEEP, BEEP_BYTES)],
        "the question starts its one sound alone"
    );
    assert_eq!(face.omitted, Vec::<String>::new(), "nothing omitted");
}

/// RED-FIRST (SPEC-410 R1, R3, A2): the answer shows the question's videos, then its own, typed by
/// its extension in any case; when the preset skips the question on replay, its own alone.
#[test]
fn the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips() {
    let (dispatcher, cards) = video_notes("video-answer", VIDEO_FRONT, VIDEO_BACK);
    let reader = Showing(video_files());
    let plain = dispatcher
        .face(cards.plain, Side::Answer, true, &reader)
        .expect("the engine renders the card");
    let skip = dispatcher
        .face(cards.skip, Side::Answer, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        videos(&plain.text),
        format!("{SIGN_ELEMENT}{WAVE_ELEMENT}{TURN_ELEMENT}"),
        "the answer ends with the question's videos, then its own: {}",
        plain.text
    );
    assert_eq!(
        videos(&skip.text),
        TURN_ELEMENT,
        "the preset that skips the question on replay leaves the answer's own video alone: {}",
        skip.text
    );
    assert_eq!(
        plain.replay,
        vec![sound(BEEP, BEEP_BYTES)],
        "the answer replays the question's one sound"
    );
    assert_eq!(
        skip.replay,
        Vec::<Clip>::new(),
        "the skipping preset replays nothing"
    );
    assert_eq!(
        (plain.omitted, skip.omitted),
        (Vec::<String>::new(), Vec::<String>::new()),
        "nothing omitted"
    );
}

/// RED-FIRST (SPEC-410 R4, A3): a video one byte over the file cap adds no element and is named
/// once, and the video within the cap is written alone.
#[test]
fn a_video_over_the_file_cap_is_omitted_and_named_once() {
    let (dispatcher, cards) = video_notes("video-cap", "[sound:large.mp4][sound:small.mp4]", "");
    let reader = Showing(Memory(HashMap::from([
        ("large.mp4", vec![0; 4_194_305]),
        ("small.mp4", b"small-video".to_vec()),
    ])));
    let face = dispatcher
        .face(cards.plain, Side::Question, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        videos(&face.text),
        SMALL_ELEMENT,
        "the question's text ends with small's element alone: {}",
        face.text
    );
    assert_eq!(
        face.omitted,
        vec!["large.mp4".to_owned()],
        "the video over the cap is named once"
    );
    assert_eq!(face.replay, Vec::<Clip>::new(), "nothing is played");
}

/// NOT RED (SPEC-410 R7, A4): for a reader that keeps the trait's own answer, every face is as the
/// base completes it: an `.mp4` sound tag plays as a sound, and an `.m4v` or `.MOV` one is named.
#[test]
fn a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound() {
    let (dispatcher, cards) = video_notes("video-not-asked", VIDEO_FRONT, VIDEO_BACK);
    let reader = video_files();
    let question = dispatcher
        .face(cards.plain, Side::Question, true, &reader)
        .expect("the engine renders the card");
    let answer = dispatcher
        .face(cards.plain, Side::Answer, true, &reader)
        .expect("the engine renders the card");
    assert_eq!(
        question.replay,
        vec![sound(SIGN, SIGN_BYTES), sound(BEEP, BEEP_BYTES)],
        "the question replays sign, then beep, as sounds"
    );
    assert_eq!(
        question.omitted,
        vec![WAVE.to_owned()],
        "the question names wave"
    );
    assert_eq!(
        answer.omitted,
        vec![WAVE.to_owned(), TURN.to_owned()],
        "the answer names wave, then turn"
    );
    for face in [&question, &answer] {
        assert!(
            !face.text.contains("<video"),
            "the text holds no element: {}",
            face.text
        );
    }
}

/// What a sound tag's plain spelling presents (SPEC-464 section 3).
#[derive(Clone, Copy)]
enum Shown {
    /// Refused before any read, and named once in `omitted` as written.
    Refused,
    /// Read, then replayed and autoplayed as a sound over the file's bytes, named as written.
    Sound,
    /// Read, then written after the side's text as a video element of this type over this base64.
    Element(&'static str, &'static str),
}

/// One extension of `TYPES` or `VIDEO_TYPES`: its plain spelling, the spelling with its dot
/// escaped, the spelling with its extension's first letter escaped, and what the plain spelling
/// presents for a reader that asks for video and for one that does not. Each file's bytes are its
/// plain name's.
type Spelt = (&'static str, &'static str, &'static str, Shown, Shown);

/// Every extension of `TYPES` and `VIDEO_TYPES` once, written out by hand (SPEC-464 R2).
const SPELLINGS: [Spelt; 18] = [
    (
        "x.png",
        "x%2Epng",
        "x.%70ng",
        Shown::Refused,
        Shown::Refused,
    ),
    (
        "x.jpg",
        "x%2Ejpg",
        "x.%6Apg",
        Shown::Refused,
        Shown::Refused,
    ),
    (
        "x.jpeg",
        "x%2Ejpeg",
        "x.%6Apeg",
        Shown::Refused,
        Shown::Refused,
    ),
    (
        "x.gif",
        "x%2Egif",
        "x.%67if",
        Shown::Refused,
        Shown::Refused,
    ),
    (
        "x.webp",
        "x%2Ewebp",
        "x.%77ebp",
        Shown::Refused,
        Shown::Refused,
    ),
    (
        "x.svg",
        "x%2Esvg",
        "x.%73vg",
        Shown::Refused,
        Shown::Refused,
    ),
    (
        "x.avif",
        "x%2Eavif",
        "x.%61vif",
        Shown::Refused,
        Shown::Refused,
    ),
    ("x.mp3", "x%2Emp3", "x.%6Dp3", Shown::Sound, Shown::Sound),
    ("x.m4a", "x%2Em4a", "x.%6D4a", Shown::Sound, Shown::Sound),
    ("x.aac", "x%2Eaac", "x.%61ac", Shown::Sound, Shown::Sound),
    ("x.ogg", "x%2Eogg", "x.%6Fgg", Shown::Sound, Shown::Sound),
    ("x.oga", "x%2Eoga", "x.%6Fga", Shown::Sound, Shown::Sound),
    ("x.opus", "x%2Eopus", "x.%6Fpus", Shown::Sound, Shown::Sound),
    ("x.wav", "x%2Ewav", "x.%77av", Shown::Sound, Shown::Sound),
    ("x.webm", "x%2Ewebm", "x.%77ebm", Shown::Sound, Shown::Sound),
    (
        "x.mp4",
        "x%2Emp4",
        "x.%6Dp4",
        Shown::Element("video/mp4", "eC5tcDQ="),
        Shown::Sound,
    ),
    (
        "x.m4v",
        "x%2Em4v",
        "x.%6D4v",
        Shown::Element("video/mp4", "eC5tNHY="),
        Shown::Refused,
    ),
    (
        "x.mov",
        "x%2Emov",
        "x.%6Dov",
        Shown::Element("video/quicktime", "eC5tb3Y="),
        Shown::Refused,
    ),
];

/// A front whose sound tags spell, with escapes, names `admit` refuses beside both caps: a name that
/// does not decode, names that decode to a separator, a backslash, a NUL or a dot name, a file past
/// the file cap under both spellings, a file the reader lacks, and five sounds, the fifth past the
/// face cap (SPEC-464 R3).
const ESCAPED_REFUSALS_FRONT: &str = concat!(
    "[sound:%FF.mp4][sound:sub%2Fx.mp4][sound:sub%5Cx.mp4][sound:x%00.mp4][sound:%2E%2E]",
    "[sound:large%2Emp4][sound:large.mp4][sound:gone%2Emp4]",
    "[sound:f1%2Emp3][sound:f2%2Emp3][sound:f3%2Emp3][sound:f4%2Emp3][sound:f5%2Emp3]",
);

/// What one side presents to one reader: the elements its text ends with, the clips it replays and
/// autoplays, the names it omits, and every name its reader was asked for, in order.
struct Presented {
    elements: String,
    replay: Vec<Clip>,
    autoplay: Vec<Clip>,
    omitted: Vec<String>,
    asked: Vec<String>,
}

/// Reads from memory like [`Memory`], asks for a sound tag's video when `asks` holds, and records
/// every name it is asked for.
struct VideoRecording {
    memory: Memory,
    asks: bool,
    asked: RefCell<Vec<String>>,
}

impl Reader for VideoRecording {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        self.asked.borrow_mut().push(name.to_owned());
        self.memory.read(name, limit)
    }

    fn shows_video(&self) -> bool {
        self.asks
    }
}

/// The files `SPELLINGS` names, each holding its plain name's bytes.
fn spelt_files() -> Memory {
    Memory(
        SPELLINGS
            .iter()
            .map(|spelt| (spelt.0, spelt.0.as_bytes().to_vec()))
            .collect(),
    )
}

/// The files `ESCAPED_REFUSALS_FRONT` reaches: the names a separator, a backslash or a NUL
/// decode to, a file one byte past the file cap, four files at the file cap and one more byte.
fn refusal_files() -> Memory {
    Memory(HashMap::from([
        ("sub/x.mp4", b"hostile".to_vec()),
        ("sub\\x.mp4", b"hostile".to_vec()),
        ("x\0.mp4", b"hostile".to_vec()),
        ("large.mp4", vec![0; 4_194_305]),
        ("f1.mp3", vec![0; 4_194_304]),
        ("f2.mp3", vec![0; 4_194_304]),
        ("f3.mp3", vec![0; 4_194_304]),
        ("f4.mp3", vec![0; 4_194_304]),
        ("f5.mp3", b"f".to_vec()),
    ]))
}

/// A sound tag's video element as SPEC-410 R3 writes it, over a type and a base64 written by hand.
fn element(media_type: &str, base64: &str) -> String {
    [
        "<video controls ",
        "style=\"display:block;max-width:100%;height:auto;margin:0 auto\" ",
        "src=\"data:",
        media_type,
        ";base64,",
        base64,
        "\"></video>",
    ]
    .concat()
}

/// The question side of a note whose front is `front`, read from `files`, as presented to a reader
/// that asks for video and then to one that does not.
fn presented(scratch: &str, front: &str, files: fn() -> Memory) -> [Presented; 2] {
    let (dispatcher, cards) = video_notes(scratch, front, "");
    [true, false].map(|asks| {
        let reader = VideoRecording {
            memory: files(),
            asks,
            asked: RefCell::new(Vec::new()),
        };
        let face = dispatcher
            .face(cards.plain, Side::Question, true, &reader)
            .expect("the engine renders the card");
        let elements = face
            .text
            .find("<video ")
            .map_or("", |at| &face.text[at..])
            .to_owned();
        Presented {
            elements,
            replay: face.replay,
            autoplay: face.autoplay,
            omitted: face.omitted,
            asked: reader.asked.into_inner(),
        }
    })
}

/// What the hand table says a side presents when its sound tags spell each name by `spell`, for a
/// reader that asks for video (`asks`) or one that does not.
fn expected(spell: fn(&Spelt) -> &'static str, asks: bool) -> Presented {
    let mut want = Presented {
        elements: String::new(),
        replay: Vec::new(),
        autoplay: Vec::new(),
        omitted: Vec::new(),
        asked: Vec::new(),
    };
    for spelt in &SPELLINGS {
        let written = spell(spelt);
        let sound = || Clip::Sound {
            name: written.to_owned(),
            bytes: spelt.0.as_bytes().to_vec(),
        };
        let shown = if asks { spelt.3 } else { spelt.4 };
        match shown {
            Shown::Refused => want.omitted.push(written.to_owned()),
            Shown::Sound => {
                want.replay.push(sound());
                want.autoplay.push(sound());
                want.asked.push(spelt.0.to_owned());
            }
            Shown::Element(media_type, base64) => {
                want.elements.push_str(&element(media_type, base64));
                want.asked.push(spelt.0.to_owned());
            }
        }
    }
    want
}

/// The names of `clips`, for a failure message that never prints a file's bytes.
fn clip_names(clips: &[Clip]) -> Vec<String> {
    clips
        .iter()
        .map(|clip| {
            if let Clip::Sound { name, .. } = clip {
                name.clone()
            } else {
                "not a sound".to_owned()
            }
        })
        .collect()
}

/// Holds `got` to `want` field by field, naming `what` in each message.
fn assert_presents(got: &Presented, want: &Presented, what: &str) {
    assert_eq!(
        got.elements, want.elements,
        "{what}: the elements after the side's text"
    );
    assert!(
        got.replay == want.replay,
        "{what}: replays {:?}, not {:?}",
        clip_names(&got.replay),
        clip_names(&want.replay)
    );
    assert!(
        got.autoplay == want.autoplay,
        "{what}: autoplays {:?}, not {:?}",
        clip_names(&got.autoplay),
        clip_names(&want.autoplay)
    );
    assert_eq!(
        got.omitted, want.omitted,
        "{what}: the names omitted, as written"
    );
    assert_eq!(
        got.asked, want.asked,
        "{what}: the names its reader was asked for"
    );
}

/// Checks the hand table against both tables and its own spellings, and the element helper against
/// SPEC-410's own pin; returns how many extensions it examined.
fn examined_spellings() -> usize {
    let mut tables: Vec<&str> = TYPES
        .iter()
        .chain(VIDEO_TYPES.iter())
        .map(|(extension, _)| *extension)
        .collect();
    tables.sort_unstable();
    tables.dedup();
    let mut named = Vec::new();
    for spelt in &SPELLINGS {
        let (stem, extension) = spelt.0.split_once('.').expect("a plain spelling has a dot");
        let mut letters = extension.chars();
        let first = letters.next().expect("an extension has a first letter");
        assert_eq!(
            spelt.1,
            format!("{stem}%2E{extension}"),
            "{}: the dot escaped",
            spelt.0
        );
        assert_eq!(
            spelt.2,
            format!("{stem}.%{:02X}{}", u32::from(first), letters.as_str()),
            "{}: the first letter escaped",
            spelt.0
        );
        named.push(extension);
    }
    named.sort_unstable();
    assert_eq!(
        named, tables,
        "the hand table names every extension of both tables, once"
    );
    assert_eq!(
        element("video/mp4", "c2lnbi12aWRlbw=="),
        SIGN_ELEMENT,
        "the element helper writes SPEC-410 R3's element"
    );
    SPELLINGS.len()
}

/// Drives every extension of both tables through a note spelt plainly and a note spelt by
/// `spell`, for a reader that asks for video and for one that does not, and holds each face to the
/// hand table.
fn presents_as_its_plain_spelling(scratch: &str, spell: fn(&Spelt) -> &'static str, how: &str) {
    assert_eq!(examined_spellings(), 18, "examined 18 of 18 extensions");
    let front = |by: fn(&Spelt) -> &'static str| -> String {
        SPELLINGS
            .iter()
            .flat_map(|spelt| ["[sound:", by(spelt), "]"])
            .collect()
    };
    let plain: fn(&Spelt) -> &'static str = |spelt| spelt.0;
    let [plain_asks, plain_does_not_ask] =
        presented(&format!("{scratch}-plain"), &front(plain), spelt_files);
    assert_presents(
        &plain_asks,
        &expected(plain, true),
        "a reader that asks, the plain spelling",
    );
    assert_presents(
        &plain_does_not_ask,
        &expected(plain, false),
        "a reader that does not ask, the plain spelling",
    );
    let [asks, does_not_ask] = presented(scratch, &front(spell), spelt_files);
    assert_presents(
        &asks,
        &expected(spell, true),
        &format!("a reader that asks, {how}"),
    );
    assert_presents(
        &does_not_ask,
        &expected(spell, false),
        &format!("a reader that does not ask, {how}"),
    );
}

/// RED-FIRST (SPEC-464 R1, R2, A1): over every extension of both tables, a sound tag whose dot is
/// escaped presents as its plain spelling does, for a reader that asks for video and for one that
/// does not: the same elements, the same clips under the name as written, the same names omitted
/// and the same files read.
#[test]
fn an_escaped_dot_presents_as_its_plain_spelling_over_the_whole_type_table() {
    presents_as_its_plain_spelling("escaped-dot", |spelt| spelt.1, "the dot escaped");
}

/// RED-FIRST (SPEC-464 R1, R2, A2): as A1, for a sound tag whose extension's first letter is
/// escaped.
#[test]
fn an_escaped_extension_letter_presents_as_its_plain_spelling_over_the_whole_type_table() {
    presents_as_its_plain_spelling(
        "escaped-letter",
        |spelt| spelt.2,
        "the extension's first letter escaped",
    );
}

/// MUTATION COVERAGE (SPEC-464 R3, A3): names `admit` refuses stay refused when spelt with
/// escapes, under both caps, for a reader that asks for video and for one that does not, and only a
/// name that passes the name and type rules is read.
#[test]
fn escaped_names_admit_refuses_stay_refused_under_both_caps_for_both_readers() {
    let sounds = || -> Vec<Clip> {
        ["f1%2Emp3", "f2%2Emp3", "f3%2Emp3", "f4%2Emp3"]
            .into_iter()
            .map(|name| Clip::Sound {
                name: name.to_owned(),
                bytes: vec![0; 4_194_304],
            })
            .collect()
    };
    let owned =
        |names: &[&str]| -> Vec<String> { names.iter().copied().map(str::to_owned).collect() };
    let want = || Presented {
        elements: String::new(),
        replay: sounds(),
        autoplay: sounds(),
        omitted: owned(&[
            "%FF.mp4",
            "sub%2Fx.mp4",
            "sub%5Cx.mp4",
            "x%00.mp4",
            "%2E%2E",
            "large%2Emp4",
            "large.mp4",
            "gone%2Emp4",
            "f5%2Emp3",
        ]),
        asked: owned(&[
            "large.mp4",
            "large.mp4",
            "gone.mp4",
            "f1.mp3",
            "f2.mp3",
            "f3.mp3",
            "f4.mp3",
            "f5.mp3",
        ]),
    };
    let [asks, does_not_ask] = presented("escaped-refusals", ESCAPED_REFUSALS_FRONT, refusal_files);
    assert_presents(&asks, &want(), "a reader that asks");
    assert_presents(&does_not_ask, &want(), "a reader that does not ask");
}
