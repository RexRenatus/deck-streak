//! The crate graph holds the engine core to the two client adapters (SPEC-345 A7, A8; ADR-356 D4).
//!
//! The census reads every member manifest line by line: the dependency tables (`[dependencies]`,
//! `[dev-dependencies]`, `[build-dependencies]`, their `[target.<cfg>.…]` forms and their dotted
//! `[dependencies.<name>]` forms) and each entry's `package = "…"` rename, resolved through the
//! root's `[workspace.dependencies]`. A header that names dependencies in any other form, and a
//! dependency line it cannot read, are refused as unreadable rather than skipped. Each test also
//! judges a planted tree or map, so a census that went blind fails it.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// The engine core's package.
const CORE: &str = "deck-streak-engine-core";
/// Anki's engine.
const ENGINE: &str = "anki";
/// The two client adapters, which no member may name.
const ADAPTERS: [&str; 2] = ["deck-streak-ffi", "deck-streak-web-engine"];
/// The `cfg` of the web engine's target table (ADR-348).
const WASM32: &str = "cfg(target_arch = \"wasm32\")";
/// The members that may name the core, each in the one table it may name it in (ADR-356 D4).
const DEPENDENTS: [(&str, Kind, Option<&str>); 2] = [
    ("ffi", Kind::Normal, None),
    ("web-engine", Kind::Normal, Some(WASM32)),
];
/// The members that may name the engine outside a dev table: the core and the mirror's own port.
const ENGINE_HOLDERS: [&str; 2] = ["engine-core", "ingest"];

/// Which kind of dependency table an entry sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Normal,
    Dev,
    Build,
}

/// One dependency a member's manifest declares.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    member: String,
    package: String,
    kind: Kind,
    target: Option<String>,
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            Kind::Normal => "dependencies",
            Kind::Dev => "dev-dependencies",
            Kind::Build => "build-dependencies",
        };
        match &self.target {
            Some(cfg) => write!(
                f,
                "{} names {} under [target.'{cfg}'.{kind}]",
                self.member, self.package
            ),
            None => write!(f, "{} names {} under [{kind}]", self.member, self.package),
        }
    }
}

/// What a `[…]` header opens.
#[derive(Debug, PartialEq, Eq)]
enum Opens {
    /// A dependency table: its kind, its target `cfg`, and the one entry a dotted table names.
    Table(Kind, Option<String>, Option<String>),
    /// A table that holds no dependency.
    Other,
    /// A header naming dependencies in a form the census does not read.
    Unreadable,
}

/// Strips one pair of matching quotes, `'…'` or `"…"`.
fn unquote(text: &str) -> &str {
    let text = text.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = text.strip_prefix(quote).and_then(|t| t.strip_suffix(quote)) {
            return inner;
        }
    }
    text
}

/// Splits `<cfg>.<rest>` after `target.`, where the cfg may be quoted and hold dots.
fn split_target(after: &str) -> Option<(String, &str)> {
    let quote = after.chars().next().filter(|c| *c == '\'' || *c == '"');
    let (cfg, rest) = if let Some(quote) = quote {
        let end = after[1..].find(quote)? + 1;
        (&after[1..end], &after[end + 1..])
    } else {
        let end = after.find('.')?;
        (&after[..end], &after[end..])
    };
    Some((cfg.to_owned(), rest.strip_prefix('.')?))
}

/// What the header line `line` opens.
fn opens(line: &str) -> Opens {
    let inner = line
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    let (target, rest) = if let Some(after) = inner.strip_prefix("target.") {
        match split_target(after) {
            Some((cfg, rest)) => (Some(cfg), rest),
            None => return Opens::Unreadable,
        }
    } else {
        (None, inner)
    };
    let (head, dotted) = match rest.split_once('.') {
        Some((head, name)) => (head, Some(unquote(name).to_owned())),
        None => (rest, None),
    };
    let kind = match head {
        "dependencies" => Kind::Normal,
        "dev-dependencies" => Kind::Dev,
        "build-dependencies" => Kind::Build,
        _ if inner.contains("dependencies") => return Opens::Unreadable,
        _ => return Opens::Other,
    };
    Opens::Table(kind, target, dotted)
}

