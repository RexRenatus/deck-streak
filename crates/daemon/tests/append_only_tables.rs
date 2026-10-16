//! The append-only tables take only their sanctioned writes (SPEC-432 R3 to R6; ADR-446 D2).
//!
//! The register names each table a migration's header declares written once. The census reads
//! every string literal of every `.rs` file under `crates/*/src/`, decoded by the shared lexer,
//! and every `migrations/*.sql` file, its SQL comments dropped, and finds each `UPDATE` (with or
//! without `OR <action>`), `DELETE FROM`, `REPLACE INTO`, `INSERT OR REPLACE INTO` and upsert of
//! a registered table, written bare, quoted, bracketed or `main.`-qualified, in any case and as a
//! whole word. A write it cannot read fails closed. The found rewrites equal the sanctioned list,
//! counts included. Every walk prints how many it examined and refuses zero (R9).

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

// The shared reader of every crate's literals (SPEC-324 R1 to R5), included by path.
#[path = "../../../tools/table-census/table_census.rs"]
mod table_census;

use table_census::Token;

/// The register (SPEC-432 R3): each append-only table, the context that owns it and the migration
/// whose header declares it written once.
const REGISTER: [(&str, &str, &str); 4] = [
    (
        "coin_ledger",
        "economy",
        "migrations/008201_economy_wallet_and_shop.sql",
    ),
    (
        "xp_ledger",
        "progression",
        "migrations/004001_progression_xp_ledger.sql",
    ),
    (
        "badges_earned",
        "progression",
        "migrations/007301_progression_badges_earned.sql",
    ),
    (
        "band_milestones",
        "curriculum",
        "migrations/007701_curriculum_road_to_c2_and_law.sql",
    ),
];

/// The sanctioned rewrites (SPEC-432 R6): the file under `crates/`, the item, the kind, the table
/// and how many statements write it, each the sanction its migration names.
const SANCTIONED: [(&str, &str, &str, &str, usize); 7] = [
    (
        "economy/src/wallet.rs",
        "settle_mint_on",
        UPDATE,
        "coin_ledger",
        1,
    ),
    (
        "economy/src/data_rights.rs",
        "erase",
        DELETE,
        "coin_ledger",
        1,
    ),
    (
        "progression/src/data_rights.rs",
        "erase",
        DELETE,
        "xp_ledger",
        1,
    ),
    (
        "progression/src/data_rights.rs",
        "erase",
        DELETE,
        "badges_earned",
        1,
    ),
    (
        "curriculum/src/data_rights.rs",
        "erase",
        DELETE,
        "band_milestones",
        1,
    ),
    (
        "coordination/src/recompute/badges.rs",
        "offer_badges",
        UPDATE,
        "badges_earned",
        1,
    ),
    (
        "curriculum/src/store.rs",
        "mark_band_up",
        UPDATE,
        "band_milestones",
        1,
    ),
];

/// A rewrite by `UPDATE`.
const UPDATE: &str = "UPDATE";
/// A rewrite by `UPDATE OR <action>`.
const UPDATE_OR: &str = "UPDATE OR";
/// A rewrite by `DELETE FROM`.
const DELETE: &str = "DELETE";
/// A rewrite by `REPLACE INTO`.
const REPLACE_INTO: &str = "REPLACE INTO";
/// A rewrite by `INSERT OR REPLACE INTO`.
const INSERT_OR_REPLACE: &str = "INSERT OR REPLACE";
/// A rewrite by an `INSERT ... INTO` whose statement holds `DO UPDATE`.
const UPSERT: &str = "UPSERT";

/// The keywords whose next identifier names the item a literal sits in (ADR-446 D1).
const ITEM_KEYWORDS: [&str; 7] = ["fn", "const", "static", "type", "struct", "enum", "trait"];

/// The name a literal takes when no item keyword comes before it in its file.
const NO_ITEM: &str = "(no item)";

/// The file a planted migration is written to.
const PLANTED_MIGRATION: &str = "migrations/000001_planted.sql";

/// One token of SQL text.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Sql {
    /// A name as written: word characters and quoted or bracketed parts, joined by `.`.
    Name(String),
    /// A string literal, whose content is data.
    Text,
    /// Any other character.
    Mark(char),
}

