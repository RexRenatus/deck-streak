//! SPEC-435 A1 to A5 (R1 to R7, ADR-449 D1): a destructive migration names its ruling.
//!
//! The census reads every `.sql` file directly in `migrations/`, in name order, and names no file
//! to skip. It blanks `--` and `/* */` comments, single-quoted literals and double-quoted
//! identifiers to spaces, keeping every line break; splits the text at `;`; upper-cases each
//! statement and splits it into words of `[A-Z0-9_]`; and judges each statement against seven
//! classes, each at most once per statement. A statement's line is the line of its first
//! character that is not white space, so a statement split across lines is one hit. The removal
//! verb is joined from two parts at run time and is spelled whole nowhere in the tree, and every
//! plant builds it from the same two parts.
//!
//! The register is [`ADMITTED`]: one row per migration and class, with the count of its hits, the
//! SPEC that admits it and the reason. The census refuses a hit no row lists, a row whose count
//! differs, a row it does not reach, and a row whose one `docs/specs/SPEC-<n>-*.md` file does not
//! name the migration's file name. It prints `examined N migration(s)` and each hit as
//! `<path>:<line> <class>`, never a statement's text, and refuses zero examined, which is VOID.
//!
//! Every red is planted in a temporary directory at run time, never in the product tree. The tree
//! test reads `migrations/` and `docs/specs/` and runs in CI.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One row of the register: a migration's hits of one class, and the ruling that admits them.
#[expect(
    dead_code,
    reason = "a reviewer reads each row's reason; the census reads its counts and its ruling"
)]
struct Admitted {
    /// The migration's file name, in `migrations/`.
    migration: &'static str,
    /// The class of R4.
    class: &'static str,
    /// How many statements of the class the migration holds.
    count: usize,
    /// The SPEC that admits it, as `SPEC-<n>`: its one file `docs/specs/SPEC-<n>-*.md` names the
    /// migration's file name.
    spec: &'static str,
    /// Why it stands.
    reason: &'static str,
}

/// Every destructive statement on dev, entered as admitted history (SPEC-435 R6). A row is added
/// by the delivery that adds its destructive migration, citing its own SPEC, whose text names the
/// migration.
const ADMITTED: [Admitted; 2] = [
    Admitted {
        migration: "004102_notifications_queue_claim.sql",
        class: "removes a table",
        count: 1,
        spec: "SPEC-041",
        reason: "a `STRICT` rebuild that adds the claim column and the sending state (section 8)",
    },
    Admitted {
        migration: "004102_notifications_queue_claim.sql",
        class: "renames",
        count: 1,
        spec: "SPEC-041",
        reason: "the same rebuild's rename of the new table into place (section 8)",
    },
];

/// A destructive statement the census found.
#[derive(Debug)]
struct Hit {
    /// The migration, from the repository's root.
    path: String,
    /// The line of the statement's first character that is not white space.
    line: usize,
    /// The class of R4.
    class: &'static str,
}

/// What the census read.
#[derive(Debug, Default)]
struct Census {
    /// How many migrations it read.
    examined: usize,
    /// Every hit, in name order, then by line.
    hits: Vec<Hit>,
}

/// The removal verb's two parts, joined at run time so the verb is spelled whole nowhere (R5).
const REMOVAL_PARTS: [&str; 2] = ["DR", "OP"];

/// The classes of R4, as the census names them.
const REMOVES_TABLE: &str = "removes a table";
const REMOVES_COLUMN: &str = "removes a column";
const REMOVES_INDEX: &str = "removes an index";
const REMOVES_VIEW: &str = "removes a view";
const REMOVES_TRIGGER: &str = "removes a trigger";
const RENAMES: &str = "renames";
const DELETES_ROWS: &str = "deletes rows";

