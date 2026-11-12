//! The skip profile agrees with the one-expression check on every golden skip search, an unknown
//! key stops start naming the setting and never the value, and the skip search is refused before
//! its wrap when the profile refuses it (SPEC-514 A1, A2 and R8).

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::settings::{SKIP_SEARCH, SkipSearch};
use deck_streak_ingest::skip::{SearchRefusal, is_one_expression, skip_search};
use deck_streak_kernel::{Environment, SettingsError};
use deck_streak_search::Profile;

#[test]
fn the_skip_profile_accepts_and_refuses_each_golden_skip_search_as_the_one_expression_check_does() {
    let mut accepted = 0;
    let examined = golden::each_case("skip_search", |case| {
        let search = case.input["search"].as_str().expect("a search");
        let one = is_one_expression(search);
        assert_eq!(
            Profile::SKIP.parse(search).is_ok(),
            one,
            "the profile and the check disagree on {search:?}"
        );
        if one {
            accepted += 1;
        }
    });
    assert!(examined.count > 0);
    // The golden's searches are all one expression; the shapes the check refuses are held here
    // too, so the agreement is examined on both sides.
    let refused = [
        "deck:X) or (deck:X",
        "",
        "   ",
        "(a",
        "a)",
        "\"a",
        "deck:a\\\\\"b",
        "deck:\"a\\\"",
        ")(",
        "((a)",
    ];
    for search in refused {
        assert!(!is_one_expression(search), "{search:?}");
        assert!(Profile::SKIP.parse(search).is_err(), "{search:?}");
    }
    for search in [
        "deck:\"a)\"",
        "deck:\"a(\"",
        "deck:a\\\"b",
        "deck:a\\(b",
        "deck:a\\)b",
    ] {
        assert!(is_one_expression(search), "{search:?}");
        assert!(Profile::SKIP.parse(search).is_ok(), "{search:?}");
    }
    println!(
        "A1: examined {} golden search(es), {accepted} accepted, and {} refused shape(s)",
        examined.count,
        refused.len()
    );
}

#[test]
fn an_unknown_key_stops_start_naming_the_setting_never_the_value() {
    for value in [
        "dekc:Spanish",
        "deck:Spanish Frnot:x",
        "(deck:A or zebra:yak)",
        "added:1",
    ] {
        let refused = SkipSearch::from_env(&Environment::from_vars([(SKIP_SEARCH, value)]));
        assert_eq!(
            refused,
            Err(SettingsError::Malformed {
                setting: SKIP_SEARCH,
                expected: "one Anki search expression"
            }),
            "{value:?} stops start"
        );
        let named = refused.map(|_| ()).unwrap_err().to_string();
        assert!(named.contains(SKIP_SEARCH), "{named}");
        for part in value
            .split([' ', ':', '(', ')'])
            .filter(|part| part.len() > 2)
        {
            assert!(!named.contains(part), "{named} names {part:?} of the value");
        }
    }
    let known = SkipSearch::from_env(&Environment::from_vars([(SKIP_SEARCH, "deck:Spanish")]))
        .expect("a known key starts");
    assert_eq!(known.as_str(), "deck:Spanish");
}

#[test]
fn a_search_the_skip_profile_refuses_is_refused_before_the_wrap() {
    assert_eq!(
        skip_search("dekc:Spanish"),
        Err(SearchRefusal::OutsideTheSkipProfile)
    );
    assert_eq!(
        skip_search("flag:1"),
        Err(SearchRefusal::OutsideTheSkipProfile)
    );
    assert_eq!(
        skip_search(&"a ".repeat(600)),
        Err(SearchRefusal::OutsideTheSkipProfile)
    );
    assert_eq!(
        skip_search("deck:X) or (deck:X"),
        Err(SearchRefusal::NotOneExpression)
    );
    assert_eq!(
        skip_search("tag:daily or deck:Math").as_deref(),
        Ok(
            "(tag:daily or deck:Math) -is:new -is:learn prop:due=0 -is:suspended -is:buried -deck:filtered"
        )
    );
}