/// A rewrite's key: the file, the item, the kind and the table.
type Site = (String, String, String, String);

/// What the census read under one root: every source, literal, migration and statement, each
/// rewrite of a registered table with how many statements make it, and each write it cannot read.
struct Census {
    sources: Vec<String>,
    literals: usize,
    migrations: Vec<String>,
    statements: usize,
    found: BTreeMap<Site, usize>,
    unreadable: Vec<String>,
}

/// Prints how many items a walk examined and refuses zero (the tdd pack's examined contract).
fn examined(what: &str, count: usize) -> usize {
    println!("examined {count} {what}");
    assert!(
        count > 0,
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    count
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `directory` whose name ends in `extension`, recursively, sorted by path.
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

/// `path` under `base`, with `/` between its parts.
fn relative(base: &Path, path: &Path) -> String {
    let inside = path.strip_prefix(base).expect("a path under its base");
    inside
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Writes `text` at `path` under `root`, making its directories.
fn plant(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("a planted file has a directory"))
        .expect("the planted directory is writable");
    fs::write(&path, text).expect("the planted file is writable");
}

/// The registered tables' names.
fn tables() -> Vec<&'static str> {
    REGISTER.iter().map(|(table, _, _)| *table).collect()
}

/// Whether `word` could be a Rust identifier.
fn identifier(word: &str) -> bool {
    let mut characters = word.chars();
    characters
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && characters.all(|inside| inside.is_alphanumeric() || inside == '_')
}

/// For each token, the item that holds it: the identifier after the last `fn`, `const`,
/// `static`, `type`, `struct`, `enum` or `trait` keyword before it (ADR-446 D1). A lifetime's
/// `'static` is no keyword.
fn items(tokens: &[Token]) -> Vec<Option<String>> {
    let word = |at: usize| match tokens.get(at) {
        Some(Token::Word(text)) => Some(text.as_str()),
        _ => None,
    };
    let mut current: Option<String> = None;
    let mut found = Vec::with_capacity(tokens.len());
    for at in 0..tokens.len() {
        let lifetime = at > 0 && tokens.get(at - 1) == Some(&Token::Punct('\''));
        let declares = word(at).is_some_and(|text| ITEM_KEYWORDS.contains(&text)) && !lifetime;
        if declares
            && let Some(name) =
                word(at + 1).filter(|name| identifier(name) && !ITEM_KEYWORDS.contains(name))
        {
            current = Some(name.to_owned());
        }
        found.push(current.clone());
    }
    found
}

/// The index after the quote that opens at `at` and closes with `close`. A doubled closing quote
/// inside is an escape; an unclosed quote runs to the end.
fn past_quote(characters: &[char], at: usize, close: char) -> usize {
    let mut index = at + 1;
    while let Some(&inside) = characters.get(index) {
        index += 1;
        if inside == close {
            if close != ']' && characters.get(index) == Some(&close) {
                index += 1;
                continue;
            }
            return index;
        }
    }
    index
}

/// SQL text with its `--` and `/* */` comments dropped; a quoted span is kept whole.
fn without_comments(sql: &str) -> String {
    let characters: Vec<char> = sql.chars().collect();
    let mut kept = String::with_capacity(sql.len());
    let mut at = 0;
    while let Some(&found) = characters.get(at) {
        let next = characters.get(at + 1).copied();
        let opens_comment =
            (found == '-' && next == Some('-')) || (found == '/' && next == Some('*'));
        if opens_comment {
            at = if found == '-' {
                (at..characters.len())
                    .find(|&index| characters[index] == '\n')
                    .unwrap_or(characters.len())
            } else {
                (at + 2..characters.len())
                    .find(|&index| characters[index - 1] == '*' && characters[index] == '/')
                    .map_or(characters.len(), |index| index + 1)
            };
            kept.push(' ');
            continue;
        }
        let close = match found {
            '\'' | '"' | '`' => Some(found),
            '[' => Some(']'),
            _ => None,
        };
        let end = close.map_or(at + 1, |close| past_quote(&characters, at, close));
        kept.extend(&characters[at..end]);
        at = end;
    }
    kept
}

/// Whether `character` belongs to a SQL word.
fn word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

/// The index after one part of a name at `at`: a run of word characters, or a quoted or
/// bracketed part.
fn part_end(characters: &[char], at: usize) -> Option<usize> {
    let found = *characters.get(at)?;
    match found {
        '"' | '`' => Some(past_quote(characters, at, found)),
        '[' => Some(past_quote(characters, at, ']')),
        _ if word_character(found) => {
            let mut end = at;
            while characters.get(end).copied().is_some_and(word_character) {
                end += 1;
            }
            Some(end)
        }
        _ => None,
    }
}

/// SQL text as tokens. A name runs over its parts and the `.` between them, so `main.t` and
/// `excluded.c` are one name each.
fn sql_tokens(sql: &str) -> Vec<Sql> {
    let characters: Vec<char> = sql.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(&found) = characters.get(at) {
        if found.is_whitespace() {
            at += 1;
        } else if found == '\'' {
            at = past_quote(&characters, at, '\'');
            tokens.push(Sql::Text);
        } else if let Some(mut end) = part_end(&characters, at) {
            while characters.get(end) == Some(&'.')
                && let Some(next) = part_end(&characters, end + 1)
            {
                end = next;
            }
            tokens.push(Sql::Name(characters[at..end].iter().collect()));
            at = end;
        } else {
            tokens.push(Sql::Mark(found));
            at += 1;
        }
    }
    tokens
}

/// SQL text, its comments dropped, as statements split at each `;`.
fn statements(sql: &str) -> Vec<Vec<Sql>> {
    sql_tokens(&without_comments(sql))
        .split(|token| *token == Sql::Mark(';'))
        .filter(|statement| !statement.is_empty())
        .map(<[Sql]>::to_vec)
        .collect()
}

/// The rewrites of a registered table one statement makes, each with its kind and its table.
fn rewrites(statement: &[Sql], tables: &[&str]) -> Vec<(&'static str, String)> {
    let _ = (statement, tables);
    Vec::new()
}

/// Why a literal holds a write the check cannot read, if it does.
fn unreadable(literal: &str) -> Option<&'static str> {
    let _ = literal;
    None
}

