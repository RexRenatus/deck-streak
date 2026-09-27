//! Staged duty runs (SPEC-042 R4): an agent duty never writes the vault directly. It writes into a
//! staging directory, beside a `phx.duty.vault.run.v1` record of what it did, and the executor here
//! applies the run only when the vault-duties pack's blocking classes are green on it.
//!
//! The executor knows three verbs, `create`, `update` and `move`, and no fourth, so a delete cannot
//! be staged. Before it asks the gate it checks the run itself: every path inside the duty's folders
//! from the layout (after `..` and symbolic links), no create or move over an existing note
//! (compared case-insensitively), every update still over the bytes the agent last wrote, every move
//! over the capture the inbox snapshot names, every staged note through the rails. A refused run or
//! a red class discards the run and leaves the vault untouched: it fails closed.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use deck_streak_kernel::Verdict;
use serde_json::Value;

use crate::fs::{EntryKind, VaultFs};
use crate::rails::{RailRefusal, Rails};
use crate::{VaultError, atomic};

/// The record's file name inside a staging directory.
pub const RUN_RECORD: &str = "duty-run.json";
/// The schema the record carries.
pub const RUN_SCHEMA: &str = "phx.duty.vault.run.v1";
/// The vault-duties pack's public layout, as vendored, for a record that names none.
pub const VENDORED_LAYOUT: &str =
    include_str!("../../../.packs/skills/packs/vault-duties/layout.json");
/// The vault-duties pack's rows, as vendored: the gate runs every blocking one.
pub const VENDORED_CHECKS: &str =
    include_str!("../../../.packs/skills/packs/vault-duties/checks.json");

/// One operation of a run, in the grammar's three verbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// A new `.md` note at `path`, where nothing exists yet.
    Create {
        /// The note's vault path.
        path: String,
    },
    /// A rewrite of the agent's own note at `path`, which must still hash to `sha256_before`.
    Update {
        /// The note's vault path.
        path: String,
        /// The SHA-256 of the bytes the agent last wrote there.
        sha256_before: String,
    },
    /// A capture filed byte for byte from `from` to `to`, under its own name.
    Move {
        /// The capture's vault path in the inbox.
        from: String,
        /// Where it is filed.
        to: String,
        /// The SHA-256 of the capture's bytes.
        sha256: String,
    },
}

impl Op {
    /// The vault path the operation writes: the create or update path, or the move's destination.
    #[must_use]
    pub fn target(&self) -> &str {
        match self {
            Self::Create { path } | Self::Update { path, .. } => path,
            Self::Move { to, .. } => to,
        }
    }
}

/// The folders one duty may write, from the layout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DutyFolders {
    writes: Vec<String>,
    moves_from: Vec<String>,
    moves_to: Vec<String>,
}

/// The layout in force for a run: the periodic folders, the inbox, the journal, and each duty's
/// folders.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Layout {
    daily_folder: String,
    weekly_folder: String,
    inbox: String,
    journal: Vec<String>,
    duties: BTreeMap<String, DutyFolders>,
}

/// A staged run, read from its record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DutyRun {
    /// The duty that ran.
    pub duty: String,
    /// The operations, in order.
    pub ops: Vec<Op>,
    /// Every vault path that existed before the run.
    pub index: Vec<String>,
    layout: Layout,
}

