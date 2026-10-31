//! The handler panic census (SPEC-436 R13 to R17 and R23, ADR-450 D3): a panicking macro or an
//! unwrapping method in a shipped source of a request-handler crate is a site, keyed by its file,
//! its function, its form and its ordinal. The listing is empty, and the census starts at
//! `Level::Warn` (ADR-450 D5).

mod shipped_sources;

use proc_macro2::{Delimiter, Ident, TokenTree};
use shipped_sources::{
    Key, Level, before, fn_head, is_ident, is_punct, keyed, opens_const_item, path_ending_at,
    planted, repository_sources, shipped, tokens_in, verdict,
};

/// The rule this census names in its lines.
const RULE: &str = "handler-panic";

/// The census's level (R23). Moving it to `Fail` is its own delivery (R24).
const LEVEL: Level = Level::Warn;

/// Today's sites: none (R23).
const LISTED: &[&str] = &[];

/// The request-handler crates, by name (R13).
const HANDLER_CRATES: &[&str] = &["api", "bot", "daemon", "mcp"];

/// The panicking macros (R15), one per line.
const MACROS: &[&str] = &[
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "assert",
    "assert_eq",
    "assert_ne",
];

/// The prefixes a panicking macro may be invoked under (R15).
const PREFIXES: [&str; 2] = ["std", "core"];

/// The unwrapping methods (R15).
const METHODS: &[&str] = &["unwrap", "expect", "unwrap_err", "expect_err"];

/// What the census found: the keys, the handler crates it examined and its file count.
#[derive(Default)]
struct Census {
    keys: Vec<Key>,
    crates: Vec<String>,
    files: usize,
}

/// The census over a list of (path, text) (R19): the four crates' shipped sources (R14). It
/// prints its count, then refuses an empty population and a named crate with no source (R13).
fn census(files: &[(String, String)]) -> Result<Census, String> {
    let handler: Vec<(String, String)> = files
        .iter()
        .filter(|(path, _)| crate_of(path).is_some())
        .cloned()
        .collect();
    let sources = shipped(&handler)?;
    let mut census = Census::default();
    for source in &sources {
        let mut sites = Vec::new();
        walk(&source.tokens, "", &mut sites);
        census.keys.extend(keyed(&source.path, &sites));
    }
    census.files = sources.len();
    census.crates = HANDLER_CRATES
        .iter()
        .filter(|name| {
            sources
                .iter()
                .any(|source| crate_of(&source.path) == Some(**name))
        })
        .map(|name| (*name).to_owned())
        .collect();
    eprintln!(
        "examined {} shipped files in {} handler crates",
        census.files,
        census.crates.len()
    );
    if census.files == 0 {
        return Err(format!(
            "examined 0 shipped files in {} handler crates",
            census.crates.len()
        ));
    }
    if let Some(missing) = HANDLER_CRATES
        .iter()
        .find(|name| !census.crates.iter().any(|kept| kept.as_str() == **name))
    {
        return Err(format!("crates/{missing} has no src directory"));
    }
    Ok(census)
}

/// The handler crate whose `src` holds a path, if one does (R13, R14).
fn crate_of(path: &str) -> Option<&'static str> {
    HANDLER_CRATES.iter().copied().find(|name| {
        path.strip_prefix("crates/")
            .and_then(|rest| rest.strip_prefix(name))
            .is_some_and(|rest| rest.starts_with("/src/"))
    })
}

/// Walks one token list inside `function` (R15, R16): a `const` item's body and a `const { }`
/// block are skipped. Each position comes from the list's own index.
fn walk(tokens: &[TokenTree], function: &str, sites: &mut Vec<(String, String)>) {
    let mut in_const = false;
    let mut head: Option<String> = None;
    for (at, token) in tokens.iter().enumerate() {
        if in_const {
            in_const = !is_punct(Some(token), ';');
            continue;
        }
        match token {
            TokenTree::Group(group) => {
                let brace = group.delimiter() == Delimiter::Brace;
                if brace && is_ident(before(tokens, at, 1), "const") {
                    continue;
                }
                let inner = tokens_in(group);
                if let (Some(name), true) = (&head, brace) {
                    walk(&inner, name, sites);
                    head = None;
                } else {
                    walk(&inner, function, sites);
                }
            }
            TokenTree::Ident(ident) => {
                if let Some((name, _)) = fn_head(tokens, at) {
                    head = Some(name);
                } else if opens_const_item(tokens, at) {
                    in_const = true;
                } else if let Some(form) = site(tokens, at, ident) {
                    sites.push((function.to_owned(), form));
                }
            }
            TokenTree::Punct(punct) if punct.as_char() == ';' => head = None,
            _ => {}
        }
    }
}

