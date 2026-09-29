//! The verdict's `#[must_use]` (SPEC-043 A18): a caller cannot ignore how a run ended, and the
//! attribute is pinned because nothing else observes it.
#![allow(clippy::expect_used)]

/// Whether `line` is one whole attribute: the bracket that closes `#[` is the last character, so
/// a trailing `// ]` after an item is not read as an attribute's end.
fn is_one_whole_attribute(line: &str) -> bool {
    let line = line.trim();
    // A bracket inside a string, a character or a comment would be counted as the attribute's
    // own, so a line holding a quote or a comment marker is never one whole attribute: it fails
    // closed, and the scan stops there.
    if !line.starts_with("#[") || line.contains(['"', '\'']) || line.contains("//") {
        return false;
    }
    if line.contains("/*") || line.contains("*/") {
        return false;
    }
    let mut depth = 0_usize;
    for (at, ch) in line.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at + 1 == line.len();
                }
            }
            _ => {}
        }
    }
    false
}

/// Whether `line` is a doc comment the scan may pass: one that cannot close a string or a block
/// comment opened above it, so an attribute hidden inside either is never read.
fn is_a_plain_doc_line(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("///") && !line.contains('"') && !line.contains("/*") && !line.contains("*/")
}

/// `text` with every raw-identifier prefix removed: identifiers and attribute paths compare by the
/// name the compiler reads, so `r#Verdict` is `Verdict` and `r#cfg` is `cfg`.
fn without_raw_prefixes(text: &str) -> String {
    text.replace("r#", "")
}

/// Whether `line` declares the verdict enum, whichever way its name is spelled.
fn declares_the_verdict(line: &str) -> bool {
    without_raw_prefixes(line.trim()) == "pub enum Verdict {"
}

/// How many lines of `source` declare the verdict enum: a copy above the real one, inside a
/// comment or a string, or compiled out, would lend the real enum the copy's attributes.
fn declarations_of(source: &str) -> usize {
    source
        .lines()
        .filter(|line| declares_the_verdict(line))
        .count()
}

/// Whether `attribute` is conditional. Not yet read: the test below names what it must find.
fn is_conditional(_attribute: &str) -> bool {
    false
}

