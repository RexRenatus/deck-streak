//! The memory state's equality and its lenient reader hold at their boundaries (SPEC-077 A1).
//!
//! Each test names the mutants it kills; they reach the crate only through `memory_state`'s public
//! items.

// An integration test is test code: it unwraps what it just built.
#![allow(clippy::expect_used)]

use deck_streak_ingest::memory_state::{MemoryState, parse};

/// A state whose every field is set, so a test can change one field at a time.
fn base() -> MemoryState {
    MemoryState {
        stability: 12.5,
        difficulty: 4.25,
        decay: 0.3,
        desired_retention: Some(0.9),
        last_review_sec: Some(1_700_000_000),
    }
}

#[test]
fn equal_states_are_equal() {
    assert_eq!(base(), base());
    assert_eq!(base(), base());
}

#[test]
fn a_state_differing_in_stability_alone_is_not_equal() {
    let other = MemoryState {
        stability: 12.75,
        ..base()
    };
    assert_ne!(base(), other);
}

#[test]
fn a_state_differing_in_difficulty_alone_is_not_equal() {
    let other = MemoryState {
        difficulty: 4.5,
        ..base()
    };
    assert_ne!(base(), other);
}

#[test]
fn a_state_differing_in_decay_alone_is_not_equal() {
    let other = MemoryState {
        decay: 0.31,
        ..base()
    };
    assert_ne!(base(), other);
}

#[test]
fn a_state_differing_in_desired_retention_alone_is_not_equal() {
    let other = MemoryState {
        desired_retention: Some(0.85),
        ..base()
    };
    assert_ne!(base(), other);
    let none = MemoryState {
        desired_retention: None,
        ..base()
    };
    assert_ne!(base(), none);
}

#[test]
fn a_state_differing_in_last_review_alone_is_not_equal() {
    let other = MemoryState {
        last_review_sec: Some(1_700_000_001),
        ..base()
    };
    assert_ne!(base(), other);
    let none = MemoryState {
        last_review_sec: None,
        ..base()
    };
    assert_ne!(base(), none);
}

#[test]
fn equality_compares_bits_so_nan_equals_itself_and_zero_signs_differ() {
    let nan = MemoryState {
        difficulty: f64::NAN,
        ..base()
    };
    assert_eq!(nan, nan);
    let zero = MemoryState {
        difficulty: 0.0,
        ..base()
    };
    let negative_zero = MemoryState {
        difficulty: -0.0,
        ..base()
    };
    assert_ne!(zero, negative_zero);
}

#[test]
fn an_escaped_unicode_key_is_read_under_its_decoded_name() {
    // The escape \u0064 spells the key "d"; the reader must keep the escape whole.
    let state = parse(Some(r#"{"s":2,"\u0064":5}"#)).expect("a state");
    assert_eq!(state.difficulty.to_bits(), 5.0_f64.to_bits());
}

#[test]
fn a_backslash_before_a_multibyte_char_does_not_split_it() {
    // An invalid escape: the text is not JSON, so the parse answers none, and must not panic.
    assert_eq!(parse(Some("{\"k\":\"\\\u{e9}\",\"s\":3}")), None);
}

#[test]
fn a_multibyte_char_before_an_escape_does_not_split_it() {
    let state = parse(Some("{\"k\":\"\u{e9}\\n\",\"s\":3,\"d\":6}")).expect("a state");
    assert_eq!(state.stability.to_bits(), 3.0_f64.to_bits());
    assert_eq!(state.difficulty.to_bits(), 6.0_f64.to_bits());
}

#[test]
fn a_string_holding_a_bare_word_stays_a_string_and_the_object_survives() {
    let state = parse(Some(r#"{"s":2,"k":"-Infinity 1e999 NaN","d":4}"#)).expect("a state");
    assert_eq!(state.stability.to_bits(), 2.0_f64.to_bits());
    assert_eq!(state.difficulty.to_bits(), 4.0_f64.to_bits());
}

#[test]
fn a_bare_word_outside_a_string_drops_only_its_field() {
    let state = parse(Some(r#"{"s":2,"d":NaN,"decay":-Infinity,"dr":1e999}"#)).expect("a state");
    assert_eq!(state.difficulty.to_bits(), 0.0_f64.to_bits());
    assert_eq!(state.desired_retention, None);
}

#[test]
fn a_number_shaped_run_inside_a_string_is_not_read_as_a_number() {
    // \u1e99 is a whole escape, so the 9 after it and the 1e99 inside it are string text, never
    // a number that overflows into null and breaks the escape.
    let state = parse(Some(r#"{"s":2,"k":"\u1e999","d":4}"#)).expect("a state");
    assert_eq!(state.difficulty.to_bits(), 4.0_f64.to_bits());
}

/// A source-text guard: the crate root declares the memory state module, public and bare, between
/// its neighbours. Row S07723 mutates this declaration, a change no behaviour test can see.
#[test]
fn the_crate_root_declares_the_memory_state_module_publicly() {
    let root = include_str!("../src/lib.rs");
    assert!(
        root.contains("pub mod lock;\npub mod memory_state;\npub mod reader;\n"),
        "the crate root declares `pub mod memory_state;` with no attribute before it"
    );
}
