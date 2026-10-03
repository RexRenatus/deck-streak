//! The owner's photo, voice note or document lands in the vault inbox (SPEC-118 R6 to R9, A7 to
//! A14, #154): the choice and the replies are the predecessor's goldens, the extension rule and the
//! size cap are R7's and R8's own limits, a stream past the cap leaves nothing, a missing vault is
//! reported, the file URL never reaches a log line, and only the owner's media is admitted.
//!
//! Each test that asserts an absence first asserts the present case of the same input, so a test
//! that went blind fails (the tdd pack's positive control).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;
#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use deck_streak_bot::Commands;
use deck_streak_bot::capture::{self, Choice, MAX_DOWNLOAD_BYTES, Outcome};
use deck_streak_bot::gate::{self, Admission, Dropped};
use deck_streak_bot::transport::SEND_ATTEMPTS;
use deck_streak_coordination::inbox_capture::{CaptureKind, InboxCaptures, LayoutInForce, RealFs};
use deck_streak_kernel::Db;
use fake_bot_api::{
    Answer, Bench, CLIENT_TIMEOUT, DOWNLOAD, GROUP, OWNER, STRANGER, ScriptedSync, Served, TOKEN,
    incoming, owner, payload,
};
use frankenstein::types::Message;
use serde_json::{Value, json};

/// The inbox folder of the layout the tests capture into.
const INBOX: &str = "90-Inbox";

/// The Bot API's download limit, 20 MB, as R8 states it: written out here, never read from the
/// code under test.
const CAP: u64 = 20 * 1024 * 1024;

/// The predecessor's line for a file that was not fetched (golden `media_capture_replies`).
const FETCH_FAILED: &str = "⚠️ Couldn't fetch that file from Telegram — nothing saved.";

/// The predecessor's line for a save the vault refused (golden `media_capture_replies`).
const SAVE_FAILED: &str = "⚠️ Couldn't save that capture to the vault inbox.";

/// How the predecessor's saved line starts (golden `media_capture_replies`).
const SAVED: &str = "📥 Saved to Inbox as <code>";

/// A message carrying `media` from `from` in the chat `chat` of type `chat_type`, as update
/// `update_id`.
fn media(update_id: i64, from: i64, chat: i64, chat_type: &str, media: &Value) -> Value {
    let mut message = json!({
        "message_id": update_id,
        "date": 0,
        "chat": {"id": chat, "type": chat_type},
        "from": {"id": from, "is_bot": false, "first_name": "Synthetic"},
    });
    for (key, value) in media.as_object().expect("the media's fields") {
        message[key] = value.clone();
    }
    json!({"update_id": update_id, "message": message})
}

/// The owner's `media` in the owner's private chat, as update `update_id`.
fn owner_sends(update_id: i64, body: &Value) -> Value {
    media(update_id, OWNER, OWNER, "private", body)
}

/// A document `file_id` named `notes.pdf`, declaring `size` when given.
fn document(file_id: &str, size: Option<u64>) -> Value {
    let mut document = json!({
        "file_id": file_id,
        "file_unique_id": format!("{file_id}-unique"),
        "file_name": "notes.pdf",
    });
    if let Some(size) = size {
        document["file_size"] = json!(size);
    }
    json!({ "document": document })
}

/// A photo of one size, `file_id`.
fn photo(file_id: &str) -> Value {
    json!({"photo": [{
        "file_id": file_id,
        "file_unique_id": format!("{file_id}-unique"),
        "width": 640,
        "height": 480,
    }]})
}

/// `value` as frankenstein reads a message.
fn message_of(value: &Value) -> Message {
    serde_json::from_value(value.clone()).expect("a message")
}

/// `getFile`'s answer for `file_id`: its path, and `size` when it declares one.
fn remote(file_id: &str, size: Option<u64>) -> Answer {
    let mut file = json!({
        "file_id": file_id,
        "file_unique_id": format!("{file_id}-unique"),
        "file_path": format!("documents/{file_id}.pdf"),
    });
    if let Some(size) = size {
        file["file_size"] = json!(size);
    }
    Answer::Result(file)
}

