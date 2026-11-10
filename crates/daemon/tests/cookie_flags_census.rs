//! The cookie flag census (SPEC-436 R25 to R29, ADR-450 D10): a `Set-Cookie` header a shipped
//! source sets is a pair, a parenthesised group whose first item names the header and whose second
//! is its value. A pair whose value is a string literal, or a `format!` whose format string is one,
//! carrying `Secure`, `HttpOnly` and a `SameSite` attribute passes; any other pair is a site, keyed
//! by its file, its function, its form and its ordinal. The listing holds today's sites, and the
//! census starts at `Level::Warn` (ADR-450 D5).

mod shipped_sources;

use proc_macro2::{Delimiter, TokenTree};
use shipped_sources::{
    Key, Level, fn_head, is_punct, keyed, opens_const_item, path_ending_at, planted,
    repository_sources, shipped, tokens_in, verdict,
};

/// The rule this census names in its lines.
const RULE: &str = "cookie-flags";

/// The census's level (R28). Moving it to `Fail` is its own delivery (R24).
const LEVEL: Level = Level::Warn;

/// Today's sites (R28): the two pairs whose value is a name the census does not read.
const LISTED: &[&str] = &[
    "crates/api/src/linking_routes.rs ceremony_started set-cookie #1",
    "crates/api/src/linking_routes.rs ceremony_ended set-cookie #1",
];

/// The header's name as the `http` crate's constant spells it, the last segment of a path (R25).
const HEADER_CONSTANT: &str = "SET_COOKIE";

/// The header's name as a string literal spells it, compared in lower case (R25).
const HEADER_LITERAL: &str = "set-cookie";

/// The three attributes a pair's value carries, compared in lower case (R26).
const SECURE: &str = "secure";
const HTTP_ONLY: &str = "httponly";
const SAME_SITE: &str = "samesite=";

/// The form every site prints (R27).
const FORM: &str = "set-cookie";

/// What the census found: the keys, the files it examined and the pairs it judged.
#[derive(Default)]
struct Census {
    keys: Vec<Key>,
    files: usize,
    pairs: usize,
}

/// The census over a list of (path, text) (R19): every member's shipped sources (R5, R6). It
/// prints its two counts, then refuses an empty population and a population with no pair (R29).
fn census(files: &[(String, String)]) -> Result<Census, String> {
    let sources = shipped(files)?;
    let mut census = Census::default();
    for source in &sources {
        let mut sites = Vec::new();
        walk(&source.tokens, "", &mut sites, &mut census.pairs);
        census.keys.extend(keyed(&source.path, &sites));
    }
    census.files = sources.len();
    eprintln!("examined {} shipped files", census.files);
    eprintln!("examined {} set-cookie pairs", census.pairs);
    if census.files == 0 {
        return Err("examined 0 shipped files".to_owned());
    }
    if census.pairs == 0 {
        return Err("examined 0 set-cookie pairs".to_owned());
    }
    Ok(census)
}

/// Walks one token list inside `function` (R25, R27): every item is read, a `const` item and a
/// `const { }` block included, and every parenthesised group is read as a possible pair. Outside
/// any function, a pair in a `const` item keys the const's name, which `opens_const_item` finds.
fn walk(
    tokens: &[TokenTree],
    function: &str,
    sites: &mut Vec<(String, String)>,
    pairs: &mut usize,
) {
    let mut head: Option<String> = None;
    let mut item: Option<String> = None;
    for (at, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => {
                let brace = group.delimiter() == Delimiter::Brace;
                let inner = tokens_in(group);
                let key = item.as_deref().unwrap_or(function);
                if group.delimiter() == Delimiter::Parenthesis
                    && let Some(passes) = pair(&inner)
                {
                    *pairs += 1;
                    if !passes {
                        sites.push((key.to_owned(), FORM.to_owned()));
                    }
                }
                if let (Some(name), true) = (&head, brace) {
                    walk(&inner, name, sites, pairs);
                    head = None;
                } else {
                    walk(&inner, key, sites, pairs);
                }
            }
            TokenTree::Ident(_) => {
                if let Some((name, _)) = fn_head(tokens, at) {
                    head = Some(name);
                } else if function.is_empty() && opens_const_item(tokens, at) {
                    item = tokens.get(at + 1).map(ToString::to_string);
                }
            }
            TokenTree::Punct(punct) if punct.as_char() == ';' => {
                head = None;
                item = None;
            }
            _ => {}
        }
    }
}