/// The key a dependency line declares (`name = …` or `name.workspace = true`), or `None`.
fn key(line: &str) -> Option<String> {
    let (left, _) = line.split_once('=')?;
    let left = left.trim();
    let name = unquote(left.split('.').next()?);
    let readable = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    readable.then(|| name.to_owned())
}

/// The value of `package = "…"` on a line, if it carries one.
fn package(line: &str) -> Option<String> {
    let at = line.find("package")?;
    let rest = line[at + "package".len()..]
        .trim_start()
        .strip_prefix('=')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_owned())
}

/// A manifest read by the census: the edges it declares and the lines it could not read.
#[derive(Debug, Default)]
struct Read {
    edges: Vec<Edge>,
    unreadable: Vec<String>,
}

/// Reads `text`, the manifest of `member`, resolving `.workspace = true` keys through `renames`.
fn read(member: &str, text: &str, renames: &BTreeMap<String, String>) -> Read {
    let mut out = Read::default();
    let mut table: Option<(Kind, Option<String>)> = None;
    let mut dotted: Option<Edge> = None;
    let resolve = |key: String| renames.get(&key).cloned().unwrap_or(key);
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            out.edges.extend(dotted.take());
            table = None;
            match opens(line) {
                Opens::Table(kind, target, None) => table = Some((kind, target)),
                Opens::Table(kind, target, Some(name)) => {
                    dotted = Some(Edge {
                        member: member.to_owned(),
                        package: resolve(name),
                        kind,
                        target,
                    });
                }
                Opens::Other => {}
                Opens::Unreadable => out.unreadable.push(format!(
                    "{member} line {}: unreadable header {line}",
                    number + 1
                )),
            }
            continue;
        }
        if let Some(edge) = dotted.as_mut() {
            if let Some(renamed) = package(line) {
                edge.package = renamed;
            }
            continue;
        }
        let Some((kind, target)) = &table else {
            continue;
        };
        match key(line) {
            Some(name) => out.edges.push(Edge {
                member: member.to_owned(),
                package: package(line).unwrap_or_else(|| resolve(name)),
                kind: *kind,
                target: target.clone(),
            }),
            None => out.unreadable.push(format!(
                "{member} line {}: unreadable dependency line {line}",
                number + 1
            )),
        }
    }
    out.edges.extend(dotted);
    out
}

/// The root's `[workspace.dependencies]` keys that rename their package.
fn renames(root_manifest: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    for line in root_manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[workspace.dependencies]";
        } else if inside && let (Some(name), Some(renamed)) = (key(line), package(line)) {
            found.insert(name, renamed);
        }
    }
    found
}

/// The census of the workspace at `root`: every member's edges, and the refusals by name.
struct Census {
    manifests: Vec<String>,
    edges: Vec<Edge>,
    refused: Vec<String>,
}

