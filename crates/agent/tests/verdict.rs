//! The verdict's `#[must_use]` (SPEC-043 A18): a caller cannot ignore how a run ended, and the
//! attribute is pinned because nothing else observes it.
#![allow(clippy::expect_used)]

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// `source` as rustc's lexer reads it (SPEC-043 R12b): no comment is a token, a doc comment is a
/// `doc` attribute, a literal of any kind is one token whatever it holds, and whitespace is what
/// separates tokens. A source that does not lex yields nothing, and the pin refuses it.
fn tokens_of(source: &str) -> Vec<TokenTree> {
    source
        .parse::<TokenStream>()
        .map(|stream| stream.into_iter().collect())
        .unwrap_or_default()
}

/// `text` with every raw-identifier prefix removed: identifiers and attribute paths compare by the
/// name the compiler reads, so `r#Verdict` is `Verdict` and `r#cfg` is `cfg`.
fn without_raw_prefixes(text: &str) -> String {
    text.replace("r#", "")
}

/// Whether `tokens[at]` begins a declaration of the verdict enum: the keyword `enum`, then the
/// name `Verdict`, its raw prefix removed.
fn declares_the_verdict(tokens: &[TokenTree], at: usize) -> bool {
    matches!(
        (&tokens[at], tokens.get(at + 1)),
        (TokenTree::Ident(keyword), Some(TokenTree::Ident(name)))
            if *keyword == "enum" && without_raw_prefixes(&name.to_string()) == "Verdict"
    )
}

/// How many times `tokens`, at any depth, declare the verdict enum: one at the top of the file,
/// inside a module, a function, a macro's body or any other group, compiled out or not.
fn declarations(tokens: &[TokenTree]) -> usize {
    tokens
        .iter()
        .enumerate()
        .map(|(at, token)| {
            usize::from(declares_the_verdict(tokens, at))
                + match token {
                    TokenTree::Group(group) => {
                        declarations(&group.stream().into_iter().collect::<Vec<_>>())
                    }
                    TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => 0,
                }
        })
        .sum()
}

/// How many times `source` declares the verdict enum, compiled out or not: a copy under `cfg`, in
/// a module or in a macro's body is a declaration, and a copy in a comment or a literal is none.
fn declarations_of(source: &str) -> usize {
    declarations(&tokens_of(source))
}

/// `tokens` written out with no whitespace but a space between two words, so one attribute reads
/// the same however it was spaced: `# [ derive ( Clone ) ]` is `#[derive(Clone)]`.
fn rendered(tokens: TokenStream) -> String {
    let mut text = String::new();
    for token in tokens {
        let piece = match &token {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::None => ("", ""),
                };
                format!("{open}{}{close}", rendered(group.stream()))
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => token.to_string(),
        };
        let word = |ch: char| ch.is_alphanumeric() || ch == '_' || ch == '"';
        if text.ends_with(word) && piece.starts_with(word) {
            text.push(' ');
        }
        text.push_str(&piece);
    }
    text
}

/// The outer attributes directly before the declaration of the verdict enum at the top of
/// `source`, each written out by `rendered`: the `#[..]` pairs that precede its visibility, up to
/// the first token that is not one. Empty when no declaration stands at the top of the file.
fn attributes_of(source: &str) -> Vec<String> {
    let tokens = tokens_of(source);
    let Some(mut first) = (0..tokens.len()).find(|&at| declares_the_verdict(&tokens, at)) else {
        return Vec::new();
    };
    let is_pub =
        |token: Option<&TokenTree>| matches!(token, Some(TokenTree::Ident(word)) if *word == "pub");
    if first >= 2
        && matches!(&tokens[first - 1], TokenTree::Group(scope) if scope.delimiter() == Delimiter::Parenthesis)
        && is_pub(tokens.get(first - 2))
    {
        first -= 2;
    } else if first >= 1 && is_pub(tokens.get(first - 1)) {
        first -= 1;
    }
    let mut attributes = Vec::new();
    while first >= 2 {
        match (&tokens[first - 2], &tokens[first - 1]) {
            (TokenTree::Punct(hash), TokenTree::Group(body))
                if hash.as_char() == '#' && body.delimiter() == Delimiter::Bracket =>
            {
                attributes.push(format!("#[{}]", rendered(body.stream())));
                first -= 2;
            }
            _ => break,
        }
    }
    attributes.reverse();
    attributes
}