/// The form the identifier `name` at `at` invokes when it panics (R15, R16): a listed macro,
/// bare or under `std::` or `core::`, or a listed method called with parentheses.
fn site(tokens: &[TokenTree], at: usize, name: &Ident) -> Option<String> {
    let invoked = is_punct(tokens.get(at + 1), '!')
        && matches!(tokens.get(at + 2), Some(TokenTree::Group(_)));
    if invoked && MACROS.iter().any(|form| name == form) {
        let path = path_ending_at(tokens, at);
        let prefixed = path.len() == 2
            && path
                .first()
                .is_some_and(|first| PREFIXES.contains(&first.as_str()));
        return (path.len() == 1 || prefixed).then(|| format!("{name}!"));
    }
    let called = matches!(
        tokens.get(at + 1),
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis
    );
    let method = called && is_punct(before(tokens, at, 1), '.');
    (method && METHODS.iter().any(|form| name == form)).then(|| format!(".{name}"))
}

/// The found keys, as they print.
fn printed(census: &Census) -> Vec<String> {
    census.keys.iter().map(ToString::to_string).collect()
}

/// An empty source in each handler crate but `api`, so a planted population names all four.
const OTHER_CRATES: [(&str, &str); 3] = [
    ("crates/bot/src/lib.rs", ""),
    ("crates/daemon/src/lib.rs", ""),
    ("crates/mcp/src/lib.rs", ""),
];

#[test]
fn every_panic_form_in_a_handler_crate_is_found() {
    let mut files = planted(&[(
        "crates/api/src/planted.rs",
        r#"fn handle(value: Option<u8>, result: Result<u8, u8>) {
    panic!("a");
    unreachable!();
    todo!();
    unimplemented!();
    assert!(true);
    assert_eq!(1, 1);
    assert_ne!(1, 2);
    std::panic!("a");
    core::assert!(true);
    value.unwrap();
    value.expect("a");
    result.unwrap_err();
    result.expect_err("a");
    panic!("b");
}
"#,
    )]);
    files.extend(planted(&OTHER_CRATES));
    let census = census(&files).expect("the census reads the planted files");
    assert_eq!(
        printed(&census),
        [
            "crates/api/src/planted.rs handle panic! #1",
            "crates/api/src/planted.rs handle unreachable! #1",
            "crates/api/src/planted.rs handle todo! #1",
            "crates/api/src/planted.rs handle unimplemented! #1",
            "crates/api/src/planted.rs handle assert! #1",
            "crates/api/src/planted.rs handle assert_eq! #1",
            "crates/api/src/planted.rs handle assert_ne! #1",
            "crates/api/src/planted.rs handle panic! #2",
            "crates/api/src/planted.rs handle assert! #2",
            "crates/api/src/planted.rs handle .unwrap #1",
            "crates/api/src/planted.rs handle .expect #1",
            "crates/api/src/planted.rs handle .unwrap_err #1",
            "crates/api/src/planted.rs handle .expect_err #1",
            "crates/api/src/planted.rs handle panic! #3",
        ]
    );
    assert_eq!(census.crates, ["api", "bot", "daemon", "mcp"]);
    assert_eq!(census.files, 4);
}

#[test]
fn test_code_debug_assertions_const_contexts_and_lookalikes_are_not_handler_panics() {
    let mut files = planted(&[(
        "crates/api/src/quiet.rs",
        r#"const LIMIT: u8 = { assert!(true); 1 };

static TABLE: u8 = const { panic!("a") };

fn quiet(value: Option<u8>) {
    debug_assert!(true);
    debug_assert_eq!(1, 1);
    debug_assert_ne!(1, 2);
    let block = const { assert!(true) };
    value.unwrap_or(1);
    value.unwrap_or_else(|| 1);
    value.unwrap_or_default();
    // panic!("a");
    let text = "value.unwrap()";
    panic!("control");
}

#[cfg(test)]
mod tests {
    fn inline() {
        panic!("a");
    }
}

#[test]
fn marked() {
    None::<u8>.unwrap();
}
"#,
    )]);
    files.extend(planted(&OTHER_CRATES));
    let census = census(&files).expect("the census reads the planted files");
    assert_eq!(
        printed(&census),
        ["crates/api/src/quiet.rs quiet panic! #1"]
    );
}

#[test]
fn the_handler_crates_hold_exactly_todays_listing() {
    let files = repository_sources().expect("the tree's sources read");
    let census = census(&files).expect("the census reads the tree");
    assert_eq!(census.crates, ["api", "bot", "daemon", "mcp"]);
    assert_eq!(printed(&census), LISTED);
    assert_eq!(LEVEL, Level::Warn);
    assert!(census.files > 0);
    assert_eq!(verdict(RULE, LEVEL, &census.keys, LISTED), Ok(Vec::new()));
}

#[test]
fn the_panic_census_refuses_an_empty_population() {
    assert_eq!(
        census(&[]).err(),
        Some("examined 0 shipped files in 0 handler crates".to_owned())
    );
    let files = planted(&OTHER_CRATES[..2]);
    let mut with_api = planted(&[("crates/api/src/lib.rs", "")]);
    with_api.extend(files);
    assert_eq!(
        census(&with_api).err(),
        Some("crates/mcp has no src directory".to_owned())
    );
}
