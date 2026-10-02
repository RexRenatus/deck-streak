//! The inbox capture use case (SPEC-118 R3, R4, R5, R10): the bot's streamed entry and the Mini
//! App's quick capture both write through the vault's one capture, so each lands once, attachment
//! first; a quick capture's text is bounded before anything is written; a retry answers the first
//! name; a missing inbox is refused and never created; and no write reaches a journal folder.
//!
//! The captures run inside `tokio::spawn` on the multi-thread runtime where they can, because the
//! API's handlers and the bot's tasks await them there, which needs their futures to be `Send`.

// An integration test is test code: its helpers panic on an unreadable file.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deck_streak_coordination::inbox_capture::{
    Capture, CaptureKind, Captured, InboxCaptures, LayoutInForce, QUICK_TEXT_CHARS, QuickAnswer,
    QuickKind, RealFs, Source, VaultError,
};
use deck_streak_kernel::{Db, UtcMillis};
use deck_streak_vault::inbox::{attachment_name, miniapp_stub, stem, stub_name, telegram_stub};

/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// One hour, in milliseconds.
const HOUR_MS: i64 = 3_600_000;
/// The epoch day the captures here are taken on.
const DAY: i64 = 20_000;
/// The inbox folder the layouts here name.
const INBOX: &str = "90-Inbox";

/// The instant `millis` into the epoch day `day`.
fn at(day: i64, millis: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * DAY_MS + millis)
}

/// The vendored inbox, with `journal` as the journal folders.
fn layout(journal: &[&str]) -> LayoutInForce {
    LayoutInForce {
        inbox: INBOX.to_owned(),
        journal: journal.iter().map(|folder| (*folder).to_owned()).collect(),
    }
}

/// A temporary directory holding the ledger and a vault root whose inbox folder exists.
async fn setup() -> (tempfile::TempDir, Db, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    let root = dir.path().join("vault");
    fs::create_dir_all(root.join(INBOX)).expect("the inbox folder");
    (dir, db, root)
}

/// The names in `folder`, sorted.
fn files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("the folder lists")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("a UTF-8 name")
        })
        .collect();
    names.sort();
    names
}

/// The text of the file `name` in the inbox under `root`.
fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(INBOX).join(name)).expect("the capture's file")
}

