//! The query's own values answer for themselves (SPEC-514 R1 and R2): each comparison of `prop:`
//! holds on its own side of the value and spells its own text, and each operator of the closed
//! grammar is named, and displayed, by its key.

use deck_streak_search::{Compare, Operator};

#[test]
fn each_comparison_holds_on_its_own_side_of_the_value() {
    let cases = [
        (Compare::Lt, vec![2]),
        (Compare::Le, vec![2, 3]),
        (Compare::Eq, vec![3]),
        (Compare::Ne, vec![2, 4]),
        (Compare::Ge, vec![3, 4]),
        (Compare::Gt, vec![4]),
    ];
    for (compare, expected) in &cases {
        let held: Vec<i64> = [2, 3, 4]
            .into_iter()
            .filter(|&left| compare.holds(left, 3))
            .collect();
        assert_eq!(&held, expected, "{compare:?} against 3");
    }
    println!("examined {} comparison(s)", cases.len());
    assert_eq!(cases.len(), 6, "every comparison");
}

#[test]
fn each_comparison_spells_its_own_text() {
    let cases = [
        (Compare::Lt, "<"),
        (Compare::Le, "<="),
        (Compare::Eq, "="),
        (Compare::Ne, "!="),
        (Compare::Ge, ">="),
        (Compare::Gt, ">"),
    ];
    for (compare, text) in cases {
        assert_eq!(compare.text(), text, "{compare:?}");
    }
    println!("examined {} comparison(s)", cases.len());
    assert_eq!(cases.len(), 6, "every comparison");
}

#[test]
fn each_operator_is_named_and_displayed_by_its_key() {
    let cases = [
        (Operator::Text, "text"),
        (Operator::Field, "field"),
        (Operator::Deck, "deck"),
        (Operator::Tag, "tag"),
        (Operator::Is, "is"),
        (Operator::Prop, "prop"),
        (Operator::Card, "card"),
        (Operator::Note, "note"),
        (Operator::Added, "added"),
        (Operator::Rated, "rated"),
    ];
    for (operator, key) in cases {
        assert_eq!(operator.name(), key, "{operator:?}'s name");
        assert_eq!(operator.to_string(), key, "{operator:?} displayed");
    }
    println!("examined {} operator(s)", cases.len());
    assert_eq!(
        cases.map(|(operator, _)| operator),
        Operator::ALL,
        "every operator, in declaration order"
    );
}