/// The census of `migrations/` under `root` (R1 to R5, R7).
fn census(root: &Path) -> Result<Census, String> {
    let dir = root.join("migrations");
    let unreadable = |error: std::io::Error| format!("cannot read migrations/: {error}");
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().map_err(unreadable)?.is_file()
            && Path::new(&name).extension().is_some_and(|ext| ext == "sql")
        {
            names.push(name);
        }
    }
    names.sort();
    println!("examined {} migration(s)", names.len());
    if names.is_empty() {
        return Err(
            "examined 0 migration(s): the population is empty, which is VOID and never a pass"
                .to_owned(),
        );
    }
    let mut hits = Vec::new();
    for name in &names {
        let path = format!("migrations/{name}");
        let text = fs::read_to_string(dir.join(name))
            .map_err(|error| format!("cannot read {path}: {error}"))?;
        for (line, class) in hits_of(&text) {
            println!("{path}:{line} {class}");
            hits.push(Hit {
                path: path.clone(),
                line,
                class,
            });
        }
    }
    Ok(Census {
        examined: names.len(),
        hits,
    })
}

/// Every destructive statement of one migration's text, as its line and its class (R2 to R4).
fn hits_of(text: &str) -> Vec<(usize, &'static str)> {
    let blanked = blank(text);
    let verb = REMOVAL_PARTS.concat();
    let mut hits = Vec::new();
    let mut line = 1;
    for statement in blanked.split(';') {
        if let Some(offset) = statement.find(|c: char| !c.is_whitespace()) {
            let at = line + statement[..offset].matches('\n').count();
            let upper = statement.to_ascii_uppercase();
            let words: Vec<&str> = upper
                .split(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
                .filter(|word| !word.is_empty())
                .collect();
            hits.extend(classes(&words, &verb).into_iter().map(|class| (at, class)));
        }
        line += statement.matches('\n').count();
    }
    hits
}

/// The classes one statement's words hold, each at most once (R4).
fn classes<'w>(words: &[&'w str], verb: &'w str) -> Vec<&'static str> {
    let opens = |first: &str, second: &str| words.starts_with(&[first, second]);
    let holds = |word| words.contains(&word);
    let then = |first: &str, second: &str| {
        words
            .windows(2)
            .any(|pair| pair[0] == first && pair[1] == second)
    };
    let mut found = Vec::new();
    if then(verb, "TABLE") {
        found.push(REMOVES_TABLE);
    }
    if opens("ALTER", "TABLE") && holds(verb) {
        found.push(REMOVES_COLUMN);
    }
    if then(verb, "INDEX") {
        found.push(REMOVES_INDEX);
    }
    if then(verb, "VIEW") {
        found.push(REMOVES_VIEW);
    }
    if then(verb, "TRIGGER") {
        found.push(REMOVES_TRIGGER);
    }
    if opens("ALTER", "TABLE") && holds("RENAME") {
        found.push(RENAMES);
    }
    if then("DELETE", "FROM") || then("OR", "REPLACE") || opens("REPLACE", "INTO") {
        found.push(DELETES_ROWS);
    }
    found
}

/// The text with `--` and `/* */` comments, single-quoted literals and double-quoted identifiers
/// each replaced by spaces, every line break kept (R2). A doubled quote inside a literal closes it
/// and opens the next, which blanks the same characters.
fn blank(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut blanked = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let next = chars.get(at + 1).copied();
        let end = if chars[at] == '-' && next == Some('-') {
            position(&chars, at + 2, &['\n']).unwrap_or(chars.len())
        } else if chars[at] == '/' && next == Some('*') {
            position(&chars, at + 2, &['*', '/']).map_or(chars.len(), |close| close + 2)
        } else if chars[at] == '\'' || chars[at] == '"' {
            position(&chars, at + 1, &[chars[at]]).map_or(chars.len(), |close| close + 1)
        } else {
            blanked.push(chars[at]);
            at += 1;
            continue;
        };
        blanked.extend(
            chars[at..end]
                .iter()
                .map(|&c| if c == '\n' { '\n' } else { ' ' }),
        );
        at = end;
    }
    blanked
}

/// Where `pattern` next starts in `chars`, at or after `from`.
fn position(chars: &[char], from: usize, pattern: &[char]) -> Option<usize> {
    (from..chars.len()).find(|&at| chars[at..].starts_with(pattern))
}

