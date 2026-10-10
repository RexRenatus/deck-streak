//! The native face of an image occlusion card, withheld with one line (SPEC-380 R4, R10, A7, A8).
//!
//! The review fixture's second collection is built in a scratch directory and opened through the
//! adapter: its one card's face, on both sides, must be withheld, its page holding the English line
//! in the card's place and no image, CSS or clip. The first collection's text card keeps its page.
//! Every page is written out here whole, so no expectation is the adapter's own constant.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the examined helper prints its count"
)]

#[expect(
    dead_code,
    reason = "this test opens the review fixture's collections, so it reads only the wire helpers"
)]
mod support;

#[path = "support/occlusion.rs"]
mod occlusion;
#[path = "support/review.rs"]
mod review;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::engine::Engine;
use deck_streak_ffi::face::CardFace;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// A day page's head, as the adapter writes it before the note type's CSS.
const HEAD: &str = concat!(
    "<!DOCTYPE html><html><head><meta charset=\"utf-8\">",
    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
);
/// The line a withheld page holds, as SPEC-380 R6 words it in English.
const LINE: &str = "This image occlusion card cannot be shown here, because this app does not draw its masks. You can still bury or flag it.";
/// The stock Basic note type's CSS, as the engine's own `styling.css` spells it.
const BASIC_CSS: &str = concat!(
    ".card {\n    font-family: arial;\n    font-size: 20px;\n    line-height: 1.5;\n",
    "    text-align: center;\n    color: black;\n    background-color: white;\n}\n",
);

fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A directory of its own for one test under the target's scratch space.
fn scratch(test: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-occlusion")
        .join(format!("{test}-{}-{stamp}", std::process::id()))
}

fn text(path: &Path) -> &str {
    path.to_str().expect("a scratch path is UTF-8")
}

/// The adapter, with the collection at `collection`, in `dir`, open.
fn open(dir: &Path, collection: &Path) -> Arc<Engine> {
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

/// A day face with nothing to play and nothing left out.
fn face(css: &str, body: &str, withheld: bool) -> CardFace {
    CardFace {
        document: format!(
            "{HEAD}<style>{css}</style></head><body class=\"card\">{body}</body></html>"
        ),
        autoplay: Vec::new(),
        replay: Vec::new(),
        omitted: Vec::new(),
        withheld,
    }
}

#[test]
fn an_occlusion_card_face_is_withheld_with_the_line() {
    let fixture =
        occlusion::build(&scratch("a7")).expect("the builder writes the second collection");
    let engine = open(&fixture.dir, &fixture.collection);
    let sides = [false, true];
    for answer in sides {
        let shown = engine
            .face(fixture.card, answer, false, true)
            .expect("the adapter completes the occlusion card's face");
        assert_eq!(
            shown,
            face("", LINE, true),
            "the occlusion card's face (answer: {answer}) is not withheld with the line"
        );
    }
    examined("side(s) of the occlusion card", sides.to_vec());
}

#[test]
fn a_review_card_face_is_not_withheld() {
    let fixture = review::build(&scratch("a8")).expect("the builder writes the review fixture");
    let engine = open(&fixture.dir, &fixture.collection);
    let faces = [
        (false, face(BASIC_CSS, "a text card", false)),
        (
            true,
            face(
                BASIC_CSS,
                "a text card\n\n<hr id=answer>\n\nthe back",
                false,
            ),
        ),
    ];
    for (answer, expected) in &faces {
        let shown = engine
            .face(fixture.cards.text, *answer, false, true)
            .expect("the adapter completes the text card's face");
        assert_eq!(
            &shown, expected,
            "the text card's face (answer: {answer}) is not the page it was"
        );
    }
    examined("side(s) of the text card", faces.to_vec());
}