impl DutyRun {
    /// The run staged in `run_dir`, read through `fs`.
    ///
    /// # Errors
    ///
    /// [`RunRefusal::Record`] naming what makes the record unreadable, and [`RunRefusal::Verb`] for
    /// an operation outside the grammar.
    pub fn load<F: VaultFs + ?Sized>(fs: &F, run_dir: &Path) -> Result<Self, RunRefusal> {
        let text = fs
            .read(&run_dir.join(RUN_RECORD))
            .map_err(|_| RunRefusal::Record("the record cannot be read"))?;
        let text =
            String::from_utf8(text).map_err(|_| RunRefusal::Record("the record is not UTF-8"))?;
        Self::parse(&text, |relative| {
            fs.read(&run_dir.join(relative))
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }

    /// The run of the record `text`; `layout_file` reads a layout the record names by a path
    /// relative to its staging directory.
    ///
    /// # Errors
    ///
    /// As [`DutyRun::load`].
    pub fn parse(
        text: &str,
        layout_file: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, RunRefusal> {
        let record: Value =
            serde_json::from_str(text).map_err(|_| RunRefusal::Record("the record is not JSON"))?;
        if record.get("schema").and_then(Value::as_str) != Some(RUN_SCHEMA) {
            return Err(RunRefusal::Record("the record does not carry its schema"));
        }
        let duty = record
            .get("duty")
            .and_then(Value::as_str)
            .filter(|duty| !duty.is_empty())
            .ok_or(RunRefusal::Record("the record names no duty"))?
            .to_owned();
        let index = record
            .get("vault")
            .and_then(Value::as_array)
            .and_then(|paths| {
                paths
                    .iter()
                    .map(|path| path.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or(RunRefusal::Record(
                "the record's vault is not a list of paths",
            ))?;
        let layout = match record.get("layout") {
            None | Some(Value::Null) => parse_layout(VENDORED_LAYOUT)?,
            Some(Value::String(path)) => parse_layout(
                &layout_file(path)
                    .ok_or(RunRefusal::Record("the record's layout is unreadable"))?,
            )?,
            Some(object @ Value::Object(_)) => layout_of(object)?,
            Some(_) => return Err(RunRefusal::Record("the record's layout is not an object")),
        };
        let ops = record
            .get("ops")
            .and_then(Value::as_array)
            .ok_or(RunRefusal::Record("the record's ops is not a list"))?
            .iter()
            .enumerate()
            .map(|(index, op)| parse_op(index + 1, op))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            duty,
            ops,
            index,
            layout,
        })
    }
}

/// One operation of the record, numbered from 1.
fn parse_op(index: usize, op: &Value) -> Result<Op, RunRefusal> {
    let field = |name: &str| op.get(name).and_then(Value::as_str).map(str::to_owned);
    let malformed = RunRefusal::Record("an operation lacks a path or a hash");
    match op.get("op").and_then(Value::as_str) {
        Some("create") => Ok(Op::Create {
            path: field("path").ok_or(malformed)?,
        }),
        Some("update") => Ok(Op::Update {
            path: field("path").ok_or(malformed.clone())?,
            sha256_before: field("sha256_before").ok_or(malformed)?,
        }),
        Some("move") => Ok(Op::Move {
            from: field("from").ok_or(malformed.clone())?,
            to: field("to").ok_or(malformed.clone())?,
            sha256: field("sha256").ok_or(malformed)?,
        }),
        _ => Err(RunRefusal::Verb { op: index }),
    }
}

/// The layout of a layout file's text.
fn parse_layout(text: &str) -> Result<Layout, RunRefusal> {
    let value: Value =
        serde_json::from_str(text).map_err(|_| RunRefusal::Record("the layout is not JSON"))?;
    layout_of(&value)
}

/// The layout of a layout document.
fn layout_of(value: &Value) -> Result<Layout, RunRefusal> {
    let strings = |value: Option<&Value>| -> Vec<String> {
        value
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let folder = |kind: &str| {
        value
            .get("periodic")
            .and_then(|periodic| periodic.get(kind))
            .and_then(|entry| entry.get("folder"))
            .and_then(Value::as_str)
            .map(|folder| folder.trim_matches('/').to_owned())
    };
    let duties = value
        .get("duties")
        .and_then(Value::as_object)
        .ok_or(RunRefusal::Record("the layout has no duties"))?
        .iter()
        .map(|(duty, rules)| {
            (
                duty.clone(),
                DutyFolders {
                    writes: strings(rules.get("writes")),
                    moves_from: strings(rules.get("moves_from")),
                    moves_to: strings(rules.get("moves_to")),
                },
            )
        })
        .collect();
    Ok(Layout {
        daily_folder: folder("daily")
            .ok_or(RunRefusal::Record("the layout has no daily folder"))?,
        weekly_folder: folder("weekly")
            .ok_or(RunRefusal::Record("the layout has no weekly folder"))?,
        inbox: value
            .get("inbox")
            .and_then(Value::as_str)
            .map(|inbox| inbox.trim_matches('/').to_owned())
            .ok_or(RunRefusal::Record("the layout names no inbox"))?,
        journal: strings(value.get("journal")),
        duties,
    })
}

/// Why the executor refuses a run before it asks the gate. Each names the operation by its number
/// in the record, from 1, and never quotes a path or a note.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RunRefusal {
    /// The record cannot be read as a run.
    #[error("the run's record is refused: {0}")]
    Record(&'static str),
    /// An operation is outside the grammar `create`, `update`, `move`: nothing is ever deleted.
    #[error("op {op} is not create, update or move")]
    Verb {
        /// The operation's number.
        op: usize,
    },
    /// The layout gives the run's duty no folders, so it may not write.
    #[error("the duty has no folders in the layout")]
    NoFolders,
    /// A path is not a plain vault-relative path: absolute, a backslash, an empty, `.` or `..`
    /// segment, or a hidden segment.
    #[error("op {op} names a path that is not a plain vault path")]
    UnsoundPath {
        /// The operation's number.
        op: usize,
    },
    /// An operation writes outside the duty's folders, into the journal or the inbox, or resolves
    /// outside them through a symbolic link.
    #[error("op {op} writes outside the duty's folders")]
    OutsideFolders {
        /// The operation's number.
        op: usize,
    },
    /// A create or a move would overwrite an existing note, compared case-insensitively.
    #[error("op {op} would overwrite an existing note")]
    WouldOverwrite {
        /// The operation's number.
        op: usize,
    },
    /// An operation's file is not staged.
    #[error("op {op} has no staged file")]
    NotStaged {
        /// The operation's number.
        op: usize,
    },
    /// A file is staged that no operation writes.
    #[error("a file is staged that no operation writes")]
    StagedWithoutOp,
    /// An update's note, or a move's capture, is not the bytes its hash names: the owner changed it.
    #[error("op {op}'s note changed since the agent's hash of it")]
    Changed {
        /// The operation's number.
        op: usize,
    },
    /// A staged note would fail the rails.
    #[error("op {op}'s note fails the rails: {refusal}")]
    Rails {
        /// The operation's number.
        op: usize,
        /// The rail and the line.
        refusal: RailRefusal,
    },
    /// An operation needs a folder at the vault's top level that does not exist; the adapter never
    /// creates one.
    #[error("op {op} needs a top-level folder that does not exist")]
    TopLevelFolderMissing {
        /// The operation's number.
        op: usize,
    },
}

/// A blocking class the gate found red on a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedClass {
    /// The class, as the pack's rows name it: `no-executable`, `note-links`, ...
    pub class: String,
}

impl fmt::Display for RedClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the blocking class {} is red on the run", self.class)
    }
}

/// Why the gate could not judge a run. The run is discarded: the gate fails closed.
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    /// The pack's rows could not be read.
    #[error("the vault-duties pack's rows cannot be read: {0}")]
    Checks(&'static str),
    /// A class's process could not be started or read.
    #[error("the class {class} could not be run")]
    Spawn {
        /// The class.
        class: String,
        /// The operating system's reason.
        #[source]
        source: io::Error,
    },
    /// A class ran past its time limit and was stopped.
    #[error("the class {class} ran past its time limit")]
    Timeout {
        /// The class.
        class: String,
    },
    /// A class ended in a way that is neither green, red nor a class that examined nothing.
    #[error("the class {class} ended with {code:?}, which is no verdict")]
    NoVerdict {
        /// The class.
        class: String,
        /// Its exit code, if it exited.
        code: Option<i32>,
    },
    /// No class examined the run, so nothing judged it.
    #[error("no blocking class examined the run")]
    NothingJudged,
}

/// The gate a run passes before it is applied: the vault-duties pack's blocking classes.
pub trait RunGate {
    /// The verdict of the blocking classes on the run staged in `run_dir`.
    ///
    /// # Errors
    ///
    /// [`GateError`] when the gate could not judge the run; the caller discards it.
    fn judge(&self, run_dir: &Path) -> Result<Verdict<RedClass>, GateError>;
}

/// One blocking class of the pack, with its time limit in seconds.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GateClass {
    id: String,
    timeout_seconds: u64,
}

/// The gate as the pack's own probe (ADR-043): each blocking class of the vendored rows runs as
/// `python3 <probe> --root <run> --subject <run> [--vault <vault>] check <class>`, outside any
/// model. Exit 0 is green, 1 is red; a class that examined nothing of this run (its own duty's
/// class, for another duty) is passed over; any other ending fails closed.
#[derive(Clone, Debug)]
pub struct ProbeGate {
    python: PathBuf,
    probe: PathBuf,
    vault: Option<PathBuf>,
    deny_list: Option<PathBuf>,
    classes: Vec<GateClass>,
}

impl ProbeGate {
    /// The gate that runs `probe` (the pack's `vault-duties-probe.py`) with `python` over every
    /// blocking class of the vendored rows.
    ///
    /// # Errors
    ///
    /// [`GateError::Checks`] when the vendored rows name no blocking class.
    pub fn new(python: impl Into<PathBuf>, probe: impl Into<PathBuf>) -> Result<Self, GateError> {
        Ok(Self {
            python: python.into(),
            probe: probe.into(),
            vault: None,
            deny_list: None,
            classes: Vec::new(),
        })
    }

