//! A capture lands in the vault inbox once, attachment first (SPEC-118 A1 to A4, A19, A23, A24):
//! the stem and the stub are the predecessor's (`vault_bridge.py:save_inbox_capture` at `27ee2bc`,
//! golden `inbox_capture_stub`), a missing inbox is refused and never created, a capture sent twice
//! is written once, an `md` attachment never takes its stub's name, a Mini App retry after UTC
//! midnight answers the first name, and an erase deletes the rows and never a vault file.

// An integration test is test code: its helpers panic on an unreadable file, and the golden reader
// prints the examined count on purpose. clippy.toml's in-test allowances cover `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use deck_streak_kernel::data_rights::{DataRights, Disposition};
use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_vault::capture_store::INBOX_CAPTURES_TABLE;
use deck_streak_vault::data_rights::VaultDataRights;
use deck_streak_vault::inbox::{self, Capture, CaptureKind, Captured, Inbox, Source};
use deck_streak_vault::{
    DirEntry, EntryKind, LayoutInForce, RealFs, VaultError, VaultFile, VaultFs,
};

/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// One hour, in milliseconds.
const HOUR_MS: i64 = 3_600_000;
/// The epoch day most captures here are taken on.
const DAY: i64 = 20_000;

/// The instant `millis` into the epoch day `day`.
fn at(day: i64, millis: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * DAY_MS + millis)
}

/// The ISO date of the epoch day `day`, as the kernel writes it.
fn iso_day(day: i64) -> String {
    StudyDay::from_epoch_day(day).to_string()
}

/// The layout the tests write into: the vendored inbox and no journal.
fn layout() -> LayoutInForce {
    LayoutInForce {
        inbox: "90-Inbox".to_owned(),
        journal: Vec::new(),
    }
}

fn telegram(kind: CaptureKind, unique: &str, when: UtcMillis, caption: &str) -> Capture {
    Capture {
        kind,
        source: Source::Telegram,
        unique: unique.to_owned(),
        when,
        caption: caption.to_owned(),
    }
}

fn miniapp(kind: CaptureKind, unique: &str, when: UtcMillis, text: &str) -> Capture {
    Capture {
        kind,
        source: Source::MiniApp,
        unique: unique.to_owned(),
        when,
        caption: text.to_owned(),
    }
}

/// A temporary directory holding the ledger and a vault root whose inbox folder exists.
async fn setup() -> (tempfile::TempDir, Db, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    let root = dir.path().join("vault");
    fs::create_dir_all(root.join("90-Inbox")).expect("the inbox folder");
    (dir, db, root)
}

/// The file names in `folder`, sorted.
fn files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("the folder")
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

/// Sends `capture` with the attachment `bytes`, streamed in two chunks, and answers the capture.
async fn send<F: VaultFs>(
    db: &Db,
    fs: &F,
    inbox: &Inbox,
    capture: &Capture,
    ext: &str,
    bytes: &[u8],
) -> Captured {
    let mut attachment = inbox
        .attachment(fs, capture, ext)
        .expect("the attachment starts");
    let (head, tail) = bytes.split_at(bytes.len() / 2);
    attachment.write(head).expect("the first chunk");
    attachment.write(tail).expect("the second chunk");
    inbox::capture(db, fs, inbox, capture, Some(attachment))
        .await
        .expect("the capture")
}