/// Whether a parenthesised group's items are a `Set-Cookie` pair (R25), and if so whether its
/// value carries the three attributes (R26): `None` when the group is no pair. The first item is
/// a path ending in `SET_COOKIE` or the literal `"set-cookie"` in any case; a trailing comma is
/// dropped; a value that is neither a literal nor a `format!` of one carries nothing.
fn pair(inner: &[TokenTree]) -> Option<bool> {
    let comma = inner.iter().position(|token| is_punct(Some(token), ','))?;
    let name = comma.checked_sub(1)?;
    let named = match inner.get(name)? {
        TokenTree::Ident(ident) => {
            let path = path_ending_at(inner, name);
            ident == HEADER_CONSTANT && path.len() * 3 == comma + 2
        }
        TokenTree::Literal(literal) => {
            comma == 1 && literal.to_string().to_lowercase() == format!("\"{HEADER_LITERAL}\"")
        }
        _ => false,
    };
    if !named {
        return None;
    }
    let value = inner.get(comma + 1..).unwrap_or_default();
    let value = match value {
        [rest @ .., last] if is_punct(Some(last), ',') => rest,
        _ => value,
    };
    let text = match value {
        [TokenTree::Literal(literal)] => literal.to_string(),
        [TokenTree::Ident(ident), bang, TokenTree::Group(group)]
            if ident == "format" && is_punct(Some(bang), '!') =>
        {
            tokens_in(group)
                .first()
                .map(ToString::to_string)
                .unwrap_or_default()
        }
        _ => String::new(),
    };
    let attributes: Vec<String> = text
        .split(';')
        .map(|part| {
            part.trim()
                .trim_matches(|end| end == '"' || end == '#')
                .to_lowercase()
        })
        .collect();
    Some(
        attributes.iter().any(|attribute| attribute == SECURE)
            && attributes.iter().any(|attribute| attribute == HTTP_ONLY)
            && attributes
                .iter()
                .any(|attribute| attribute.starts_with(SAME_SITE)),
    )
}

/// The found keys, as they print.
fn printed(census: &Census) -> Vec<String> {
    census.keys.iter().map(ToString::to_string).collect()
}

#[test]
fn every_set_cookie_pair_is_judged_by_its_three_flags() {
    let files = planted(&[(
        "crates/planted/src/cookies.rs",
        r#"fn set(token: &str, value: String, headers: &mut HeaderMap) {
    let constant = (SET_COOKIE, "s=1; Path=/; Secure; HttpOnly; SameSite=Strict");
    let qualified = (header::SET_COOKIE, format!("s={token}; Secure; HttpOnly; SameSite=Lax"));
    let trailing = (SET_COOKIE, "s=1; secure; httponly; samesite=none",);
    let no_secure = (SET_COOKIE, "s=1; HttpOnly; SameSite=Strict");
    let no_http_only = (SET_COOKIE, "s=1; Secure; SameSite=Strict");
    let no_same_site = (header::SET_COOKIE, "s=1; Secure; HttpOnly");
    let literal_name = ("Set-Cookie", "s=1; Secure; HttpOnly");
    let unread = (SET_COOKIE, value);
    headers.insert(SET_COOKIE, format!("{token}; {ATTRIBUTES}"));
}
"#,
    )]);
    let census = census(&files).expect("the census reads the planted file");
    assert_eq!(
        printed(&census),
        [
            "crates/planted/src/cookies.rs set set-cookie #1",
            "crates/planted/src/cookies.rs set set-cookie #2",
            "crates/planted/src/cookies.rs set set-cookie #3",
            "crates/planted/src/cookies.rs set set-cookie #4",
            "crates/planted/src/cookies.rs set set-cookie #5",
            "crates/planted/src/cookies.rs set set-cookie #6",
        ]
    );
    assert_eq!(census.pairs, 9);
    assert_eq!(census.files, 1);
}

#[test]
fn a_const_item_pair_is_judged_and_test_code_and_other_headers_are_not_pairs() {
    let files = planted(&[(
        "crates/planted/src/quiet.rs",
        r#"use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};

const PAIR: (HeaderName, &str) = (SET_COOKIE, "s=1");

fn quiet(value: String) {
    let sensitive = [AUTHORIZATION, COOKIE, SET_COOKIE];
    let request = (COOKIE, value);
    let suffixed = (NOT_SET_COOKIE_AT_ALL, value);
    // let commented = (SET_COOKIE, value);
    let text = "(SET_COOKIE, value)";
    let control = (SET_COOKIE, "s=1; Secure; HttpOnly; SameSite=Strict");
    let block = const { (SET_COOKIE, "s=1; Secure; SameSite=Strict") };
}

#[cfg(test)]
mod tests {
    fn inline(value: String) {
        let unread = (SET_COOKIE, value);
    }
}

#[test]
fn marked(value: String) {
    let unread = (SET_COOKIE, value);
}
"#,
    )]);
    let census = census(&files).expect("the census reads the planted file");
    assert_eq!(
        printed(&census),
        [
            "crates/planted/src/quiet.rs PAIR set-cookie #1",
            "crates/planted/src/quiet.rs quiet set-cookie #1",
        ]
    );
    assert_eq!(census.pairs, 3);
}

#[test]
fn the_repository_lists_exactly_todays_cookie_sites() {
    let files = repository_sources().expect("the tree's sources read");
    let census = census(&files).expect("the census reads the tree");
    assert_eq!(printed(&census), LISTED);
    assert_eq!(LEVEL, Level::Warn);
    assert!(census.files > 0);
    assert!(census.pairs > 0);
    assert_eq!(verdict(RULE, LEVEL, &census.keys, LISTED), Ok(Vec::new()));
}

#[test]
fn the_cookie_census_refuses_an_empty_population() {
    assert_eq!(
        census(&[]).err(),
        Some("examined 0 shipped files".to_owned())
    );
    let files = planted(&[(
        "crates/planted/src/plain.rs",
        "fn plain() { let header = (CONTENT_TYPE, \"text/plain\"); }\n",
    )]);
    assert_eq!(
        census(&files).err(),
        Some("examined 0 set-cookie pairs".to_owned())
    );
}