/// The attribute lines directly above `pub enum Verdict` in `source`.
fn attributes_of(source: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| declares_the_verdict(line))
        .expect("the verdict enum is declared in the source");
    lines[..at]
        .iter()
        .rev()
        .take_while(|line| {
            // One whole attribute per line: `#[rustfmt::skip] fn f() {}` is an item, not an attribute.
            is_one_whole_attribute(line) || is_a_plain_doc_line(line)
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
    assert_eq!(
        declarations_of(include_str!("../src/verdict.rs")),
        1,
        "the verdict enum is not declared exactly once"
    );
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
    assert!(
        !attributes.iter().any(|a| is_conditional(a)),
        "`pub enum Verdict` carries a conditional attribute: {attributes:?}"
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

#[test]
fn a_bracket_inside_a_string_or_a_comment_never_closes_an_attribute() {
    let good = "#[must_use]\n#[derive(Clone)]\npub enum Verdict {";
    assert!(attributes_of(good).iter().any(|a| a == "#[must_use]"));
    // Each decoy leaves the enum without a `#[must_use]` of its own: the first gives it to a
    // function, the second and third hide it in a raw string and in a block comment.
    for decoy in [
        "#[must_use]\n#[doc = \"[\"] pub fn decoy() {} // ]\n#[derive(Clone)]\npub enum Verdict {",
        "pub const DECOY: &str = r#\"\n#[must_use]\n#[\"#; #[derive(Clone)] // ]\n#[derive(Debug)]\npub enum Verdict {",
        "/*\n#[must_use]\n/// */\n#[derive(Clone)]\npub enum Verdict {",
    ] {
        let attributes = attributes_of(decoy);
        assert!(
            attributes.iter().any(|a| a.starts_with("#[derive(")),
            "the enum's own derive was not read: {attributes:?}"
        );
        assert!(
            !attributes.iter().any(|a| a == "#[must_use]"),
            "a must_use outside the enum's attributes was read as one: {attributes:?}"
        );
    }
}

#[test]
fn a_commented_copy_of_the_enum_above_it_is_refused() {
    assert_eq!(
        declarations_of("#[must_use]\n#[derive(Clone)]\npub enum Verdict {"),
        1
    );
    // The copy inside the comment carries the `#[must_use]`; the enum that compiles carries none.
    let decoy = "/*\n#[must_use]\n#[derive(Clone)]\npub enum Verdict {\n*/\n#[derive(Clone)]\npub enum Verdict {";
    assert_eq!(declarations_of(decoy), 2);
}

#[test]
fn a_raw_identifier_declaration_is_the_verdict_enum() {
    let raw = "#[must_use]\n#[derive(Clone)]\npub enum r#Verdict {";
    assert_eq!(declarations_of(raw), 1);
    assert!(attributes_of(raw).iter().any(|a| a == "#[must_use]"));
    // The copy the scan reaches first is compiled out; the enum that compiles is spelled with the
    // raw identifier and carries no `#[must_use]`, so it is a second declaration.
    let decoy = "#[must_use]\n#[cfg(any())]\n#[derive(Clone)]\npub enum Verdict {\n}\n#[derive(Clone)]\npub enum r#Verdict {";
    assert_eq!(declarations_of(decoy), 2);
}

#[test]
fn a_conditional_attribute_on_the_enum_is_refused() {
    let good = attributes_of("#[must_use]\n#[derive(Clone)]\npub enum Verdict {");
    assert!(good.iter().any(|a| a == "#[must_use]"));
    assert!(!good.iter().any(|a| is_conditional(a)));
    // A copy compiled out by `cfg`, in either spelling, and a `must_use` given only under a
    // `cfg_attr` condition: each is read, and each is conditional.
    for decoy in [
        "#[must_use]\n#[cfg(any())]\n#[derive(Clone)]\npub enum Verdict {",
        "#[must_use]\n#[r#cfg(any())]\n#[derive(Clone)]\npub enum Verdict {",
        "#[cfg_attr(test, must_use)]\n#[derive(Clone)]\npub enum Verdict {",
    ] {
        let attributes = attributes_of(decoy);
        assert!(
            attributes.iter().any(|a| a.starts_with("#[derive(")),
            "the enum's own derive was not read: {attributes:?}"
        );
        assert!(
            attributes.iter().any(|a| is_conditional(a)),
            "a conditional attribute was not seen: {attributes:?}"
        );
    }
}

#[test]
fn every_spelling_of_the_declaration_and_of_a_conditional_attribute_is_read() {
    // The population: two declaration spellings, four attribute names (each a spelling the
    // compiler accepts, checked once in a scratch crate), each written with and without spaces
    // inside the brackets. Every member compiles.
    let mut members = 0;
    for name in ["Verdict", "r#Verdict"] {
        let plain = format!("#[must_use]\n#[derive(Clone)]\npub enum {name} {{");
        assert_eq!(declarations_of(&plain), 1, "not declared: {name}");
        let read = attributes_of(&plain);
        assert!(read.iter().any(|a| a == "#[must_use]"), "{name}: {read:?}");
        assert!(!read.iter().any(|a| is_conditional(a)), "{name}: {read:?}");
        for (attribute, argument) in [
            ("cfg", "any()"),
            ("r#cfg", "any()"),
            ("cfg_attr", "test, must_use"),
            ("r#cfg_attr", "test, must_use"),
        ] {
            for written in [
                format!("#[{attribute}({argument})]"),
                format!("#[ {attribute} ({argument}) ]"),
            ] {
                members += 1;
                let source =
                    format!("#[must_use]\n{written}\n#[derive(Clone)]\npub enum {name} {{");
                assert_eq!(declarations_of(&source), 1, "not declared: {source}");
                let read = attributes_of(&source);
                assert!(
                    read.iter().any(|a| a.starts_with("#[derive(")),
                    "the enum's own derive was not read: {source}"
                );
                assert!(
                    read.iter().any(|a| is_conditional(a)),
                    "a conditional attribute was not refused: {source}"
                );
            }
        }
    }
    assert_eq!(members, 16, "the population changed");
}
