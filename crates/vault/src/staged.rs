//! Staged duty runs (SPEC-042 R4): an agent duty never writes the vault directly. It writes into a
//! staging directory, beside a `phx.duty.vault.run.v1` record of what it did, and the executor here
//! applies the run only when the vault-duties pack's blocking classes are green on it.
//!
//! The executor knows three verbs, `create`, `update` and `move`, and no fourth, so a delete cannot
//! be staged. Before it asks the gate it checks the run itself: every path plain and inside the
//! duty's folders from the layout (after `..` and symbolic links), the staged files exactly the
//! operations' files, no create or move over an existing note (compared case-insensitively), every
//! update still over the bytes the agent last wrote, every move over the capture its hash names,
//! every staged note through the rails. A refused run or a red class discards the run and leaves the
//! vault untouched: it fails closed. The checks run again after the gate, before the first write,
//! because the owner's devices keep writing the vault while the gate runs. Each folder, note and
//! capture is then guarded at the moment it is applied: one whose path resolves into a journal
//! folder, through a link the vault gained after the checks, stops the run there (SPEC-118 R5,
//! ADR-316).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use deck_streak_kernel::Verdict;
use serde_json::Value;

use crate::fs::{DirEntry, EntryKind, JournalGuard, VaultFile, VaultFs};
use crate::rails::{RailRefusal, Rails};
use crate::{VaultError, atomic, sha256};

/// The record's file name inside a staging directory.
pub const RUN_RECORD: &str = "duty-run.json";
/// The schema the record carries.
pub const RUN_SCHEMA: &str = "phx.duty.vault.run.v1";
/// The default layout, for a record that names none: the crate's own `data/layout.json`, which
/// keeps the fields of the vault-duties pack's public layout that the executor reads (ADR-069).
pub const VENDORED_LAYOUT: &str = include_str!("../data/layout.json");
/// The gate's classes: the crate's own `data/gate-classes.json`, which keeps each of the
/// vault-duties pack's rows by its id, severity and time limit. The gate runs every blocking one.
pub const VENDORED_CHECKS: &str = include_str!("../data/gate-classes.json");

/// How long the gate waits between two looks at a class's process, in milliseconds.
const POLL_MILLIS: u64 = 20;
/// A class's time limit when its row names none, in seconds: the pack's own default.
const DEFAULT_TIMEOUT_SECONDS: u64 = 120;

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

    /// Every vault path the operation names.
    fn paths(&self) -> Vec<&str> {
        match self {
            Self::Create { path } | Self::Update { path, .. } => vec![path],
            Self::Move { from, to, .. } => vec![from, to],
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
                    .map(|item| item.trim_matches('/').to_owned())
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
    /// A staged entry is a symbolic link or not a regular file, which the executor never reads.
    #[error("a staged entry is not a regular file")]
    StagedNotAFile,
    /// An update's note, or a move's capture, is not the bytes its hash names: the owner changed it.
    #[error("op {op}'s note changed since the agent's hash of it")]
    Changed {
        /// The operation's number.
        op: usize,
    },
    /// A staged note is not UTF-8 text, so the rails cannot read it.
    #[error("op {op}'s note is not UTF-8 text")]
    NotText {
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

/// The gate as the pack's own probe (ADR-043): each blocking class of the owned rows runs as
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
    /// blocking class of the owned rows.
    ///
    /// # Errors
    ///
    /// [`GateError::Checks`] when the owned rows name no blocking class.
    pub fn new(python: impl Into<PathBuf>, probe: impl Into<PathBuf>) -> Result<Self, GateError> {
        Ok(Self {
            python: python.into(),
            probe: probe.into(),
            vault: None,
            deny_list: None,
            classes: blocking_classes(VENDORED_CHECKS)?,
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

    /// Runs `class` over `run_dir`: its exit code, or `None` when a signal ended it, and its
    /// standard output. A class that outlives its time limit is stopped.
    fn run_class(
        &self,
        class: &GateClass,
        run_dir: &Path,
    ) -> Result<(Option<i32>, String), GateError> {
        let spawn_error = |source| GateError::Spawn {
            class: class.id.clone(),
            source,
        };
        let mut command = Command::new(&self.python);
        command
            .arg(&self.probe)
            .arg("--root")
            .arg(run_dir)
            .arg("--subject")
            .arg(run_dir);
        if let Some(vault) = &self.vault {
            command.arg("--vault").arg(vault);
        }
        if let Some(deny_list) = &self.deny_list {
            command.arg("--deny-list").arg(deny_list);
        }
        // The probe's private inputs come only from this gate's own arguments, and it writes no
        // bytecode beside the probe's scripts.
        command
            .arg("check")
            .arg(&class.id)
            .env_remove("PERSONA_CORE_DENY_LIST")
            .env_remove("VAULT_DUTIES_LAYOUT")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(spawn_error)?;
        let stdout = child.stdout.take();
        let reader = thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut stdout) = stdout {
                let _read = stdout.read_to_string(&mut text);
            }
            text
        });
        // The wait is counted in polls, so no clock is read.
        let polls = class.timeout_seconds.saturating_mul(1000) / POLL_MILLIS;
        for _ in 0..=polls {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let text = reader.join().unwrap_or_default();
                    return Ok((status.code(), text));
                }
                Ok(None) => thread::sleep(Duration::from_millis(POLL_MILLIS)),
                Err(source) => return Err(spawn_error(source)),
            }
        }
        let _killed = child.kill();
        let _reaped = child.wait();
        Err(GateError::Timeout {
            class: class.id.clone(),
        })
    }
}

