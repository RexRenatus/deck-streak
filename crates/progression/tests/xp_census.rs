//! Only progression names `xp_settlement`, and only coordination settles (SPEC-072 A12; R9;
//! ADR-072): no other crate's code names the table, no migration but progression's names it, no
//! crate but coordination names the `settle` operation, and inside coordination a caller outside
//! `crates/coordination/src/recompute/` passes the owner's-correction cause and never the
//! recompute's.
//!
//! The census reads every crate's `src` and every migration, prints how many it examined and
//! refuses zero. It asserts the positive artifacts beside the absences: progression's `settle`
//! module and migration name the table, and the fold's XP step calls `settle`. A planted crate, a
//! planted migration and a planted caller are each refused, by name.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The table the census guards.
const TABLE: &str = "xp_settlement";
/// The context that owns it (docs/CONTEXT-MAP.md).
const OWNER: &str = "progression";
/// The one context that calls `settle`.
const CALLER: &str = "coordination";
/// How a source names the operation: its module's path in progression's crate.
const OPERATION: &str = "deck_streak_progression::settle";
/// The request type `settle` takes. A grouped import never spells `OPERATION` contiguously, and
/// nothing can call `settle` without building this by name.
const REQUEST: &str = "SettleRequest";
/// The crate's name in a path: a source that never names it cannot reach progression's re-exports.
const PROGRESSION_CRATE: &str = "deck_streak_progression";
/// The crate's package name, which a manifest may bind to another name (`package = "..."`).
const PROGRESSION_PACKAGE: &str = "deck-streak-progression";
/// The names progression's own re-exports and aliases are followed from: the operation, its
/// request type, and (through `settle`) its module.
const ORIGINALS: [&str; 2] = ["settle", REQUEST];
/// Where a recompute step lives inside coordination.
const RECOMPUTE_DIR: &str = "crates/coordination/src/recompute/";
/// The cause a caller outside the recompute steps passes.
const CORRECTION_CAUSE: &str = "SettleCause::OwnersCorrection";
/// The cause only a recompute step passes.
const RECOMPUTE_CAUSE: &str = "SettleCause::Recompute";

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

/// The lines of Rust `text` that are code: a whole-line comment is prose, and so is what follows
/// ` //` on a line.
fn code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .map(|line| line.split(" //").next().unwrap_or_default())
        .collect()
}

/// Whether Rust source text names `needle` on a line of code.
fn rust_names(text: &str, needle: &str) -> bool {
    code_lines(text).iter().any(|line| line.contains(needle))
}

/// Whether Rust source text names the operation's path itself, not a longer name that begins with
/// it (`settled_of_day`).
fn names_the_operation(text: &str) -> bool {
    code_lines(text).iter().any(|line| {
        line.match_indices(OPERATION).any(|(at, _)| {
            !line[at + OPERATION.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
        })
    })
}

/// Whether SQL text names the table outside a `--` comment.
fn sql_names_the_table(text: &str) -> bool {
    text.lines()
        .any(|line| line.split("--").next().unwrap_or_default().contains(TABLE))
}

/// The tokens of Rust `text` that a `use` tree is read from: identifiers, and the punctuation that
/// shapes a tree. A path separator is dropped, because the identifiers in a row are the path.
fn tokens(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut word = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
            continue;
        }
        // `r#name` is the raw spelling of `name`, and the compiler reads the two as one name.
        if character == '#' && word == "r" {
            word.clear();
            continue;
        }
        if !word.is_empty() {
            found.push(std::mem::take(&mut word));
        }
        if matches!(
            character,
            '{' | '}' | ',' | ';' | '*' | '(' | ')' | '=' | '<'
        ) {
            found.push(character.to_string());
        }
    }
    found
}

/// One leaf of a `use` tree: the path to it, and the name it is bound under when it is renamed.
struct Leaf {
    path: Vec<String>,
    alias: Option<String>,
}

