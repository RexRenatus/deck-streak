//! Only progression names `xp_settlement`, and only coordination settles (SPEC-072 A12; R9;
//! ADR-072): no other crate's code names the table, no migration but progression's names it, no
//! crate but coordination calls the `settle` operation, and inside coordination a caller outside
//! `crates/coordination/src/recompute/` passes the owner's-correction cause and never the
//! recompute's.
//!
//! The table census reads every crate's `src` and every migration. The settle census reads no Rust:
//! the compiler finds the callers (ADR-197, round 6). Progression's build script gives progression
//! alone the `settle_census` cfg when the census compiles, and under it `settle` carries a
//! deprecation. The census has cargo check every target of every workspace package, with the
//! deprecation forced to warn in every crate, in four passes (debug assertions on and off, each
//! with the unwind and the abort panic strategy), and every use of `settle` rustc reports is a
//! caller, in the package cargo names: progression's own is accepted, coordination's goes to the
//! cause rule, and any other package's is refused. What the compiler is not asked about is refused
//! by construction: a workspace that does not build, a member's feature, a cargo configuration, a
//! package in the repository outside the workspace, and a package from outside it that depends on
//! progression.
//!
//! The census's verdict depends only on the tree it judges (SPEC-072 §12, round 8). Progression is
//! found by its manifest's path, never by its package's name, and a graph where that cannot be told
//! is refused by name. Each census compiles in an empty target of its own, and its cargo inherits
//! only the variables it names; a cargo configuration in the tree, above it or in cargo's home is
//! refused, and so is code of the tree that reads a file outside it or a variable the host sets.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use sha2::{Digest, Sha256};

// The generated population of the table's spellings (SPEC-324, ADR-029's include by path).
#[macro_use]
#[path = "../../../tools/table-census/population.rs"]
mod population;
// The shared reader of every crate's literals (SPEC-324 R1 to R5), included by path as above.
#[path = "../../../tools/table-census/table_census.rs"]
mod table_census;

use population::Spelling;

/// The table the census guards.
const TABLE: &str = "xp_settlement";
/// The context that owns it (docs/CONTEXT-MAP.md).
const OWNER: &str = "progression";
/// The one context that calls `settle`.
const CALLER: &str = "coordination";
/// Where a recompute step lives inside coordination.
const RECOMPUTE_DIR: &str = "crates/coordination/src/recompute/";
/// The cause a caller outside the recompute steps passes.
const CORRECTION_CAUSE: &str = "SettleCause::OwnersCorrection";
/// The cause only a recompute step passes.
const RECOMPUTE_CAUSE: &str = "SettleCause::Recompute";
/// The variable that arms progression's probe (`crates/progression/build.rs`).
const ARMING: &str = "SETTLE_CENSUS";
/// The note of the deprecation `settle` carries under the census's cfg
/// (`crates/progression/src/settle.rs`): rustc reports every use of the operation with it.
const PROBE_NOTE: &str = "the settle census's probe";
/// The flags every crate of the build is compiled with: the deprecation forced to warn, which no
/// `allow`, `deny`, `forbid` or `expect` in a crate and no `--cap-lints` can silence.
const RUSTFLAGS: [&str; 2] = ["--force-warn", "deprecated"];
/// Coordination's targets that are not its product: a test, a bench or an example may call
/// `settle` with any cause.
const CALLER_AIDS: [&str; 3] = ["test", "bench", "example"];
/// The files of progression's own code that progression admits may call `settle` beside its own
/// re-exports (#445): none, so every wrapper of the operation in progression is refused by name.
const OWNER_ADMITS: [&str; 0] = [];
/// The longest one cargo run of the census may take before the census fails by name.
const CARGO_LIMIT: Duration = Duration::from_mins(30);
/// The longest chain of macro calls the census follows from one use of `settle`: past it the
/// census fails by name rather than walk on.
const EXPANSION_LIMIT: usize = 1024;
/// Progression's package, as cargo names it.
const PROGRESSION_PACKAGE: &str = "deck-streak-progression";
/// The variables that would compile the census's build apart from cargo's defaults: another
/// compiler, a wrapper that could drop the census's flags, or flags of their own.
const SCRUBBED: [&str; 5] = [
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
];
/// The families of cargo's configuration variables that could do the same: the build's settings
/// (its target, flags and wrappers), the profiles, a target's settings, and unstable options.
const SCRUBBED_PREFIXES: [&str; 4] = [
    "CARGO_BUILD_",
    "CARGO_PROFILE_",
    "CARGO_TARGET_",
    "CARGO_UNSTABLE_",
];
/// The only variables of the census's own environment its cargo inherits: where the programs, the
/// home, cargo's home and rustup's home are, and the toolchain rustup chose to run this test, which
/// the repository pins. Every other variable is dropped, so that no variable the tree does not set
/// reaches a build script, a macro or rustc in the census's build (SPEC-072 §12, round 8).
const INHERITED: [&str; 5] = [
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
];
/// The owner's manifest, by which the census finds the owner: never by its package's name, which
/// any other package of the graph can carry (main's round-7 ruling, condition 1).
const OWNER_MANIFEST: &str = "crates/progression/Cargo.toml";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `directory` whose name ends in `extension`, recursively, by path.
fn files(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(files(&path, extension));
        } else if path
            .extension()
            .is_some_and(|name| name.to_str() == Some(extension))
        {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Rust `text` as the compiler reads it, by one lexer: every comment (a `//` or doc line, or a
/// `/* */` block, which nests) is the space it stands for, and each literal (a string, a raw, byte
/// or C string, or a character) is kept whole when `literals` is true and emptied when it is
/// false. The table census reads the table from code and its literals, where SQL lives, and the
/// cause rule reads a cause from code alone. A lifetime (`'a`) is code.
fn strip(text: &str, literals: bool) -> String {
    let characters: Vec<char> = text.chars().collect();
    let at_char = |index: usize| characters.get(index).copied();
    let mut code = String::new();
    let mut at = 0;
    while let Some(character) = at_char(at) {
        let next = at_char(at + 1);
        if character == '/' && next == Some('/') {
            while at_char(at).is_some_and(|line| line != '\n') {
                at += 1;
            }
            code.push(' ');
            continue;
        }
        if character == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while let Some(opened) = at_char(at) {
                if opened == '/' && at_char(at + 1) == Some('*') {
                    depth += 1;
                    at += 2;
                } else if opened == '*' && at_char(at + 1) == Some('/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    at += 1;
                }
            }
            code.push(' ');
            continue;
        }
        let end = literal_end(&characters, at);
        if let Some(end) = end {
            if literals {
                code.extend(&characters[at..end]);
            } else {
                code.push_str("\"\"");
            }
            at = end;
            continue;
        }
        code.push(character);
        at += 1;
    }
    code
}

/// Where the literal that opens at `at` ends, or `None` when no literal opens there. A literal
/// opens at a quote, or at `r`, `b`, `c`, `br` or `cr` that starts a word and stands before one
/// (or, raw, before `#`s and one); a `'` opens a character only when it closes one character
/// later or escapes, and is otherwise a lifetime's or a label's.
fn literal_end(characters: &[char], at: usize) -> Option<usize> {
    let at_char = |index: usize| characters.get(index).copied();
    let word_start = at == 0 || !at_char(at - 1).is_some_and(|c| c.is_alphanumeric() || c == '_');
    let prefix = ["br", "cr", "r", "b", "c", ""]
        .into_iter()
        .find(|prefix| {
            word_start
                && prefix.chars().zip(&characters[at..]).all(|(a, b)| a == *b)
                && characters.len() > at + prefix.len()
        })
        .unwrap_or_default();
    let open = at + prefix.len();
    let raw = prefix.ends_with('r');
    let hashes = if raw {
        characters[open..].iter().take_while(|c| **c == '#').count()
    } else {
        0
    };
    match at_char(open + hashes)? {
        '"' => {
            let mut end = open + hashes + 1;
            loop {
                match at_char(end) {
                    Some('\\') if !raw => end += 2,
                    Some('"')
                        if characters[end + 1..]
                            .iter()
                            .take(hashes)
                            .filter(|c| **c == '#')
                            .count()
                            == hashes =>
                    {
                        return Some(end + 1 + hashes);
                    }
                    Some(_) => end += 1,
                    None => return Some(characters.len()),
                }
            }
        }
        '\'' if !raw && matches!(prefix, "" | "b") => {
            match (at_char(open + 1), at_char(open + 2)) {
                (Some('\\'), _) => {
                    let mut end = open + 3;
                    while at_char(end).is_some_and(|c| c != '\'') {
                        end += 1;
                    }
                    Some((end + 1).min(characters.len()))
                }
                (Some(_), Some('\'')) => Some(open + 3),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether Rust source text names the table in its code or its literals, outside every comment.
fn names_the_table(text: &str) -> bool {
    text.contains(TABLE) && strip(text, true).contains(TABLE)
}

/// Whether SQL text names the table outside a `--` comment.
fn sql_names_the_table(text: &str) -> bool {
    text.lines()
        .any(|line| line.split("--").next().unwrap_or_default().contains(TABLE))
}

/// The workspace's members, by path.
fn members(root: &Path) -> Vec<PathBuf> {
    let mut members: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    members
}

/// Whether the variable `name` would compile the census's build apart from cargo's defaults, so
/// that the census removes it from cargo's environment.
fn scrubbed(name: &str) -> bool {
    SCRUBBED.contains(&name)
        || SCRUBBED_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

/// Whether cargo, run by the census, inherits the variable `name` from the census's environment:
/// only a name of [`INHERITED`], and never one that would compile the build apart from cargo's
/// defaults.
fn inherited(name: &str) -> bool {
    INHERITED.contains(&name) && !scrubbed(name)
}

/// Runs the cargo that runs this test with `arguments` in `root`, with the census's flags and its
/// arming, and answers whether it succeeded, its standard output and its standard error. Cargo
/// starts from an empty environment and inherits only the names [`inherited`] admits, so the build
/// is cargo's own with the census's flags, and a variable the tree does not set reaches it only
/// through a name of [`INHERITED`]. The run is bounded: past `limit` (`CARGO_LIMIT`, which the
/// census passes) the child is stopped and the census fails by name.
fn cargo(
    root: &Path,
    arguments: &[String],
    limit: Duration,
) -> Result<(bool, String, String), String> {
    let program = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(program);
    command.env_clear();
    for (name, value) in std::env::vars_os() {
        if inherited(&name.to_string_lossy()) {
            command.env(&name, &value);
        }
    }
    let mut child = command
        .args(arguments)
        .current_dir(root)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_ENCODED_RUSTFLAGS", RUSTFLAGS.join("\u{1f}"))
        .env(ARMING, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cargo cannot start ({error}), so the census sees no caller"))?;
    let mut stdout = child.stdout.take().expect("a piped standard output");
    let mut stderr = child.stderr.take().expect("a piped standard error");
    let out = std::thread::spawn(move || {
        let mut text = String::new();
        stdout.read_to_string(&mut text).map(|_| text)
    });
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        stderr.read_to_string(&mut text).map(|_| text)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if started.elapsed() > limit {
            child.kill().map_err(|error| error.to_string())?;
            child.wait().map_err(|error| error.to_string())?;
            return Err(format!(
                "cargo {} ran past {} s, so the census cannot see its callers",
                arguments.first().map_or("", String::as_str),
                limit.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let out = out
        .join()
        .expect("the reader of cargo's output")
        .map_err(|error| error.to_string())?;
    let err = err
        .join()
        .expect("the reader of cargo's errors")
        .map_err(|error| error.to_string())?;
    Ok((status.success(), out, err))
}

/// The first line of cargo's standard error that names an error, for a refusal to quote.
fn first_error(stderr: &str) -> &str {
    stderr
        .lines()
        .find(|line| line.starts_with("error"))
        .unwrap_or("no error line")
}

/// `path` under `workspace` with `/` separators, or the path whole when it lies outside.
fn under(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace).map_or_else(
        |_| path.to_string_lossy().into_owned(),
        |inner| {
            inner
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        },
    )
}

/// Every `Cargo.toml` under `directory`, the census's own build output and version control aside.
/// The walk never follows a link, so it ends on every tree.
fn manifests(root: &Path, directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() && path != root.join("target") && path != root.join(".git") {
            manifests(root, &path, found);
        } else if kind.is_file() && entry.file_name() == "Cargo.toml" {
            found.push(path);
        }
    }
}

/// The files of the repository a use of `settle` is written in, innermost first: each primary
/// span's own file, then each macro call site and each `include!` it was expanded from, as long as
/// the compiler wrote the chain, to `EXPANSION_LIMIT` calls. Each file is named by its real path,
/// every `..` and link resolved, so a `#[path]` through `recompute/..` names the file it reaches. A
/// file the census's own build wrote (a build script's output) is not the repository's, so its call
/// site stands for it.
fn written_in(message: &Value, workspace: &Path, target: &Path) -> Result<Vec<String>, String> {
    let real = |path: &Path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let (workspace, target) = (real(workspace), real(target));
    let mut chain = Vec::new();
    for span in message["message"]["spans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|span| span["is_primary"] == true)
    {
        let mut at = Some(span);
        for _ in 0..EXPANSION_LIMIT {
            let Some(span) = at else {
                break;
            };
            if let Some(file) = span["file_name"].as_str() {
                let path = real(&workspace.join(file));
                if path.starts_with(&workspace) && !path.starts_with(&target) {
                    chain.push(under(&workspace, &path));
                }
            }
            at = span["expansion"]
                .get("span")
                .filter(|next| next.is_object());
        }
        if at.is_some() {
            return Err(format!(
                "a use of settle expands through more than {EXPANSION_LIMIT} macro calls, so the \
                 census cannot name its file"
            ));
        }
    }
    Ok(chain)
}

/// Where a use of `settle` is written: the innermost file of the repository its primary span or a
/// macro call site it was expanded from lies in, by its path under the workspace, and the byte the
/// span starts at there (rustc counts bytes), or none when no file of the repository holds it.
fn innermost(message: &Value, workspace: &Path, target: &Path) -> Option<(String, usize)> {
    let (workspace, target) = (canonical(workspace), canonical(target));
    let primary = message["message"]["spans"]
        .as_array()?
        .iter()
        .find(|span| span["is_primary"] == true)?;
    std::iter::successors(Some(primary), |span| {
        Some(&span["expansion"]["span"]).filter(|next| next.is_object())
    })
    .take(EXPANSION_LIMIT)
    .find_map(|span| {
        let path = canonical(&workspace.join(span["file_name"].as_str()?));
        let byte = usize::try_from(span["byte_start"].as_u64()?).ok()?;
        (path.starts_with(&workspace) && !path.starts_with(&target))
            .then(|| (under(&workspace, &path), byte))
    })
}

/// One use of `settle` rustc reported: the folder of the package cargo compiled it in, the kind of
/// the target, the repository's files it is written in (innermost first), and the file it is
/// named by: the outermost of those, or the target's root when the code is not the repository's.
/// `at` is where it is written: the innermost of those files and the byte its span starts at.
struct Use {
    folder: String,
    kind: String,
    chain: Vec<String>,
    file: String,
    at: Option<(String, usize)>,
}

/// The `--config` arguments of one pass: every package the workspace compiles, each named, with
/// debug assertions `on`, and the `abort` or the unwind panic strategy, the only two things a
/// profile sets that stable rustc offers as a cfg. A package's named setting stands above every
/// other profile setting of it (the manifest's `package."*"`, `build-override` and base settings),
/// and a `--config` setting above the manifest's own at one level, so progression, every
/// dependency, every build script and every macro compile in each pass as the members do: a macro
/// expands as the cfg of the crate that defines it wrote it. Each is a `dev` setting, which the
/// `test` profile of a test, a bench and a library's own tests inherits and cannot override. A
/// manifest that names a package by another spec (`name@version`) makes two settings of it, which
/// cargo refuses to compile. A test and a bench ignore the panic strategy.
fn pass(on: bool, abort: bool, packages: &BTreeSet<String>) -> Vec<String> {
    let mut arguments = vec![
        "--config".to_owned(),
        format!(
            "profile.dev.panic=\"{}\"",
            if abort { "abort" } else { "unwind" }
        ),
    ];
    for package in packages {
        arguments.push("--config".to_owned());
        arguments.push(format!(
            "profile.dev.package.\"{package}\".debug-assertions={on}"
        ));
    }
    arguments
}

/// The SHA-256 of `crates/progression/build.rs`, the one build script of a package that can name
/// `settle` that the census admits (ADR-197, round 7). Editing that script, or adding another that
/// can reach `settle`, is refused by name until this constant is changed in review.
const PROGRESSION_BUILD_SHA256: &str =
    "a55c986660b4a75d5f8be32d91e3ae82b8140245eb7e40cba6d5306eb9c3815a";

/// `path` with every `..` and link resolved, or `path` itself when it names nothing, so that two
/// spellings of one file compare equal.
fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The build-script target of `package`, if it has one: cargo gives a package one build script at
/// most, and a target list holding more is refused by the caller.
fn scripts_of(package: &Value) -> Vec<&Value> {
    package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|target| {
            target["kind"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| kind == "custom-build")
        })
        .collect()
}

/// The SHA-256 of the file at `path`, in lowercase hex, or why it cannot be read.
fn sha256_of(path: &Path) -> std::io::Result<String> {
    fs::read(path).map(|bytes| {
        Sha256::digest(&bytes)
            .iter()
            .fold(String::new(), |mut hex, byte| {
                write!(hex, "{byte:02x}").expect("a string takes every write");
                hex
            })
    })
}

/// Refuses, by name, every package that has a build script and can name `settle`: the package that
/// defines it, or one that depends on it by a normal, a build or a dev edge, at any depth. A build
/// script's cfg reaches only its own package, and the census's passes never set a cfg from the
/// build environment, so such a package could hold code the passes never compile. The owner is the
/// one workspace member whose manifest is [`OWNER_MANIFEST`], found by that path and never by a
/// name (main's round-7 ruling, condition 1): the census refuses, by name, a graph where that
/// member is absent or ambiguous, where its package is not [`PROGRESSION_PACKAGE`], where any other
/// package carries its name, whatever its version or source, and where it has no build script or
/// more than one. Its own script is admitted at [`PROGRESSION_BUILD_SHA256`] alone. A package from
/// outside the workspace that reaches the owner through the graph is refused, whatever it names.
/// A graph cargo cannot give is a refusal, never a skipped check, and no graph without its owner
/// is ever accepted.
#[allow(clippy::too_many_lines)]
fn build_scripts(root: &Path, metadata: &Value) -> Result<Vec<String>, Vec<String>> {
    let unreadable = |what: &str| vec![format!("cargo metadata cannot give the graph ({what})")];
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or_else(|| unreadable("it holds no resolve graph"))?;
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in nodes {
        let id = node["id"]
            .as_str()
            .ok_or_else(|| unreadable("a node holds no id"))?;
        for dependency in node["deps"].as_array().into_iter().flatten() {
            let target = dependency["pkg"]
                .as_str()
                .ok_or_else(|| unreadable("a dependency holds no package"))?;
            dependents.entry(target).or_default().push(id);
        }
    }
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| unreadable("it holds no packages"))?;
    let workspace = metadata["workspace_root"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| unreadable("it names no workspace root"))?;
    let member_ids: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .ok_or_else(|| unreadable("it names no workspace members"))?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let manifest = canonical(&root.join(OWNER_MANIFEST));
    let owners: Vec<&Value> = packages
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| member_ids.contains(id))
                && package["manifest_path"]
                    .as_str()
                    .is_some_and(|path| canonical(Path::new(path)) == manifest)
        })
        .collect();
    let [owner_package] = owners.as_slice() else {
        return Err(vec![if owners.is_empty() {
            format!(
                "{OWNER_MANIFEST} is the manifest of no workspace member, so the census has no \
                 owner and cannot tell its callers"
            )
        } else {
            format!(
                "{} workspace members have the manifest {OWNER_MANIFEST}, so the census cannot \
                 tell its owner",
                owners.len()
            )
        }]);
    };
    let owner = owner_package["id"].as_str();
    let owner_name = owner_package["name"].as_str().unwrap_or_default();
    let mut refused = Vec::new();
    if owner_name != PROGRESSION_PACKAGE {
        refused.push(format!(
            "{OWNER_MANIFEST} names its package {owner_name}, not {PROGRESSION_PACKAGE}, so the \
             census cannot tell its owner by the name the graph's other packages use"
        ));
    }
    let decoys: Vec<String> = packages
        .iter()
        .filter(|package| {
            package["id"] != owner_package["id"]
                && (package["name"] == PROGRESSION_PACKAGE || package["name"] == owner_name)
        })
        .map(|package| {
            format!(
                "{} {} ({})",
                package["name"].as_str().unwrap_or_default(),
                package["version"].as_str().unwrap_or_default(),
                package["source"].as_str().unwrap_or("a path")
            )
        })
        .collect();
    if !decoys.is_empty() {
        refused.push(format!(
            "another package of the graph carries the owner's name ({}), so the census cannot \
             tell the owner's callers by name",
            decoys.join(", ")
        ));
    }
    match scripts_of(owner_package).as_slice() {
        [] => refused.push(format!(
            "{owner_name} has no build script, so the census's cfg is never set and no use of \
             settle is reported"
        )),
        [script] => {
            let path = Path::new(script["src_path"].as_str().unwrap_or_default());
            let digest = sha256_of(path);
            if digest.ok().as_deref() != Some(PROGRESSION_BUILD_SHA256) {
                refused.push(format!(
                    "{owner_name}'s build script ({}) is not the one pinned by \
                     PROGRESSION_BUILD_SHA256",
                    under(&workspace, path)
                ));
            }
        }
        more => refused.push(format!(
            "{owner_name} has {} build scripts, so the census cannot tell which it pins",
            more.len()
        )),
    }
    let mut reaching: BTreeSet<&str> = BTreeSet::new();
    let mut queue: Vec<&str> = owner.into_iter().collect();
    while let Some(id) = queue.pop() {
        if reaching.insert(id) {
            queue.extend(dependents.get(id).into_iter().flatten());
        }
    }
    for package in packages {
        let id = package["id"].as_str().unwrap_or_default();
        let name = package["name"].as_str().unwrap_or_default();
        if Some(id) == owner {
            continue;
        }
        if !package["source"].is_null() && reaching.contains(id) {
            refused.push(format!(
                "{name} comes from {} and reaches the owner through the graph, so the census \
                 cannot name its callers",
                package["source"].as_str().unwrap_or_default()
            ));
        }
        if scripts_of(package).is_empty() {
            continue;
        }
        if !reaching.contains(id) {
            continue;
        }
        refused.push(format!(
            "{name} has a build script and can name settle, and a build script's cfg is one \
             the census's passes never set"
        ));
    }
    Ok(refused)
}

/// Every use of `settle` in the code the workspace at `root` compiles, with the files outside the
/// tree that the workspace's own code read (each a refusal that does not stop the census naming
/// its uses), or the reasons the census cannot see them all. `target` is the census's own target
/// directory, made empty for this census alone and apart from every other build.
#[allow(clippy::too_many_lines)]
fn compiled_uses(root: &Path, target: &Path) -> Result<(Vec<Use>, Vec<String>), Vec<String>> {
    let arguments: Vec<String> = ["metadata", "--format-version", "1", "--locked", "--offline"]
        .map(str::to_owned)
        .to_vec();
    let (ok, stdout, stderr) =
        cargo(root, &arguments, CARGO_LIMIT).map_err(|reason| vec![reason])?;
    if !ok {
        return Err(vec![format!(
            "cargo metadata cannot give the graph ({}), so the census cannot see its callers",
            first_error(&stderr)
        )]);
    }
    let metadata: Value = serde_json::from_str(&stdout)
        .map_err(|error| vec![format!("cargo metadata cannot give the graph ({error})")])?;
    let mut refused = build_scripts(root, &metadata)?;
    let workspace = PathBuf::from(metadata["workspace_root"].as_str().unwrap_or_default());
    let tree = canonical(root);
    if canonical(&workspace) != tree {
        refused.push(format!(
            "cargo reads the workspace at {}, a manifest above the tree, and the census judges \
             the tree alone",
            workspace.display()
        ));
    }
    let member_ids: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut folders: BTreeMap<&str, String> = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut tested = false;
    let mut built = BTreeSet::from(["Cargo.toml".to_owned()]);
    for package in metadata["packages"].as_array().into_iter().flatten() {
        let id = package["id"].as_str().unwrap_or_default();
        let manifest = Path::new(package["manifest_path"].as_str().unwrap_or_default());
        let folder = under(&workspace, manifest.parent().unwrap_or(manifest));
        names.insert(package["name"].as_str().unwrap_or_default().to_owned());
        if member_ids.contains(id) {
            built.insert(under(&workspace, manifest));
            tested |= package["dependencies"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|dependency| dependency["kind"] == "dev");
            if package["features"]
                .as_object()
                .is_some_and(|features| !features.is_empty())
            {
                refused.push(format!(
                    "{folder}/Cargo.toml declares a feature, and the census compiles none"
                ));
            }
            if package["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|target| target["kind"].as_array().into_iter().flatten())
                .any(|kind| kind == "proc-macro")
            {
                refused.push(format!(
                    "{folder} is a proc-macro crate, and rustc reports no deprecation inside a \
                     derive's expansion"
                ));
            }
        } else if package["source"].is_null() {
            refused.push(format!(
                "{folder} is a package outside the workspace, which the census does not compile"
            ));
        } else if package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|dependency| dependency["name"] == PROGRESSION_PACKAGE)
        {
            refused.push(format!(
                "{folder} depends on progression from outside the workspace, whose callers the \
                 census cannot name"
            ));
        }
        folders.insert(id, folder);
    }
    let mut found = Vec::new();
    manifests(root, root, &mut found);
    for manifest in found {
        let name = relative(root, &manifest);
        if !built.contains(&name) {
            refused.push(format!(
                "{name} is a package outside the workspace, which the census does not compile"
            ));
        }
    }
    // Cargo reads its configuration from the folder it runs in and every folder above it, and from
    // its home (https://doc.rust-lang.org/cargo/reference/config.html#hierarchical-structure).
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    for folder in tree.ancestors() {
        for config in [".cargo/config", ".cargo/config.toml"] {
            if folder.join(config).exists() {
                refused.push(format!(
                    "{} configures cargo, and the census compiles with cargo's own defaults",
                    under(&tree, &folder.join(config))
                ));
            }
        }
    }
    for file in ["config", "config.toml"]
        .iter()
        .filter_map(|config| home.as_ref().map(|home| home.join(config)))
    {
        if file.exists() {
            refused.push(format!(
                "{} configures cargo from its home, and the census compiles with cargo's own \
                 defaults",
                file.display()
            ));
        }
    }
    if !refused.is_empty() {
        return Err(refused);
    }
    // Resolver 2 gives a dependency the features a dev-dependency asks for only in a build of a
    // target that needs it, so when a member has a dev-dependency, its libraries and binaries as a
    // build without tests compiles them are a variant of their own, which the census compiles too.
    let mut selections: Vec<&[&str]> = vec![&["--all-targets"]];
    if tested {
        selections.push(&[]);
    }
    let mut uses = Vec::new();
    let mut read = BTreeMap::new();
    for selection in selections {
        for (on, abort) in [(true, false), (false, false), (true, true), (false, true)] {
            let mut arguments: Vec<String> = [
                "check",
                "--workspace",
                "--message-format=json",
                "--target-dir",
            ]
            .map(str::to_owned)
            .to_vec();
            arguments.push(target.to_string_lossy().into_owned());
            arguments.extend(selection.iter().map(|flag| (*flag).to_owned()));
            arguments.push("--locked".to_owned());
            arguments.extend(pass(on, abort, &names));
            let (ok, stdout, stderr) =
                cargo(root, &arguments, CARGO_LIMIT).map_err(|reason| vec![reason])?;
            if !ok {
                return Err(vec![format!(
                    "the workspace does not compile ({}), so the census cannot see its callers",
                    first_error(&stderr)
                )]);
            }
            for line in stdout.lines() {
                let Ok(message) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if message["reason"] == "compiler-artifact"
                    && message["package_id"]
                        .as_str()
                        .is_some_and(|id| member_ids.contains(id))
                    && message["target"]["kind"] != serde_json::json!(["custom-build"])
                {
                    for file in message["filenames"].as_array().into_iter().flatten() {
                        let file = Path::new(file.as_str().unwrap_or_default());
                        let stem = file
                            .file_stem()
                            .map(|stem| stem.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let info = file.with_file_name(format!(
                            "{}.d",
                            stem.strip_prefix("lib").unwrap_or(&stem)
                        ));
                        let folder = message["package_id"]
                            .as_str()
                            .and_then(|id| folders.get(id))
                            .cloned()
                            .unwrap_or_default();
                        read.insert(info, folder);
                    }
                    continue;
                }
                if message["reason"] != "compiler-message"
                    || message["message"]["code"]["code"] != "deprecated"
                    || !message["message"]["message"]
                        .as_str()
                        .is_some_and(|text| text.contains(PROBE_NOTE))
                {
                    continue;
                }
                let chain =
                    written_in(&message, &workspace, target).map_err(|reason| vec![reason])?;
                uses.push(Use {
                    folder: message["package_id"]
                        .as_str()
                        .and_then(|id| folders.get(id))
                        .cloned()
                        .unwrap_or_default(),
                    // A target of one kind is named by it; a target of several is named by none,
                    // so it is never taken for a test, a bench or an example.
                    kind: match message["target"]["kind"].as_array().map(Vec::as_slice) {
                        Some([kind]) => kind.as_str().unwrap_or_default().to_owned(),
                        _ => String::new(),
                    },
                    at: innermost(&message, &workspace, target),
                    file: chain.last().cloned().unwrap_or_else(|| {
                        under(
                            &workspace,
                            Path::new(message["target"]["src_path"].as_str().unwrap_or_default()),
                        )
                    }),
                    chain,
                });
            }
        }
    }
    Ok((
        uses,
        outside_the_tree(&tree, &workspace, &canonical(target), &read),
    ))
}

/// Every file outside the tree, and every variable the host sets, that the workspace's own code
/// read, as rustc's dep-info for each member's compile names them (`read` maps each dep-info file
/// to its member's folder), each a refusal: a file that no path of the tree holds, one in the
/// tree's own build output, or a variable of [`INHERITED`] could move the verdict while the tree
/// stays as it is. The census's own target, made empty for this census, holds only what this
/// census's build wrote.
fn outside_the_tree(
    tree: &Path,
    workspace: &Path,
    target: &Path,
    read: &BTreeMap<PathBuf, String>,
) -> Vec<String> {
    let mut refused = Vec::new();
    for (info, folder) in read {
        let Ok(text) = fs::read_to_string(info) else {
            refused.push(format!(
                "rustc wrote no dep-info for {folder} ({}), so the census cannot tell what its \
                 code read",
                info.display()
            ));
            continue;
        };
        for line in text.lines() {
            if let Some(variable) = line.strip_prefix("# env-dep:") {
                let name = variable.split_once('=').map_or(variable, |(name, _)| name);
                if INHERITED.contains(&name) {
                    refused.push(format!(
                        "{folder}'s code reads the variable {name}, which the host sets, and the \
                         census judges the tree alone"
                    ));
                }
                continue;
            }
            let Some(file) = line.strip_suffix(':') else {
                continue;
            };
            if file.is_empty() || line.starts_with('#') {
                continue;
            }
            let path = workspace.join(file.replace("\\ ", " "));
            let resolved = fs::canonicalize(&path).unwrap_or(path);
            let inside = resolved.starts_with(target)
                || (resolved.starts_with(tree) && !resolved.starts_with(tree.join("target")));
            if !inside {
                refused.push(format!(
                    "{folder}'s code reads {file}, which lies outside the tree, and the census \
                     judges the tree alone"
                ));
            }
        }
    }
    refused
}

/// What the census found: the sources and migrations it read, the files that name the table, the
/// files that call `settle` (progression's own aside), and what it refused.
struct Census {
    sources: Vec<String>,
    migrations: Vec<String>,
    naming: BTreeSet<String>,
    calling: BTreeSet<String>,
    refused: Vec<String>,
}

/// The census of a planted workspace at `root`, whose code is checked in its own target directory.
fn census(root: &Path) -> Census {
    locked(root);
    census_in(root, &root.join("target").join("settle-census"))
}

/// Writes the lock file of a planted workspace that holds none, as the maintainer's own `cargo
/// generate-lockfile` does, so that the census reads the planted tree's resolve graph under
/// `--locked --offline` exactly as it reads the real tree's. A planted git dependency is fetched
/// here, which is what makes it resolvable offline afterwards. A tree that already holds a lock
/// file keeps it, whatever it holds.
fn locked(root: &Path) {
    if root.join("Cargo.lock").exists() {
        return;
    }
    let (ok, _, stderr) = cargo(root, &["generate-lockfile".to_owned()], CARGO_LIMIT)
        .expect("cargo generates a planted workspace's lock file");
    assert!(ok, "generate-lockfile in {}: {stderr}", root.display());
}

/// The file of `used` that the cause rule refuses, if any: a use by a coordination target that is
/// not a test, a bench or an example is accepted only when every file of the repository it is
/// written in lies in a recompute step or passes the owner's correction and never the recompute's.
/// A use written in no file of the repository is refused.
fn outside_the_steps(root: &Path, used: &Use) -> Option<String> {
    if CALLER_AIDS.contains(&used.kind.as_str()) {
        return None;
    }
    if used.chain.is_empty() {
        return Some(used.file.clone());
    }
    used.chain
        .iter()
        .find(|file| {
            if file.starts_with(RECOMPUTE_DIR) {
                return false;
            }
            let code = fs::read_to_string(root.join(file))
                .map(|text| strip(&text, false))
                .unwrap_or_default();
            !code.contains(CORRECTION_CAUSE) || code.contains(RECOMPUTE_CAUSE)
        })
        .cloned()
}

/// The refusal of `used` by the owner rule (#445), if any: a use of `settle` in progression's own
/// package, by a target that is not a test, a bench or an example, in a file `admits` does not
/// name, written outside every `use` declaration of its innermost file. A re-export or an import
/// is the owner's own wiring; a wrapper, a function pointer or a generic is a second door to the
/// operation. A use the census cannot place in a file of the repository is refused.
fn owners_own_operation(root: &Path, used: &Use, admits: &[&str]) -> Option<String> {
    if used.folder != format!("crates/{OWNER}") || CALLER_AIDS.iter().any(|aid| *aid == used.kind) {
        return None;
    }
    if admits.contains(&used.file.as_str()) {
        return None;
    }
    if let Some((file, offset)) = &used.at {
        let text = fs::read_to_string(root.join(file)).unwrap_or_default();
        if table_census::use_declarations(&text)
            .iter()
            .any(|range| range.contains(offset))
        {
            return None;
        }
    }
    Some(format!(
        "{} calls settle inside {OWNER}'s own code, and only {CALLER}'s code may unless {OWNER} \
         admits it",
        used.file
    ))
}

/// The census of the workspace at `root`, whose code is checked in `target`, a target directory
/// apart from every other build: cargo holds the running build's own directory.
fn census_in(root: &Path, target: &Path) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        migrations: Vec::new(),
        naming: BTreeSet::new(),
        calling: BTreeSet::new(),
        refused: Vec::new(),
    };
    for member in members(root) {
        let context = member
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 crate directory")
            .to_owned();
        for path in files(&member.join("src"), "rs") {
            let name = relative(root, &path);
            let text = fs::read_to_string(&path).expect("a readable source");
            if names_the_table(&text) {
                census.naming.insert(name.clone());
                if context != OWNER {
                    census
                        .refused
                        .push(format!("{name} names {TABLE}, and only {OWNER}'s code may"));
                }
            }
            census.sources.push(name);
        }
    }
    census
        .refused
        .extend(table_census::refusals(root, TABLE, OWNER, &census.naming));
    // Each census compiles in a target directory of its own, made empty under `target` and removed
    // when the census ends, so no build output, build-script output or fingerprint that another
    // build wrote can serve this one (SPEC-072 §12, round 8).
    let fresh = fs::create_dir_all(target)
        .and_then(|()| {
            tempfile::Builder::new()
                .prefix("census-")
                .tempdir_in(target)
        })
        .map_err(|error| {
            vec![format!(
                "the census cannot make an empty target under {} ({error}), so it cannot see its \
                 callers",
                target.display()
            )]
        });
    match fresh.and_then(|fresh| compiled_uses(root, fresh.path())) {
        Err(reasons) => census.refused.extend(reasons),
        Ok((uses, read)) => {
            census.refused.extend(read);
            for used in uses {
                census
                    .refused
                    .extend(owners_own_operation(root, &used, &OWNER_ADMITS));
                if used.folder == format!("crates/{OWNER}") {
                    continue;
                }
                census.calling.insert(used.file.clone());
                if used.folder != format!("crates/{CALLER}") {
                    census.refused.push(format!(
                        "{} calls settle, and only {CALLER}'s code may",
                        used.file
                    ));
                } else if let Some(file) = outside_the_steps(root, &used) {
                    census.refused.push(format!(
                        "{file} calls settle outside the recompute steps, and only the owner's \
                         correction may"
                    ));
                }
            }
        }
    }
    for migration in files(&root.join("migrations"), "sql") {
        let name = relative(root, &migration);
        let file = migration
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 migration name");
        // `<SPEC number and sequence>_<owning context>_<slug>.sql` (ADR-020).
        let context = file.split('_').nth(1).unwrap_or_default();
        let text = fs::read_to_string(&migration).expect("a readable migration");
        if sql_names_the_table(&text) {
            census.naming.insert(name.clone());
            if context != OWNER {
                census.refused.push(format!(
                    "{name} names {TABLE}, and only {OWNER}'s migrations may"
                ));
            }
        }
        census.migrations.push(name);
    }
    census.refused.sort();
    census.refused.dedup();
    census
}

/// `path` relative to `root`, with `/` separators.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("a path under the root")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Writes `text` to `root`/`path`, making its folders.
fn plant(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("a planted file's folder")).expect("its folder");
    fs::write(file, text).expect("the planted file");
}
/// A planted member's manifest: its package, and a path dependency on each crate of `on`.
fn manifest_of(name: &str, on: &[&str]) -> String {
    let dependencies = on.iter().fold(String::new(), |mut text, crate_name| {
        writeln!(
            text,
            "deck-streak-{crate_name} = {{ path = \"../{crate_name}\" }}"
        )
        .expect("a string takes every write");
        text
    });
    format!(
        "[package]\nname = \"deck-streak-{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\n{dependencies}"
    )
}

/// Plants a workspace: its manifest, and progression with its real build script and a `settle`
/// that carries the real probe.
fn plant_workspace(root: &Path) {
    plant(root, "Cargo.toml", KILLER_WORKSPACE);
    plant(
        root,
        "crates/progression/Cargo.toml",
        &manifest_of(OWNER, &[]),
    );
    plant(root, "crates/progression/build.rs", KILLER_BUILD);
    plant(root, "crates/progression/src/lib.rs", KILLER_LIB);
    plant(root, "crates/progression/src/settle.rs", KILLER_SETTLE);
}

/// Plants the member `name`, depending on progression, with `files` under its folder.
fn plant_member(root: &Path, name: &str, files: &[(&str, &str)]) {
    plant(
        root,
        &format!("crates/{name}/Cargo.toml"),
        &manifest_of(name, &[OWNER]),
    );
    for (path, text) in files {
        plant(root, &format!("crates/{name}/{path}"), text);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn only_progression_writes_xp_settlement_and_only_coordination_settles() {
    let found = census_in(
        &root(),
        &Path::new(env!("CARGO_TARGET_TMPDIR")).join("settle-census"),
    );
    examined("crate source file(s)", found.sources.clone());
    examined("migration(s)", found.migrations.clone());
    assert_eq!(found.refused, Vec::<String>::new());
    // The positive artifacts: progression's `settle` module and its migration name the table, and
    // the fold's XP step calls the operation.
    let writers: BTreeSet<&str> = [
        "crates/progression/src/settle.rs",
        "migrations/007201_progression_xp_settlement.sql",
    ]
    .into_iter()
    .filter(|path| found.naming.contains(*path))
    .collect();
    assert_eq!(
        writers,
        BTreeSet::from([
            "crates/progression/src/settle.rs",
            "migrations/007201_progression_xp_settlement.sql",
        ]),
        "the settle module and its migration name the table; every file that does: {:?}",
        found.naming
    );
    assert!(
        found
            .calling
            .contains("crates/coordination/src/recompute/xp.rs"),
        "the fold's XP step calls settle; every file that does: {:?}",
        found.calling
    );

    // A planted crate that names the table, a planted migration of another context, a planted
    // crate that calls settle, and a planted coordination caller outside the recompute steps that
    // passes the recompute's cause are refused by name; the owner's correction outside the steps,
    // a recompute step, and a mention in a comment are not.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant_member(
        planted.path(),
        "quests",
        &[
            ("src/lib.rs", "pub mod chest;\n"),
            (
                "src/chest.rs",
                "//! A chest never writes xp_settlement itself.\n\
                 pub fn pay() -> &'static str {\n    \"UPDATE xp_settlement SET amount = 0\" // a debit\n}\n",
            ),
        ],
    );
    plant_member(
        planted.path(),
        "streaks",
        &[
            (
                "src/lib.rs",
                "pub mod grouped;\npub mod note;\npub mod relight;\n",
            ),
            (
                "src/relight.rs",
                "pub use deck_streak_progression::settle::settle;\n",
            ),
            (
                "src/grouped.rs",
                "pub use deck_streak_progression::{settle::{settle as plant_call, SettleRequest as PlantRequest}};\n",
            ),
            (
                "src/note.rs",
                "// deck_streak_progression::settle is coordination's alone\npub fn quiet() {}\n",
            ),
        ],
    );
    plant_member(
        planted.path(),
        CALLER,
        &[
            (
                "src/lib.rs",
                "pub mod correction;\npub mod recompute;\npub mod shortcut;\n",
            ),
            ("src/recompute/mod.rs", "pub mod xp;\n"),
            (
                "src/recompute/xp.rs",
                "use deck_streak_progression::settle::{SettleCause, settle};\n\
                 pub fn step() -> usize { let _ = SettleCause::Recompute; settle() }\n",
            ),
            (
                "src/correction.rs",
                "use deck_streak_progression::settle::{SettleCause, settle};\n\
                 pub fn fix() -> usize { let _ = SettleCause::OwnersCorrection; settle() }\n",
            ),
            (
                "src/shortcut.rs",
                "use deck_streak_progression::settle::{SettleCause, settle};\n\
                 pub fn quick() -> usize { let _ = SettleCause::Recompute; settle() }\n",
            ),
        ],
    );
    plant(
        planted.path(),
        "migrations/007201_progression_xp_settlement.sql",
        "CREATE TABLE xp_settlement (id INTEGER PRIMARY KEY) STRICT;\n",
    );
    plant(
        planted.path(),
        "migrations/999901_quests_debit.sql",
        "-- a planted migration of another context\nDELETE FROM xp_settlement;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/coordination/src/shortcut.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/quests/src/chest.rs names xp_settlement, and only progression's code may",
            "crates/streaks/src/grouped.rs calls settle, and only coordination's code may",
            "crates/streaks/src/relight.rs calls settle, and only coordination's code may",
            "migrations/999901_quests_debit.sql names xp_settlement, and only progression's \
             migrations may",
        ]
    );
}

/// Plants progression's re-exports of `settle` under other names at `root` (one through another
/// alias, one inside a nested module and one through the renamed module) and its own wrapper of
/// `settle` in `inner.rs`.
fn plant_progressions_aliases(root: &Path) {
    plant(
        root,
        "crates/progression/src/lib.rs",
        "pub mod settle;\nmod inner;\n\
         pub use settle::{\n    SettleCause,\n    SettledRow,\n    settle as tally,\n    SettleRequest as TallyRequest,\n    settled_of_day,\n};\n\
         pub use settle as ledger_write;\n\
         pub use self::tally as tally_again;\n\
         pub mod api {\n    pub use super::settle::settle as run_it;\n}\n",
    );
    plant(
        root,
        "crates/progression/src/inner.rs",
        "use crate::settle::settle as tally;\npub fn own() -> usize { tally() }\n",
    );
}

#[test]
fn the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates() {
    // Progression re-exports `settle` under other names, one through another alias, one inside a
    // nested module and one through the renamed module. A caller outside coordination that names
    // only the new names never spells `settle`, and the compiler reports it all the same.
    // Progression's own re-exports and imports of the names, coordination's callers by the same
    // rules as before, a mention in a comment and a crate's own function of an alias's name are not
    // refused. Progression's own wrapper in `inner.rs` is, since progression admits no file (#445).
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant_progressions_aliases(planted.path());
    plant_member(
        planted.path(),
        "streaks",
        &[
            (
                "src/lib.rs",
                "pub mod homonym;\npub mod note;\npub mod tally_user;\n",
            ),
            (
                "src/tally_user.rs",
                "use deck_streak_progression::{TallyRequest, tally};\n\
                 pub fn go() -> usize { let _ = TallyRequest; tally() }\n",
            ),
            (
                "src/note.rs",
                "pub use deck_streak_progression::SettledRow;\n// tally and ledger_write are coordination's\n",
            ),
            (
                "src/homonym.rs",
                "fn tally() -> usize { 1 }\npub fn again() -> usize { tally() }\n",
            ),
        ],
    );
    plant_member(
        planted.path(),
        "quests",
        &[
            (
                "src/lib.rs",
                "pub mod chained_user;\npub mod module_user;\npub mod nested_user;\n",
            ),
            (
                "src/module_user.rs",
                "use deck_streak_progression::ledger_write;\npub fn go() -> usize { ledger_write::settle() }\n",
            ),
            (
                "src/chained_user.rs",
                "pub use deck_streak_progression::tally_again;\n",
            ),
            (
                "src/nested_user.rs",
                "pub use deck_streak_progression::api::run_it;\n",
            ),
        ],
    );
    plant_member(
        planted.path(),
        CALLER,
        &[
            (
                "src/lib.rs",
                "pub mod correction;\npub mod recompute;\npub mod shortcut;\n",
            ),
            ("src/recompute/mod.rs", "pub mod xp;\n"),
            (
                "src/recompute/xp.rs",
                "use deck_streak_progression::{SettleCause, tally};\n\
                 pub fn step() -> usize { let _ = SettleCause::Recompute; tally() }\n",
            ),
            (
                "src/correction.rs",
                "use deck_streak_progression::{SettleCause, tally};\n\
                 pub fn fix() -> usize { let _ = SettleCause::OwnersCorrection; tally() }\n",
            ),
            (
                "src/shortcut.rs",
                "use deck_streak_progression::{SettleCause, tally};\n\
                 pub fn quick() -> usize { let _ = SettleCause::Recompute; tally() }\n",
            ),
        ],
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/coordination/src/shortcut.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/progression/src/inner.rs calls settle inside progression's own code, and only \
             coordination's code may unless progression admits it",
            "crates/quests/src/chained_user.rs calls settle, and only coordination's code may",
            "crates/quests/src/module_user.rs calls settle, and only coordination's code may",
            "crates/quests/src/nested_user.rs calls settle, and only coordination's code may",
            "crates/streaks/src/tally_user.rs calls settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_census_follows_a_crate_alias() {
    // `prog` is progression's crate under another name, so `use crate::prog::tally` reaches the
    // renamed `settle` without ever spelling the crate's own name.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant_member(
        planted.path(),
        "markets",
        &[
            ("src/lib.rs", "pub mod prog;\npub mod via_prog;\n"),
            ("src/prog.rs", "pub use deck_streak_progression as prog;\n"),
            (
                "src/via_prog.rs",
                "use crate::prog::prog::tally;\npub fn go() -> usize { tally() }\n",
            ),
        ],
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        ["crates/markets/src/via_prog.rs calls settle, and only coordination's code may"]
    );
}

#[test]
fn the_census_follows_a_grouped_module_renaming_and_a_chain_read_before_its_link() {
    // `a_chain.rs` renames `tally` and the renamed module's `settle` before `lib.rs` makes either
    // name, and the module is renamed inside a group, by `self`: the compiler resolves each, in
    // whatever order a reader would meet them.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/a_chain.rs",
        "pub use crate::ledger::settle as via_ledger;\npub use crate::tally as tally_early;\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod a_chain;\npub mod settle;\npub use settle::{self as ledger, settle as tally};\n",
    );
    plant_member(
        planted.path(),
        "quests",
        &[
            (
                "src/lib.rs",
                "pub mod early_user;\npub mod ledger_user;\npub mod via_ledger_user;\n",
            ),
            (
                "src/early_user.rs",
                "use deck_streak_progression::a_chain::tally_early;\n\
                 pub fn go() -> usize { tally_early() }\n",
            ),
            (
                "src/ledger_user.rs",
                "use deck_streak_progression::ledger;\npub fn go() -> usize { ledger::settle() }\n",
            ),
            (
                "src/via_ledger_user.rs",
                "use deck_streak_progression::a_chain::via_ledger;\n\
                 pub fn go() -> usize { via_ledger() }\n",
            ),
        ],
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/early_user.rs calls settle, and only coordination's code may",
            "crates/quests/src/ledger_user.rs calls settle, and only coordination's code may",
            "crates/quests/src/via_ledger_user.rs calls settle, and only coordination's code may",
        ]
    );
}

