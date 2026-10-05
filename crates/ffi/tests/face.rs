//! The native face (SPEC-348 R5, A8).
//!
//! The review fixture is built in a scratch directory and opened through the adapter, which keeps
//! the media folder its open request names. The image card's face is asked for twice, by day and
//! by night: its document must be one closed page whose image is the `data:` URL of the file's
//! own bytes, read back here by a decoder of the test's own, and whose body carries the night
//! classes only when asked.

#![allow(clippy::expect_used, reason = "a failed fixture should fail its test")]

#[expect(
    dead_code,
    reason = "this test opens the review fixture, so it reads only the wire helpers"
)]
mod support;

#[path = "support/review.rs"]
mod review;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::engine::Engine;

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
        ("by day", &day, "card"),
        ("by night", &night, "card nightMode night_mode"),
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
