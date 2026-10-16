//! Every sync write names its idempotency (SPEC-432 R7, R8; ADR-446 D3).
//!
//! The census reads every string literal of every `.rs` file under `crates/ingest/src/`, decoded
//! by the shared lexer. A literal whose first SQL word is `INSERT`, `UPDATE`, `DELETE` or `REPLACE`
//! is a write; a `WITH` that holds a write, or a write keyword joined to a `{` placeholder, is a
//! write the check cannot read and is refused. Every module holding a write is a sync module or a
//! named exempt one, and each listed module holds a write. Each write in a sync module is classed
//! KEYED, UPSERT, SET, DELETE or PLAIN; an assignment whose right side names its own column outside
//! `excluded.` is an INCREMENT and is refused; the PLAIN sites equal the listed three. Every walk
//! prints how many it examined and refuses zero (R9).

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

// The shared reader of every crate's literals (SPEC-324 R1 to R5), included by path.
#[path = "../../../tools/table-census/table_census.rs"]
mod table_census;

use table_census::Token;

/// The sync modules (SPEC-432 R7): each write in them is classed.
const SYNC_MODULES: [&str; 3] = ["skip.rs", "state.rs", "sync_runs.rs"];

/// The exempt modules (SPEC-432 R7), each with its reason.
const EXEMPT_MODULES: [(&str, &str); 3] = [
    ("data_rights.rs", "the owner's erase and export"),
    ("sensitive.rs", "the owner's sensitive-deck marks"),
    ("write_class_stop.rs", "the owner's stop"),
];

/// The listed PLAIN sites (SPEC-432 R8): the module, the item and why a second application is
/// absorbed or why it is listed without a key.
const PLAIN: [(&str, &str, &str); 3] = [
    ("skip.rs", "record_prior", "the key refuses a second row"),
    ("sync_runs.rs", "record", "an event record with no key"),
    (
        "sync_runs.rs",
        "record_skipped",
        "an event record with no key",
    ),
];

/// The first SQL words of a write (ADR-446 D3).
const WRITE_WORDS: [&str; 4] = ["insert", "update", "delete", "replace"];

/// The keywords whose next identifier names the item a literal sits in (ADR-446 D1).
const ITEM_KEYWORDS: [&str; 7] = ["fn", "const", "static", "type", "struct", "enum", "trait"];

/// The name a literal takes when no item keyword comes before it in its file.
const NO_ITEM: &str = "(no item)";

/// How a second application of a write is absorbed (ADR-446 D3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    /// `ON CONFLICT ... DO NOTHING` or `INSERT OR IGNORE`: the key refuses the second row.
    Keyed,
    /// `DO UPDATE SET`, `REPLACE` or `INSERT OR REPLACE`: the second write sets the same row.
    Upsert,
    /// An `UPDATE`: the second write sets the same values.
    Set,
    /// A `DELETE`: the second finds nothing.
    Delete,
    /// An `INSERT` with no clause: nothing in the statement absorbs a second application.
    Plain,
}

impl Class {
    /// Every class, in the order the census prints them.
    const ALL: [Self; 5] = [
        Self::Keyed,
        Self::Upsert,
        Self::Set,
        Self::Delete,
        Self::Plain,
    ];

    /// The class's name as a refusal and a print write it.
    fn label(self) -> &'static str {
        match self {
            Self::Keyed => "KEYED",
            Self::Upsert => "UPSERT",
            Self::Set => "SET",
            Self::Delete => "DELETE",
            Self::Plain => "PLAIN",
        }
    }
}

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

/// One write the census found: its module, its item, its class and every self-referencing
/// assignment it makes.
struct Write {
    module: String,
    item: String,
    class: Option<Class>,
    increments: Vec<String>,
}