/// A tree planted over the workspace's files, and the refusals the census owes it.
type PlantedRefusal<'a> = (Vec<(&'a str, &'a str)>, Vec<&'a str>);

#[test]
#[allow(clippy::too_many_lines)]
fn the_census_refuses_what_the_compiler_is_not_asked() {
    // Each tree is a valid workspace at `ws` holding one thing the census does not compile or
    // cannot see through: a member's feature, a cargo configuration in either spelling cargo reads,
    // a package in the repository outside the workspace, a proc-macro member, a path package
    // outside the repository, a git package that depends on progression, a lock file cargo would
    // have to change, code that does not compile, and coordination's use of `settle` in a file
    // outside the repository, which no file of the repository places. Each is refused by name,
    // with exactly the refusals its case names: a git package that reaches progression is refused
    // for its dependency and for the graph edge, and coordination's file outside the repository for
    // its cause and for being read from outside the tree.
    const HABITS: &str =
        "[package]\nname = \"deck-streak-habits\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n";
    let features = format!("{HABITS}[features]\nquiet = []\n");
    let proc_macro = format!("{HABITS}[lib]\nproc-macro = true\n");
    let aside = format!("{HABITS}[dependencies]\naside = {{ path = \"../../../aside\" }}\n");
    let patched = format!(
        "{KILLER_WORKSPACE}\n[patch.\"@GIT@\"]\n{PROGRESSION_PACKAGE} = {{ path = \"crates/progression\" }}\n"
    );
    let through_git = format!("{HABITS}[dependencies]\ndeck-streak-g = {{ git = \"@GIT@\" }}\n");
    let git_package = format!(
        "{}\n[dependencies]\n{PROGRESSION_PACKAGE} = {{ git = \"@GIT@\" }}\n",
        killer_package("g")
    );
    let git_progression = killer_package(OWNER);
    let coordination = manifest_of(CALLER, &[OWNER]);
    let trees: [PlantedRefusal<'_>; 10] = [
        (
            vec![("ws/crates/habits/Cargo.toml", &features)],
            vec!["crates/habits/Cargo.toml declares a feature, and the census compiles none"],
        ),
        (
            vec![("ws/.cargo/config.toml", "[build]\nincremental = false\n")],
            vec![
                ".cargo/config.toml configures cargo, and the census compiles with cargo's own \
                 defaults",
            ],
        ),
        (
            vec![("ws/.cargo/config", "[build]\nincremental = false\n")],
            vec![
                ".cargo/config configures cargo, and the census compiles with cargo's own defaults",
            ],
        ),
        (
            vec![(
                "ws/tools/aside/Cargo.toml",
                "[package]\nname = \"aside\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
            )],
            vec![
                "tools/aside/Cargo.toml is a package outside the workspace, which the census does \
                 not compile",
            ],
        ),
        (
            vec![("ws/crates/habits/Cargo.toml", &proc_macro)],
            vec![
                "crates/habits is a proc-macro crate, and rustc reports no deprecation inside a \
                 derive's expansion",
            ],
        ),
        (
            vec![
                ("ws/crates/habits/Cargo.toml", &aside),
                (
                    "aside/Cargo.toml",
                    "[package]\nname = \"aside\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
                ),
                ("aside/src/lib.rs", "pub fn quiet() {}\n"),
            ],
            vec!["aside is a package outside the workspace, which the census does not compile"],
        ),
        (
            vec![
                ("ws/Cargo.toml", &patched),
                ("ws/crates/habits/Cargo.toml", &through_git),
                ("git/g/Cargo.toml", &git_package),
                ("git/g/src/lib.rs", "pub fn quiet() {}\n"),
                ("git/p/Cargo.toml", &git_progression),
                ("git/p/src/lib.rs", "pub fn settle() -> usize {\n    1\n}\n"),
            ],
            vec![
                "depends on progression from outside the workspace, whose callers the census \
                 cannot name",
                "deck-streak-g comes from git+file://",
            ],
        ),
        (
            vec![("ws/Cargo.lock", "version = 4\n")],
            vec![
                "because --locked was passed to prevent this), so the census cannot see its callers",
            ],
        ),
        (
            vec![(
                "ws/crates/habits/src/lib.rs",
                "pub fn broken() -> usize {\n    \"not a number\"\n}\n",
            )],
            vec!["the workspace does not compile (error: could not compile `deck-streak-habits`"],
        ),
        (
            vec![
                ("ws/crates/coordination/Cargo.toml", &coordination),
                (
                    "ws/crates/coordination/src/lib.rs",
                    "#[path = \"../../../../outside.rs\"]\npub mod outside;\n",
                ),
                (
                    "outside.rs",
                    "use deck_streak_progression::settle::SettleCause;\n\
                     pub fn fix() -> usize { let _ = SettleCause::OwnersCorrection; \
                     deck_streak_progression::settle() }\n",
                ),
            ],
            vec![
                "crates/coordination/src/lib.rs calls settle outside the recompute steps, and only \
                 the owner's correction may",
                "crates/coordination's code reads crates/coordination/src/../../../../outside.rs, \
                 which lies outside the tree",
            ],
        ),
    ];
    for (files, lines) in examined("tree(s) the census does not compile", Vec::from(trees)) {
        let planted = tempfile::tempdir().expect("a temporary directory");
        let root = planted.path().join("ws");
        let git = planted.path().join("git");
        let url = format!("file://{}", git.display());
        plant_workspace(&root);
        plant_member(&root, "habits", &[("src/lib.rs", "pub fn quiet() {}\n")]);
        for (path, text) in &files {
            plant(planted.path(), path, &text.replace("@GIT@", &url));
        }
        if git.exists() {
            committed(&git);
        }
        let refused = census(&root).refused;
        assert!(
            refused.len() == lines.len()
                && lines
                    .iter()
                    .all(|line| refused.iter().any(|refusal| refusal.contains(line))),
            "planted {files:?}: {refused:?}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn the_census_names_each_use_in_its_package_and_file() {
    // One workspace holds every use the compiler reports and every rule that judges it: a use under
    // either debug-assertion state, whatever the manifest sets for tests or for one member, and
    // under the abort panic strategy; through progression's re-export and a git dependency's macro,
    // each of which writes it only without debug assertions, whatever the manifest pins for either
    // package; through a git dependency's macro that writes it only without a feature habits'
    // dev-dependency asks for, which a build of its library alone compiles; in a library and in a
    // test, in a test target of another crate, in progression's own test, under an
    // `allow(deprecated)`, inside progression's macro and inside coordination's recompute macro,
    // each expanded where it is called, in a workspace whose default members leave the callers out;
    // a member's own deprecated function, which is not `settle`; coordination's test with any
    // cause; a correction whose cause is only a string and one that passes both causes (one a
    // build script writes is refused with its package, as a package that can name settle); a coordination file compiled by a `#[path]`
    // through `recompute/..`, which is not a recompute step; a nested comment naming the table; and
    // a link that loops back to the root, which the walk for manifests never follows.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(
        planted.path(),
        "Cargo.toml",
        &format!(
            "{KILLER_WORKSPACE}default-members = [\"crates/progression\"]\n\n\
             [profile.test]\ndebug-assertions = true\n\n\
             [profile.dev.package.deck-streak-habits]\ndebug-assertions = true\n\n\
             [profile.test.package.deck-streak-habits]\ndebug-assertions = true\n\n\
             [profile.dev.package.deck-streak-progression]\ndebug-assertions = true\n\n\
             [profile.dev.package.deck-streak-g]\ndebug-assertions = true\n"
        ),
    );
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\npub use settle::settle;\n\
         #[cfg(debug_assertions)]\npub fn later() -> usize {\n    0\n}\n\
         #[cfg(not(debug_assertions))]\npub use settle::settle as later;\n\
         #[macro_export]\nmacro_rules! settle_now {\n    () => {\n        $crate::settle::settle()\n    };\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/tests/own.rs",
        "#[test]\nfn own() {\n    assert_eq!(deck_streak_progression::settle(), 0);\n}\n",
    );
    plant_member(
        planted.path(),
        "habits",
        &[
            (
                "src/lib.rs",
                "pub mod aborting;\npub mod asserted;\npub mod dated;\npub mod dependency;\n\
                 pub mod later;\npub mod released;\npub mod macro_user;\npub mod production;\n\
                 pub mod silenced;\n\
                 /* a /* nested */ note: xp_settlement is progression's */\npub fn quiet() {}\n",
            ),
            (
                "src/aborting.rs",
                "#[cfg(panic = \"abort\")]\npub fn go() -> usize { deck_streak_progression::settle() }\n",
            ),
            (
                "src/dated.rs",
                "#[deprecated(note = \"a member's own\")]\npub fn old() -> usize {\n    0\n}\n\
                 pub fn go() -> usize {\n    old()\n}\n",
            ),
            (
                "src/asserted.rs",
                "#[cfg(debug_assertions)]\npub fn go() -> usize { deck_streak_progression::settle() }\n",
            ),
            (
                "src/released.rs",
                "#[cfg(not(debug_assertions))]\npub fn go() -> usize { deck_streak_progression::settle() }\n",
            ),
            (
                "src/dependency.rs",
                "pub fn go() -> usize { deck_streak_g::later!() }\n",
            ),
            (
                "src/later.rs",
                "pub fn go() -> usize { deck_streak_progression::later() }\n",
            ),
            (
                "src/macro_user.rs",
                "pub fn go() -> usize { deck_streak_progression::settle_now!() }\n",
            ),
            (
                "src/production.rs",
                "pub fn go() -> usize { deck_streak_g::untested!() }\n",
            ),
            (
                "src/silenced.rs",
                "#![allow(deprecated)]\npub fn go() -> usize { deck_streak_progression::settle() }\n",
            ),
            (
                "tests/released.rs",
                "#[cfg(not(debug_assertions))]\n#[test]\nfn released() {\n    \
                 assert_eq!(deck_streak_progression::settle(), 0);\n}\n",
            ),
            (
                "tests/through.rs",
                "#[test]\nfn through() {\n    assert_eq!(deck_streak_progression::settle(), 0);\n}\n",
            ),
        ],
    );
    plant_member(
        planted.path(),
        CALLER,
        &[
            (
                "src/lib.rs",
                "#[macro_use]\npub mod recompute;\npub mod both;\npub mod literal;\npub mod wrapped;\n\
                 #[path = \"recompute/../spoofed.rs\"]\npub mod spoofed;\n",
            ),
            (
                "src/both.rs",
                "use deck_streak_progression::settle::SettleCause;\n\
                 pub fn fix() -> usize {\n    let _ = (SettleCause::OwnersCorrection, SettleCause::Recompute);\n    \
                 deck_streak_progression::settle()\n}\n",
            ),
            (
                "src/recompute/mod.rs",
                "macro_rules! step {\n    () => {\n        deck_streak_progression::settle()\n    };\n}\n\
                 pub fn fold() -> usize { step!() }\n",
            ),
            ("src/wrapped.rs", "pub fn quick() -> usize { step!() }\n"),
            (
                "src/spoofed.rs",
                "use deck_streak_progression::settle::SettleCause;\n\
                 pub fn fold() -> usize { let _ = SettleCause::Recompute; deck_streak_progression::settle() }\n",
            ),
            (
                "src/literal.rs",
                "pub const CAUSE: &str = \"SettleCause::OwnersCorrection\";\n\
                 pub fn fix() -> usize { deck_streak_progression::settle() }\n",
            ),
            (
                "tests/cycle.rs",
                "use deck_streak_progression::settle::SettleCause;\n\
                 #[test]\nfn cycle() {\n    let _ = SettleCause::Recompute;\n    assert_eq!(deck_streak_progression::settle(), 0);\n}\n",
            ),
        ],
    );
    let git = tempfile::tempdir().expect("a temporary directory");
    plant(
        git.path(),
        "Cargo.toml",
        &format!("{}\n[features]\ntested = []\n", killer_package("g")),
    );
    plant(
        git.path(),
        "src/lib.rs",
        "#[cfg(debug_assertions)]\n#[macro_export]\n\
         macro_rules! later {\n    () => {\n        0\n    };\n}\n\
         #[cfg(not(debug_assertions))]\n#[macro_export]\n\
         macro_rules! later {\n    () => {\n        \
         ::deck_streak_progression::settle()\n    };\n}\n\
         #[cfg(feature = \"tested\")]\n#[macro_export]\n\
         macro_rules! untested {\n    () => {\n        0\n    };\n}\n\
         #[cfg(not(feature = \"tested\"))]\n#[macro_export]\n\
         macro_rules! untested {\n    () => {\n        \
         ::deck_streak_progression::settle()\n    };\n}\n",
    );
    committed(git.path());
    let url = format!("file://{}", git.path().display());
    plant(
        planted.path(),
        "crates/habits/Cargo.toml",
        &format!(
            "{}deck-streak-g = {{ git = \"{url}\" }}\n\n[dev-dependencies]\n\
             deck-streak-g = {{ git = \"{url}\", features = [\"tested\"] }}\n",
            manifest_of("habits", &[OWNER]),
        ),
    );
    #[cfg(unix)]
    {
        fs::create_dir_all(planted.path().join("docs")).expect("a folder for the link");
        std::os::unix::fs::symlink("..", planted.path().join("docs/root")).expect("the link");
    }
    let found = census(planted.path());
    examined("planted crate source file(s)", found.sources.clone());
    assert_eq!(
        found.refused,
        [
            "crates/coordination/src/both.rs calls settle outside the recompute steps, and only \
             the owner's correction may",
            "crates/coordination/src/literal.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/coordination/src/spoofed.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/coordination/src/wrapped.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/habits/src/aborting.rs calls settle, and only coordination's code may",
            "crates/habits/src/asserted.rs calls settle, and only coordination's code may",
            "crates/habits/src/dependency.rs calls settle, and only coordination's code may",
            "crates/habits/src/later.rs calls settle, and only coordination's code may",
            "crates/habits/src/macro_user.rs calls settle, and only coordination's code may",
            "crates/habits/src/production.rs calls settle, and only coordination's code may",
            "crates/habits/src/released.rs calls settle, and only coordination's code may",
            "crates/habits/src/silenced.rs calls settle, and only coordination's code may",
            "crates/habits/tests/released.rs calls settle, and only coordination's code may",
            "crates/habits/tests/through.rs calls settle, and only coordination's code may",
        ]
    );
    assert_eq!(
        found.calling,
        BTreeSet::from(
            [
                "crates/coordination/src/both.rs",
                "crates/coordination/src/literal.rs",
                "crates/coordination/src/recompute/mod.rs",
                "crates/coordination/src/spoofed.rs",
                "crates/coordination/src/wrapped.rs",
                "crates/coordination/tests/cycle.rs",
                "crates/habits/src/aborting.rs",
                "crates/habits/src/asserted.rs",
                "crates/habits/src/dependency.rs",
                "crates/habits/src/later.rs",
                "crates/habits/src/macro_user.rs",
                "crates/habits/src/production.rs",
                "crates/habits/src/released.rs",
                "crates/habits/src/silenced.rs",
                "crates/habits/tests/released.rs",
                "crates/habits/tests/through.rs",
            ]
            .map(str::to_owned)
        )
    );
    // The same workspace with a correction that coordination's build script writes and a recompute
    // step includes, which no file of the repository holds: coordination can name settle and now
    // has a build script, so the census refuses it by name before it compiles anything.
    plant(
        planted.path(),
        "crates/coordination/build.rs",
        "fn main() {\n    let out = std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\");\n    \
         let step = \"pub fn made() -> usize {\\n    \
         let _ = deck_streak_progression::settle::SettleCause::OwnersCorrection;\\n    \
         deck_streak_progression::settle()\\n}\\n\";\n    \
         std::fs::write(std::path::Path::new(&out).join(\"step.rs\"), step).expect(\"step.rs\");\n}\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/recompute/mod.rs",
        "macro_rules! step {\n    () => {\n        deck_streak_progression::settle()\n    };\n}\n\
         pub fn fold() -> usize { step!() }\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/step.rs\"));\n",
    );
    let written = census(planted.path());
    assert_eq!(
        written.refused,
        [
            "deck-streak-coordination has a build script and can name settle, and a build script's \
             cfg is one the census's passes never set"
        ]
    );
    assert_eq!(written.calling, BTreeSet::new());
}

