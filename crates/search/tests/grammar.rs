//! The closed grammar reads the engine's syntax into one typed query, refuses every shape it
//! cannot read and every key outside it by name, and refuses past its depth, length and arity
//! limits (SPEC-514 A2 and A3). No refusal's text carries the search.

use deck_streak_search::{
    Compare, Limits, Operator, Profile, Prop, Query, Refusal, State, Template, Term,
};

fn term(term: Term) -> Query {
    Query::Term(term)
}

fn text(word: &str) -> Query {
    term(Term::Text(word.to_owned()))
}

/// Parses under the operator profile, panicking with the refusal.
fn parsed(search: &str) -> Query {
    Profile::OPERATOR
        .parse(search)
        .unwrap_or_else(|refusal| panic!("{search:?} parses: {refusal}"))
}

#[test]
fn depth_length_and_arity_limits_refuse() {
    let limits = Limits::DEFAULT;
    assert_eq!((limits.length, limits.depth, limits.arity), (1024, 8, 32));
    let longest = "a".repeat(limits.length);
    assert_eq!(parsed(&longest), text(&longest));
    let too_long = "a".repeat(limits.length + 1);
    assert_eq!(Profile::OPERATOR.parse(&too_long), Err(Refusal::TooLong));
    let nested = |depth: usize| format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
    assert_eq!(parsed(&nested(limits.depth)), text("a"));
    assert_eq!(
        Profile::OPERATOR.parse(&nested(limits.depth + 1)),
        Err(Refusal::TooDeep)
    );
    let joined = |count: usize| vec!["a"; count].join(" ");
    assert!(Profile::OPERATOR.parse(&joined(limits.arity)).is_ok());
    assert_eq!(
        Profile::OPERATOR.parse(&joined(limits.arity + 1)),
        Err(Refusal::TooWide)
    );
    assert_eq!(
        Profile::OPERATOR.parse(&format!("({})", joined(limits.arity + 1))),
        Err(Refusal::TooWide),
        "a group is held to the arity limit too"
    );
    let ored = vec!["a"; limits.arity + 1].join(" or ");
    assert_eq!(Profile::OPERATOR.parse(&ored), Err(Refusal::TooWide));
    let narrow = Limits {
        length: 8,
        depth: 1,
        arity: 2,
    };
    assert_eq!(
        Profile::SKIP.parse_within("deck:Ds Home", narrow),
        Err(Refusal::TooLong)
    );
    assert_eq!(
        Profile::SKIP.parse_within("((a))", narrow),
        Err(Refusal::TooDeep)
    );
    assert_eq!(
        Profile::SKIP.parse_within("a b c", narrow),
        Err(Refusal::TooWide)
    );
    for refusal in [Refusal::TooLong, Refusal::TooDeep, Refusal::TooWide] {
        let said = refusal.to_string();
        assert!(said.contains("limit") && !said.contains('a'.to_string().repeat(2).as_str()));
    }
}

#[test]
fn the_grammar_reads_the_engines_syntax_into_one_typed_query() {
    let cases: Vec<(&str, Query)> = vec![
        (
            "a b OR c",
            Query::Or(vec![Query::And(vec![text("a"), text("b")]), text("c")]),
        ),
        (
            "a (b or c)",
            Query::And(vec![text("a"), Query::Or(vec![text("b"), text("c")])]),
        ),
        ("a and b", Query::And(vec![text("a"), text("b")])),
        (
            "-deck:\"Ds Home\"",
            Query::negate(term(Term::Deck("Ds Home".to_owned()))),
        ),
        (
            "\"deck:Two Words\"",
            term(Term::Deck("Two Words".to_owned())),
        ),
        (
            "tag:daily or deck:Math",
            Query::Or(vec![
                term(Term::Tag("daily".to_owned())),
                term(Term::Deck("Math".to_owned())),
            ]),
        ),
        ("prop:due=0", term(Term::Prop(Prop::Due, Compare::Eq, 0))),
        ("prop:due>=-3", term(Term::Prop(Prop::Due, Compare::Ge, -3))),
        (
            "prop:ivl!=4",
            term(Term::Prop(Prop::Interval, Compare::Ne, 4)),
        ),
        ("card:1", term(Term::Card(Template::Ordinal(0)))),
        ("card:0", term(Term::Card(Template::Ordinal(0)))),
        (
            "card:Forward",
            term(Term::Card(Template::Name("Forward".to_owned()))),
        ),
        (
            "rated:1:3",
            term(Term::Rated {
                days: 1,
                ease: Some(3),
            }),
        ),
        ("added:0", term(Term::Added(1))),
        ("is:suspended", term(Term::Is(State::Suspended))),
        ("IS:new", term(Term::Is(State::New))),
        ("a\\(b", text("a(b")),
        ("a\\*b", text("a\\*b")),
        ("\"ember fjord\"", text("ember fjord")),
        (
            "Prompt:*",
            term(Term::Field {
                field: "Prompt".to_owned(),
                text: "*".to_owned(),
            }),
        ),
        ("- a", Query::And(vec![text("-"), text("a")])),
        (
            "(a)  -(b)\u{3000}c",
            Query::And(vec![text("a"), Query::negate(text("b")), text("c")]),
        ),
    ];
    for (search, expected) in &cases {
        assert_eq!(&parsed(search), expected, "{search:?}");
    }
    println!("examined {} parse(s)", cases.len());
}

