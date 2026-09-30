//! The share's caption is bounded as the photo's is (SPEC-132 A17 and A18): every caption the
//! share accepts is within the Bot API's limit and every caption over it is refused before any Bot
//! API call, by the one bound function the photo path uses. The population is generated from
//! character classes at the bound, one below and one past, and from each entity form a caption can
//! carry.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_kernel::StudyDayRule;
use deck_streak_notifications::{
    BotTransport, FileId, Pass, Photo, PhotoError, Prepared, PushFuture, Pushed, Router,
    ShareFuture,
};
use support::{DAY, Harness, at};

/// The Bot API's caption limit, in UTF-16 units.
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

/// The character classes, as (name, one unit of it): ASCII, a BMP letter beyond ASCII, a CJK
/// character, an astral emoji (a surrogate pair), a combining sequence, a ZWJ sequence, a flag, and
/// an emoji with a variation selector.
const CLASSES: [(&str, &str); 8] = [
    ("ascii", "a"),
    ("bmp", "\u{e9}"),
    ("cjk", "\u{65e5}"),
    ("astral", "\u{1f600}"),
    ("combining", "e\u{301}"),
    (
        "zwj",
        "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}",
    ),
    ("flag", "\u{1f1ef}\u{1f1f5}"),
    ("variation", "\u{2764}\u{fe0f}"),
];

/// The lengths, in UTF-16 units, one below the bound, at it, and one past it.
const TARGETS: [usize; 3] = [LIMIT - 1, LIMIT, LIMIT + 1];

/// The entity forms a caption carries as HTML tags, as (open, close).
const TAGS: [(&str, &str); 8] = [
    ("<b>", "</b>"),
    ("<i>", "</i>"),
    ("<code>", "</code>"),
    ("<pre>", "</pre>"),
    ("<tg-spoiler>", "</tg-spoiler>"),
    ("<blockquote>", "</blockquote>"),
    ("<a href=\"https://example.test/x\">", "</a>"),
    ("<tg-emoji emoji-id=\"5368324170671202286\">", "</tg-emoji>"),
];

/// The character references the bot's HTML reads, each as its written form and what it parses to.
const REFERENCES: [(&str, &str); 4] = [
    ("&amp;", "&"),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&quot;", "\""),
];

/// The number of UTF-16 units in `text`.
fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `piece` repeated, then padded with `a`, to exactly `target` UTF-16 units.
fn fill(piece: &str, target: usize) -> String {
    let mut text = piece.repeat(target / units(piece));
    while units(&text) < target {
        text.push('a');
    }
    text
}

/// The units of `caption` as the Bot API measures them: after its tags are read and its character
/// references are decoded.
fn parsed_units(caption: &str) -> usize {
    let mut plain = String::new();
    let mut in_tag = false;
    for c in caption.chars() {
        match c {
            '<' if !in_tag && caption.contains('>') => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            _ => plain.push(c),
        }
    }
    for (written, parsed) in REFERENCES {
        plain = plain.replace(written, parsed);
    }
    units(&plain)
}

/// Every caption of the population, as its name and text.
fn population() -> Vec<(String, String)> {
    let mut members = Vec::new();
    for (class, piece) in CLASSES {
        for target in TARGETS {
            members.push((format!("{class} at {target}"), fill(piece, target)));
            for (open, close) in TAGS {
                let overhead = units(open) + units(close);
                for (how, inner) in [("written", target - overhead), ("parsed", target)] {
                    members.push((
                        format!("{class} in {open} with {how} {target}"),
                        format!("{open}{}{close}", fill(piece, inner)),
                    ));
                }
            }
            for (written, _) in REFERENCES {
                let width = units(written);
                for (how, inner) in [("written", target - width), ("parsed", target - 1)] {
                    members.push((
                        format!("{class} with {written} and {how} {target}"),
                        format!("{}{written}", fill(piece, inner)),
                    ));
                }
            }
        }
    }
    members
}