/// Reads the tree that starts at `at` (grouped, nested, with or without `as`) below `prefix`, into
/// `out`, and stops before the `}` or `;` that ends it.
fn use_tree(tokens: &[String], at: &mut usize, prefix: &[String], out: &mut Vec<Leaf>) {
    loop {
        let mut path = prefix.to_vec();
        let mut alias = None;
        let mut grouped = false;
        while let Some(token) = tokens.get(*at) {
            match token.as_str() {
                "," | "}" | ";" => break,
                "{" => {
                    *at += 1;
                    use_tree(tokens, at, &path, out);
                    grouped = true;
                    if tokens.get(*at).is_some_and(|next| next == "}") {
                        *at += 1;
                    }
                }
                "as" => {
                    alias = tokens.get(*at + 1).cloned();
                    *at += 2;
                }
                "*" => {
                    path.push("*".to_owned());
                    *at += 1;
                }
                name => {
                    path.push(name.to_owned());
                    *at += 1;
                }
            }
        }
        // `settle::{self as ledger}` renames the module the group is read below: its path is the
        // group's own prefix, so it is a leaf because it carries a name.
        let renamed_self = alias.is_some() && path.last().is_some_and(|last| last == "self");
        if path.last().is_some_and(|last| last == "self") {
            path.pop();
        }
        if !grouped && (path.len() > prefix.len() || renamed_self) {
            out.push(Leaf { path, alias });
        }
        if tokens.get(*at).is_some_and(|next| next == ",") {
            *at += 1;
        } else {
            return;
        }
    }
}

/// Every leaf of every `pub use` (of any visibility, at any depth of module) in Rust `text`.
fn reexports(text: &str) -> Vec<Leaf> {
    let words = tokens(&code_lines(text).join(" "));
    let mut leaves = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let public = index > 0
            && match words[index - 1].as_str() {
                "pub" => true,
                ")" => words[..index - 1]
                    .iter()
                    .rposition(|open| open == "(")
                    .is_some_and(|open| open > 0 && words[open - 1] == "pub"),
                _ => false,
            };
        if word == "use" && public {
            use_tree(&words, &mut (index + 1), &[], &mut leaves);
        }
        // `pub type Alias<'a> = path::Original<'a>;` names the original as `Alias`.
        if word == "type" && public {
            let equals = words[index..].iter().position(|next| next == "=");
            if let (Some(alias), Some(equals)) = (words.get(index + 1), equals) {
                let path: Vec<String> = words[index + equals + 1..]
                    .iter()
                    .take_while(|next| !matches!(next.as_str(), "<" | ";"))
                    .cloned()
                    .collect();
                if !path.is_empty() {
                    leaves.push(Leaf {
                        path,
                        alias: Some(alias.clone()),
                    });
                }
            }
        }
    }
    leaves
}