/// What the census read under one root: every module, how many literals, every write and each
/// write it cannot read with why.
struct Census {
    modules: Vec<String>,
    literals: usize,
    writes: Vec<Write>,
    unreadable: Vec<(String, String, &'static str)>,
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

/// Every `.rs` file under `directory`, recursively, sorted by path.
fn files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
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

/// Writes each module's text under `crates/ingest/src/` of `root`, making its directories.
fn plant(root: &Path, modules: &[(&str, &str)]) {
    for (module, text) in modules {
        let path = root.join("crates/ingest/src").join(module);
        fs::create_dir_all(path.parent().expect("a planted file has a directory"))
            .expect("the planted directory is writable");
        fs::write(&path, text).expect("the planted file is writable");
    }
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

/// The statement's first word, lower-cased, when it opens with a name.
fn first_word(statement: &[Sql]) -> Option<String> {
    match statement.first() {
        Some(Sql::Name(name)) => Some(name.to_ascii_lowercase()),
        _ => None,
    }
}

/// How a second application of the write is absorbed, by ADR-446 D3.
fn class_of(statement: &[Sql]) -> Option<Class> {
    let _ = statement;
    None
}

/// Each assignment of a `SET` list whose right side names its own column outside `excluded.`.
fn increments(statement: &[Sql]) -> Vec<String> {
    let _ = statement;
    Vec::new()
}

/// Why a literal holds a write the check cannot read, if it does.
fn unreadable(statement: &[Sql]) -> Option<&'static str> {
    let _ = statement;
    None
}

/// Reads every string literal of every `.rs` file under `crates/ingest/src/` of `root`.
fn census(root: &Path) -> Census {
    let source = root.join("crates/ingest/src");
    let mut census = Census {
        modules: Vec::new(),
        literals: 0,
        writes: Vec::new(),
        unreadable: Vec::new(),
    };
    for path in files(&source) {
        let module = relative(&source, &path);
        let text = fs::read_to_string(&path).expect("a readable source");
        let tokens: Vec<Token> = table_census::lex(&text)
            .into_iter()
            .map(|(token, _)| token)
            .collect();
        let named = items(&tokens);
        for (at, token) in tokens.iter().enumerate() {
            let Token::Literal(value) = token else {
                continue;
            };
            census.literals += 1;
            let item = named[at].clone().unwrap_or_else(|| NO_ITEM.to_owned());
            let statement = sql_tokens(&without_comments(value));
            if let Some(why) = unreadable(&statement) {
                census.unreadable.push((module.clone(), item, why));
                continue;
            }
            let write =
                first_word(&statement).is_some_and(|word| WRITE_WORDS.contains(&word.as_str()));
            if write {
                census.writes.push(Write {
                    module: module.clone(),
                    item,
                    class: class_of(&statement),
                    increments: increments(&statement),
                });
            }
        }
        census.modules.push(module);
    }
    census
}

/// The judge (SPEC-432 R7, R8): unreadable writes, unclassified modules, listed modules holding no
/// write, INCREMENTs and PLAIN sites that are not the listed ones are each refused by name.
fn judge(census: &Census) -> Vec<String> {
    let _ = census;
    Vec::new()
}

/// The writes of the census that sit in a sync module.
fn sync_writes(census: &Census) -> Vec<&Write> {
    census
        .writes
        .iter()
        .filter(|write| SYNC_MODULES.contains(&write.module.as_str()))
        .collect()
}

/// The listed modules as a planted tree holds them: `state.rs` upserts, `sync_runs.rs` records its
/// two events, and each exempt module writes.
const PLANTED_LISTED: [(&str, &str); 5] = [
    (
        "state.rs",
        r#"pub fn write_window(floor: i64) -> &'static str {
    "INSERT INTO ingest_state (id, window_floor) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET window_floor = excluded.window_floor"
}
"#,
    ),
    (
        "sync_runs.rs",
        r#"pub fn record() -> &'static str {
    "INSERT INTO sync_runs (trigger, status) VALUES (?1, ?2)"
}

pub fn record_skipped() -> &'static str {
    "INSERT INTO sync_runs (trigger, status) VALUES (?1, 'skipped')"
}
"#,
    ),
    (
        "data_rights.rs",
        r#"pub fn erase() -> &'static str {
    "DELETE FROM sync_runs"
}
"#,
    ),
    (
        "sensitive.rs",
        r#"pub fn mark() -> &'static str {
    "INSERT INTO sensitive_decks (deck_id) VALUES (?1) ON CONFLICT (deck_id) DO NOTHING"
}
"#,
    ),
    (
        "write_class_stop.rs",
        r#"pub fn stop() -> &'static str {
    "UPDATE write_class_stop SET stopped = 1"
}
"#,
    ),
];

/// A planted `skip.rs` holding the listed PLAIN site and writes that can apply twice.
const PLANTED_TWICE: &str = r#"pub fn record_prior() -> &'static str {
    "INSERT INTO skip_card_snapshot (skip_id, card_id) VALUES (?1, ?2)"
}

pub fn record_twice() -> &'static str {
    "INSERT INTO skip_days (study_day) VALUES (?1)"
}

pub fn bump_count() -> &'static str {
    "UPDATE skip_days SET due_count = due_count + 1 WHERE id = ?1"
}

