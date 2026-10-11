//! The native face (SPEC-348 R5, A8).
//!
//! The review fixture is built in a scratch directory and opened through the adapter, which keeps
//! the media folder its open request names. The image card's face is asked for twice, by day and
//! by night: its document must be one closed page whose image is the `data:` URL of the file's
//! own bytes, read back here by a decoder of the test's own, and whose body carries the night
//! classes only when asked.
//!
//! The parity tests (SPEC-393 A2, A8, A13, A15) build the parity collection the same way and ask
//! for its two cards' faces: the speaking card's for its font and its voices, the showing card's
//! for its body class and its videos.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

#[expect(
    dead_code,
    reason = "this test opens the review fixture and the parity collection, so it reads only the \
              wire helpers and the parity builder"
)]
mod support;

#[path = "support/review.rs"]
mod review;

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::engine::Engine;
use deck_streak_ffi::face::Clip;
use support::parity::{self, Cards};

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// What a `data:` URL of a PNG opens with.
const PNG_URL: &str = "data:image/png;base64,";

fn text(path: &Path) -> &str {
    path.to_str().expect("a scratch path is UTF-8")
}

/// Every value of `attribute` in `document`, in order.
fn values<'a>(document: &'a str, attribute: &str) -> Vec<&'a str> {
    document
        .split(&format!("{attribute}=\""))
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or_default())
        .collect()
}

/// The standard base64 alphabet's decoding, written here so the expectation is not the
/// encoder's own.
fn decode(encoded: &str) -> Vec<u8> {
    let sextet = |symbol: u8| -> u32 {
        let value = match symbol {
            b'A'..=b'Z' => symbol - b'A',
            b'a'..=b'z' => symbol - b'a' + 26,
            b'0'..=b'9' => symbol - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("{symbol} is not a base64 symbol"),
        };
        u32::from(value)
    };
    let mut bytes = Vec::new();
    for group in encoded.as_bytes().chunks(4) {
        let symbols: Vec<u8> = group.iter().copied().filter(|&s| s != b'=').collect();
        let word = symbols
            .iter()
            .enumerate()
            .fold(0_u32, |word, (index, &symbol)| {
                word | (sextet(symbol) << (18 - 6 * index))
            });
        let [_, first, second, third] = word.to_be_bytes();
        bytes.extend_from_slice(&[first, second, third][..symbols.len() - 1]);
    }
    bytes
}

#[test]
fn the_native_document_is_one_closed_page() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-native-face")
        .join(format!("{}-{stamp}", std::process::id()));
    let fixture = review::build(&dir).expect("the builder writes the review fixture");
    let media = fixture.dir.join("collection.media");
    let mut request = Vec::new();
    support::wire::put_bytes(&mut request, 1, text(&fixture.collection).as_bytes());
    support::wire::put_bytes(&mut request, 2, text(&media).as_bytes());
    support::wire::put_bytes(
        &mut request,
        3,
        text(&fixture.dir.join("collection.media.db")).as_bytes(),
    );
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, request)
        .expect("the adapter opens the fixture's collection");
    let image = std::fs::read(media.join("dot.png")).expect("the fixture's image is read");

    let day = engine
        .face(fixture.cards.image, false, false, true)
        .expect("the adapter completes the image card's face");
    let night = engine
        .face(fixture.cards.image, false, true, true)
        .expect("the adapter completes the image card's night face");
    for (case, face, classes) in [
        ("by day", &day, "card card1"),
        ("by night", &night, "card card1 nightMode night_mode"),
    ] {
        let document = &face.document;
        let sources = values(document, "src");
        assert_eq!(sources.len(), 1, "{case}: one image: {sources:?}");
        let payload = sources[0].strip_prefix(PNG_URL);
        assert!(
            payload.is_some(),
            "{case}: the image's src is the data: URL of the file, not {:?}",
            sources[0]
        );
        assert_eq!(
            decode(payload.unwrap_or_default()),
            image,
            "{case}: the data: URL holds the file's own bytes"
        );
        assert!(
            document.contains(r#"alt="a grey dot""#),
            "{case}: the image keeps its alt text"
        );
        assert_eq!(
            face.omitted,
            Vec::<String>::new(),
            "{case}: nothing omitted"
        );

        assert!(
            document.starts_with("<!DOCTYPE html>"),
            "{case}: a whole document"
        );
        let lower = document.to_ascii_lowercase();
        for (element, count) in [
            ("<html", 1),
            ("<head", 1),
            ("<body", 1),
            ("<style", 1),
            ("name=\"viewport\"", 1),
            ("<script", 0),
            ("<base", 0),
            ("<link", 0),
            ("<iframe", 0),
        ] {
            assert_eq!(
                lower.matches(element).count(),
                count,
                "{case}: {element} appears {count} time(s)"
            );
        }
        for scheme in ["http:", "https:", "file:", "javascript:", "blob:"] {
            assert!(!lower.contains(scheme), "{case}: the page names {scheme}");
        }
        for attribute in ["src", "href", "data"] {
            for value in values(document, attribute) {
                assert!(
                    value.is_empty() || value.starts_with("data:"),
                    "{case}: {attribute} names a URL that is not data:"
                );
            }
        }
        assert_eq!(
            values(document, "class"),
            [classes],
            "{case}: the body's classes"
        );
    }
}