/// The Mini App's stub name for `kind`, `capture_id` and `when`.
fn quick_name(kind: CaptureKind, capture_id: &str, when: UtcMillis) -> String {
    stub_name(&stem(kind, capture_id, when))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_quick_capture_writes_the_miniapp_stub_through_the_vault() {
    let (_dir, db, root) = setup().await;
    let captures = Arc::new(InboxCaptures::new(RealFs, root.clone(), layout(&[])));
    let when = at(DAY, HOUR_MS);
    let cases = [
        (
            QuickKind::Text,
            CaptureKind::Text,
            "6f1d2c9a-text",
            "call the clinic",
        ),
        (
            QuickKind::Journal,
            CaptureKind::Journal,
            "6f1d2c9a-journal",
            "  a calm morning \n",
        ),
    ];
    for (quick, kind, capture_id, text) in cases {
        let (captures, db) = (Arc::clone(&captures), db.clone());
        let answer =
            tokio::spawn(async move { captures.quick(&db, capture_id, quick, text, when).await })
                .await
                .expect("the task finishes")
                .expect("the quick capture");
        let name = quick_name(kind, capture_id, when);
        assert_eq!(answer, QuickAnswer::Saved { name: name.clone() });
        assert_eq!(
            read(&root, &name),
            miniapp_stub(kind, when, text),
            "the stub is the vault's Mini App stub"
        );
    }
    assert_eq!(files(&root.join(INBOX)).len(), 2, "one stub per capture");

    assert_eq!(QuickKind::from_name("text"), Some(QuickKind::Text));
    assert_eq!(QuickKind::from_name("journal"), Some(QuickKind::Journal));
    assert_eq!(
        QuickKind::from_name("photo"),
        None,
        "a media kind is the bot's"
    );
    assert_eq!(QuickKind::from_name("Text"), None, "the name is exact");
}

#[tokio::test]
async fn a_quick_capture_out_of_bounds_writes_nothing() {
    let (_dir, db, root) = setup().await;
    let captures = InboxCaptures::new(RealFs, root.clone(), layout(&[]));
    let when = at(DAY, HOUR_MS);
    let too_long = "x".repeat(QUICK_TEXT_CHARS + 1);
    for text in ["", " \u{1c}\u{1f}\n\t ", too_long.as_str()] {
        let answer = captures
            .quick(&db, "6f1d2c9a-bounds", QuickKind::Text, text, when)
            .await
            .expect("an out-of-bounds text is an answer, never an error");
        assert_eq!(
            answer,
            QuickAnswer::TextOutOfBounds,
            "{} characters",
            text.chars().count()
        );
    }
    assert!(files(&root.join(INBOX)).is_empty(), "nothing was written");

    // The controls: the longest text, padded by the trim and counted in characters rather than
    // bytes, fits, and its capture id was never recorded by the refusals above.
    let longest = format!("  {} \n", "é".repeat(QUICK_TEXT_CHARS));
    let answer = captures
        .quick(&db, "6f1d2c9a-bounds", QuickKind::Text, &longest, when)
        .await
        .expect("the longest text");
    assert_eq!(
        answer,
        QuickAnswer::Saved {
            name: quick_name(CaptureKind::Text, "6f1d2c9a-bounds", when),
        },
        "the refused texts recorded no capture"
    );
    let answer = captures
        .quick(&db, "6f1d2c9a-short", QuickKind::Text, "x", when)
        .await
        .expect("one character");
    assert!(
        matches!(answer, QuickAnswer::Saved { .. }),
        "one character fits"
    );
}

#[tokio::test]
async fn a_quick_retry_answers_the_first_name_and_writes_nothing() {
    let (_dir, db, root) = setup().await;
    let captures = InboxCaptures::new(RealFs, root.clone(), layout(&[]));
    let first_at = at(DAY, DAY_MS - 500);
    let name = quick_name(CaptureKind::Text, "6f1d2c9a-retry", first_at);

    let answer = captures
        .quick(
            &db,
            "6f1d2c9a-retry",
            QuickKind::Text,
            "call the clinic",
            first_at,
        )
        .await
        .expect("the quick capture");
    assert_eq!(answer, QuickAnswer::Saved { name: name.clone() });

    let answer = captures
        .quick(
            &db,
            "6f1d2c9a-retry",
            QuickKind::Text,
            "call the clinic again",
            at(DAY + 1, 200),
        )
        .await
        .expect("the retry");
    assert_eq!(
        answer,
        QuickAnswer::AlreadyCaptured { name: name.clone() },
        "a retry after UTC midnight answers the first name"
    );
    assert_eq!(
        files(&root.join(INBOX)),
        vec![name.clone()],
        "the retry wrote nothing"
    );
    assert_eq!(
        read(&root, &name),
        miniapp_stub(CaptureKind::Text, first_at, "call the clinic"),
        "the first capture's stub is unchanged"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_streamed_capture_lands_its_attachment_then_its_stub() {
    let (_dir, db, root) = setup().await;
    let captures = Arc::new(InboxCaptures::new(RealFs, root.clone(), layout(&[])));
    let when = at(DAY, HOUR_MS);
    let photo = Capture {
        kind: CaptureKind::Photo,
        source: Source::Telegram,
        unique: "AgADx1".to_owned(),
        when,
        caption: "the whiteboard".to_owned(),
    };
    let photo_stem = stem(CaptureKind::Photo, "AgADx1", when);
    let attachment = attachment_name(&photo_stem, ".jpg");

    let (task_captures, task_db, task_photo) = (Arc::clone(&captures), db.clone(), photo.clone());
    let (streamed, answer) = tokio::spawn(async move {
        let mut capture = task_captures
            .stream(task_photo, ".jpg")
            .expect("the attachment starts");
        let streamed = capture.name().to_owned();
        capture.write(b"first half, ").expect("the first chunk");
        capture.write(b"second half").expect("the second chunk");
        (streamed, capture.capture(&task_db).await)
    })
    .await
    .expect("the task finishes");
    assert_eq!(streamed, attachment, "the attachment's name is the vault's");
    assert_eq!(
        answer.expect("the capture"),
        Captured::Saved {
            name: attachment.clone(),
        }
    );
    let mut landed = vec![attachment.clone(), stub_name(&photo_stem)];
    landed.sort();
    assert_eq!(
        files(&root.join(INBOX)),
        landed,
        "the attachment and its stub, nothing else"
    );
    assert_eq!(
        fs::read(root.join(INBOX).join(&attachment)).expect("the attachment"),
        b"first half, second half"
    );
    assert_eq!(
        read(&root, &stub_name(&photo_stem)),
        telegram_stub(CaptureKind::Photo, when, &attachment, "the whiteboard")
    );

    // A resend the same day is written once: it answers the first name, its bytes are discarded
    // and its temporary file is gone.
    let mut resend = captures.stream(photo, ".jpg").expect("the resend starts");
    resend.write(b"other bytes").expect("the resend's chunk");
    let answer = resend.capture(&db).await.expect("the resend");
    assert_eq!(
        answer,
        Captured::AlreadyCaptured {
            name: attachment.clone(),
        }
    );
    assert_eq!(files(&root.join(INBOX)), landed, "the resend left no file");
    assert_eq!(
        fs::read(root.join(INBOX).join(&attachment)).expect("the attachment"),
        b"first half, second half",
        "the first bytes are kept"
    );
}

#[tokio::test]
async fn a_missing_inbox_reads_vault_missing_and_creates_nothing() {
    let (dir, db, root) = setup().await;
    fs::remove_dir(root.join(INBOX)).expect("the inbox folder is removed");
    let captures = InboxCaptures::new(RealFs, root.clone(), layout(&[]));
    let when = at(DAY, HOUR_MS);
    let photo = Capture {
        kind: CaptureKind::Photo,
        source: Source::Telegram,
        unique: "AgADx2".to_owned(),
        when,
        caption: String::new(),
    };

    let quick = captures
        .quick(&db, "6f1d2c9a-missing", QuickKind::Text, "a note", when)
        .await;
    assert!(matches!(quick, Err(VaultError::VaultMissing)), "{quick:?}");
    let streamed = captures.stream(photo.clone(), ".jpg");
    assert!(
        matches!(streamed, Err(VaultError::VaultMissing)),
        "{streamed:?}"
    );
    assert!(files(&root).is_empty(), "no folder was created");

    let gone = InboxCaptures::new(RealFs, dir.path().join("no-vault"), layout(&[]));
    let quick = gone
        .quick(&db, "6f1d2c9a-missing", QuickKind::Text, "a note", when)
        .await;
    assert!(matches!(quick, Err(VaultError::VaultMissing)), "{quick:?}");
    assert!(!dir.path().join("no-vault").exists(), "no root was created");

    // The control: the inbox is located per capture, so it is found once it exists again.
    fs::create_dir(root.join(INBOX)).expect("the inbox folder returns");
    let quick = captures
        .quick(&db, "6f1d2c9a-missing", QuickKind::Text, "a note", when)
        .await
        .expect("the capture once the inbox exists");
    assert!(matches!(quick, QuickAnswer::Saved { .. }), "{quick:?}");
    let streamed = captures.stream(photo, ".jpg");
    assert!(streamed.is_ok(), "{streamed:?}");
}

#[tokio::test]
async fn a_journal_layout_refuses_every_write() {
    let (dir, db, root) = setup().await;
    let when = at(DAY, HOUR_MS);
    let photo = Capture {
        kind: CaptureKind::Photo,
        source: Source::Telegram,
        unique: "AgADx3".to_owned(),
        when,
        caption: String::new(),
    };

    // The inbox itself lies under a journal folder: located, it is refused.
    let journal = InboxCaptures::new(RealFs, root.clone(), layout(&[INBOX]));
    let quick = journal
        .quick(&db, "6f1d2c9a-journal", QuickKind::Journal, "a note", when)
        .await;
    assert!(
        matches!(quick, Err(VaultError::JournalRefused)),
        "{quick:?}"
    );
    let streamed = journal.stream(photo.clone(), ".jpg");
    assert!(
        matches!(streamed, Err(VaultError::JournalRefused)),
        "{streamed:?}"
    );

    // A journal folder that names the very file a capture writes, reached through a vault root
    // that is a symbolic link: the guard refuses the write at the inbox's resolved path, and a
    // refused stub records nothing.
    let link = dir.path().join("vault-link");
    std::os::unix::fs::symlink(&root, &link).expect("a link to the vault");
    let attachment = attachment_name(&stem(CaptureKind::Photo, "AgADx3", when), ".jpg");
    let named = format!("{INBOX}/{attachment}");
    let guarded = InboxCaptures::new(RealFs, link.clone(), layout(&[&named]));
    let streamed = guarded.stream(photo, ".jpg");
    assert!(
        matches!(streamed, Err(VaultError::JournalRefused)),
        "{streamed:?}"
    );

    let stub = quick_name(CaptureKind::Journal, "6f1d2c9a-guard", when);
    let named = format!("{INBOX}/{stub}");
    let guarded = InboxCaptures::new(RealFs, link.clone(), layout(&[&named]));
    let quick = guarded
        .quick(&db, "6f1d2c9a-guard", QuickKind::Journal, "a note", when)
        .await;
    assert!(
        matches!(quick, Err(VaultError::JournalRefused)),
        "{quick:?}"
    );
    assert!(files(&root.join(INBOX)).is_empty(), "nothing was written");

    // The control: the same capture under a layout with no journal is saved, so the refused one
    // recorded no capture.
    let open = InboxCaptures::new(RealFs, link, layout(&[]));
    let quick = open
        .quick(&db, "6f1d2c9a-guard", QuickKind::Journal, "a note", when)
        .await
        .expect("the capture with no journal");
    assert_eq!(quick, QuickAnswer::Saved { name: stub });
}

#[tokio::test]
async fn the_captures_print_the_attachments_name_and_never_the_vault_root() {
    let (_dir, _db, root) = setup().await;
    let captures = InboxCaptures::new(RealFs, root.clone(), layout(&[]));
    assert_eq!(format!("{captures:?}"), "InboxCaptures(..)");

    let when = at(DAY, HOUR_MS);
    let photo = Capture {
        kind: CaptureKind::Photo,
        source: Source::Telegram,
        unique: "AgADx3".to_owned(),
        when,
        caption: String::new(),
    };
    let attachment = attachment_name(&stem(CaptureKind::Photo, "AgADx3", when), ".jpg");
    let streaming = captures
        .stream(photo, ".jpg")
        .expect("the attachment starts");
    let printed = format!("{streaming:?}");
    assert_eq!(
        printed,
        format!("StreamingCapture {{ name: {attachment:?}, .. }}")
    );
    assert!(
        !printed.contains(&*root.to_string_lossy()),
        "the vault root is never printed: {printed}"
    );
}