#[test]
fn the_census_compiles_with_cargos_own_defaults() {
    // Every variable that could compile the census's build apart from cargo's defaults is removed
    // from cargo's environment, and cargo's own locations are kept.
    let removed = [
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_PROFILE_DEV_DEBUG_ASSERTIONS",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        "CARGO_UNSTABLE_BUILD_STD",
    ];
    let kept = [
        "CARGO_HOME",
        "PATH",
        "RUSTUP_HOME",
        "CARGO_NET_OFFLINE",
        "CARGO_TERM_COLOR",
    ];
    for name in examined("variable(s) the census removes", removed.to_vec()) {
        assert!(scrubbed(name), "{name} is kept");
    }
    for name in examined("variable(s) the census keeps", kept.to_vec()) {
        assert!(!scrubbed(name), "{name} is removed");
    }
}

#[test]
fn the_census_fails_by_name_past_its_cargo_limit() {
    // A cargo run the census waits on past its limit is stopped, and the census fails by name
    // rather than wait on; a run within the limit is read.
    let workspace = tempfile::tempdir().expect("a temporary directory");
    fs::create_dir(workspace.path().join("src")).expect("a source folder");
    fs::write(workspace.path().join("src/lib.rs"), "").expect("a library root");
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname = \"deck-streak-limit\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("a manifest");
    let metadata = ["metadata", "--format-version", "1", "--no-deps"].map(str::to_owned);
    let stopped = cargo(workspace.path(), &metadata, Duration::ZERO);
    assert!(
        stopped
            .as_ref()
            .is_err_and(|reason| reason.contains("ran past 0 s")),
        "a run past its limit is not stopped by name: {stopped:?}"
    );
    let read = cargo(workspace.path(), &metadata, CARGO_LIMIT);
    assert!(
        read.as_ref().is_ok_and(|(ok, _, _)| *ok),
        "a run within its limit is not read: {read:?}"
    );
}

#[test]
fn the_census_fails_by_name_past_its_expansion_limit() {
    // A use expanded through `EXPANSION_LIMIT` macro calls is named by its outermost file, and one
    // expanded through one more fails by name rather than walk on.
    let workspace = tempfile::tempdir().expect("a temporary directory");
    let chain = |calls: usize| {
        let mut span = serde_json::json!({ "file_name": "crates/habits/src/lib.rs" });
        for _ in 1..calls {
            span = serde_json::json!({
                "file_name": "crates/habits/src/lib.rs",
                "expansion": { "span": span },
            });
        }
        let mut primary = span;
        primary["is_primary"] = Value::Bool(true);
        serde_json::json!({ "message": { "spans": [primary] } })
    };
    let target = workspace.path().join("target");
    assert_eq!(
        written_in(&chain(EXPANSION_LIMIT), workspace.path(), &target).map(|files| files.len()),
        Ok(EXPANSION_LIMIT)
    );
    assert_eq!(
        written_in(&chain(EXPANSION_LIMIT + 1), workspace.path(), &target),
        Err(format!(
            "a use of settle expands through more than {EXPANSION_LIMIT} macro calls, so the \
             census cannot name its file"
        ))
    );
}

#[test]
fn the_killer_plants_progressions_own_probe() {
    // Every killer tree and every fence plants progression's build script and probe from the
    // killer's literals, so they must be the census's own: a changed build script or probe would
    // otherwise leave the killer judging a census the workspace no longer runs.
    assert_eq!(KILLER_BUILD, include_str!("../build.rs"));
    let probe = KILLER_SETTLE
        .split_once("#[cfg_attr(")
        .and_then(|(_, rest)| rest.split_once(")]\n"))
        .map(|(probe, _)| format!("#[cfg_attr({probe})]\npub async fn settle("))
        .expect("the killer's probe");
    assert!(probe.contains(PROBE_NOTE), "{probe}");
    assert!(include_str!("../src/settle.rs").contains(&probe), "{probe}");
}

/// One tree of round 5's population (VR5): its files, whether it is a member (it reaches
/// progression's `settle`) or a control (the same text reaching another crate's), and whether the
/// census must refuse it.
struct Planted {
    axis: &'static str,
    label: String,
    member: bool,
    refused: bool,
    files: Vec<(String, String)>,
}

/// The crate a VR5 case reaches: progression's for a member, `deck-streak-other`'s for a control,
/// as (folder, crate, package).
const fn vr5_target(member: bool) -> (&'static str, &'static str, &'static str) {
    if member {
        (
            "progression",
            "deck_streak_progression",
            "deck-streak-progression",
        )
    } else {
        ("other", "deck_streak_other", "deck-streak-other")
    }
}

/// The member `m`'s manifest, with `extra` in its `[package]` table and then `dependencies`.
fn vr5_manifest(dependencies: &str, extra: &str) -> String {
    format!(
        "[package]\nname = \"deck-streak-m\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{extra}\n\
         {dependencies}"
    )
}

/// A plain path dependency of `m` on the case's crate.
fn vr5_dependency(member: bool) -> String {
    let (folder, _, package) = vr5_target(member);
    format!("[dependencies]\n{package} = {{ path = \"../{folder}\" }}\n")
}

/// A function that calls `settle` below `name`, as a path or through a `use`.
fn vr5_call(name: &str, shape: &str) -> String {
    let body = if shape == "path" {
        format!("{name}::settle()")
    } else {
        format!("{{ use {name}::settle as go; go() }}")
    };
    format!("pub fn call() -> usize {{\n    {body}\n}}\n")
}

/// The workspace's manifest with its members.
fn vr5_root(members: &str, extra: &str) -> (String, String) {
    (
        "Cargo.toml".to_owned(),
        format!("[workspace]\nmembers = {members}\nresolver = \"3\"\n{extra}"),
    )
}

/// VR5's population, generated from the axes of Cargo's "Specifying Dependencies", "Workspaces"
/// and "Cargo Targets", the TOML specification and the Rust Reference (read 2026-09-30). rustc compiled
/// every tree, with a deprecation on each crate's `settle`: each member fired progression's in its
/// caller and each control fired the other crate's, so each member is a caller by the compiler's
/// own reading. A control the census cannot read as rustc does is refused too: the rule fails
/// closed there, loudly.
#[allow(clippy::too_many_lines)]
fn vr5_population() -> Vec<Planted> {
    let mut cases = Vec::new();
    let mut add = |axis, label: String, member, refused, files: Vec<(String, String)>| {
        cases.push(Planted {
            axis,
            label,
            member,
            refused: member || refused,
            files,
        });
    };
    let root = vr5_root("[\"crates/*\"]", "");
    // R: files rustc compiles by `#[path]` or `include!`, in and out of the census's reading.
    for mechanism in ["include-items", "include-expr", "path-mod", "cfg-attr-path"] {
        for (location, real, written) in [
            ("src", "crates/m/src/", ""),
            ("src-sub", "crates/m/src/sub/", "sub/"),
            ("member-outside-src", "crates/m/extra/", "../extra/"),
            ("repo-outside-crates", "shared/", "../../../shared/"),
        ] {
            for extension in [".rs", ".in", ".inc", ".txt", "", ".RS", ".rs.in"] {
                let shapes: &[&str] = if mechanism == "include-expr" {
                    &["path"]
                } else {
                    &["path", "use"]
                };
                for shape in shapes {
                    for member in [true, false] {
                        let (_, name, _) = vr5_target(member);
                        let literal = format!("{written}call{extension}");
                        let (lib, payload) = match mechanism {
                            "include-expr" => (
                                format!(
                                    "pub fn call() -> usize {{\n    include!(\"{literal}\")\n}}\n"
                                ),
                                format!("{name}::settle()\n"),
                            ),
                            "include-items" => {
                                (format!("include!(\"{literal}\");\n"), vr5_call(name, shape))
                            }
                            "path-mod" => (
                                format!("#[path = \"{literal}\"]\npub mod call;\n"),
                                vr5_call(name, shape),
                            ),
                            _ => (
                                format!(
                                    "#[cfg_attr(all(), path = \"{literal}\")]\npub mod call;\n"
                                ),
                                vr5_call(name, shape),
                            ),
                        };
                        add(
                            "R reading scope",
                            format!("{mechanism} / {location} / ext '{extension}' / {shape}"),
                            member,
                            false,
                            vec![
                                root.clone(),
                                (
                                    "crates/m/Cargo.toml".to_owned(),
                                    vr5_manifest(&vr5_dependency(member), ""),
                                ),
                                ("crates/m/src/lib.rs".to_owned(), lib),
                                (format!("{real}call{extension}"), payload),
                            ],
                        );
                    }
                }
            }
        }
    }
    // W: crates Cargo builds outside `crates/<member>/src`.
    for shape in ["path", "use"] {
        for member in [true, false] {
            let (folder, name, package) = vr5_target(member);
            let body = vr5_call(name, shape);
            let tool = |up: &str| {
                format!(
                    "[package]\nname = \"deck-streak-t\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                     [dependencies]\n{package} = {{ path = \"{up}crates/{folder}\" }}\n"
                )
            };
            for (label, members, at) in [
                (
                    "member by glob tools/*",
                    "[\"crates/*\", \"tools/*\"]",
                    "tools/t",
                ),
                (
                    "member by path tools/t",
                    "[\"crates/*\", \"tools/t\"]",
                    "tools/t",
                ),
                ("member at the root t", "[\"crates/*\", \"t\"]", "t"),
            ] {
                let up = "../".repeat(at.split('/').count());
                add(
                    "W workspace layout",
                    format!("{label} / {shape}"),
                    member,
                    false,
                    vec![
                        vr5_root(members, ""),
                        (format!("{at}/Cargo.toml"), tool(&up)),
                        (format!("{at}/src/lib.rs"), body.clone()),
                    ],
                );
            }
            add(
                "W workspace layout",
                format!("path dependency auto-member tools/t / {shape}"),
                member,
                false,
                vec![
                    root.clone(),
                    ("tools/t/Cargo.toml".to_owned(), tool("../../")),
                    ("tools/t/src/lib.rs".to_owned(), body.clone()),
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        vr5_manifest(
                            "[dependencies]\ndeck-streak-t = { path = \"../../tools/t\" }\n",
                            "",
                        ),
                    ),
                    (
                        "crates/m/src/lib.rs".to_owned(),
                        "pub use deck_streak_t;\n".to_owned(),
                    ),
                ],
            );
            for (label, written, real) in [
                ("[lib] path at the member root", "lib.rs", "crates/m/lib.rs"),
                (
                    "[lib] path with extension .in",
                    "src/lib.in",
                    "crates/m/src/lib.in",
                ),
                (
                    "[lib] path outside crates",
                    "../../shared/m.rs",
                    "shared/m.rs",
                ),
            ] {
                add(
                    "W workspace layout",
                    format!("{label} / {shape}"),
                    member,
                    false,
                    vec![
                        root.clone(),
                        (
                            "crates/m/Cargo.toml".to_owned(),
                            vr5_manifest(
                                &vr5_dependency(member),
                                &format!("\n[lib]\npath = \"{written}\"\n"),
                            ),
                        ),
                        (real.to_owned(), body.clone()),
                    ],
                );
            }
        }
    }
    // T: a manifest's rename, in every key and value spelling TOML 1.0 reads and every place Cargo
    // reads a dependency.
    let keys = [
        ("bare", "bound"),
        ("basic", "\"bound\""),
        ("literal", "'bound'"),
        ("basic \\u", "\"\\u0062ound\""),
        ("basic \\U", "\"\\U00000062ound\""),
        ("basic mid \\u", "\"bo\\u0075nd\""),
    ];
    let caller = "pub fn call() -> usize {\n    bound::settle()\n}\n";
    for (key_name, key) in keys {
        for member in [true, false] {
            let (folder, _, package) = vr5_target(member);
            let (head, rest) = package.split_once('-').expect("a package with a dash");
            let values = [
                ("basic", format!("\"{package}\"")),
                ("literal", format!("'{package}'")),
                (
                    "basic \\u letter",
                    format!("\"{}\"", package.replacen('d', "\\u0064", 1)),
                ),
                (
                    "basic \\u dash",
                    format!("\"{}\"", package.replacen('-', "\\u002d", 1)),
                ),
                ("ml-basic first newline", format!("\"\"\"\n{package}\"\"\"")),
                ("ml-literal first newline", format!("'''\n{package}'''")),
                (
                    "ml-basic line-ending backslash",
                    format!("\"\"\"{head}-\\\n    {rest}\"\"\""),
                ),
                ("ml-basic", format!("\"\"\"{package}\"\"\"")),
            ];
            for (value_name, value) in &values {
                let path = format!("path = \"../{folder}\"");
                let inherited = format!(
                    "\n[workspace.dependencies]\n{key} = {{ package = {value}, path = \"crates/{folder}\" }}\n"
                );
                let placements = [
                    (
                        "inline",
                        format!("[dependencies]\n{key} = {{ package = {value}, {path} }}\n"),
                        String::new(),
                    ),
                    (
                        "table",
                        format!("[dependencies.{key}]\npackage = {value}\n{path}\n"),
                        String::new(),
                    ),
                    (
                        "dotted",
                        format!("[dependencies]\n{key}.package = {value}\n{key}.{path}\n"),
                        String::new(),
                    ),
                    (
                        "dotted, spaces around the dot",
                        format!("[dependencies]\n{key} . package = {value}\n{key} . {path}\n"),
                        String::new(),
                    ),
                    (
                        "header, spaces around the dot",
                        format!("[ dependencies . {key} ]\npackage = {value}\n{path}\n"),
                        String::new(),
                    ),
                    (
                        "target inline",
                        format!(
                            "[target.'cfg(unix)'.dependencies]\n{key} = {{ package = {value}, {path} }}\n"
                        ),
                        String::new(),
                    ),
                    (
                        "target table",
                        format!(
                            "[target.\"cfg(unix)\".dependencies.{key}]\npackage = {value}\n{path}\n"
                        ),
                        String::new(),
                    ),
                    (
                        "workspace inherited",
                        format!("[dependencies]\n{key} = {{ workspace = true }}\n"),
                        inherited.clone(),
                    ),
                    (
                        "workspace inherited, member key bare",
                        "[dependencies]\nbound = { workspace = true }\n".to_owned(),
                        inherited.clone(),
                    ),
                ];
                for (place, dependencies, extra) in placements {
                    add(
                        "T manifests",
                        format!("key {key_name} / value {value_name} / {place}"),
                        member,
                        false,
                        vec![
                            vr5_root("[\"crates/*\"]", &extra),
                            (
                                "crates/m/Cargo.toml".to_owned(),
                                vr5_manifest(&dependencies, ""),
                            ),
                            ("crates/m/src/lib.rs".to_owned(), caller.to_owned()),
                        ],
                    );
                }
            }
        }
    }
    for (label, description) in [
        ("ml-basic holding #", "\"\"\"a # b\"\"\""),
        (
            "ml-basic holding a header",
            "\"\"\"\n[dependencies]\nbound = 1\n\"\"\"",
        ),
        ("ml-literal holding #", "'''a # b'''"),
        (
            "ml-literal holding a comment and a header",
            "'''\n# c\n[x]\n'''",
        ),
        (
            "ml-basic holding two quotes and #",
            "\"\"\"a \"\" b # c\"\"\"",
        ),
        (
            "ml-basic holding an escaped triple quote",
            "\"\"\"a \\\"\"\" # c\"\"\"",
        ),
        ("literal holding a quote and #", "'a \"# b'"),
        ("basic holding an apostrophe and #", "\"a '# b\""),
        ("ml-basic ending in four quotes", "\"\"\"a\"\"\"\""),
        ("ml-literal ending in four apostrophes", "'''a''''"),
    ] {
        for member in [true, false] {
            let (folder, _, package) = vr5_target(member);
            add(
                "T manifests",
                format!("a [package] description {label}, then a plain rename"),
                member,
                false,
                vec![
                    root.clone(),
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        vr5_manifest(
                            &format!(
                                "[dependencies]\nbound = {{ package = \"{package}\", path = \"../{folder}\" }}\n"
                            ),
                            &format!("description = {description}\n"),
                        ),
                    ),
                    ("crates/m/src/lib.rs".to_owned(), caller.to_owned()),
                ],
            );
        }
    }
    // U: identifiers rustc reads in NFC, and connectors it joins into one.
    let letters = [
        ("e-acute", "\u{e9}", "e\u{301}"),
        ("n-tilde", "\u{f1}", "n\u{303}"),
        ("a-ring", "\u{e5}", "a\u{30a}"),
        ("o-umlaut", "\u{f6}", "o\u{308}"),
        ("hangul-ga", "\u{ac00}", "\u{1100}\u{1161}"),
    ];
    let mut spellings = Vec::new();
    for (letter, nfc, nfd) in letters {
        for (place, before, after) in [("mid", "pr", "g"), ("end", "pr", "")] {
            for (pair, bound, called) in [
                ("NFC/NFD", nfc, nfd),
                ("NFD/NFC", nfd, nfc),
                ("NFD/NFD", nfd, nfd),
                ("NFC/NFC", nfc, nfc),
            ] {
                spellings.push((
                    format!("{letter} {place} {pair}"),
                    format!("{before}{bound}{after}"),
                    format!("{before}{called}{after}"),
                ));
            }
        }
    }
    for (connector, character) in [
        ("undertie", '\u{203f}'),
        ("middle-dot", '\u{b7}'),
        ("dashed-low-line", '\u{fe4f}'),
        ("character-tie", '\u{2040}'),
    ] {
        spellings.push((
            connector.to_owned(),
            format!("pr{character}g"),
            format!("pr{character}g"),
        ));
    }
    for (spelling, bound, called) in &spellings {
        for form in [
            "use as",
            "extern crate as",
            "{self as}",
            "progression op alias",
        ] {
            for shape in ["path", "use"] {
                for member in [true, false] {
                    let (_, name, _) = vr5_target(member);
                    let mut files = vec![
                        root.clone(),
                        (
                            "crates/m/Cargo.toml".to_owned(),
                            vr5_manifest(&vr5_dependency(member), ""),
                        ),
                    ];
                    let lib = match form {
                        "use as" => format!("use {name} as {bound};\n{}", vr5_call(called, shape)),
                        "extern crate as" => {
                            format!(
                                "extern crate {name} as {bound};\n{}",
                                vr5_call(called, shape)
                            )
                        }
                        "{self as}" => {
                            format!(
                                "use {name}::{{self as {bound}}};\n{}",
                                vr5_call(called, shape)
                            )
                        }
                        _ => {
                            let (owner, text) = if member {
                                (
                                    "crates/progression/src/lib.rs",
                                    format!("{KILLER_LIB}pub use settle::settle as t{bound};\n"),
                                )
                            } else {
                                (
                                    VR5_OTHER.0,
                                    format!("{}pub use self::settle as t{bound};\n", VR5_OTHER.1),
                                )
                            };
                            files.push((owner.to_owned(), text));
                            if shape == "path" {
                                format!("pub fn call() -> usize {{\n    {name}::t{called}()\n}}\n")
                            } else {
                                format!(
                                    "use {name}::t{called};\npub fn call() -> usize {{\n    t{called}()\n}}\n"
                                )
                            }
                        }
                    };
                    files.push(("crates/m/src/lib.rs".to_owned(), lib));
                    add(
                        "U identifiers",
                        format!("{form} / {spelling} / {shape}"),
                        member,
                        false,
                        files,
                    );
                }
            }
        }
    }
    // M: macros by example in a member.
    let macros = [
        (
            "ident crate, literal op",
            "macro_rules! go {\n    ($c:ident) => { $c::settle() };\n}\npub fn call() -> usize {\n    go!(@N)\n}\n",
        ),
        (
            "ident crate, ident op",
            "macro_rules! go {\n    ($c:ident, $f:ident) => { $c::$f() };\n}\npub fn call() -> usize {\n    go!(@N, settle)\n}\n",
        ),
        (
            "literal crate, ident op",
            "macro_rules! go {\n    ($f:ident) => { @N::$f() };\n}\npub fn call() -> usize {\n    go!(settle)\n}\n",
        ),
        (
            "path fragment",
            "macro_rules! go {\n    ($c:path) => { $c() };\n}\npub fn call() -> usize {\n    go!(@N::settle)\n}\n",
        ),
        (
            "macro writes the use",
            "macro_rules! go {\n    ($c:ident) => { use $c as p; };\n}\ngo!(@N);\npub fn call() -> usize {\n    p::settle()\n}\n",
        ),
        (
            "macro writes the fn",
            "macro_rules! go {\n    ($c:ident) => { pub fn call() -> usize { $c::settle() } };\n}\ngo!(@N);\n",
        ),
        (
            "tt passthrough",
            "macro_rules! go {\n    ($($t:tt)*) => { $($t)* };\n}\npub fn call() -> usize {\n    go!(@N::settle())\n}\n",
        ),
        (
            "returns the fn item",
            "macro_rules! go {\n    ($c:ident) => { $c::settle };\n}\npub fn call() -> usize {\n    (go!(@N))()\n}\n",
        ),
        (
            "two macros",
            "macro_rules! b {\n    ($c:ident, $f:ident) => { $c::$f() };\n}\nmacro_rules! a {\n    ($c:ident) => { b!($c, settle) };\n}\npub fn call() -> usize {\n    a!(@N)\n}\n",
        ),
    ];
    for (label, text) in macros {
        for spelled in ["canonical", "alias"] {
            for member in [true, false] {
                let (_, name, _) = vr5_target(member);
                let lib = if spelled == "canonical" {
                    text.replace("@N", name)
                } else {
                    format!("use {name} as p2;\n{}", text.replace("@N", "p2"))
                };
                add(
                    "M member macros",
                    format!("{label} / {spelled}"),
                    member,
                    false,
                    vec![
                        root.clone(),
                        (
                            "crates/m/Cargo.toml".to_owned(),
                            vr5_manifest(&vr5_dependency(member), ""),
                        ),
                        ("crates/m/src/lib.rs".to_owned(), lib),
                    ],
                );
            }
        }
    }
    // P: paths the Reference admits; the other crate has no `settle` module, so the two paths
    // through progression's module have no control.
    let paths = [
        (
            "leading ::",
            "pub fn call() -> usize {\n    ::@N::settle()\n}\n",
            true,
        ),
        (
            "crate:: after extern crate",
            "extern crate @N;\npub fn call() -> usize {\n    crate::@N::settle()\n}\n",
            true,
        ),
        (
            "self:: after extern crate",
            "extern crate @N;\npub fn call() -> usize {\n    self::@N::settle()\n}\n",
            true,
        ),
        (
            "empty turbofish",
            "pub fn call() -> usize {\n    @N::settle::<>()\n}\n",
            true,
        ),
        (
            "block-scope glob",
            "pub fn call() -> usize {\n    use @N::*;\n    settle()\n}\n",
            true,
        ),
        (
            "module then fn",
            "pub fn call() -> usize {\n    @N::settle::settle()\n}\n",
            false,
        ),
        (
            "group of the module self",
            "use @N::{settle::{self}};\npub fn call() -> usize {\n    settle::settle()\n}\n",
            false,
        ),
        (
            "fn pointer const",
            "pub const F: fn() -> usize = @N::settle;\npub fn call() -> usize {\n    F()\n}\n",
            true,
        ),
    ];
    for (label, text, controlled) in paths {
        for member in [true, false] {
            if !member && !controlled {
                continue;
            }
            let (_, name, _) = vr5_target(member);
            add(
                "P paths",
                label.to_owned(),
                member,
                false,
                vec![
                    root.clone(),
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        vr5_manifest(&vr5_dependency(member), ""),
                    ),
                    ("crates/m/src/lib.rs".to_owned(), text.replace("@N", name)),
                ],
            );
        }
    }
    cases
}

