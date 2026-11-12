//! The closed grammar at its edges (SPEC-514 A2 and A3): an escape keeps the character it escapes
//! inside a quote, a key and a term, wherever in the search it falls; a backslash before white
//! space is refused before any key reads it; a closed group gives back the depth it took; a
//! field's `re:` and `nc:` modes are refused by name; and every `is:` value reads as its state.

use deck_streak_search::{Limits, Profile, Query, Refusal, State, Term};

fn text(word: &str) -> Query {
    Query::Term(Term::Text(word.to_owned()))
}

/// Parses under the operator profile, panicking with the refusal.
fn parsed(search: &str) -> Query {
    Profile::OPERATOR
        .parse(search)
        .unwrap_or_else(|refusal| panic!("{search:?} parses: {refusal}"))
}

#[test]
fn sibling_groups_each_give_back_the_depth_they_took() {
    let one = Limits {
        length: 64,
        depth: 1,
        arity: 8,
    };
    assert_eq!(
        Profile::OPERATOR.parse_within("(a) (b) -(c)", one),
        Ok(Query::And(vec![
            text("a"),
            text("b"),
            Query::negate(text("c"))
        ])),
        "three groups side by side nest one deep"
    );
    assert_eq!(
        Profile::OPERATOR.parse_within("(a) ((b))", one),
        Err(Refusal::TooDeep),
        "a group inside a group nests two deep"
    );
}

#[test]
fn a_quoted_escape_keeps_the_character_it_escapes_at_any_position() {
    // The first case's backslash sits at the third character, the second's at the fourth.
    let cases = [("\"a\\\"b\"", "a\"b"), ("\"ab\\\"c\"", "ab\"c")];
    for (search, word) in cases {
        assert_eq!(parsed(search), text(word), "{search:?}");
    }
    println!("examined {} quoted search(es)", cases.len());
}

#[test]
fn an_escape_inside_a_key_keeps_the_key_whole_before_its_quoted_value() {
    assert_eq!(
        parsed("a\\:b:\"c\""),
        Query::Term(Term::Field {
            field: "a:b".to_owned(),
            text: "c".to_owned(),
        })
    );
}

#[test]
fn an_escaped_space_is_refused_before_a_key_reads_it() {
    assert_eq!(
        Profile::SKIP.parse("a\\ b:\"c\""),
        Err(Refusal::BadEscape),
        "before a quoted value"
    );
    assert_eq!(
        Profile::SKIP.parse("a\\ b:c"),
        Err(Refusal::BadEscape),
        "before an unquoted value"
    );
    assert_eq!(
        Profile::OPERATOR.parse("is:new\\ x"),
        Err(Refusal::BadEscape),
        "inside a value"
    );
}

#[test]
fn an_escaped_colon_in_a_term_is_text_and_splits_no_key() {
    let cases = [("a\\:b", "a:b"), ("\\:a", ":a")];
    for (search, word) in cases {
        assert_eq!(parsed(search), text(word), "{search:?}");
    }
    println!("examined {} escaped colon(s)", cases.len());
}

#[test]
fn a_fields_regex_and_no_combining_modes_are_refused_by_name() {
    for search in ["Prompt:re:x", "Prompt:nc:x"] {
        assert_eq!(
            Profile::OPERATOR.parse(search),
            Err(Refusal::Unsupported("a field's re: or nc: mode")),
            "{search:?}"
        );
    }
    assert_eq!(
        parsed("Prompt:rex"),
        Query::Term(Term::Field {
            field: "Prompt".to_owned(),
            text: "rex".to_owned(),
        }),
        "a value that only begins with re is searched"
    );
}

#[test]
fn every_is_value_reads_as_its_own_state() {
    let cases = [
        ("is:new", State::New),
        ("is:learn", State::Learn),
        ("is:review", State::Review),
        ("is:due", State::Due),
        ("is:buried", State::Buried),
        ("is:suspended", State::Suspended),
    ];
    for (search, state) in cases {
        assert_eq!(parsed(search), Query::Term(Term::Is(state)), "{search:?}");
    }
    println!("examined {} state(s)", cases.len());
}