fn census(root: &Path) -> Census {
    let root_manifest =
        fs::read_to_string(root.join("Cargo.toml")).expect("the root manifest reads");
    let renames = renames(&root_manifest);
    let mut members: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .expect("the crates directory reads")
        .map(|entry| entry.expect("a directory entry reads").path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    members.sort();
    let mut manifests = Vec::new();
    let mut edges = Vec::new();
    let mut refused = Vec::new();
    for dir in members {
        let member = dir
            .file_name()
            .expect("a member directory has a name")
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(dir.join("Cargo.toml")).expect("a member manifest reads");
        let read = read(&member, &text, &renames);
        manifests.push(format!("crates/{member}/Cargo.toml"));
        edges.extend(read.edges);
        refused.extend(read.unreadable);
    }
    for edge in &edges {
        let allowed_dependent = DEPENDENTS.iter().any(|&(member, kind, target)| {
            edge.member == member && edge.kind == kind && edge.target.as_deref() == target
        });
        if edge.package == CORE && !allowed_dependent {
            refused.push(format!(
                "{edge}: only ffi and the web engine's wasm32 table may"
            ));
        }
        if edge.package == ENGINE
            && edge.kind != Kind::Dev
            && !ENGINE_HOLDERS.contains(&edge.member.as_str())
        {
            refused.push(format!("{edge}: only the core and ingest hold the engine"));
        }
        if ADAPTERS.contains(&edge.package.as_str()) {
            refused.push(format!("{edge}: no member depends on a client adapter"));
        }
    }
    Census {
        manifests,
        edges,
        refused,
    }
}

/// The edges that name the core, as `(member, kind, target)`.
fn dependents(edges: &[Edge]) -> Vec<(String, Kind, Option<String>)> {
    let mut found: Vec<_> = edges
        .iter()
        .filter(|edge| edge.package == CORE)
        .map(|edge| (edge.member.clone(), edge.kind, edge.target.clone()))
        .collect();
    found.sort();
    found
}

/// Writes `files` (path, text) under a fresh scratch directory and returns it.
fn plant(test: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = support::scratch("engine-core-graph", test);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a planted file has a directory"))
            .expect("a planted directory");
        fs::write(path, text).expect("a planted file writes");
    }
    root
}

#[test]
fn only_the_client_adapters_reach_the_engine_core() {
    let workspace = census(&support::workspace());
    support::examined("member manifest(s)", workspace.manifests.clone());
    assert_eq!(
        dependents(&workspace.edges),
        DEPENDENTS
            .iter()
            .map(|&(member, kind, target)| (member.to_owned(), kind, target.map(str::to_owned)))
            .collect::<Vec<_>>(),
        "the members that name the core, and the table each names it in"
    );
    assert_eq!(
        workspace.refused,
        Vec::<String>::new(),
        "the workspace's refusals"
    );

    let planted = plant(
        "planted",
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.dependencies]\n\
                 deck-streak-engine-core = { path = \"crates/engine-core\" }\n\
                 core-by-another-name = { package = \"deck-streak-engine-core\", path = \"crates/engine-core\" }\n\
                 anki = { git = \"https://example.invalid/anki.git\" }\n",
            ),
            (
                "crates/engine-core/Cargo.toml",
                "[package]\nname = \"deck-streak-engine-core\"\n\n[dependencies]\nanki.workspace = true\n",
            ),
            (
                "crates/ffi/Cargo.toml",
                "[dependencies]\ndeck-streak-engine-core.workspace = true\n\n[dev-dependencies]\nanki.workspace = true\n",
            ),
            (
                "crates/web-engine/Cargo.toml",
                "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\ndeck-streak-engine-core.workspace = true\n",
            ),
            (
                "crates/daemon/Cargo.toml",
                "[dev-dependencies]\ndeck-streak-engine-core.workspace = true\n",
            ),
            (
                "crates/api/Cargo.toml",
                "[dependencies.deck-streak-engine-core]\nworkspace = true\n",
            ),
            (
                "crates/bot/Cargo.toml",
                "[dependencies]\nengine = { package = \"deck-streak-engine-core\", path = \"../engine-core\" }\n",
            ),
            (
                "crates/mcp/Cargo.toml",
                "[dependencies]\ncore-by-another-name.workspace = true\n",
            ),
            (
                "crates/coordination/Cargo.toml",
                "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\ndeck-streak-engine-core.workspace = true\n",
            ),
            (
                "crates/readings/Cargo.toml",
                "[build-dependencies]\nanki.workspace = true\n",
            ),
            (
                "crates/vault/Cargo.toml",
                "[dev-dependencies]\ndeck-streak-ffi.workspace = true\n",
            ),
            (
                "crates/agent/Cargo.toml",
                "[target.'cfg(unix)'.shared-dependencies]\nanki.workspace = true\n",
            ),
        ],
    );
    let planted = census(&planted);
    assert_eq!(
        planted.manifests.len(),
        11,
        "the planted manifests: {:?}",
        planted.manifests
    );
    assert_eq!(
        planted.refused,
        vec![
            "agent line 1: unreadable header [target.'cfg(unix)'.shared-dependencies]".to_owned(),
            "api names deck-streak-engine-core under [dependencies]: only ffi and the web engine's wasm32 table may".to_owned(),
            "bot names deck-streak-engine-core under [dependencies]: only ffi and the web engine's wasm32 table may".to_owned(),
            "coordination names deck-streak-engine-core under [target.'cfg(target_arch = \"wasm32\")'.dependencies]: only ffi and the web engine's wasm32 table may".to_owned(),
            "daemon names deck-streak-engine-core under [dev-dependencies]: only ffi and the web engine's wasm32 table may".to_owned(),
            "mcp names deck-streak-engine-core under [dependencies]: only ffi and the web engine's wasm32 table may".to_owned(),
            "readings names anki under [build-dependencies]: only the core and ingest hold the engine".to_owned(),
            "vault names deck-streak-ffi under [dev-dependencies]: no member depends on a client adapter".to_owned(),
        ],
        "each planted dependent is refused by name"
    );
}