/// VR5's baseline, lexer and export axes (v5pop.py `baseline`, `gen_paths_lexer`, `gen_exports`):
/// every oracle-VALID case. The glob of the other crate's `settle` module is not one, since the
/// other crate has none.
fn vr5_rest() -> Vec<Planted> {
    let mut cases = Vec::new();
    let mut add = |axis, label: &str, member, files: Vec<(String, String)>| {
        cases.push(Planted {
            axis,
            label: label.to_owned(),
            member,
            refused: member,
            files,
        });
    };
    add("0 baseline", "stubs only", false, Vec::new());
    add(
        "0 baseline",
        "canonical call",
        true,
        vec![
            (
                "crates/m/Cargo.toml".to_owned(),
                vr5_manifest(&vr5_dependency(true), ""),
            ),
            (
                "crates/m/src/lib.rs".to_owned(),
                vr5_call("deck_streak_progression", "path"),
            ),
        ],
    );
    let preludes = [
        ("char quote", "let _ = '\"';"),
        ("byte char quote", "let _ = b'\"';"),
        ("escaped backslash string", "let _ = \"\\\\\";"),
        ("raw string holding // and /*", "let _ = r#\"\" // /*\"#;"),
        ("c string holding //", "let _ = c\"//\";"),
        ("nested block comment holding a quote", "/* /* */ \" */"),
        ("doc line holding a quote", "/// \"\n    let _ = 0;"),
        ("escaped apostrophe char", "let _ = '\\'';"),
        ("label", "'a: loop {\n        break 'a;\n    }"),
        (
            "lifetime then apostrophe string",
            "fn f<'a>(_: &'a str) {}\n    let _ = \"'\";",
        ),
        (
            "raw byte string with a short hash run",
            "let _ = br##\"x\"#\"##;",
        ),
        ("unicode escape char of a quote", "let _ = '\\u{22}';"),
        ("string holding a block opener", "let _ = \"/*\";"),
    ];
    for (label, prelude) in preludes {
        for member in [true, false] {
            let (_, name, _) = vr5_target(member);
            add(
                "L lexer",
                label,
                member,
                vec![
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        vr5_manifest(&vr5_dependency(member), ""),
                    ),
                    (
                        "crates/m/src/lib.rs".to_owned(),
                        format!(
                            "pub fn call() -> usize {{\n    {prelude}\n    {name}::settle()\n}}\n"
                        ),
                    ),
                ],
            );
        }
    }
    let routes = [
        (
            "renamed op re-export in a holder",
            "pub use @N::settle as go;\n",
            "deck_streak_h::go()",
        ),
        (
            "op re-export in a holder's module",
            "pub mod inner {\n    pub use @N::settle;\n}\n",
            "deck_streak_h::inner::settle()",
        ),
        (
            "glob of progression's module",
            "pub use @N::settle::*;\n",
            "deck_streak_h::settle()",
        ),
        (
            "pub extern crate, no alias",
            "pub extern crate @N;\n",
            "deck_streak_h::@N::settle()",
        ),
        (
            "holder renamed by the caller's manifest",
            "pub use @N::*;\n",
            "h2::settle()",
        ),
    ];
    for (label, holder, call) in routes {
        for member in [true, false] {
            if !member && holder.contains("::settle::*") {
                continue;
            }
            let (_, name, _) = vr5_target(member);
            let dependency = if call.starts_with("h2") {
                "[dependencies]\nh2 = { package = \"deck-streak-h\", path = \"../h\" }\n"
            } else {
                "[dependencies]\ndeck-streak-h = { path = \"../h\" }\n"
            };
            add(
                "X exports",
                label,
                member,
                vec![
                    (
                        "crates/h/Cargo.toml".to_owned(),
                        format!("{}\n{}", killer_package("h"), vr5_dependency(member)),
                    ),
                    ("crates/h/src/lib.rs".to_owned(), holder.replace("@N", name)),
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        vr5_manifest(dependency, ""),
                    ),
                    (
                        "crates/m/src/lib.rs".to_owned(),
                        format!(
                            "pub fn call() -> usize {{\n    {}\n}}\n",
                            call.replace("@N", name)
                        ),
                    ),
                ],
            );
        }
    }
    cases
}

/// A proc-macro crate `deck-streak-pm` at `crates/pm` with `body` as its root and `dependencies`.
fn s2_proc_macro(body: &str, dependencies: &str) -> Vec<(String, String)> {
    vec![
        (
            "crates/pm/Cargo.toml".to_owned(),
            format!(
                "{}\n[lib]\nproc-macro = true\n\n{dependencies}",
                killer_package("pm")
            ),
        ),
        (
            "crates/pm/src/lib.rs".to_owned(),
            format!("use proc_macro::TokenStream;\n\n{body}"),
        ),
    ]
}

/// The round-6 axes (S2) the earlier rounds did not measure, generated from Cargo's "Build
/// Scripts", "Cargo Targets", "Specifying Dependencies" and "Profiles" and the Reference's
/// "Procedural Macros", "Macros By Example", "Conditional compilation" and "Diagnostic attributes"
/// (read 2026-09-30). Each case is a tree of its own, for a member reaching progression's `settle`
/// and a control reaching the other crate's; a case whose text reaches `settle` in code no build
/// compiles is a control for both.
#[allow(clippy::too_many_lines)]
fn s2_population() -> Vec<Planted> {
    let mut cases = Vec::new();
    let mut add = |axis, label: String, member, refused, files: Vec<(String, String)>| {
        cases.push(Planted {
            axis,
            label,
            member,
            refused: member || refused,
            files,
        });
    };
    for target in [true, false] {
        let (folder, name, package) = vr5_target(target);
        let dependency = format!("{package} = {{ path = \"../{folder}\" }}\n");
        let normal = format!("[dependencies]\n{dependency}");
        let call = vr5_call(name, "path");
        let main = format!("fn main() {{\n    let _ = {name}::settle();\n}}\n");
        let check = format!("#[test]\nfn check() {{\n    let _ = {name}::settle();\n}}\n");
        // B: build scripts and their build dependencies.
        let written = |text: &str| {
            format!(
                "fn main() {{\n    let out = std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\");\n    \
                 std::fs::write(std::path::Path::new(&out).join(\"gen.rs\"), {text})\n        \
                 .expect(\"gen.rs\");\n}}\n"
            )
        };
        let builds = [
            (
                "build.rs calls settle through [build-dependencies]",
                vr5_manifest(&format!("[build-dependencies]\n{dependency}"), ""),
                "build.rs",
                main.clone(),
                String::new(),
            ),
            (
                "a build script declared by package.build calls settle",
                vr5_manifest(
                    &format!("[build-dependencies]\n{dependency}"),
                    "build = \"gen/make.rs\"",
                ),
                "gen/make.rs",
                main.clone(),
                String::new(),
            ),
            (
                "build.rs writes the call into OUT_DIR, and the lib includes it",
                vr5_manifest(&normal, ""),
                "build.rs",
                written(&format!("{call:?}")),
                "include!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));\n".to_owned(),
            ),
            (
                "build.rs writes the call's name in two pieces",
                vr5_manifest(&normal, ""),
                "build.rs",
                written(&format!(
                    "concat!(\"pub fn call() -> usize {{ {name}::set\", \"tle() }}\")"
                )),
                "include!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));\n".to_owned(),
            ),
            (
                "build.rs sets the cfg the call needs",
                vr5_manifest(&normal, ""),
                "build.rs",
                "fn main() {\n    println!(\"cargo::rustc-check-cfg=cfg(made)\");\n    \
                 println!(\"cargo::rustc-cfg=made\");\n}\n"
                    .to_owned(),
                format!("#[cfg(made)]\n{call}"),
            ),
            (
                "build.rs names the file the lib includes",
                vr5_manifest(&normal, ""),
                "build.rs",
                "fn main() {\n    println!(\"cargo::rustc-env=CALL_FILE=call.in\");\n}\n"
                    .to_owned(),
                "include!(env!(\"CALL_FILE\"));\n".to_owned(),
            ),
        ];
        for (label, manifest, script, text, lib) in builds {
            let mut files = vec![
                ("crates/m/Cargo.toml".to_owned(), manifest),
                (format!("crates/m/{script}"), text),
                ("crates/m/src/lib.rs".to_owned(), lib),
            ];
            if label.contains("names the file") {
                files.push(("crates/m/src/call.in".to_owned(), call.clone()));
            }
            // SPEC-324 R2 (ruling 115, under ruling 108 (1)): an include joined onto any variable
            // but OUT_DIR fails closed, so the census refuses this control by construction, with
            // the line "crates/m/src/lib.rs includes a file the census cannot name, so it cannot
            // read it for xp_settlement".
            let refused = label.contains("names the file");
            add(
                "S2 B build scripts",
                label.to_owned(),
                target,
                refused,
                files,
            );
        }
        add(
            "S2 B build scripts",
            format!("build = false leaves a build.rs naming {folder}'s settle uncompiled"),
            false,
            false,
            vec![
                (
                    "crates/m/Cargo.toml".to_owned(),
                    vr5_manifest(
                        &format!("[build-dependencies]\n{dependency}"),
                        "build = false",
                    ),
                ),
                ("crates/m/build.rs".to_owned(), main.clone()),
                ("crates/m/src/lib.rs".to_owned(), String::new()),
            ],
        );
        // P: proc-macro crates, whose output rustc compiles in the caller.
        let user = |lib: &str| {
            vec![
                (
                    "crates/m/Cargo.toml".to_owned(),
                    vr5_manifest(
                        &format!("{normal}deck-streak-pm = {{ path = \"../pm\" }}\n"),
                        "",
                    ),
                ),
                ("crates/m/src/lib.rs".to_owned(), lib.to_owned()),
            ]
        };
        let tokens = format!("{:?}", call.replace('\n', " "));
        let macros = [
            (
                "a function-like macro writes the call",
                format!(
                    "#[proc_macro]\npub fn call(_: TokenStream) -> TokenStream {{\n    \
                     {tokens}.parse().expect(\"tokens\")\n}}\n"
                ),
                "deck_streak_pm::call!();\n",
            ),
            (
                "a function-like macro joins the call's name from two pieces",
                format!(
                    "#[proc_macro]\npub fn call(_: TokenStream) -> TokenStream {{\n    \
                     concat!(\"pub fn call() -> usize {{ {name}::set\", \"tle() }}\")\n        \
                     .parse()\n        .expect(\"tokens\")\n}}\n"
                ),
                "deck_streak_pm::call!();\n",
            ),
            (
                "an attribute macro adds the call beside its item",
                format!(
                    "#[proc_macro_attribute]\npub fn add(_: TokenStream, item: TokenStream) -> \
                     TokenStream {{\n    let mut out: TokenStream = {tokens}.parse().expect(\
                     \"tokens\");\n    out.extend(item);\n    out\n}}\n"
                ),
                "#[deck_streak_pm::add]\npub struct S;\n",
            ),
            (
                "a derive writes the call into an impl",
                format!(
                    "#[proc_macro_derive(Go)]\npub fn go(_: TokenStream) -> TokenStream {{\n    \
                     \"impl S {{ pub fn call() -> usize {{ {name}::settle() }} }}\"\n        \
                     .parse()\n        .expect(\"tokens\")\n}}\n"
                ),
                "#[derive(deck_streak_pm::Go)]\npub struct S;\n",
            ),
        ];
        for (label, body, lib) in macros {
            let mut files = s2_proc_macro(&body, "");
            files.extend(user(lib));
            add(
                "S2 P proc-macro crates",
                label.to_owned(),
                target,
                true,
                files,
            );
        }
        let mut files = s2_proc_macro(
            &format!(
                "#[proc_macro]\npub fn at(_: TokenStream) -> TokenStream {{\n    let _ = \
                 {name}::settle();\n    TokenStream::new()\n}}\n"
            ),
            &normal,
        );
        files.extend([
            (
                "crates/m/Cargo.toml".to_owned(),
                vr5_manifest(
                    "[dependencies]\ndeck-streak-pm = { path = \"../pm\" }\n",
                    "",
                ),
            ),
            (
                "crates/m/src/lib.rs".to_owned(),
                "deck_streak_pm::at!();\n".to_owned(),
            ),
        ]);
        add(
            "S2 P proc-macro crates",
            "the macro calls settle while it expands".to_owned(),
            target,
            true,
            files,
        );
        let mut files = s2_proc_macro(
            "#[proc_macro_attribute]\npub fn drop_it(_: TokenStream, _: TokenStream) -> \
             TokenStream {\n    TokenStream::new()\n}\n",
            "",
        );
        files.extend(user(&format!("#[deck_streak_pm::drop_it]\n{call}")));
        add(
            "S2 P proc-macro crates",
            format!("an attribute macro drops the item that names {folder}'s settle"),
            false,
            true,
            files,
        );
        // A derive's expansion hides a use from rustc's deprecation, so a proc-macro crate is
        // refused wherever it is a path package: in the workspace, or outside it.
        let mut files = s2_proc_macro(
            &format!(
                "#[proc_macro_derive(Go)]\npub fn go(_: TokenStream) -> TokenStream {{\n    \
                 \"impl S {{ pub fn call() -> usize {{ {name}::settle() }} }}\"\n        \
                 .parse()\n        .expect(\"tokens\")\n}}\n"
            ),
            "",
        );
        for file in &mut files {
            file.0 = file.0.replace("crates/pm/", "../pm/");
        }
        files.extend([
            (
                "crates/m/Cargo.toml".to_owned(),
                vr5_manifest(
                    &format!("{normal}deck-streak-pm = {{ path = \"../../../pm\" }}\n"),
                    "",
                ),
            ),
            (
                "crates/m/src/lib.rs".to_owned(),
                "#[derive(deck_streak_pm::Go)]\npub struct S;\n".to_owned(),
            ),
        ]);
        add(
            "S2 P proc-macro crates",
            "a derive from a path package outside the repository writes the call".to_owned(),
            target,
            true,
            files,
        );
        // T: targets, auto-discovered and declared.
        let targets = [
            (
                "tests/t.rs auto-discovered",
                "",
                "tests/t.rs",
                check.clone(),
            ),
            (
                "benches/b.rs auto-discovered",
                "",
                "benches/b.rs",
                check.clone(),
            ),
            (
                "examples/e.rs auto-discovered",
                "",
                "examples/e.rs",
                main.clone(),
            ),
            (
                "src/bin/b.rs auto-discovered",
                "",
                "src/bin/b.rs",
                main.clone(),
            ),
            (
                "src/main.rs beside the lib",
                "",
                "src/main.rs",
                main.clone(),
            ),
            (
                "tests/t/main.rs auto-discovered",
                "",
                "tests/t/main.rs",
                check.clone(),
            ),
            (
                "benches/b/main.rs auto-discovered",
                "",
                "benches/b/main.rs",
                check.clone(),
            ),
            (
                "examples/e/main.rs auto-discovered",
                "",
                "examples/e/main.rs",
                main.clone(),
            ),
            (
                "src/bin/b/main.rs auto-discovered",
                "",
                "src/bin/b/main.rs",
                main.clone(),
            ),
            (
                "[[test]] at checks/t.rs",
                "\n[[test]]\nname = \"t\"\npath = \"checks/t.rs\"\n",
                "checks/t.rs",
                check.clone(),
            ),
            (
                "[[bench]] at perf/b.rs without a harness",
                "\n[[bench]]\nname = \"b\"\npath = \"perf/b.rs\"\nharness = false\n",
                "perf/b.rs",
                main.clone(),
            ),
            (
                "[[example]] at demos/e.rs",
                "\n[[example]]\nname = \"e\"\npath = \"demos/e.rs\"\n",
                "demos/e.rs",
                main.clone(),
            ),
            (
                "[[bin]] at tools/b.rs",
                "\n[[bin]]\nname = \"b\"\npath = \"tools/b.rs\"\n",
                "tools/b.rs",
                main.clone(),
            ),
        ];
        for (label, extra, path, text) in targets {
            add(
                "S2 T targets",
                label.to_owned(),
                target,
                false,
                vec![
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        format!("{}{extra}", vr5_manifest(&normal, "")),
                    ),
                    ("crates/m/src/lib.rs".to_owned(), String::new()),
                    (format!("crates/m/{path}"), text),
                ],
            );
        }
        add(
            "S2 T targets",
            "tests/t.rs through a dev-dependency".to_owned(),
            target,
            false,
            vec![
                (
                    "crates/m/Cargo.toml".to_owned(),
                    vr5_manifest(&format!("[dev-dependencies]\n{dependency}"), ""),
                ),
                ("crates/m/src/lib.rs".to_owned(), String::new()),
                ("crates/m/tests/t.rs".to_owned(), check.clone()),
            ],
        );
        for (flag, path, text) in [
            ("autotests", "tests/t.rs", &check),
            ("autobenches", "benches/b.rs", &check),
            ("autoexamples", "examples/e.rs", &main),
            ("autobins", "src/bin/b.rs", &main),
        ] {
            add(
                "S2 T targets",
                format!("{flag} = false leaves {path} naming {folder}'s settle uncompiled"),
                false,
                false,
                vec![
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        vr5_manifest(&normal, &format!("{flag} = false")),
                    ),
                    ("crates/m/src/lib.rs".to_owned(), String::new()),
                    (format!("crates/m/{path}"), text.clone()),
                ],
            );
        }
        // D: dependencies from outside `crates/`.
        let vendored = vec![
            (
                "vendor/v/Cargo.toml".to_owned(),
                format!(
                    "{}\n[dependencies]\n{package} = {{ path = \"../../crates/{folder}\" }}\n",
                    killer_package("v")
                ),
            ),
            ("vendor/v/src/lib.rs".to_owned(), call.clone()),
            (
                "crates/m/Cargo.toml".to_owned(),
                vr5_manifest(
                    "[dependencies]\ndeck-streak-v = { path = \"../../vendor/v\" }\n",
                    "",
                ),
            ),
            (
                "crates/m/src/lib.rs".to_owned(),
                "pub use deck_streak_v::call;\n".to_owned(),
            ),
        ];
        add(
            "S2 D dependencies",
            "a path dependency under vendor/ is a member by being one".to_owned(),
            target,
            false,
            vendored.clone(),
        );
        let mut excluded = vendored;
        excluded.push((
            "Cargo.toml".to_owned(),
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"vendor/v\"]\nresolver = \"3\"\n"
                .to_owned(),
        ));
        add(
            "S2 D dependencies",
            "a path dependency excluded from the workspace".to_owned(),
            target,
            true,
            excluded,
        );
        add(
            "S2 D dependencies",
            "a git dependency patched onto the workspace's crate".to_owned(),
            target,
            false,
            vec![
                (
                    "Cargo.toml".to_owned(),
                    format!(
                        "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"3\"\n\n\
                         [patch.\"@GIT@\"]\n{package} = {{ path = \"crates/{folder}\" }}\n"
                    ),
                ),
                (
                    "../git/g/Cargo.toml".to_owned(),
                    format!(
                        "{}\n[dependencies]\n{package} = {{ git = \"@GIT@\" }}\n",
                        killer_package("g")
                    ),
                ),
                ("../git/g/src/lib.rs".to_owned(), call.clone()),
                ("../git/p/Cargo.toml".to_owned(), killer_package(folder)),
                (
                    "../git/p/src/lib.rs".to_owned(),
                    "pub fn settle() -> usize {\n    1\n}\n".to_owned(),
                ),
                (
                    "crates/m/Cargo.toml".to_owned(),
                    vr5_manifest("[dependencies]\ndeck-streak-g = { git = \"@GIT@\" }\n", ""),
                ),
                (
                    "crates/m/src/lib.rs".to_owned(),
                    "pub use deck_streak_g::call;\n".to_owned(),
                ),
            ],
        );
        // M: macros by example beyond VR5's.
        let exported = vec![
            (
                "crates/h/Cargo.toml".to_owned(),
                format!("{}\n{normal}", killer_package("h")),
            ),
            (
                "crates/h/src/lib.rs".to_owned(),
                format!(
                    "pub use {name} as reached;\n#[macro_export]\nmacro_rules! go {{\n    () => \
                     {{ $crate::reached::settle() }};\n}}\n"
                ),
            ),
            (
                "crates/m/Cargo.toml".to_owned(),
                vr5_manifest("[dependencies]\ndeck-streak-h = { path = \"../h\" }\n", ""),
            ),
            (
                "crates/m/src/lib.rs".to_owned(),
                "pub fn call() -> usize {\n    deck_streak_h::go!()\n}\n".to_owned(),
            ),
        ];
        add(
            "S2 M macros",
            "another member's exported macro calls settle through $crate".to_owned(),
            target,
            false,
            exported,
        );
        for (label, lib) in [
            (
                "$m! names the macro that calls",
                format!(
                    "macro_rules! inner {{\n    () => {{ {name}::settle() }};\n}}\nmacro_rules! go \
                     {{\n    ($m:ident) => {{ $m!() }};\n}}\npub fn call() -> usize {{\n    \
                     go!(inner)\n}}\n"
                ),
            ),
            (
                "a macro defines the macro that calls",
                format!(
                    "macro_rules! def {{\n    ($n:ident, $c:ident) => {{\n        macro_rules! $n \
                     {{\n            () => {{ $c::settle() }};\n        }}\n    }};\n}}\ndef!(made, \
                     {name});\npub fn call() -> usize {{\n    made!()\n}}\n"
                ),
            ),
        ] {
            add(
                "S2 M macros",
                label.to_owned(),
                target,
                false,
                vec![
                    ("crates/m/Cargo.toml".to_owned(), vr5_manifest(&normal, "")),
                    ("crates/m/src/lib.rs".to_owned(), lib),
                ],
            );
        }
        // C: configuration a build can set.
        let configured = [
            ("#[cfg(debug_assertions)]", "", "#[cfg(debug_assertions)]\n"),
            (
                "#[cfg(not(debug_assertions))]",
                "",
                "#[cfg(not(debug_assertions))]\n",
            ),
            (
                "#[cfg(panic = \"abort\")] under a release profile that aborts",
                "\n[profile.release]\npanic = \"abort\"\n",
                "#[cfg(panic = \"abort\")]\n",
            ),
            (
                "#[cfg(panic = \"unwind\")] under a dev profile that aborts",
                "\n[profile.dev]\npanic = \"abort\"\n",
                "#[cfg(panic = \"unwind\")]\n",
            ),
            (
                "#[cfg(all(not(debug_assertions), panic = \"abort\"))]",
                "\n[profile.release]\npanic = \"abort\"\n",
                "#[cfg(all(not(debug_assertions), panic = \"abort\"))]\n",
            ),
            (
                "#[cfg(not(settle_census))], the census's own cfg",
                "",
                "#[cfg(not(settle_census))]\n",
            ),
            (
                "a build script's own debug assertions off",
                "\n[profile.dev.build-override]\ndebug-assertions = true\n",
                "",
            ),
        ];
        for (label, profile, attribute) in configured {
            let mut files = vec![
                (
                    "Cargo.toml".to_owned(),
                    format!("[workspace]\nmembers = [\"crates/*\"]\nresolver = \"3\"\n{profile}"),
                ),
                (
                    "crates/m/Cargo.toml".to_owned(),
                    vr5_manifest(&format!("{normal}\n[build-dependencies]\n{dependency}"), ""),
                ),
                (
                    "crates/m/src/lib.rs".to_owned(),
                    format!("{attribute}{call}"),
                ),
            ];
            if label.contains("build script") {
                files.push((
                    "crates/m/build.rs".to_owned(),
                    format!(
                        "fn main() {{\n    #[cfg(not(debug_assertions))]\n    let _ = \
                         {name}::settle();\n}}\n"
                    ),
                ));
                files[2].1 = String::new();
            }
            add("S2 C configuration", label.to_owned(), target, false, files);
        }
        add(
            "S2 C configuration",
            "#[cfg(test)] module of the lib".to_owned(),
            target,
            false,
            vec![
                ("crates/m/Cargo.toml".to_owned(), vr5_manifest(&normal, "")),
                (
                    "crates/m/src/lib.rs".to_owned(),
                    format!("#[cfg(test)]\nmod tests {{\n    {check}}}\n"),
                ),
            ],
        );
        add(
            "S2 C configuration",
            format!("#[cfg(settle_census)] in a member leaves {folder}'s settle uncompiled"),
            false,
            false,
            vec![
                ("crates/m/Cargo.toml".to_owned(), vr5_manifest(&normal, "")),
                (
                    "crates/m/src/lib.rs".to_owned(),
                    format!("#[cfg(settle_census)]\n{call}"),
                ),
            ],
        );
        add(
            "S2 C configuration",
            "a feature gates the call".to_owned(),
            target,
            true,
            vec![
                (
                    "crates/m/Cargo.toml".to_owned(),
                    vr5_manifest(&format!("{normal}\n[features]\nslow = []\n"), ""),
                ),
                (
                    "crates/m/src/lib.rs".to_owned(),
                    format!("#[cfg(not(feature = \"slow\"))]\n{call}"),
                ),
            ],
        );
        add(
            "S2 C configuration",
            "a cargo configuration sets a cfg".to_owned(),
            target,
            true,
            vec![
                (
                    ".cargo/config.toml".to_owned(),
                    "[build]\nrustflags = [\"--cfg\", \"hidden\"]\n".to_owned(),
                ),
                ("crates/m/Cargo.toml".to_owned(), vr5_manifest(&normal, "")),
                (
                    "crates/m/src/lib.rs".to_owned(),
                    format!("#[cfg(not(hidden))]\n{call}"),
                ),
            ],
        );
        // S: attributes and manifests that would silence the probe.
        for (label, extra, lib) in [
            (
                "#[allow(deprecated)] on the call",
                "",
                format!("#[allow(deprecated)]\n{call}"),
            ),
            (
                "#![allow(deprecated)] on the crate",
                "",
                format!("#![allow(deprecated)]\n{call}"),
            ),
            (
                "#[expect(deprecated)] on the call",
                "",
                format!("#[expect(deprecated)]\n{call}"),
            ),
            (
                "#![allow(warnings)] on the crate",
                "",
                format!("#![allow(warnings)]\n{call}"),
            ),
            (
                "#![forbid(deprecated)] on the crate",
                "",
                format!("#![forbid(deprecated)]\n{call}"),
            ),
            (
                "[lints.rust] deprecated = \"allow\"",
                "\n[lints.rust]\ndeprecated = \"allow\"\n",
                call.clone(),
            ),
        ] {
            add(
                "S2 S silencing",
                label.to_owned(),
                target,
                false,
                vec![
                    (
                        "crates/m/Cargo.toml".to_owned(),
                        format!("{}{extra}", vr5_manifest(&normal, "")),
                    ),
                    ("crates/m/src/lib.rs".to_owned(), lib),
                ],
            );
        }
    }
    // A: coordination's attribution, every case reaching progression's `settle`.
    let coordination = |files: &[(&str, &str)]| -> Vec<(String, String)> {
        let mut planted = vec![(
            "crates/coordination/Cargo.toml".to_owned(),
            format!(
                "{}\n[dependencies]\ndeck-streak-progression = {{ path = \"../progression\" }}\n",
                killer_package("coordination")
            ),
        )];
        planted.extend(
            files
                .iter()
                .map(|(path, text)| (format!("crates/coordination/{path}"), (*text).to_owned())),
        );
        if !files.iter().any(|(path, _)| *path == "src/lib.rs") {
            planted.push(("crates/coordination/src/lib.rs".to_owned(), String::new()));
        }
        planted
    };
    let recompute = "let _ = deck_streak_progression::settle::SettleCause::Recompute;\n    \
                     let _ = deck_streak_progression::settle();";
    let correction = "let _ = deck_streak_progression::settle::SettleCause::OwnersCorrection;\n    \
                      let _ = deck_streak_progression::settle();";
    let in_main = |body: &str| format!("fn main() {{\n    {body}\n}}\n");
    let in_test = |body: &str| format!("#[test]\nfn check() {{\n    {body}\n}}\n");
    for (label, member, files) in [
        (
            "src/bin/fix.rs passes the recompute cause",
            true,
            coordination(&[("src/bin/fix.rs", &in_main(recompute))]),
        ),
        (
            "src/bin/fix.rs passes the owner's correction",
            false,
            coordination(&[("src/bin/fix.rs", &in_main(correction))]),
        ),
        (
            "tests/t.rs passes the recompute cause",
            false,
            coordination(&[("tests/t.rs", &in_test(recompute))]),
        ),
        (
            "benches/b.rs passes the recompute cause",
            false,
            coordination(&[("benches/b.rs", &in_test(recompute))]),
        ),
        (
            "examples/e.rs passes the recompute cause",
            false,
            coordination(&[("examples/e.rs", &in_main(recompute))]),
        ),
        (
            "the lib's own test module passes the recompute cause",
            true,
            coordination(&[(
                "src/lib.rs",
                &format!("#[cfg(test)]\nmod tests {{\n    {}}}\n", in_test(recompute)),
            )]),
        ),
        (
            "include! brings a recompute-cause call into a file passing the correction",
            true,
            coordination(&[
                (
                    "src/lib.rs",
                    "pub fn fix() {\n    let _ = \
                     deck_streak_progression::settle::SettleCause::OwnersCorrection;\n    \
                     include!(\"step.in\");\n}\n",
                ),
                ("src/step.in", &format!("{{\n    {recompute}\n}}\n")),
            ]),
        ),
        (
            "include! brings a correction-cause call into a file passing the correction",
            false,
            coordination(&[
                (
                    "src/lib.rs",
                    "pub fn fix() {\n    let _ = \
                     deck_streak_progression::settle::SettleCause::OwnersCorrection;\n    \
                     include!(\"step.in\");\n}\n",
                ),
                ("src/step.in", &format!("{{\n    {correction}\n}}\n")),
            ]),
        ),
        (
            "a recompute step's macro called outside the steps",
            true,
            coordination(&[
                (
                    "src/lib.rs",
                    "pub mod recompute;\npub fn fix() {\n    crate::step!();\n}\n",
                ),
                (
                    "src/recompute/mod.rs",
                    &format!(
                        "#[macro_export]\nmacro_rules! step {{\n    () => {{{{\n    {recompute}\n    \
                         }}}};\n}}\n"
                    ),
                ),
            ]),
        ),
        (
            "a correction macro from outside the steps called in a step",
            false,
            coordination(&[
                (
                    "src/lib.rs",
                    &format!(
                        "#[macro_export]\nmacro_rules! fix {{\n    () => {{{{\n    {correction}\n    \
                         }}}};\n}}\npub mod recompute;\n"
                    ),
                ),
                (
                    "src/recompute/mod.rs",
                    "pub fn step() {\n    crate::fix!();\n}\n",
                ),
            ]),
        ),
        (
            "a #[path] through recompute/.. reaches a file outside the steps",
            true,
            coordination(&[
                (
                    "src/lib.rs",
                    "pub mod recompute;\n#[path = \"recompute/../fixer.rs\"]\npub mod fixer;\n",
                ),
                ("src/recompute/mod.rs", ""),
                (
                    "src/fixer.rs",
                    &format!("pub fn fix() {{\n    {recompute}\n}}\n"),
                ),
            ]),
        ),
        (
            "a recompute step passes the recompute cause",
            false,
            coordination(&[
                ("src/lib.rs", "pub mod recompute;\n"),
                (
                    "src/recompute/mod.rs",
                    &format!("pub fn step() {{\n    {recompute}\n}}\n"),
                ),
            ]),
        ),
        (
            "a #[path] module outside coordination's folder passes the recompute cause",
            true,
            {
                let mut files = coordination(&[(
                    "src/lib.rs",
                    "pub fn fix() {\n    let _ = \
                     deck_streak_progression::settle::SettleCause::OwnersCorrection;\n}\n\
                     #[path = \"../../../shared/call.rs\"]\nmod call;\n",
                )]);
                files.push((
                    "shared/call.rs".to_owned(),
                    format!("pub fn call() {{\n    {recompute}\n}}\n"),
                ));
                files
            },
        ),
    ] {
        add(
            "S2 A coordination's attribution",
            label.to_owned(),
            member,
            false,
            files,
        );
    }
    cases
}

