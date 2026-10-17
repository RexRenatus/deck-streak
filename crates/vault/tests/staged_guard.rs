//! A staged run never writes a journal folder through a link the vault gains after the run was
//! checked (SPEC-118 R5, #56; ADR-316): each folder it creates, each note it writes and each
//! capture it files, by target and by source, is refused at the moment it is applied when its path
//! resolves into the journal. The run stops there, the operations before it stay applied, and the
//! journal is unchanged.
//!
//! The vault here gains the link the way the owner's devices could: a test file system replaces a
//! plain folder with a link into the journal once the run's first operation has landed, after both
//! of the executor's checks. Each arm runs the same vault without the link first, so an arm whose
//! run never reached its second operation cannot pass (the tdd pack's positive control).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use deck_streak_kernel::Verdict;
use deck_streak_vault::sha256;
use deck_streak_vault::{
    DirEntry, EntryKind, Executor, GateError, Rails, RealFs, RedClass, RunGate, RunOutcome,
    VaultError, VaultFile, VaultFs,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// The journal folder the layout names.
const JOURNAL: &str = "Journal";

/// A layout with a journal, whose daily note writes `Daily` and whose curator files from the inbox
/// into `12-Readings`.
fn layout() -> Value {
    json!({
        "schema": "phx.duty.vault.layout.v1",
        "periodic": {
            "daily": {"folder": "Daily", "format": "[Today]"},
            "weekly": {"folder": "Weekly", "format": "[This week]"}
        },
        "inbox": "90-Inbox",
        "folders": {"12-Readings": "title", "90-Inbox": "any"},
        "duties": {
            "daily-note": {"writes": ["@daily"]},
            "inbox-curator": {"moves_from": ["90-Inbox"], "moves_to": ["12-Readings"]}
        },
        "journal": [JOURNAL]
    })
}

/// A note the daily-note duty writes.
const NOTE: &str = "# Today\n\nThe day's note.\n";

/// A capture in the inbox, as the owner's device left it.
const CAPTURE: &str = "# A capture\n\nFiled by the curator.\n";

/// What the journal holds before every run: a note at its top, and a note in a folder of its own.
const JOURNAL_NOTES: [(&str, &str); 2] = [
    ("Journal/Entry.md", "# An entry\n\nThe owner's own.\n"),
    (
        "Journal/Sub/Second.md",
        "# A second entry\n\nThe owner's too.\n",
    ),
];

/// The SHA-256 of `text`, as a run's record names it.
fn hash(text: &str) -> String {
    sha256::hex(&sha256::digest(text.as_bytes()))
}

/// The record of `duty`'s run of `ops`.
fn run_of(duty: &str, ops: &Value) -> Value {
    json!({
        "schema": "phx.duty.vault.run.v1",
        "duty": duty,
        "vault": [],
        "layout": layout(),
        "ops": ops
    })
}

/// A staging directory holding `record` and the files `staged` names, at their vault paths.
fn stage(record: &Value, staged: &[(&str, &str)]) -> TempDir {
    let run = tempfile::tempdir().expect("a staging directory");
    fs::write(
        run.path().join("duty-run.json"),
        serde_json::to_string_pretty(record).expect("a record"),
    )
    .expect("the record");
    for (path, text) in staged {
        let file = run.path().join(path);
        fs::create_dir_all(file.parent().expect("a folder")).expect("a staged folder");
        fs::write(file, text).expect("a staged file");
    }
    run
}

/// A vault holding the journal's notes, an empty `Journal/Two`, the daily-notes folder with a plain
/// folder `Daily/Two`, `12-Readings/Sub`, and two captures in the inbox, one of them in a folder.
fn vault() -> TempDir {
    let vault = tempfile::tempdir().expect("a temporary vault");
    let notes = JOURNAL_NOTES.into_iter().chain([
        ("90-Inbox/A.md", CAPTURE),
        ("90-Inbox/B.md", CAPTURE),
        ("90-Inbox/Sub/Second.md", CAPTURE),
    ]);
    for (path, text) in notes {
        let file = vault.path().join(path);
        fs::create_dir_all(file.parent().expect("a folder")).expect("a vault folder");
        fs::write(file, text).expect("a vault note");
    }
    for folder in ["Journal/Two", "Daily/Two", "12-Readings/Sub"] {
        fs::create_dir_all(vault.path().join(folder)).expect("a vault folder");
    }
    vault
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every file and folder under `root`, each file with its bytes, keyed by its relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("a readable folder") {
            let path = entry.expect("a readable entry").path();
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .to_path_buf();
            if path.is_symlink() {
                found.insert(relative, Some(b"a symbolic link".to_vec()));
            } else if path.is_dir() {
                pending.push(path);
                found.insert(relative, None);
            } else {
                found.insert(relative, Some(fs::read(&path).expect("a readable file")));
            }
        }
    }
    examined("journal entries", found.into_iter().collect())
        .into_iter()
        .collect()
}

