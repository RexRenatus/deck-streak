//! The verdict is declared `#[must_use]`, so a dropped refusal does not compile under the
//! workspace's `unused_must_use = "deny"`; the `compile_fail` doc test on the type runs in the
//! doctest stage and proves it (SPEC-020 A10).

// An integration test is test code: its helper panics on an unreadable source file.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;

use deck_streak_kernel::Verdict;

/// The attribute lines written directly above the item that begins with `item` in `source`,
/// through its doc comment.
fn attributes_above(source: &str, item: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim_start().starts_with(item))
        .unwrap_or_else(|| panic!("{item} is not declared"));
    lines[..at]
        .iter()
        .rev()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("#[") || line.starts_with("///"))
        .filter(|line| line.starts_with("#["))
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_verdict_type_is_declared_must_use() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/verdict.rs");
    let source = fs::read_to_string(&path).expect("src/verdict.rs is readable");
    let attributes = attributes_above(&source, "pub enum Verdict<");
    assert!(
        attributes
            .iter()
            .any(|attribute| attribute.starts_with("#[must_use")),
        "Verdict is not declared #[must_use]: {attributes:?}"
    );
    // The proof the doctest stage runs: a program that drops a verdict, which must not compile.
    assert!(
        source.contains("```compile_fail\n//! #![deny(unused_must_use)]"),
        "the compile_fail doc test that drops a verdict is missing"
    );
    // What the attribute protects: a refusal reaches the caller that matches on it.
    assert_eq!(Verdict::Refuse("stale").refusal(), Some("stale"));
    assert_eq!(Verdict::<&str>::Pass.into_result(), Ok(()));
    assert!(Verdict::<&str>::Pass.is_pass());
}

#[test]
fn a_refusal_is_not_a_pass_and_carries_its_reason_up_as_an_error() {
    let refused: Verdict<&str> = Verdict::Refuse("stale");
    assert!(!refused.is_pass());
    assert_eq!(refused.into_result(), Err("stale"));
}
