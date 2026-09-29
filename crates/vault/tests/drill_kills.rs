//! Post-green killers for the law drill's pure helpers, its notes folder walk and its grade reads
//! (SPEC-110): each asserts the whole value a mutant changes.

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Db, Environment, StudyDay, UtcMillis};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::drill_notes::DrillNotes;
use deck_streak_vault::drill_store::{GradeRow, recent_grades, record_grade};
use deck_streak_vault::drills::{
    self, ACTIVE, DrillMeta, GRADED, GradedDrill, Rollup, answered, prompt_of, read_view, rollup,
    sanitise_defer_reason, strip_frontmatter,
};
use deck_streak_vault::fs::{DirEntry, EntryKind, VaultFile, VaultFs};
use deck_streak_vault::{Rails, RealFs, StartRefusal, VaultError, VaultSettings};

fn day(n: i64) -> StudyDay {
    StudyDay::from_epoch_day(n)
}

#[test]
fn a_frontmatter_strip_keeps_each_line_break_and_each_last_line() {
    assert_eq!(
        strip_frontmatter("---\r\nk: v\r\n---\r\nbody\r\nmore"),
        "body\nmore"
    );
    assert_eq!(strip_frontmatter("---\nk: v\n---\nbody\n"), "body");
    assert_eq!(strip_frontmatter("---\nk: v\n---\nbody"), "body");
    assert_eq!(strip_frontmatter("---\nk: v\n---\na\nb\n"), "a\nb");
}

#[test]
fn a_prompt_drops_each_comment_and_keeps_the_text_around_it() {
    assert_eq!(prompt_of("a<!-- x -->b"), "ab");
    assert_eq!(prompt_of("abcdefg<!--x-->h<!--\ny\n-->i"), "abcdefghi");
    assert_eq!(
        prompt_of("keep <!-- never closed"),
        "keep <!-- never closed"
    );
}

#[test]
fn a_prompt_ends_at_each_answer_heading() {
    for heading in [
        "## Free Recall",
        "## Issue",
        "## Court & Year",
        "## From-Memory Outline",
        "## Self-Check",
    ] {
        assert_eq!(
            prompt_of(&format!("The prompt.\n{heading}\nan answer")),
            "The prompt.",
            "{heading}"
        );
    }
    assert_eq!(
        prompt_of("The prompt.\n## Other\nmore"),
        "The prompt.\n## Other\nmore"
    );
}

#[test]
fn a_deferral_reason_is_kept_unless_it_is_a_bare_block_indicator() {
    assert_eq!(sanitise_defer_reason("x"), "x");
    assert_eq!(sanitise_defer_reason("> later"), "> later");
    assert_eq!(sanitise_defer_reason("| later"), "| later");
    assert_eq!(sanitise_defer_reason(">"), "");
    assert_eq!(sanitise_defer_reason("|-"), "");
    assert_eq!(sanitise_defer_reason(">+"), "");
    assert_eq!(sanitise_defer_reason(">x"), ">x");
}

fn meta(id: &str, answered: bool, age: Option<i64>, created: Option<StudyDay>) -> DrillMeta {
    DrillMeta {
        drill_id: id.to_owned(),
        kind: "irac".to_owned(),
        subject: "Torts".to_owned(),
        title: id.to_owned(),
        created,
        age_days: age,
        answered,
        deferred: false,
    }
}

#[test]
fn the_rollup_names_the_first_of_two_equally_old_drills() {
    let subjects: BTreeSet<String> = ["Torts".to_owned()].into();
    let all = [
        meta("a", false, Some(5), Some(day(100))),
        meta("b", false, Some(5), Some(day(101))),
        meta("c", false, Some(2), Some(day(103))),
    ];
    assert_eq!(
        rollup(&all, &subjects),
        Rollup {
            active: 3,
            awaiting_grading: 0,
            oldest_age_days: Some(5),
            oldest_created: Some(day(100)),
            unmatched_active: 0,
            deferred: 0,
        }
    );
}

const READY: &str = "- [ ] **Ready for grading**";

#[test]
fn the_ready_label_and_the_self_check_heading_are_read_as_written() {
    assert!(!answered(READY));
    assert!(answered("- [x] **Ready for grading**"));
    assert!(!answered(
        "- [x] **Ready for grading soon**"
            .replace("grading soon", "gradin")
            .as_str()
    ));
    let raw = format!(
        "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-02-20\n---\n# T\n\nP.\n\n## Free Recall\n\n## Self-Check\n\n- [ ] one\n- [ ] two\n\n## Self-Checked\n\n- [ ] not a check\n\n{READY}\n"
    );
    let view = read_view("irac-1", &raw, day(20_500));
    assert_eq!(view.sections, vec!["Free Recall", "Self-Checked"]);
    assert_eq!(view.self_check, vec!["one", "two"]);
}

#[derive(Debug)]
struct Faulty {
    inner: RealFs,
    canonicalize: Option<io::ErrorKind>,
    list: Option<io::ErrorKind>,
}

impl VaultFs for Faulty {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
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
        match self.list {
            Some(kind) => Err(io::Error::from(kind)),
            None => self.inner.list(dir),
        }
    }
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir(path)
    }
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        match self.canonicalize {
            Some(kind) => Err(io::Error::from(kind)),
            None => self.inner.canonicalize(path),
        }
    }
}