/// Holds the census's hits equal to `register`, and each row's one SPEC file to the migration it
/// names (R6). A refusal names its reason on each of its lines.
fn judge(root: &Path, census: &Census, register: &[Admitted]) -> Result<(), String> {
    let mut refusals = Vec::new();
    for hit in &census.hits {
        let listed = register.iter().any(|row| {
            hit.path == format!("migrations/{}", row.migration) && hit.class == row.class
        });
        if !listed {
            refusals.push(format!("unlisted: {}:{} {}", hit.path, hit.line, hit.class));
        }
    }
    for row in register {
        let path = format!("migrations/{}", row.migration);
        let found = census
            .hits
            .iter()
            .filter(|hit| hit.path == path && hit.class == row.class)
            .count();
        if found == 0 {
            refusals.push(format!("stale: {path} {} {}", row.class, row.count));
        } else if found != row.count {
            refusals.push(format!(
                "recounted: {path} {}: listed {}, found {found}",
                row.class, row.count
            ));
        }
        if let Err(why) = ruled(root, row) {
            refusals.push(format!("unruled: {path} {}: {why}", row.class));
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals.join("\n"))
    }
}

/// Whether the row's one SPEC file, `docs/specs/SPEC-<n>-*.md` found by name, names its
/// migration's file name (R6).
fn ruled(root: &Path, row: &Admitted) -> Result<(), String> {
    let prefix = format!("{}-", row.spec);
    let unreadable = |error: std::io::Error| format!("cannot read docs/specs/: {error}");
    let mut files = Vec::new();
    for entry in fs::read_dir(root.join("docs/specs")).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) && Path::new(&name).extension().is_some_and(|ext| ext == "md")
        {
            files.push(entry.path());
        }
    }
    let [file] = files.as_slice() else {
        return Err(format!(
            "docs/specs/{prefix}*.md matches {} files, not one",
            files.len()
        ));
    };
    let text =
        fs::read_to_string(file).map_err(|error| format!("cannot read {prefix}*.md: {error}"))?;
    if !text.contains(row.migration) {
        return Err(format!("{} does not name {}", row.spec, row.migration));
    }
    Ok(())
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The removal verb, for a plant, joined from the same two parts the census joins (R5).
fn removal() -> String {
    ["DR", "OP"].concat()
}

/// A temporary tree holding `migrations/` with each planted migration and `docs/specs/` with each
/// planted SPEC.
fn planted_tree(migrations: &[(&str, String)], specs: &[(&str, &str)]) -> tempfile::TempDir {
    let tree = tempfile::tempdir().expect("a temporary directory");
    let migrations_dir = tree.path().join("migrations");
    fs::create_dir_all(&migrations_dir).expect("migrations/ is made");
    for (name, text) in migrations {
        fs::write(migrations_dir.join(name), text).expect("a planted migration is written");
    }
    let specs_dir = tree.path().join("docs/specs");
    fs::create_dir_all(&specs_dir).expect("docs/specs/ is made");
    for (name, text) in specs {
        fs::write(specs_dir.join(name), text).expect("a planted SPEC is written");
    }
    tree
}

/// The census's hits by migration and class, with how many each pair holds.
fn found(census: &Census) -> BTreeMap<(String, String), usize> {
    let mut found = BTreeMap::new();
    for hit in &census.hits {
        *found
            .entry((hit.path.clone(), hit.class.to_owned()))
            .or_default() += 1;
    }
    found
}

/// The register's rows by migration and class, with the count each row lists.
fn listed(register: &[Admitted]) -> BTreeMap<(String, String), usize> {
    register
        .iter()
        .map(|row| {
            let path = format!("migrations/{}", row.migration);
            ((path, row.class.to_owned()), row.count)
        })
        .collect()
}

