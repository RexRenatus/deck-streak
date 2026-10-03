//! Only progression names the XP ledger (SPEC-040 A9, R10): no other crate's code names
//! `xp_ledger`, and no migration but progression's names it, so the grant port is the one writer of
//! a grant and the data-rights port the one eraser, beside it (CHARTER 5, 13).
//!
//! The census reads every crate's `src` and every migration of the repository, prints how many it
//! examined and refuses zero. It asserts a positive artifact beside the absence: the grant port's
//! repository and progression's migration do name the table. A planted crate and a planted
//! migration that name it are refused, by name.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

// The generated population of the table's spellings (SPEC-324, ADR-029's include by path).
#[macro_use]
#[path = "../../../tools/table-census/population.rs"]
mod population;
// The shared reader of every crate's literals (SPEC-324 R1 to R5), included by path as above.
#[path = "../../../tools/table-census/table_census.rs"]
mod table_census;

use population::Spelling;

/// The table the census guards.
const TABLE: &str = "xp_ledger";
/// The context that owns it (docs/CONTEXT-MAP.md).
const OWNER: &str = "progression";

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

/// Whether Rust source text names the table on a line of code. A whole-line comment is prose, and
/// so is what follows ` //` on a line.
fn rust_names_the_table(text: &str) -> bool {
    text.lines().any(|line| {
        let code = line.trim_start();
        if code.starts_with("//") {
            return false;
        }
        let code = code.split(" //").next().unwrap_or_default();
        code.contains(TABLE)
    })
}

/// Whether SQL text names the table outside a `--` comment.
fn sql_names_the_table(text: &str) -> bool {
    text.lines()
        .any(|line| line.split("--").next().unwrap_or_default().contains(TABLE))
}

/// The census of a tree at `root`: every file it read, every file that names the table, and why
/// each one it refused was refused.
struct Census {
    sources: Vec<String>,
    migrations: Vec<String>,
    naming: BTreeSet<String>,
    refused: Vec<String>,
}