/// Whether a word of Rust source is one of sqlx's macros that read their SQL from a file.
fn reads_sql_from_a_file(word: &str) -> bool {
    let _ = word;
    false
}

/// Reads every string literal of every `.rs` file under `crates/*/src/` of `root`.
fn read_sources(root: &Path, tables: &[&str], census: &mut Census) {
    let crates = root.join("crates");
    let mut members: Vec<PathBuf> = fs::read_dir(&crates)
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    for member in members {
        for path in files(&member.join("src"), "rs") {
            let file = relative(&crates, &path);
            let text = fs::read_to_string(&path).expect("a readable source");
            let tokens: Vec<Token> = table_census::lex(&text)
                .into_iter()
                .map(|(token, _)| token)
                .collect();
            let named = items(&tokens);
            for (at, token) in tokens.iter().enumerate() {
                let item = || named[at].clone().unwrap_or_else(|| NO_ITEM.to_owned());
                match token {
                    Token::Literal(value) => {
                        census.literals += 1;
                        if let Some(why) = unreadable(value) {
                            census.unreadable.push(format!("{file} {}: {why}", item()));
                        }
                        for statement in statements(value) {
                            census.statements += 1;
                            for (kind, table) in rewrites(&statement, tables) {
                                let site = (file.clone(), item(), kind.to_owned(), table);
                                *census.found.entry(site).or_insert(0) += 1;
                            }
                        }
                    }
                    Token::Word(word) if reads_sql_from_a_file(word) => {
                        census
                            .unreadable
                            .push(format!("{file} {}: a sqlx query_file macro", item()));
                    }
                    _ => {}
                }
            }
            census.sources.push(file);
        }
    }
}