/// The vault root inside the bench's directory, with its inbox folder.
fn vault(bench: &Bench) -> PathBuf {
    let root = bench.directory.path().join("vault");
    fs::create_dir_all(root.join(INBOX)).expect("the inbox folder");
    root
}

/// The inbox at `root`, opened as the bot role opens it: no journal folder.
fn captures(root: &Path) -> Arc<InboxCaptures<RealFs>> {
    Arc::new(InboxCaptures::new(
        RealFs,
        root.to_path_buf(),
        LayoutInForce {
            inbox: INBOX.to_owned(),
            journal: Vec::new(),
        },
    ))
}

/// The owner's handlers over `bench`, saving media into the vault at `root`.
fn capturing(bench: &Bench, root: &Path) -> Commands<ScriptedSync> {
    bench
        .commands(ScriptedSync::answering([]))
        .with_capture(captures(root))
}

/// Prints how many items a check examined and hands them back (the tdd pack's examined contract).
/// An empty inbox is a legitimate answer for a capture that leaves nothing, so zero is printed and
/// not refused; each such test asserts the present case of the same input beside it.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    items
}

/// Every entry of the inbox at `root`, temporary files included, each with its size, sorted.
fn inbox(root: &Path) -> Vec<(String, u64)> {
    let mut entries: Vec<(String, u64)> = examined(
        "inbox entr(ies)",
        fs::read_dir(root.join(INBOX))
            .expect("the inbox folder")
            .map(|entry| {
                let entry = entry.expect("an entry");
                let size = entry.metadata().expect("its metadata").len();
                let name = entry.file_name().into_string().expect("a UTF-8 name");
                (name, size)
            })
            .collect(),
    );
    entries.sort();
    entries
}

/// How many captures the ledger records.
async fn rows(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM inbox_captures")
        .fetch_one(db.reader())
        .await
        .expect("the capture rows")
}

/// The texts the bot sent, in order.
fn sent(bench: &Bench) -> Vec<String> {
    bench
        .fake
        .calls_of("sendMessage")
        .iter()
        .map(|call| payload(call)["text"].as_str().expect("a text").to_owned())
        .collect()
}

/// A golden's kind, as the capture names it.
fn kind_of(name: &str) -> CaptureKind {
    match name {
        "photo" => CaptureKind::Photo,
        "voice" => CaptureKind::Voice,
        "document" => CaptureKind::Document,
        other => panic!("the golden names no kind {other}"),
    }
}

fn text(value: &Value) -> String {
    value.as_str().expect("a string").to_owned()
}

#[test]
fn media_choice_matches_the_predecessors_golden() {
    let examined = golden::each_case("media_capture_choice", |case| {
        let chosen = capture::choose(&message_of(&case.input["message"]));
        let calls: Vec<Value> = chosen
            .iter()
            .map(|choice| {
                json!({
                    "caption": choice.caption,
                    "ext": choice.ext,
                    "file_id": choice.file_id,
                    "kind": choice.kind.as_str(),
                    "unique": choice.file_unique_id,
                })
            })
            .collect();
        let output = json!({"calls": calls, "captured": chosen.is_some()});
        assert_eq!(output, case.output, "the case {:?}", case.class);
    });
    assert_eq!(examined.count, 26, "every case of the golden was examined");
}

#[test]
fn a_document_extension_off_the_rule_reads_bin() {
    let extension = |name: &str| {
        let message = message_of(&json!({
            "message_id": 1,
            "date": 0,
            "chat": {"id": OWNER, "type": "private"},
            "document": {"file_id": "document-file", "file_unique_id": "u", "file_name": name},
        }));
        capture::choose(&message).map(|choice| choice.ext)
    };
    // The present case first: ten plain characters, and a short one, are kept.
    assert_eq!(extension("a.abcdefghij"), Some(".abcdefghij".to_owned()));
    assert_eq!(extension("Scan.PDF2"), Some(".PDF2".to_owned()));
    // Eleven characters, or any character but a letter or a digit, read `.bin` (R7).
    for name in ["a.abcdefghijk", "a.tar-gz", "a.p_f", "a.p f", "a.pdé", "a."] {
        assert_eq!(
            extension(name),
            Some(".bin".to_owned()),
            "the extension of {name:?}"
        );
    }
}