// The killer (SPEC-072 A12, ADR-197 round 6): one generated population, judged by the census tree
// by tree, each tree alone. It holds round 5's population (VR5, every case cargo and rustc compile)
// and the axes no earlier round measured (S2: build scripts, proc-macro crates, targets,
// dependencies, macros, configuration, silencing, and coordination's attribution). A member reaches
// progression's `settle` in code cargo compiles, so the census must refuse its tree; a control
// holds the same text reaching another crate's `settle`, or text no build compiles, so the census
// must accept it, unless the census refuses it by construction (a feature, a cargo configuration, a
// proc-macro crate, a package outside the workspace), which its case says.

/// A killer package's manifest.
fn killer_package(name: &str) -> String {
    format!("[package]\nname = \"deck-streak-{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
}

/// The workspace manifest every killer tree holds.
const KILLER_WORKSPACE: &str = "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"3\"\n";

/// Progression's build script as every killer tree holds it. It is the census's own, and
/// `the_killer_plants_progressions_own_probe` holds it equal to `crates/progression/build.rs`.
const KILLER_BUILD: &str = r#"//! Arms the settle census's probe (SPEC-072 A12, ADR-197): when the census compiles the workspace
//! it sets `SETTLE_CENSUS`, and progression alone is then compiled with the `settle_census` cfg,
//! under which `settle` carries a deprecation that rustc reports at every use. No other crate sees
//! the cfg, so no other crate compiles differently under the census.

#[allow(
    clippy::print_stdout,
    reason = "cargo reads a build script's instructions from its standard output"
)]
fn main() {
    println!("cargo::rustc-check-cfg=cfg(settle_census)");
    println!("cargo::rerun-if-env-changed=SETTLE_CENSUS");
    if std::env::var_os("SETTLE_CENSUS").is_some() {
        println!("cargo::rustc-cfg=settle_census");
    }
}
"#;

/// Progression's root as every killer tree holds it: the module, the operation re-exported under
/// its own name and another, and a type that is not the operation.
const KILLER_LIB: &str = "pub mod settle;\npub use settle::settle;\npub use settle::settle as tally;\npub struct Level;\n";

/// Progression's `settle` module as every killer tree holds it, with the probe written as
/// `crates/progression/src/settle.rs` writes it.
const KILLER_SETTLE: &str = "pub struct SettleRequest;\npub struct SettledRow;\n\
     pub enum SettleCause {\n    Recompute,\n    OwnersCorrection,\n}\n\
     #[cfg_attr(\n    settle_census,\n    \
     deprecated(note = \"the settle census's probe: SPEC-072 A12 names every caller by it\")\n)]\n\
     pub fn settle() -> usize {\n    0\n}\n\
     pub fn settled_of_day() -> usize {\n    0\n}\n\
     pub const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n";

/// The workspace, progression (its real build script, and a `settle` carrying the real probe) and
/// the other crate, as every killer tree holds them.
fn killer_stub() -> Vec<(String, String)> {
    [
        ("Cargo.toml", KILLER_WORKSPACE.to_owned()),
        ("crates/progression/Cargo.toml", killer_package(OWNER)),
        ("crates/progression/build.rs", KILLER_BUILD.to_owned()),
        ("crates/progression/src/lib.rs", KILLER_LIB.to_owned()),
        ("crates/progression/src/settle.rs", KILLER_SETTLE.to_owned()),
        ("crates/other/Cargo.toml", killer_package("other")),
        (VR5_OTHER.0, VR5_OTHER.1.to_owned()),
    ]
    .into_iter()
    .map(|(path, text)| (path.to_owned(), text))
    .collect()
}

/// The other crate every VR5 and S2 tree holds beside progression.
const VR5_OTHER: (&str, &str) = (
    "crates/other/src/lib.rs",
    "pub fn settle() -> usize {\n    0\n}\npub struct Level;\n",
);

/// Commits the planted git repository at `directory`, so that cargo can fetch it.
fn committed(directory: &Path) {
    let steps: [&[&str]; 3] = [
        &["init", "--quiet"],
        &["add", "--all"],
        &[
            "-c",
            "user.name=census",
            "-c",
            "user.email=census@invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--message",
            "planted",
        ],
    ];
    for arguments in steps {
        let status = std::process::Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .expect("git runs");
        assert!(
            status.success(),
            "git {arguments:?} in {}",
            directory.display()
        );
    }
}

/// One judging thread's workspace, planted at one fixed path for every tree it judges. A tree is
/// judged alone: before each one, everything in the workspace but the stub and the target is
/// removed (the previous tree's members, its `Cargo.lock` and its git repository), and a stub file
/// is rewritten only when its text differs. The census compiles each tree in an empty target of its
/// own under the workspace's target directory (`census_in`), so nothing an earlier tree built,
/// fingerprints and build-script output included, reaches a later tree's report (SPEC-072 §12,
/// round 8).
struct KillerWorker {
    _planted: tempfile::TempDir,
    root: PathBuf,
    trees: std::cell::Cell<usize>,
}

impl KillerWorker {
    fn new() -> Self {
        let planted = tempfile::tempdir().expect("a temporary directory");
        let root = planted.path().join("ws");
        Self {
            _planted: planted,
            root,
            trees: std::cell::Cell::new(0),
        }
    }

    /// What the census refuses in the tree holding `stub` and then `files`. A file under `../git/`
    /// is planted beside the workspace and committed as a git repository, whose URL replaces
    /// `@GIT@` in every file.
    fn judge(&self, stub: &[(String, String)], files: &[(String, String)]) -> Vec<String> {
        // A git repository of its own for each tree, so that its URL is one cargo has never seen.
        let number = self.trees.get();
        self.trees.set(number + 1);
        let folder = format!("git{number}");
        let git = self.root.with_file_name(&folder);
        if number > 0 {
            let previous = self.root.with_file_name(format!("git{}", number - 1));
            if previous.exists() {
                fs::remove_dir_all(&previous).expect("the previous tree's git repository");
            }
        }
        if self.root.exists() {
            let kept: BTreeSet<PathBuf> =
                stub.iter().map(|(path, _)| self.root.join(path)).collect();
            Self::prune(&self.root, &self.root, &kept);
        }
        let url = format!("file://{}", git.display());
        for (path, text) in stub {
            let file = self.root.join(path);
            if fs::read_to_string(&file).ok().as_deref() != Some(text) {
                plant(&self.root, path, text);
            }
        }
        for (path, text) in files {
            let path = path.replace("../git/", &format!("../{folder}/"));
            plant(&self.root, &path, &text.replace("@GIT@", &url));
        }
        if git.exists() {
            committed(&git);
        }
        census(&self.root).refused
    }

    /// Removes what `kept` does not hold under `directory`, and the folders that hold none of it,
    /// apart from the target directory of the workspace's root.
    fn prune(root: &Path, directory: &Path, kept: &BTreeSet<PathBuf>) {
        for entry in fs::read_dir(directory).expect("the workspace's folder") {
            let path = entry.expect("an entry").path();
            let is_folder = fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir());
            if is_folder {
                if directory == root && path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                if kept.iter().any(|file| file.starts_with(&path)) {
                    Self::prune(root, &path, kept);
                } else {
                    fs::remove_dir_all(&path).expect("a folder of the previous tree");
                }
            } else if !kept.contains(&path) {
                fs::remove_file(&path).expect("a file of the previous tree");
            }
        }
    }
}

/// The count and the digest of the killer's population, pinned together (main's round-7 addendum,
/// condition 2): a population that loses or changes a tree fails here, not only one that shrinks.
const KILLER_POPULATION: (usize, &str) = (
    2218,
    "f5f615e6ce0c85098b00b2c87b6de5b8fb5bab962b744bf0c316b112d0ea75b3",
);

#[test]
fn the_census_refuses_every_caller_the_compiler_finds() {
    let started = std::time::Instant::now();
    let mut cases = vr5_population();
    cases.extend(vr5_rest());
    cases.extend(s2_population());
    let stub = killer_stub();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8);
    let wrong: Vec<String> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let worker = KillerWorker::new();
                    let mut wrong = Vec::new();
                    while let Some(case) =
                        cases.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                    {
                        let refused = worker.judge(&stub, &case.files);
                        // Every case compiles, so a census that could not compile one has judged
                        // nothing there, and that is wrong whatever the case expects.
                        let unjudged = refused.iter().any(|refusal| {
                            refusal.starts_with("the workspace does not compile")
                                || refusal.starts_with("cargo metadata cannot give the graph")
                        });
                        if unjudged || refused.is_empty() == case.refused {
                            wrong.push(format!(
                                "{} {}: {} | {}",
                                if case.member { "member" } else { "control" },
                                case.axis,
                                case.label,
                                refused.first().map_or("accepted", String::as_str)
                            ));
                        }
                    }
                    wrong
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a judging thread"))
            .collect()
    });
    let mut axes: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for case in &cases {
        let counts = axes.entry(case.axis).or_default();
        counts[if case.member {
            0
        } else if case.refused {
            1
        } else {
            2
        }] += 1;
    }
    for (axis, [members, refused, accepted]) in &axes {
        println!(
            "examined {members} member(s) and {} control(s) of {axis} ({refused} refused by \
             construction)",
            refused + accepted
        );
    }
    for line in wrong.iter().take(40) {
        println!("wrong: {line}");
    }
    let escaping = wrong
        .iter()
        .filter(|line| line.starts_with("member "))
        .count();
    println!(
        "killer examined {} tree(s) in {:?}; members escaping: {escaping}; controls judged \
         wrongly: {}",
        cases.len(),
        started.elapsed(),
        wrong.len() - escaping
    );
    let digest = population_digest(cases.iter().map(|case| {
        (
            format!(
                "{}\0{}\0{}\0{}",
                case.axis, case.label, case.member, case.refused
            ),
            case.files.as_slice(),
        )
    }));
    println!("the killer's population digest: {digest}");
    assert_eq!(
        (cases.len(), digest.as_str()),
        KILLER_POPULATION,
        "the killer's population is not the one pinned"
    );
    assert_eq!(
        wrong.first(),
        None,
        "trees the census judges wrongly: {}",
        wrong.len()
    );
}

// The graph refusal (SPEC-072 A12, ADR-197 round 7; main's round-6 ruling): a build script's cfg
// reaches only its own package, so the census refuses, by name, every package that has a build
// script and can name `settle`: the package that defines it, or one that depends on it, by a
// normal, a build or a dev edge. Progression's own build script is admitted only at a pinned
// digest. When cargo cannot give the graph, the census refuses. A build script's cfg that a
// package which cannot name `settle` sets, and a macro of that package expands into a package that
// can, is a disclosed kind, which the generated population below measures without asserting.

/// A planted workspace holding a member `name` that depends on `on` and carries a build script
/// that prints nothing.
fn plant_scripted(root: &Path, name: &str, on: &[&str]) {
    plant(
        root,
        &format!("crates/{name}/Cargo.toml"),
        &manifest_of(name, on),
    );
    plant(
        root,
        &format!("crates/{name}/src/lib.rs"),
        "pub fn call() {}\n",
    );
    plant(root, &format!("crates/{name}/build.rs"), "fn main() {}\n");
}

#[test]
fn a_build_script_in_a_package_that_depends_on_settle_is_refused_by_name() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant_scripted(planted.path(), "api", &[OWNER]);
    let refused = census(planted.path()).refused;
    println!("examined {} refusal(s)", refused.len());
    assert!(
        refused
            .iter()
            .any(|line| line.contains("deck-streak-api") && line.contains("build script")),
        "a build script in api, which depends on progression, is refused by name: {refused:?}"
    );
}

/// Plants `api`, which carries a build script and holds `dependencies` under `section`, beside the
/// members `others` that depend on progression and hold none.
fn plant_edge(root: &Path, section: &str, dependencies: &str, others: &[&str]) {
    plant_workspace(root);
    for other in others {
        plant_member(root, other, &[("src/lib.rs", "pub fn quiet() {}\n")]);
    }
    plant(
        root,
        "crates/api/Cargo.toml",
        &format!(
            "[package]\nname = \"deck-streak-api\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [{section}]\n{dependencies}"
        ),
    );
    plant(root, "crates/api/src/lib.rs", "pub fn call() {}\n");
    plant(root, "crates/api/build.rs", "fn main() {}\n");
}

/// Whether the census refuses `api` for its build script.
fn refuses_api_for_its_script(root: &Path) -> bool {
    let refused = census(root).refused;
    println!("examined {} refusal(s)", refused.len());
    refused
        .iter()
        .any(|line| line.contains("deck-streak-api") && line.contains("build script"))
}

#[test]
fn a_build_script_reached_through_a_dev_dependency_is_refused() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_edge(
        planted.path(),
        "dev-dependencies",
        "deck-streak-progression = { path = \"../progression\" }\n",
        &[],
    );
    assert!(refuses_api_for_its_script(planted.path()));
}

#[test]
fn a_build_script_reached_through_a_build_dependency_is_refused() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_edge(
        planted.path(),
        "build-dependencies",
        "deck-streak-progression = { path = \"../progression\" }\n",
        &[],
    );
    assert!(refuses_api_for_its_script(planted.path()));
}

#[test]
fn a_build_script_reached_through_a_chain_of_packages_is_refused() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_edge(
        planted.path(),
        "dependencies",
        "deck-streak-mid = { path = \"../mid\" }\n",
        &["mid"],
    );
    assert!(refuses_api_for_its_script(planted.path()));
}

#[test]
fn a_build_script_in_a_package_that_cannot_name_settle_is_accepted() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant_scripted(planted.path(), "kernel", &[]);
    let refused = census(planted.path()).refused;
    println!("examined {} refusal(s)", refused.len());
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "kernel has no internal dependency, so its build script cannot reach settle"
    );
}

#[test]
fn a_one_byte_edit_of_progressions_build_script_is_refused_on_the_pin() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(
        planted.path(),
        "crates/progression/build.rs",
        &format!("{KILLER_BUILD} "),
    );
    let refused = census(planted.path()).refused;
    println!("examined {} refusal(s)", refused.len());
    assert!(
        refused
            .iter()
            .any(|line| line.contains("deck-streak-progression") && line.contains("pin")),
        "an edited build script of progression is refused on the pin: {refused:?}"
    );
}

#[test]
fn a_corrupt_lock_file_is_refused_by_the_fail_closed_arm() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(planted.path(), "Cargo.lock", "this is not a lock file\n");
    let refused = census(planted.path()).refused;
    println!("examined {} refusal(s)", refused.len());
    assert!(
        refused
            .iter()
            .any(|line| line.starts_with("cargo metadata cannot give the graph")),
        "a lock file cargo cannot read makes the census refuse by name: {refused:?}"
    );
}

/// The build-script keys of round 6's population (axis BE): each is a condition on the build
/// environment that a member's build script tests before it sets a cfg.
const BE_KEYS: [(&str, &str); 26] = [
    (
        "OPT_LEVEL is 0",
        r#"std::env::var("OPT_LEVEL").as_deref() == Ok("0")"#,
    ),
    (
        "DEBUG is true",
        r#"std::env::var("DEBUG").as_deref() == Ok("true")"#,
    ),
    (
        "PROFILE is debug",
        r#"std::env::var("PROFILE").as_deref() == Ok("debug")"#,
    ),
    (
        "TARGET equals HOST",
        r#"std::env::var("TARGET").ok() == std::env::var("HOST").ok()"#,
    ),
    (
        "CARGO_CFG_TARGET_OS is linux",
        r#"std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")"#,
    ),
    (
        "CARGO_CFG_DEBUG_ASSERTIONS present",
        r#"std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some()"#,
    ),
    (
        "CARGO_CFG_DEBUG_ASSERTIONS absent",
        r#"std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_none()"#,
    ),
    (
        "CARGO_CFG_PANIC is abort",
        r#"std::env::var("CARGO_CFG_PANIC").as_deref() == Ok("abort")"#,
    ),
    (
        "CARGO_CFG_PANIC is unwind",
        r#"std::env::var("CARGO_CFG_PANIC").as_deref() == Ok("unwind")"#,
    ),
    (
        "CARGO_PKG_NAME is the member's",
        r#"std::env::var("CARGO_PKG_NAME").as_deref() == Ok("deck-streak-m")"#,
    ),
    (
        "a file of the package exists",
        r#"std::path::Path::new("flag.txt").exists()"#,
    ),
    (
        "pointer width 64",
        r#"std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").as_deref() == Ok("64")"#,
    ),
    ("RUSTC is set", r#"std::env::var_os("RUSTC").is_some()"#),
    (
        "NUM_JOBS is not 1",
        r#"std::env::var("NUM_JOBS").as_deref() != Ok("1")"#,
    ),
    ("OUT_DIR is set", r#"std::env::var_os("OUT_DIR").is_some()"#),
    (
        "target feature sse2",
        r#"std::env::var("CARGO_CFG_TARGET_FEATURE").map_or(false, |f| f.split(',').any(|x| x == "sse2"))"#,
    ),
    (
        "no feature of the package",
        r#"std::env::var_os("CARGO_CFG_FEATURE").is_none()"#,
    ),
    (
        "no RUSTC_WRAPPER",
        r#"std::env::var_os("RUSTC_WRAPPER").is_none()"#,
    ),
    (
        "DECK_MODE unset",
        r#"std::env::var_os("DECK_MODE").is_none()"#,
    ),
    (
        "OPT_LEVEL is not 0",
        r#"std::env::var("OPT_LEVEL").as_deref() != Ok("0")"#,
    ),
    (
        "PROFILE is release",
        r#"std::env::var("PROFILE").as_deref() == Ok("release")"#,
    ),
    (
        "DEBUG is false",
        r#"std::env::var("DEBUG").as_deref() == Ok("false")"#,
    ),
    (
        "OPT_LEVEL is 2 (a custom profile)",
        r#"std::env::var("OPT_LEVEL").as_deref() == Ok("2")"#,
    ),
    (
        "NUM_JOBS is 1",
        r#"std::env::var("NUM_JOBS").as_deref() == Ok("1")"#,
    ),
    (
        "DECK_MODE is ship",
        r#"std::env::var("DECK_MODE").as_deref() == Ok("ship")"#,
    ),
    (
        "no census flags",
        r#"std::env::var("CARGO_ENCODED_RUSTFLAGS").map_or(true, |f| f.is_empty())"#,
    ),
];

/// The places a gated call to `settle` is written (axis BE): a function, a `#[path]` module, an
/// `include!` and an integration test.
const BE_GATES: [&str; 4] = ["fn", "path-mod", "include", "test-target"];

/// One tree of axis BE: the member `m` carries a build script that sets the cfg `k` when `key`
/// holds, and code under `k` calls `settle` of the crate `crate_name` (a member reaches
/// progression's, a control the other crate's).
fn be_tree(member: bool, key: &str, gate: &str, custom_profile: bool) -> Vec<(String, String)> {
    let (folder, crate_name, package) = vr5_target(member);
    let script = format!(
        "fn main() {{\n    println!(\"cargo::rustc-check-cfg=cfg(k)\");\n    \
         println!(\"cargo::rerun-if-env-changed=DECK_MODE\");\n    \
         if {key} {{\n        println!(\"cargo::rustc-cfg=k\");\n    }}\n}}\n"
    );
    let call = format!("pub fn call() -> usize {{\n    {crate_name}::settle()\n}}\n");
    let mut files = vec![
        ("crates/m/build.rs".to_owned(), script),
        ("crates/m/flag.txt".to_owned(), "flag\n".to_owned()),
    ];
    match gate {
        "fn" => files.push((
            "crates/m/src/lib.rs".to_owned(),
            format!("#[cfg(k)]\n{call}"),
        )),
        "path-mod" => {
            files.push((
                "crates/m/src/lib.rs".to_owned(),
                "#[cfg_attr(k, path = \"on.rs\")]\n#[cfg_attr(not(k), path = \"off.rs\")]\n\
                 pub mod gated;\n"
                    .to_owned(),
            ));
            files.push(("crates/m/src/on.rs".to_owned(), call));
            files.push((
                "crates/m/src/off.rs".to_owned(),
                "pub fn call() -> usize {\n    0\n}\n".to_owned(),
            ));
        }
        "include" => {
            files.push((
                "crates/m/src/lib.rs".to_owned(),
                "#[cfg(k)]\ninclude!(\"inc.rs\");\n".to_owned(),
            ));
            files.push(("crates/m/src/inc.rs".to_owned(), call));
        }
        _ => {
            files.push((
                "crates/m/src/lib.rs".to_owned(),
                "pub fn nothing() {}\n".to_owned(),
            ));
            files.push((
                "crates/m/tests/t.rs".to_owned(),
                format!("#[cfg(k)]\n#[test]\nfn t() {{\n    let _ = {crate_name}::settle();\n}}\n"),
            ));
        }
    }
    files.push((
        "crates/m/Cargo.toml".to_owned(),
        vr5_manifest(
            &format!("[dependencies]\n{package} = {{ path = \"../{folder}\" }}\n"),
            "",
        ),
    ));
    if custom_profile {
        files.push((
            "Cargo.toml".to_owned(),
            format!("{KILLER_WORKSPACE}\n[profile.ship]\ninherits = \"release\"\nopt-level = 2\n"),
        ));
    }
    files
}

/// The refusals of every tree of `trees`, each judged alone by the census, on up to eight threads.
fn judge_alone(stub: &[(String, String)], trees: &[Vec<(String, String)>]) -> Vec<Vec<String>> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8);
    let mut judged: Vec<(usize, Vec<String>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let worker = KillerWorker::new();
                    let mut done = Vec::new();
                    loop {
                        let at = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(files) = trees.get(at) else {
                            break;
                        };
                        done.push((at, worker.judge(stub, files)));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a judging thread"))
            .collect()
    });
    judged.sort_by_key(|(at, _)| *at);
    judged.into_iter().map(|(_, refused)| refused).collect()
}

/// Generated from axis BE of round 6's population (the build environment's conditions x the four
/// places a gated call is written, and one custom profile): every member's build script keys a cfg
/// on the build environment, so no census pass can know whether the cfg holds, and the census
/// refuses the package by name. Each control holds the same build script in a package that cannot
/// name `settle`, and the census accepts it. The git-dependency axis (BG) is a disclosed kind, a
/// build script's cfg read through a macro by a package that can name `settle` while the script's
/// own package cannot: it is generated and measured here and never asserted (main's round-6
/// ruling).
#[test]
fn the_census_refuses_every_build_script_that_can_name_settle() {
    let stub = killer_stub();
    let mut members: Vec<Vec<(String, String)>> = Vec::new();
    let mut controls: Vec<Vec<(String, String)>> = Vec::new();
    for (_, key) in BE_KEYS {
        for gate in BE_GATES {
            let ship = key == r#"std::env::var("OPT_LEVEL").as_deref() == Ok("2")"#;
            members.push(be_tree(true, key, gate, ship));
            controls.push(be_tree(false, key, gate, ship));
        }
    }
    let generated = BE_KEYS.len() * BE_GATES.len();
    assert_eq!(members.len(), generated);
    let judged = judge_alone(&stub, &members);
    let named = judged
        .iter()
        .filter(|refused| {
            refused
                .iter()
                .any(|line| line.contains("deck-streak-m") && line.contains("build script"))
        })
        .count();
    println!("examined {named} build-script member(s) of {generated} generated");
    let accepted: Vec<usize> = judge_alone(&stub, &controls)
        .iter()
        .enumerate()
        .filter(|(_, refused)| refused.is_empty())
        .map(|(at, _)| at)
        .collect();
    println!(
        "examined {} build-script control(s) accepted of {}",
        accepted.len(),
        controls.len()
    );
    assert_eq!(
        named, generated,
        "every build-script member is refused, naming its package"
    );
    assert_eq!(accepted.len(), controls.len(), "every control is accepted");
}

/// One tree of axis BG: a git dependency `g`, which cannot name `settle`, has a build script that
/// sets the cfg `k` when `key` holds and exports a macro that expands a call to `settle` only under
/// `k`; the member `m` depends on progression and on `g`, and expands the macro.
fn bg_tree(key: &str) -> Vec<(String, String)> {
    let script = format!(
        "fn main() {{\n    println!(\"cargo::rustc-check-cfg=cfg(k)\");\n    \
         if {key} {{\n        println!(\"cargo::rustc-cfg=k\");\n    }}\n}}\n"
    );
    vec![
        (
            "../git/Cargo.toml".to_owned(),
            "[package]\nname = \"g\"\nversion = \"0.1.0\"\nedition = \"2024\"\n".to_owned(),
        ),
        ("../git/build.rs".to_owned(), script),
        (
            "../git/src/lib.rs".to_owned(),
            "#[cfg(k)]\n#[macro_export]\nmacro_rules! call {\n    () => {\n        \
             deck_streak_progression::settle()\n    };\n}\n#[cfg(not(k))]\n\
             #[macro_export]\nmacro_rules! call {\n    () => {\n        0\n    };\n}\n"
                .to_owned(),
        ),
        (
            "crates/m/Cargo.toml".to_owned(),
            vr5_manifest(
                "[dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n\
                 g = { git = \"@GIT@\" }\n",
                "",
            ),
        ),
        (
            "crates/m/src/lib.rs".to_owned(),
            "pub fn call() -> usize {\n    g::call!()\n}\n".to_owned(),
        ),
    ]
}

/// Axis BG is measured and printed, never asserted (main's round-6 ruling): a git dependency's
/// build script, in a package that cannot name `settle`, sets a cfg that the macro it exports reads
/// where a package that can name `settle` expands it. Each member's line says whether the census
/// refused it and what it said.
#[test]
fn the_git_dependency_build_scripts_are_measured_and_the_kind_is_disclosed() {
    let stub = killer_stub();
    let trees: Vec<Vec<(String, String)>> = BE_KEYS.iter().map(|(_, key)| bg_tree(key)).collect();
    let judged = judge_alone(&stub, &trees);
    for (index, refused) in judged.iter().enumerate() {
        println!("BG member {index} ({}): {refused:?}", BE_KEYS[index].0);
    }
    println!(
        "examined {} git-dependency build-script member(s), {} refused",
        judged.len(),
        judged.iter().filter(|refused| !refused.is_empty()).count()
    );
    assert_eq!(judged.len(), BE_KEYS.len());
}

/// A build script that sets the cfg `k` only in a release build, which no census pass is.
fn r7_script() -> String {
    "fn main() {\n    println!(\"cargo::rustc-check-cfg=cfg(k)\");\n    \
     if std::env::var(\"PROFILE\").as_deref() == Ok(\"release\") {\n        \
     println!(\"cargo::rustc-cfg=k\");\n    }\n}\n"
        .to_owned()
}

/// A package manifest: its name, `extra` lines in `[package]`, then `rest`.
fn r7_pkg(name: &str, extra: &str, rest: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{extra}\n{rest}"
    )
}

/// A gated call to `settle` of `krate` under the cfg `k`.
fn r7_gated(krate: &str) -> String {
    format!("#[cfg(k)]\npub fn call() -> usize {{\n    {krate}::settle()\n}}\n")
}