/// The `depends on:` of each named context in the `context-map` fence of `map`, or `None`.
fn declared<'a>(map: &str, contexts: &[&'a str]) -> Vec<(&'a str, Option<String>)> {
    let fence = map
        .split("```context-map\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .unwrap_or("");
    let lines = support::examined(
        "context-map line(s)",
        fence
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect(),
    );
    contexts
        .iter()
        .map(|&context| {
            let edge = lines
                .iter()
                .find(|line| line.split_whitespace().next() == Some(context))
                .and_then(|line| line.rsplit_once("depends on:"))
                .map(|(_, edge)| edge.trim().to_owned());
            (context, edge)
        })
        .collect()
}

#[test]
fn the_context_map_declares_the_core_and_its_two_edges() {
    let contexts = [
        "deck-streak-engine-core",
        "deck-streak-ffi",
        "deck-streak-web-engine",
    ];
    let map = fs::read_to_string(support::workspace().join("docs/CONTEXT-MAP.md"))
        .expect("the context map reads");
    assert_eq!(
        declared(&map, &contexts),
        vec![
            ("deck-streak-engine-core", Some("fsrs7".to_owned())),
            ("deck-streak-ffi", Some("engine-core".to_owned())),
            ("deck-streak-web-engine", Some("engine-core".to_owned())),
        ],
        "docs/CONTEXT-MAP.md's fence"
    );

    let planted = "```context-map\n\
                   deck-streak-kernel   (shared kernel)  depends on: nothing\n\
                   deck-streak-ffi      (FFI adapter)    depends on: nothing\n\
                   ```\n";
    assert_eq!(
        declared(planted, &contexts),
        vec![
            ("deck-streak-engine-core", None),
            ("deck-streak-ffi", Some("nothing".to_owned())),
            ("deck-streak-web-engine", None),
        ],
        "a planted map without the core reads as such"
    );
}

/// The FSRS-7 crate's name as source code spells it.
const SEAM_CRATE: &str = "deck_streak_fsrs7";

/// The one core source that may name the FSRS-7 crate (SPEC-386 R7, ADR-400 D1).
const SEAM_FILE: &str = "src/replay.rs";

/// What no client adapter's source may name: the replay's call, its result and its crate (SPEC-386 R14).
const REPLAY_NAMES: [&str; 3] = [".replay(", "CardReplay", SEAM_CRATE];

/// Every `.rs` file under `root.join(dir)`, as `/`-separated paths relative to `root`, sorted.
fn sources(root: &Path, dir: &str) -> Vec<String> {
    let mut pending = vec![root.join(dir)];
    let mut found = Vec::new();
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("a source directory reads") {
            let path = entry.expect("a source entry reads").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("a walked file sits under its root");
                found.push(
                    relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    found.sort();
    found
}

/// Each of `files` (relative to `root`) whose text holds `name`.
fn naming(root: &Path, files: &[String], name: &str) -> Vec<String> {
    files
        .iter()
        .filter(|file| {
            fs::read_to_string(root.join(file.as_str()))
                .expect("a walked source reads")
                .contains(name)
        })
        .cloned()
        .collect()
}

/// The core's sources that name the FSRS-7 crate, each refused unless it is the one seam file.
fn seam_refusals(core: &Path, walked: &[String]) -> Vec<String> {
    naming(core, walked, SEAM_CRATE)
        .into_iter()
        .filter(|file| file.as_str() != SEAM_FILE)
        .map(|file| format!("{file} names {SEAM_CRATE}: only {SEAM_FILE} may"))
        .collect()
}

#[test]
fn only_the_replay_module_names_the_fsrs7_crate() {
    let core = support::workspace().join("crates/engine-core");
    let walked = sources(&core, "src");
    assert_eq!(
        naming(&core, &walked, SEAM_CRATE),
        vec![SEAM_FILE.to_owned()],
        "the core's sources that name the FSRS-7 crate"
    );
    assert_eq!(
        seam_refusals(&core, &walked),
        Vec::<String>::new(),
        "the core's seam census"
    );
    support::examined("engine-core source file(s)", walked);

    let planted = plant(
        "only_the_replay_module_names_the_fsrs7_crate",
        &[
            ("src/lib.rs", "pub mod replay;\npub mod second;\n"),
            ("src/replay.rs", "use deck_streak_fsrs7::stock;\n"),
            ("src/second.rs", "use deck_streak_fsrs7::convert;\n"),
        ],
    );
    let planted_walk = sources(&planted, "src");
    assert_eq!(
        seam_refusals(&planted, &planted_walk),
        vec!["src/second.rs names deck_streak_fsrs7: only src/replay.rs may".to_owned()],
        "a planted second file naming the crate is refused by name"
    );
    support::examined("planted source file(s)", planted_walk);
}

/// Each adapter source under `root` that names the replay, refused by the name it holds.
fn adapter_refusals(root: &Path, walked: &[String]) -> Vec<String> {
    REPLAY_NAMES
        .iter()
        .flat_map(|name| {
            naming(root, walked, name)
                .into_iter()
                .map(move |file| format!("{file} names {name}: no client adapter may"))
        })
        .collect()
}

#[test]
fn no_client_adapter_names_the_replay() {
    let workspace = support::workspace();
    let walked = [
        sources(&workspace, "crates/ffi/src"),
        sources(&workspace, "crates/web-engine/src"),
    ]
    .concat();
    assert_eq!(
        adapter_refusals(&workspace, &walked),
        Vec::<String>::new(),
        "the client adapters' sources that name the replay"
    );
    support::examined("client adapter source file(s)", walked);

    let planted = plant(
        "no_client_adapter_names_the_replay",
        &[
            (
                "crates/ffi/src/engine.rs",
                "let result = core.replay(&decks, &[], 0.9, 36500);\n",
            ),
            ("crates/ffi/src/lib.rs", "pub mod engine;\n"),
            (
                "crates/web-engine/src/wasm.rs",
                "fn open(result: CardReplay) {}\n",
            ),
            (
                "crates/web-engine/src/study.rs",
                "use deck_streak_fsrs7::stock;\n",
            ),
        ],
    );
    let planted_walk = [
        sources(&planted, "crates/ffi/src"),
        sources(&planted, "crates/web-engine/src"),
    ]
    .concat();
    assert_eq!(
        adapter_refusals(&planted, &planted_walk),
        vec![
            "crates/ffi/src/engine.rs names .replay(: no client adapter may".to_owned(),
            "crates/web-engine/src/wasm.rs names CardReplay: no client adapter may".to_owned(),
            "crates/web-engine/src/study.rs names deck_streak_fsrs7: no client adapter may"
                .to_owned(),
        ],
        "each planted adapter source naming the replay is refused by name"
    );
    support::examined("planted adapter source file(s)", planted_walk);
}