/// The names progression's own `src` gives `settle`, its request and its module by a renaming
/// `pub use`, each with the original it stands for. A renamed name is followed too, so a chain of
/// renamings ends at the original.
fn progression_aliases(root: &Path) -> BTreeMap<String, String> {
    let leaves: Vec<Leaf> = files(&root.join("crates").join(OWNER).join("src"), "rs")
        .iter()
        .flat_map(|source| reexports(&fs::read_to_string(source).expect("a readable source")))
        .collect();
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    loop {
        let before = aliases.len();
        for leaf in &leaves {
            let (Some(last), Some(alias)) = (leaf.path.last(), &leaf.alias) else {
                continue;
            };
            let original = if ORIGINALS.contains(&last.as_str()) {
                last.clone()
            } else if let Some(original) = aliases.get(last) {
                original.clone()
            } else {
                continue;
            };
            aliases.entry(alias.clone()).or_insert(original);
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}

/// Every leaf of every `use` tree in Rust `words`, of any visibility.
fn use_leaves(words: &[String]) -> Vec<Leaf> {
    let mut leaves = Vec::new();
    for (index, word) in words.iter().enumerate() {
        if word == "use" {
            use_tree(words, &mut (index + 1), &[], &mut leaves);
        }
    }
    leaves
}

/// The tokens of every Rust source in `member`'s `src`.
fn member_words(member: &Path) -> Vec<Vec<String>> {
    files(&member.join("src"), "rs")
        .iter()
        .map(|source| {
            let text = fs::read_to_string(source).expect("a readable source");
            tokens(&code_lines(&text).join(" "))
        })
        .collect()
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

/// The names a manifest (the workspace's or a member's) binds progression's package to:
/// `prog = { package = "deck-streak-progression", .. }`, `prog.package = ..`, or a
/// `[dependencies.prog]` table with that `package`, whose key's `-` is `_` in a path.
fn manifest_names(root: &Path) -> BTreeSet<String> {
    let needle = format!("package={PROGRESSION_PACKAGE}");
    let mut names = BTreeSet::new();
    let manifests = std::iter::once(root.join("Cargo.toml")).chain(
        members(root)
            .into_iter()
            .map(|member| member.join("Cargo.toml")),
    );
    for manifest in manifests {
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let mut table = String::new();
        for line in text.lines() {
            // The line without its spaces and quotes: `prog={package=deck-streak-progression}`.
            let line: String = line
                .chars()
                .filter(|character| !character.is_whitespace() && !matches!(character, '"' | '\''))
                .collect();
            let renames = line.match_indices(&needle).any(|(at, _)| {
                !line[at + needle.len()..]
                    .starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '-')
            });
            if line.starts_with('[') {
                line.trim_matches(['[', ']']).clone_into(&mut table);
            } else if renames {
                let key = if line.starts_with("package=") {
                    table.rsplit('.').next().unwrap_or_default()
                } else {
                    line.split(['=', '.']).next().unwrap_or_default()
                };
                names.insert(key.replace('-', "_"));
            }
        }
    }
    names
}

/// The names a source reaches progression's crate by: its own; every name a manifest binds its
/// package to; and every name a `use` or an `extern crate` in any crate's `src` binds one of those
/// to (`as prog`, `{self as prog}`, `as r#prog`, or through a name already found), to a fixpoint.
fn crate_names(root: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::from([PROGRESSION_CRATE.to_owned()]);
    names.extend(manifest_names(root));
    let mut links: Vec<(String, String)> = Vec::new();
    for words in members(root).iter().flat_map(|member| member_words(member)) {
        for window in words.windows(3) {
            if window[1] == "as" {
                links.push((window[0].clone(), window[2].clone()));
            }
        }
        links.extend(
            use_leaves(&words)
                .into_iter()
                .filter_map(|leaf| Some((leaf.path.last()?.clone(), leaf.alias?))),
        );
    }
    loop {
        let before = names.len();
        for (original, alias) in &links {
            if names.contains(original) {
                names.insert(alias.clone());
            }
        }
        if names.len() == before {
            return names;
        }
    }
}

/// The members whose `src` imports progression's crate, or a module of it, whole with a glob of
/// any visibility (`use prog::*`): each of their files reaches progression's names as
/// `crate::name` or `super::name` without naming the crate.
fn glob_members(root: &Path, crate_names: &BTreeSet<String>) -> BTreeSet<String> {
    members(root)
        .iter()
        .filter(|member| {
            member_words(member).iter().any(|words| {
                use_leaves(words).iter().any(|leaf| {
                    leaf.path.last().is_some_and(|last| last == "*")
                        && leaf
                            .path
                            .iter()
                            .any(|segment| crate_names.contains(segment))
                })
            })
        })
        .filter_map(|member| member.file_name()?.to_str().map(str::to_owned))
        .collect()
}

/// Whether a source reaches the operation itself through a name that denotes progression's crate:
/// `alias::settle` in a path, a `use` or a grouped `use`, or, in a member that globs the crate's
/// root, `settle` as a word (`crate::settle`).
fn reaches_settle(words: &[String], crate_names: &BTreeSet<String>, globbed: bool) -> bool {
    let step = |pair: &[String]| crate_names.contains(&pair[0]) && pair[1] == ORIGINALS[0];
    (globbed && words.iter().any(|word| word == ORIGINALS[0]))
        || words.windows(2).any(step)
        || use_leaves(words)
            .iter()
            .any(|leaf| leaf.path.windows(2).any(step))
}

/// The census of a tree at `root`.
struct Census {
    sources: Vec<String>,
    migrations: Vec<String>,
    naming: BTreeSet<String>,
    calling: BTreeSet<String>,
    refused: Vec<String>,
}

fn census(root: &Path) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        migrations: Vec::new(),
        naming: BTreeSet::new(),
        calling: BTreeSet::new(),
        refused: Vec::new(),
    };
    let aliases = progression_aliases(root);
    let crate_names = crate_names(root);
    let globbed = glob_members(root, &crate_names);
    for member in members(root) {
        let context = member
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 crate directory")
            .to_owned();
        for source in files(&member.join("src"), "rs") {
            let name = relative(root, &source);
            let text = fs::read_to_string(&source).expect("a readable source");
            if rust_names(&text, TABLE) {
                census.naming.insert(name.clone());
                if context != OWNER {
                    census
                        .refused
                        .push(format!("{name} names {TABLE}, and only {OWNER}'s code may"));
                }
            }
            let source_words = tokens(&code_lines(&text).join(" "));
            let direct = names_the_operation(&text)
                || rust_names(&text, REQUEST)
                || reaches_settle(&source_words, &crate_names, globbed.contains(&context));
            // A source that names progression's crate and one of progression's own renamings
            // reaches `settle` without spelling it.
            let words: BTreeSet<String> =
                tokens(&code_lines(&text).join(" ")).into_iter().collect();
            let through: Vec<(&String, &String)> = if context != OWNER
                && !direct
                && (globbed.contains(&context)
                    || crate_names
                        .iter()
                        .any(|crate_name| words.contains(crate_name)))
            {
                aliases
                    .iter()
                    .filter(|(alias, _)| words.contains(*alias))
                    .collect()
            } else {
                Vec::new()
            };
            if context != OWNER && (direct || !through.is_empty()) {
                census.calling.insert(name.clone());
                if context != CALLER {
                    if direct {
                        census
                            .refused
                            .push(format!("{name} calls settle, and only {CALLER}'s code may"));
                    }
                    for (alias, original) in through {
                        census.refused.push(format!(
                            "{name} calls settle through {alias}, {OWNER}'s alias of {original}, \
                             and only {CALLER}'s code may"
                        ));
                    }
                } else if !name.starts_with(RECOMPUTE_DIR)
                    && (!rust_names(&text, CORRECTION_CAUSE) || rust_names(&text, RECOMPUTE_CAUSE))
                {
                    census.refused.push(format!(
                        "{name} calls settle outside the recompute steps, and only the owner's \
                         correction may"
                    ));
                }
            }
            census.sources.push(name);
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

#[test]
fn only_progression_writes_xp_settlement_and_only_coordination_settles() {
    let found = census(&root());
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
    plant(
        planted.path(),
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/chest.rs",
        "//! A chest never writes xp_settlement itself.\n\
         fn pay() {\n    let _ = sqlx::query(\"UPDATE xp_settlement SET amount = 0\"); // a debit\n}\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/relight.rs",
        "use deck_streak_progression::settle::settle;\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/grouped.rs",
        "use deck_streak_progression::{settle::{settle as plant_call, SettleRequest as PlantRequest}};\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/recompute/xp.rs",
        "use deck_streak_progression::settle::{SettleCause, settle};\n\
         fn step() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/correction.rs",
        "use deck_streak_progression::settle::{SettleCause, settle};\n\
         fn fix() { let _ = SettleCause::OwnersCorrection; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/shortcut.rs",
        "use deck_streak_progression::settle::{SettleCause, settle};\n\
         fn quick() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/note.rs",
        "// deck_streak_progression::settle is coordination's alone\nfn quiet() {}\n",
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
    // Progression re-exports `settle`, its request and its module under other names, one of them
    // through another alias and one inside a nested module. A caller outside coordination that
    // names only the new names never spells `settle` or `SettleRequest`, and is refused by file,
    // alias and original all the same. Progression's own use of the names, coordination's callers
    // by the same rules as before, a mention in a comment, a crate that never imports progression
    // and a re-export that renames nothing of the settlement are not refused.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\n\
         pub use settle::{\n    SettledRow,\n    settle as tally,\n    SettleRequest as TallyRequest,\n    settled_of_day,\n};\n\
         pub use settle as ledger_write;\n\
         pub use self::tally as tally_again;\n\
         pub mod api {\n    pub use super::settle::settle as run_it;\n}\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/inner.rs",
        "use crate::settle::settle as tally;\nfn own() { let _ = tally; }\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/tally_user.rs",
        "use deck_streak_progression::{TallyRequest, tally};\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/module_user.rs",
        "use deck_streak_progression::ledger_write;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/chained_user.rs",
        "use deck_streak_progression::tally_again;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/nested_user.rs",
        "use deck_streak_progression::api::run_it;\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/recompute/xp.rs",
        "use deck_streak_progression::{SettleCause, tally};\n\
         fn step() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/correction.rs",
        "use deck_streak_progression::{SettleCause, tally};\n\
         fn fix() { let _ = SettleCause::OwnersCorrection; }\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/shortcut.rs",
        "use deck_streak_progression::{SettleCause, tally};\n\
         fn quick() { let _ = SettleCause::Recompute; }\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/note.rs",
        "use deck_streak_progression::SettledRow;\n// tally and ledger_write are coordination's\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/homonym.rs",
        "fn tally() {}\nfn again() { tally(); }\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/coordination/src/shortcut.rs calls settle outside the recompute steps, and \
             only the owner's correction may",
            "crates/quests/src/chained_user.rs calls settle through tally_again, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/module_user.rs calls settle through ledger_write, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/nested_user.rs calls settle through run_it, progression's alias \
             of settle, and only coordination's code may",
            "crates/streaks/src/tally_user.rs calls settle through TallyRequest, progression's \
             alias of SettleRequest, and only coordination's code may",
            "crates/streaks/src/tally_user.rs calls settle through tally, progression's alias \
             of settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_census_follows_a_grouped_module_renaming_and_a_chain_read_before_its_link() {
    // `a_chain.rs` is read before `lib.rs`, so its renamings of `tally` and of a crate-visible
    // link are met before either is known. The module is renamed inside a group, by `self`.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/a_chain.rs",
        "pub use crate::tally as tally_early;\npub use crate::crate_link as via_crate;\n",
    );
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\n\
         pub mod a_chain;\n\
         pub use settle::{self as ledger, settle as tally};\n\
         pub(crate) use settle::settle as crate_link;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/early_user.rs",
        "use deck_streak_progression::a_chain::tally_early;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/crate_link_user.rs",
        "use deck_streak_progression::a_chain::via_crate;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/ledger_user.rs",
        "use deck_streak_progression::ledger;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/crate_link_user.rs calls settle through via_crate, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/early_user.rs calls settle through tally_early, progression's \
             alias of settle, and only coordination's code may",
            "crates/quests/src/ledger_user.rs calls settle through ledger, progression's alias \
             of settle, and only coordination's code may",
        ]
    );
}

/// Plants progression's `settle` module, the file every advisory case reads its names from.
fn plant_settle(root: &Path) {
    plant(
        root,
        "crates/progression/src/settle.rs",
        "const Q: &str = \"INSERT INTO xp_settlement (amount) VALUES (1)\";\n",
    );
}

#[test]
fn the_census_follows_a_crate_alias() {
    // `prog` is progression's crate under another name, so `use crate::prog::tally` reaches the
    // renamed `settle` without ever spelling the crate's own name.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\npub use settle::settle as tally;\n",
    );
    plant(
        planted.path(),
        "crates/markets/src/prog.rs",
        "pub use deck_streak_progression as prog;\n",
    );
    plant(
        planted.path(),
        "crates/markets/src/via_prog.rs",
        "use crate::prog::tally;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/markets/src/via_prog.rs calls settle through tally, progression's alias of \
             settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_census_follows_a_type_alias() {
    // `pub type Wrapped<'a> = ...SettleRequest<'a>` names the request under another name.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/wrapped.rs",
        "pub type Wrapped<'a> = crate::settle::SettleRequest<'a>;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/typed.rs",
        "use deck_streak_progression::wrapped::Wrapped;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/typed.rs calls settle through Wrapped, progression's alias of \
             SettleRequest, and only coordination's code may",
        ]
    );
}

