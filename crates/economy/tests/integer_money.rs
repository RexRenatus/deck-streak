//! Money in the economy crate is whole coins (SPEC-432 R1, R2; ADR-446 D1). Every `.rs` file under
//! `crates/economy/src/` is read through the shared lexer, which drops comments and decodes
//! literals, and a float token is counted by five rules, F1 to F5, each token named by the item
//! that holds it.
//!
//! The found map of file and item to count equals the listed map of today's sites, so an unlisted
//! item, a grown count, a shrunk count and a listed item gone are each refused by name. Every walk
//! prints how many it examined and refuses zero (R9). The planted trees are temporary directories.

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

/// Today's float sites (SPEC-432 R2): the file under `crates/economy/src/`, the item and its
/// float tokens. Each constant holds a type word and a decimal literal; each rule an `as f64`.
const LISTED: [(&str, &str, usize); 5] = [
    ("constants.rs", "DAILY_LOSS_CAP_WALLET_FRAC", 2),
    ("constants.rs", "FINE_WALLET_FRAC", 2),
    ("constants.rs", "PASS_SURCHARGE_MULT", 2),
    ("rules.rs", "daily_loss_cap", 1),
    ("rules.rs", "scaled_fine", 1),
];

/// The keywords whose next identifier names the item a token sits in (ADR-446 D1).
const ITEM_KEYWORDS: [&str; 7] = ["fn", "const", "static", "type", "struct", "enum", "trait"];

/// The name a token takes when no item keyword comes before it in its file.
const NO_ITEM: &str = "(no item)";

/// The five float rules of ADR-446 D1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    /// The type word `f32` or `f64`, a cast `as f64` included.
    F1,
    /// A word ending `_f32` or `_f64`, or a digit-led word ending `f32` or `f64`.
    F2,
    /// A decimal literal: a digit-led word, a `.`, then a digit-led word.
    F3,
    /// An exponent literal: a digit-led word holding `e` or `E` after a digit.
    F4,
    /// A trailing-dot literal: a digit-led word, then a `.` before neither a word nor a `.`.
    F5,
}

/// What the census read under one root: every file, how many tokens it lexed, and each item
/// holding a float token with the rule of each token.
struct Census {
    files: Vec<String>,
    tokens: usize,
    found: BTreeMap<(String, String), Vec<Rule>>,
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

/// Writes `text` at `path` under `root`, making its directories.
fn plant(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("a planted file has a directory"))
        .expect("the planted directory is writable");
    fs::write(&path, text).expect("the planted file is writable");
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

/// The float rule the token at `at` opens, with how many tokens the float spans.
fn float_rule(tokens: &[Token], at: usize) -> Option<(Rule, usize)> {
    let _ = (tokens, at);
    None
}

/// Reads every `.rs` file under `crates/economy/src/` of `root` and counts its float tokens.
fn census(root: &Path) -> Census {
    let source = root.join("crates/economy/src");
    let mut census = Census {
        files: Vec::new(),
        tokens: 0,
        found: BTreeMap::new(),
    };
    for path in files(&source) {
        let file = relative(&source, &path);
        let text = fs::read_to_string(&path).expect("a readable source");
        let tokens: Vec<Token> = table_census::lex(&text)
            .into_iter()
            .map(|(token, _)| token)
            .collect();
        let named = items(&tokens);
        let mut at = 0;
        while at < tokens.len() {
            let Some((rule, width)) = float_rule(&tokens, at) else {
                at += 1;
                continue;
            };
            let item = named[at].clone().unwrap_or_else(|| NO_ITEM.to_owned());
            census
                .found
                .entry((file.clone(), item))
                .or_default()
                .push(rule);
            at += width;
        }
        census.tokens += tokens.len();
        census.files.push(file);
    }
    census
}

/// The judge (SPEC-432 R2): the found map must equal the listed one.
fn judge(
    found: &BTreeMap<(String, String), Vec<Rule>>,
    listed: &[(&str, &str, usize)],
) -> Vec<String> {
    let _ = (found, listed);
    Vec::new()
}

/// A planted file holding one float per rule, each in its own item.
const PLANTED_RULES: &str = "pub fn by_type(wallet: i64) -> i64 {
    (wallet as f64) as i64
}

pub fn by_suffix(wallet: i64) -> i64 {
    wallet.to_f64()
}

pub const BY_DECIMAL: i64 = 1.5;

pub const BY_EXPONENT: i64 = 1e3;

pub const BY_TRAILING_DOT: i64 = 2.;
";

/// A planted file holding whole-number shapes and floats only in a comment and a string.
const PLANTED_SHAPES: &str = r#"pub fn field_path(pair: (i64, i64)) -> i64 {
    pair.0 + pair.1
}

pub fn nested_field_path(nested: ((i64, i64), i64)) -> i64 {
    nested.0.1
}

pub fn range() -> i64 {
    (0..10).sum()
}

pub fn method_on_an_integer() -> i64 {
    5_i64.max(3)
}

pub const HEX: i64 = 0x1E;

// A comment's 1.5 and f64 are prose.
pub const COMMENTED: i64 = 2; /* 2.5e3 as f64 */

pub const STRING: &str = "1.5 as f64";
"#;

/// A planted economy whose listed sites moved: one grown, one shrunk, one gone, one unlisted.
const PLANTED_CONSTANTS: &str = "pub const DAILY_LOSS_CAP_WALLET_FRAC: f64 = 0.30 * 1.0;
pub const FINE_WALLET_FRAC: f64 = 2;
pub const PASS_SURCHARGE_MULT: f64 = 1.5;
pub const UNLISTED_SHARE: f64 = 0.5;
";

