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

/// Runs the cargo that runs this test with `arguments` in `root`, with the census's flags and its
/// arming, and answers whether it succeeded, its standard output and its standard error. The
/// variables that would configure the build apart from the census are removed, so the build is
/// cargo's own with the census's flags. The run is bounded: past `limit` (`CARGO_LIMIT`, which the
/// census passes) the child is stopped and the census fails by name.
fn cargo(
    root: &Path,
    arguments: &[String],
    limit: Duration,
) -> Result<(bool, String, String), String> {
    let program = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(program);
    for (name, _) in std::env::vars_os() {
        if scrubbed(&name.to_string_lossy()) {
            command.env_remove(&name);
        }
    }
    let mut child = command
        .args(arguments)
        .current_dir(root)
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

/// One use of `settle` rustc reported: the folder of the package cargo compiled it in, the kind of
/// the target, the repository's files it is written in (innermost first), and the file it is
/// named by: the outermost of those, or the target's root when the code is not the repository's.
struct Use {
    folder: String,
    kind: String,
    chain: Vec<String>,
    file: String,
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

/// Every use of `settle` in the code the workspace at `root` compiles, or the reasons the census
/// cannot see them all. `target` is the census's own target directory, apart from every other
/// build.
#[allow(clippy::too_many_lines)]
fn compiled_uses(root: &Path, target: &Path) -> Result<Vec<Use>, Vec<String>> {
    let locked = root.join("Cargo.lock").exists();
    let mut arguments: Vec<String> = ["metadata", "--format-version", "1"]
        .map(str::to_owned)
        .to_vec();
    if locked {
        arguments.push("--locked".to_owned());
    }
    let (ok, stdout, stderr) =
        cargo(root, &arguments, CARGO_LIMIT).map_err(|reason| vec![reason])?;
    if !ok {
        return Err(vec![format!(
            "cargo cannot read the workspace ({}), so the census cannot see its callers",
            first_error(&stderr)
        )]);
    }
    let metadata: Value = serde_json::from_str(&stdout)
        .map_err(|error| vec![format!("cargo metadata is not JSON: {error}")])?;
    let workspace = PathBuf::from(metadata["workspace_root"].as_str().unwrap_or_default());
    let member_ids: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut refused = Vec::new();
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
    for config in [".cargo/config", ".cargo/config.toml"] {
        if root.join(config).exists() {
            refused.push(format!(
                "{config} configures cargo, and the census compiles with cargo's own defaults"
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
            if locked {
                arguments.push("--locked".to_owned());
            }
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
                    kind: message["target"]["kind"][0]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
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
    Ok(uses)
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
    census_in(root, &root.join("target").join("settle-census"))
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
    match compiled_uses(root, target) {
        Err(reasons) => census.refused.extend(reasons),
        Ok(uses) => {
            for used in uses {
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

#[test]
fn the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates() {
    // Progression re-exports `settle` under other names, one through another alias, one inside a
    // nested module and one through the renamed module. A caller outside coordination that names
    // only the new names never spells `settle`, and the compiler reports it all the same.
    // Progression's own use of the names, coordination's callers by the same rules as before, a
    // mention in a comment and a crate's own function of an alias's name are not refused.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_workspace(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\nmod inner;\n\
         pub use settle::{\n    SettleCause,\n    SettledRow,\n    settle as tally,\n    SettleRequest as TallyRequest,\n    settled_of_day,\n};\n\
         pub use settle as ledger_write;\n\
         pub use self::tally as tally_again;\n\
         pub mod api {\n    pub use super::settle::settle as run_it;\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/inner.rs",
        "use crate::settle::settle as tally;\npub fn own() -> usize { tally() }\n",
    );
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

#[test]
#[allow(clippy::too_many_lines)]
fn the_census_refuses_what_the_compiler_is_not_asked() {
    // Each tree is a valid workspace at `ws` holding one thing the census does not compile or
    // cannot see through: a member's feature, a cargo configuration in either spelling cargo reads,
    // a package in the repository outside the workspace, a proc-macro member, a path package
    // outside the repository, a git package that depends on progression, a lock file cargo would
    // have to change, code that does not compile, and coordination's use of `settle` in a file
    // outside the repository, which no file of the repository places. Each is refused by name, and
    // alone.
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
    let trees: [(Vec<(&str, &str)>, &str); 10] = [
        (
            vec![("ws/crates/habits/Cargo.toml", &features)],
            "crates/habits/Cargo.toml declares a feature, and the census compiles none",
        ),
        (
            vec![("ws/.cargo/config.toml", "[build]\nincremental = false\n")],
            ".cargo/config.toml configures cargo, and the census compiles with cargo's own defaults",
        ),
        (
            vec![("ws/.cargo/config", "[build]\nincremental = false\n")],
            ".cargo/config configures cargo, and the census compiles with cargo's own defaults",
        ),
        (
            vec![(
                "ws/tools/aside/Cargo.toml",
                "[package]\nname = \"aside\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
            )],
            "tools/aside/Cargo.toml is a package outside the workspace, which the census does not \
             compile",
        ),
        (
            vec![("ws/crates/habits/Cargo.toml", &proc_macro)],
            "crates/habits is a proc-macro crate, and rustc reports no deprecation inside a derive's \
             expansion",
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
            "aside is a package outside the workspace, which the census does not compile",
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
            "depends on progression from outside the workspace, whose callers the census cannot \
             name",
        ),
        (
            vec![("ws/Cargo.lock", "version = 4\n")],
            "because --locked was passed to prevent this), so the census cannot see its callers",
        ),
        (
            vec![(
                "ws/crates/habits/src/lib.rs",
                "pub fn broken() -> usize {\n    \"not a number\"\n}\n",
            )],
            "the workspace does not compile (error: could not compile `deck-streak-habits`",
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
            "crates/coordination/src/lib.rs calls settle outside the recompute steps, and only the \
             owner's correction may",
        ),
    ];
    for (files, line) in examined("tree(s) the census does not compile", Vec::from(trees)) {
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
            refused.len() == 1 && refused.iter().all(|refusal| refusal.contains(line)),
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
    // cause; a correction whose cause is only a string, one that passes both causes, and one
    // coordination's build script writes and a recompute step includes, which no file of the
    // repository holds, so its package's root names it; a coordination file compiled by a `#[path]`
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
                "build.rs",
                "fn main() {\n    let out = std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\");\n    \
                 let step = \"pub fn made() -> usize {\\n    \
                 let _ = deck_streak_progression::settle::SettleCause::OwnersCorrection;\\n    \
                 deck_streak_progression::settle()\\n}\\n\";\n    \
                 std::fs::write(std::path::Path::new(&out).join(\"step.rs\"), step).expect(\"step.rs\");\n}\n",
            ),
            (
                "src/recompute/mod.rs",
                "macro_rules! step {\n    () => {\n        deck_streak_progression::settle()\n    };\n}\n\
                 pub fn fold() -> usize { step!() }\n\
                 include!(concat!(env!(\"OUT_DIR\"), \"/step.rs\"));\n",
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
            "crates/coordination/src/lib.rs calls settle outside the recompute steps, and only \
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
                "crates/coordination/src/lib.rs",
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
/// and "Cargo Targets", TOML 1.0 and the Rust Reference (read 2026-09-30). rustc 1.97.0 compiled
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
            add("S2 B build scripts", label.to_owned(), target, false, files);
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

/// What the census refuses in a tree of its own holding `stub` and then `files`, planted at
/// `<temporary>/ws`. A file under `../git/` is planted beside the workspace and committed as a git
/// repository, whose URL replaces `@GIT@` in every file.
fn killer_judge(stub: &[(String, String)], files: &[(String, String)]) -> Vec<String> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    let root = planted.path().join("ws");
    let git = planted.path().join("git");
    let url = format!("file://{}", git.display());
    for (path, text) in stub.iter().chain(files) {
        plant(&root, path, &text.replace("@GIT@", &url));
    }
    if git.exists() {
        committed(&git);
    }
    census(&root).refused
}

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
                    let mut wrong = Vec::new();
                    while let Some(case) =
                        cases.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                    {
                        let refused = killer_judge(&stub, &case.files);
                        // Every case compiles, so a census that could not compile one has judged
                        // nothing there, and that is wrong whatever the case expects.
                        let unjudged = refused.iter().any(|refusal| {
                            refusal.starts_with("the workspace does not compile")
                                || refusal.starts_with("cargo cannot read the workspace")
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
    assert!(
        cases.len() >= 2218,
        "the killer's population holds {} tree(s), fewer than the 2218 it was measured with",
        cases.len()
    );
    assert_eq!(
        wrong.first(),
        None,
        "trees the census judges wrongly: {}",
        wrong.len()
    );
}