#[test]
fn a_private_alias_behind_an_attribute_is_not_a_reexport() {
    // The `)` that closes `#[cfg(test)]` is not the `)` that closes `pub(crate)`: the alias behind
    // it is private, so a caller's own function of the same name is a homonym and stays accepted,
    // while the `pub(crate)` link is followed.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\n\
         #[cfg(test)]\n\
         use settle::settle as gated;\n\
         pub(crate) use settle::settle as crate_link;\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/homonym.rs",
        "use deck_streak_progression::SettledRow;\nfn gated() {}\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/link.rs",
        "use deck_streak_progression::crate_link;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/link.rs calls settle through crate_link, progression's alias of \
             settle, and only coordination's code may",
        ]
    );
}

#[test]
fn the_operation_is_matched_as_a_word_not_a_prefix() {
    // `settled_of_day` begins with the operation's path and is another function.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/quests/src/day.rs",
        "use deck_streak_progression::settled_of_day;\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/bare.rs",
        "use deck_streak_progression::settle;\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/direct.rs",
        "use deck_streak_progression::settle::settle;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/streaks/src/bare.rs calls settle, and only coordination's code may",
            "crates/streaks/src/direct.rs calls settle, and only coordination's code may",
        ]
    );
}

/// One way a member binds a name to progression's crate: a label, the files that bind it, and the
/// name a caller reaches the crate by (`crate` for a glob of the crate root). `{name}` in a text is
/// the binding's name and `{krate}` is the crate it names.
struct Binding {
    label: &'static str,
    files: &'static [(&'static str, &'static str)],
    reached_as: &'static str,
}