/// Prints how many `what` the test examined and refuses none: a population that came back empty
/// judged nothing, and every assertion over it would pass.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// An engine with the collection at `collection`, in `dir`, open with its media folder.
fn opened(dir: &Path, collection: &Path) -> Arc<Engine> {
    let mut request = Vec::new();
    support::wire::put_bytes(&mut request, 1, text(collection).as_bytes());
    support::wire::put_bytes(
        &mut request,
        2,
        text(&dir.join("collection.media")).as_bytes(),
    );
    support::wire::put_bytes(
        &mut request,
        3,
        text(&dir.join("collection.media.db")).as_bytes(),
    );
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, request)
        .expect("the adapter opens the collection");
    engine
}

/// The parity collection, built in a scratch directory of the test's own and opened through the
/// adapter, with its two cards' ids.
fn opened_parity(test: &str) -> (Arc<Engine>, Cards) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-native-face")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    let built = parity::build(&dir).expect("the builder writes the parity collection");
    (opened(&built.dir, &built.collection), built.cards)
}

/// Every `url(` argument in `document`, the name matched in any ASCII case: read up to its `)`,
/// trimmed, and stripped of one pair of matching quotes.
fn url_arguments(document: &str) -> Vec<&str> {
    document
        .to_ascii_lowercase()
        .match_indices("url(")
        .map(|(start, _)| {
            let rest = &document[start + "url(".len()..];
            let argument = rest[..rest.find(')').unwrap_or(rest.len())].trim();
            ['"', '\'']
                .into_iter()
                .find_map(|quote| {
                    argument
                        .strip_prefix(quote)
                        .and_then(|inner| inner.strip_suffix(quote))
                })
                .unwrap_or(argument)
        })
        .collect()
}

/// Every `<video` start tag in `document`, the name matched in any ASCII case and ending at
/// whitespace, `/` or `>`, up to and including its `>`.
fn video_start_tags(document: &str) -> Vec<&str> {
    document
        .to_ascii_lowercase()
        .match_indices("<video")
        .filter(|&(start, name)| {
            document[start + name.len()..]
                .starts_with(|next: char| next.is_ascii_whitespace() || next == '/' || next == '>')
        })
        .map(|(start, _)| {
            let rest = &document[start..];
            &rest[..rest.find('>').map_or(rest.len(), |end| end + 1)]
        })
        .collect()
}

/// RED-FIRST (SPEC-393 R2, A2): the body's class names the card's template, counted from one,
/// with the night classes after it only when asked.
#[test]
fn the_body_class_names_the_cards_template() {
    let (engine, cards) = opened_parity("class");
    for (case, night, classes) in [
        ("by day", false, "card card2"),
        ("by night", true, "card card2 nightMode night_mode"),
    ] {
        let face = engine
            .face(cards.shows, false, night, true)
            .expect("the adapter completes the showing card's face");
        assert_eq!(
            values(&face.document, "class"),
            [classes],
            "{case}: the body's classes"
        );
    }
}

/// RED-FIRST (SPEC-393 R6, A8): the native reader asks for fonts, so the note type's font reaches
/// the document as the `data:` URL of its bytes, and no `url(` in it names another URL.
#[test]
fn the_native_document_carries_its_fonts_inline() {
    let (engine, cards) = opened_parity("fonts");
    let face = engine
        .face(cards.speaks, false, false, true)
        .expect("the adapter completes the speaking card's face");
    let document = &face.document;
    assert!(
        document.contains(r#"url("data:font/ttf;base64,cGFyaXR5LWZvbnQ=")"#),
        "the font is the data: URL of its bytes: {document}"
    );
    let arguments = url_arguments(document);
    for argument in &arguments {
        assert!(
            argument.starts_with("data:"),
            "a url( names a URL that is not data: {argument:?}"
        );
    }
    examined("url( arguments", arguments);
    assert_eq!(face.omitted, Vec::<String>::new(), "nothing omitted");
}

/// RED-FIRST (SPEC-393 R7, A13): the speaking card's clip carries the voices its tag asks for, in
/// order, across the FFI.
#[test]
fn the_speech_clip_carries_the_tags_voices() {
    let (engine, cards) = opened_parity("voices");
    let face = engine
        .face(cards.speaks, false, false, true)
        .expect("the adapter completes the speaking card's face");
    assert_eq!(
        face.replay,
        vec![Clip::Speech {
            text: "a parity card".to_owned(),
            language: "en-US".to_owned(),
            rate: 0.5,
            voices: vec!["Absent_Voice".to_owned(), "Desk_Parity_Voice".to_owned()],
        }],
        "the clip carries the tag's two voices in order"
    );
}

/// RED-FIRST (SPEC-393 R11, A15): each video start tag, in either case, plays inline, and a custom
/// element whose name starts with `video` is as the template wrote it.
#[test]
fn a_video_plays_inline_in_the_native_document() {
    let (engine, cards) = opened_parity("video");
    let face = engine
        .face(cards.shows, false, false, true)
        .expect("the adapter completes the showing card's face");
    let document = &face.document;
    let tags = video_start_tags(document);
    assert_eq!(
        tags,
        [
            r#"<video playsinline src="data:audio/mp4;base64,cGFyaXR5LXZpZGVv" controls>"#,
            r#"<VIDEO playsinline controls src="data:audio/mp4;base64,cGFyaXR5LXZpZGVv">"#,
        ],
        "both video start tags carry playsinline after the element's name"
    );
    assert!(
        document.contains("<video-note>a note beside the video</video-note>"),
        "the custom element is as the template wrote it: {document}"
    );
    examined("video start tags of 2", tags);
}
