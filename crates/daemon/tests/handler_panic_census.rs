//! The handler panic census (SPEC-436 R13 to R17 and R23, ADR-450 D3): a panicking macro or an
//! unwrapping method in a shipped source of a request-handler crate is a site, keyed by its file,
//! its function, its form and its ordinal. The listing is empty, and the census starts at
//! `Level::Warn` (ADR-450 D5).

mod shipped_sources;

use proc_macro2::TokenTree;
use shipped_sources::{Key, Level, keyed, planted, repository_sources, shipped, verdict};

/// The rule this census names in its lines.
const RULE: &str = "handler-panic";

/// The census's level (R23). Moving it to `Fail` is its own delivery (R24).
const LEVEL: Level = Level::Warn;

/// Today's sites: none (R23).
const LISTED: &[&str] = &[];

/// What the census found: the keys, the handler crates it examined and its file count.
#[derive(Default)]
struct Census {
    keys: Vec<Key>,
    crates: Vec<String>,
    files: usize,
}

/// The census over a list of (path, text) (R19).
fn census(files: &[(String, String)]) -> Result<Census, String> {
    let sources = shipped(files)?;
    let mut census = Census::default();
    for source in &sources {
        let mut sites = Vec::new();
        walk(&source.tokens, "", &mut sites);
        census.keys.extend(keyed(&source.path, &sites));
    }
    eprintln!(
        "examined {} shipped files in {} handler crates",
        census.files,
        census.crates.len()
    );
    Ok(census)
}

/// Walks one token list.
fn walk(_tokens: &[TokenTree], _function: &str, _sites: &mut Vec<(String, String)>) {}

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
