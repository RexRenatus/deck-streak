//! The engine's text matching as the evaluator computes it (SPEC-514 R7): a search text's `LIKE`
//! pattern keeps its escaped backslash and star, a glob folds case and says whether it holds a
//! wildcard, and the sort field is read with its markup removed.

use deck_streak_search::pattern::{Glob, strip_html, to_like};

#[test]
fn a_search_texts_like_pattern_keeps_its_escaped_backslash_and_star() {
    let cases = [
        ("a*b", "a%b"),
        ("a\\*b", "a*b"),
        ("a\\\\*", "a\\\\%"),
        ("a\\_b", "a\\_b"),
        ("50%", "50\\%"),
    ];
    for (text, pattern) in cases {
        assert_eq!(to_like(text), pattern, "{text:?}");
    }
    println!("examined {} search text(s)", cases.len());
}

#[test]
fn a_glob_folds_case_and_knows_its_own_wildcards() {
    let plain = Glob::new("Ds Home");
    assert!(!plain.is_wild(), "a name with no wildcard");
    assert!(plain.matches("ds home"), "folded to lower case");
    assert!(plain.matches("DS HOME"), "folded to lower case");
    assert!(!plain.matches("Ds Homes"), "matched whole");
    assert!(!plain.matches("Ds Hom"), "matched whole");
    assert!(Glob::new("Ärger").matches("äRGER"), "folded past ASCII");
    assert!(
        !Glob::new("\u{130}").matches("i"),
        "a letter whose lower case is two characters is kept"
    );
    assert!(Glob::new("Ds*").is_wild(), "a star");
    assert!(Glob::new("b_sil").is_wild(), "an underscore");
    assert!(!Glob::new("a\\*b").is_wild(), "an escaped star");
    assert!(!Glob::new("a\\_b").is_wild(), "an escaped underscore");
}

#[test]
fn the_sort_field_is_read_with_its_markup_removed() {
    let cases = [
        ("<b>amber</b>", "amber"),
        ("am<i>b</i>er", "amber"),
        ("a > b", "a > b"),
        ("ember<br", "ember"),
        ("x&nbsp;&lt;y&gt;&amp;&quot;&#39;", "x <y>&\"'"),
        ("plain", "plain"),
    ];
    for (field, sorted) in cases {
        assert_eq!(strip_html(field), sorted, "{field:?}");
    }
    println!("examined {} field(s)", cases.len());
}