#[tokio::test]
async fn a_file_over_twenty_megabytes_is_never_fetched() {
    let bench = Bench::start().await;
    let root = vault(&bench);
    let mut commands = capturing(&bench, &root);
    bench
        .fake
        .script("getFile", [remote("document-file-1", Some(CAP))]);
    bench
        .fake
        .serve_file(Served::Bytes(b"synthetic bytes".to_vec()));

    // A file declared at exactly the cap is fetched.
    commands
        .handle(incoming(owner_sends(
            1,
            &document("document-file-1", Some(CAP)),
        )))
        .await;
    assert_eq!(
        bench.fake.calls_of("getFile").len(),
        1,
        "a file declared at the cap is asked for"
    );
    assert_eq!(bench.fake.calls_of(DOWNLOAD).len(), 1, "and downloaded");

    // One byte more is never fetched: no getFile, no download, the fetch line.
    commands
        .handle(incoming(owner_sends(
            2,
            &document("document-file-2", Some(CAP + 1)),
        )))
        .await;
    assert_eq!(
        bench.fake.calls_of("getFile").len(),
        1,
        "a file declared one byte over the cap is never asked for"
    );
    assert_eq!(bench.fake.calls_of(DOWNLOAD).len(), 1, "nor downloaded");
    assert_eq!(sent(&bench).last().map(String::as_str), Some(FETCH_FAILED));
    assert_eq!(MAX_DOWNLOAD_BYTES, 20_971_520, "the cap is R8's 20 MB");
}

#[tokio::test]
async fn a_stream_past_the_cap_is_stopped_and_discarded() {
    let cap = usize::try_from(CAP).expect("the cap fits in memory");
    let bench = Bench::start().await;
    let root = vault(&bench);
    let mut commands = capturing(&bench, &root);
    bench.fake.script(
        "getFile",
        [
            remote("document-file-1", None),
            remote("document-file-2", None),
        ],
    );
    bench.fake.serve_file(Served::Bytes(vec![b'a'; cap]));
    bench.fake.serve_file(Served::Bytes(vec![b'b'; cap + 1]));

    // A stream of exactly the cap lands, and is recorded.
    commands
        .handle(incoming(owner_sends(1, &document("document-file-1", None))))
        .await;
    let landed = inbox(&root);
    assert!(
        landed.iter().any(|(_, size)| *size == CAP),
        "a stream of exactly the cap lands in the inbox: {landed:?}"
    );
    assert_eq!(rows(&bench.db).await, 1, "and is recorded");
    assert!(
        sent(&bench)
            .last()
            .is_some_and(|line| line.starts_with(SAVED)),
        "and the owner is told: {:?}",
        sent(&bench)
    );

    // One byte past the cap: stopped, no file, no temporary file, no row, the fetch line.
    commands
        .handle(incoming(owner_sends(2, &document("document-file-2", None))))
        .await;
    assert_eq!(
        inbox(&root),
        landed,
        "a stream past the cap leaves no file and no temporary file"
    );
    assert_eq!(rows(&bench.db).await, 1, "and records nothing");
    assert_eq!(sent(&bench).last().map(String::as_str), Some(FETCH_FAILED));
}

