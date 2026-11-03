//! Each profile admits exactly the operators it lists, and the skip profile's list is the closed
//! one its holds were written against (SPEC-514 R3; the Rust side of K-operator-profile).

use deck_streak_search::{Operator, Profile, Refusal, Term};

/// A search that uses `operator` and nothing else.
fn sample(operator: Operator) -> &'static str {
    match operator {
        Operator::Text => "amber",
        Operator::Field => "Prompt:amber",
        Operator::Deck => "deck:Ds",
        Operator::Tag => "tag:ds",
        Operator::Is => "is:review",
        Operator::Prop => "prop:due=0",
        Operator::Card => "card:1",
        Operator::Note => "note:ds*",
        Operator::Added => "added:1",
        Operator::Rated => "rated:1",
    }
}

#[test]
fn each_profile_admits_exactly_its_listed_operators() {
    let mut examined = 0;
    for profile in Profile::ALL {
        for operator in Operator::ALL {
            examined += 1;
            let parsed = profile.parse(sample(operator));
            let negated = profile.parse(&format!("-({})", sample(operator)));
            if profile.admits(operator) {
                assert!(parsed.is_ok(), "{} admits {operator}", profile.name());
                assert!(negated.is_ok(), "{} admits -({operator})", profile.name());
                let used: Vec<Operator> = parsed
                    .expect("admitted")
                    .terms()
                    .into_iter()
                    .map(Term::operator)
                    .collect();
                assert_eq!(used, vec![operator], "{} reads {operator}", profile.name());
            } else {
                let refusal = if operator == Operator::Field {
                    Refusal::UnknownKey
                } else {
                    Refusal::NotInProfile(operator)
                };
                assert_eq!(
                    parsed,
                    Err(refusal),
                    "{} refuses {operator}",
                    profile.name()
                );
                assert_eq!(
                    negated,
                    Err(refusal),
                    "{} refuses -({operator})",
                    profile.name()
                );
            }
        }
    }
    println!("examined {examined} profile and operator pair(s)");
    assert_eq!(examined, Profile::ALL.len() * Operator::ALL.len());
}

#[test]
fn the_skip_profile_is_the_closed_list_its_holds_were_written_against() {
    assert_eq!(
        Profile::SKIP.operators(),
        &[
            Operator::Text,
            Operator::Deck,
            Operator::Tag,
            Operator::Is,
            Operator::Prop,
            Operator::Card,
            Operator::Note,
        ]
    );
    assert_eq!(Profile::OPERATOR.operators(), &Operator::ALL);
    assert_eq!(
        (Profile::SKIP.name(), Profile::OPERATOR.name()),
        ("skip", "operator")
    );
}
