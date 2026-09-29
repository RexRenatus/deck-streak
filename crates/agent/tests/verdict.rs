//! The verdict's `#[must_use]` (SPEC-043 A18): a caller cannot ignore how a run ended, and the
//! attribute is pinned because nothing else observes it.
#![allow(clippy::expect_used)]

/// The attribute lines directly above `pub enum Verdict`, read from the source.
fn attributes_of_the_verdict() -> Vec<String> {
    let source = include_str!("../src/verdict.rs");
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == "pub enum Verdict {")
        .expect("the verdict enum is declared in verdict.rs");
    lines[..at]
        .iter()
        .rev()
        .take_while(|line| {
            line.trim_start().starts_with("#[") || line.trim_start().starts_with("///")
        })
        .filter(|line| line.trim_start().starts_with("#["))
        .map(|line| line.trim().to_owned())
        .collect()
}

#[test]
fn the_verdict_type_is_must_use() {
    let attributes = attributes_of_the_verdict();
    // The positive fact: the enum was found and its attributes were read, derive included.
    assert!(
        attributes.iter().any(|a| a.starts_with("#[derive(")),
        "the attributes of `pub enum Verdict` were not read: {attributes:?}"
    );
    assert!(
        attributes.iter().any(|a| a == "#[must_use]"),
        "`pub enum Verdict` lost its #[must_use]: {attributes:?}"
    );
}