/// A `[dependencies]` table on progression by its path.
const R7_ON_PROGRESSION: &str =
    "[dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n";

/// A planted file: its path under the workspace and its text.
fn r7f(path: &str, text: &str) -> (String, String) {
    (path.to_owned(), text.to_owned())
}

/// A planted tree's files, each a path and its text.
type TreeFiles = Vec<(String, String)>;

/// One case of round 7's population: its name, what the census answers, and its files.
type R7Case = (&'static str, &'static str, TreeFiles);

/// Verify round 7's hostile population (46 trees) of the graph refusal, the owner, the pin and the
/// fail-closed arm, as that verifier generated it: each case's name, what the census answers with
/// the owner found by path, and its files.
#[allow(clippy::too_many_lines)]
fn r7_population() -> Vec<R7Case> {
    let script = r7_script();
    let gated = r7_gated("deck_streak_progression");
    let m = |rest: &str| r7_pkg("deck-streak-m", "", rest);
    let decoy = |version: &str| {
        vec![
            r7f(
                "../git/Cargo.toml",
                &format!(
                    "[package]\nname = \"deck-streak-progression\"\nversion = \"{version}\"\nedition = \"2024\"\n"
                ),
            ),
            r7f("../git/src/lib.rs", "pub fn noop() {}\n"),
            r7f(
                "crates/d/Cargo.toml",
                &r7_pkg(
                    "deck-streak-d",
                    "",
                    "[dependencies]\ndecoy = { package = \"deck-streak-progression\", git = \"@GIT@\" }\n",
                ),
            ),
            r7f("crates/d/src/lib.rs", "pub fn d() {}\n"),
        ]
    };
    let macro_pair = |outside: &str| {
        format!(
            "{KILLER_LIB}#[cfg(not(settle_census))]\n#[macro_export]\nmacro_rules! tally_all {{\n    () => {{\n        {outside}\n    }};\n}}\n\
             #[cfg(settle_census)]\n#[macro_export]\nmacro_rules! tally_all {{\n    () => {{\n        0\n    }};\n}}\n"
        )
    };
    let mut cases: Vec<R7Case> = Vec::new();
    // Graph refusal: every edge and manifest shape.
    cases.push((
        "H01 normal edge",
        "refused",
        vec![
            r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", &gated),
        ],
    ));
    cases.push(("H02 build edge only", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[build-dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
    ]));
    cases.push((
        "H03 dev edge only, gated test",
        "refused",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                &m("[dev-dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n"),
            ),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", "pub fn nothing() {}\n"),
            r7f(
                "crates/m/tests/t.rs",
                "#[cfg(k)]\n#[test]\nfn t() {\n    let _ = deck_streak_progression::settle();\n}\n",
            ),
        ],
    ));
    cases.push(("H04 target-specific edge cfg(unix)", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[target.'cfg(unix)'.dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", &gated),
    ]));
    cases.push(("H05 target-specific edge cfg(windows)", "either", vec![
        r7f("crates/m/Cargo.toml", &m("[target.'cfg(windows)'.dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", "#[cfg(all(k, windows))]\npub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n"),
    ]));
    cases.push(("H06 renamed dependency", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[dependencies]\nprog = { package = \"deck-streak-progression\", path = \"../progression\" }\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", &r7_gated("prog")),
    ]));
    cases.push(("H07 optional dependency behind a feature", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ndeck-streak-progression = { path = \"../progression\", optional = true }\n\n[features]\np = [\"dep:deck-streak-progression\"]\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", "#[cfg(all(k, feature = \"p\"))]\npub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n"),
    ]));
    cases.push(("H08 optional dependency on by default", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ndeck-streak-progression = { path = \"../progression\", optional = true }\n\n[features]\ndefault = [\"p\"]\np = [\"dep:deck-streak-progression\"]\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", "#[cfg(all(k, feature = \"p\"))]\npub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n"),
    ]));
    cases.push(("H09 workspace-inherited dependency", "refused", vec![
        r7f("Cargo.toml", &format!("{KILLER_WORKSPACE}\n[workspace.dependencies]\ndeck-streak-progression = {{ path = \"crates/progression\" }}\n")),
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ndeck-streak-progression.workspace = true\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", &gated),
    ]));
    cases.push(("H10 path dependency outside crates/ inside the root", "refused", vec![
        r7f("vendor/x/Cargo.toml", &r7_pkg("deck-streak-x", "", "[dependencies]\ndeck-streak-progression = { path = \"../../crates/progression\" }\n")),
        r7f("vendor/x/build.rs", &script),
        r7f("vendor/x/src/lib.rs", &gated),
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ndeck-streak-x = { path = \"../../vendor/x\" }\n")),
        r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
    ]));
    cases.push(("H10b path dependency outside the workspace root", "refused", vec![
        r7f("../git/x/Cargo.toml", &r7_pkg("deck-streak-x", "", "[dependencies]\ndeck-streak-progression = { path = \"../../ws/crates/progression\" }\n")),
        r7f("../git/x/build.rs", &script),
        r7f("../git/x/src/lib.rs", &gated),
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ndeck-streak-x = { path = \"../../../git/x\" }\n")),
        r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
    ]));
    cases.push((
        "H11 build = \"gen.rs\"",
        "refused",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                &r7_pkg("deck-streak-m", "build = \"gen.rs\"", R7_ON_PROGRESSION),
            ),
            r7f("crates/m/gen.rs", &script),
            r7f("crates/m/src/lib.rs", &gated),
        ],
    ));
    cases.push(("H12 build = the pinned file of progression, in another package", "refused", vec![
        r7f("crates/m/Cargo.toml", &r7_pkg("deck-streak-m", "build = \"../progression/build.rs\"", R7_ON_PROGRESSION)),
        r7f("crates/m/src/lib.rs", "#[cfg(not(settle_census))]\npub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n"),
    ]));
    cases.push((
        "H13 build = false with a build.rs present",
        "accepted",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                &r7_pkg("deck-streak-m", "build = false", R7_ON_PROGRESSION),
            ),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", &gated),
        ],
    ));
    cases.push(("H14 proc-macro with a build script", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[lib]\nproc-macro = true\n\n[dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", "#[cfg(k)]\n#[allow(dead_code)]\nfn call() -> usize {\n    deck_streak_progression::settle()\n}\n"),
    ]));
    cases.push((
        "H15 links package",
        "refused",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                &r7_pkg("deck-streak-m", "links = \"deckm\"", R7_ON_PROGRESSION),
            ),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", &gated),
        ],
    ));
    cases.push((
        "H16 links with build = false",
        "refused",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                &r7_pkg(
                    "deck-streak-m",
                    "links = \"deckm\"\nbuild = false",
                    R7_ON_PROGRESSION,
                ),
            ),
            r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
        ],
    ));
    cases.push((
        "H17 two-hop chain through a re-export",
        "refused",
        vec![
            r7f(
                "crates/mid/Cargo.toml",
                &r7_pkg("deck-streak-mid", "", R7_ON_PROGRESSION),
            ),
            r7f(
                "crates/mid/src/lib.rs",
                "pub use deck_streak_progression::settle;\n",
            ),
            r7f(
                "crates/m/Cargo.toml",
                &m("[dependencies]\ndeck-streak-mid = { path = \"../mid\" }\n"),
            ),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", &r7_gated("deck_streak_mid")),
        ],
    ));
    cases.push((
        "H18 three-hop chain",
        "refused",
        vec![
            r7f(
                "crates/b/Cargo.toml",
                &r7_pkg("deck-streak-b", "", R7_ON_PROGRESSION),
            ),
            r7f(
                "crates/b/src/lib.rs",
                "pub use deck_streak_progression::settle;\n",
            ),
            r7f(
                "crates/a/Cargo.toml",
                &r7_pkg(
                    "deck-streak-a",
                    "",
                    "[dependencies]\ndeck-streak-b = { path = \"../b\" }\n",
                ),
            ),
            r7f("crates/a/src/lib.rs", "pub use deck_streak_b::settle;\n"),
            r7f(
                "crates/m/Cargo.toml",
                &m("[dependencies]\ndeck-streak-a = { path = \"../a\" }\n"),
            ),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", &r7_gated("deck_streak_a")),
        ],
    ));
    cases.push(("H19 the root package with a build script", "refused", vec![
        r7f("Cargo.toml", &format!("[package]\nname = \"deck-streak-root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\ndeck-streak-progression = {{ path = \"crates/progression\" }}\n\n{KILLER_WORKSPACE}")),
        r7f("build.rs", &script),
        r7f("src/lib.rs", &gated),
    ]));
    cases.push((
        "H20 [patch] of a git dependency by a member",
        "refused",
        vec![
            r7f("../git/Cargo.toml", &r7_pkg("g", "", "")),
            r7f("../git/src/lib.rs", "pub fn noop() {}\n"),
            r7f(
                "Cargo.toml",
                &format!("{KILLER_WORKSPACE}\n[patch.\"@GIT@\"]\ng = {{ path = \"crates/gp\" }}\n"),
            ),
            r7f("crates/gp/Cargo.toml", &r7_pkg("g", "", R7_ON_PROGRESSION)),
            r7f("crates/gp/build.rs", &script),
            r7f("crates/gp/src/lib.rs", &gated),
            r7f(
                "crates/m/Cargo.toml",
                &m("[dependencies]\ng = { git = \"@GIT@\" }\n"),
            ),
            r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
        ],
    ));
    // The owner found by name: a git package that carries progression's name.
    let mut h21 = decoy("0.0.1");
    h21.extend([
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", &gated),
    ]);
    cases.push((
        "H21 a git package named as progression, version 0.0.1, beside a member's build script",
        "refused",
        h21,
    ));
    let mut h22 = decoy("0.2.0");
    h22.extend([
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", &gated),
    ]);
    cases.push(("H22 the same, version 0.2.0", "refused", h22));
    let mut h23 = decoy("0.0.1");
    h23.push(r7f(
        "crates/progression/build.rs",
        &format!("{KILLER_BUILD} "),
    ));
    cases.push((
        "H23 the 0.0.1 git package beside a one-byte edit of the pinned script",
        "refused",
        h23,
    ));
    let mut h23b = decoy("0.0.1");
    h23b.extend([
        r7f("crates/progression/build.rs", "fn main() {}\n"),
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f(
            "crates/m/src/lib.rs",
            "pub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n",
        ),
    ]);
    cases.push(("H23b the 0.0.1 git package beside a disarmed progression script and a member calling settle", "refused", h23b));
    let mut at_020 = decoy("0.2.0");
    at_020.extend([
        r7f("crates/progression/build.rs", "fn main() {}\n"),
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f(
            "crates/m/src/lib.rs",
            "pub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n",
        ),
    ]);
    cases.push((
        "H23c control of H23b: the git package at 0.2.0",
        "refused",
        at_020,
    ));
    // The pinned script's own cfg, read by progression's code.
    cases.push((
        "H24 progression's macro expands settle only without settle_census",
        "disclosed",
        vec![
            r7f(
                "crates/progression/src/lib.rs",
                &macro_pair("$crate::settle()"),
            ),
            r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn call() -> usize {\n    deck_streak_progression::tally_all!()\n}\n",
            ),
        ],
    ));
    cases.push(("H24b progression re-exports settle as step only without settle_census", "disclosed", vec![
        r7f("crates/progression/src/lib.rs", &format!("{KILLER_LIB}#[cfg(not(settle_census))]\npub use settle::settle as step;\n#[cfg(settle_census)]\npub use settle::settled_of_day as step;\n")),
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f("crates/m/src/lib.rs", "pub fn call() -> usize {\n    deck_streak_progression::step()\n}\n"),
    ]));
    cases.push((
        "H25 control: the macro's other arm names settled_of_day",
        "accepted",
        vec![
            r7f(
                "crates/progression/src/lib.rs",
                &macro_pair("$crate::settle::settled_of_day()"),
            ),
            r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn call() -> usize {\n    deck_streak_progression::tally_all!()\n}\n",
            ),
        ],
    ));
    cases.push((
        "H26 sanity: a member calls settle",
        "refused",
        vec![
            r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n",
            ),
        ],
    ));
    // The pin.
    cases.push((
        "H27 pin: a trailing newline",
        "refused",
        vec![r7f(
            "crates/progression/build.rs",
            &format!("{KILLER_BUILD}\n"),
        )],
    ));
    cases.push((
        "H28 pin: CRLF line endings",
        "refused",
        vec![r7f(
            "crates/progression/build.rs",
            &KILLER_BUILD.replace('\n', "\r\n"),
        )],
    ));
    cases.push((
        "H29 pin: one byte in a comment",
        "refused",
        vec![r7f(
            "crates/progression/build.rs",
            &KILLER_BUILD.replacen("Arms", "arms", 1),
        )],
    ));
    cases.push((
        "H30 pin: build = b2.rs holding the pinned bytes",
        "accepted",
        vec![
            r7f(
                "crates/progression/Cargo.toml",
                &format!("{}build = \"b2.rs\"\n", killer_package(OWNER)),
            ),
            r7f("crates/progression/b2.rs", KILLER_BUILD),
        ],
    ));
    cases.push((
        "H31 progression build = false and a member calls settle",
        "refused",
        vec![
            r7f(
                "crates/progression/Cargo.toml",
                &format!("{}build = false\n", killer_package(OWNER)),
            ),
            r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn call() -> usize {\n    deck_streak_progression::settle()\n}\n",
            ),
        ],
    ));
    cases.push((
        "H32 progression build = none.rs, an empty script",
        "refused",
        vec![
            r7f(
                "crates/progression/Cargo.toml",
                &format!("{}build = \"none.rs\"\n", killer_package(OWNER)),
            ),
            r7f("crates/progression/none.rs", "fn main() {}\n"),
        ],
    ));
    // Fail closed.
    cases.push((
        "H33 fail closed: a member manifest cargo cannot parse",
        "refused",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                "[package\nname = \"deck-streak-m\"\n",
            ),
            r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
        ],
    ));
    cases.push((
        "H34 fail closed: a lock file with no package",
        "refused",
        vec![r7f("Cargo.lock", "version = 4\n")],
    ));
    cases.push(("H35 fail closed: a locked git source never fetched (offline)", "refused", vec![
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ng = { git = \"file:///nonexistent/v7-never\" }\n")),
        r7f("crates/m/src/lib.rs", "pub fn call() {}\n"),
        r7f("Cargo.lock", "version = 4\n\n[[package]]\nname = \"deck-streak-m\"\nversion = \"0.1.0\"\ndependencies = [\n \"g\",\n]\n\n[[package]]\nname = \"deck-streak-other\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"deck-streak-progression\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"g\"\nversion = \"0.1.0\"\nsource = \"git+file:///nonexistent/v7-never#0123456789abcdef0123456789abcdef01234567\"\n"),
    ]));
    cases.push(("H36 the owner under another package name", "refused", vec![
        r7f("crates/progression/Cargo.toml", "[package]\nname = \"deck-streak-progress\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        r7f("crates/m/Cargo.toml", &m("[dependencies]\ndeck-streak-progress = { path = \"../progression\" }\n")),
        r7f("crates/m/build.rs", &script),
        r7f("crates/m/src/lib.rs", &r7_gated("deck_streak_progress")),
    ]));
    // Controls and the disclosed kind.
    cases.push((
        "H37 control: a build script in a package progression depends on",
        "accepted",
        vec![
            r7f(
                "crates/progression/Cargo.toml",
                &format!(
                    "{}\n[dependencies]\ndeck-streak-x = {{ path = \"../x\" }}\n",
                    killer_package(OWNER)
                ),
            ),
            r7f("crates/x/Cargo.toml", &r7_pkg("deck-streak-x", "", "")),
            r7f("crates/x/build.rs", &script),
            r7f("crates/x/src/lib.rs", "pub fn x() {}\n"),
        ],
    ));
    cases.push((
        "H38 control: a member with a build script reaching the other crate",
        "accepted",
        vec![
            r7f(
                "crates/m/Cargo.toml",
                &m("[dependencies]\ndeck-streak-other = { path = \"../other\" }\n"),
            ),
            r7f("crates/m/build.rs", &script),
            r7f("crates/m/src/lib.rs", &r7_gated("deck_streak_other")),
        ],
    ));
    let exported = |key: &str| {
        vec![
            r7f("crates/g/Cargo.toml", &r7_pkg("deck-streak-g", "", "")),
            r7f(
                "crates/g/build.rs",
                &format!(
                    "fn main() {{\n    println!(\"cargo::rustc-check-cfg=cfg(k)\");\n    if {key} {{\n        println!(\"cargo::rustc-cfg=k\");\n    }}\n}}\n"
                ),
            ),
            r7f(
                "crates/g/src/lib.rs",
                "#[cfg(k)]\n#[macro_export]\nmacro_rules! call {\n    () => {\n        deck_streak_progression::settle()\n    };\n}\n#[cfg(not(k))]\n#[macro_export]\nmacro_rules! call {\n    () => {\n        0\n    };\n}\n",
            ),
            r7f(
                "crates/m/Cargo.toml",
                &m(
                    "[dependencies]\ndeck-streak-progression = { path = \"../progression\" }\ndeck-streak-g = { path = \"../g\" }\n",
                ),
            ),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn call() -> usize {\n    deck_streak_g::call!()\n}\n",
            ),
        ]
    };
    cases.push((
        "H39 disclosed kind, workspace package: release-keyed cfg read by an exported macro",
        "disclosed",
        exported(r#"std::env::var("PROFILE").as_deref() == Ok("release")"#),
    ));
    cases.push((
        "H40 disclosed kind, workspace package: a cfg keyed on the census's variable",
        "disclosed",
        exported(r#"std::env::var_os("SETTLE_CENSUS").is_none()"#),
    ));
    cases.push(("H41 round 6's names_each_use case: a member's build script writes OUT_DIR code a module includes", "refused", vec![
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f("crates/m/build.rs", "fn main() {\n    let out = std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\");\n    std::fs::write(std::path::Path::new(&out).join(\"step.rs\"), \"pub fn made() -> usize {\\n    deck_streak_progression::settle()\\n}\\n\").expect(\"step.rs\");\n}\n"),
        r7f("crates/m/src/lib.rs", "include!(concat!(env!(\"OUT_DIR\"), \"/step.rs\"));\n"),
    ]));
    cases.push(("H42 progression's const fn pointer to settle only without settle_census, called by a member", "disclosed", vec![
        r7f("crates/progression/src/lib.rs", &format!("{KILLER_LIB}#[cfg(not(settle_census))]\npub const STEP: fn() -> usize = settle::settle;\n#[cfg(settle_census)]\npub const STEP: fn() -> usize = settle::settled_of_day;\n")),
        r7f("crates/m/Cargo.toml", &m(R7_ON_PROGRESSION)),
        r7f("crates/m/src/lib.rs", "pub fn call() -> usize {\n    (deck_streak_progression::STEP)()\n}\n"),
    ]));
    cases
}
// Round 8 (main's round-7 ruling and its addendum; SPEC-072 §12, ADR-197): the census finds its
// owner by path and refuses, by name, every graph where that cannot be told; every lookup it makes
// is unique or refused; and its verdict depends only on the tree it judges. Every census compiles
// in an empty target of its own, with only the environment it names, and refuses a cargo
// configuration from outside the tree and code that reads a file outside the tree or a variable
// the host sets. The killers below hold verify round 7's population, its order and copy pairs, and
// the carried-state generation main named for round 8.

/// The SHA-256, in lowercase hex, of a population: each case's name, then each file's path and
/// text, every field closed by a NUL and every case by a one, so that each count pin is paired with
/// the set it counts (main's round-7 addendum, condition 2).
fn population_digest<'a>(
    cases: impl IntoIterator<Item = (String, &'a [(String, String)])>,
) -> String {
    let mut hasher = Sha256::new();
    for (name, files) in cases {
        hasher.update(name.as_bytes());
        hasher.update([0]);
        for (path, text) in files {
            hasher.update(path.as_bytes());
            hasher.update([0]);
            hasher.update(text.as_bytes());
            hasher.update([0]);
        }
        hasher.update([1]);
    }
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("a string takes every write");
            hex
        })
}

/// Plants `stub` (a stub file only when its text differs, so that its modification time holds)
/// and then `files` at `root`, a file under `../git/` in the folder `git` beside it, committed
/// when it exists and named by its URL wherever a file says `@GIT@` (and by its folder wherever a
/// file's text says `../git/`); writes the lock file unless the tree holds one, leaving a lock
/// cargo cannot write to the census's `--locked` read to refuse; and answers what the census
/// refuses when it compiles the tree in `target`, each refusal as [`planted_text`] writes it.
fn census_of_tree(
    root: &Path,
    git: &str,
    target: &Path,
    stub: &[(String, String)],
    files: &[(String, String)],
) -> Vec<String> {
    let folder = root.with_file_name(git);
    let url = format!("file://{}", folder.display());
    for (path, text) in stub {
        if fs::read_to_string(root.join(path)).ok().as_deref() != Some(text) {
            plant(root, path, text);
        }
    }
    for (path, text) in files {
        let beside = format!("../{git}/");
        let text = text.replace("@GIT@", &url).replace("../git/", &beside);
        plant(root, &path.replace("../git/", &beside), &text);
    }
    if folder.exists() {
        committed(&folder);
    }
    if !root.join("Cargo.lock").exists() {
        let _unwritten = cargo(root, &["generate-lockfile".to_owned()], CARGO_LIMIT);
    }
    census_in(root, target)
        .refused
        .iter()
        .map(|refusal| planted_text(refusal, root, &folder))
        .collect()
}

/// `refusal` with the planted workspace's path written `@WS@`, and its git folder's path, URL and
/// revision written `@GIT@`, so that one tree's verdict reads alike wherever it is planted: the
/// differential compares verdicts, not the temporary paths a refusal names.
fn planted_text(refusal: &str, root: &Path, folder: &Path) -> String {
    let mut text = refusal
        .replace(&format!("file://{}", folder.display()), "@GIT@")
        .replace(&folder.display().to_string(), "@GIT@")
        .replace(&root.display().to_string(), "@WS@");
    // A git source names its revision after `#`, which the commit's time moves.
    while let Some(at) = text.find("@GIT@#") {
        let start = at + "@GIT@#".len();
        let end = text[start..]
            .find(|character: char| !character.is_ascii_hexdigit())
            .map_or(text.len(), |length| start + length);
        text.replace_range(at + "@GIT@".len()..end, "");
    }
    text
}

/// What the census refuses in `files`, judged in a workspace path and a target nobody has used.
fn census_fresh(stub: &[(String, String)], files: &[(String, String)]) -> Vec<String> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    let root = planted.path().join("ws");
    census_of_tree(
        &root,
        "git",
        &root.join("target").join("settle-census"),
        stub,
        files,
    )
}

/// What the census refuses in each of `trees`, judged in turn at one workspace path and in one
/// target, as a worker that kept its target between trees would judge them: before each tree,
/// everything but the stub and the target is removed, and each tree has a git folder of its own.
fn census_chain(stub: &[(String, String)], trees: &[&[(String, String)]]) -> Vec<Vec<String>> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    let root = planted.path().join("ws");
    let target = root.join("target").join("settle-census");
    trees
        .iter()
        .enumerate()
        .map(|(number, files)| {
            if root.exists() {
                let kept: BTreeSet<PathBuf> =
                    stub.iter().map(|(path, _)| root.join(path)).collect();
                KillerWorker::prune(&root, &root, &kept);
            }
            census_of_tree(&root, &format!("git{number}"), &target, stub, files)
        })
        .collect()
}

/// `judge` of every item of `items`, on up to eight threads, answered in the items' order.
fn in_parallel<T: Sync, R: Send>(items: &[T], judge: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8);
    let judge = &judge;
    let next = &next;
    let mut done: Vec<(usize, R)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(move || {
                    let mut done = Vec::new();
                    loop {
                        let at = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(item) = items.get(at) else {
                            break;
                        };
                        done.push((at, judge(item)));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a judging thread"))
            .collect()
    });
    done.sort_by_key(|(at, _)| *at);
    done.into_iter().map(|(_, result)| result).collect()
}

/// Whether the census could not judge a tree at all: it could not compile it or read its graph.
fn unjudged(refused: &[String]) -> bool {
    refused.iter().any(|refusal| {
        refusal.starts_with("the workspace does not compile")
            || refusal.starts_with("cargo metadata cannot give the graph")
    })
}

/// The refusal each named case of verify round 7's population carries with the owner found by path,
/// by the start of the case's name. A case named here is refused by that refusal; `H05` may go
/// either way; the controls and the disclosed kinds are listed apart.
const R7_NAMED: [(&str, &str); 35] = [
    (
        "H01 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H02 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H03 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H04 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H06 ",
        "deck-streak-m has a build script and can name settle",
    ),
    ("H07 ", "crates/m/Cargo.toml declares a feature"),
    (
        "H08 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H09 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H10 ",
        "deck-streak-x has a build script and can name settle",
    ),
    (
        "H10b ",
        "deck-streak-x has a build script and can name settle",
    ),
    (
        "H11 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H12 ",
        "deck-streak-m has a build script and can name settle",
    ),
    ("H14 ", "crates/m is a proc-macro crate"),
    (
        "H15 ",
        "deck-streak-m has a build script and can name settle",
    ),
    ("H16 ", "cargo metadata cannot give the graph"),
    (
        "H17 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H18 ",
        "deck-streak-m has a build script and can name settle",
    ),
    (
        "H19 ",
        "deck-streak-root has a build script and can name settle",
    ),
    ("H20 ", "g has a build script and can name settle"),
    (
        "H21 ",
        "another package of the graph carries the owner's name",
    ),
    (
        "H22 ",
        "another package of the graph carries the owner's name",
    ),
    (
        "H23 ",
        "another package of the graph carries the owner's name",
    ),
    (
        "H23b ",
        "another package of the graph carries the owner's name",
    ),
    (
        "H23c ",
        "another package of the graph carries the owner's name",
    ),
    ("H26 ", "crates/m/src/lib.rs calls settle"),
    ("H27 ", "is not the one pinned by PROGRESSION_BUILD_SHA256"),
    ("H28 ", "is not the one pinned by PROGRESSION_BUILD_SHA256"),
    ("H29 ", "is not the one pinned by PROGRESSION_BUILD_SHA256"),
    ("H31 ", "deck-streak-progression has no build script"),
    ("H32 ", "is not the one pinned by PROGRESSION_BUILD_SHA256"),
    ("H33 ", "cargo metadata cannot give the graph"),
    ("H34 ", "cargo metadata cannot give the graph"),
    ("H35 ", "cargo metadata cannot give the graph"),
    (
        "H36 ",
        "names its package deck-streak-progress, not deck-streak-progression",
    ),
    (
        "H41 ",
        "deck-streak-m has a build script and can name settle",
    ),
];

/// The controls of verify round 7's population, which the census accepts.
const R7_ACCEPTED: [&str; 5] = ["H13 ", "H25 ", "H30 ", "H37 ", "H38 "];

/// The cases of verify round 7's population that are disclosed kinds (SPEC-072 §12): progression's
/// own code reading the census's cfg (`H24`, `H24b`, `H42`), and a build script's cfg of a package
/// that cannot name `settle` read through its macro (`H39`, `H40`). Each is judged, and its verdict
/// is printed and never asserted.
const R7_DISCLOSED: [&str; 5] = ["H24 ", "H24b ", "H39 ", "H40 ", "H42 "];

/// The count and the digest of verify round 7's population, pinned together.
const R7_POPULATION: (usize, &str) = (
    46,
    "b0fbee86a359b5cc5b3a963a38cd1e1afb629c2d7b18ab00c5242d103c4d28f0",
);

/// The seed of the differential's order over verify round 7's population.
const DIFFERENTIAL_SEED: u64 = 0x0425_0008;

/// A permutation of `0..count` from `seed`, by Fisher and Yates over a 64-bit linear congruential
/// generator (Knuth's MMIX constants), so that the differential's order is stated and repeatable.
fn seeded_order(count: usize, seed: u64) -> Vec<usize> {
    let mut state = seed;
    let mut order: Vec<usize> = (0..count).collect();
    for at in (1..count).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let pick = usize::try_from((state >> 33) % (at as u64 + 1)).expect("an index");
        order.swap(at, pick);
    }
    order
}

/// The macro body of tree `A` of verify round 7's order pair P3, which calls progression's `settle`.
const P3_A: &str = "deck_streak_progression::settle()";

/// The macro body of tree `B` of the pair, which calls the other crate's, padded to `A`'s length.
const P3_B: &str = "deck_streak_other::settle()      ";

/// Verify round 7's order pair P3: a path package's build script writes its macro into `OUT_DIR`
/// only when the file is absent, whose body is `call`: progression's `settle` in tree `A`, and the
/// other crate's, padded to the same length, in tree `B`.
fn p3_tree(call: &str) -> Vec<(String, String)> {
    vec![
        r7f(
            "crates/m/Cargo.toml",
            "[package]\nname = \"deck-streak-m\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n\
             deck-streak-other = { path = \"../other\" }\ndeck-streak-g = { path = \"../g\" }\n",
        ),
        r7f(
            "crates/m/src/lib.rs",
            "pub fn call() -> usize {\n    deck_streak_g::call!()\n}\n",
        ),
        r7f(
            "crates/g/Cargo.toml",
            "[package]\nname = \"deck-streak-g\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        r7f(
            "crates/g/build.rs",
            &format!(
                "fn main() {{\n    let out = std::path::Path::new(&std::env::var(\"OUT_DIR\")\
                 .expect(\"OUT_DIR\")).join(\"mac.rs\");\n    if !out.exists() {{\n        \
                 std::fs::write(&out, \"#[macro_export]\\nmacro_rules! call {{\\n    () => \
                 {{\\n        {call}\\n    }};\\n}}\\n\").expect(\"mac.rs\");\n    }}\n}}\n"
            ),
        ),
        r7f(
            "crates/g/src/lib.rs",
            "include!(concat!(env!(\"OUT_DIR\"), \"/mac.rs\"));\n",
        ),
    ]
}