/// A file's download is bounded by its own timeout, never by the client's: a file the Bot API
/// holds for three of the client's timeouts still lands whole, while a `getFile` held as long
/// fails on the client's timeout (SPEC-118 R8). The download's own bound is minutes long, so it is
/// read here from what it lets through, not waited out.
#[tokio::test]
async fn a_download_held_past_the_clients_timeout_still_lands() {
    let held = CLIENT_TIMEOUT * 3;
    let bench = Bench::start().await;
    let root = vault(&bench);
    let mut commands = capturing(&bench, &root);
    bench.fake.script(
        "getFile",
        [remote("document-file-1", None)]
            .into_iter()
            .chain((0..SEND_ATTEMPTS).map(|_| Answer::Silence)),
    );
    bench.fake.serve_file(Served::Held(vec![b'h'; 4096], held));

    // A download the Bot API holds past the client's timeout lands, and is recorded.
    let started = Instant::now();
    commands
        .handle(incoming(owner_sends(1, &document("document-file-1", None))))
        .await;
    assert!(
        started.elapsed() >= held,
        "the fake held the file for {held:?}: {:?}",
        started.elapsed()
    );
    let landed = inbox(&root);
    assert!(
        landed.iter().any(|(_, size)| *size == 4096),
        "a file held past the client's timeout lands in the inbox: {landed:?}"
    );
    assert_eq!(rows(&bench.db).await, 1, "and is recorded");
    assert!(
        sent(&bench)
            .last()
            .is_some_and(|line| line.starts_with(SAVED)),
        "and the owner is told: {:?}",
        sent(&bench)
    );

    // The same client fails a getFile held as long: its own timeout is the shorter one.
    assert!(
        held > CLIENT_TIMEOUT,
        "the hold outlasts the client's timeout"
    );
    commands
        .handle(incoming(owner_sends(2, &document("document-file-2", None))))
        .await;
    assert_eq!(inbox(&root), landed, "a getFile held silent saves nothing");
    assert_eq!(rows(&bench.db).await, 1, "and records nothing");
    assert_eq!(sent(&bench).last().map(String::as_str), Some(FETCH_FAILED));
}

#[test]
fn capture_replies_match_the_predecessors_golden() {
    let examined = golden::each_case("media_capture_replies", |case| {
        let input = &case.input;
        let choice = Choice {
            file_id: text(&input["file_id"]),
            kind: kind_of(input["kind"].as_str().expect("a kind")),
            ext: text(&input["ext"]),
            caption: text(&input["caption"]),
            file_unique_id: text(&input["unique"]),
            size: None,
        };
        let (mut downloads, mut saves, mut sent) = (Vec::new(), Vec::new(), Vec::new());
        let mut returned = false;
        if !choice.silent() {
            downloads.push(json!(choice.file_id));
            let outcome = if input["download"].is_null() {
                Outcome::NotFetched
            } else {
                saves.push(json!({
                    "caption": choice.caption,
                    "data": input["download"],
                    "ext": choice.ext,
                    "kind": choice.kind.as_str(),
                    "unique": choice.unique(),
                }));
                match input["save"]["filename"].as_str() {
                    Some(name) if input["save"]["ok"] == json!(true) => Outcome::Saved {
                        name: name.to_owned(),
                    },
                    _ => Outcome::NotSaved,
                }
            };
            returned = matches!(outcome, Outcome::Saved { .. });
            sent.push(json!(capture::reply(&outcome)));
        }
        let output = json!({
            "downloads": downloads,
            "returned": returned,
            "saves": saves,
            "sent": sent,
        });
        assert_eq!(output, case.output, "the case {:?}", case.class);
    });
    assert_eq!(examined.count, 10, "every case of the golden was examined");
}

#[tokio::test]
async fn a_missing_vault_is_reported_not_raised() {
    // The present case first: with the vault there, the photo is saved.
    let present = Bench::start().await;
    let root = vault(&present);
    let mut commands = capturing(&present, &root);
    present
        .fake
        .script("getFile", [remote("photo-file-1", None)]);
    present
        .fake
        .serve_file(Served::Bytes(b"synthetic bytes".to_vec()));
    commands
        .handle(incoming(owner_sends(1, &photo("photo-file-1"))))
        .await;
    let saved = sent(&present);
    assert!(
        saved.len() == 1 && saved[0].starts_with(SAVED),
        "a photo is saved into a vault that is there: {saved:?}"
    );

    // With the vault root missing, the owner gets the failed-save line, and nothing is created.
    let missing = Bench::start().await;
    let absent = missing.directory.path().join("absent");
    let mut commands = capturing(&missing, &absent);
    missing
        .fake
        .script("getFile", [remote("photo-file-1", None)]);
    missing
        .fake
        .serve_file(Served::Bytes(b"synthetic bytes".to_vec()));
    commands
        .handle(incoming(owner_sends(1, &photo("photo-file-1"))))
        .await;
    assert_eq!(sent(&missing), vec![SAVE_FAILED.to_owned()]);
    assert!(!absent.exists(), "the missing vault was not created");
}