#[test]
fn every_shape_refusal_is_named_and_carries_no_text() {
    let cases = [
        ("", Refusal::Blank),
        ("  \t ", Refusal::Blank),
        ("zebra)", Refusal::UnopenedGroup),
        ("zebra) or (zebra", Refusal::UnopenedGroup),
        ("(zebra", Refusal::UnclosedGroup),
        ("()", Refusal::EmptyGroup),
        ("\"zebra", Refusal::UnclosedQuote),
        ("deck:\"zebra", Refusal::UnclosedQuote),
        ("\"\"", Refusal::EmptyQuote),
        ("zebra or", Refusal::MisplacedOperator),
        ("or zebra", Refusal::MisplacedOperator),
        ("zebra and or yak", Refusal::MisplacedOperator),
        (":zebra", Refusal::MissingKey),
        ("zebra\\ yak", Refusal::BadEscape),
        ("zebra\\x", Refusal::BadEscape),
        ("zebra\\", Refusal::BadEscape),
    ];
    for (search, refusal) in cases {
        assert_eq!(Profile::OPERATOR.parse(search), Err(refusal), "{search:?}");
        assert!(refusal.is_structural(), "{refusal:?}");
        let said = refusal.to_string();
        assert!(!said.contains("zebra") && !said.contains("yak"), "{said}");
    }
    println!("examined {} refusal(s)", cases.len());
}

#[test]
fn a_key_outside_the_grammar_is_refused_by_name() {
    let operator = [
        ("flag:1", Refusal::Unsupported("flag:")),
        ("FLAG:1", Refusal::Unsupported("flag:")),
        ("dupe:1,x", Refusal::Unsupported("dupe:")),
        ("prop:ease>2", Refusal::Unsupported("prop:ease")),
        ("prop:r>0.5", Refusal::Unsupported("prop:r")),
        ("prop:zebra=1", Refusal::BadValue(Operator::Prop)),
        ("prop:due=x", Refusal::BadValue(Operator::Prop)),
        ("is:zebra", Refusal::BadValue(Operator::Is)),
        ("is:buried-manually", Refusal::BadValue(Operator::Is)),
        ("added:x", Refusal::BadValue(Operator::Added)),
        ("rated:1:5", Refusal::BadValue(Operator::Rated)),
        ("tag:re:x", Refusal::Unsupported("tag:re:")),
        ("deck:current", Refusal::Unsupported("deck:current")),
    ];
    for (search, refusal) in operator {
        assert_eq!(Profile::OPERATOR.parse(search), Err(refusal), "{search:?}");
        assert!(!refusal.is_structural(), "{refusal:?}");
    }
    let skip = [
        ("zebra:yak", Refusal::UnknownKey),
        ("Prompt:*", Refusal::UnknownKey),
        ("dekc:Spanish", Refusal::UnknownKey),
        ("added:1", Refusal::NotInProfile(Operator::Added)),
        ("rated:1", Refusal::NotInProfile(Operator::Rated)),
        ("deck:A -(zebra:yak)", Refusal::UnknownKey),
    ];
    for (search, refusal) in skip {
        assert_eq!(Profile::SKIP.parse(search), Err(refusal), "{search:?}");
        let said = refusal.to_string();
        assert!(!said.contains("zebra") && !said.contains("dekc"), "{said}");
    }
    assert!(
        Profile::OPERATOR.parse("zebra:yak").is_ok(),
        "a field search"
    );
    println!("examined {} key(s)", operator.len() + skip.len());
}