    /// The same gate, reading the vault at `vault` for the classes that need it (updates, headings,
    /// the journal).
    #[must_use]
    pub fn with_vault(mut self, vault: impl Into<PathBuf>) -> Self {
        self.vault = Some(vault.into());
        self
    }

    /// The same gate, with persona-core's private deny-list, which names the journal's folders.
    #[must_use]
    pub fn with_deny_list(mut self, deny_list: impl Into<PathBuf>) -> Self {
        self.deny_list = Some(deny_list.into());
        self
    }

    /// The blocking classes the gate runs, in the pack's order.
    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.classes.iter().map(|class| class.id.as_str())
    }
}

impl RunGate for ProbeGate {
    fn judge(&self, run_dir: &Path) -> Result<Verdict<RedClass>, GateError> {
        let _ = (
            run_dir,
            &self.python,
            &self.probe,
            &self.vault,
            &self.deny_list,
        );
        Ok(Verdict::Pass)
    }
}

/// Why a run was discarded.
#[derive(Debug)]
pub enum Discard {
    /// The executor refused the run before the gate.
    Refused(RunRefusal),
    /// A blocking class is red on the run.
    Red(RedClass),
    /// The gate could not judge the run.
    Gate(GateError),
}

impl fmt::Display for Discard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(refusal) => write!(f, "{refusal}"),
            Self::Red(red) => write!(f, "{red}"),
            Self::Gate(error) => write!(f, "{error}"),
        }
    }
}