#[test]
fn every_destructive_class_is_found_in_a_planted_migration() {
    let verb = removal();
    let plants = [
        (
            "000001_removes_a_table.sql",
            format!("{verb} TABLE planted;\n"),
        ),
        (
            "000002_removes_a_column.sql",
            format!("ALTER TABLE planted {verb} COLUMN note;\n"),
        ),
        (
            "000003_removes_an_index.sql",
            format!("{verb} INDEX planted_note;\n"),
        ),
        (
            "000004_removes_a_view.sql",
            format!("{verb} VIEW planted_view;\n"),
        ),
        (
            "000005_removes_a_trigger.sql",
            format!("{verb} TRIGGER planted_trigger;\n"),
        ),
        (
            "000006_renames.sql",
            "ALTER TABLE planted RENAME TO moved;\n".to_owned(),
        ),
        (
            "000007_deletes_rows_by_delete_from.sql",
            "delete from planted where id = 1;\n".to_owned(),
        ),
        (
            "000008_deletes_rows_by_or_replace.sql",
            "INSERT OR REPLACE INTO planted (id) VALUES (1);\n".to_owned(),
        ),
        (
            "000009_deletes_rows_by_replace_into.sql",
            "REPLACE INTO planted (id) VALUES (1);\n".to_owned(),
        ),
        (
            "000010_additive.sql",
            "CREATE TABLE planted (id INTEGER PRIMARY KEY, rename TEXT) STRICT;\n\
             ALTER TABLE planted ADD COLUMN note TEXT;\n\
             CREATE INDEX planted_note ON planted (note);\n"
                .to_owned(),
        ),
    ];
    let tree = planted_tree(&plants, &[]);
    let census = census(tree.path()).expect("the census reads the planted migrations");
    let hits: Vec<(&str, &str)> = census
        .hits
        .iter()
        .map(|hit| (hit.path.as_str(), hit.class))
        .collect();

    assert_eq!(
        hits,
        [
            ("migrations/000001_removes_a_table.sql", "removes a table"),
            ("migrations/000002_removes_a_column.sql", "removes a column"),
            ("migrations/000003_removes_an_index.sql", "removes an index"),
            ("migrations/000004_removes_a_view.sql", "removes a view"),
            (
                "migrations/000005_removes_a_trigger.sql",
                "removes a trigger"
            ),
            ("migrations/000006_renames.sql", "renames"),
            (
                "migrations/000007_deletes_rows_by_delete_from.sql",
                "deletes rows"
            ),
            (
                "migrations/000008_deletes_rows_by_or_replace.sql",
                "deletes rows"
            ),
            (
                "migrations/000009_deletes_rows_by_replace_into.sql",
                "deletes rows"
            ),
        ],
        "each planted class is found in its own migration, and nothing in the additive control"
    );
    assert_eq!(census.examined, 10, "every planted migration is read");
}

#[test]
fn a_comment_or_a_literal_is_no_hit_and_a_split_statement_is_one() {
    let verb = removal();
    let text = format!(
        "-- {verb} TABLE in_a_line_comment;\n\
         /* {verb} TABLE in_a_block_comment; */\n\
         INSERT INTO planted (note) VALUES ('{verb} TABLE in_a_literal; it''s quoted');\n\
         CREATE TABLE \"{verb} TABLE in_an_identifier\" (id INTEGER PRIMARY KEY) STRICT;\n\
         {verb}\n  TABLE\n  split_over_three_lines;\n"
    );
    let tree = planted_tree(&[("000001_comments_and_literals.sql", text)], &[]);
    let census = census(tree.path()).expect("the census reads the planted migration");
    let hits: Vec<(&str, usize, &str)> = census
        .hits
        .iter()
        .map(|hit| (hit.path.as_str(), hit.line, hit.class))
        .collect();

    assert_eq!(
        hits,
        [(
            "migrations/000001_comments_and_literals.sql",
            5,
            "removes a table"
        )],
        "only the statement split over lines 5 to 7 is a hit, at its first line"
    );
    assert_eq!(census.examined, 1, "the planted migration is read");
}

