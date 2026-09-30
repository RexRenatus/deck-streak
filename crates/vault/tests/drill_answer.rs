//! A drill's answer is appended as the predecessor appends it, each outcome is returned for its
//! case, a failed note write leaves no answer row, and a row refuses an unticked note (SPEC-110
//! A4, A5, A7, A22).

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use deck_streak_kernel::{Db, Environment, StudyDay, UtcMillis};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::drill_notes::{AnswerOutcome, DrillNotes};
use deck_streak_vault::drill_store::{Surface, has_answer, insert_answer};
use deck_streak_vault::fs::{DirEntry, EntryKind, VaultFile, VaultFs};
use deck_streak_vault::{Rails, RealFs, VaultSettings};
use sqlx::Row;
use tempfile::TempDir;

const WHEN: &str = "2026-03-01 09:30";
const NOTE: &str = "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-02-20\nstatus: active\n---\n# Duty of care\n\nThe prompt.\n\n## Free Recall\n\n## Self-Check\n\n- [ ] the rule stated\n- [ ] **Ready for grading**\n";

fn day() -> StudyDay {
    StudyDay::from_epoch_day(20_500)
}

fn at() -> UtcMillis {
    UtcMillis::from_epoch_millis(1_770_000_000_000)
}

struct Fixture {
    _dir: TempDir,
    db: Db,
    settings: VaultSettings,
    active: PathBuf,
}

async fn fixture(notes: &[(&str, &str)]) -> Fixture {
    let dir = tempfile::tempdir().expect("a temp dir");
    let vault = dir.path().join("vault");
    let active = vault.join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    for (stem, text) in notes {
        fs::write(active.join(format!("{stem}.md")), text).expect("a note");
    }
    let env = Environment::from_vars([
        (VAULT_ROOT, vault.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    let settings = VaultSettings::from_env(&env).expect("settings");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    Fixture {
        _dir: dir,
        db,
        settings,
        active,
    }
}

fn notes<F: VaultFs>(fixture: &Fixture, fs: F) -> DrillNotes<F> {
    DrillNotes::open(&fixture.settings, fs, Rails::vendored().expect("the rails"))
        .expect("the vault opens")
}

async fn answer_rows(db: &Db) -> i64 {
    let mut tx = db.write().await.expect("a write");
    let row = sqlx::query("SELECT COUNT(*) AS n FROM drill_answers")
        .fetch_one(&mut *tx)
        .await
        .expect("a count");
    row.get::<i64, _>("n")
}

async fn answer<F: VaultFs>(
    notes: &DrillNotes<F>,
    db: &Db,
    id: &str,
    text: &str,
) -> Result<AnswerOutcome, deck_streak_vault::VaultError> {
    notes
        .answer(db, id, text, Surface::Bot, day(), WHEN, at())
        .await
}

#[tokio::test]
async fn an_answer_is_appended_as_the_predecessor_appends_it() {
    let fx = fixture(&[("irac-1", NOTE)]).await;
    let reader = notes(&fx, RealFs);
    let outcome = answer(&reader, &fx.db, "irac-1", "  The duty is owed.  \n")
        .await
        .expect("an outcome");
    assert_eq!(
        outcome,
        AnswerOutcome::Appended {
            title: "Duty of care".to_owned()
        }
    );
    let written = fs::read_to_string(fx.active.join("irac-1.md")).expect("the note");
    let expected = format!(
        "{}\n\n## Your Answer (via Telegram {WHEN})\n\nThe duty is owed.\n",
        NOTE.replace("- [ ] **Ready for grading**", "- [x] **Ready for grading**")
            .trim_end_matches('\n')
    );
    assert_eq!(written, expected);
    assert_eq!(answer_rows(&fx.db).await, 1, "one answer row");
}

#[tokio::test]
async fn each_answer_outcome_is_returned_for_its_case() {
    let ticked = NOTE.replace("- [ ] **Ready", "- [x] **Ready");
    let fx = fixture(&[("fresh", NOTE), ("done", ticked.as_str())]).await;
    fs::create_dir(fx.active.join("a-folder.md")).expect("a folder named as a note");
    let reader = notes(&fx, RealFs);
    assert_eq!(
        answer(&reader, &fx.db, "fresh", " \t\n")
            .await
            .expect("empty"),
        AnswerOutcome::EmptyAnswer
    );
    for id in ["missing", "..", "a/b", "a-folder", ""] {
        assert_eq!(
            answer(&reader, &fx.db, id, "text")
                .await
                .expect("not active"),
            AnswerOutcome::NotActive,
            "the id {id:?}"
        );
    }
    assert_eq!(
        answer(&reader, &fx.db, "done", "text")
            .await
            .expect("answered"),
        AnswerOutcome::AlreadyAnswered
    );
    assert_eq!(
        fs::read_to_string(fx.active.join("done.md")).expect("the note"),
        ticked,
        "an answered note is untouched"
    );
    let first = answer(&reader, &fx.db, "fresh", "one")
        .await
        .expect("first");
    assert!(matches!(first, AnswerOutcome::Appended { .. }));
    assert_eq!(
        answer(&reader, &fx.db, "fresh", "two")
            .await
            .expect("second"),
        AnswerOutcome::AlreadyAnswered
    );
    assert_eq!(answer_rows(&fx.db).await, 1);
}

/// A vault that refuses its next new file, so a note write fails after the row was made.
struct Failing {
    inner: RealFs,
    armed: Arc<AtomicBool>,
}

impl VaultFs for Failing {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        if self.armed.load(Ordering::SeqCst) {
            return Err(io::Error::other("the disk is refusing"));
        }
        self.inner.create_new(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }
    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.inner.sync_dir(dir)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.inner.read(path)
    }
    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.inner.kind(path)
    }
    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        self.inner.list(dir)
    }
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir(path)
    }
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }
}