#[tokio::test]
async fn a_share_caption_is_bounded_as_the_photo_caption_is() {
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
    let bytes = {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        png.extend_from_slice(&13_u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&64_u32.to_be_bytes());
        png.extend_from_slice(&48_u32.to_be_bytes());
        png.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
        png.resize(128, 0);
        png
    };

    let members = population();
    let expected = CLASSES.len() * TARGETS.len() * (1 + TAGS.len() * 2 + REFERENCES.len() * 2);
    assert_eq!(
        members.len(),
        expected,
        "the population is its classes by its forms"
    );
    let (mut accepted, mut refused, mut conservative) = (0_usize, 0_usize, 0_usize);
    let mut wrong = Vec::new();
    for (name, caption) in &members {
        let before = wrong.len();
        let calls = bot.shares.load(Ordering::SeqCst);
        let answer = harness.router.prepare_share(&file, caption).await;
        let called = bot.shares.load(Ordering::SeqCst) - calls;
        let photo = Photo::new(bytes.clone(), caption.as_str());
        let by_the_photo = photo.is_ok();
        let by_the_share = matches!(answer, Prepared::Ready { .. });
        if by_the_photo != by_the_share {
            wrong.push(format!(
                "{name}: the photo path says {by_the_photo}, the share {answer:?}"
            ));
        }
        if by_the_share {
            accepted += 1;
            if called != 1 || parsed_units(caption) > LIMIT {
                wrong.push(format!(
                    "{name}: accepted with {called} calls, {} units",
                    parsed_units(caption)
                ));
            }
        } else {
            refused += 1;
            if called != 0 || answer != Prepared::Refused(PhotoError::Caption) {
                wrong.push(format!(
                    "{name}: refused as {answer:?} after {called} calls"
                ));
            }
            if parsed_units(caption) <= LIMIT {
                conservative += 1;
            }
        }
        if !by_the_share && units(caption) <= LIMIT {
            wrong.push(format!("{name}: refused within the bound"));
        }
        if parsed_units(caption) > LIMIT && by_the_share {
            wrong.push(format!(
                "{name}: over the bound as the Bot API measures it, accepted"
            ));
        }
        wrong.truncate((before + 1).min(wrong.len()));
    }
    println!(
        "examined {} share captions ({accepted} accepted, {refused} refused, {conservative} refused \
         though their parsed text fits)",
        members.len()
    );
    assert!(
        wrong.is_empty(),
        "{} of {} captions are misjudged, e.g. {}",
        wrong.len(),
        members.len(),
        wrong.first().map_or("", String::as_str)
    );
    assert!(
        accepted > 0 && refused > 0,
        "the population holds both sides of the bound"
    );
}

/// Reports how many members a population holds, and refuses an empty one.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(!items.is_empty(), "examined 0 {what}: nothing was judged");
    items
}

#[test]
fn the_share_and_the_photo_read_one_bound_function() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let read = |name: &str| fs::read_to_string(sources.join(name)).expect("a source");
    let code = |text: &str| -> String {
        text.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let files: Vec<PathBuf> = fs::read_dir(&sources)
        .expect("the source directory")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    let files = examined("notifications source file(s)", files);
    let mut counted = 0;
    for path in files {
        let text = fs::read_to_string(&path).expect("a source");
        counted += code(&text).matches("encode_utf16").count();
    }
    println!("examined {counted} UTF-16 counts in the notifications sources");
    assert_eq!(counted, 1, "the caption is counted in one place");
    let photo = code(&read("photo.rs"));
    let router = code(&read("router.rs"));
    assert!(
        photo.contains("pub fn check_caption("),
        "the bound is one named function"
    );
    assert!(
        photo.matches("check_caption(").count() >= 2,
        "Photo::new calls the bound it defines"
    );
    let share = &router[router.find("fn prepare_share").expect("prepare_share")..];
    let share = &share[..share.find("\n    }\n").expect("the end of prepare_share")];
    assert!(
        share.contains("check_caption("),
        "prepare_share calls the same bound"
    );
}