/// `text` with every `{second:N}` and `{day:N}` token written as the text it stands for
/// (`tools/parity-oracle/registry/spec_118.py:expand`): N is an epoch second or an epoch day.
fn expand(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        let close = after.find('}').expect("a closed token");
        let token = &after[1..close];
        if let Some(second) = token.strip_prefix("second:") {
            let second: i64 = second.parse().expect("an epoch second");
            let day = second.div_euclid(86_400);
            let into = second.rem_euclid(86_400);
            write!(
                out,
                "{}T{:02}:{:02}:{:02}+00:00",
                iso_day(day),
                into / 3_600,
                into % 3_600 / 60,
                into % 60
            )
            .expect("a string takes the instant");
        } else if let Some(day) = token.strip_prefix("day:") {
            out.push_str(&iso_day(day.parse().expect("an epoch day")));
        } else {
            out.push_str(&after[..=close]);
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

fn kind_named(name: &str) -> CaptureKind {
    match name {
        "photo" => CaptureKind::Photo,
        "voice" => CaptureKind::Voice,
        "document" => CaptureKind::Document,
        other => panic!("the golden names an unknown kind {other}"),
    }
}

#[test]
fn the_stub_and_stem_match_the_predecessors_golden() {
    let vendored = LayoutInForce::vendored().expect("the vendored layout");
    let examined = golden::each_case("inbox_capture_stub", |case| {
        let input = &case.input;
        let output = &case.output;
        let kind = kind_named(input["kind"].as_str().expect("a kind"));
        let unique = input["unique"].as_str().expect("a unique");
        let when = UtcMillis::from_epoch_millis(input["instant_ms"].as_i64().expect("an instant"));
        let caption = input["caption"].as_str().expect("a caption");
        let stem = inbox::stem(kind, unique, when);
        let extension =
            inbox::extension(input["ext"].as_str().expect("an extension")).expect("a plain one");
        let attachment = inbox::attachment_name(&stem, &extension);
        assert_eq!(
            attachment,
            expand(output["attachment"].as_str().expect("the attachment")),
            "the attachment of {input}"
        );
        assert!(
            stem.ends_with(&format!("-{}", inbox::safe_unique(unique))),
            "the stem {stem} ends in the safe unique of {input}"
        );
        let written: Vec<String> = output["written"]
            .as_array()
            .expect("the names written")
            .iter()
            .map(|name| expand(name.as_str().expect("a name")))
            .collect();
        assert_eq!(
            vec![attachment.clone(), inbox::stub_name(&stem)],
            written,
            "the names written, in order, of {input}"
        );
        assert_eq!(
            inbox::telegram_stub(kind, when, &attachment, caption),
            expand(output["stub"].as_str().expect("the stub")),
            "the stub of {input}"
        );
        assert_eq!(
            vendored.inbox,
            output["folder"].as_str().expect("the folder"),
            "the folder of {input}"
        );
    });
    assert_eq!(examined.count, 58, "the golden's 58 cases: {examined}");
}

/// One step a write took, as the recording file system saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    CreateNew(PathBuf),
    Write(PathBuf),
    SyncFile(PathBuf),
    Rename(PathBuf, PathBuf),
    SyncDir(PathBuf),
    RemoveFile(PathBuf),
}

type Log = Arc<Mutex<Vec<Step>>>;

/// A file system that records every write step and does it on the real file system.
#[derive(Default)]
struct Recording {
    log: Log,
}

impl Recording {
    fn steps(&self) -> Vec<Step> {
        self.log.lock().expect("the log").clone()
    }
}

fn record(log: &Log, step: Step) {
    log.lock().expect("the log").push(step);
}

struct RecordingFile {
    inner: Box<dyn VaultFile>,
    path: PathBuf,
    log: Log,
}

impl VaultFile for RecordingFile {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        record(&self.log, Step::Write(self.path.clone()));
        self.inner.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        record(&self.log, Step::SyncFile(self.path.clone()));
        self.inner.sync()
    }
}