/// Whether `attribute` is `#[must_use]`, bare or with a reason, its raw prefix removed.
fn is_must_use(attribute: &str) -> bool {
    let attribute = without_raw_prefixes(attribute);
    attribute == "#[must_use]"
        || attribute
            .strip_prefix("#[must_use=")
            .is_some_and(|reason| reason.starts_with('"') && reason.ends_with("\"]"))
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

/// What is wrong with a module's declaration of the verdict enum (SPEC-043 R12b): an empty list
/// when the module declares it exactly once, its attributes are read, derive included, one is
/// `#[must_use]` and none is conditional.
fn verdict_pin_problems(source: &str) -> Vec<String> {
    let declarations = declarations_of(source);
    if declarations != 1 {
        return vec![format!(
            "the verdict enum is declared {declarations} times, not once"
        )];
    }
    let attributes = attributes_of(source);
    let mut problems = Vec::new();
    if !attributes.iter().any(|a| a.starts_with("#[derive(")) {
        problems.push(format!(
            "the enum's attributes were not read: {attributes:?}"
        ));
    }
    if !attributes.iter().any(|a| is_must_use(a)) {
        problems.push(format!("the enum lost its #[must_use]: {attributes:?}"));
    }
    if attributes.iter().any(|a| is_conditional(a)) {
        problems.push(format!(
            "the enum carries a conditional attribute: {attributes:?}"
        ));
    }
    problems
}

#[test]
fn the_verdict_type_is_must_use() {
    // The enum is declared once, its attributes are read, derive included, one is `#[must_use]`
    // and none is conditional.
    assert_eq!(
        verdict_pin_problems(include_str!("../src/verdict.rs")),
        Vec::<String>::new()
    );
}

#[test]
fn an_item_ending_in_a_bracket_comment_is_not_an_attribute() {
    let good = "#[must_use]\n#[derive(Clone)]\npub enum Verdict { A }";
    assert!(attributes_of(good).iter().any(|a| a == "#[must_use]"));
    // `#[must_use]` here belongs to the function line, which merely ends in `]`; the enum itself
    // carries only its derive.
    let decoy = "#[must_use]\n#[rustfmt::skip] pub fn decoy() {} // ]\n#[derive(Clone)]\npub enum Verdict { A }";
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
    let good = "#[must_use]\n#[derive(Clone)]\npub enum Verdict { A }";
    assert!(attributes_of(good).iter().any(|a| a == "#[must_use]"));
    // Each decoy leaves the enum without a `#[must_use]` of its own: the first gives it to a
    // function, the second and third hide it in a raw string and in a block comment.
    for decoy in [
        "#[must_use]\n#[doc = \"[\"] pub fn decoy() {} // ]\n#[derive(Clone)]\npub enum Verdict { A }",
        "pub const DECOY: &str = r#\"\n#[must_use]\n#[\"#; #[derive(Clone)] // ]\n#[derive(Debug)]\npub enum Verdict { A }",
        "/*\n#[must_use]\n/// */\n#[derive(Clone)]\npub enum Verdict { A }",
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
        verdict_pin_problems("#[must_use]\n#[derive(Clone)]\npub enum Verdict { A }"),
        Vec::<String>::new()
    );
    // The copy inside the comment carries the `#[must_use]`; the enum that compiles carries none.
    let decoy = "/*\n#[must_use]\n#[derive(Clone)]\npub enum Verdict {\n*/\n#[derive(Clone)]\npub enum Verdict { A }";
    assert!(
        !verdict_pin_problems(decoy).is_empty(),
        "the commented copy stood in for the enum: {decoy}"
    );
}

#[test]
fn a_raw_identifier_declaration_is_the_verdict_enum() {
    let raw = "#[must_use]\n#[derive(Clone)]\npub enum r#Verdict { A }";
    assert_eq!(declarations_of(raw), 1);
    assert!(attributes_of(raw).iter().any(|a| a == "#[must_use]"));
    // The copy the scan reaches first is compiled out; the enum that compiles is spelled with the
    // raw identifier and carries no `#[must_use]`, so it is a second declaration.
    let decoy = "#[must_use]\n#[cfg(any())]\n#[derive(Clone)]\npub enum Verdict {\n}\n#[derive(Clone)]\npub enum r#Verdict { A }";
    assert_eq!(declarations_of(decoy), 2);
}

#[test]
fn a_conditional_attribute_on_the_enum_is_refused() {
    let good = attributes_of("#[must_use]\n#[derive(Clone)]\npub enum Verdict { A }");
    assert!(good.iter().any(|a| a == "#[must_use]"));
    assert!(!good.iter().any(|a| is_conditional(a)));
    // A copy compiled out by `cfg`, in either spelling, and a `must_use` given only under a
    // `cfg_attr` condition: each is read, and each is conditional.
    for decoy in [
        "#[must_use]\n#[cfg(any())]\n#[derive(Clone)]\npub enum Verdict { A }",
        "#[must_use]\n#[r#cfg(any())]\n#[derive(Clone)]\npub enum Verdict { A }",
        "#[cfg_attr(test, must_use)]\n#[derive(Clone)]\npub enum Verdict { A }",
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
        let plain = format!("#[must_use]\n#[derive(Clone)]\npub enum {name} {{ A }}");
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
                    format!("#[must_use]\n{written}\n#[derive(Clone)]\npub enum {name} {{ A }}");
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
    "pub enum Verdict { A }",
    "pub enum r#Verdict { A }",
    "#[rustfmt::skip]\npub  enum Verdict { A }",
    "#[rustfmt::skip]\npub enum\nVerdict { A }",
    "#[rustfmt::skip]\npub enum /* the verdict */ Verdict { A }",
    "pub(crate) enum Verdict { A }",
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
fn a_copy_of_the_enum_in_code_is_a_declaration_and_in_a_comment_or_a_literal_is_none() {
    let copies = every_copy_of_the_enum();
    assert_eq!(copies.len(), 12, "the copies changed");
    // The first nine copies are code, compiled out or not; the last three are a block comment,
    // line comments and a string.
    let (code, held) = copies.split_at(9);
    let mut members = 0;
    for live in LIVE {
        for must_use in ["#[must_use]", "#[r#must_use]"] {
            let alone = format!("{must_use}\n#[derive(Clone)]\n{live}");
            assert_eq!(declarations_of(&alone), 1, "not declared: {alone}");
            let read = attributes_of(&alone);
            assert!(read.iter().any(|a| is_must_use(a)), "{alone}: {read:?}");
        }
        for copy in code {
            members += 1;
            let source = format!("{copy}\n#[derive(Clone)]\n{live}");
            assert_eq!(
                declarations_of(&source),
                2,
                "a copy stood in for the enum: {source}"
            );
        }
        for copy in held {
            members += 1;
            let source = format!("{copy}\n#[derive(Clone)]\n{live}");
            assert_eq!(
                declarations_of(&source),
                1,
                "a copy in a comment or a literal was read as a declaration: {source}"
            );
            let problems = verdict_pin_problems(&source);
            assert!(
                problems.len() == 1 && problems[0].starts_with("the enum lost its #[must_use]"),
                "a held copy lent the enum its #[must_use]: {source}: {problems:?}"
            );
        }
    }
    assert_eq!(members, 72, "the population changed");
    eprintln!("examined {members} copies-by-spellings");
}

/// The enum that compiles, with `attributes` above it.
fn the_enum_under(attributes: &str) -> String {
    format!("{attributes}\npub enum Verdict {{\n    A,\n}}\n")
}

/// `holder`, then the enum that compiles: its `#[must_use]` when `must_use`, its derive and its
/// declaration, on the line the holder closes on when the holder ends in a space, on the lines
/// after it otherwise.
fn the_enum_after(holder: &str, must_use: bool) -> String {
    let gap = if holder.ends_with(' ') { " " } else { "\n" };
    let must_use = if must_use {
        format!("#[must_use]{gap}")
    } else {
        String::new()
    };
    format!("{holder}{must_use}#[derive(Clone, Debug)]{gap}pub enum Verdict {{\n    A,\n}}\n")
}

/// The string-like literal kinds rustc lexes as one token whatever they hold, each an opener, a
/// closer and the type of a constant holding it: a string, raw strings with none to three hashes,
/// a byte string and a raw one, a C string and a raw one.
const LITERAL_KINDS: [(&str, &str, &str); 11] = [
    ("\"", "\"", "&str"),
    ("r\"", "\"", "&str"),
    ("r#\"", "\"#", "&str"),
    ("r##\"", "\"##", "&str"),
    ("r###\"", "\"###", "&str"),
    ("b\"", "\"", "&[u8]"),
    ("br\"", "\"", "&[u8]"),
    ("br#\"", "\"#", "&[u8]"),
    ("c\"", "\"", "&core::ffi::CStr"),
    ("cr\"", "\"", "&core::ffi::CStr"),
    ("cr#\"", "\"#", "&core::ffi::CStr"),
];

/// `text` written as the literal `open`..`close`: escaped where the kind escapes, as written where
/// it is raw, or `None` where a raw kind cannot hold `text` because `text` holds its closer.
fn literal_holding(open: &str, close: &str, text: &str) -> Option<String> {
    if open.trim_start_matches(['b', 'c']).starts_with('r') {
        (!text.contains(close)).then(|| format!("{open}{text}{close}"))
    } else {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        Some(format!("{open}{escaped}{close}"))
    }
}

/// Where a holder closes against the enum after it: a holder on one line of its own; one that
/// spans lines and closes on the line before; one that spans lines and closes on the
/// declaration's own line. Each is the text before and after the held text, then what separates
/// the holder from the enum.
const CLOSES: [(&str, &str, &str); 3] = [("", "", "\n"), ("\n", "\n", "\n"), ("\n", "\n", " ")];

/// Every comment kind rustc lexes that may stand among a module's items, as an opener and a
/// closer: line, a line of four slashes, outer doc, block, nested block and outer doc block. A
/// line kind closes where its line ends, so it holds a text of several lines one line each.
const COMMENT_KINDS: [(&str, &str); 6] = [
    ("// ", ""),
    ("//// ", ""),
    ("/// ", ""),
    ("/* ", " */"),
    ("/* /* ", " */ */"),
    ("/** ", " */"),
];

/// Every holder of `text`: each literal kind as a constant, each string kind as a doc attribute,
/// and each comment kind, closing in each place `CLOSES` names that the holder can close. Each
/// holder ends with the separator the code after it takes.
fn every_holder_of(text: &str) -> Vec<String> {
    let mut holders = Vec::new();
    for (before, after, separator) in CLOSES {
        let held = format!("{before}{text}{after}");
        for (open, close, kind) in LITERAL_KINDS {
            if let Some(literal) = literal_holding(open, close, &held) {
                holders.push(format!("const _HELD: {kind} = {literal};{separator}"));
                if kind == "&str" {
                    holders.push(format!("#[doc = {literal}]{separator}"));
                }
            }
        }
        for (open, close) in COMMENT_KINDS {
            if !close.is_empty() {
                holders.push(format!("{open}{held}{close}{separator}"));
            } else if before.is_empty() {
                holders.push(
                    held.lines()
                        .fold(String::new(), |text, line| text + open + line + "\n"),
                );
            }
        }
    }
    holders
}

#[test]
fn a_must_use_held_by_a_literal_or_a_comment_is_not_the_enums() {
    // The holder carries a `#[must_use]` and a derive; the enum that compiles carries only its
    // own derive, so each member is refused. The same holder above an enum with its own
    // `#[must_use]` is read as nothing and the enum passes.
    let holders = every_holder_of("#[must_use]\n#[derive(Clone)]");
    assert_eq!(holders.len(), 60, "the population changed");
    let mut members = 0;
    for holder in &holders {
        let weak = the_enum_after(holder, false);
        assert!(
            !verdict_pin_problems(&weak).is_empty(),
            "a held #[must_use] was read as the enum's: {weak}"
        );
        let strong = the_enum_after(holder, true);
        assert_eq!(
            verdict_pin_problems(&strong),
            Vec::<String>::new(),
            "a holder above the enum was read as code: {strong}"
        );
        members += 2;
    }
    assert_eq!(members, 120, "the population changed");
    eprintln!("examined {members} attribute holders");
}

#[test]
fn a_copy_of_the_enum_held_by_a_literal_or_a_comment_is_no_declaration() {
    // A copy carrying `#[must_use]` inside a literal or a comment declares nothing: beside an
    // enum without one the source is refused on the enum's own attributes, and beside an enum
    // with one it passes.
    let holders = every_holder_of(&the_enum_under("#[must_use]\n#[derive(Clone)]"));
    assert_eq!(holders.len(), 60, "the population changed");
    let mut members = 0;
    for holder in &holders {
        let weak = the_enum_after(holder, false);
        let problems = verdict_pin_problems(&weak);
        assert!(
            problems.len() == 1 && problems[0].starts_with("the enum lost its #[must_use]"),
            "a held copy was read as a declaration: {weak}: {problems:?}"
        );
        let strong = the_enum_after(holder, true);
        assert_eq!(
            verdict_pin_problems(&strong),
            Vec::<String>::new(),
            "a held copy was read as a second declaration: {strong}"
        );
        members += 2;
    }
    assert_eq!(members, 120, "the population changed");
    eprintln!("examined {members} held copies");
}

/// Rustc's whitespace, `Pattern_White_Space`: the eleven code points its lexer skips between
/// tokens. NBSP and U+3000 are not among them, and rustc refuses either between tokens.
const RUSTC_WHITESPACE: [char; 11] = [
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{85}', '\u{200e}', '\u{200f}', '\u{2028}',
    '\u{2029}',
];

#[test]
fn every_gap_rustc_reads_between_the_enums_tokens_is_whitespace() {
    // Each whitespace code point in each gap between the tokens the pin reads, in an enum with
    // its `#[must_use]`: each is read and passes. The three gaps of the declaration again in an
    // enum without one, under a commented copy that has one: each is refused.
    let mut members = 0;
    for w in RUSTC_WHITESPACE {
        let strong = [
            format!("#[must_use]\n#[derive(Clone)]\npub{w}enum Verdict {{\n    A,\n}}\n"),
            format!("#[must_use]\n#[derive(Clone)]\npub enum{w}Verdict {{\n    A,\n}}\n"),
            format!("#[must_use]\n#[derive(Clone)]\npub enum Verdict{w}{{\n    A,\n}}\n"),
            format!("#{w}[must_use]\n#[derive(Clone)]\npub enum Verdict {{\n    A,\n}}\n"),
            format!("#[{w}must_use]\n#[derive(Clone)]\npub enum Verdict {{\n    A,\n}}\n"),
            format!("#[must_use{w}]\n#[derive(Clone)]\npub enum Verdict {{\n    A,\n}}\n"),
            format!("#[must_use]{w}#[derive(Clone)]\npub enum Verdict {{\n    A,\n}}\n"),
        ];
        for source in &strong {
            assert_eq!(
                verdict_pin_problems(source),
                Vec::<String>::new(),
                "whitespace rustc skips was not skipped: {source:?}"
            );
            members += 1;
        }
        let copy = "/*\n#[must_use]\n#[derive(Clone)]\npub enum Verdict {\n    A,\n}\n*/\n";
        let weak = [
            format!("{copy}#[derive(Clone)]\npub{w}enum Verdict {{\n    A,\n}}\n"),
            format!("{copy}#[derive(Clone)]\npub enum{w}Verdict {{\n    A,\n}}\n"),
            format!("{copy}#[derive(Clone)]\npub enum Verdict{w}{{\n    A,\n}}\n"),
        ];
        for source in &weak {
            assert!(
                !verdict_pin_problems(source).is_empty(),
                "a commented copy stood in for the enum: {source:?}"
            );
            members += 1;
        }
    }
    // The attribute's other spellings rustc reads as `#[must_use]`.
    for attribute in [
        "#[must_use = \"a run's end is read\"]",
        "#[r#must_use]",
        "# [ must_use ]",
    ] {
        let source = the_enum_under(&format!("{attribute}\n#[derive(Clone)]"));
        assert_eq!(
            verdict_pin_problems(&source),
            Vec::<String>::new(),
            "a spelling of #[must_use] was not read: {source}"
        );
        members += 1;
    }
    assert_eq!(members, 113, "the population changed");
    eprintln!("examined {members} gaps and spellings");
}

#[test]
fn a_first_line_rustc_removes_as_a_shebang_is_refused() {
    // rustc removes a first line opening with `#!` unless `[` follows past whitespace and plain
    // comments, so a `#[must_use]` written on it is not the enum's: the pin refuses the source.
    // `\u{a0}` and `\u{3000}` are whitespace to `proc-macro2` and not to rustc.
    let enum_below = "#[derive(Clone)]\npub enum Verdict {\n    A,\n}\n";
    let mut members = 0;
    for bom in ["", "\u{feff}"] {
        for gap in [
            "",
            " ",
            "\t",
            "/usr/bin/env run-cargo-script ",
            " /* c */ ",
            " /** d */ ",
            "\u{a0}[allow(unused)] ",
            "\u{3000}[allow(unused)] ",
        ] {
            let source = format!("{bom}#!{gap}#[must_use]\n{enum_below}");
            let problems = verdict_pin_problems(&source);
            assert!(
                problems.iter().any(|problem| problem.contains("shebang")),
                "a shebang line was read as the enum's attribute: {source:?}: {problems:?}"
            );
            members += 1;
        }
    }
    assert_eq!(members, 16, "the population changed");
    // An inner attribute is no shebang: the enum under `#![allow(unused)]` is read.
    assert_eq!(
        verdict_pin_problems(&format!("#![allow(unused)]\n#[must_use]\n{enum_below}")),
        Vec::<String>::new()
    );
}

#[test]
fn a_copy_of_the_enum_compiled_out_beside_it_is_refused_as_a_second_declaration() {
    // The pin evaluates no `cfg` (SPEC-043 section 10): a copy of the enum in code beside it is a
    // second declaration, and the source is refused whatever either copy carries.
    let source = "#[cfg(any())]\nmod dead {\n    #[must_use]\n    enum Verdict {}\n}\n#[must_use]\n#[derive(Clone)]\npub enum Verdict { A }";
    assert_eq!(
        verdict_pin_problems(source),
        ["the verdict enum is declared 2 times, not once"]
    );
}