/// Reads every `migrations/*.sql` file of `root`, each statement named by its place in the file.
fn read_migrations(root: &Path, tables: &[&str], census: &mut Census) {
    let directory = root.join("migrations");
    let mut paths: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("migrations/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|name| name == "sql"))
        .collect();
    paths.sort();
    for path in paths {
        let file = format!("migrations/{}", relative(&directory, &path));
        let text = fs::read_to_string(&path).expect("a readable migration");
        for (index, statement) in statements(&text).into_iter().enumerate() {
            census.statements += 1;
            let item = format!("statement {}", index + 1);
            for (kind, table) in rewrites(&statement, tables) {
                let site = (file.clone(), item.clone(), kind.to_owned(), table);
                *census.found.entry(site).or_insert(0) += 1;
            }
        }
        census.migrations.push(file);
    }
}

/// The census of a tree at `root` (ADR-446 D2).
fn census(root: &Path, tables: &[&str]) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        literals: 0,
        migrations: Vec::new(),
        statements: 0,
        found: BTreeMap::new(),
        unreadable: Vec::new(),
    };
    read_sources(root, tables, &mut census);
    read_migrations(root, tables, &mut census);
    census
}

/// The judge (SPEC-432 R5, R6): the found rewrites must equal the sanctioned ones.
fn judge(census: &Census, sanctioned: &[(&str, &str, &str, &str, usize)]) -> Vec<String> {
    let _ = (census, sanctioned);
    Vec::new()
}

/// Prints the census's examined counts, refusing any at zero.
fn examined_all(census: &Census) {
    examined("source file(s)", census.sources.len());
    examined("literal(s)", census.literals);
    examined("migration(s)", census.migrations.len());
    examined("statement(s)", census.statements);
}

/// Each kind of rewrite with a statement of it, `{}` standing for the table as spelled.
const KINDS: [(&str, &str); 6] = [
    (UPDATE, "UPDATE {} SET delta = 0 WHERE id = ?1"),
    (UPDATE_OR, "UPDATE OR IGNORE {} SET delta = 0 WHERE id = ?1"),
    (DELETE, "DELETE FROM {} WHERE id = ?1"),
    (REPLACE_INTO, "REPLACE INTO {} (id, delta) VALUES (?1, 0)"),
    (
        INSERT_OR_REPLACE,
        "INSERT OR REPLACE INTO {} (id, delta) VALUES (?1, 0)",
    ),
    (
        UPSERT,
        "INSERT INTO {} (id, delta) VALUES (?1, 0) ON CONFLICT (id) DO UPDATE SET delta = excluded.delta",
    ),
];

/// The spellings of a table a statement may write.
const SPELLINGS: [&str; 6] = [
    "bare",
    "double_quoted",
    "backquoted",
    "bracketed",
    "qualified",
    "lower_case",
];

/// `statement` with its table written in `spelling`.
fn spelled(statement: &str, spelling: &str, table: &str) -> String {
    match spelling {
        "double_quoted" => statement.replace("{}", &format!("\"{table}\"")),
        "backquoted" => statement.replace("{}", &format!("`{table}`")),
        "bracketed" => statement.replace("{}", &format!("[{table}]")),
        "qualified" => statement.replace("{}", &format!("main.{table}")),
        "lower_case" => statement.replace("{}", table).to_lowercase(),
        _ => statement.replace("{}", table),
    }
}

/// `text` as a Rust string literal.
fn rust_literal(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A planted crate holding statements that read, key or name another table, and rewrites only
/// inside comments.
const PLANTED_NOT_REWRITES: &str = r#"pub fn keyed() -> &'static str {
    "INSERT INTO coin_ledger (id, delta) VALUES (?1, 0) ON CONFLICT DO NOTHING"
}

pub fn ignored() -> &'static str {
    "INSERT OR IGNORE INTO xp_ledger (id, amount) VALUES (?1, 0)"
}

pub fn read() -> &'static str {
    "SELECT delta FROM coin_ledger WHERE id = ?1"
}

pub fn extended() -> &'static str {
    "UPDATE coin_ledger_archive SET delta = 0 WHERE id = ?1"
}

pub fn prefixed() -> &'static str {
    "DELETE FROM old_badges_earned WHERE id = ?1"
}