fn census(root: &Path) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        migrations: Vec::new(),
        naming: BTreeSet::new(),
        refused: Vec::new(),
    };
    let crates = root.join("crates");
    let mut members: Vec<PathBuf> = fs::read_dir(&crates)
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    for member in members {
        let context = member
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 crate directory")
            .to_owned();
        for source in files(&member.join("src"), "rs") {
            let name = relative(root, &source);
            let text = fs::read_to_string(&source).expect("a readable source");
            if rust_names_the_table(&text) {
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
fn only_the_grant_port_writes_the_xp_ledger() {
    let found = census(&root());
    examined("crate source file(s)", found.sources.clone());
    examined("migration(s)", found.migrations.clone());
    assert_eq!(found.refused, Vec::<String>::new());
    // The positive artifact: the grant port's repository writes the table, and progression's own
    // migration creates it.
    let writers: BTreeSet<&str> = [
        "crates/progression/src/ledger.rs",
        "migrations/004001_progression_xp_ledger.sql",
    ]
    .into_iter()
    .filter(|path| found.naming.contains(*path))
    .collect();
    assert_eq!(
        writers,
        BTreeSet::from([
            "crates/progression/src/ledger.rs",
            "migrations/004001_progression_xp_ledger.sql",
        ]),
        "the grant port and its migration name the table; every file that does: {:?}",
        found.naming
    );

    // A planted crate whose code names the table, and a planted migration of another context that
    // does, are refused by name; the owner's own files, and a mention in a comment, are not.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/progression/src/ledger.rs",
        "const Q: &str = \"INSERT INTO xp_ledger (amount) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/chest.rs",
        "//! A chest never writes xp_ledger itself: it asks the grant port.\n\
         fn pay() {\n    let _ = sqlx::query(\"UPDATE xp_ledger SET amount = 0\"); // a debit\n}\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/relight.rs",
        "fn relight() {} // not xp_ledger: the grant port pays a relight\n",
    );
    plant(
        planted.path(),
        "migrations/004001_progression_xp_ledger.sql",
        "CREATE TABLE xp_ledger (id INTEGER PRIMARY KEY) STRICT;\n",
    );
    plant(
        planted.path(),
        "migrations/999901_quests_debit.sql",
        "-- a planted migration of another context\nDELETE FROM xp_ledger;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/quests/src/chest.rs names xp_ledger, and only progression's code may",
            "migrations/999901_quests_debit.sql names xp_ledger, and only progression's \
             migrations may",
        ]
    );
}

/// How many spellings of `xp_ledger` the population plants: its 9 splits in 9 forms each, and the
/// 14 members of the literal family (SPEC-324 A1).
const JOINED_SPELLINGS: usize = 95;

/// The literal family of `xp_ledger`: each member spells the name in a way no line of code shows
/// as written, and rustc, not the census, evaluates it.
fn family() -> Vec<(&'static str, String)> {
    vec![
        spell!("xp\x5fledger"),
        spell!("xp\u{5f}ledger"),
        spell!(
            "xp_\
             ledger"
        ),
        spell!(concat!(r##"xp_"##, r#"ledger"#)),
        spell!(['x', 'p', '_', 'l', 'e', 'd', 'g', 'e', 'r']),
        spell!(concat!(concat!("xp", "_"), "ledger")),
        spell!(concat!(stringify!(xp_), stringify!(ledger))),
        spell!("XP_LEDGER"),
        spell!(b"xp\x5fledger"),
        spell!(c"xp\x5fledger"),
        spell!([b'x', b'p', b'\x5f', b'l', b'e', b'd', b'g', b'e', b'r']),
        spell!(concat!("Xp_", "LeDgEr")),
        spell!(concat!('x', 'p', "_ledger")),
        spell!("\x78\x70\x5f\x6c\x65\x64\x67\x65\x72"),
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

/// What the census refuses in a tree that holds `spelling` alone. No planted file names the table
/// as written, so the census's line reader alone can refuse none of them.
fn census_of(spelling: &Spelling) -> Vec<String> {
    let planted = tempfile::tempdir().expect("a temporary directory");
    for (path, text) in &spelling.files {
        assert!(
            !text.contains(TABLE),
            "{path} of {} names {TABLE} as written",
            spelling.label
        );
        plant(planted.path(), path, text);
    }
    census(planted.path()).refused
}

#[test]
fn a_reserved_name_joined_from_literals_is_refused_in_every_spelling() {
    // Every spelling, each in a tree of its own, is refused by the name of every file holding a
    // piece of it; the same spelling inside progression is not refused.
    let spellings = examined("joined spelling(s)", spellings());
    assert_eq!(spellings.len(), JOINED_SPELLINGS);
    for spelling in &spellings {
        let mut expected: Vec<String> = spelling
            .files
            .iter()
            .map(|(path, _)| {
                format!(
                    "{path} spells {TABLE} from literals, joined or in another case, and only \
                     {OWNER}'s code may"
                )
            })
            .collect();
        expected.sort();
        assert_eq!(census_of(spelling), expected, "{}", spelling.label);
        let owned = spelling.in_crate(OWNER);
        assert_eq!(census_of(&owned), Vec::<String>::new(), "{}", owned.label);
    }
}

#[test]
fn pieces_that_do_not_cover_the_name_are_not_refused() {
    // Each near miss comes one character short of the name, in every form, and each control joins
    // a value the census cannot read beside pieces that are no part of the name, as the real tree's
    // two `concat!` calls do. None is refused.
    let mut trees = Vec::new();
    for (index, split) in population::near_misses(TABLE).iter().enumerate() {
        assert_eq!(
            split.concat().len(),
            TABLE.len() - 1,
            "{split:?} is one short"
        );
        trees.extend(population::planted(split, index, "quests", "streaks"));
    }
    let controls = [
        "pub const PRIVACY: &str = concat!(env!(\"CARGO_PKG_REPOSITORY\"), \"/blob/main/PRIVACY.md\");\n",
        r#"macro_rules! health_body {
    ($word:literal) => {
        concat!(
            "{\"status\":\"",
            $word,
            "\",\"version\":\"",
            env!("CARGO_PKG_VERSION"),
            "\"}"
        )
    };
}
"#,
        "pub const HOME_LEDGE: &str = concat!(env!(\"HOME\"), \"/ledge\");\n",
    ];
    for (index, text) in controls.into_iter().enumerate() {
        trees.push(Spelling {
            label: format!("control {index}"),
            files: vec![(
                format!("crates/quests/src/c{index:03}_control.rs"),
                text.to_owned(),
            )],
        });
    }
    let trees = examined("near miss and control tree(s)", trees);
    for tree in &trees {
        assert_eq!(census_of(tree), Vec::<String>::new(), "{}", tree.label);
    }
}

#[test]
fn a_name_the_census_cannot_resolve_fails_closed() {
    // A value the census cannot read, joined beside part of the name, and an include whose file
    // the census cannot name or cannot read, are each refused by name. An include joined onto
    // `OUT_DIR` is a build script's output, which SPEC-324 discloses (#585): it is not refused.
    let file = "crates/quests/src/fail.rs";
    let joins = format!(
        "{file} joins a value the census cannot read beside part of {TABLE}, and only {OWNER}'s \
         code may"
    );
    let unnamed =
        format!("{file} includes a file the census cannot name, so it cannot read it for {TABLE}");
    let unread =
        format!("{file} includes ../queries/missing.sql, which the census cannot read for {TABLE}");
    let cases = examined(
        "unresolvable case(s)",
        vec![
            (
                "pub const JOINED: &str = concat!(env!(\"PREFIX\"), \"_ledger\");\n",
                vec![joins.clone()],
            ),
            (
                "pub const JOINED: &str = concat!(\"xp\", tail!());\n",
                vec![joins.clone()],
            ),
            (
                "macro_rules! joined {\n    ($head:expr) => {\n        concat!($head, \"r\")\n    };\n}\n",
                vec![joins],
            ),
            (
                "pub const QUERY: &str = include_str!(env!(\"QUERY_FILE\"));\n",
                vec![unnamed.clone()],
            ),
            (
                "macro_rules! query {\n    ($file:literal) => {\n        include_str!($file)\n    };\n}\n",
                vec![unnamed.clone()],
            ),
            (
                "include!(concat!(env!(\"CALL_FILE\"), \"/step.rs\"));\n",
                vec![unnamed],
            ),
            (
                "pub const QUERY: &str = include_str!(\"../queries/missing.sql\");\n",
                vec![unread],
            ),
            (
                "include!(concat!(env!(\"OUT_DIR\"), \"/step.rs\"));\n",
                Vec::new(),
            ),
        ],
    );
    for (text, expected) in &cases {
        let planted = tempfile::tempdir().expect("a temporary directory");
        plant(planted.path(), file, text);
        assert_eq!(census(planted.path()).refused, *expected, "{text}");
    }
}