#[tokio::test]
async fn a_failed_note_write_leaves_no_answer_row() {
    let fx = fixture(&[("irac-1", NOTE)]).await;
    let armed = Arc::new(AtomicBool::new(true));
    let failing = notes(
        &fx,
        Failing {
            inner: RealFs,
            armed: Arc::clone(&armed),
        },
    );
    let result = answer(&failing, &fx.db, "irac-1", "an answer").await;
    assert!(result.is_err(), "the note write fails: {result:?}");
    assert_eq!(answer_rows(&fx.db).await, 0, "the row rolled back");
    assert_eq!(
        fs::read_to_string(fx.active.join("irac-1.md")).expect("the note"),
        NOTE,
        "the note is as the owner left it"
    );
    armed.store(false, Ordering::SeqCst);
    let retry = answer(&failing, &fx.db, "irac-1", "an answer")
        .await
        .expect("the retry");
    assert!(
        matches!(retry, AnswerOutcome::Appended { .. }),
        "the drill is still answerable"
    );
}

#[tokio::test]
async fn an_answer_row_refuses_an_unticked_note() {
    let fx = fixture(&[("irac-1", NOTE)]).await;
    let mut tx = fx.db.write().await.expect("a write");
    assert!(
        insert_answer(&mut tx, "irac-1", day(), Surface::MiniApp, at())
            .await
            .expect("a row")
    );
    tx.commit().await.expect("committed");
    let mut tx = fx.db.write().await.expect("a write");
    assert!(has_answer(&mut tx, "irac-1").await.expect("a read"));
    drop(tx);
    let reader = notes(&fx, RealFs);
    assert_eq!(
        answer(&reader, &fx.db, "irac-1", "late")
            .await
            .expect("an outcome"),
        AnswerOutcome::AlreadyAnswered,
        "a row wins over an unticked marker"
    );
    assert_eq!(
        fs::read_to_string(fx.active.join("irac-1.md")).expect("the note"),
        NOTE
    );
}