pub fn bump_window() -> &'static str {
    "INSERT INTO ingest_state (id, window_count) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET window_count = ingest_state.window_count + excluded.window_count"
}

pub fn build_with() -> &'static str {
    "WITH due AS (SELECT id FROM skip_days) DELETE FROM skip_days WHERE id IN (SELECT id FROM due)"
}

pub fn build_joined(table: &str) -> String {
    format!("UPDATE {table} SET state = 'failed'")
}
"#;

/// A planted `skip.rs` holding the listed PLAIN site and one write of each idempotent shape.
const PLANTED_IDEMPOTENT: &str = r#"pub fn record_prior() -> &'static str {
    "INSERT INTO skip_card_snapshot (skip_id, card_id) VALUES (?1, ?2)"
}

pub fn begin() -> &'static str {
    "INSERT INTO skip_days (study_day) VALUES (?1) ON CONFLICT (study_day) DO NOTHING"
}

pub fn mark() -> &'static str {
    "INSERT OR IGNORE INTO skip_marks (study_day) VALUES (?1)"
}

pub fn write_anchor() -> &'static str {
    "INSERT INTO ingest_state (id, anchor) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET anchor = excluded.anchor"
}

pub fn settle() -> &'static str {
    "UPDATE skip_days SET state = 'applied', cards_moved = ?2 WHERE id = ?1 AND state = 'pending'"
}

pub fn forget() -> &'static str {
    "DELETE FROM skip_days WHERE id = ?1"
}
"#;

/// A planted `skip.rs` whose listed PLAIN site is gone.
const PLANTED_SKIP_WITHOUT_PRIOR: &str = r#"pub fn begin() -> &'static str {
    "INSERT INTO skip_days (study_day) VALUES (?1) ON CONFLICT (study_day) DO NOTHING"
}
"#;

/// A planted `state.rs` that reads and writes nothing.
const PLANTED_STATE_READ: &str = r#"pub fn read() -> &'static str {
    "SELECT window_floor FROM ingest_state WHERE id = 1"
}
"#;

/// A planted module neither sync nor exempt, holding a write.
const PLANTED_ROGUE: &str = r#"pub fn stash() -> &'static str {
    "INSERT INTO rogue_rows (id) VALUES (?1) ON CONFLICT DO NOTHING"
}
"#;

/// The census of a tree holding the listed modules beside `skip.rs` and any `others`.
fn planted_census(skip: &str, others: &[(&str, &str)]) -> (tempfile::TempDir, Census) {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(planted.path(), &PLANTED_LISTED);
    plant(planted.path(), &[("skip.rs", skip)]);
    plant(planted.path(), others);
    let census = census(planted.path());
    (planted, census)
}

/// Prints the census's examined counts, refusing any at zero.
fn examined_all(census: &Census) {
    examined("module(s)", census.modules.len());
    examined("literal(s)", census.literals);
    examined("sync write(s)", sync_writes(census).len());
}

#[test]
fn every_sync_write_names_its_idempotency() {
    let census = census(&root());
    examined_all(&census);
    let writes = sync_writes(&census);
    let mut holding: BTreeMap<&str, usize> = BTreeMap::new();
    for write in &census.writes {
        *holding.entry(write.module.as_str()).or_insert(0) += 1;
    }
    for (module, count) in &holding {
        let kind = if SYNC_MODULES.contains(module) {
            "sync".to_owned()
        } else if let Some((_, reason)) = EXEMPT_MODULES.iter().find(|(name, _)| name == module) {
            format!("exempt, {reason}")
        } else {
            "unclassified".to_owned()
        };
        println!("write module {module}: {count} write(s), {kind}");
    }
    let counts: Vec<String> = Class::ALL
        .iter()
        .map(|class| {
            let count = writes
                .iter()
                .filter(|write| write.class == Some(*class))
                .count();
            format!("{} {count}", class.label())
        })
        .collect();
    let unclassed = writes.iter().filter(|write| write.class.is_none()).count();
    let increment_count: usize = writes.iter().map(|write| write.increments.len()).sum();
    println!(
        "sync writes by class: {}, INCREMENT {increment_count}, unclassed {unclassed}",
        counts.join(", ")
    );
    let unclassed: Vec<String> = writes
        .iter()
        .filter(|write| write.class.is_none())
        .map(|write| format!("{} {}", write.module, write.item))
        .collect();
    assert!(
        unclassed.is_empty(),
        "a sync write has no class: {unclassed:#?}"
    );
    let plain: BTreeSet<(String, String)> = writes
        .iter()
        .filter(|write| write.class == Some(Class::Plain))
        .map(|write| (write.module.clone(), write.item.clone()))
        .collect();
    let listed: BTreeSet<(String, String)> = PLAIN
        .iter()
        .map(|(module, item, _)| ((*module).to_owned(), (*item).to_owned()))
        .collect();
    assert_eq!(
        plain, listed,
        "the PLAIN sync writes are not the listed ones (SPEC-432 R8)"
    );
    let incrementing: Vec<String> = writes
        .iter()
        .flat_map(|write| write.increments.iter().cloned())
        .collect();
    assert!(
        incrementing.is_empty(),
        "a sync write increments: {incrementing:#?}"
    );
    assert!(
        census.unreadable.is_empty(),
        "a write the check cannot read: {:#?}",
        census.unreadable
    );
    let refused = judge(&census);
    assert!(refused.is_empty(), "refused at the head: {refused:#?}");
}