/// Every line the test's thread logs, at every level and from every target: each event, and each
/// span's fields, as `name=value` pairs.
#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<String>>>);

impl Lines {
    fn logged(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, line: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// One event's or span's fields, as `name=value` pairs.
#[derive(Default)]
struct Fields(Vec<String>);

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push(format!("{}={value}", field.name()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.0.push(format!("{}={value:?}", field.name()));
    }
}

impl tracing::Subscriber for Lines {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        let mut fields = Fields::default();
        span.record(&mut fields);
        self.push(format!(
            "span {} {}",
            span.metadata().name(),
            fields.0.join(" ")
        ));
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        let mut fields = Fields::default();
        values.record(&mut fields);
        self.push(format!("record {}", fields.0.join(" ")));
    }

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let metadata = event.metadata();
        self.push(format!(
            "{} {} {}",
            metadata.level(),
            metadata.target(),
            fields.0.join(" ")
        ));
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[tokio::test]
async fn the_file_url_never_reaches_a_log_line() {
    let lines = Lines::default();
    let _logging = log_capture::hold_capture(lines.clone());
    let bench = Bench::start().await;
    let root = vault(&bench);
    let mut commands = capturing(&bench, &root);
    bench
        .fake
        .script("getFile", [remote("document-file-1", None)]);
    bench.fake.serve_file(Served::Status(500));

    commands
        .handle(incoming(owner_sends(1, &document("document-file-1", None))))
        .await;

    // The present case first: the download was made on the URL that holds the token, and its
    // failure is logged.
    let downloads = bench.fake.calls_of(DOWNLOAD);
    assert!(
        downloads.len() == 1 && downloads[0].token_ok,
        "the file was downloaded from the URL that holds the token: {downloads:?}"
    );
    let logged = lines.logged();
    assert!(
        logged.iter().any(|line| line.contains("method=download")),
        "the failed download is logged: {logged:#?}"
    );
    // No line holds the token or the file URL's path.
    let leaking: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains(TOKEN) || line.contains("/file/bot"))
        .collect();
    println!("examined {} logged line(s)", logged.len());
    assert!(
        leaking.is_empty(),
        "a line holds the file URL: {leaking:#?}"
    );
    assert_eq!(sent(&bench).last().map(String::as_str), Some(FETCH_FAILED));
}

#[test]
fn media_is_admitted_from_the_owner_only() {
    let admitted = |update: Value| {
        let update = incoming(update).update.expect("an update");
        gate::admit(&update.content, owner())
    };
    let body = photo("photo-file-1");
    // The present case first: the owner's photo, in the owner's private chat, is admitted.
    assert_eq!(
        admitted(owner_sends(1, &body)),
        Admission::Media(Choice {
            file_id: "photo-file-1".to_owned(),
            kind: CaptureKind::Photo,
            ext: ".jpg".to_owned(),
            caption: String::new(),
            file_unique_id: "photo-file-1-unique".to_owned(),
            size: None,
        })
    );
    // A stranger's photo, and the owner's in a group, are dropped as any such update is.
    assert_eq!(
        admitted(media(2, STRANGER, STRANGER, "private", &body)),
        Admission::Dropped(Dropped {
            kind: "message",
            reason: "not_owner",
        })
    );
    assert_eq!(
        admitted(media(3, OWNER, GROUP, "group", &body)),
        Admission::Dropped(Dropped {
            kind: "message",
            reason: "not_owners_private_chat",
        })
    );
}