pub fn another_table() -> &'static str {
    "UPDATE wallet_holds SET amount = 0 WHERE id = ?1"
}

// UPDATE coin_ledger SET delta = 0 WHERE id = ?1
/* DELETE FROM xp_ledger */
pub fn sql_commented() -> &'static str {
    "SELECT 1 -- UPDATE band_milestones SET celebrated_at = 0"
}
"#;

/// A planted migration holding a keyed insert, a read, another table and rewrites only inside
/// comments.
const PLANTED_NOT_REWRITES_SQL: &str = "-- UPDATE coin_ledger SET delta = 0;
/* DELETE FROM xp_ledger; */
INSERT INTO badges_earned (badge, tier) VALUES ('streak', 1) ON CONFLICT DO NOTHING;
SELECT celebrated_at FROM band_milestones;
UPDATE band_milestones_seen SET seen = 1;
";

/// A planted crate holding writes no literal shows whole: joined to a placeholder, ending the
/// literal, and read from a file.
const PLANTED_UNREADABLE: &str = r#"pub fn joined_update(table: &str) -> String {
    format!("UPDATE {table} SET delta = 0 WHERE id = ?1")
}

pub fn joined_delete(table: &str) -> String {
    format!("DELETE FROM {table} WHERE id = ?1")
}

pub fn joined_insert(table: &str) -> String {
    format!("INSERT INTO {table} (id, delta) VALUES (?1, 0)")
}

pub fn prefix_update(table: &str) -> String {
    "UPDATE ".to_owned() + table
}

pub fn prefix_delete(table: &str) -> String {
    "DELETE FROM ".to_owned() + table
}

pub fn prefix_insert(table: &str) -> String {
    "INSERT INTO".to_owned() + " " + table
}

pub async fn from_file(db: &sqlx::SqlitePool) {
    sqlx::query_file!("queries/erase.sql").execute(db).await.ok();
}
"#;

/// A planted migration that reads only.
const PLANTED_READ_SQL: &str = "SELECT 1;\n";

#[test]
fn the_append_only_tables_take_only_their_sanctioned_writes() {
    let root = root();
    let mut reached = 0;
    for (table, context, migration) in REGISTER {
        reached += 1;
        assert!(
            root.join(migration).is_file(),
            "the register's {table} ({context}) cites {migration}, which is not in the tree"
        );
    }
    println!("examined {reached} of {} register row(s)", REGISTER.len());
    assert_eq!(
        reached,
        REGISTER.len(),
        "every register row reached its check"
    );
    let census = census(&root, &tables());
    examined_all(&census);
    for ((file, item, kind, table), count) in &census.found {
        println!("rewrite {file} {item} {kind} {table}: {count}");
    }
    let sanctioned: BTreeMap<Site, usize> = SANCTIONED
        .iter()
        .map(|(file, item, kind, table, count)| {
            let site = (
                (*file).to_owned(),
                (*item).to_owned(),
                (*kind).to_owned(),
                (*table).to_owned(),
            );
            (site, *count)
        })
        .collect();
    assert_eq!(
        census.found, sanctioned,
        "the rewrites of the append-only tables are not the sanctioned ones (SPEC-432 R6)"
    );
    assert!(
        census.unreadable.is_empty(),
        "a write the check cannot read: {:#?}",
        census.unreadable
    );
    let refused = judge(&census, &SANCTIONED);
    assert!(refused.is_empty(), "refused at the head: {refused:#?}");
}