#[test]
fn an_unlisted_recounted_unruled_or_stale_admission_is_refused() {
    let tree = planted_tree(
        &[],
        &[
            (
                "SPEC-900-names-the-rebuild.md",
                "The rebuild `migrations/000001_rebuild.sql` is admitted.\n",
            ),
            ("SPEC-901-omits-it.md", "This SPEC names no migration.\n"),
            ("SPEC-903-one-half.md", "`migrations/000001_rebuild.sql`\n"),
            (
                "SPEC-903-the-other-half.md",
                "`migrations/000001_rebuild.sql`\n",
            ),
        ],
    );
    let census = Census {
        examined: 1,
        hits: vec![
            Hit {
                path: "migrations/000001_rebuild.sql".to_owned(),
                line: 2,
                class: "removes a table",
            },
            Hit {
                path: "migrations/000001_rebuild.sql".to_owned(),
                line: 3,
                class: "renames",
            },
        ],
    };
    let row = |class: &'static str, count: usize, spec: &'static str| Admitted {
        migration: "000001_rebuild.sql",
        class,
        count,
        spec,
        reason: "a planted rebuild",
    };
    let refusal = |register: &[Admitted]| judge(tree.path(), &census, register).err();

    assert_eq!(
        judge(
            tree.path(),
            &census,
            &[
                row("removes a table", 1, "SPEC-900"),
                row("renames", 1, "SPEC-900")
            ]
        ),
        Ok(()),
        "the positive control: every hit listed, counted and ruled"
    );
    assert_eq!(
        refusal(&[row("removes a table", 1, "SPEC-900")]).as_deref(),
        Some("unlisted: migrations/000001_rebuild.sql:3 renames"),
        "a hit no row lists"
    );
    assert_eq!(
        refusal(&[
            row("removes a table", 2, "SPEC-900"),
            row("renames", 1, "SPEC-900")
        ])
        .as_deref(),
        Some("recounted: migrations/000001_rebuild.sql removes a table: listed 2, found 1"),
        "a row whose count differs"
    );
    assert_eq!(
        refusal(&[
            row("removes a table", 1, "SPEC-901"),
            row("renames", 1, "SPEC-900")
        ])
        .as_deref(),
        Some(
            "unruled: migrations/000001_rebuild.sql removes a table: \
             SPEC-901 does not name 000001_rebuild.sql"
        ),
        "a row whose SPEC omits the migration's file name"
    );
    assert_eq!(
        refusal(&[
            row("removes a table", 1, "SPEC-900"),
            row("renames", 1, "SPEC-900"),
            row("deletes rows", 1, "SPEC-900"),
        ])
        .as_deref(),
        Some("stale: migrations/000001_rebuild.sql deletes rows 1"),
        "a row the census does not reach"
    );
    assert_eq!(
        refusal(&[
            row("removes a table", 1, "SPEC-902"),
            row("renames", 1, "SPEC-903")
        ])
        .as_deref(),
        Some(
            "unruled: migrations/000001_rebuild.sql removes a table: \
             docs/specs/SPEC-902-*.md matches 0 files, not one\n\
             unruled: migrations/000001_rebuild.sql renames: \
             docs/specs/SPEC-903-*.md matches 2 files, not one"
        ),
        "a row whose SPEC is no file, or two"
    );
}

#[test]
fn every_destructive_migration_on_the_tree_is_admitted_by_its_ruling() {
    let root = root();
    let census = census(&root).expect("the census reads migrations/");

    assert_eq!(
        judge(&root, &census, &ADMITTED),
        Ok(()),
        "every hit is listed, counted and ruled by the SPEC its row names"
    );
    assert_eq!(found(&census), listed(&ADMITTED), "the hits equal ADMITTED");
    assert_eq!(census.examined, 33, "migrations/ holds 33 files");
}

#[test]
fn an_empty_migration_directory_is_void() {
    let empty = planted_tree(&[], &[]);

    assert_eq!(
        census(empty.path()).err().as_deref(),
        Some("examined 0 migration(s): the population is empty, which is VOID and never a pass"),
        "an empty migrations/ is refused"
    );
    let one = planted_tree(
        &[(
            "000001_additive.sql",
            "CREATE TABLE planted (id INTEGER PRIMARY KEY) STRICT;\n".to_owned(),
        )],
        &[],
    );
    let census = census(one.path()).expect("the census reads one migration");
    assert_eq!(
        (census.examined, census.hits.len()),
        (1, 0),
        "one additive migration is read and is no hit"
    );
}