/// Every form that binds a name to progression's crate, each with a control that names another
/// crate in the same spelling.
const BINDINGS: [Binding; 10] = [
    Binding {
        label: "as x",
        files: &[("src/lib.rs", "pub use {krate} as {name};\n")],
        reached_as: "{name}",
    },
    Binding {
        label: "as r#x",
        files: &[("src/lib.rs", "pub use {krate} as r#{name};\n")],
        reached_as: "{name}",
    },
    Binding {
        label: "{self as x}",
        files: &[("src/lib.rs", "pub use {krate}::{self as {name}};\n")],
        reached_as: "{name}",
    },
    Binding {
        label: "{self as r#x}",
        files: &[("src/lib.rs", "pub use {krate}::{self as r#{name}};\n")],
        reached_as: "{name}",
    },
    Binding {
        label: "chain of two aliases",
        files: &[
            ("src/a_link.rs", "pub use crate::{name}_first as {name};\n"),
            ("src/lib.rs", "pub use {krate} as {name}_first;\n"),
        ],
        reached_as: "{name}",
    },
    Binding {
        label: "extern crate as x",
        files: &[("src/lib.rs", "extern crate {krate} as {name};\n")],
        reached_as: "{name}",
    },
    Binding {
        label: "manifest inline package",
        files: &[(
            "Cargo.toml",
            "[dependencies]\n{name} = { package = \"{package}\", path = \"../x\" }\n",
        )],
        reached_as: "{name}",
    },
    Binding {
        label: "manifest table package",
        files: &[(
            "Cargo.toml",
            "[dependencies.{name}]\npackage = '{package}'\npath = '../x'\n",
        )],
        reached_as: "{name}",
    },
    Binding {
        label: "manifest dotted package",
        files: &[(
            "Cargo.toml",
            "[dependencies]\n{name}.package = \"{package}\"\n{name}.path = \"../x\"\n",
        )],
        reached_as: "{name}",
    },
    Binding {
        label: "re-exported glob of the crate root",
        files: &[("src/lib.rs", "pub use {krate}::*;\n")],
        reached_as: "crate",
    },
];

