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

/// The byte offsets at which `source` declares the verdict enum: every word `enum` followed,
/// across any whitespace or comment, by the name `Verdict`, its raw prefix removed. Every line is
/// read, comments and strings included, so a copy anywhere, compiled out or not, and an enum
/// spelled any way (a wider gap under `#[rustfmt::skip]`, a line break or a comment between the
/// words, another visibility) each count: no copy can stand in for the enum that compiles.
fn declarations_in(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    source
        .match_indices("enum")
        .filter(|(at, _)| {
            let before = at.checked_sub(1).and_then(|i| bytes.get(i));
            !before.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                && name_after(&source[at + "enum".len()..]) == Some("Verdict")
        })
        .map(|(at, _)| at)
        .collect()
}

/// The identifier `rest` starts with once whitespace and comments are skipped, its raw prefix
/// removed, or `None` when `rest` continues the word before it.
fn name_after(rest: &str) -> Option<&str> {
    if rest.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let mut rest = rest.trim_start();
    loop {
        if let Some(line) = rest.strip_prefix("//") {
            rest = line.find('\n').map_or("", |n| &line[n..]).trim_start();
        } else if rest.starts_with("/*") {
            rest = rest[block_comment_len(rest)..].trim_start();
        } else {
            break;
        }
    }
    let rest = rest.strip_prefix("r#").unwrap_or(rest);
    let len = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    Some(&rest[..len])
}

/// The length of the block comment `text` opens, nested blocks included.
fn block_comment_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    let (mut depth, mut end) = (0_usize, 0);
    while end < bytes.len() {
        if bytes[end..].starts_with(b"/*") {
            depth += 1;
            end += 2;
        } else if bytes[end..].starts_with(b"*/") {
            depth = depth.saturating_sub(1);
            end += 2;
            if depth == 0 {
                break;
            }
        } else {
            end += 1;
        }
    }
    end
}

/// How many times `source` declares the verdict enum.
fn declarations_of(source: &str) -> usize {
    declarations_in(source).len()
}

/// Whether `attribute` is `#[must_use]`, its raw prefix removed.
fn is_must_use(attribute: &str) -> bool {
    without_raw_prefixes(attribute) == "#[must_use]"
}

/// Whether `attribute` is conditional: the last segment of its path, raw prefix removed, is `cfg`
/// or `cfg_attr`. `#[cfg(...)]` can compile the enum the scan reads out of the build, and
/// `#[cfg_attr(...)]` applies its attribute only under its condition, so the pin refuses either
/// and judges neither.
fn is_conditional(attribute: &str) -> bool {
    let inner = attribute.trim().strip_prefix("#[").unwrap_or(attribute);
    let path = inner
        .split(['(', '=', ']'])
        .next()
        .map(without_raw_prefixes)
        .unwrap_or_default();
    matches!(
        path.rsplit("::").next().map(str::trim),
        Some("cfg" | "cfg_attr")
    )
}

/// The attribute lines directly above the first declaration of the verdict enum in `source`.
fn attributes_of(source: &str) -> Vec<String> {
    let first = *declarations_in(source)
        .first()
        .expect("the verdict enum is declared in the source");
    let lines: Vec<&str> = source.lines().collect();
    let at = source[..first].matches('\n').count();
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
        attributes.iter().any(|a| is_must_use(a)),
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
    eprintln!("members: {members} declarations-by-attributes");
}

/// Every spelling of the enum that compiles: plain, raw, a wider gap, a line break or a comment
/// between the words (each under `#[rustfmt::skip]`, which keeps them), and another visibility.
const LIVE: [&str; 6] = [
    "pub enum Verdict {",
    "pub enum r#Verdict {",
    "#[rustfmt::skip]\npub  enum Verdict {",
    "#[rustfmt::skip]\npub enum\nVerdict {",
    "#[rustfmt::skip]\npub enum /* the verdict */ Verdict {",
    "pub(crate) enum Verdict {",
];

/// Every place a copy of the enum carrying `#[must_use]` can stand above the enum that compiles:
/// compiled out by its own `cfg` (plain, nested in a `cfg_attr`, holding a quote or a comment),
/// by a `cfg` above a line the attribute scan stops at (a comment, a blank line, a doc
/// attribute), by a `cfg` on an enclosing module or applied by a macro; inside a block comment or
/// line comments; and inside a string.
fn every_copy_of_the_enum() -> Vec<String> {
    let copy = "#[must_use]\n#[derive(Clone)]\npub enum Verdict {\n    A,\n}";
    vec![
        format!("#[cfg(any())]\n{copy}"),
        format!("#[cfg_attr(all(), cfg(any()))]\n{copy}"),
        format!("#[cfg(target_os = \"none\")]\n{copy}"),
        format!("#[cfg/**/(any())]\n{copy}"),
        format!("#[cfg(any())]\n// the scan stops here\n{copy}"),
        format!("#[cfg(any())]\n\n{copy}"),
        format!("#[cfg(any())]\n#[doc = \"a copy\"]\n{copy}"),
        format!("#[cfg(any())]\nmod dead {{\n{copy}\n}}"),
        format!(
            "macro_rules! dead {{\n    ($($item:tt)*) => {{ #[cfg(any())] $($item)* }};\n}}\ndead! {{\n{copy}\n}}"
        ),
        format!("/*\n{copy}\n*/"),
        copy.lines()
            .fold(String::new(), |text, line| text + "// " + line + "\n"),
        format!("const _COPY: &str = \"{}\";", copy.replace('\n', " ")),
    ]
}

#[test]
fn every_copy_of_the_enum_beside_every_spelling_of_it_is_a_second_declaration() {
    let copies = every_copy_of_the_enum();
    assert_eq!(copies.len(), 12, "the copies changed");
    let mut members = 0;
    for live in LIVE {
        for must_use in ["#[must_use]", "#[r#must_use]"] {
            let alone = format!("{must_use}\n#[derive(Clone)]\n{live}");
            assert_eq!(declarations_of(&alone), 1, "not declared: {alone}");
            let read = attributes_of(&alone);
            assert!(read.iter().any(|a| is_must_use(a)), "{alone}: {read:?}");
        }
        for copy in &copies {
            members += 1;
            let source = format!("{copy}\n#[derive(Clone)]\n{live}");
            assert_eq!(
                declarations_of(&source),
                2,
                "a copy stood in for the enum: {source}"
            );
        }
    }
    assert_eq!(members, 72, "the population changed");
    eprintln!("members: {members} copies-by-spellings");
}
