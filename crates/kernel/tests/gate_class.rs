//! Text becomes a gate class only as a declared name, exactly (SPEC-046, ADR-046's amendment).
//! The kernel's own tests pin the parse, since a mutant of it is judged by this package's tests.

use deck_streak_kernel::{GateClass, UnknownGateClass};

#[test]
fn every_declared_class_parses_from_its_own_name_to_itself() {
    assert!(!GateClass::ALL.is_empty());
    for class in GateClass::ALL {
        assert_eq!(class.name().parse::<GateClass>(), Ok(*class));
    }
}

#[test]
fn a_name_no_class_declares_is_refused() {
    for text in [
        "",
        "zqxj-forged-class",
        " reading-length",
        "reading-length ",
        "READING-LENGTH",
    ] {
        assert_eq!(text.parse::<GateClass>(), Err(UnknownGateClass), "{text:?}");
    }
}

#[test]
fn no_two_classes_share_a_name() {
    let names: std::collections::BTreeSet<&str> =
        GateClass::ALL.iter().map(|class| class.name()).collect();
    assert_eq!(names.len(), GateClass::ALL.len());
}

#[test]
fn a_text_parses_only_to_the_class_it_names_exactly() {
    let mut examined = 0_usize;
    for class in GateClass::ALL {
        let name = class.name();
        let mut texts = vec![
            name.to_owned(),
            name.to_uppercase(),
            name.replace('-', "_"),
            format!("{name} "),
            format!(" {name}"),
            format!("{name}\n"),
            format!("{name}x"),
            format!("x{name}"),
        ];
        for (at, character) in name.char_indices() {
            texts.push(name[..at].to_owned());
            texts.push(name[at..].to_owned());
            let mut dropped = name.to_owned();
            dropped.replace_range(at..at + character.len_utf8(), "");
            texts.push(dropped);
        }
        for text in &texts {
            if let Ok(parsed) = text.parse::<GateClass>() {
                assert_eq!(
                    parsed.name(),
                    text.as_str(),
                    "{text:?} parsed to {parsed:?}"
                );
            }
            examined += 1;
        }
    }
    println!("examined {examined} text(s) near the declared names");
    assert!(examined > GateClass::ALL.len());
}