/// How a caller reaches `settle` through the name its member binds.
const CALLER_SHAPES: [(&str, &str); 3] = [
    ("path", "fn call() { let _ = {reached}::settle(); }\n"),
    (
        "use then call",
        "use {reached}::settle;\nfn call() { let _ = settle(); }\n",
    ),
    (
        "use as then call",
        "use {reached}::{settle as s};\nfn call() { let _ = s(); }\n",
    ),
];

/// Plants one binding in member `member`, naming `krate` (and `package` in a manifest), with a
/// caller of every shape; returns the caller files by their shape.
fn plant_binding(
    root: &Path,
    member: &str,
    binding: &Binding,
    krate: &str,
    package: &str,
) -> Vec<(&'static str, String)> {
    let name = format!("bound_{member}");
    let expand = |text: &str| {
        text.replace("{name}", &name)
            .replace("{krate}", krate)
            .replace("{package}", package)
    };
    for (path, text) in binding.files {
        plant(root, &format!("crates/{member}/{path}"), &expand(text));
    }
    let reached = expand(binding.reached_as);
    CALLER_SHAPES
        .iter()
        .map(|(shape, text)| {
            let caller = format!("crates/{member}/src/call_{}.rs", shape.replace(' ', "_"));
            plant(root, &caller, &text.replace("{reached}", &reached));
            (*shape, caller)
        })
        .collect()
}

