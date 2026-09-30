//! Text becomes a gate class only as a declared name, exactly (SPEC-046, ADR-046's amendment).
//! The kernel's own tests pin the parse, since a mutant of it is judged by this package's tests.
#![allow(clippy::expect_used)]

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

/// The kernel's sources with every full-line comment and every whitespace character removed, so a
/// reformat that keeps the tokens keeps the pin.
fn squashed(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(str::split_whitespace)
        .collect()
}

/// Every `.rs` file under the kernel's `src/`, squashed, by path.
fn kernel_sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("the kernel's src is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path).expect("a source file is readable");
                out.push((path.display().to_string(), squashed(&source)));
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut out,
    );
    out
}

#[test]
fn the_parse_is_pinned_to_one_comparison_of_the_text_with_each_declared_name() {
    const WHY: &str = "a reformat of from_str updates this pin in the same commit";
    // The whole parse: the struct it fails with, the one impl, its signature and its body.
    const PARSE: &str = "pub struct UnknownGateClass; \
        impl std::str::FromStr for GateClass { \
        type Err = UnknownGateClass; \
        fn from_str(name: &str) -> Result<Self, Self::Err> { \
        Self::ALL.iter().copied().find(|class| class.name() == name).ok_or(UnknownGateClass) } }";
    // The type: its derives and the macro arm admit no attribute but a doc comment, so no
    // serde, strum or clap derive and no rename attribute can name a variant another way.
    const TYPE: &str = "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum GateClass {";
    const ARM: &str = "($($(#[doc = $doc:literal])+ $variant:ident => $name:literal,)+)";
    // `name` is an inherent `const fn` that returns `&'static str`, so `==` is `str`'s.
    const NAME: &str = "pub const fn name(self) -> &'static str {";

    let gate = squashed(include_str!("../src/gate_class.rs"));
    let sources = kernel_sources();
    assert!(sources.len() > 1, "the walk read no kernel source: {WHY}");
    let (_, walked) = sources
        .iter()
        .find(|(path, _)| path.ends_with("gate_class.rs"))
        .expect("the walk reads gate_class.rs");
    assert_eq!(walked, &gate, "{WHY}");

    for (what, pinned) in [
        ("parse", PARSE),
        ("type", TYPE),
        ("macro arm", ARM),
        ("name", NAME),
    ] {
        assert!(
            gate.contains(&squashed(pinned)),
            "the {what} is not the pinned text: {WHY}"
        );
    }

    // Every other way text could become a class is absent from the file: one impl of FromStr,
    // two impls in all (the inherent one and it), no other trait impl, alias, import,
    // conversion, derive, include, module or function.
    for (needle, count) in [
        ("FromStrfor", 1),
        ("forGateClass", 1),
        ("impl", 2),
        ("implGateClass{", 1),
        ("macro_rules!", 1),
        (".find(", 1),
        ("Self::ALL", 1),
        ("fn", 2),
        ("mod", 0),
        ("include", 0),
        ("cfg", 0),
        ("=GateClass", 0),
        ("GateClassas", 0),
        ("TryFrom", 0),
        ("From<", 0),
        ("Deserialize", 0),
        ("serde", 0),
        ("strum", 0),
        ("clap", 0),
        ("ValueEnum", 0),
        ("EnumString", 0),
    ] {
        assert_eq!(
            gate.matches(needle).count(),
            count,
            "{needle:?} in gate_class.rs: {WHY}"
        );
    }
    // No other file of the kernel names the type, so none can implement a trait for it.
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| !path.ends_with("gate_class.rs"))
    {
        let rest = text
            .replace("pubmodgate_class;", "")
            .replace("pubusegate_class::{GateClass,UnknownGateClass};", "");
        assert!(
            !rest.contains("GateClass") && !rest.contains("gate_class"),
            "{path} names the gate class: {WHY}"
        );
    }
}
