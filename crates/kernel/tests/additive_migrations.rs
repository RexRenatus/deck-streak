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

/// The census of `migrations/` under `root` (R1 to R5, R7).
#[expect(
    clippy::unnecessary_wraps,
    reason = "the red commit's stub: it lists and reads nothing, finds nothing and refuses nothing"
)]
fn census(root: &Path) -> Result<Census, String> {
    let _ = root;
    Ok(Census::default())
}

/// Holds the census's hits equal to `register`, and each row's one SPEC file to the migration it
/// names (R6). A refusal names its reason on each of its lines.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the red commit's stub: it accepts whatever it is given"
)]
fn judge(root: &Path, census: &Census, register: &[Admitted]) -> Result<(), String> {
    let _ = (root, census, register);
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