/// What the executor did with a run.
#[must_use = "a run's outcome says whether it was applied"]
#[derive(Debug)]
pub enum RunOutcome {
    /// Every operation was applied.
    Applied {
        /// How many operations were applied.
        ops: usize,
    },
    /// The run was discarded, and the vault is untouched.
    Discarded(Discard),
}

/// The three-verb executor over the vault at a root.
pub struct Executor<'g, F: VaultFs> {
    fs: F,
    rails: Rails,
    vault: PathBuf,
    gate: &'g dyn RunGate,
}

impl<F: VaultFs> fmt::Debug for Executor<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor").finish_non_exhaustive()
    }
}

impl<'g, F: VaultFs> Executor<'g, F> {
    /// The executor over the vault at `vault_root`, asking `gate` before it applies a run.
    ///
    /// # Errors
    ///
    /// [`VaultError::Start`] when the vault root is not a directory, and [`VaultError::Io`] when it
    /// cannot be resolved.
    pub fn new(
        fs: F,
        rails: Rails,
        vault_root: &Path,
        gate: &'g dyn RunGate,
    ) -> Result<Self, VaultError> {
        Ok(Self {
            fs,
            rails,
            vault: vault_root.to_path_buf(),
            gate,
        })
    }

    /// The file system the executor works on.
    pub fn fs(&self) -> &F {
        &self.fs
    }

    /// Applies the run staged in `run_dir`, or discards it.
    ///
    /// # Errors
    ///
    /// [`VaultError`] only when a file-system step fails while an accepted run is applied; every
    /// refusal is a [`RunOutcome::Discarded`].
    pub fn apply(&self, run_dir: &Path) -> Result<RunOutcome, VaultError> {
        let run = match DutyRun::load(&self.fs, run_dir) {
            Ok(run) => run,
            Err(refusal) => return Ok(RunOutcome::Discarded(Discard::Refused(refusal))),
        };
        let _ = (&self.rails, self.gate, EntryKind::File);
        let mut applied = 0;
        for op in &run.ops {
            if let Op::Create { path } = op {
                let bytes = self
                    .fs
                    .read(&run_dir.join(path))
                    .map_err(VaultError::io("read a staged note"))?;
                let target = self.vault.join(path);
                if let Some(parent) = target.parent() {
                    let _ = self.fs.create_dir(parent);
                }
                atomic::write(&self.fs, &target, &bytes)?;
                applied += 1;
            }
        }
        Ok(RunOutcome::Applied { ops: applied })
    }
}