#[test]
fn verify_round_seven_population_is_judged_as_each_case_expects_on_any_target() {
    // Each of the 46 trees is judged alone in a target nobody has used, and must answer as its case
    // expects: refused by its named refusal, a control accepted, a disclosed kind judged. Then the
    // differential (main's round-7 addendum, condition 2): the same trees in the seeded order
    // DIFFERENTIAL_SEED, dealt into chains that each keep one target, with the P3 pair in both
    // orders as chains of their own; every tree's verdict in a chain must equal its verdict alone.
    let stub = killer_stub();
    let cases = r7_population();
    let digest = population_digest(
        cases
            .iter()
            .map(|(id, expect, files)| (format!("{id}\0{expect}"), files.as_slice())),
    );
    println!(
        "examined {} case(s) of verify round 7's population, digest {digest}",
        cases.len()
    );
    let (a, b) = (p3_tree(P3_A), p3_tree(P3_B));
    let mut trees: Vec<&[(String, String)]> =
        cases.iter().map(|(_, _, files)| files.as_slice()).collect();
    trees.extend([a.as_slice(), b.as_slice()]);
    let fresh = in_parallel(&trees, |files| census_fresh(&stub, files));
    let mut wrong = Vec::new();
    for ((id, expect, _), refused) in cases.iter().zip(&fresh) {
        let named = R7_NAMED
            .iter()
            .find(|(case, _)| id.starts_with(case))
            .map(|(_, named)| *named);
        let listed = |list: &[&str]| list.iter().any(|case| id.starts_with(case));
        let verdict = refused.first().map_or("accepted", String::as_str);
        if let Some(named) = named {
            if !refused.iter().any(|refusal| refusal.contains(named)) {
                wrong.push(format!("{id}: not refused by \"{named}\" | {verdict}"));
            }
        } else if listed(&R7_ACCEPTED) {
            if let Some(first) = refused.first() {
                wrong.push(format!("control {id}: refused | {first}"));
            }
        } else if listed(&R7_DISCLOSED) {
            println!("disclosed {id} ({expect}): {verdict}");
            if unjudged(refused) {
                wrong.push(format!("disclosed {id}: not judged | {verdict}"));
            }
        } else if !id.starts_with("H05 ") {
            wrong.push(format!("{id}: no expectation"));
        }
    }
    let seeded = seeded_order(cases.len(), DIFFERENTIAL_SEED);
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, 8);
    let mut chains: Vec<Vec<usize>> = (0..threads)
        .map(|chain| {
            seeded
                .iter()
                .skip(chain)
                .step_by(threads)
                .copied()
                .collect()
        })
        .collect();
    chains.extend([
        vec![cases.len(), cases.len() + 1],
        vec![cases.len() + 1, cases.len()],
    ]);
    let warm = in_parallel(&chains, |chain| {
        let files: Vec<&[(String, String)]> = chain.iter().map(|at| trees[*at]).collect();
        census_chain(&stub, &files)
    });
    let mut compared = 0;
    let mut disagreements = Vec::new();
    for (chain, verdicts) in chains.iter().zip(&warm) {
        for (step, (at, verdict)) in chain.iter().zip(verdicts).enumerate().skip(1) {
            compared += 1;
            if *verdict != fresh[*at] {
                disagreements.push(format!(
                    "tree {at} after tree {}: {} on a warmed target, {} alone",
                    chain[step - 1],
                    verdict.first().map_or("accepted", String::as_str),
                    fresh[*at].first().map_or("accepted", String::as_str)
                ));
            }
        }
    }
    println!(
        "differential: seed {DIFFERENTIAL_SEED:#x}, {} chain(s), {compared} warmed verdict(s) \
         compared with the fresh one; disagreements: {}",
        chains.len(),
        disagreements.len()
    );
    for line in wrong.iter().chain(&disagreements) {
        println!("wrong: {line}");
    }
    assert_eq!(
        (cases.len(), digest.as_str()),
        R7_POPULATION,
        "verify round 7's population is not the one pinned"
    );
    assert!(
        wrong.is_empty() && disagreements.is_empty(),
        "cases judged wrongly: {}; disagreements: {}",
        wrong.len(),
        disagreements.len()
    );
}
#[test]
fn the_order_pair_p3_is_judged_alike_in_both_orders_on_one_target() {
    // Verify round 7's P3: a build script that writes its macro only when the file is absent, in
    // two trees that differ in that script alone. Judged in either order on one target, each tree
    // must answer as it does alone.
    let stub = killer_stub();
    let (a, b) = (p3_tree(P3_A), p3_tree(P3_B));
    let alone = [census_fresh(&stub, &a), census_fresh(&stub, &b)];
    let orders = [
        census_chain(&stub, &[a.as_slice(), b.as_slice()]),
        census_chain(&stub, &[b.as_slice(), a.as_slice()]),
    ];
    assert_eq!(
        alone,
        [
            vec!["crates/m/src/lib.rs calls settle, and only coordination's code may".to_owned()],
            Vec::new()
        ],
        "the pair's trees alone"
    );
    assert_eq!(
        [&orders[0][1], &orders[1][1]],
        [&alone[1], &alone[0]],
        "the second tree of each order on a target the first tree used"
    );
}

#[test]
fn a_target_copied_from_another_trees_census_does_not_move_the_verdict() {
    // Verify round 7's S1-2b: the census of tree one leaves its target; a copy of it, as a cache
    // restores one, then serves the census of tree two, which differs from tree one in one build
    // script. Tree two must answer on the copy as it does in a target nobody has used.
    let stub = killer_stub();
    let (one, two) = (p3_tree(P3_B), p3_tree(P3_A));
    let planted = tempfile::tempdir().expect("a temporary directory");
    let root = planted.path().join("ws");
    let first = planted.path().join("first");
    census_of_tree(&root, "git", &first, &stub, &one);
    let copied = planted.path().join("copied");
    let status = Command::new("cp")
        .arg("-a")
        .arg(&first)
        .arg(&copied)
        .status()
        .expect("cp runs");
    assert!(status.success(), "cp -a {}", first.display());
    for (path, text) in &two {
        plant(&root, path, text);
    }
    let on_the_copy = census_in(&root, &copied).refused;
    assert_eq!(
        on_the_copy,
        census_fresh(&stub, &two),
        "tree two on a copy of tree one's target, and alone"
    );
    assert_eq!(
        on_the_copy,
        ["crates/m/src/lib.rs calls settle, and only coordination's code may"]
    );
}

/// Judges the case of verify round 7's population whose name starts with `case`, alone.
fn r7_case(case: &str) -> Vec<String> {
    let files = r7_population()
        .into_iter()
        .find(|(id, _, _)| id.starts_with(case))
        .map(|(_, _, files)| files)
        .expect("a case of the population");
    census_fresh(&killer_stub(), &files)
}

/// Whether one refusal of `refused` names `named`.
fn names(refused: &[String], named: &str) -> bool {
    refused.iter().any(|refusal| refusal.contains(named))
}

#[test]
fn a_git_package_carrying_the_owners_name_beside_a_members_script_is_refused_by_name() {
    // H21: a git package at version 0.0.1 carries progression's name, and a member that can name
    // settle has a build script. The owner is found by its manifest, and the decoy is refused.
    let refused = r7_case("H21 ");
    assert!(
        names(
            &refused,
            "another package of the graph carries the owner's name"
        ) && names(
            &refused,
            "deck-streak-m has a build script and can name settle"
        ),
        "{refused:?}"
    );
}

#[test]
fn a_git_package_carrying_the_owners_name_beside_an_edited_pin_is_refused_by_name() {
    // H23: the same decoy beside a one-byte edit of progression's pinned build script.
    let refused = r7_case("H23 ");
    assert!(
        names(
            &refused,
            "another package of the graph carries the owner's name"
        ) && names(
            &refused,
            "is not the one pinned by PROGRESSION_BUILD_SHA256"
        ),
        "{refused:?}"
    );
}

#[test]
fn a_git_package_carrying_the_owners_name_beside_a_disarmed_owner_is_refused_by_name() {
    // H23b: the same decoy beside a progression build script that sets no cfg, and a member that
    // calls settle, which that script would hide.
    let refused = r7_case("H23b ");
    assert!(
        names(
            &refused,
            "another package of the graph carries the owner's name"
        ) && names(
            &refused,
            "is not the one pinned by PROGRESSION_BUILD_SHA256"
        ),
        "{refused:?}"
    );
}

#[test]
fn the_owners_manifest_under_another_package_name_is_refused_by_name() {
    // H36: crates/progression/Cargo.toml names its package otherwise, and a member with a build
    // script depends on it. The owner is found by its manifest, and the name is refused.
    let refused = r7_case("H36 ");
    assert!(
        names(
            &refused,
            "crates/progression/Cargo.toml names its package deck-streak-progress, not \
             deck-streak-progression"
        ) && names(
            &refused,
            "deck-streak-m has a build script and can name settle"
        ),
        "{refused:?}"
    );
}

#[test]
fn an_owner_without_a_build_script_is_refused_by_name() {
    // H31: progression says `build = false`, so no cfg is ever set and no use of settle is
    // reported, while a member calls it.
    let refused = r7_case("H31 ");
    assert!(
        names(&refused, "deck-streak-progression has no build script"),
        "{refused:?}"
    );
}

/// A package of a synthetic graph: its id, name, version, source (`None` for a path), manifest,
/// and its targets, each a kind and a source path.
fn synthetic_package(
    id: &str,
    name: &str,
    version: &str,
    source: Option<&str>,
    manifest: &Path,
    targets: &[(&str, &Path)],
) -> Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "version": version,
        "source": source,
        "manifest_path": manifest.to_string_lossy(),
        "dependencies": [],
        "features": {},
        "targets": targets
            .iter()
            .map(|(kind, path)| serde_json::json!({
                "kind": [kind],
                "name": name,
                "src_path": path.to_string_lossy(),
            }))
            .collect::<Vec<_>>(),
    })
}

/// A synthetic graph of `packages` rooted at `root`, whose members are `members`, each package a
/// node with no edges.
fn synthetic_graph(root: &Path, packages: Vec<Value>, members: &[&str]) -> Value {
    let nodes: Vec<Value> = packages
        .iter()
        .map(|package| serde_json::json!({"id": package["id"], "deps": []}))
        .collect();
    serde_json::json!({
        "packages": Value::Array(packages),
        "workspace_members": members,
        "workspace_root": root.to_string_lossy(),
        "resolve": {"nodes": nodes},
    })
}

#[test]
#[allow(clippy::too_many_lines)]
fn the_owner_is_the_member_at_its_manifest_and_every_other_lookup_is_unique_or_refused() {
    // Synthetic graphs, which cargo would never give, for every arm of the owner lookup: absent,
    // ambiguous, renamed (the synthetic H36), a decoy by each source (path, git and registry,
    // listed before the owner), no build script (H31) and two, and the control.
    let planted = tempfile::tempdir().expect("a temporary directory");
    let root = planted.path();
    plant(
        root,
        "crates/progression/Cargo.toml",
        &killer_package(OWNER),
    );
    plant(root, "crates/progression/build.rs", KILLER_BUILD);
    plant(root, "crates/progression/src/lib.rs", KILLER_LIB);
    let manifest = root.join("crates/progression/Cargo.toml");
    let script = root.join("crates/progression/build.rs");
    let lib = root.join("crates/progression/src/lib.rs");
    let owner = |id: &str, name: &str, targets: &[(&str, &Path)]| {
        synthetic_package(id, name, "0.1.0", None, &manifest, targets)
    };
    let scripted = [("lib", lib.as_path()), ("custom-build", script.as_path())];
    let elsewhere = root.join("vendor/p/Cargo.toml");
    let decoy = |source: Option<&str>| {
        synthetic_package(
            "decoy",
            PROGRESSION_PACKAGE,
            "0.0.1",
            source,
            &elsewhere,
            &[("lib", lib.as_path())],
        )
    };
    let cases: Vec<(&str, Value, Option<&str>)> = vec![
        (
            "the owner's manifest is no member's",
            synthetic_graph(root, vec![owner("p", PROGRESSION_PACKAGE, &scripted)], &[]),
            Some("is the manifest of no workspace member"),
        ),
        (
            "two members have the owner's manifest",
            synthetic_graph(
                root,
                vec![
                    owner("p", PROGRESSION_PACKAGE, &scripted),
                    owner("q", PROGRESSION_PACKAGE, &scripted),
                ],
                &["p", "q"],
            ),
            Some("2 workspace members have the manifest crates/progression/Cargo.toml"),
        ),
        (
            "the owner renamed",
            synthetic_graph(
                root,
                vec![owner("p", "deck-streak-progress", &scripted)],
                &["p"],
            ),
            Some("names its package deck-streak-progress, not deck-streak-progression"),
        ),
        (
            "a path decoy",
            synthetic_graph(
                root,
                vec![decoy(None), owner("p", PROGRESSION_PACKAGE, &scripted)],
                &["p"],
            ),
            Some("carries the owner's name (deck-streak-progression 0.0.1 (a path))"),
        ),
        (
            "a git decoy",
            synthetic_graph(
                root,
                vec![
                    decoy(Some("git+file:///decoy#0123")),
                    owner("p", PROGRESSION_PACKAGE, &scripted),
                ],
                &["p"],
            ),
            Some("carries the owner's name (deck-streak-progression 0.0.1 (git+file:///decoy"),
        ),
        (
            "a registry decoy",
            synthetic_graph(
                root,
                vec![
                    decoy(Some(
                        "registry+https://github.com/rust-lang/crates.io-index",
                    )),
                    owner("p", PROGRESSION_PACKAGE, &scripted),
                ],
                &["p"],
            ),
            Some("carries the owner's name (deck-streak-progression 0.0.1 (registry+"),
        ),
        (
            "no build script",
            synthetic_graph(
                root,
                vec![owner("p", PROGRESSION_PACKAGE, &[("lib", lib.as_path())])],
                &["p"],
            ),
            Some("deck-streak-progression has no build script"),
        ),
        (
            "two build scripts",
            synthetic_graph(
                root,
                vec![owner(
                    "p",
                    PROGRESSION_PACKAGE,
                    &[
                        ("lib", lib.as_path()),
                        ("custom-build", script.as_path()),
                        ("custom-build", script.as_path()),
                    ],
                )],
                &["p"],
            ),
            Some("deck-streak-progression has 2 build scripts"),
        ),
        (
            "the control, beside another member",
            synthetic_graph(
                root,
                vec![
                    owner("p", PROGRESSION_PACKAGE, &scripted),
                    synthetic_package(
                        "o",
                        "deck-streak-other",
                        "0.1.0",
                        None,
                        &root.join("crates/other/Cargo.toml"),
                        &[("lib", lib.as_path())],
                    ),
                ],
                &["p", "o"],
            ),
            None,
        ),
    ];
    let mut wrong = Vec::new();
    for (case, graph, named) in examined("synthetic graph(s)", cases) {
        let answer = build_scripts(root, &graph);
        let refused = match &answer {
            Ok(refused) | Err(refused) => refused,
        };
        let right = match named {
            Some(named) => names(refused, named),
            None => matches!(&answer, Ok(refused) if refused.is_empty()),
        };
        if !right {
            wrong.push(format!("{case}: {answer:?}"));
        }
    }
    assert_eq!(wrong, Vec::<String>::new());
}

/// Main's round-8 generation of an input the tree does not hold: a package that cannot name
/// `settle` sets its cfg `k` unless the variable `R8_KEY` is set, and its macro, which a member
/// expands, calls `settle` under `k`. Alone, the tree is refused for the member's call.
fn variable_tree() -> Vec<(String, String)> {
    vec![
        r7f("crates/g/Cargo.toml", &r7_pkg("deck-streak-g", "", "")),
        r7f(
            "crates/g/build.rs",
            "fn main() {\n    println!(\"cargo::rustc-check-cfg=cfg(k)\");\n    \
             println!(\"cargo::rerun-if-env-changed=R8_KEY\");\n    \
             if std::env::var_os(\"R8_KEY\").is_none() {\n        \
             println!(\"cargo::rustc-cfg=k\");\n    }\n}\n",
        ),
        r7f(
            "crates/g/src/lib.rs",
            "#[cfg(k)]\n#[macro_export]\nmacro_rules! call {\n    () => {\n        \
             deck_streak_progression::settle()\n    };\n}\n#[cfg(not(k))]\n#[macro_export]\n\
             macro_rules! call {\n    () => {\n        0\n    };\n}\n",
        ),
        r7f(
            "crates/m/Cargo.toml",
            &r7_pkg(
                "deck-streak-m",
                "",
                "[dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n\
                 deck-streak-g = { path = \"../g\" }\n",
            ),
        ),
        r7f(
            "crates/m/src/lib.rs",
            "pub fn call() -> usize {\n    deck_streak_g::call!()\n}\n",
        ),
    ]
}

/// The refusal a member's call to `settle` carries.
const M_CALLS_SETTLE: &str = "crates/m/src/lib.rs calls settle, and only coordination's code may";

#[test]
#[ignore = "run by the killers of an input the tree does not hold, in a process of its own"]
fn census_of_the_variable_tree_in_a_process_of_its_own() {
    // The census of `variable_tree`, in a fresh workspace, printed for the process that started
    // this one with the environment it chose.
    if std::env::var_os("R8_CHILD").is_some() {
        let refused = census_fresh(&killer_stub(), &variable_tree());
        println!(
            "R8 REFUSED {}",
            serde_json::to_string(&refused).expect("a list of strings")
        );
    }
}

/// What the census of `variable_tree` refuses in a process of its own, with `set` added to its
/// environment and `unset` removed.
fn census_in_a_child(set: &[(&str, &Path)], unset: &[&str]) -> Vec<String> {
    let mut command = Command::new(std::env::current_exe().expect("this test's program"));
    command.args([
        "--exact",
        "census_of_the_variable_tree_in_a_process_of_its_own",
        "--ignored",
        "--nocapture",
        "--test-threads",
        "1",
    ]);
    command.env("R8_CHILD", "1");
    for (name, value) in set {
        command.env(name, value);
    }
    for name in unset {
        command.env_remove(name);
    }
    let output = command.output().expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find_map(|line| line.split_once("R8 REFUSED ").map(|(_, verdict)| verdict))
        .unwrap_or_else(|| panic!("the child printed no verdict: {stdout}"));
    serde_json::from_str(line).expect("the child's verdict")
}

#[test]
fn a_variable_the_tree_does_not_set_does_not_move_the_verdict() {
    // The same tree, judged with `R8_KEY` set and unset in the census's own environment, must
    // answer alike: a variable the tree does not set never reaches a build script.
    let unset = census_in_a_child(&[], &["R8_KEY"]);
    let set = census_in_a_child(&[("R8_KEY", Path::new("1"))], &[]);
    assert_eq!(set, unset, "the variable tree with R8_KEY set, and unset");
    assert_eq!(unset, [M_CALLS_SETTLE]);
}

#[test]
fn a_cargo_configuration_in_cargos_home_is_refused_by_name() {
    // A cargo home whose configuration sets `R8_KEY` for every build script: the census refuses it
    // by name rather than judge the tree under it.
    let home = tempfile::tempdir().expect("a temporary directory");
    plant(home.path(), "config.toml", "[env]\nR8_KEY = \"1\"\n");
    let refused = census_in_a_child(&[("CARGO_HOME", home.path())], &["R8_KEY"]);
    assert!(
        names(&refused, "configures cargo from its home"),
        "{refused:?}"
    );
}

#[test]
fn a_members_code_reading_a_variable_the_host_sets_is_refused_by_name() {
    // A member's code reads `HOME` at compile time, which the host sets and the tree does not: the
    // census refuses it by name rather than judge a verdict the host's value could move.
    let refused = census_fresh(
        &killer_stub(),
        &[
            r7f("crates/m/Cargo.toml", &r7_pkg("deck-streak-m", "", "")),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn home() -> &'static str {\n    env!(\"HOME\")\n}\n",
            ),
        ],
    );
    assert!(
        names(
            &refused,
            "crates/m's code reads the variable HOME, which the host sets"
        ),
        "{refused:?}"
    );
}

#[test]
fn a_cargo_configuration_above_the_tree_is_refused_by_name() {
    // A configuration in a folder above the tree, which cargo reads, sets `R8_KEY` for every build
    // script: the census refuses it by name rather than judge the tree under it.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        ".cargo/config.toml",
        "[env]\nR8_KEY = \"1\"\n",
    );
    let root = planted.path().join("ws");
    let refused = census_of_tree(
        &root,
        "git",
        &root.join("target").join("settle-census"),
        &killer_stub(),
        &variable_tree(),
    );
    assert!(
        names(
            &refused,
            "configures cargo, and the census compiles with cargo's own defaults"
        ),
        "{refused:?}"
    );
}

#[test]
fn a_git_url_reused_with_other_content_is_judged_as_a_fresh_url() {
    // Main's warm cargo home: two trees fetch one git URL with other content, into one cargo home
    // and one target. The second must answer as it does alone at a URL cargo has never seen.
    let stub = killer_stub();
    let tree = |settle: &str| {
        vec![
            r7f(
                "../git/Cargo.toml",
                "[package]\nname = \"g\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            ),
            r7f(
                "../git/src/lib.rs",
                &format!(
                    "#[macro_export]\nmacro_rules! call {{\n    () => {{\n        {settle}()\n    \
                     }};\n}}\n"
                ),
            ),
            r7f(
                "crates/m/Cargo.toml",
                &r7_pkg(
                    "deck-streak-m",
                    "",
                    "[dependencies]\ndeck-streak-progression = { path = \"../progression\" }\n\
                     deck-streak-other = { path = \"../other\" }\ng = { git = \"@GIT@\" }\n",
                ),
            ),
            r7f(
                "crates/m/src/lib.rs",
                "pub fn call() -> usize {\n    g::call!()\n}\n",
            ),
        ]
    };
    let (one, two) = (
        tree("deck_streak_other::settle"),
        tree("deck_streak_progression::settle"),
    );
    let planted = tempfile::tempdir().expect("a temporary directory");
    let root = planted.path().join("ws");
    let target = root.join("target").join("settle-census");
    census_of_tree(&root, "git", &target, &stub, &one);
    fs::remove_file(root.join("Cargo.lock")).expect("tree one's lock file");
    let second = census_of_tree(&root, "git", &target, &stub, &two);
    assert!(!unjudged(&second) && !second.is_empty(), "{second:?}");
    assert_eq!(
        second,
        census_fresh(&stub, &two),
        "tree two after tree one, and alone"
    );
}

#[test]
fn main_round_seven_generation_is_refused_by_name() {
    // Main's round-7 generation, each tree alone: two members with one name, a decoy as a path
    // dependency, as a git dependency, in dev-dependencies only and on another target, the owner's
    // build script under another name (refused off the pin, accepted on it), the owner excluded
    // from the workspace, and a member excluded. The registry decoy is the synthetic graph's.
    let stub = killer_stub();
    let decoy_git = |section: &str| {
        vec![
            r7f(
                "../git/Cargo.toml",
                "[package]\nname = \"deck-streak-progression\"\nversion = \"0.0.1\"\n\
                 edition = \"2024\"\n",
            ),
            r7f("../git/src/lib.rs", "pub fn noop() {}\n"),
            r7f(
                "crates/d/Cargo.toml",
                &r7_pkg(
                    "deck-streak-d",
                    "",
                    &format!(
                        "{section}\ndecoy = {{ package = \"deck-streak-progression\", git = \
                         \"@GIT@\" }}\n"
                    ),
                ),
            ),
            r7f("crates/d/src/lib.rs", "pub fn d() {}\n"),
        ]
    };
    let excluded = |folder: &str| {
        r7f(
            "Cargo.toml",
            &format!(
                "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/{folder}\"]\n\
                 resolver = \"3\"\n"
            ),
        )
    };
    let cases: Vec<(&str, TreeFiles, Option<&str>)> = vec![
        (
            "two members with one name",
            vec![
                r7f("crates/n/Cargo.toml", &r7_pkg("deck-streak-other", "", "")),
                r7f("crates/n/src/lib.rs", "pub fn n() {}\n"),
            ],
            Some("cargo metadata cannot give the graph"),
        ),
        (
            "two members with the owner's name",
            vec![
                r7f(
                    "crates/n/Cargo.toml",
                    &r7_pkg("deck-streak-progression", "", ""),
                ),
                r7f("crates/n/src/lib.rs", "pub fn n() {}\n"),
            ],
            Some("cargo metadata cannot give the graph"),
        ),
        (
            "a decoy as a path dependency",
            vec![
                r7f(
                    "../git/p/Cargo.toml",
                    "[package]\nname = \"deck-streak-progression\"\nversion = \"0.0.1\"\n\
                     edition = \"2024\"\n",
                ),
                r7f("../git/p/src/lib.rs", "pub fn noop() {}\n"),
                r7f(
                    "crates/d/Cargo.toml",
                    &r7_pkg(
                        "deck-streak-d",
                        "",
                        "[dependencies]\ndecoy = { package = \"deck-streak-progression\", path \
                         = \"../../../git/p\" }\n",
                    ),
                ),
                r7f("crates/d/src/lib.rs", "pub fn d() {}\n"),
            ],
            Some("another package of the graph carries the owner's name"),
        ),
        (
            "a decoy as a git dependency",
            decoy_git("[dependencies]"),
            Some("another package of the graph carries the owner's name"),
        ),
        (
            "a decoy in dev-dependencies only",
            decoy_git("[dev-dependencies]"),
            Some("another package of the graph carries the owner's name"),
        ),
        (
            "a decoy on another target",
            decoy_git("[target.'cfg(windows)'.dependencies]"),
            Some("another package of the graph carries the owner's name"),
        ),
        (
            "the owner's script as custom.rs off the pin",
            vec![
                r7f(
                    "crates/progression/Cargo.toml",
                    &format!("{}build = \"custom.rs\"\n", killer_package(OWNER)),
                ),
                r7f("crates/progression/custom.rs", "fn main() {}\n"),
            ],
            Some("is not the one pinned by PROGRESSION_BUILD_SHA256"),
        ),
        (
            "the owner's script as custom.rs on the pin",
            vec![
                r7f(
                    "crates/progression/Cargo.toml",
                    &format!("{}build = \"custom.rs\"\n", killer_package(OWNER)),
                ),
                r7f("crates/progression/custom.rs", KILLER_BUILD),
            ],
            None,
        ),
        (
            "the owner excluded from the workspace",
            vec![excluded("progression")],
            Some("is the manifest of no workspace member"),
        ),
        (
            "a member excluded from the workspace",
            vec![
                excluded("x"),
                r7f(
                    "crates/x/Cargo.toml",
                    &r7_pkg("deck-streak-x", "", R7_ON_PROGRESSION),
                ),
                r7f("crates/x/build.rs", &r7_script()),
                r7f("crates/x/src/lib.rs", &r7_gated("deck_streak_progression")),
                r7f(
                    "crates/m/Cargo.toml",
                    &r7_pkg(
                        "deck-streak-m",
                        "",
                        "[dependencies]\ndeck-streak-x = { path = \"../x\" }\n",
                    ),
                ),
                r7f("crates/m/src/lib.rs", "pub fn m() {}\n"),
            ],
            Some("deck-streak-x has a build script and can name settle"),
        ),
    ];
    let cases = examined("tree(s) of main's round-7 generation", cases);
    let judged = in_parallel(&cases, |(_, files, _)| census_fresh(&stub, files));
    let wrong: Vec<String> = cases
        .iter()
        .zip(&judged)
        .filter(|((_, _, named), refused)| match named {
            Some(named) => !names(refused, named),
            None => !refused.is_empty(),
        })
        .map(|((case, _, _), refused)| format!("{case}: {refused:?}"))
        .collect();
    assert_eq!(wrong, Vec::<String>::new());
}

/// How many spellings of `xp_settlement` the population plants: its 13 splits in 9 forms each, and
/// the 14 members of the literal family (SPEC-324 A2).
const JOINED_SPELLINGS: usize = 131;

/// The literal family of `xp_settlement`: each member spells the name in a way no line of code
/// shows as written, and rustc, not the census, evaluates it.
fn family() -> Vec<(&'static str, String)> {
    vec![
        spell!("xp\x5fsettlement"),
        spell!("xp\u{5f}settlement"),
        spell!(
            "xp_\
             settlement"
        ),
        spell!(concat!(r##"xp_"##, r#"settlement"#)),
        spell!([
            'x', 'p', '_', 's', 'e', 't', 't', 'l', 'e', 'm', 'e', 'n', 't'
        ]),
        spell!(concat!(concat!("xp", "_"), "settlement")),
        spell!(concat!(stringify!(xp_), stringify!(settlement))),
        spell!("XP_SETTLEMENT"),
        spell!(b"xp\x5fsettlement"),
        spell!(c"xp\x5fsettlement"),
        spell!([
            b'x', b'p', b'\x5f', b's', b'e', b't', b't', b'l', b'e', b'm', b'e', b'n', b't'
        ]),
        spell!(concat!("Xp_", "SeTtLeMeNt")),
        spell!(concat!('x', 'p', "_settlement")),
        spell!("\x78\x70\x5f\x73\x65\x74\x74\x6c\x65\x6d\x65\x6e\x74"),
    ]
}

/// Every spelling of the table's name the population plants: each split in every form, in quests
/// with a piece in streaks, then each member of the family. Plain concatenation and rustc decide
/// that each one is the name.
fn spellings() -> Vec<Spelling> {
    let mut found = Vec::new();
    for (index, split) in population::splits(TABLE).iter().enumerate() {
        assert_eq!(
            split.concat(),
            TABLE,
            "the split {split:?} joins to the name"
        );
        let forms = population::planted(split, index, "quests", "streaks");
        assert_eq!(forms.len(), population::FORMS);
        found.extend(forms);
    }
    for (index, (text, value)) in family().into_iter().enumerate() {
        assert_eq!(
            value.to_ascii_lowercase(),
            TABLE,
            "rustc reads {text} as the name"
        );
        found.push(population::spelled(text, index, "quests"));
    }
    found
}

#[test]
fn a_reserved_name_joined_from_literals_is_refused_in_every_spelling() {
    // One planted workspace holds every spelling in files no module declares, so the census reads
    // them and compiles none, and a copy of each inside progression. Every file outside
    // progression is refused by name, and no copy inside it is.
    let spellings = examined("joined spelling(s)", spellings());
    assert_eq!(spellings.len(), JOINED_SPELLINGS);
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    for member in ["quests", "streaks"] {
        plant_member(
            planted.path(),
            member,
            &[("src/lib.rs", "pub fn quiet() {}\n")],
        );
    }
    let mut expected = Vec::new();
    for spelling in &spellings {
        let owned = spelling.in_crate(OWNER);
        for (path, text) in spelling.files.iter().chain(&owned.files) {
            assert!(
                !text.contains(TABLE),
                "{path} of {} names {TABLE} as written",
                spelling.label
            );
            plant(planted.path(), path, text);
        }
        expected.extend(spelling.files.iter().map(|(path, _)| {
            format!(
                "{path} spells {TABLE} from literals, joined or in another case, and only \
                 {OWNER}'s code may"
            )
        }));
    }
    expected.sort();
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(refused.refused, expected);
}

/// The refusal of progression's own use of `settle` in the file `file`.
fn owners_own(file: &str) -> String {
    format!(
        "{file} calls settle inside {OWNER}'s own code, and only {CALLER}'s code may unless \
         {OWNER} admits it"
    )
}

#[test]
fn the_owners_own_operation_is_refused_unless_it_admits_it() {
    // A wrapper, a function pointer and a generic in progression's library are each a new
    // operation in the owner's code, and none is admitted. Its imports (grouped, renaming the
    // module, inside a function body, after text that is not ASCII), its tests and its examples
    // are accepted.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        &format!(
            "{KILLER_LIB}pub mod generic;\npub mod imports;\npub mod pointer;\npub mod unicode;\n\
             pub mod wrapper;\n"
        ),
    );
    plant(
        planted.path(),
        "crates/progression/src/wrapper.rs",
        "pub fn wrapped() -> usize {\n    crate::settle::settle()\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/pointer.rs",
        "pub const STEP: fn() -> usize = crate::settle::settle;\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/generic.rs",
        "pub fn run<F: Fn() -> usize>(step: F) -> usize {\n    step()\n}\n\
         pub fn go() -> usize {\n    run(crate::settle::settle)\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/imports.rs",
        "pub use crate::settle::{SettledRow, settle as grouped};\n\
         pub use crate::settle::{self as module, settle as by_module};\n\
         pub fn quiet() -> usize {\n    #[allow(unused_imports)]\n    \
         use crate::settle::settle as inner;\n    0\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/unicode.rs",
        &format!(
            "pub const GREETING: &str = \"{}\";\npub use crate::settle::settle as after_unicode;\n",
            "\u{fc}".repeat(40)
        ),
    );
    plant(
        planted.path(),
        "crates/progression/tests/own.rs",
        "#[test]\nfn own() {\n    let _ = deck_streak_progression::settle();\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/examples/demo.rs",
        "fn main() {\n    let _ = deck_streak_progression::settle();\n}\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            owners_own("crates/progression/src/generic.rs"),
            owners_own("crates/progression/src/pointer.rs"),
            owners_own("crates/progression/src/wrapper.rs"),
        ]
    );
}

