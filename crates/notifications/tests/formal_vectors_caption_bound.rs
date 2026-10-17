//! SPEC-472 A1 and A2 (R4 to R6; ADR-489 D2 and D5): the photo caption bound answers every vector
//! of `formal/vectors/caption-bound.jsonl`. The vectors are written by
//! `formal/lean/Formal/CaptionBound.lean`'s port and read here through the crate's own code:
//! `photo::check_caption`, `Photo::new` over an image within its bounds, and
//! `Router::prepare_share` with a transport that counts its calls.
//!
//! MUTATION COVERAGE, not red first: the bound is already on dev; rows S47200 to S47206 are its reds.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_kernel::StudyDayRule;
use deck_streak_notifications::photo::check_caption;
use deck_streak_notifications::{
    BotTransport, FileId, Pass, Photo, PhotoError, Prepared, PushFuture, Pushed, Router,
    ShareFuture,
};
use serde_json::Value;
use support::{DAY, Harness, at};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/caption-bound.jsonl");

/// The platform's caption limit, in UTF-16 units: spelt here, never read from the crate.
const LIMIT: usize = 1_024;

/// A transport that counts what reaches it: only a share is answered.
struct Counting {
    shares: AtomicUsize,
}

impl BotTransport for Counting {
    fn push_message<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        Box::pin(std::future::ready(Pushed::Delivered))
    }

    fn prepare_share<'a>(
        &'a self,
        _pass: &'a Pass,
        _file: &'a FileId,
        _caption: &'a str,
    ) -> ShareFuture<'a> {
        self.shares.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::ready(Prepared::Ready {
            id: "synthetic-prepared".to_owned(),
        }))
    }
}

/// A PNG of 64 by 48 pixels: within every bound of the photo but the caption's.
fn image() -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend_from_slice(&13_u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&64_u32.to_be_bytes());
    png.extend_from_slice(&48_u32.to_be_bytes());
    png.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    png.resize(128, 0);
    png
}

/// The caption a vector names: each of its runs, `[code point, count]`, as that character
/// repeated `count` times, in order.
fn caption(vector: &Value) -> String {
    vector["runs"]
        .as_array()
        .unwrap_or_else(|| panic!("the runs of {vector}"))
        .iter()
        .flat_map(|run| {
            let code = run[0].as_u64().expect("a code point");
            let count = run[1].as_u64().expect("a count");
            let character =
                char::from_u32(u32::try_from(code).expect("a code point")).expect("a scalar value");
            std::iter::repeat_n(character, usize::try_from(count).expect("a count"))
        })
        .collect()
}

/// The header and the vectors, each line parsed as JSON.
fn read() -> (Value, Vec<Value>) {
    let mut lines = VECTORS.lines();
    let header = serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    let vectors = lines
        .map(|line| serde_json::from_str(line).expect("a JSON vector"))
        .collect();
    (header, vectors)
}

/// Reports how many members a population holds, and refuses an empty one.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(!items.is_empty(), "examined 0 {what}: nothing was judged");
    items
}

#[tokio::test]
async fn the_caption_bound_answers_every_lean_vector() {
    let (header, vectors) = read();
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "CaptionBound", "{header}");
    assert_eq!(
        header["covers"], "crates/notifications/src/photo.rs",
        "{header}"
    );
    assert_eq!(header["anchor"], "check_caption", "{header}");
    let bot = Arc::new(Counting {
        shares: AtomicUsize::new(0),
    });
    let mut harness = Harness::without_bot(at(DAY, 12, 0)).await;
    harness.router = Router::new(
        Arc::clone(&harness.policy),
        harness.db.clone(),
        harness.clock.clone(),
        StudyDayRule::default(),
    )
    .with_bot(Arc::clone(&bot) as Arc<dyn BotTransport>);
    let file = FileId::new("synthetic-file-id").expect("a file id");
    let (mut taken, mut refused) = (0_u64, 0_u64);
    let mut misjudged = Vec::new();
    for (index, vector) in vectors.iter().enumerate() {
        // The header is the file's first line.
        let line = index + 2;
        assert_eq!(vector["rule"], "caption", "line {line}: {vector}");
        let caption = caption(vector);
        let units = vector["units"].as_u64().expect("a unit count");
        let verdict = vector["takes"].as_bool().expect("a takes flag");
        let expected = if verdict {
            Ok(())
        } else {
            Err(PhotoError::Caption)
        };
        let before = bot.shares.load(Ordering::SeqCst);
        let share = match harness.router.prepare_share(&file, &caption).await {
            Prepared::Ready { .. } => Some(Ok(())),
            Prepared::Refused(error) => Some(Err(error)),
            Prepared::Failed | Prepared::Unsupported => None,
        };
        let called = bot.shares.load(Ordering::SeqCst) - before;
        let ours = (
            caption.encode_utf16().count(),
            check_caption(&caption),
            Photo::new(image(), caption.as_str()).map(|_| ()),
            share,
            called,
        );
        let theirs = (
            usize::try_from(units).expect("a unit count"),
            expected,
            expected,
            Some(expected),
            usize::from(verdict),
        );
        if ours != theirs {
            misjudged.push(format!(
                "line {line}: {vector}: the code answers {ours:?}, the port {theirs:?}"
            ));
        }
        if verdict {
            taken += 1;
        } else {
            refused += 1;
        }
    }
    let vectors = examined(
        &format!("caption vectors, {taken} taken and {refused} refused"),
        vectors,
    );
    assert!(
        misjudged.is_empty(),
        "{} of {} vectors are misjudged, the first {}",
        misjudged.len(),
        vectors.len(),
        misjudged.first().map_or("", String::as_str)
    );
    assert_eq!(
        (taken, refused),
        (37, 18),
        "every caption the writer prints"
    );
    assert_eq!(
        header["vectors"].as_u64(),
        Some(taken + refused),
        "{header}"
    );
}

#[test]
fn the_lean_vectors_hold_each_caption_a_broken_count_misjudges() {
    let (_, vectors) = read();
    let captions: Vec<(String, bool)> = vectors
        .iter()
        .map(|vector| {
            let verdict = vector["takes"].as_bool().expect("a takes flag");
            (caption(vector), verdict)
        })
        .collect();
    let by_characters = captions
        .iter()
        .filter(|(caption, verdict)| *verdict != (caption.chars().count() <= LIMIT))
        .count();
    let by_bytes = captions
        .iter()
        .filter(|(caption, verdict)| *verdict != (caption.len() <= LIMIT))
        .count();
    let at_the_limit = captions
        .iter()
        .filter(|(caption, _)| caption.encode_utf16().count() == LIMIT)
        .count();
    let ending_past_the_plane = captions
        .iter()
        .filter(|(caption, _)| {
            caption
                .chars()
                .next_back()
                .is_some_and(|last| u32::from(last) >= 0x1_0000)
        })
        .count();
    let characters: BTreeSet<char> = captions
        .iter()
        .flat_map(|(caption, _)| caption.chars())
        .collect();
    let captions = examined("caption vectors", captions);
    assert_eq!(
        (
            by_characters,
            by_bytes,
            at_the_limit,
            ending_past_the_plane,
            characters.len()
        ),
        (6, 31, 18, 12, 9),
        "of {} captions: those a count of characters misjudges, those a count of UTF-8 bytes \
         misjudges, those at exactly the limit, those whose last character is past the basic \
         plane, and the distinct characters",
        captions.len()
    );
}