impl VaultFs for Recording {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        record(&self.log, Step::CreateNew(path.to_path_buf()));
        Ok(Box::new(RecordingFile {
            inner: RealFs.create_new(path)?,
            path: path.to_path_buf(),
            log: Arc::clone(&self.log),
        }))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        record(
            &self.log,
            Step::Rename(from.to_path_buf(), to.to_path_buf()),
        );
        RealFs.rename(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        record(&self.log, Step::SyncDir(dir.to_path_buf()));
        RealFs.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        record(&self.log, Step::RemoveFile(path.to_path_buf()));
        RealFs.remove_file(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        RealFs.read(path)
    }

    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        RealFs.kind(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        RealFs.list(dir)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        RealFs.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        RealFs.remove_dir(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        RealFs.canonicalize(path)
    }
}

#[tokio::test]
async fn the_attachment_lands_before_its_stub() {
    let (_dir, db, root) = setup().await;
    let fs = Recording::default();
    let inbox = Inbox::locate(&fs, &root, &layout()).expect("the inbox");
    let capture = telegram(
        CaptureKind::Photo,
        "AgADBAADr6cxG",
        at(DAY, 9 * HOUR_MS),
        "the whiteboard",
    );

    let answer = send(&db, &fs, &inbox, &capture, ".jpg", b"the photo's bytes").await;

    let name = format!("{}-photo-AgADBAADr6cxG.jpg", iso_day(DAY));
    let stub = format!("{}-photo-AgADBAADr6cxG.md", iso_day(DAY));
    assert_eq!(
        answer,
        Captured::Saved { name: name.clone() },
        "the capture answers its attachment's name"
    );
    let folder = inbox.folder().to_path_buf();
    let pid = std::process::id();
    let attachment_temp = folder.join(format!(".{name}.{pid}.tmp"));
    let stub_temp = folder.join(format!(".{stub}.{pid}.tmp"));
    assert_eq!(
        fs.steps(),
        vec![
            Step::CreateNew(attachment_temp.clone()),
            Step::Write(attachment_temp.clone()),
            Step::Write(attachment_temp.clone()),
            Step::SyncFile(attachment_temp.clone()),
            Step::Rename(attachment_temp, folder.join(&name)),
            Step::SyncDir(folder.clone()),
            Step::CreateNew(stub_temp.clone()),
            Step::Write(stub_temp.clone()),
            Step::SyncFile(stub_temp.clone()),
            Step::Rename(stub_temp, folder.join(&stub)),
            Step::SyncDir(folder.clone()),
        ],
        "the attachment streams into its temporary file and is renamed into place before the \
         stub's temporary file is created"
    );
    assert_eq!(
        fs::read(folder.join(&name)).expect("the attachment"),
        b"the photo's bytes",
        "every chunk landed"
    );
    assert!(
        fs::read_to_string(folder.join(&stub))
            .expect("the stub")
            .contains(&format!("[[{name}]]")),
        "the stub names its attachment"
    );
    assert_eq!(
        files(&folder),
        vec![name, stub],
        "no temporary file is left"
    );
}

#[test]
fn a_missing_inbox_is_refused_and_never_created() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("vault");

    let refused = Inbox::locate(&RealFs, &root, &layout());
    assert!(
        matches!(refused, Err(VaultError::VaultMissing)),
        "a missing vault root is refused with vault_missing: {refused:?}"
    );
    assert!(!root.exists(), "the vault root was created");

    fs::create_dir(&root).expect("the vault root");
    let refused = Inbox::locate(&RealFs, &root, &layout());
    let Err(error) = refused else {
        panic!("a missing inbox folder is refused: {refused:?}");
    };
    assert!(
        matches!(error, VaultError::VaultMissing),
        "a missing inbox folder is refused with vault_missing: {error:?}"
    );
    assert_eq!(error.to_string(), "vault_missing");
    assert!(
        files(&root).is_empty(),
        "nothing is created at the vault's top level"
    );

    fs::write(root.join("90-Inbox"), "a file, not a folder").expect("a file at the inbox");
    let refused = Inbox::locate(&RealFs, &root, &layout());
    assert!(
        matches!(refused, Err(VaultError::VaultMissing)),
        "a file where the inbox folder should be is refused with vault_missing: {refused:?}"
    );
    assert!(
        root.join("90-Inbox").is_file(),
        "the file at the inbox is left as it was"
    );
}

#[tokio::test]
async fn a_capture_sent_twice_is_written_once() {
    let (_dir, db, root) = setup().await;
    let inbox = Inbox::locate(&RealFs, &root, &layout()).expect("the inbox");
    let day = iso_day(DAY);
    let name = format!("{day}-document-BQACAgQAAxkB.pdf");
    let stub = format!("{day}-document-BQACAgQAAxkB.md");

    let first = telegram(
        CaptureKind::Document,
        "BQACAgQAAxkB",
        at(DAY, 9 * HOUR_MS),
        "the first send",
    );
    let answer = send(&db, &RealFs, &inbox, &first, ".pdf", b"the first bytes").await;
    assert_eq!(answer, Captured::Saved { name: name.clone() });

    let resent = telegram(
        CaptureKind::Document,
        "BQACAgQAAxkB",
        at(DAY, 17 * HOUR_MS),
        "the resend",
    );
    let answer = send(&db, &RealFs, &inbox, &resent, ".pdf", b"the second bytes").await;
    assert_eq!(
        answer,
        Captured::AlreadyCaptured { name: name.clone() },
        "a resend of a recorded stem answers already_captured with the first name"
    );
    let folder = inbox.folder();
    assert_eq!(
        fs::read(folder.join(&name)).expect("the attachment"),
        b"the first bytes",
        "the resend wrote nothing in place"
    );
    let written = fs::read_to_string(folder.join(&stub)).expect("the stub");
    assert!(
        written.contains("the first send") && !written.contains("the resend"),
        "the stub is the first send's"
    );
    assert_eq!(
        files(folder),
        vec![stub.clone(), name],
        "the resend's temporary file is removed (the names in sorted order: `.md` before `.pdf`)"
    );

    let quick = miniapp(
        CaptureKind::Text,
        "6f1d2c9a",
        at(DAY, 10 * HOUR_MS),
        "call the clinic",
    );
    let quick_name = format!("{day}-text-6f1d2c9a.md");
    let answer = inbox::capture(&db, &RealFs, &inbox, &quick, None)
        .await
        .expect("the quick capture");
    assert_eq!(
        answer,
        Captured::Saved {
            name: quick_name.clone()
        }
    );
    let again = miniapp(
        CaptureKind::Text,
        "6f1d2c9a",
        at(DAY, 11 * HOUR_MS),
        "call the clinic again",
    );
    let answer = inbox::capture(&db, &RealFs, &inbox, &again, None)
        .await
        .expect("the retried quick capture");
    assert_eq!(
        answer,
        Captured::AlreadyCaptured {
            name: quick_name.clone()
        },
        "a retried quick capture answers the first name"
    );
    assert!(
        !fs::read_to_string(folder.join(&quick_name))
            .expect("the quick stub")
            .contains("again"),
        "the retry wrote nothing in place"
    );
    assert_eq!(files(folder).len(), 3, "two captures, three files");
}

#[tokio::test]
async fn inbox_captures_export_and_erase_leave_the_files() {
    let port = VaultDataRights;
    let declaration = port.declaration().expect("a declaration");
    assert!(
        matches!(
            declaration.disposition(INBOX_CAPTURES_TABLE),
            Some(Disposition::ExportAndErase)
        ),
        "{INBOX_CAPTURES_TABLE} is exported and erased"
    );

    let (_dir, db, root) = setup().await;
    let inbox = Inbox::locate(&RealFs, &root, &layout()).expect("the inbox");
    let day = iso_day(DAY);
    let photo = telegram(CaptureKind::Photo, "AgADBAADr6cxG", at(DAY, HOUR_MS), "");
    send(&db, &RealFs, &inbox, &photo, ".jpg", b"the photo's bytes").await;
    let quick = miniapp(
        CaptureKind::Journal,
        "6f1d2c9a",
        at(DAY, 2 * HOUR_MS),
        "a line for the journal",
    );
    inbox::capture(&db, &RealFs, &inbox, &quick, None)
        .await
        .expect("the quick capture");
    let before = files(inbox.folder());
    assert_eq!(before.len(), 3, "two captures, three files");

    let mut tx = db.write().await.expect("a write");
    let exported = port.export(&mut tx).await.expect("an export");
    let table = exported
        .iter()
        .find(|table| table.table == INBOX_CAPTURES_TABLE)
        .expect("the table exported");
    let stems: Vec<&str> = table
        .rows
        .iter()
        .map(|row| row["stem"].as_str().expect("a stem"))
        .collect();
    assert_eq!(
        stems,
        vec![
            format!("{day}-journal-6f1d2c9a"),
            format!("{day}-photo-AgADBAADr6cxG"),
        ],
        "every row is exported, by stem"
    );
    assert_eq!(table.rows[0]["source"], "miniapp");
    assert_eq!(table.rows[0]["capture_key"], "6f1d2c9a");
    assert!(
        table.rows[0]["attachment"].is_null(),
        "a quick capture has no attachment"
    );
    assert_eq!(
        table.rows[1]["attachment"],
        format!("{day}-photo-AgADBAADr6cxG.jpg")
    );
    assert_eq!(table.rows[1]["state"], "captured");

    port.erase(&mut tx).await.expect("an erase");
    let exported = port
        .export(&mut tx)
        .await
        .expect("an export after the erase");
    let table = exported
        .iter()
        .find(|table| table.table == INBOX_CAPTURES_TABLE)
        .expect("the table exported");
    assert!(table.rows.is_empty(), "an erase leaves no row");
    tx.commit().await.expect("committed");
    assert_eq!(
        files(inbox.folder()),
        before,
        "an erase deletes no vault file"
    );
    assert_eq!(
        fs::read(
            inbox
                .folder()
                .join(format!("{day}-photo-AgADBAADr6cxG.jpg"))
        )
        .expect("the attachment remains"),
        b"the photo's bytes"
    );
}

#[test]
fn an_md_attachment_never_takes_its_stubs_name() {
    let when = at(DAY, 12 * HOUR_MS);
    let stem = inbox::stem(CaptureKind::Document, "BQACAgQAAxkB", when);
    assert_eq!(stem, format!("{}-document-BQACAgQAAxkB", iso_day(DAY)));
    assert!(!stem.contains('.'), "a stem holds no dot");
    let stub = inbox::stub_name(&stem);
    assert_eq!(stub, format!("{stem}.md"));
    for ext in [".md", "md", "MD", ".Md", "mD"] {
        let extension = inbox::extension(ext).expect("a plain extension");
        let name = inbox::attachment_name(&stem, &extension);
        let kept = ext.trim_start_matches('.');
        assert_eq!(
            name,
            format!("{stem}.attachment.{kept}"),
            "the attachment of {ext:?} keeps its extension under its own name"
        );
        assert!(
            !name.eq_ignore_ascii_case(&stub),
            "the attachment {name} takes its stub's name {stub}"
        );
        let written = inbox::telegram_stub(CaptureKind::Document, when, &name, "");
        assert!(
            written.contains(&format!("attachment: {name}\n"))
                && written.contains(&format!("[[{name}]]")),
            "the stub names the attachment {name}: {written:?}"
        );
    }
}

#[tokio::test]
async fn every_extension_keeps_its_bytes_apart_from_the_stub() {
    let (_dir, db, root) = setup().await;
    let inbox = Inbox::locate(&RealFs, &root, &layout()).expect("the inbox");
    let population = ["md", "MD", "Md", "mD", "markdown", "txt", "", "bin"];
    for (index, ext) in population.into_iter().enumerate() {
        let unique = format!("BQACAgQAAxk{index}");
        let stem = format!("{}-document-{unique}", iso_day(DAY));
        let name = match ext {
            "" => format!("{stem}.bin"),
            md if md.eq_ignore_ascii_case("md") => format!("{stem}.attachment.{md}"),
            other => format!("{stem}.{other}"),
        };
        let capture = telegram(
            CaptureKind::Document,
            &unique,
            at(DAY, 8 * HOUR_MS),
            "a document",
        );
        let bytes = format!("the bytes of the {ext:?} document");

        let answer = send(&db, &RealFs, &inbox, &capture, ext, bytes.as_bytes()).await;

        assert_eq!(
            answer,
            Captured::Saved { name: name.clone() },
            "the {ext:?} attachment's name"
        );
        let folder = inbox.folder();
        assert_eq!(
            fs::read(folder.join(&name)).expect("the attachment"),
            bytes.as_bytes(),
            "the {ext:?} attachment's bytes survive its stub"
        );
        assert!(
            fs::read_to_string(folder.join(format!("{stem}.md")))
                .expect("the stub")
                .contains(&format!("[[{name}]]")),
            "the {ext:?} stub names its attachment"
        );
    }
    assert_eq!(
        files(inbox.folder()).len(),
        2 * population.len(),
        "every attachment and every stub is a file of its own"
    );
}

#[tokio::test]
async fn a_miniapp_retry_on_a_later_utc_day_answers_the_first_name() {
    let (_dir, db, root) = setup().await;
    let inbox = Inbox::locate(&RealFs, &root, &layout()).expect("the inbox");
    let (day, next) = (iso_day(DAY), iso_day(DAY + 1));
    let name = format!("{day}-text-6f1d2c9a-retry.md");

    let first = miniapp(
        CaptureKind::Text,
        "6f1d2c9a-retry",
        at(DAY, DAY_MS - 500),
        "call the clinic",
    );
    let answer = inbox::capture(&db, &RealFs, &inbox, &first, None)
        .await
        .expect("the quick capture");
    assert_eq!(answer, Captured::Saved { name: name.clone() });

    let retry = miniapp(
        CaptureKind::Text,
        "6f1d2c9a-retry",
        at(DAY + 1, 200),
        "call the clinic",
    );
    let answer = inbox::capture(&db, &RealFs, &inbox, &retry, None)
        .await
        .expect("the retry");
    assert_eq!(
        answer,
        Captured::AlreadyCaptured { name: name.clone() },
        "a retry sent after UTC midnight answers the first name"
    );
    assert_eq!(
        files(inbox.folder()),
        vec![name.clone()],
        "the retry wrote nothing"
    );

    // The control: a Telegram capture of the same unique on a later day is a new capture, as the
    // predecessor writes it.
    let sent = telegram(CaptureKind::Photo, "6f1d2c9a-retry", at(DAY, HOUR_MS), "");
    let answer = send(&db, &RealFs, &inbox, &sent, ".jpg", b"the first photo").await;
    let sent_name = format!("{day}-photo-6f1d2c9a-retry.jpg");
    assert_eq!(answer, Captured::Saved { name: sent_name });
    let resent = telegram(
        CaptureKind::Photo,
        "6f1d2c9a-retry",
        at(DAY + 1, HOUR_MS),
        "",
    );
    let answer = send(&db, &RealFs, &inbox, &resent, ".jpg", b"the later photo").await;
    let resent_name = format!("{next}-photo-6f1d2c9a-retry.jpg");
    assert_eq!(
        answer,
        Captured::Saved {
            name: resent_name.clone()
        },
        "a Telegram capture of the same unique on a later day is a new capture"
    );
    assert_eq!(
        fs::read(inbox.folder().join(&resent_name)).expect("the later photo"),
        b"the later photo"
    );
    assert_eq!(
        files(inbox.folder()).len(),
        5,
        "one quick capture and two Telegram captures"
    );
}

#[test]
fn a_quick_text_fits_one_to_four_thousand_characters_after_the_trim() {
    let longest = "é".repeat(inbox::QUICK_TEXT_CHARS);
    assert!(inbox::quick_text_fits("x"), "one character");
    assert!(
        inbox::quick_text_fits(&longest),
        "the longest, counted in characters"
    );
    assert!(
        inbox::quick_text_fits(&format!(" \u{1c}{longest}\u{1f}\n")),
        "the trim's characters are not counted"
    );
    assert!(!inbox::quick_text_fits(""), "an empty text");
    assert!(
        !inbox::quick_text_fits(" \u{1c}\u{1f}\n\t"),
        "a text the trim empties"
    );
    assert!(
        !inbox::quick_text_fits(&format!("{longest}x")),
        "one character past the bound"
    );
}

#[test]
fn an_extension_is_up_to_ten_letters_or_digits_and_never_a_path() {
    assert_eq!(
        inbox::extension("abcdefghij").expect("ten letters"),
        ".abcdefghij"
    );
    for ext in ["a/b", ".a/b", "abcdefghijk", ".abcdefghijk"] {
        let refused = inbox::extension(ext);
        assert!(
            matches!(refused, Err(VaultError::InvalidExtension)),
            "{ext:?} is not an extension: {refused:?}"
        );
    }
}

#[test]
fn a_miniapp_stub_is_its_keys_then_the_line_then_the_trimmed_text() {
    let when = at(DAY, 12 * HOUR_MS + 34 * 60_000 + 56_000);
    assert_eq!(
        inbox::miniapp_stub(CaptureKind::Text, when, " \u{1c}the padded  note\u{1f}\n\t"),
        "---\nstatus: captured\nsource: miniapp\nkind: text\ncaptured: 2024-10-04T12:34:56+00:00\n\
         tags: [inbox, miniapp-capture]\n---\n\nCaptured via the Mini App.\n\nthe padded  note\n"
    );
}

#[test]
fn an_inbox_and_its_attachment_print_the_name_and_never_a_path() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("vault");
    fs::create_dir_all(root.join("90-Inbox")).expect("the inbox folder");
    let inbox = Inbox::locate(&RealFs, &root, &layout()).expect("the inbox");
    assert_eq!(format!("{inbox:?}"), "Inbox(..)");