/// The blocking classes of a `checks.json` text, in its order.
fn blocking_classes(text: &str) -> Result<Vec<GateClass>, GateError> {
    let rows: Value =
        serde_json::from_str(text).map_err(|_| GateError::Checks("the rows are not JSON"))?;
    let classes = rows
        .get("checks")
        .and_then(Value::as_array)
        .ok_or(GateError::Checks("the rows hold no checks"))?
        .iter()
        .filter(|row| row.get("severity").and_then(Value::as_str) == Some("block"))
        .map(|row| {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .ok_or(GateError::Checks("a blocking row has no id"))?;
            let timeout_seconds = row
                .get("probe")
                .and_then(|probe| probe.get("timeout_seconds"))
                .and_then(Value::as_u64)
                .unwrap_or(DEFAULT_TIMEOUT_SECONDS);
            Ok(GateClass {
                id: id.to_owned(),
                timeout_seconds,
            })
        })
        .collect::<Result<Vec<_>, GateError>>()?;
    if classes.is_empty() {
        return Err(GateError::Checks("no row is blocking"));
    }
    Ok(classes)
}

impl RunGate for ProbeGate {
    fn judge(&self, run_dir: &Path) -> Result<Verdict<RedClass>, GateError> {
        let mut judged = 0_usize;
        for class in &self.classes {
            let (code, stdout) = self.run_class(class, run_dir)?;
            let examined_nothing = format!("{}: VOID: examined nothing", class.id);
            match code {
                Some(0) => judged += 1,
                Some(1) => {
                    return Ok(Verdict::Refuse(RedClass {
                        class: class.id.clone(),
                    }));
                }
                Some(3) if stdout.lines().any(|line| line == examined_nothing) => {}
                code => {
                    return Err(GateError::NoVerdict {
                        class: class.id.clone(),
                        code,
                    });
                }
            }
        }
        if judged == 0 {
            return Err(GateError::NothingJudged);
        }
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
    /// An operation would have reached a journal folder when it was applied, through a link the
    /// vault gained after the run was checked: the run stopped there, before that operation wrote
    /// anything (SPEC-118 R5, #56). The operations before it stay applied (ADR-316).
    Stopped {
        /// How many operations were applied before the refusal.
        applied: usize,
        /// The refusal: [`VaultError::JournalRefused`].
        refusal: VaultError,
    },
}

/// Where an operation's path lands, after the links of the folders that already exist.
enum Placement {
    /// Inside the folder the operation may write.
    Inside,
    /// Outside it, or outside the vault.
    Outside,
    /// Its top-level folder does not exist.
    TopLevelMissing,
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
        let not_a_directory = || VaultError::Start(crate::StartRefusal::RootNotADirectory);
        let vault = match fs.canonicalize(vault_root) {
            Ok(vault) => vault,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(not_a_directory()),
            Err(error) => return Err(VaultError::io("resolve the vault root")(error)),
        };
        if fs
            .kind(&vault)
            .map_err(VaultError::io("read the vault root"))?
            != Some(EntryKind::Dir)
        {
            return Err(not_a_directory());
        }
        Ok(Self {
            fs,
            rails,
            vault,
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
    /// [`VaultError`] only when a file-system step fails while a run is checked or applied; every
    /// refusal is a [`RunOutcome::Discarded`].
    pub fn apply(&self, run_dir: &Path) -> Result<RunOutcome, VaultError> {
        let run = match DutyRun::load(&self.fs, run_dir) {
            Ok(run) => run,
            Err(refusal) => return Ok(RunOutcome::Discarded(Discard::Refused(refusal))),
        };
        if let Err(refusal) = self.check(&run, run_dir)? {
            return Ok(RunOutcome::Discarded(Discard::Refused(refusal)));
        }
        match self.gate.judge(run_dir) {
            Ok(Verdict::Pass) => {}
            Ok(Verdict::Refuse(red)) => return Ok(RunOutcome::Discarded(Discard::Red(red))),
            Err(error) => return Ok(RunOutcome::Discarded(Discard::Gate(error))),
        }
        // The owner's devices keep writing the vault while the gate runs: check again.
        if let Err(refusal) = self.check(&run, run_dir)? {
            return Ok(RunOutcome::Discarded(Discard::Refused(refusal)));
        }
        for (applied, op) in run.ops.iter().enumerate() {
            match self.apply_op(&run, op, run_dir) {
                Ok(()) => {}
                Err(VaultError::JournalRefused) => {
                    return Ok(RunOutcome::Stopped {
                        applied,
                        refusal: VaultError::JournalRefused,
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(RunOutcome::Applied { ops: run.ops.len() })
    }

    /// The executor's own checks of a run, in order: plain paths, the duty's folders, the staged
    /// files, and then each operation against the vault as it stands.
    fn check(&self, run: &DutyRun, run_dir: &Path) -> Result<Result<(), RunRefusal>, VaultError> {
        let Some(rules) = run.layout.duties.get(&run.duty).filter(|rules| {
            !(rules.writes.is_empty() && rules.moves_from.is_empty() && rules.moves_to.is_empty())
        }) else {
            return Ok(Err(RunRefusal::NoFolders));
        };
        let numbered = || {
            run.ops
                .iter()
                .enumerate()
                .map(|(index, op)| (index + 1, op))
        };
        for (number, op) in numbered() {
            if !op.paths().into_iter().all(plain) {
                return Ok(Err(RunRefusal::UnsoundPath { op: number }));
            }
        }
        let mut folders = Vec::with_capacity(run.ops.len());
        for (number, op) in numbered() {
            match allowed_folders(op, rules, &run.layout) {
                Some(allowed) => folders.push(allowed),
                None => return Ok(Err(RunRefusal::OutsideFolders { op: number })),
            }
        }
        let Some(staged) = self.staged_files(run_dir)? else {
            return Ok(Err(RunRefusal::StagedNotAFile));
        };
        for (number, op) in numbered() {
            if !staged.contains(op.target()) {
                return Ok(Err(RunRefusal::NotStaged { op: number }));
            }
        }
        let targets: BTreeSet<&str> = run.ops.iter().map(Op::target).collect();
        if staged.iter().any(|path| !targets.contains(path.as_str())) {
            return Ok(Err(RunRefusal::StagedWithoutOp));
        }
        for ((number, op), (target_folder, source_folder)) in numbered().zip(folders) {
            if let Err(refusal) = self.check_op(
                number,
                op,
                &target_folder,
                source_folder.as_deref(),
                run_dir,
            )? {
                return Ok(Err(refusal));
            }
        }
        Ok(Ok(()))
    }

    /// One operation against the vault as it stands: where it lands, what it would overwrite, the
    /// bytes its hash names, and the rails.
    fn check_op(
        &self,
        number: usize,
        op: &Op,
        target_folder: &str,
        source_folder: Option<&str>,
        run_dir: &Path,
    ) -> Result<Result<(), RunRefusal>, VaultError> {
        match self.placement(op.target(), target_folder)? {
            Placement::Inside => {}
            Placement::Outside => return Ok(Err(RunRefusal::OutsideFolders { op: number })),
            Placement::TopLevelMissing => {
                return Ok(Err(RunRefusal::TopLevelFolderMissing { op: number }));
            }
        }
        let target = self.vault.join(op.target());
        match op {
            Op::Create { .. } => {
                if self.taken(&target)? {
                    return Ok(Err(RunRefusal::WouldOverwrite { op: number }));
                }
            }
            Op::Update { sha256_before, .. } => {
                if self.hash_of(&target)?.as_deref() != Some(sha256_before.as_str()) {
                    return Ok(Err(RunRefusal::Changed { op: number }));
                }
            }
            Op::Move {
                from, to, sha256, ..
            } => {
                let from_folder = source_folder.unwrap_or_default();
                if !matches!(self.placement(from, from_folder)?, Placement::Inside) {
                    return Ok(Err(RunRefusal::OutsideFolders { op: number }));
                }
                let staged = self.staged_hash(run_dir, to)?;
                if self.hash_of(&self.vault.join(from))?.as_deref() != Some(sha256.as_str())
                    || staged.as_deref() != Some(sha256.as_str())
                {
                    return Ok(Err(RunRefusal::Changed { op: number }));
                }
                if self.taken(&target)? {
                    return Ok(Err(RunRefusal::WouldOverwrite { op: number }));
                }
                return Ok(Ok(()));
            }
        }
        let bytes = self
            .fs
            .read(&run_dir.join(op.target()))
            .map_err(VaultError::io("read a staged note"))?;
        let Ok(text) = String::from_utf8(bytes) else {
            return Ok(Err(RunRefusal::NotText { op: number }));
        };
        if let Verdict::Refuse(refusal) = self.rails.check(&text) {
            return Ok(Err(RunRefusal::Rails {
                op: number,
                refusal,
            }));
        }
        Ok(Ok(()))
    }

    /// Where `relative` lands: its top-level folder must exist, and the folders that exist on its
    /// way, resolved through their links, and the plain names after them must stay under `folder`.
    fn placement(&self, relative: &str, folder: &str) -> Result<Placement, VaultError> {
        let segments: Vec<&str> = relative.split('/').collect();
        let Some((name, folders)) = segments.split_last() else {
            return Ok(Placement::Outside);
        };
        if let Some(top) = folders.first()
            && self.kind(&self.vault.join(top))?.is_none()
        {
            return Ok(Placement::TopLevelMissing);
        }
        let mut existing = self.vault.clone();
        let mut present = 0;
        for part in folders {
            let next = existing.join(part);
            if self.kind(&next)?.is_none() {
                break;
            }
            existing = next;
            present += 1;
        }
        let resolved = self
            .fs
            .canonicalize(&existing)
            .map_err(VaultError::io("resolve a vault folder"))?;
        let Ok(inside) = resolved.strip_prefix(&self.vault) else {
            return Ok(Placement::Outside);
        };
        let mut landed: Vec<String> = inside
            .iter()
            .map(|part| part.to_string_lossy().into_owned())
            .collect();
        landed.extend(folders[present..].iter().map(|part| (*part).to_owned()));
        landed.push((*name).to_owned());
        Ok(if under(&landed.join("/"), folder) {
            Placement::Inside
        } else {
            Placement::Outside
        })
    }

    /// Whether a name in `target`'s folder is `target`'s name in any case.
    fn taken(&self, target: &Path) -> Result<bool, VaultError> {
        let (Some(folder), Some(name)) = (target.parent(), target.file_name()) else {
            return Ok(true);
        };
        if self.kind(folder)?.is_none() {
            return Ok(false);
        }
        let wanted = name.to_string_lossy().to_lowercase();
        Ok(self
            .fs
            .list(folder)
            .map_err(VaultError::io("list a vault folder"))?
            .iter()
            .any(|entry| entry.name.to_string_lossy().to_lowercase() == wanted))
    }

    /// The SHA-256 of the regular file at `path`, as lowercase hexadecimal, or `None` when no
    /// regular file is there.
    fn hash_of(&self, path: &Path) -> Result<Option<String>, VaultError> {
        if self.kind(path)? != Some(EntryKind::File) {
            return Ok(None);
        }
        let bytes = self
            .fs
            .read(path)
            .map_err(VaultError::io("read a vault note"))?;
        Ok(Some(sha256::hex(&sha256::digest(&bytes))))
    }

    /// The SHA-256 of the staged file at `relative` in `run_dir`.
    fn staged_hash(&self, run_dir: &Path, relative: &str) -> Result<Option<String>, VaultError> {
        self.hash_of(&run_dir.join(relative))
    }

    /// Every staged file of `run_dir` but the record, by its vault path; `None` when an entry is a
    /// link or not a regular file.
    fn staged_files(&self, run_dir: &Path) -> Result<Option<BTreeSet<String>>, VaultError> {
        let mut files = BTreeSet::new();
        let mut pending = vec![(run_dir.to_path_buf(), String::new())];
        while let Some((folder, prefix)) = pending.pop() {
            for entry in self
                .fs
                .list(&folder)
                .map_err(VaultError::io("list the staging directory"))?
            {
                let name = entry.name.to_string_lossy().into_owned();
                let relative = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}/{name}")
                };
                match entry.kind {
                    EntryKind::Dir => pending.push((folder.join(&name), relative)),
                    EntryKind::File if relative == RUN_RECORD => {}
                    EntryKind::File => {
                        files.insert(relative);
                    }
                    EntryKind::Symlink | EntryKind::Other => return Ok(None),
                }
            }
        }
        Ok(Some(files))
    }

    /// The executor's file system, guarded against the journal folders of `run`'s layout: every
    /// folder, note and capture the run applies goes through it (SPEC-118 R5).
    fn guard(&self, run: &DutyRun) -> JournalGuard<Borrowed<'_, F>> {
        JournalGuard::new(
            Borrowed(&self.fs),
            run.layout
                .journal
                .iter()
                .map(|folder| self.vault.join(folder))
                .collect(),
        )
    }

    /// Applies one checked operation of `run`. Each path it writes, a capture's source included, is
    /// refused as [`VaultError::JournalRefused`] when it resolves into a journal folder at the
    /// moment it is written: the checks ran before the gate, and a link the vault gained since
    /// would otherwise carry the write into the journal (ADR-316).
    fn apply_op(&self, run: &DutyRun, op: &Op, run_dir: &Path) -> Result<(), VaultError> {
        let guard = self.guard(run);
        let target = self.vault.join(op.target());
        if let Op::Move { from, .. } = op {
            atomic::refuse_journal_resolved(&guard, &self.vault.join(from))?;
        }
        self.create_folders(&guard, op.target())?;
        match op {
            Op::Create { .. } | Op::Update { .. } => {
                let bytes = self
                    .fs
                    .read(&run_dir.join(op.target()))
                    .map_err(VaultError::io("read a staged note"))?;
                atomic::write(&guard, &target, &bytes)
            }
            Op::Move { from, .. } => {
                let source = self.vault.join(from);
                atomic::refuse_journal_resolved(&guard, &target)?;
                guard
                    .rename(&source, &target)
                    .map_err(VaultError::io("file a capture"))?;
                for folder in [target.parent(), source.parent()].into_iter().flatten() {
                    self.fs
                        .sync_dir(folder)
                        .map_err(VaultError::io("sync a vault folder"))?;
                }
                Ok(())
            }
        }
    }

    /// Creates the folders `relative` needs below its top-level folder, which exists, through
    /// `guard`: a folder that resolves into a journal folder is refused before it is created.
    fn create_folders(
        &self,
        guard: &JournalGuard<Borrowed<'_, F>>,
        relative: &str,
    ) -> Result<(), VaultError> {
        let segments: Vec<&str> = relative.split('/').collect();
        let Some((_, folders)) = segments.split_last() else {
            return Ok(());
        };
        let mut folder = self.vault.clone();
        for part in folders {
            folder.push(part);
            if self.kind(&folder)?.is_none() {
                atomic::refuse_journal_resolved(guard, &folder)?;
                guard
                    .create_dir(&folder)
                    .map_err(VaultError::io("create a vault folder"))?;
            }
        }
        Ok(())
    }

    /// What is at `path`, without following a link.
    fn kind(&self, path: &Path) -> Result<Option<EntryKind>, VaultError> {
        self.fs
            .kind(path)
            .map_err(VaultError::io("read a vault entry"))
    }
}

/// The executor's file system, borrowed for one operation's journal guard: [`JournalGuard`] owns
/// the file system it wraps, and the executor keeps its own. Every call passes straight through.
struct Borrowed<'f, F>(&'f F);

impl<F: VaultFs> VaultFs for Borrowed<'_, F> {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn VaultFile>> {
        self.0.create_new(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.0.rename(from, to)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.0.sync_dir(dir)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.0.remove_file(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.0.read(path)
    }

    fn kind(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.0.kind(path)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<DirEntry>> {
        self.0.list(dir)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.0.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.0.remove_dir(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.0.canonicalize(path)
    }

    fn journal(&self) -> &[PathBuf] {
        self.0.journal()
    }
}

/// Whether `path` is a plain vault-relative path: not absolute, no drive letter, no backslash, and
/// no empty, `.`, `..` or hidden segment.
fn plain(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    !drive
        && !path.contains('\\')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && !segment.starts_with('.'))
}

/// Whether `path` is `folder` or inside it, compared case-insensitively; every path is inside the
/// vault root, the empty folder.
fn under(path: &str, folder: &str) -> bool {
    if folder.is_empty() {
        return true;
    }
    let path = path.to_lowercase();
    let folder = folder.to_lowercase();
    path == folder || path.starts_with(&format!("{folder}/"))
}

/// Whether `path` is strictly inside `folder`.
fn inside(path: &str, folder: &str) -> bool {
    under(path, folder) && path.to_lowercase() != folder.to_lowercase()
}

/// The folder an operation may write its target in, and a move's folder its source may come from,
/// from the duty's rules; `None` when the operation leaves them, or touches the journal or files
/// back into the inbox.
fn allowed_folders(
    op: &Op,
    rules: &DutyFolders,
    layout: &Layout,
) -> Option<(String, Option<String>)> {
    if op
        .paths()
        .iter()
        .any(|path| layout.journal.iter().any(|folder| under(path, folder)))
    {
        return None;
    }
    match op {
        Op::Create { path } | Op::Update { path, .. } => {
            path.strip_suffix(".md")?;
            rules.writes.iter().find_map(|entry| {
                let folder = match entry.as_str() {
                    "@daily" => layout.daily_folder.as_str(),
                    "@weekly" => layout.weekly_folder.as_str(),
                    folder => folder,
                };
                inside(path, folder).then(|| (folder.to_owned(), None))
            })
        }
        Op::Move { from, to, .. } => {
            if !layout.inbox.is_empty() && under(to, &layout.inbox) {
                return None;
            }
            let source = rules
                .moves_from
                .iter()
                .find(|folder| inside(from, folder))?;
            let target = rules.moves_to.iter().find(|folder| inside(to, folder))?;
            Some((target.clone(), Some(source.clone())))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use std::path::PathBuf;

    use super::{GateClass, GateError, ProbeGate, VENDORED_CHECKS, blocking_classes};

    /// SPEC-056 A8: the owned classes keep every field the gate's parser reads.
    #[test]
    fn the_gate_class_parser_refuses_the_owned_classes_without_a_field_it_reads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("gate-classes.json");
        let text = std::fs::read_to_string(&path).expect("the owned gate classes can be read");
        assert_eq!(
            VENDORED_CHECKS, text,
            "the gate compiles in the owned classes"
        );
        let classes = blocking_classes(&text).expect("the owned classes read as classes");
        assert!(
            !classes.is_empty(),
            "the owned classes name a blocking class"
        );
        let document: Value = serde_json::from_str(&text).expect("the owned classes are JSON");
        let first_blocking = |document: &mut Value| -> Option<usize> {
            document
                .get("checks")
                .and_then(Value::as_array)?
                .iter()
                .position(|row| row.get("severity").and_then(Value::as_str) == Some("block"))
        };
        // Without its rows, the file names no class.
        let mut rowless = document.clone();
        rowless
            .as_object_mut()
            .expect("the owned classes are an object")
            .remove("checks");
        assert!(blocking_classes(&rowless.to_string()).is_err());
        // Without a blocking row's id, its class cannot be named.
        let mut nameless = document.clone();
        let at = first_blocking(&mut nameless).expect("a blocking row");
        nameless["checks"][at]
            .as_object_mut()
            .expect("a row is an object")
            .remove("id");
        assert!(blocking_classes(&nameless.to_string()).is_err());
        // Without the severities, no row is blocking.
        let mut unranked = document.clone();
        for row in unranked["checks"]
            .as_array_mut()
            .expect("the rows are a list")
        {
            row.as_object_mut()
                .expect("a row is an object")
                .remove("severity");
        }
        assert!(blocking_classes(&unranked.to_string()).is_err());
        // A row's time limit is read: a changed limit changes its class's.
        let mut slower = document.clone();
        let at = first_blocking(&mut slower).expect("a blocking row");
        let limit = slower["checks"][at]["probe"]["timeout_seconds"]
            .as_u64()
            .expect("a blocking row names its time limit");
        slower["checks"][at]["probe"]["timeout_seconds"] = Value::from(limit + 1);
        assert_ne!(
            blocking_classes(&slower.to_string()).expect("the slower classes read"),
            classes,
            "the parser reads each row's time limit"
        );
    }

    /// A class that outlives its time limit is stopped at the limit its row names. The limit is
    /// counted in polls, so no clock is read: a class of one second whose process would run far
    /// longer ends as a timeout, and a limit counted wrong lets the process run to its end.
    #[test]
    fn a_class_that_outlives_its_time_limit_is_stopped_as_a_timeout() {
        let dir = tempfile::tempdir().expect("a staging directory");
        let probe = dir.path().join("slow-probe.sh");
        std::fs::write(&probe, "exec sleep 20\n").expect("a probe that outlives its limit");
        let gate = ProbeGate {
            python: PathBuf::from("sh"),
            probe,
            vault: None,
            deny_list: None,
            classes: vec![GateClass {
                id: "slow".to_owned(),
                timeout_seconds: 1,
            }],
        };

        let ended = gate.run_class(&gate.classes[0], dir.path());

        assert!(
            matches!(ended, Err(GateError::Timeout { ref class }) if class == "slow"),
            "the one-second class ended as {ended:?}"
        );
    }
}