#[test]
fn the_census_refuses_every_member_of_the_binding_population() {
    // The class: a name that denotes progression's crate, by any binding form, reached by any
    // caller shape. Every product of the two is planted and must be refused; the same spelling
    // that names another crate is accepted.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    plant(
        planted.path(),
        "crates/progression/src/lib.rs",
        "pub mod settle;\npub use settle::settle as tally;\n",
    );
    let mut members = Vec::new();
    let mut controls = Vec::new();
    for (index, binding) in BINDINGS.iter().enumerate() {
        for (shape, file) in plant_binding(
            planted.path(),
            &format!("pop{index}"),
            binding,
            "deck_streak_progression",
            "deck-streak-progression",
        ) {
            members.push((format!("{} / {shape}", binding.label), file));
        }
        for (shape, file) in plant_binding(
            planted.path(),
            &format!("ctl{index}"),
            binding,
            "deck_streak_other",
            "deck-streak-other",
        ) {
            controls.push((format!("{} / {shape}", binding.label), file));
        }
    }
    let found = census(planted.path());
    println!("class members: examined {}", members.len());
    assert_eq!(members.len(), BINDINGS.len() * CALLER_SHAPES.len());
    let refused_file = |file: &String| {
        found
            .refused
            .iter()
            .any(|line| line.starts_with(&format!("{file} ")))
    };
    let escaping: Vec<&String> = members
        .iter()
        .filter(|(_, file)| !refused_file(file))
        .map(|(label, _)| label)
        .collect();
    assert_eq!(
        escaping,
        Vec::<&String>::new(),
        "members that escape the census: {} of {}",
        escaping.len(),
        members.len()
    );
    let wrongly_refused: Vec<&String> = controls
        .iter()
        .filter(|(_, file)| refused_file(file))
        .map(|(label, _)| label)
        .collect();
    assert_eq!(
        wrongly_refused,
        Vec::<&String>::new(),
        "a control naming another crate is accepted"
    );
    assert_eq!(controls.len(), members.len());
}

#[test]
fn the_census_reads_a_raw_identifier_as_its_plain_name() {
    // `r#raw_tally` and `raw_tally` are one name to the compiler, so a caller may import the alias
    // in either spelling.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    for (path, text) in [
        (
            "crates/progression/src/lib.rs",
            "pub mod settle;\npub use settle::settle as r#raw_tally;\n",
        ),
        (
            "crates/quests/src/plain.rs",
            "use deck_streak_progression::raw_tally;\n",
        ),
        (
            "crates/quests/src/raw.rs",
            "use deck_streak_progression::r#raw_tally;\n",
        ),
    ] {
        plant(planted.path(), path, text);
    }
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/plain.rs calls settle through raw_tally, progression's alias of \
             settle, and only coordination's code may",
            "crates/quests/src/raw.rs calls settle through raw_tally, progression's alias of \
             settle, and only coordination's code may",
        ]
    );
}

/// The refusal of a planted `crates/{file}` that reaches the renamed `settle` as `tally`.
fn through_tally(file: &str) -> String {
    format!(
        "crates/{file} calls settle through tally, progression's alias of settle, and only \
         coordination's code may"
    )
}

