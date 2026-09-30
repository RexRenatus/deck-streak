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
    /// Fail the resolve only for the drill folders, so the vault root still opens.
    folders_only: bool,
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
        let is_folder = path.ends_with(ACTIVE) || path.ends_with(GRADED);
        match self.canonicalize {
            Some(kind) if is_folder || !self.folders_only => Err(io::Error::from(kind)),
            _ => self.inner.canonicalize(path),
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
        folders_only: false,
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
fn a_folder_that_cannot_be_resolved_is_an_io_error_and_only_a_missing_one_is_none() {
    let dir = vault_with(&[]);
    let mut fault = faulty(Some(io::ErrorKind::PermissionDenied), None);
    fault.folders_only = true;
    let refused = open(dir.path(), fault).expect("the root still opens");
    let error = refused.graded().expect_err("a resolve failure");
    assert_eq!(error.to_string(), "the vault could not resolve a folder");
    let error = refused
        .list_active(day(20_500))
        .expect_err("a resolve failure");
    assert_eq!(error.to_string(), "the vault could not resolve a folder");
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

const OUTSIDE_NOTE: &str = "---\ntype: drill-irac\nsubject: \"Elsewhere\"\nstatus: graded\nxp: 25\n---\n# Outside\n\nOutside text.\n\n- [ ] **Ready for grading**\n";

#[tokio::test]
async fn a_link_in_the_drills_folders_is_never_read_or_written_through() {
    let outside = tempfile::tempdir().expect("a folder outside the vault");
    let note = outside.path().join("note.md");
    fs::write(&note, OUTSIDE_NOTE).expect("a note outside the vault");
    let dir = vault_with(&[]);
    let drills_folder = dir.path().join("11-Drills");
    fs::create_dir_all(drills_folder.join(GRADED)).expect("the graded folder");
    std::os::unix::fs::symlink(&note, drills_folder.join(ACTIVE).join("linked.md"))
        .expect("a linked active note");
    std::os::unix::fs::symlink(&note, drills_folder.join(GRADED).join("linked.md"))
        .expect("a linked graded note");
    let notes = open(dir.path(), RealFs).expect("opens");
    assert_eq!(
        notes.list_active(day(20_500)).expect("a list"),
        Vec::<DrillMeta>::new(),
        "a linked note is not listed"
    );
    assert!(notes.view("linked", day(20_500)).is_none(), "nor viewed");
    assert_eq!(
        notes.graded().expect("the graded drills"),
        Vec::<GradedDrill>::new(),
        "nor paid"
    );

    let linked = vault_with(&[]);
    let active = linked.path().join("11-Drills").join(ACTIVE);
    fs::remove_dir(&active).expect("the active folder goes");
    std::os::unix::fs::symlink(outside.path(), &active).expect("a linked active folder");
    std::os::unix::fs::symlink(outside.path(), linked.path().join("11-Drills").join(GRADED))
        .expect("a linked graded folder");
    let notes = open(linked.path(), RealFs).expect("opens");
    assert!(matches!(
        notes.list_active(day(20_500)),
        Err(VaultError::NotAFolder)
    ));
    assert!(matches!(notes.graded(), Err(VaultError::NotAFolder)));
    assert!(notes.view("note", day(20_500)).is_none());
    let db = Db::open(&linked.path().join("deck_streak.db"))
        .await
        .expect("the database");
    let answered = notes
        .answer(
            &db,
            "note",
            "an answer",
            deck_streak_vault::drill_store::Surface::Bot,
            day(20_500),
            "2026-03-01 09:30",
            UtcMillis::from_epoch_millis(1_770_000_000_000),
        )
        .await;
    assert!(
        matches!(answered, Err(VaultError::NotAFolder)),
        "{answered:?}"
    );
    assert_eq!(
        fs::read_to_string(&note).expect("the outside note"),
        OUTSIDE_NOTE,
        "nothing is written outside the vault"
    );
}

/// What a link in the drills folders points at (the class rule: no link is followed, whatever it
/// points at, and a link to a place inside the vault is refused too).
#[derive(Clone, Copy, Debug)]
enum Target {
    File,
    Dir,
    LinkToLink,
    Dangling,
    InsideVault,
}

/// Where the link sits.
#[derive(Clone, Copy, Debug)]
enum Place {
    NoteInActive,
    NoteInGraded,
    ActiveFolder,
    GradedFolder,
}

/// What the adapter is asked to do.
#[derive(Clone, Copy, Debug)]
enum Op {
    List,
    View,
    Pay,
    Answer,
}

const TARGETS: [Target; 5] = [
    Target::File,
    Target::Dir,
    Target::LinkToLink,
    Target::Dangling,
    Target::InsideVault,
];
const PLACES: [Place; 4] = [
    Place::NoteInActive,
    Place::NoteInGraded,
    Place::ActiveFolder,
    Place::GradedFolder,
];
const OPS: [Op; 4] = [Op::List, Op::View, Op::Pay, Op::Answer];

/// The text no result may carry: it is in every file a link can lead to.
const SENTINEL: &str = "Outside text.";

/// A vault, a place outside it, and the files whose bytes and times must not change.
struct Member {
    vault: tempfile::TempDir,
    _outside: tempfile::TempDir,
    watched: Vec<PathBuf>,
}

/// The link's target for `place`, built beside the vault or inside it.
fn target_path(target: Target, place: Place, outside: &Path, vault: &Path) -> PathBuf {
    let folder_place = matches!(place, Place::ActiveFolder | Place::GradedFolder);
    match target {
        Target::File => outside.join("note.md"),
        Target::Dir => outside.join("dir"),
        Target::LinkToLink if folder_place => outside.join("hop-dir"),
        Target::LinkToLink => outside.join("hop-note"),
        Target::Dangling => outside.join("missing"),
        Target::InsideVault if folder_place => vault.join("elsewhere").join("dir"),
        Target::InsideVault => vault.join("elsewhere").join("inside.md"),
    }
}

/// Builds one member, or names why it cannot be built here.
fn build(target: Target, place: Place) -> Result<Member, String> {
    let outside = tempfile::tempdir().expect("a place outside the vault");
    let vault = tempfile::tempdir().expect("a vault");
    let out = outside.path();
    fs::write(out.join("note.md"), OUTSIDE_NOTE).expect("an outside note");
    fs::create_dir_all(out.join("dir")).expect("an outside folder");
    fs::write(out.join("dir").join("note.md"), OUTSIDE_NOTE).expect("a note");
    fs::write(out.join("dir").join("linked.md"), OUTSIDE_NOTE).expect("a note");
    let inside = vault.path().join("elsewhere");
    fs::create_dir_all(inside.join("dir")).expect("an inside folder");
    fs::write(inside.join("inside.md"), OUTSIDE_NOTE).expect("an inside note");
    fs::write(inside.join("dir").join("note.md"), OUTSIDE_NOTE).expect("a note");
    fs::write(inside.join("dir").join("linked.md"), OUTSIDE_NOTE).expect("a note");
    let hop = |name: &str, to: &Path| {
        std::os::unix::fs::symlink(to, out.join(name)).map_err(|error| error.to_string())
    };
    hop("hop-note", &out.join("note.md"))?;
    hop("hop-dir", &out.join("dir"))?;
    let drills = vault.path().join("11-Drills");
    let link_to = target_path(target, place, out, vault.path());
    let (linked_active, linked_graded) = (
        matches!(place, Place::ActiveFolder),
        matches!(place, Place::GradedFolder),
    );
    for (name, linked) in [(ACTIVE, linked_active), (GRADED, linked_graded)] {
        if linked {
            fs::create_dir_all(&drills).expect("the drills folder");
            std::os::unix::fs::symlink(&link_to, drills.join(name))
                .map_err(|error| error.to_string())?;
        } else {
            fs::create_dir_all(drills.join(name)).expect("a folder");
        }
    }
    match place {
        Place::NoteInActive => {
            std::os::unix::fs::symlink(&link_to, drills.join(ACTIVE).join("linked.md"))
        }
        Place::NoteInGraded => {
            std::os::unix::fs::symlink(&link_to, drills.join(GRADED).join("linked.md"))
        }
        Place::ActiveFolder | Place::GradedFolder => Ok(()),
    }
    .map_err(|error| error.to_string())?;
    let watched = vec![
        out.join("note.md"),
        out.join("dir").join("note.md"),
        out.join("dir").join("linked.md"),
        inside.join("inside.md"),
        inside.join("dir").join("note.md"),
        inside.join("dir").join("linked.md"),
    ];
    Ok(Member {
        vault,
        _outside: outside,
        watched,
    })
}

/// Whether `op` reads the folder the link replaces, and so must be refused with
/// [`VaultError::NotAFolder`] before that folder is listed (R1). A dangling link resolves nowhere,
/// so it reads as a missing folder, which lists empty (R1); every other member lists empty too.
fn refused(target: Target, place: Place, op: Op) -> bool {
    let linked_folder = match op {
        Op::List => matches!(place, Place::ActiveFolder),
        Op::Pay => matches!(place, Place::GradedFolder),
        Op::View | Op::Answer => false,
    };
    linked_folder && !matches!(target, Target::Dangling)
}

/// A member's listing: [`VaultError::NotAFolder`] when `refused`, else an empty list (R1).
fn check_listing<T: std::fmt::Debug>(label: &str, refused: bool, got: Result<Vec<T>, VaultError>) {
    if refused {
        assert!(
            matches!(got, Err(VaultError::NotAFolder)),
            "{label}: a linked folder is refused before it is listed: {got:?}"
        );
    } else {
        let got = got.expect("a listing");
        assert!(got.is_empty(), "{label}: listed {got:?}");
    }
}

/// The bytes and modification time of every watched file.
fn fingerprint(watched: &[PathBuf]) -> Vec<(Vec<u8>, std::time::SystemTime)> {
    watched
        .iter()
        .map(|path| {
            (
                fs::read(path).expect("a watched file"),
                fs::metadata(path)
                    .and_then(|meta| meta.modified())
                    .expect("its time"),
            )
        })
        .collect()
}

#[tokio::test]
async fn no_link_in_any_placement_is_read_listed_paid_from_or_written_through() {
    // The control: with no link the same four operations do reach a note, so the refusals below
    // are the gate's and not an empty fixture's.
    let control = vault_with(&[("real.md", OUTSIDE_NOTE)]);
    fs::write(
        control
            .path()
            .join("11-Drills")
            .join(ACTIVE)
            .join("real.md"),
        OUTSIDE_NOTE,
    )
    .expect("an active note");
    let notes = open(control.path(), RealFs).expect("opens");
    assert_eq!(notes.list_active(day(20_500)).expect("a list").len(), 1);
    assert!(notes.view("real", day(20_500)).is_some());
    assert_eq!(notes.graded().expect("graded").len(), 1);

    let mut examined = 0_usize;
    let mut unbuilt: Vec<String> = Vec::new();
    for target in TARGETS {
        for place in PLACES {
            let member = match build(target, place) {
                Ok(member) => member,
                Err(why) => {
                    unbuilt.extend(
                        OPS.iter()
                            .map(|op| format!("{target:?}/{place:?}/{op:?}: {why}")),
                    );
                    continue;
                }
            };
            let before = fingerprint(&member.watched);
            let notes = open(member.vault.path(), RealFs).expect("opens");
            for op in OPS {
                let label = format!("{target:?} {place:?} {op:?}");
                match op {
                    Op::List => check_listing(
                        &label,
                        refused(target, place, op),
                        notes.list_active(day(20_500)),
                    ),
                    Op::View => {
                        for id in ["linked", "note"] {
                            assert!(
                                notes.view(id, day(20_500)).is_none(),
                                "{label}: viewed {id}"
                            );
                        }
                    }
                    Op::Pay => check_listing(&label, refused(target, place, op), notes.graded()),
                    Op::Answer => {
                        let db = Db::open(&member.vault.path().join("deck_streak.db"))
                            .await
                            .expect("the database");
                        for id in ["linked", "note"] {
                            let outcome = notes
                                .answer(
                                    &db,
                                    id,
                                    "an answer",
                                    deck_streak_vault::drill_store::Surface::Bot,
                                    day(20_500),
                                    "2026-03-01 09:30",
                                    UtcMillis::from_epoch_millis(1_770_000_000_000),
                                )
                                .await;
                            assert!(
                                matches!(
                                    outcome,
                                    Err(_) | Ok(deck_streak_vault::drill_notes::AnswerOutcome::NotActive)
                                ),
                                "{label}: answered {id}: {outcome:?}"
                            );
                            assert!(
                                !format!("{outcome:?}").contains(SENTINEL),
                                "{label}: the outside text surfaced"
                            );
                        }
                    }
                }
                examined += 1;
            }
            assert!(
                fingerprint(&member.watched) == before,
                "{target:?} {place:?}: a file behind a link changed"
            );
        }
    }
    println!("examined {examined} placements; unbuilt on this file system: {unbuilt:?}");
    assert_eq!(
        examined + unbuilt.len(),
        TARGETS.len() * PLACES.len() * OPS.len(),
        "every member is examined or named"
    );
    assert!(examined > 0, "the population is not empty");
}