/// A gate that finds every run green.
struct GreenGate;

impl RunGate for GreenGate {
    fn judge(&self, _run_dir: &Path) -> Result<Verdict<RedClass>, GateError> {
        Ok(Verdict::Pass)
    }
}

/// The real file system, which replaces the plain folder `folder` with a link to `into` once a
/// rename has landed a file at a path ending in `landed`: the link a device makes while the run is
/// applied, after the executor checked it.
struct Relinking {
    landed: PathBuf,
    relink: Option<(PathBuf, PathBuf)>,
    done: Cell<bool>,
}

impl Relinking {
    /// The file system that relinks `folder` to `into` once `landed` has landed.
    fn linking(landed: &str, folder: &Path, into: &Path) -> Self {
        Self {
            landed: PathBuf::from(landed),
            relink: Some((folder.to_path_buf(), into.to_path_buf())),
            done: Cell::new(false),
        }
    }

    /// The same file system, which never relinks: the positive control.
    fn plain(landed: &str) -> Self {
        Self {
            landed: PathBuf::from(landed),
            relink: None,
            done: Cell::new(false),
        }
    }
}

impl VaultFs for Relinking {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        RealFs.create_new(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        RealFs.rename(from, to)?;
        if !self.done.get() && to.ends_with(&self.landed) {
            self.done.set(true);
            if let Some((folder, into)) = &self.relink {
                fs::remove_dir_all(folder)?;
                symlink(into, folder)?;
            }
        }
        Ok(())
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        RealFs.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
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

/// One arm: a two-operation run, its staged files, the path its first operation lands, the plain
/// folder relinked after that, and the journal folder the link reaches.
struct Arm {
    run: Value,
    staged: Vec<(&'static str, &'static str)>,
    landed: &'static str,
    relinked: &'static str,
    into: &'static str,
}

/// What the executor did with `arm` over a fresh vault, without the link and then with it, and the
/// vault each left.
fn both(arm: &Arm) -> ((RunOutcome, TempDir), (RunOutcome, TempDir)) {
    let apply = |fs: Relinking, vault: TempDir| {
        let run = stage(&arm.run, &arm.staged);
        let executor = Executor::new(
            fs,
            Rails::vendored().expect("the vendored rails"),
            vault.path(),
            &GreenGate,
        )
        .expect("the executor");
        let outcome = executor.apply(run.path()).expect("the executor runs");
        (outcome, vault)
    };
    let control = apply(Relinking::plain(arm.landed), vault());
    let linked_vault = vault();
    let fs = Relinking::linking(
        arm.landed,
        &linked_vault.path().join(arm.relinked),
        &linked_vault.path().join(arm.into),
    );
    (control, apply(fs, linked_vault))
}

/// Runs `arm`: without the link every operation is applied and `second` lands at its plain path;
/// with it, the journal is unchanged, the run stops at its second operation, and the first stays
/// applied.
fn guarded(arm: &Arm, first: &str, second: &str) {
    let ((control, plain_vault), (outcome, vault)) = both(arm);
    assert!(
        matches!(control, RunOutcome::Applied { ops: 2 }),
        "without the link, both operations are applied: {control:?}"
    );
    assert!(
        plain_vault.path().join(second).is_file(),
        "without the link, the second operation lands at {second}"
    );

    // The journal as the fixture made it, read from a vault no run has touched.
    let fixture = self::vault();
    assert_eq!(
        snapshot(&vault.path().join(JOURNAL)),
        snapshot(&fixture.path().join(JOURNAL)),
        "the run reached the journal through the link"
    );
    assert!(
        matches!(
            outcome,
            RunOutcome::Stopped {
                applied: 1,
                refusal: VaultError::JournalRefused
            }
        ),
        "the run did not stop at the operation the link reaches: {outcome:?}"
    );
    assert!(
        vault.path().join(first).is_file(),
        "the operation before the refusal stays applied: {first}"
    );
}

#[test]
fn a_note_written_through_a_new_link_into_the_journal_is_refused() {
    let arm = Arm {
        run: run_of(
            "daily-note",
            &json!([
                {"op": "create", "path": "Daily/Today.md"},
                {"op": "create", "path": "Daily/Two/Today.md"}
            ]),
        ),
        staged: vec![("Daily/Today.md", NOTE), ("Daily/Two/Today.md", NOTE)],
        landed: "Daily/Today.md",
        relinked: "Daily/Two",
        into: "Journal/Two",
    };
    guarded(&arm, "Daily/Today.md", "Daily/Two/Today.md");
}

#[test]
fn a_folder_created_through_a_new_link_into_the_journal_is_refused() {
    let arm = Arm {
        run: run_of(
            "daily-note",
            &json!([
                {"op": "create", "path": "Daily/Today.md"},
                {"op": "create", "path": "Daily/Two/Deeper/Today.md"}
            ]),
        ),
        staged: vec![
            ("Daily/Today.md", NOTE),
            ("Daily/Two/Deeper/Today.md", NOTE),
        ],
        landed: "Daily/Today.md",
        relinked: "Daily/Two",
        into: "Journal/Two",
    };
    guarded(&arm, "Daily/Today.md", "Daily/Two/Deeper/Today.md");
}

#[test]
fn a_capture_filed_through_a_new_link_into_the_journal_is_refused() {
    let arm = Arm {
        run: run_of(
            "inbox-curator",
            &json!([
                {"op": "move", "from": "90-Inbox/A.md", "to": "12-Readings/A.md", "sha256": hash(CAPTURE)},
                {"op": "move", "from": "90-Inbox/B.md", "to": "12-Readings/Sub/B.md", "sha256": hash(CAPTURE)}
            ]),
        ),
        staged: vec![
            ("12-Readings/A.md", CAPTURE),
            ("12-Readings/Sub/B.md", CAPTURE),
        ],
        landed: "12-Readings/A.md",
        relinked: "12-Readings/Sub",
        into: "Journal/Two",
    };
    guarded(&arm, "12-Readings/A.md", "12-Readings/Sub/B.md");
}

#[test]
fn a_capture_taken_through_a_new_link_out_of_the_journal_is_refused() {
    let arm = Arm {
        run: run_of(
            "inbox-curator",
            &json!([
                {"op": "move", "from": "90-Inbox/A.md", "to": "12-Readings/A.md", "sha256": hash(CAPTURE)},
                {"op": "move", "from": "90-Inbox/Sub/Second.md", "to": "12-Readings/Second.md", "sha256": hash(CAPTURE)}
            ]),
        ),
        staged: vec![
            ("12-Readings/A.md", CAPTURE),
            ("12-Readings/Second.md", CAPTURE),
        ],
        landed: "12-Readings/A.md",
        relinked: "90-Inbox/Sub",
        into: "Journal/Sub",
    };
    guarded(&arm, "12-Readings/A.md", "12-Readings/Second.md");
}