    let when = at(DAY, 12 * HOUR_MS);
    let capture = telegram(CaptureKind::Document, "BQACx4", when, "");
    let attachment = inbox
        .attachment(&RealFs, &capture, "pdf")
        .expect("the attachment starts");
    let name = "2024-10-04-document-BQACx4.pdf";
    assert_eq!(attachment.name(), name);
    assert_eq!(
        attachment.name(),
        inbox::attachment_name(&inbox::stem(CaptureKind::Document, "BQACx4", when), ".pdf")
    );
    let printed = format!("{attachment:?}");
    assert_eq!(printed, format!("Attachment {{ name: {name:?}, .. }}"));
    assert!(
        !printed.contains(&*root.to_string_lossy()),
        "the vault root is never printed: {printed}"
    );
}

#[test]
fn an_inbox_through_a_link_resolves_inside_the_root_or_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("vault");
    let real = root.join("00-Capture");
    fs::create_dir_all(&real).expect("the real inbox folder");
    let link = root.join("90-Inbox");

    // A link to a folder inside the root is the inbox, resolved.
    std::os::unix::fs::symlink(&real, &link).expect("the inbox links to a folder");
    let inbox = Inbox::locate(&RealFs, &root, &layout()).expect("the linked inbox");
    assert_eq!(
        inbox.folder(),
        fs::canonicalize(&real).expect("the real folder")
    );

    // A link to a file is no folder.
    fs::remove_file(&link).expect("the link is removed");
    let file = root.join("a-note.md");
    fs::write(&file, "a note\n").expect("a file");
    std::os::unix::fs::symlink(&file, &link).expect("the inbox links to a file");
    let refused = Inbox::locate(&RealFs, &root, &layout());
    assert!(
        matches!(refused, Err(VaultError::VaultMissing)),
        "a link to a file is refused with vault_missing: {refused:?}"
    );

    // A link to the root itself, or to a folder outside it, leaves the confinement.
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).expect("a folder outside the vault");
    for target in [&root, &outside] {
        fs::remove_file(&link).expect("the link is removed");
        std::os::unix::fs::symlink(target, &link).expect("the inbox links to a folder");
        let refused = Inbox::locate(&RealFs, &root, &layout());
        assert!(
            matches!(refused, Err(VaultError::OutsideConfinement)),
            "a link to {} is refused: {refused:?}",
            target.display()
        );
    }
}
