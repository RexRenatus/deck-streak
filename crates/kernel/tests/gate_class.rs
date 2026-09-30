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
    for (at, class) in GateClass::ALL.iter().enumerate() {
        for other in &GateClass::ALL[at + 1..] {
            assert_ne!(class.name(), other.name());
        }
    }
}