#[test]
fn the_census_follows_a_crate_alias_however_it_is_written() {
    // Each member reaches progression's crate by a name other than its own: by a glob, by an
    // `extern crate`, renamed inside a group, through a chain read before its link, and in raw
    // spelling. A member whose glob is another crate's, or which imports progression's crate
    // without a glob, keeps its own `tally`.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    for (path, text) in [
        (
            "crates/progression/src/lib.rs",
            "pub mod settle;\npub use settle::settle as tally;\n",
        ),
        (
            "crates/habits/src/lib.rs",
            "pub use deck_streak_progression::*;\n",
        ),
        ("crates/habits/src/via_glob.rs", "use crate::tally;\n"),
        (
            "crates/economy/src/lib.rs",
            "extern crate deck_streak_progression as ext_prog;\n",
        ),
        ("crates/economy/src/via_extern.rs", "use ext_prog::tally;\n"),
        (
            "crates/markets/src/lib.rs",
            "pub use deck_streak_progression::{self as grouped_prog};\n",
        ),
        (
            "crates/markets/src/via_grouped.rs",
            "use crate::grouped_prog::tally;\n",
        ),
        (
            "crates/quests/src/a_link.rs",
            "pub use crate::first_prog as second_prog;\n",
        ),
        (
            "crates/quests/src/lib.rs",
            "pub use deck_streak_progression as first_prog;\n",
        ),
        (
            "crates/quests/src/via_chain.rs",
            "use crate::second_prog::tally;\n",
        ),
        ("crates/readings/src/lib.rs", "use std::io::*;\n"),
        ("crates/readings/src/own.rs", "fn tally() {}\n"),
        (
            "crates/streaks/src/lib.rs",
            "pub use deck_streak_progression as r#raw_prog;\n",
        ),
        (
            "crates/streaks/src/via_raw.rs",
            "use crate::raw_prog::tally;\n",
        ),
        (
            "crates/vault/src/lib.rs",
            "use deck_streak_progression::SettledRow;\n",
        ),
        ("crates/vault/src/own.rs", "fn tally() {}\n"),
    ] {
        plant(planted.path(), path, text);
    }
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            through_tally("economy/src/via_extern.rs"),
            through_tally("habits/src/via_glob.rs"),
            through_tally("markets/src/via_grouped.rs"),
            through_tally("quests/src/via_chain.rs"),
            through_tally("streaks/src/via_raw.rs"),
        ]
    );
}

#[test]
fn the_census_follows_a_crate_renamed_by_a_manifest() {
    // A manifest's `package` binds progression's package to another name: in the workspace's own
    // table, and in a member's inline, table (with a `-` in its key) and dotted-key forms. A
    // package whose name only begins with progression's is another package.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant_settle(planted.path());
    for (path, text) in [
        (
            "crates/progression/src/lib.rs",
            "pub mod settle;\npub use settle::settle as tally;\n",
        ),
        (
            "Cargo.toml",
            "[workspace.dependencies]\n\
             workspace_prog = { package = \"deck-streak-progression\", path = \"crates/progression\" }\n",
        ),
        (
            "crates/analytics/src/via_workspace.rs",
            "use workspace_prog::tally;\n",
        ),
        (
            "crates/focus/Cargo.toml",
            "[dependencies]\n\
             focus_prog = { package = \"deck-streak-progression\", path = \"../progression\" }\n",
        ),
        ("crates/focus/src/via_inline.rs", "use focus_prog::tally;\n"),
        (
            "crates/insights/Cargo.toml",
            "[dependencies.insights-prog]\n\
             package = 'deck-streak-progression'\n\
             path = '../progression'\n",
        ),
        (
            "crates/insights/src/via_table.rs",
            "use insights_prog::tally;\n",
        ),
        (
            "crates/markets/Cargo.toml",
            "[dependencies]\n\
             dotted_prog.package = \"deck-streak-progression\"\n\
             dotted_prog.path = \"../progression\"\n",
        ),
        (
            "crates/markets/src/via_dotted.rs",
            "use dotted_prog::tally;\n",
        ),
        (
            "crates/vault/Cargo.toml",
            "[dependencies]\n\
             vault_other = { package = \"deck-streak-progression-extra\", path = \"../extra\" }\n",
        ),
        ("crates/vault/src/own.rs", "use vault_other::tally;\n"),
    ] {
        plant(planted.path(), path, text);
    }
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            through_tally("analytics/src/via_workspace.rs"),
            through_tally("focus/src/via_inline.rs"),
            through_tally("insights/src/via_table.rs"),
            through_tally("markets/src/via_dotted.rs"),
        ]
    );
}