/// The planted economy's rules: the loss cap keeps its cast, the fine is whole coins.
const PLANTED_RULES_FILE: &str = "pub fn daily_loss_cap(wallet: i64) -> i64 {
    (wallet as f64) as i64
}

pub fn scaled_fine(wallet: i64) -> i64 {
    wallet / 50
}
";

#[test]
fn the_economy_holds_floats_only_at_its_listed_sites() {
    let census = census(&root());
    examined("source file(s)", census.files.len());
    examined("token(s)", census.tokens);
    for ((file, item), rules) in &census.found {
        println!("float site {file} {item}: {} by {rules:?}", rules.len());
    }
    let found: BTreeMap<(String, String), usize> = census
        .found
        .iter()
        .map(|(site, rules)| (site.clone(), rules.len()))
        .collect();
    let listed: BTreeMap<(String, String), usize> = LISTED
        .iter()
        .map(|(file, item, count)| (((*file).to_owned(), (*item).to_owned()), *count))
        .collect();
    assert_eq!(
        found, listed,
        "the economy's float sites are not the listed ones (SPEC-432 R2)"
    );
    let refused = judge(&census.found, &LISTED);
    assert!(refused.is_empty(), "refused at the head: {refused:#?}");
}

#[test]
fn a_float_planted_in_the_economy_is_refused_by_its_rule() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/economy/src/planted.rs",
        PLANTED_RULES,
    );
    let census = census(planted.path());
    examined("source file(s)", census.files.len());
    examined("token(s)", census.tokens);
    let refused = judge(&census.found, &LISTED);
    let cases = [
        ("by_type", Rule::F1),
        ("by_suffix", Rule::F2),
        ("BY_DECIMAL", Rule::F3),
        ("BY_EXPONENT", Rule::F4),
        ("BY_TRAILING_DOT", Rule::F5),
    ];
    let mut reached = 0;
    let mut missing = Vec::new();
    for (item, rule) in cases {
        reached += 1;
        let want =
            format!("planted.rs {item}: 1 float token(s) by {rule:?}, and the item is not listed");
        if !refused.contains(&want) {
            missing.push(want);
        }
    }
    println!("examined {reached} of {} planted rule(s)", cases.len());
    assert_eq!(reached, cases.len(), "every planted rule reached its check");
    assert!(
        missing.is_empty(),
        "a planted float is not refused by its rule: {missing:#?}\nrefused: {refused:#?}"
    );
}

#[test]
fn a_whole_number_shape_is_not_a_float() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/economy/src/planted.rs",
        PLANTED_SHAPES,
    );
    let census = census(planted.path());
    examined("source file(s)", census.files.len());
    examined("token(s)", census.tokens);
    assert_eq!(census.files, vec!["planted.rs".to_owned()]);
    assert_eq!(
        census.tokens,
        table_census::lex(PLANTED_SHAPES).len(),
        "the census read every token of the planted shapes"
    );
    let shapes = [
        "field_path",
        "nested_field_path",
        "range",
        "method_on_an_integer",
        "HEX",
        "COMMENTED",
        "STRING",
    ];
    let mut reached = 0;
    let mut counted = Vec::new();
    for item in shapes {
        reached += 1;
        let site = ("planted.rs".to_owned(), item.to_owned());
        if let Some(rules) = census.found.get(&site) {
            counted.push(format!("{item}: {rules:?}"));
        }
    }
    println!(
        "examined {reached} of {} whole-number shape(s)",
        shapes.len()
    );
    assert_eq!(reached, shapes.len(), "every shape reached its check");
    assert!(
        counted.is_empty(),
        "a whole-number shape is counted as a float: {counted:#?}"
    );
    assert!(
        census.found.is_empty(),
        "no planted shape holds a float: {:#?}",
        census.found
    );
}

#[test]
fn a_listed_float_site_gone_or_grown_is_refused() {
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/economy/src/constants.rs",
        PLANTED_CONSTANTS,
    );
    plant(
        planted.path(),
        "crates/economy/src/rules.rs",
        PLANTED_RULES_FILE,
    );
    let census = census(planted.path());
    examined("source file(s)", census.files.len());
    examined("token(s)", census.tokens);
    let refused: BTreeSet<String> = judge(&census.found, &LISTED).into_iter().collect();
    let expected: BTreeSet<String> = [
        "constants.rs UNLISTED_SHARE: 2 float token(s) by F1, F3, and the item is not listed",
        "constants.rs DAILY_LOSS_CAP_WALLET_FRAC: 3 float token(s) by F1, F3, F3, listed 2",
        "constants.rs FINE_WALLET_FRAC: 1 float token(s) by F1, listed 2",
        "rules.rs scaled_fine: listed 1 float token(s), and none is found",
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
    println!("examined {reached} of {} moved site(s)", expected.len());
    assert_eq!(
        reached,
        expected.len(),
        "every moved site reached its check"
    );
    assert!(
        missing.is_empty(),
        "a moved site is not refused by name: {missing:#?}\nrefused: {refused:#?}"
    );
    assert_eq!(
        refused, expected,
        "the unlisted item, the grown count, the shrunk count and the listed item gone are each \
         refused by name, and the two unmoved sites are not"
    );
}