fn settings_of(root: &Path) -> VaultSettings {
    let env = Environment::from_vars([
        (VAULT_ROOT, root.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    VaultSettings::from_env(&env).expect("settings")
}

fn faulty(canonicalize: Option<io::ErrorKind>, list: Option<io::ErrorKind>) -> Faulty {
    Faulty {
        inner: RealFs,
        canonicalize,
        list,
    }
}

fn open<F: VaultFs>(root: &Path, fs: F) -> Result<DrillNotes<F>, VaultError> {
    DrillNotes::open(
        &settings_of(root),
        fs,
        Rails::vendored().expect("the rails"),
    )
}

#[test]
fn a_missing_root_refuses_to_start_and_any_other_failure_is_an_io_error() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let missing = dir.path().join("nowhere");
    match open(&missing, RealFs) {
        Err(VaultError::Start(StartRefusal::RootNotADirectory)) => {}
        other => panic!("a missing root: {other:?}"),
    }
    match open(
        dir.path(),
        faulty(Some(io::ErrorKind::PermissionDenied), None),
    ) {
        Err(error @ VaultError::Io { .. }) => {
            assert_eq!(
                error.to_string(),
                "the vault could not resolve the vault root"
            );
        }
        other => panic!("an unreadable root: {other:?}"),
    }
    match open(dir.path(), faulty(Some(io::ErrorKind::NotFound), None)) {
        Err(VaultError::Start(StartRefusal::RootNotADirectory)) => {}
        other => panic!("a vanished root: {other:?}"),
    }
}

fn vault_with(graded: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    let drills = dir.path().join("11-Drills");
    fs::create_dir_all(drills.join(ACTIVE)).expect("the active folder");
    if !graded.is_empty() {
        fs::create_dir_all(drills.join(GRADED)).expect("the graded folder");
    }
    for (name, text) in graded {
        fs::write(drills.join(GRADED).join(name), text).expect("a note");
    }
    dir
}

#[test]
fn a_missing_folder_lists_none_and_any_other_list_failure_is_an_io_error() {
    let dir = vault_with(&[]);
    let notes = open(dir.path(), RealFs).expect("opens");
    assert_eq!(notes.graded().expect("none"), Vec::<GradedDrill>::new());
    let refused = open(
        dir.path(),
        faulty(None, Some(io::ErrorKind::PermissionDenied)),
    )
    .expect("opens");
    let error = refused.graded().expect_err("a list failure");
    assert_eq!(error.to_string(), "the vault could not list a folder");
    let error = refused
        .list_active(day(20_500))
        .expect_err("a list failure");
    assert_eq!(error.to_string(), "the vault could not list a folder");
    let vanished = open(dir.path(), faulty(None, Some(io::ErrorKind::NotFound))).expect("opens");
    assert_eq!(vanished.graded().expect("none"), Vec::<GradedDrill>::new());
}

#[test]
fn the_graded_folder_is_read_and_the_active_folder_is_not() {
    let dir = vault_with(&[
        (
            "irac-2026-02-20-duty.md",
            "---\ntype: drill-irac\nsubject: \"Torts\"\nstatus: graded\nxp: 18\n---\n# Duty\n",
        ),
        ("skipped.md", "---\nstatus: active\n---\n# Skipped\n"),
    ]);
    fs::write(
        dir.path()
            .join("11-Drills")
            .join(ACTIVE)
            .join("active-one.md"),
        "---\nstatus: graded\nxp: 12\n---\n# Active\n",
    )
    .expect("an active note");
    let notes = open(dir.path(), RealFs).expect("opens");
    assert_eq!(
        notes.graded().expect("the graded drills"),
        vec![GradedDrill {
            drill_id: "irac-2026-02-20-duty".to_owned(),
            kind: "irac".to_owned(),
            subject: "Torts".to_owned(),
            xp: 18,
        }]
    );
    assert_eq!(drills::ACTIVE, "Active");
    assert_eq!(drills::GRADED, "Graded");
}

fn grade(id: &str, xp: i64, subject: &str) -> GradeRow {
    GradeRow {
        drill_id: id.to_owned(),
        drill_type: "irac".to_owned(),
        subject: subject.to_owned(),
        xp,
        study_day: day(20_500),
    }
}

#[tokio::test]
async fn recent_grades_are_the_newest_of_the_subject_with_every_column() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    let mut tx = db.write().await.expect("a write");
    let mut rows = Vec::new();
    for (index, (id, xp, subject)) in [
        ("d1", 11, "Torts"),
        ("d2", 22, "Torts"),
        ("d3", 15, "Contracts"),
        ("d4", 25, "Torts"),
    ]
    .into_iter()
    .enumerate()
    {
        let row = grade(id, xp, subject);
        let at = UtcMillis::from_epoch_millis(
            1_770_000_000_000 + i64::try_from(index).unwrap_or(0) * 1000,
        );
        assert!(record_grade(&mut tx, &row, at).await.expect("recorded"));
        rows.push(row);
    }
    let found = recent_grades(&mut tx, "Torts", 2).await.expect("a read");
    assert_eq!(found, vec![rows[3].clone(), rows[1].clone()]);
    let all = recent_grades(&mut tx, "Torts", 10).await.expect("a read");
    assert_eq!(all, vec![rows[3].clone(), rows[1].clone(), rows[0].clone()]);
}