#[test]
fn an_admitted_file_of_the_owner_is_accepted() {
    // A wrapper progression's library writes is refused while progression admits no file, and
    // accepted once progression admits the file the use is named by.
    let planted = tempfile::tempdir().expect("a temporary directory");
    let file = "crates/progression/src/wrapper.rs";
    let text = "pub fn wrap() -> usize {\n    crate::settle::settle()\n}\n";
    plant(planted.path(), file, text);
    let used = Use {
        folder: format!("crates/{OWNER}"),
        kind: "lib".to_owned(),
        chain: vec![file.to_owned()],
        file: file.to_owned(),
        at: Some((
            file.to_owned(),
            text.rfind("settle()").expect("the call is planted"),
        )),
    };
    assert_eq!(
        owners_own_operation(planted.path(), &used, &[]),
        Some(owners_own(file))
    );
    assert_eq!(owners_own_operation(planted.path(), &used, &[file]), None);
}

#[test]
fn a_reexport_progressions_macro_writes_is_followed_or_refused() {
    // A macro of progression that writes the re-export in its own body is followed to the member
    // that calls the new name; a macro that takes the re-exported path as its argument is
    // reported at that argument, outside any import, and is refused in progression.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        &format!(
            "{KILLER_LIB}macro_rules! reexport_step {{\n    () => {{\n        \
             pub use $crate::settle::settle as step;\n    }};\n}}\nreexport_step!();\n\
             macro_rules! reexport {{\n    ($($path:tt)+) => {{\n        \
             pub use $($path)+ as again;\n    }};\n}}\nreexport!(crate::settle::settle);\n"
        ),
    );
    plant_member(
        planted.path(),
        "streaks",
        &[
            ("src/lib.rs", "pub mod caller;\n"),
            (
                "src/caller.rs",
                "pub fn go() -> usize {\n    deck_streak_progression::step()\n}\n",
            ),
        ],
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            owners_own("crates/progression/src/lib.rs"),
            "crates/streaks/src/caller.rs calls settle, and only coordination's code may"
                .to_owned(),
        ]
    );
}

/// The literal half of the census of the tree at `root`, which compiles nothing: every file outside
/// progression that names the table as written, and the shared reader's refusals. SPEC-331's
/// populations of many small trees are judged by it; the compiled half is judged by the tests
/// above.
fn literal_census(root: &Path) -> Vec<String> {
    let mut naming = BTreeSet::new();
    let mut refused = Vec::new();
    for member in members(root) {
        let outside = member.file_name().is_some_and(|name| name != OWNER);
        for path in files(&member.join("src"), "rs") {
            let name = relative(root, &path);
            if names_the_table(&fs::read_to_string(&path).expect("a readable source")) {
                if outside {
                    refused.push(format!("{name} names {TABLE}, and only {OWNER}'s code may"));
                }
                naming.insert(name);
            }
        }
    }
    refused.extend(table_census::refusals(root, TABLE, OWNER, &naming));
    refused
}

/// One of SPEC-331's plants: its label and its files, none of which names the table as written.
type Plant = (&'static str, Vec<(&'static str, &'static str)>);

/// What the literal census refuses in a tree that holds the plant `files` alone.
fn literal_census_of(label: &str, files: &[(&str, &str)]) -> Vec<String> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    for (path, text) in files {
        assert!(
            !text.contains(TABLE),
            "{path} of {label} names {TABLE} as written"
        );
        plant(planted.path(), path, text);
    }
    literal_census(planted.path())
}

/// The refusal of the file `path` for spelling the table from its literals.
fn spells_the_table(path: &str) -> String {
    format!(
        "{path} spells {TABLE} from literals, joined or in another case, and only {OWNER}'s code may"
    )
}

/// SPEC-331 A2's joins (the design's plants P1 to P13, B4 to B7 and B9, and P14): each is a real
/// join of the name from pieces, and each is refused by exactly the files holding its pieces. P14
/// joins two files that reach nothing of each other in another case, so only the pool's case
/// folding refuses it.
#[allow(clippy::too_many_lines)]
fn join_plants() -> Vec<Plant> {
    vec![
        (
            "P1 concat! in one file",
            vec![(
                "crates/quests/src/p1.rs",
                "pub const JOINED: &str = concat!(\"xp_\", \"settlement\");\n",
            )],
        ),
        (
            "P2 const in X joined with a piece in Y",
            vec![
                (
                    "crates/quests/src/p2_head.rs",
                    "pub const HEAD: &str = \"xp_sett\";\n",
                ),
                (
                    "crates/quests/src/p2_join.rs",
                    "pub fn joined() -> String {\n    [super::p2_head::HEAD, \"lement\"].concat()\n}\n",
                ),
            ],
        ),
        (
            "P3 case variant",
            vec![(
                "crates/quests/src/p3.rs",
                "pub const NAME: &str = \"XP_Settlement\";\n",
            )],
        ),
        (
            "P4 stringify! split",
            vec![(
                "crates/quests/src/p4.rs",
                "pub const JOINED: &str = concat!(stringify!(xp_), stringify!(settlement));\n",
            )],
        ),
        (
            "P5 char-by-char array in one file",
            vec![(
                "crates/quests/src/p5.rs",
                "pub fn joined() -> String {\n    ['x', 'p', '_', 's', 'e', 't', 't', 'l', 'e', 'm', 'e', 'n', 't'].iter().collect()\n}\n",
            )],
        ),
        (
            "P6 char-by-char push sequence in one file",
            vec![(
                "crates/quests/src/p6.rs",
                "pub fn joined() -> String {\n    let mut s = String::new();\n    s.push('x');\n    s.push('p');\n    s.push('_');\n    s.push('s');\n    s.push('e');\n    s.push('t');\n    s.push('t');\n    s.push('l');\n    s.push('e');\n    s.push('m');\n    s.push('e');\n    s.push('n');\n    s.push('t');\n    s\n}\n",
            )],
        ),
        (
            "P7 one-char middle joined in format! with consts of two other files",
            vec![
                (
                    "crates/quests/src/p7_head.rs",
                    "pub const HEAD: &str = \"xp_settle\";\n",
                ),
                (
                    "crates/streaks/src/p7_tail.rs",
                    "pub const TAIL: &str = \"ent\";\n",
                ),
                (
                    "crates/quests/src/p7_join.rs",
                    "pub fn joined() -> String {\n    format!(\"{}{}{}\", super::p7_head::HEAD, 'm', deck_streak_streaks::p7_tail::TAIL)\n}\n",
                ),
            ],
        ),
        (
            "P8 one-char const in another crate, joined by format!",
            vec![
                ("crates/streaks/src/p8_m.rs", "pub const M: char = 'm';\n"),
                (
                    "crates/quests/src/p8_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", deck_streak_streaks::p8_m::M)\n}\n",
                ),
            ],
        ),
        (
            "P9 one-char middle pushed between consts of two other files",
            vec![
                (
                    "crates/quests/src/p9_head.rs",
                    "pub const HEAD: &str = \"xp_settle\";\n",
                ),
                (
                    "crates/quests/src/p9_tail.rs",
                    "pub const TAIL: &str = \"ent\";\n",
                ),
                (
                    "crates/quests/src/p9_join.rs",
                    "pub fn joined() -> String {\n    let mut s = String::new();\n    s.push_str(super::p9_head::HEAD);\n    s.push('m');\n    s.push_str(super::p9_tail::TAIL);\n    s\n}\n",
                ),
            ],
        ),
        (
            "P10 one-char const imported and captured inline",
            vec![
                (
                    "crates/quests/src/p10_m.rs",
                    "pub const MIDDLE: &str = \"m\";\n",
                ),
                (
                    "crates/quests/src/p10_join.rs",
                    "use super::p10_m::MIDDLE;\npub fn joined() -> String {\n    format!(\"xp_settle{MIDDLE}ent\")\n}\n",
                ),
            ],
        ),
        (
            "P13 one-char const glob-imported and captured inline",
            vec![
                (
                    "crates/quests/src/p13_m.rs",
                    "pub const MIDDLE: &str = \"m\";\n",
                ),
                (
                    "crates/quests/src/p13_join.rs",
                    "use super::p13_m::*;\npub fn joined() -> String {\n    format!(\"xp_settle{MIDDLE}ent\")\n}\n",
                ),
            ],
        ),
        (
            "P11 one-char const renamed on import",
            vec![
                (
                    "crates/quests/src/p11_m.rs",
                    "pub const MIDDLE: &str = \"m\";\n",
                ),
                (
                    "crates/quests/src/p11_join.rs",
                    "use super::p11_m::MIDDLE as X;\npub fn joined() -> String {\n    [\"xp_settle\", X, \"ent\"].concat()\n}\n",
                ),
            ],
        ),
        (
            "P12 char array const in X collected and joined in Y",
            vec![
                (
                    "crates/quests/src/p12_head.rs",
                    "pub const HEAD: [char; 2] = ['x', 'p'];\n",
                ),
                (
                    "crates/quests/src/p12_join.rs",
                    "pub fn joined() -> String {\n    super::p12_head::HEAD.iter().collect::<String>() + \"_settlement\"\n}\n",
                ),
            ],
        ),
        (
            "B4 real join: a multi-char piece returned by a fn in another crate",
            vec![
                (
                    "crates/quests/src/b4_prefix.rs",
                    "pub fn prefix() -> &'static str {\n    \"xp_\"\n}\n",
                ),
                (
                    "crates/streaks/src/b4_join.rs",
                    "pub fn joined() -> String {\n    format!(\"{}settlement\", deck_streak_quests::b4_prefix::prefix())\n}\n",
                ),
            ],
        ),
        (
            "B5 real join: a multi-char argument passed to a fn in another crate",
            vec![
                (
                    "crates/quests/src/b5_join.rs",
                    "pub fn joined(rest: &str) -> String {\n    format!(\"xp_{rest}\")\n}\n",
                ),
                (
                    "crates/streaks/src/b5_call.rs",
                    "pub fn call() -> String {\n    deck_streak_quests::b5_join::joined(\"settlement\")\n}\n",
                ),
            ],
        ),
        (
            "B6 real join: a multi-char piece in a struct field read in another file",
            vec![
                (
                    "crates/quests/src/b6_table.rs",
                    "pub struct Table {\n    pub head: &'static str,\n}\npub static TABLE: Table = Table { head: \"xp_sett\" };\n",
                ),
                (
                    "crates/quests/src/b6_join.rs",
                    "pub fn joined() -> String {\n    format!(\"{}lement\", super::b6_table::TABLE.head)\n}\n",
                ),
            ],
        ),
        (
            "B9 a one-char const whose name a macro_rules! takes as an argument",
            vec![
                (
                    "crates/quests/src/b9_m.rs",
                    "macro_rules! mark {\n    ($name:ident) => {\n        pub const $name: char = 'm';\n    };\n}\nmark!(MIDDLE);\n",
                ),
                (
                    "crates/quests/src/b9_join.rs",
                    "use super::b9_m::MIDDLE;\npub fn joined() -> String {\n    format!(\"xp_settle{}ent\", MIDDLE)\n}\n",
                ),
            ],
        ),
        (
            "B7 real join: a one-char piece in a struct field read in another file",
            vec![
                (
                    "crates/quests/src/b7_table.rs",
                    "pub struct Table {\n    pub mid: char,\n}\npub static TABLE: Table = Table { mid: 'm' };\n",
                ),
                (
                    "crates/quests/src/b7_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", super::b7_table::TABLE.mid)\n}\n",
                ),
            ],
        ),
        (
            "P14 case variant across crates: a multi-char piece returned by a fn in another crate",
            vec![
                (
                    "crates/quests/src/p14_prefix.rs",
                    "pub fn prefix() -> &'static str {\n    \"XP_\"\n}\n",
                ),
                (
                    "crates/streaks/src/p14_join.rs",
                    "pub fn joined() -> String {\n    format!(\"{}Settlement\", deck_streak_quests::p14_prefix::prefix())\n}\n",
                ),
            ],
        ),
        (
            "O4 control: a one-char associated const in an impl, named by path",
            vec![
                (
                    "crates/streaks/src/o4_t.rs",
                    "pub struct T;\nimpl T {\n    pub const M: char = 'm';\n}\n",
                ),
                (
                    "crates/quests/src/o4_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", deck_streak_streaks::o4_t::T::M)\n}\n",
                ),
            ],
        ),
        (
            "O5 control: a one-char const in an inline module",
            vec![
                (
                    "crates/streaks/src/o5_m.rs",
                    "pub mod inner {\n    pub const M: &str = \"m\";\n}\n",
                ),
                (
                    "crates/quests/src/o5_join.rs",
                    "pub fn joined() -> String {\n    [\"xp_settle\", deck_streak_streaks::o5_m::inner::M, \"ent\"].concat()\n}\n",
                ),
            ],
        ),
        (
            "O6 control: a one-char const with a block initialiser",
            vec![
                (
                    "crates/streaks/src/o6_m.rs",
                    "pub const M: &str = {\n    let x = \"m\";\n    x\n};\n",
                ),
                (
                    "crates/quests/src/o6_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", deck_streak_streaks::o6_m::M)\n}\n",
                ),
            ],
        ),
    ]
}

/// SPEC-331 A3's disclosed class (the design's plants B1, B2 and B8, then O1, O2, O3 and O7): one character carried to its
/// join only through a function's return value or argument, alone or beside a multi-character piece
/// carried the same way, and the second-step class: a `const` naming another `const` across files, a
/// renamed re-export, or an `include!` or `include_str!` inside a named item's initialiser (O1, O2,
/// O3 and O7). The reader follows includes and the names of `const` and `static` items one step,
/// never calls, so none is refused (#585).
#[allow(clippy::too_many_lines)]
fn disclosed_plants() -> Vec<Plant> {
    vec![
        (
            "B1 boundary: a one-char value returned by a fn in another crate",
            vec![
                (
                    "crates/streaks/src/b1_m.rs",
                    "pub fn middle() -> char {\n    'm'\n}\n",
                ),
                (
                    "crates/quests/src/b1_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", deck_streak_streaks::b1_m::middle())\n}\n",
                ),
            ],
        ),
        (
            "B2 boundary: a one-char argument passed to a fn in another crate",
            vec![
                (
                    "crates/quests/src/b2_join.rs",
                    "pub fn joined(c: char) -> String {\n    format!(\"xp_settle{c}ent\")\n}\n",
                ),
                (
                    "crates/streaks/src/b2_call.rs",
                    "pub fn call() -> String {\n    deck_streak_quests::b2_join::joined('m')\n}\n",
                ),
            ],
        ),
        (
            "B8 mixed: a one-char const and a multi-char fn return joined in a third file",
            vec![
                (
                    "crates/quests/src/b8_mid.rs",
                    "pub const MID: char = 'm';\n",
                ),
                (
                    "crates/streaks/src/b8_tail.rs",
                    "pub fn tail() -> &'static str {\n    \"ent\"\n}\n",
                ),
                (
                    "crates/quests/src/b8_join.rs",
                    "use super::b8_mid::MID;\npub fn joined() -> String {\n    format!(\"xp_settle{MID}{}\", deck_streak_streaks::b8_tail::tail())\n}\n",
                ),
            ],
        ),
        (
            "O1 second step: a one-char const initialised by include_str!, named in another file",
            vec![
                (
                    "crates/quests/src/o1_m.rs",
                    "pub const M: &str = include_str!(\"o1_m.txt\");\n",
                ),
                ("crates/quests/src/o1_m.txt", "m"),
                (
                    "crates/quests/src/o1_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", super::o1_m::M)\n}\n",
                ),
            ],
        ),
        (
            "O2 second step: a const alias chain, a const naming a one-char const of another crate",
            vec![
                ("crates/streaks/src/o2_m.rs", "pub const N: char = 'm';\n"),
                (
                    "crates/quests/src/o2_alias.rs",
                    "pub const M: char = deck_streak_streaks::o2_m::N;\n",
                ),
                (
                    "crates/quests/src/o2_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", super::o2_alias::M)\n}\n",
                ),
            ],
        ),
        (
            "O3 second step: a one-char const re-exported under another name in a third file",
            vec![
                (
                    "crates/quests/src/o3_m.rs",
                    "pub const MIDDLE: &str = \"m\";\n",
                ),
                (
                    "crates/quests/src/o3_reexport.rs",
                    "pub use super::o3_m::MIDDLE as X;\n",
                ),
                (
                    "crates/quests/src/o3_join.rs",
                    "use super::o3_reexport::X;\npub fn joined() -> String {\n    [\"xp_settle\", X, \"ent\"].concat()\n}\n",
                ),
            ],
        ),
        (
            "O7 second step: a one-char const initialised by include! of a Rust expression",
            vec![
                (
                    "crates/quests/src/o7_m.rs",
                    "pub const M: char = include!(\"o7_m.in\");\n",
                ),
                ("crates/quests/src/o7_m.in", "'m'\n"),
                (
                    "crates/quests/src/o7_join.rs",
                    "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", super::o7_m::M)\n}\n",
                ),
            ],
        ),
    ]
}

/// SPEC-331 AD2's plant: a join whose own file holds no literal piece, reaching three one-piece
/// files only through the items it names. Each of the three holds a piece, so each is refused.
fn the_no_own_piece_join() -> Vec<Plant> {
    vec![(
        "AD2 no-own-piece join: a joining file that holds no literal reaches three pieces only through the items it names",
        vec![
            (
                "crates/quests/src/ad2_a.rs",
                "pub const A: &str = \"xp_settle\";\n",
            ),
            ("crates/quests/src/ad2_b.rs", "pub const B: char = 'm';\n"),
            (
                "crates/quests/src/ad2_c.rs",
                "pub const C: &str = \"ent\";\n",
            ),
            (
                "crates/quests/src/ad2_join.rs",
                "use super::ad2_a::A;\nuse super::ad2_b::B;\nuse super::ad2_c::C;\npub fn j() -> String {\n    [A, &B.to_string(), C].concat()\n}\n",
            ),
        ],
    )]
}

/// The number of SPEC-331 A2's join plants: P1 to P14, B4 to B7 and B9, and the controls O4 to O6.
const JOIN_PLANTS: usize = 22;

/// The number of SPEC-331 A3's disclosed plants: B1, B2 and B8, and the second-step routes O1, O2,
/// O3 and O7.
const DISCLOSED_PLANTS: usize = 7;

#[test]
fn the_joins_the_census_names_stay_refused() {
    // SPEC-331 A2: every join the census names stays refused, each by exactly the files holding
    // its pieces, as dev's reader refused it: a multi-character piece joined anywhere, and a
    // one-character piece joined in its own file, through an include, or through a `const` or
    // `static` item named by a word or a format placeholder.
    let plants = examined("join plant(s)", join_plants());
    assert_eq!(plants.len(), JOIN_PLANTS);
    for (label, files) in &plants {
        let mut expected: Vec<String> = files
            .iter()
            .map(|(path, _)| spells_the_table(path))
            .collect();
        expected.sort();
        assert_eq!(literal_census_of(label, files), expected, "{label}");
    }
}

#[test]
fn a_join_in_a_file_holding_no_literal_stays_refused() {
    // SPEC-331 R2: every file read has a reach, also one that holds no piece of its own. The
    // joining file here holds none, so only its reach joins the three pieces, and each file
    // holding one is refused. The joining file holds no piece and is not itself refused.
    let plants = examined("no-own-piece join plant(s)", the_no_own_piece_join());
    for (label, files) in &plants {
        let mut expected: Vec<String> = files
            .iter()
            .filter(|(path, _)| !path.ends_with("_join.rs"))
            .map(|(path, _)| spells_the_table(path))
            .collect();
        expected.sort();
        assert_eq!(expected.len(), 3, "{label}");
        assert_eq!(literal_census_of(label, files), expected, "{label}");
    }
}

#[test]
fn a_character_carried_by_a_function_is_disclosed() {
    // SPEC-331 A3: the disclosed class is not refused, and A3 pins it, so widening or narrowing the
    // class moves this test. The positive control writes the carried character in the join's own
    // file, which is refused by that file, so a census that reads nothing fails here too.
    let plants = examined("disclosed plant(s)", disclosed_plants());
    assert_eq!(plants.len(), DISCLOSED_PLANTS);
    let refused: Vec<(&str, Vec<String>)> = plants
        .iter()
        .map(|(label, files)| (*label, literal_census_of(label, files)))
        .filter(|(_, found)| !found.is_empty())
        .collect();
    let files: usize = refused.iter().map(|(_, found)| found.len()).sum();
    assert!(
        refused.is_empty(),
        "{files} file(s) refused in {} of {} disclosed plant(s): {refused:?}",
        refused.len(),
        plants.len()
    );
    let control = [(
        "crates/quests/src/a3_control.rs",
        "pub fn joined() -> String {\n    format!(\"xp_settle{}ent\", 'm')\n}\n",
    )];
    assert_eq!(
        literal_census_of("A3's control", &control),
        vec![spells_the_table("crates/quests/src/a3_control.rs")]
    );
}

/// The number of trees in SPEC-331 A1's population for `xp_settlement`: its 12 letters in both
/// cases and its `_` once, in five shapes each.
const LONE_CHARACTER_TREES: usize = 125;

/// One of SPEC-331 A1's lone shapes: its label, and the file it writes around a character.
type LoneShape = (&'static str, fn(char) -> String);

/// SPEC-331 A1's population for the census's own name. At each position of the name, the piece
/// before it and the piece after it are lone literals in two files, and its character is a lone
/// literal in a third, in each of five lone shapes: a method argument (#600's `strip_prefix` line
/// among them), a `matches!` arm, a format template, a named `const` used alone, and a split set.
/// A letter is planted in both cases. Each tree is a label and its planted files.
fn lone_character_trees() -> Vec<(String, Vec<(String, String)>)> {
    let shapes: [LoneShape; 5] = [
        ("method argument", |c| {
            format!("pub fn c(date: &str) -> Option<&str> {{\n    date.strip_prefix({c:?})\n}}\n")
        }),
        ("matches! arm", |c| {
            format!("pub fn c(x: char) -> bool {{\n    matches!(x, {c:?})\n}}\n")
        }),
        ("format template", |c| {
            format!("pub fn c(n: u32) -> String {{\n    format!(\"{c}{{n}}\")\n}}\n")
        }),
        ("named const used alone", |c| {
            format!(
                "const MARK: char = {c:?};\npub fn c(t: &str) -> Option<&str> {{\n    \
                 t.strip_prefix(MARK)\n}}\n"
            )
        }),
        ("split set", |c| {
            format!("pub fn c(t: &str) -> usize {{\n    t.split([{c:?}, '.']).count()\n}}\n")
        }),
    ];
    let mut trees = Vec::new();
    for (at, character) in TABLE.char_indices() {
        let before = &TABLE[..at];
        let after = &TABLE[at + 1..];
        let mut cases = vec![character];
        if character.is_ascii_alphabetic() {
            cases.push(character.to_ascii_uppercase());
        }
        for case in cases {
            for (shape, text) in &shapes {
                let mut files = Vec::new();
                if !before.is_empty() {
                    files.push((
                        "crates/quests/src/a1_before.rs".to_owned(),
                        format!(
                            "pub fn a(t: &str) -> bool {{\n    t.starts_with({before:?})\n}}\n"
                        ),
                    ));
                }
                if !after.is_empty() {
                    files.push((
                        "crates/quests/src/a1_after.rs".to_owned(),
                        format!("pub fn b(t: &str) -> bool {{\n    t.ends_with({after:?})\n}}\n"),
                    ));
                }
                files.push(("crates/streaks/src/a1_char.rs".to_owned(), text(case)));
                trees.push((format!("{TABLE}@{at} {case:?} {shape}"), files));
            }
        }
    }
    trees
}

#[test]
fn a_lone_character_unjoined_in_its_file_completes_no_path() {
    // SPEC-331 A1: a lone literal of one character, in a file that joins it with nothing, completes
    // no path, whatever the other files hold, so no tree of the population is refused. The positive
    // control joins pieces of the name in one file and is refused by that file, so a census that
    // reads nothing fails here too.
    let trees = examined("lone-character tree(s)", lone_character_trees());
    assert_eq!(trees.len(), LONE_CHARACTER_TREES);
    let mut refused = Vec::new();
    for (label, files) in &trees {
        let planted = tempfile::tempdir().expect("a temporary directory");
        for (path, text) in files {
            assert!(
                !text.contains(TABLE),
                "{path} of {label} names {TABLE} as written"
            );
            plant(planted.path(), path, text);
        }
        let found = literal_census(planted.path());
        if !found.is_empty() {
            refused.push((label.clone(), found));
        }
    }
    let files: usize = refused.iter().map(|(_, found)| found.len()).sum();
    assert!(
        refused.is_empty(),
        "{files} file(s) refused in {} of {} tree(s); the first: {:?}",
        refused.len(),
        trees.len(),
        refused.first()
    );
    let middle = TABLE.len() / 2;
    let joined = format!(
        "pub fn joined() -> String {{\n    format!(\"{{}}{{}}{{}}\", {:?}, {:?}, {:?})\n}}\n",
        &TABLE[..middle],
        TABLE[middle..]
            .chars()
            .next()
            .expect("the middle character")
            .to_ascii_uppercase(),
        &TABLE[middle + 1..]
    );
    assert!(
        !joined.contains(TABLE),
        "the control names {TABLE} as written"
    );
    let control = tempfile::tempdir().expect("a temporary directory");
    plant(control.path(), "crates/quests/src/a1_joined.rs", &joined);
    assert_eq!(
        literal_census(control.path()),
        vec![format!(
            "crates/quests/src/a1_joined.rs spells {TABLE} from literals, joined or in another \
             case, and only {OWNER}'s code may"
        )]
    );
}