#[test]
fn a_planted_sync_write_that_can_apply_twice_is_refused() {
    let (_planted, census) = planted_census(PLANTED_TWICE, &[]);
    examined_all(&census);
    let refused: BTreeSet<String> = judge(&census).into_iter().collect();
    let expected: BTreeSet<String> = [
        "skip.rs record_twice PLAIN: 1 plain insert(s), and the site is not listed",
        "skip.rs bump_count SET INCREMENT: `due_count = due_count + 1` names its own column",
        "skip.rs bump_window UPSERT INCREMENT: `window_count = ingest_state.window_count + \
         excluded.window_count` names its own column",
        "skip.rs build_with: a WITH that holds a write, a write the check cannot read",
        "skip.rs build_joined: a write keyword joined to a placeholder, a write the check cannot \
         read",
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
    println!("examined {reached} of {} planted write(s)", expected.len());
    assert_eq!(
        reached,
        expected.len(),
        "every planted write reached its check"
    );
    assert!(
        missing.is_empty(),
        "a planted write that can apply twice is not refused by name and class: {missing:#?}\n\
         refused: {refused:#?}"
    );
    assert_eq!(
        refused, expected,
        "only the planted writes are refused; the listed PLAIN site is not"
    );
}

#[test]
fn an_idempotent_write_shape_is_classed_not_refused() {
    let (_planted, census) = planted_census(PLANTED_IDEMPOTENT, &[]);
    examined_all(&census);
    let classes: BTreeMap<&str, Option<Class>> = census
        .writes
        .iter()
        .filter(|write| write.module == "skip.rs")
        .map(|write| (write.item.as_str(), write.class))
        .collect();
    let cases = [
        ("begin", Class::Keyed),
        ("mark", Class::Keyed),
        ("write_anchor", Class::Upsert),
        ("settle", Class::Set),
        ("forget", Class::Delete),
    ];
    let mut reached = 0;
    let mut misclassed = Vec::new();
    for (item, class) in cases {
        reached += 1;
        let found = classes.get(item).copied().flatten();
        if found != Some(class) {
            misclassed.push(format!("{item}: {found:?}, expected {class:?}"));
        }
    }
    println!("examined {reached} of {} idempotent shape(s)", cases.len());
    assert_eq!(
        reached,
        cases.len(),
        "every idempotent shape reached its check"
    );
    assert!(
        misclassed.is_empty(),
        "an idempotent write is not classed by its shape: {misclassed:#?}"
    );
    let refused = judge(&census);
    assert!(
        refused.is_empty(),
        "an idempotent write shape is refused: {refused:#?}"
    );
}

#[test]
fn a_write_module_is_classified_and_a_listed_one_holds_a_write() {
    let (_planted, census) = planted_census(
        PLANTED_SKIP_WITHOUT_PRIOR,
        &[
            ("state.rs", PLANTED_STATE_READ),
            ("rogue.rs", PLANTED_ROGUE),
        ],
    );
    examined_all(&census);
    let refused: BTreeSet<String> = judge(&census).into_iter().collect();
    let expected: BTreeSet<String> = [
        "rogue.rs holds 1 write(s) and is neither a sync module nor exempt",
        "state.rs is a listed module and holds no write",
        "skip.rs record_prior PLAIN: listed, and none is found",
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
    println!("examined {reached} of {} module case(s)", expected.len());
    assert_eq!(
        reached,
        expected.len(),
        "every module case reached its check"
    );
    assert!(
        missing.is_empty(),
        "a module case is not refused by name: {missing:#?}\nrefused: {refused:#?}"
    );
    assert_eq!(refused, expected, "only the three module cases are refused");
}