#[test]
fn a_planted_rewrite_of_an_append_only_table_is_refused_by_kind() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    let registered = tables();
    let mut source = String::new();
    let mut migration = String::new();
    let mut expected = BTreeSet::new();
    let mut statement_number = 0;
    for (kind_index, (kind, statement)) in KINDS.iter().enumerate() {
        for (spelling_index, spelling) in SPELLINGS.iter().enumerate() {
            let table = registered[(kind_index + spelling_index) % registered.len()];
            let text = spelled(statement, spelling, table);
            let item = format!("rewrite_{kind_index}_{spelling}");
            writeln!(
                source,
                "pub fn {item}() -> &'static str {{\n    {}\n}}\n",
                rust_literal(&text)
            )
            .expect("a String takes any text");
            expected.insert(format!(
                "planted/src/lib.rs {item} {kind} {table}: 1 rewrite(s) of an append-only table, \
                 and none is sanctioned"
            ));
            migration.push_str(&text);
            migration.push_str(";\n");
            statement_number += 1;
            expected.insert(format!(
                "{PLANTED_MIGRATION} statement {statement_number} {kind} {table}: 1 rewrite(s) of \
                 an append-only table, and none is sanctioned"
            ));
        }
    }
    plant(planted.path(), "crates/planted/src/lib.rs", &source);
    plant(planted.path(), PLANTED_MIGRATION, &migration);
    let census = census(planted.path(), &registered);
    examined_all(&census);
    let refused: BTreeSet<String> = judge(&census, &[]).into_iter().collect();
    let mut reached = 0;
    let mut missing = Vec::new();
    for want in &expected {
        reached += 1;
        if !refused.contains(want) {
            missing.push(want.as_str());
        }
    }
    println!(
        "examined {reached} of {} planted rewrite(s)",
        expected.len()
    );
    assert_eq!(
        reached,
        expected.len(),
        "every planted rewrite reached its check"
    );
    assert!(
        missing.is_empty(),
        "a planted rewrite is not refused by file, item, kind and table: {missing:#?}"
    );
    assert_eq!(refused, expected, "only the planted rewrites are refused");
}

#[test]
fn an_insert_a_read_or_another_table_is_not_a_rewrite() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/planted/src/lib.rs",
        PLANTED_NOT_REWRITES,
    );
    plant(planted.path(), PLANTED_MIGRATION, PLANTED_NOT_REWRITES_SQL);
    let census = census(planted.path(), &tables());
    examined_all(&census);
    assert_eq!(census.sources, vec!["planted/src/lib.rs".to_owned()]);
    assert_eq!(census.migrations, vec![PLANTED_MIGRATION.to_owned()]);
    assert_eq!(census.literals, 7, "the census read every planted literal");
    let items = [
        ("planted/src/lib.rs", "keyed"),
        ("planted/src/lib.rs", "ignored"),
        ("planted/src/lib.rs", "read"),
        ("planted/src/lib.rs", "extended"),
        ("planted/src/lib.rs", "prefixed"),
        ("planted/src/lib.rs", "another_table"),
        ("planted/src/lib.rs", "sql_commented"),
        (PLANTED_MIGRATION, "statement 1"),
        (PLANTED_MIGRATION, "statement 2"),
        (PLANTED_MIGRATION, "statement 3"),
    ];
    let mut reached = 0;
    let mut counted = Vec::new();
    for (file, item) in items {
        reached += 1;
        for ((found_file, found_item, kind, table), count) in &census.found {
            if found_file == file && found_item == item {
                counted.push(format!("{file} {item} {kind} {table}: {count}"));
            }
        }
    }
    println!(
        "examined {reached} of {} planted non-rewrite(s)",
        items.len()
    );
    assert_eq!(reached, items.len(), "every planted case reached its check");
    assert!(
        counted.is_empty(),
        "a planted insert, read or other table is counted as a rewrite: {counted:#?}"
    );
    assert!(
        census.found.is_empty(),
        "no planted statement rewrites a registered table: {:#?}",
        census.found
    );
    assert!(
        census.unreadable.is_empty(),
        "every planted statement is readable: {:#?}",
        census.unreadable
    );
    let refused = judge(&census, &[]);
    assert!(refused.is_empty(), "refused: {refused:#?}");
}

