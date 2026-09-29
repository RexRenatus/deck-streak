//! The verdict's `#[must_use]` (SPEC-043 A18): a caller cannot ignore how a run ended, and the
//! attribute is pinned because nothing else observes it.
#![allow(clippy::expect_used)]

/// The attribute lines directly above `pub enum Verdict` in `source`.
fn attributes_of(source: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == "pub enum Verdict {")
        .expect("the verdict enum is declared in the source");
    lines[..at]
        .iter()
        .rev()
        .take_while(|line| {
            // One whole attribute per line: `#[rustfmt::skip] fn f() {}` is an item, not an attribute.
            let line = line.trim();
            (line.starts_with("#[") && line.ends_with(']')) || line.starts_with("///")
        })
        .filter(|line| line.trim_start().starts_with("#["))
        .map(|line| line.trim().to_owned())
        .collect()
}

/// The attribute lines directly above `pub enum Verdict`, read from the source.
fn attributes_of_the_verdict() -> Vec<String> {
    attributes_of(include_str!("../src/verdict.rs"))
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

#[test]
fn an_item_ending_in_a_bracket_comment_is_not_an_attribute() {
    let good = "#[must_use]\n#[derive(Clone)]\npub enum Verdict {";
    assert!(attributes_of(good).iter().any(|a| a == "#[must_use]"));
    // `#[must_use]` here belongs to the function line, which merely ends in `]`; the enum itself
    // carries only its derive.
    let decoy = "#[must_use]\n#[rustfmt::skip] pub fn decoy() {} // ]\n#[derive(Clone)]\npub enum Verdict {";
    let attributes = attributes_of(decoy);
    assert!(
        attributes.iter().any(|a| a == "#[derive(Clone)]"),
        "the enum's own derive was not read: {attributes:?}"
    );
    assert!(
        !attributes.iter().any(|a| a == "#[must_use]"),
        "an item line ending in a comment was read as an attribute: {attributes:?}"
    );
}