#[test]
fn a_write_the_check_cannot_read_fails_closed() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/planted/src/lib.rs",
        PLANTED_UNREADABLE,
    );
    plant(planted.path(), PLANTED_MIGRATION, PLANTED_READ_SQL);
    let census = census(planted.path(), &tables());
    examined_all(&census);
    let refused: BTreeSet<String> = judge(&census, &[]).into_iter().collect();
    let expected: BTreeSet<String> = [
        ("joined_update", "a write keyword joined to a placeholder"),
        ("joined_delete", "a write keyword joined to a placeholder"),
        ("joined_insert", "a write keyword joined to a placeholder"),
        ("prefix_update", "a write keyword ending the literal"),
        ("prefix_delete", "a write keyword ending the literal"),
        ("prefix_insert", "a write keyword ending the literal"),
        ("from_file", "a sqlx query_file macro"),
    ]
    .into_iter()
    .map(|(item, why)| format!("planted/src/lib.rs {item}: {why}, a write the check cannot read"))
    .collect();
    let mut reached = 0;
    let mut missing = Vec::new();
    for want in &expected {
        reached += 1;
        if !refused.contains(want) {
            missing.push(want.as_str());
        }
    }
    println!(
        "examined {reached} of {} unreadable write(s)",
        expected.len()
    );
    assert_eq!(
        reached,
        expected.len(),
        "every unreadable write reached its check"
    );
    assert!(
        missing.is_empty(),
        "an unreadable write is not refused: {missing:#?}\nrefused: {refused:#?}"
    );
    assert_eq!(refused, expected, "only the unreadable writes are refused");
}

#[test]
fn a_sanctioned_write_gone_or_doubled_is_refused() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    let tree = [
        (
            "crates/economy/src/wallet.rs",
            r#"pub fn settle_mint_on() -> [&'static str; 2] {
    [
        "UPDATE coin_ledger SET delta = ?1 WHERE id = ?2",
        "UPDATE coin_ledger SET delta = ?1 WHERE id = ?3",
    ]
}
"#,
        ),
        (
            "crates/economy/src/data_rights.rs",
            r#"fn erase() -> &'static str {
    "DELETE FROM coin_ledger"
}
"#,
        ),
        (
            "crates/economy/src/rogue.rs",
            r#"pub fn rewrite_a_grant() -> &'static str {
    "UPDATE xp_ledger SET amount = 0 WHERE id = ?1"
}
"#,
        ),
        (
            "crates/progression/src/data_rights.rs",
            r#"fn erase() -> &'static str {
    "DELETE FROM xp_ledger"
}
"#,
        ),
        (
            "crates/curriculum/src/data_rights.rs",
            r#"fn erase() -> &'static str {
    "DELETE FROM band_milestones"
}
"#,
        ),
        (
            "crates/curriculum/src/store.rs",
            r#"pub fn mark_band_up() -> &'static str {
    "UPDATE band_milestones SET celebrated_at = ?3 WHERE course = ?1 AND band = ?2"
}
"#,
        ),
        (
            "crates/coordination/src/recompute/badges.rs",
            r#"pub fn offer_badges() -> &'static str {
    "UPDATE badges_earned SET celebrated_at = ?1 WHERE badge = ?2"
}
"#,
        ),
        (PLANTED_MIGRATION, PLANTED_READ_SQL),
    ];
    for (path, text) in tree {
        plant(planted.path(), path, text);
    }
    let census = census(planted.path(), &tables());
    examined_all(&census);
    let refused: BTreeSet<String> = judge(&census, &SANCTIONED).into_iter().collect();
    let expected: BTreeSet<String> = [
        "economy/src/rogue.rs rewrite_a_grant UPDATE xp_ledger: 1 rewrite(s) of an append-only \
         table, and none is sanctioned",
        "economy/src/wallet.rs settle_mint_on UPDATE coin_ledger: 2 rewrite(s), sanctioned 1",
        "progression/src/data_rights.rs erase DELETE badges_earned: sanctioned 1, and none is \
         found",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut reached = 0;
    let mut missing = Vec::new();
    for want in &expected {
        reached += 1;
        if !refused.contains(want) {
            missing.push(want.as_str());
        }
    }
    println!("examined {reached} of {} moved sanction(s)", expected.len());
    assert_eq!(
        reached,
        expected.len(),
        "every moved sanction reached its check"
    );
    assert!(
        missing.is_empty(),
        "a moved sanction is not refused by name: {missing:#?}\nrefused: {refused:#?}"
    );
    assert_eq!(
        refused, expected,
        "the unsanctioned rewrite, the doubled sanction and the sanction gone are refused, and \
         the five kept sanctions are not"
    );
}
